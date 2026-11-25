use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use pebble_core::adapters::fs::{JsonTrustStore, LocalFileStore};
use pebble_core::application::engine::{
    spawn_engine, EngineCommand, EngineDeps, EngineHandle, PeerSummary,
};
use pebble_core::application::events::UiEvent;
use pebble_core::application::net::{
    InboundId, InboundRequest, NetCommand, NetEvent, NetworkLink, PairingAsk, StreamMsg,
};
use pebble_core::application::ports::{
    CredentialStore, DeviceCredentials, FileStore, IncomingTarget, NetworkFactory, ServerApi,
    TrustStore, UiEvents,
};
use pebble_core::application::server_sync::{spawn_server_sync, ServerDeps};
use pebble_core::application::state::{ActivityStore, SettingsStore};
use pebble_core::application::supervisor::P2pSupervisor;
use pebble_core::domain::error::{AppResult, ErrorCode};
use pebble_core::domain::files::FileSnapshot;
use pebble_core::domain::path::{FileName, RelativePath};
use pebble_core::domain::peer::PeerId;
use pebble_core::domain::settings::Settings;
use pebble_core::domain::transfer::TransferId;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

use pebble_core::application::engine::TransferState;
use pebble_core::application::net::PairingReply;
use pebble_core::application::ports::{ChunkSink, OpenedFile};
use pebble_core::domain::activity::ActivityRecord;
use pebble_core::domain::error::AppError;
use pebble_core::domain::files::{FileEntry, FolderStats};
use pebble_core::domain::transfer::TransferStatus;

const PEER: &str = "12D3KooWAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

fn tmp(label: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("break1-{label}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[derive(Default)]
struct Rec(Mutex<Vec<serde_json::Value>>);
impl UiEvents for Rec {
    fn emit(&self, e: UiEvent) {
        self.0
            .lock()
            .unwrap()
            .push(serde_json::to_value(&e).unwrap());
    }
}
impl Rec {
    fn count(&self, name: &str) -> usize {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e["event"] == name)
            .count()
    }
}

struct MemTrust(Vec<PeerId>);
#[async_trait]
impl TrustStore for MemTrust {
    async fn load(&self) -> AppResult<Vec<PeerId>> {
        Ok(self.0.clone())
    }
    async fn save(&self, _: &[PeerId]) -> AppResult<()> {
        Ok(())
    }
}

struct Harness {
    handle: EngineHandle,
    cmds: mpsc::UnboundedReceiver<NetCommand>,
    events: mpsc::Sender<NetEvent>,
    ui: Arc<Rec>,
    shared: PathBuf,
    root: PathBuf,
}

fn harness(trusted: &[&str]) -> Harness {
    let trust = MemTrust(trusted.iter().map(|p| PeerId::parse(p).unwrap()).collect());
    harness_with(Arc::new(trust))
}

fn harness_with(trust: Arc<dyn TrustStore>) -> Harness {
    let root = tmp("engine");
    let shared = root.join("shared");
    let ui = Arc::new(Rec::default());
    let mut s = Settings::with_home(Some(root.clone()));
    s.shared_folder = shared.clone();
    let deps = EngineDeps {
        ui: ui.clone(),
        files: Arc::new(LocalFileStore::new(root.join("downloads"))),
        settings: SettingsStore::new(s),
        activity: ActivityStore::default(),
        trust,
    };
    let (net, cmds) = mpsc::unbounded_channel();
    let (events, rx) = mpsc::channel(256);
    let (handle, _task) = spawn_engine(deps, net, rx);
    Harness {
        handle,
        cmds,
        events,
        ui,
        shared,
        root,
    }
}

async fn next_command(h: &mut Harness, wanted: impl Fn(&NetCommand) -> bool) -> NetCommand {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let command = h.cmds.recv().await.expect("엔진이 멈춤");
            if wanted(&command) {
                return command;
            }
        }
    })
    .await
    .expect("기다린 네트워크 명령이 5초 안에 오지 않음")
}

async fn peers_until(h: &Harness, ready: impl Fn(&[PeerSummary]) -> bool) -> Vec<PeerSummary> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let peers = h
                .handle
                .ask(|reply| EngineCommand::Peers { reply })
                .await
                .unwrap();
            if ready(&peers) {
                return peers;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("기다린 기기 상태가 5초 안에 되지 않음")
}

fn drain(h: &mut Harness) -> Vec<NetCommand> {
    let mut v = Vec::new();
    while let Ok(c) = h.cmds.try_recv() {
        v.push(c);
    }
    v
}

#[tokio::test]
async fn upload_to_unpaired_peer_must_not_send_file() {
    let mut h = harness(&[]);
    let file = h.root.join("secret.txt");
    std::fs::write(&file, b"secret").unwrap();
    let peer = PeerId::parse(PEER).unwrap();
    let refused = h
        .handle
        .ask(|reply| EngineCommand::Upload {
            peer: peer.clone(),
            file: file.clone(),
            remote_dir: RelativePath::root(),
            reply,
        })
        .await
        .unwrap_err();
    assert_eq!(refused.code, ErrorCode::AccessDenied);
    let sent_init = drain(&mut h).iter().any(|c| {
        matches!(
            c,
            NetCommand::SendStream {
                msg: StreamMsg::Init { .. },
                ..
            }
        )
    });
    assert!(!sent_init, "페어링 안 된 기기에 파일 Init 을 보냈다");
}

#[tokio::test]
async fn cancel_arriving_before_init_must_not_leave_partial_file() {
    let mut h = harness(&[PEER]);
    let peer = PeerId::parse(PEER).unwrap();
    let id = TransferId::from_wire("t-1".into());
    h.events
        .send(NetEvent::Inbound {
            peer: peer.clone(),
            id: InboundId(1),
            request: InboundRequest::Stream(StreamMsg::Cancel {
                transfer_id: id.clone(),
            }),
        })
        .await
        .unwrap();
    h.events
        .send(NetEvent::Inbound {
            peer,
            id: InboundId(2),
            request: InboundRequest::Stream(StreamMsg::Init {
                transfer_id: id,
                file_name: "a.txt".into(),
                total_size: 10,
                remote_path: "/".into(),
                for_pull: false,
            }),
        })
        .await
        .unwrap();
    let ack = next_command(
        &mut h,
        |c| matches!(c, NetCommand::RespondStream { inbound, .. } if *inbound == InboundId(2)),
    )
    .await;
    assert!(
        matches!(ack, NetCommand::RespondStream { ack, .. } if ack.result.is_err()),
        "취소 뒤에 온 Init 을 받아들임"
    );
    let parts: Vec<_> = std::fs::read_dir(&h.shared)
        .map(|d| {
            d.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    let parts: Vec<String> = parts
        .into_iter()
        .filter(|n: &String| n.ends_with(".part"))
        .collect();
    assert!(
        parts.is_empty(),
        "취소가 먼저 왔는데 임시 파일이 남음: {parts:?}"
    );
    assert_eq!(h.ui.count("transfer-started"), 0);
}

#[tokio::test]
async fn device_name_must_not_survive_reconnect() {
    let h = harness(&[]);
    let peer = PeerId::parse(PEER).unwrap();
    let huge = format!("\u{1b}[2J\nfake{}", "x".repeat(1024 * 1024));
    for ev in [
        NetEvent::Connected { peer: peer.clone() },
        NetEvent::DeviceInfoReplied {
            peer: peer.clone(),
            device_name: huge,
        },
        NetEvent::Disconnected { peer: peer.clone() },
        NetEvent::Connected { peer: peer.clone() },
    ] {
        h.events.send(ev).await.unwrap();
    }
    let peers = peers_until(&h, |p| p.first().is_some_and(|s| s.device_name.is_some())).await;
    let name = peers[0].device_name.clone();
    assert!(
        name.as_ref()
            .is_some_and(|n| n.chars().count() <= 64 && !n.chars().any(char::is_control)),
        "끊었다 다시 붙어도 1MiB 이름이 그대로 남음(길이 {:?})",
        name.map(|n| n.len())
    );
}

#[tokio::test]
async fn commit_failure_must_not_leave_part_file() {
    let dir = tmp("commit");
    let store = LocalFileStore::new(dir.clone());
    for n in 0..1000 {
        let name = if n == 0 {
            "a.txt".to_string()
        } else {
            format!("a ({n}).txt")
        };
        std::fs::write(dir.join(name), b"x").unwrap();
    }
    let name = FileName::parse("a.txt").unwrap();
    let mut sink = store
        .create_incoming(IncomingTarget::Downloads, &name)
        .await
        .unwrap();
    sink.write(b"data").await.unwrap();
    assert!(sink.commit().await.is_err());
    let left: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".part"))
        .collect();
    assert!(left.is_empty(), "commit 실패 뒤 임시 파일이 남음: {left:?}");
}

#[tokio::test]
async fn long_but_legal_file_name_can_be_received() {
    let dir = tmp("long");
    let store = LocalFileStore::new(dir);
    let name = FileName::parse(&format!("{}.txt", "a".repeat(240))).unwrap();
    let r = store
        .create_incoming(IncomingTarget::Downloads, &name)
        .await;
    assert!(
        r.is_ok(),
        "255자 이하 이름인데 임시 파일 이름이 길어 실패: {:?}",
        r.err()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_trust_saves_all_succeed() {
    let dir = tmp("trust");
    let store = Arc::new(JsonTrustStore::new(dir.join("trusted-peers.json")));
    let peers = vec![PeerId::parse(PEER).unwrap()];
    let mut tasks = Vec::new();
    for _ in 0..64 {
        let (s, p) = (store.clone(), peers.clone());
        tasks.push(tokio::spawn(async move { s.save(&p).await }));
    }
    let mut failed = 0;
    for t in tasks {
        if t.await.unwrap().is_err() {
            failed += 1;
        }
    }
    assert_eq!(
        failed, 0,
        "동시 저장 {failed}건 실패(임시 파일 이름이 하나뿐)"
    );
}

struct HangingNet(Arc<tokio::sync::Notify>);
impl NetworkFactory for HangingNet {
    fn start(&self) -> AppResult<NetworkLink> {
        let (commands, mut rx) = mpsc::unbounded_channel();
        let (_tx, events) = mpsc::channel(8);
        let asked_to_stop = self.0.clone();
        let task = tokio::spawn(async move {
            while let Some(command) = rx.recv().await {
                if matches!(command, NetCommand::Shutdown) {
                    asked_to_stop.notify_one();
                }
            }
            std::future::pending::<()>().await
        });
        std::mem::forget(_tx);
        Ok(NetworkLink {
            local_peer: PeerId::parse(PEER).unwrap(),
            commands,
            events,
            task,
        })
    }
}

#[tokio::test]
async fn stop_must_not_block_status_queries() {
    let root = tmp("sup");
    let deps = EngineDeps {
        ui: Arc::new(Rec::default()),
        files: Arc::new(LocalFileStore::new(root.join("d"))),
        settings: SettingsStore::new(Settings::with_home(Some(root.clone()))),
        activity: ActivityStore::default(),
        trust: Arc::new(MemTrust(vec![])),
    };
    let asked_to_stop = Arc::new(tokio::sync::Notify::new());
    let sup = Arc::new(P2pSupervisor::new(
        deps,
        Arc::new(HangingNet(asked_to_stop.clone())),
        None,
        None,
    ));
    sup.start().await.unwrap();
    let s2 = sup.clone();
    let stopper = tokio::spawn(async move { s2.stop().await });
    tokio::time::timeout(Duration::from_secs(5), asked_to_stop.notified())
        .await
        .expect("정지가 네트워크 종료 단계까지 가지 않음");
    let t = std::time::Instant::now();
    let _ = sup.is_running().await;
    let waited = t.elapsed();
    stopper.abort();
    assert!(
        waited < Duration::from_secs(1),
        "정지 중 상태 조회가 {waited:?} 막힘"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_binds_on_same_socket_only_one_wins() {
    use pebble_core::adapters::control::bind;
    let dir = tmp("bind");
    let mut both = 0;
    for i in 0..300 {
        let p: PathBuf = dir.join(format!("c{}.sock", i % 3));
        let _ = std::fs::remove_file(&p);
        let (p1, p2) = (p.clone(), p.clone());
        let a = tokio::spawn(async move { bind(&p1).await });
        let b = tokio::spawn(async move { bind(&p2).await });
        let (a, b) = (a.await.unwrap(), b.await.unwrap());
        if a.is_ok() && b.is_ok() {
            both += 1;
        }
    }
    assert_eq!(both, 0, "300번 중 {both}번 두 쪽 다 소켓을 잡음");
}

struct CountingTrust {
    load_fails: bool,
    save_fails: bool,
    saves: Mutex<usize>,
}

#[async_trait]
impl TrustStore for CountingTrust {
    async fn load(&self) -> AppResult<Vec<PeerId>> {
        if self.load_fails {
            Err(pebble_core::domain::error::AppError::new(
                pebble_core::domain::error::ErrorCode::Io,
                "broken",
            ))
        } else {
            Ok(Vec::new())
        }
    }
    async fn save(&self, _: &[PeerId]) -> AppResult<()> {
        *self.saves.lock().unwrap() += 1;
        if self.save_fails {
            Err(pebble_core::domain::error::AppError::new(
                pebble_core::domain::error::ErrorCode::Io,
                "disk full",
            ))
        } else {
            Ok(())
        }
    }
}

async fn approve_incoming_pairing(trust: Arc<CountingTrust>) -> (Harness, AppResult<()>) {
    let h = harness_with(trust);
    let peer = PeerId::parse(PEER).unwrap();
    h.events
        .send(NetEvent::Inbound {
            peer: peer.clone(),
            id: InboundId(7),
            request: InboundRequest::Pairing(PairingAsk {
                device_name: "office".into(),
                pin: "123456".into(),
            }),
        })
        .await
        .unwrap();
    peers_until(&h, |p| p.iter().any(|s| s.pending_pin.is_some())).await;
    let result = h
        .handle
        .ask(|reply| EngineCommand::RespondPairing {
            peer,
            approved: true,
            reply,
        })
        .await;
    (h, result)
}

#[tokio::test]
async fn unreadable_trust_list_is_never_overwritten() {
    let trust = Arc::new(CountingTrust {
        load_fails: true,
        save_fails: false,
        saves: Mutex::new(0),
    });
    let (mut h, result) = approve_incoming_pairing(trust.clone()).await;
    assert!(result.is_err());
    assert_eq!(*trust.saves.lock().unwrap(), 0);
    let replied_ok = drain(&mut h)
        .iter()
        .any(|c| matches!(c, NetCommand::RespondPairing { reply, .. } if reply.approved));
    assert!(!replied_ok);
}

#[tokio::test]
async fn failed_trust_save_rejects_pairing() {
    let trust = Arc::new(CountingTrust {
        load_fails: false,
        save_fails: true,
        saves: Mutex::new(0),
    });
    let (h, result) = approve_incoming_pairing(trust.clone()).await;
    assert!(result.is_err());
    assert_eq!(*trust.saves.lock().unwrap(), 1);
    let peers = h
        .handle
        .ask(|reply| EngineCommand::Peers { reply })
        .await
        .unwrap();
    assert!(peers.iter().all(|p| !p.trusted));
    assert_eq!(h.ui.count("pairing-approved"), 0);
}

#[derive(Default)]
struct FlakyServer {
    register_failures: Mutex<u32>,
    registers: Mutex<u32>,
    syncs: Mutex<u32>,
    heartbeats: Mutex<u32>,
}

#[async_trait]
impl ServerApi for FlakyServer {
    async fn register(&self, peer: &PeerId, _: &str) -> AppResult<DeviceCredentials> {
        *self.registers.lock().unwrap() += 1;
        let mut failures = self.register_failures.lock().unwrap();
        if *failures > 0 {
            *failures -= 1;
            return Err(pebble_core::domain::error::AppError::new(
                pebble_core::domain::error::ErrorCode::Network,
                "down",
            ));
        }
        Ok(DeviceCredentials {
            peer_id: peer.clone(),
            device_db_id: "1".into(),
            device_secret: "s".into(),
        })
    }
    async fn heartbeat(&self, _: &DeviceCredentials) -> AppResult<()> {
        *self.heartbeats.lock().unwrap() += 1;
        Ok(())
    }
    async fn sync_files(&self, _: &DeviceCredentials, _: Vec<FileSnapshot>) -> AppResult<()> {
        *self.syncs.lock().unwrap() += 1;
        Ok(())
    }
}

struct NoCredentials;

impl CredentialStore for NoCredentials {
    fn load(&self, _: &PeerId) -> AppResult<Option<DeviceCredentials>> {
        Ok(None)
    }
    fn save(&self, _: &DeviceCredentials) -> AppResult<()> {
        Ok(())
    }
}

#[tokio::test(start_paused = true)]
async fn server_sync_retries_registration_then_syncs_and_stops() {
    let h = harness(&[]);
    let server = Arc::new(FlakyServer {
        register_failures: Mutex::new(2),
        ..FlakyServer::default()
    });
    let root = tmp("server");
    let mut settings = Settings::with_home(Some(root.clone()));
    settings.shared_folder = root.clone();
    let deps = ServerDeps {
        api: server.clone(),
        credentials: Arc::new(NoCredentials),
        files: Arc::new(LocalFileStore::new(root.join("downloads"))),
        settings: SettingsStore::new(settings),
    };
    let (stop, stop_rx) = watch::channel(false);
    let (_sync, sync_rx) = mpsc::channel(1);
    let task = spawn_server_sync(
        deps,
        PeerId::parse(PEER).unwrap(),
        h.handle.clone(),
        stop_rx,
        sync_rx,
    );
    tokio::time::sleep(Duration::from_secs(20)).await;
    assert_eq!(*server.registers.lock().unwrap(), 3);
    assert_eq!(*server.syncs.lock().unwrap(), 1);
    assert!(*server.heartbeats.lock().unwrap() >= 1);
    stop.send(true).unwrap();
    tokio::time::timeout(Duration::from_secs(1), task)
        .await
        .expect("멈춤 신호 뒤에도 서버 작업이 끝나지 않음")
        .unwrap();
}

#[tokio::test]
async fn unpaired_sender_is_refused_by_receiver() {
    let mut h = harness(&[]);
    let peer = PeerId::parse(PEER).unwrap();
    h.events
        .send(NetEvent::Inbound {
            peer,
            id: InboundId(3),
            request: InboundRequest::Stream(StreamMsg::Init {
                transfer_id: TransferId::from_wire("t-9".into()),
                file_name: "intruder.txt".into(),
                total_size: 4,
                remote_path: "/".into(),
                for_pull: false,
            }),
        })
        .await
        .unwrap();
    let ack = next_command(&mut h, |c| matches!(c, NetCommand::RespondStream { .. })).await;
    assert!(
        matches!(ack, NetCommand::RespondStream { ack, .. } if ack.result == Err("페어링이 필요합니다.".to_string()))
    );
    assert!(!h.shared.join("intruder.txt").exists());
    assert_eq!(h.ui.count("transfer-started"), 0);
}

const PEER_B: &str = "12D3KooWBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB";

fn payloads(ui: &Rec, name: &str) -> Vec<serde_json::Value> {
    ui.0.lock()
        .unwrap()
        .iter()
        .filter(|e| e["event"] == name)
        .map(|e| e["payload"].clone())
        .collect()
}

fn event_names(ui: &Rec) -> Vec<String> {
    ui.0.lock()
        .unwrap()
        .iter()
        .map(|e| e["event"].as_str().unwrap_or_default().to_string())
        .collect()
}

fn part_files(dir: &std::path::Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|d| {
            d.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.ends_with(".part"))
                .collect()
        })
        .unwrap_or_default()
}

fn wire(raw: &str) -> TransferId {
    TransferId::from_wire(raw.to_string())
}

struct ListedTrust {
    initial: Vec<PeerId>,
    load_fails: bool,
    save_fails: bool,
    saved: Mutex<Vec<Vec<PeerId>>>,
}

impl ListedTrust {
    fn new(initial: &[&str], load_fails: bool, save_fails: bool) -> Arc<Self> {
        Arc::new(Self {
            initial: initial.iter().map(|p| PeerId::parse(p).unwrap()).collect(),
            load_fails,
            save_fails,
            saved: Mutex::new(Vec::new()),
        })
    }

    fn saves(&self) -> Vec<Vec<PeerId>> {
        self.saved.lock().unwrap().clone()
    }
}

#[async_trait]
impl TrustStore for ListedTrust {
    async fn load(&self) -> AppResult<Vec<PeerId>> {
        if self.load_fails {
            Err(AppError::new(ErrorCode::Io, "broken"))
        } else {
            Ok(self.initial.clone())
        }
    }
    async fn save(&self, peers: &[PeerId]) -> AppResult<()> {
        self.saved.lock().unwrap().push(peers.to_vec());
        if self.save_fails {
            Err(AppError::new(ErrorCode::Io, "disk full"))
        } else {
            Ok(())
        }
    }
}

async fn unpair(h: &Harness, peer: &str) -> AppResult<()> {
    let peer = PeerId::parse(peer).unwrap();
    h.handle
        .ask(|reply| EngineCommand::Unpair { peer, reply })
        .await
}

async fn connect(h: &Harness, peer: &str) {
    h.events
        .send(NetEvent::Connected {
            peer: PeerId::parse(peer).unwrap(),
        })
        .await
        .unwrap();
    peers_until(h, |p| {
        p.iter().any(|s| s.peer_id.as_str() == peer && s.connected)
    })
    .await;
}

fn summary_of(peers: &[PeerSummary], peer: &str) -> PeerSummary {
    peers
        .iter()
        .find(|p| p.peer_id.as_str() == peer)
        .cloned()
        .unwrap_or_else(|| panic!("기기 목록에 {peer} 가 없음"))
}

#[tokio::test]
async fn ac_d4_4_1_unpair_untrusts_the_peer_and_saves_the_remaining_list_once() {
    let trust = ListedTrust::new(&[PEER, PEER_B], false, false);
    let h = harness_with(trust.clone());
    connect(&h, PEER).await;
    unpair(&h, PEER)
        .await
        .expect("믿는 기기 해제가 성공해야 함");
    let peers = peers_until(&h, |p| p.iter().any(|s| s.peer_id.as_str() == PEER)).await;
    assert!(!summary_of(&peers, PEER).trusted);
    assert!(summary_of(&peers, PEER_B).trusted);
    assert_eq!(trust.saves(), vec![vec![PeerId::parse(PEER_B).unwrap()]]);
}

#[tokio::test]
async fn ac_d4_4_2_unpair_of_an_untrusted_peer_is_not_found_and_saves_nothing() {
    let trust = ListedTrust::new(&[PEER_B], false, false);
    let h = harness_with(trust.clone());
    let err = unpair(&h, PEER).await.unwrap_err();
    assert_eq!(err.code, ErrorCode::NotFound);
    assert_eq!(err.message, "믿는 기기가 아닙니다.");
    assert!(trust.saves().is_empty());
}

#[tokio::test]
async fn ac_d4_4_3_failed_save_is_io_and_the_peer_stays_trusted() {
    let trust = ListedTrust::new(&[PEER], false, true);
    let h = harness_with(trust.clone());
    connect(&h, PEER).await;
    let err = unpair(&h, PEER).await.unwrap_err();
    assert_eq!(err.code, ErrorCode::Io);
    assert_eq!(trust.saves().len(), 1);
    let peers = peers_until(&h, |p| p.iter().any(|s| s.peer_id.as_str() == PEER)).await;
    assert!(summary_of(&peers, PEER).trusted);
}

#[tokio::test]
async fn ac_d4_4_4_unreadable_trust_list_blocks_unpair_with_io_and_never_saves() {
    let trust = ListedTrust::new(&[], true, false);
    let h = harness_with(trust.clone());
    let err = unpair(&h, PEER).await.unwrap_err();
    assert_eq!(err.code, ErrorCode::Io);
    assert!(trust.saves().is_empty());
}

#[tokio::test]
async fn ac_d4_4_5_after_unpair_upload_download_and_file_list_are_refused() {
    let mut h = harness_with(ListedTrust::new(&[PEER], false, false));
    let file = h.root.join("secret.txt");
    std::fs::write(&file, b"secret").unwrap();
    unpair(&h, PEER).await.expect("해제가 성공해야 함");
    drain(&mut h);
    let peer = PeerId::parse(PEER).unwrap();
    let upload = h
        .handle
        .ask(|reply| EngineCommand::Upload {
            peer: peer.clone(),
            file: file.clone(),
            remote_dir: RelativePath::root(),
            reply,
        })
        .await
        .unwrap_err();
    assert_eq!(upload.code, ErrorCode::AccessDenied);
    let download = h
        .handle
        .ask(|reply| EngineCommand::Download {
            peer: peer.clone(),
            path: RelativePath::parse("/x.txt").unwrap(),
            reply,
        })
        .await
        .unwrap_err();
    assert_eq!(download.code, ErrorCode::AccessDenied);
    let sent_before_listing = drain(&mut h)
        .iter()
        .any(|c| matches!(c, NetCommand::SendStream { .. }));
    assert!(
        !sent_before_listing,
        "거절된 보내기·받기가 스트림 메시지를 보냄"
    );
    h.events
        .send(NetEvent::Inbound {
            peer,
            id: InboundId(21),
            request: InboundRequest::FileList { path: "/".into() },
        })
        .await
        .unwrap();
    let listed = next_command(
        &mut h,
        |c| matches!(c, NetCommand::RespondFileList { inbound, .. } if *inbound == InboundId(21)),
    )
    .await;
    match listed {
        NetCommand::RespondFileList { reply, .. } => {
            assert_eq!(reply.error.as_deref(), Some("페어링이 필요합니다."));
            assert!(reply.files.is_empty());
        }
        _ => unreachable!(),
    }
    let sent_stream = drain(&mut h)
        .iter()
        .any(|c| matches!(c, NetCommand::SendStream { .. }));
    assert!(!sent_stream, "해제된 기기에 스트림 메시지를 보냄");
}

#[tokio::test]
async fn ac_d4_4_6_unpair_fails_transfers_with_that_peer_and_tells_the_peer_to_cancel() {
    let mut h = harness_with(ListedTrust::new(&[PEER], false, false));
    let file = h.root.join("up.bin");
    std::fs::write(&file, vec![7u8; 10]).unwrap();
    let peer = PeerId::parse(PEER).unwrap();
    let uploading = h
        .handle
        .ask(|reply| EngineCommand::Upload {
            peer: peer.clone(),
            file: file.clone(),
            remote_dir: RelativePath::root(),
            reply,
        })
        .await
        .unwrap();
    let pulling = h
        .handle
        .ask(|reply| EngineCommand::Download {
            peer: peer.clone(),
            path: RelativePath::parse("/x.txt").unwrap(),
            reply,
        })
        .await
        .unwrap();
    drain(&mut h);
    unpair(&h, PEER).await.expect("해제가 성공해야 함");
    let cancelled: Vec<TransferId> = drain(&mut h)
        .into_iter()
        .filter_map(|c| match c {
            NetCommand::SendStream {
                msg: StreamMsg::Cancel { transfer_id },
                ..
            } => Some(transfer_id),
            _ => None,
        })
        .collect();
    assert!(cancelled.contains(&uploading), "{cancelled:?}");
    assert!(cancelled.contains(&pulling), "{cancelled:?}");
    let failed = payloads(&h.ui, "transfer-failed");
    assert_eq!(failed.len(), 2, "{failed:?}");
    assert!(failed
        .iter()
        .all(|f| f["error"] == "페어링이 해제되었습니다."));
}

#[tokio::test]
async fn ac_d4_4_7_unpair_rejects_a_pending_pin_and_clears_it() {
    let mut h = harness_with(ListedTrust::new(&[PEER], false, false));
    connect(&h, PEER).await;
    h.events
        .send(NetEvent::Inbound {
            peer: PeerId::parse(PEER).unwrap(),
            id: InboundId(7),
            request: InboundRequest::Pairing(PairingAsk {
                device_name: "office".into(),
                pin: "123456".into(),
            }),
        })
        .await
        .unwrap();
    peers_until(&h, |p| p.iter().any(|s| s.pending_pin.is_some())).await;
    drain(&mut h);
    unpair(&h, PEER).await.expect("해제가 성공해야 함");
    let rejected = drain(&mut h).iter().any(|c| {
        matches!(c, NetCommand::RespondPairing { inbound, reply }
            if *inbound == InboundId(7) && !reply.approved)
    });
    assert!(rejected, "대기 중인 PIN 을 거절하지 않음");
    let peers = peers_until(&h, |p| p.iter().any(|s| s.peer_id.as_str() == PEER)).await;
    assert!(summary_of(&peers, PEER).pending_pin.is_none());
}

#[tokio::test]
async fn ac_d4_4_8_pairing_again_after_unpair_trusts_the_peer_again() {
    let trust = ListedTrust::new(&[PEER], false, false);
    let h = harness_with(trust.clone());
    let peer = PeerId::parse(PEER).unwrap();
    unpair(&h, PEER).await.expect("해제가 성공해야 함");
    h.handle
        .ask(|reply| EngineCommand::RequestPairing {
            peer: peer.clone(),
            reply,
        })
        .await
        .unwrap();
    h.events
        .send(NetEvent::PairingReplied {
            peer,
            reply: PairingReply {
                approved: true,
                device_name: "office".into(),
                shared_folder_name: None,
                error: None,
            },
        })
        .await
        .unwrap();
    let peers = peers_until(&h, |p| {
        p.iter().any(|s| s.peer_id.as_str() == PEER && s.trusted)
    })
    .await;
    assert!(summary_of(&peers, PEER).trusted);
    assert_eq!(
        trust.saves(),
        vec![Vec::new(), vec![PeerId::parse(PEER).unwrap()]]
    );
}

struct Gate {
    release: watch::Sender<bool>,
    entered: watch::Sender<usize>,
    fail: AtomicBool,
    panics: AtomicBool,
}

impl Gate {
    fn blocking() -> Arc<Self> {
        Arc::new(Self {
            release: watch::channel(false).0,
            entered: watch::channel(0).0,
            fail: AtomicBool::new(false),
            panics: AtomicBool::new(false),
        })
    }

    fn failing() -> Arc<Self> {
        Arc::new(Self {
            release: watch::channel(true).0,
            entered: watch::channel(0).0,
            fail: AtomicBool::new(true),
            panics: AtomicBool::new(false),
        })
    }

    fn panicking() -> Arc<Self> {
        Arc::new(Self {
            release: watch::channel(true).0,
            entered: watch::channel(0).0,
            fail: AtomicBool::new(false),
            panics: AtomicBool::new(true),
        })
    }

    fn open(&self) {
        self.release.send_replace(true);
    }

    async fn entered(&self, count: usize) {
        let mut rx = self.entered.subscribe();
        tokio::time::timeout(Duration::from_secs(5), rx.wait_for(|n| *n >= count))
            .await
            .expect("commit 이 시작되지 않음")
            .unwrap();
    }
}

struct GateStore {
    inner: LocalFileStore,
    gate: Arc<Gate>,
}

struct GateSink {
    inner: Box<dyn ChunkSink>,
    gate: Arc<Gate>,
}

#[async_trait]
impl ChunkSink for GateSink {
    async fn write(&mut self, data: &[u8]) -> AppResult<()> {
        self.inner.write(data).await
    }

    async fn commit(self: Box<Self>) -> AppResult<PathBuf> {
        let GateSink { inner, gate } = *self;
        gate.entered.send_modify(|n| *n += 1);
        let mut rx = gate.release.subscribe();
        let _ = rx.wait_for(|open| *open).await;
        if gate.panics.load(Ordering::SeqCst) {
            panic!("gate commit panicked");
        }
        if gate.fail.load(Ordering::SeqCst) {
            drop(inner);
            return Err(AppError::new(ErrorCode::Io, "gate commit failed"));
        }
        inner.commit().await
    }

    async fn abort(self: Box<Self>) {
        self.inner.abort().await
    }
}

#[async_trait]
impl FileStore for GateStore {
    async fn ensure_dir(&self, dir: &std::path::Path) -> AppResult<()> {
        self.inner.ensure_dir(dir).await
    }
    async fn list_dir(
        &self,
        root: &std::path::Path,
        rel: &RelativePath,
    ) -> AppResult<Vec<FileEntry>> {
        self.inner.list_dir(root, rel).await
    }
    async fn folder_stats(&self, root: &std::path::Path) -> AppResult<FolderStats> {
        self.inner.folder_stats(root).await
    }
    async fn snapshot_top_level(&self, root: &std::path::Path) -> AppResult<Vec<FileSnapshot>> {
        self.inner.snapshot_top_level(root).await
    }
    async fn open_shared(
        &self,
        root: &std::path::Path,
        rel: &RelativePath,
    ) -> AppResult<OpenedFile> {
        self.inner.open_shared(root, rel).await
    }
    async fn open_local(&self, path: &std::path::Path) -> AppResult<OpenedFile> {
        self.inner.open_local(path).await
    }
    async fn create_incoming(
        &self,
        target: IncomingTarget<'_>,
        name: &FileName,
    ) -> AppResult<Box<dyn ChunkSink>> {
        let inner = self.inner.create_incoming(target, name).await?;
        Ok(Box::new(GateSink {
            inner,
            gate: self.gate.clone(),
        }))
    }
}

struct GatedEngine {
    h: Harness,
    task: JoinHandle<()>,
    gate: Arc<Gate>,
    activity: ActivityStore,
}

fn gated_engine(gate: Arc<Gate>) -> GatedEngine {
    let root = tmp("gated");
    let shared = root.join("shared");
    std::fs::create_dir_all(&shared).unwrap();
    let ui = Arc::new(Rec::default());
    let mut s = Settings::with_home(Some(root.clone()));
    s.shared_folder = shared.clone();
    let activity = ActivityStore::default();
    let deps = EngineDeps {
        ui: ui.clone(),
        files: Arc::new(GateStore {
            inner: LocalFileStore::new(root.join("downloads")),
            gate: gate.clone(),
        }),
        settings: SettingsStore::new(s),
        activity: activity.clone(),
        trust: Arc::new(MemTrust(vec![PeerId::parse(PEER).unwrap()])),
    };
    let (net, cmds) = mpsc::unbounded_channel();
    let (events, rx) = mpsc::channel(256);
    let (handle, task) = spawn_engine(deps, net, rx);
    GatedEngine {
        h: Harness {
            handle,
            cmds,
            events,
            ui,
            shared,
            root,
        },
        task,
        gate,
        activity,
    }
}

async fn inbound_stream(h: &Harness, id: u64, msg: StreamMsg) {
    h.events
        .send(NetEvent::Inbound {
            peer: PeerId::parse(PEER).unwrap(),
            id: InboundId(id),
            request: InboundRequest::Stream(msg),
        })
        .await
        .unwrap();
}

fn init_msg(id: &str, name: &str, size: u64) -> StreamMsg {
    StreamMsg::Init {
        transfer_id: wire(id),
        file_name: name.into(),
        total_size: size,
        remote_path: "/".into(),
        for_pull: false,
    }
}

fn chunk_msg(id: &str, index: u32, data: &[u8]) -> StreamMsg {
    StreamMsg::Chunk {
        transfer_id: wire(id),
        index,
        data: data.to_vec(),
    }
}

async fn ack_for(h: &mut Harness, inbound: u64) -> Result<(), String> {
    let found = next_command(
        h,
        |c| matches!(c, NetCommand::RespondStream { inbound: i, .. } if *i == InboundId(inbound)),
    )
    .await;
    match found {
        NetCommand::RespondStream { ack, .. } => ack.result,
        _ => unreachable!(),
    }
}

async fn start_incoming(h: &mut Harness, id: &str, name: &str, size: u64, inbound: u64) {
    inbound_stream(h, inbound, init_msg(id, name, size)).await;
    assert_eq!(ack_for(h, inbound).await, Ok(()), "Init 거절됨");
}

async fn within_one_second<T>(what: &str, fut: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(1), fut)
        .await
        .unwrap_or_else(|_| panic!("{what}: 1초 안에 답하지 않음(엔진이 commit 에 막힘)"))
}

async fn peers_now(h: &Harness) -> Vec<PeerSummary> {
    within_one_second(
        "peers",
        h.handle.ask(|reply| EngineCommand::Peers { reply }),
    )
    .await
    .unwrap()
}

async fn state_now(h: &Harness, id: &str) -> TransferState {
    let transfer_id = wire(id);
    within_one_second(
        "transfer_state",
        h.handle
            .ask(|reply| EngineCommand::TransferState { transfer_id, reply }),
    )
    .await
    .unwrap()
}

async fn wait_event_count(h: &Harness, name: &str, count: usize) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while h.ui.count(name) < count {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("{name} 이벤트 {count}건이 5초 안에 오지 않음"));
}

#[tokio::test]
async fn ac_d1_1_commit_blocked_does_not_block_engine() {
    let mut g = gated_engine(Gate::blocking());
    start_incoming(&mut g.h, "a", "a.txt", 4, 10).await;
    inbound_stream(&g.h, 11, chunk_msg("a", 0, b"AAAA")).await;
    g.gate.entered(1).await;
    let _ = peers_now(&g.h).await;
    assert!(matches!(state_now(&g.h, "a").await, TransferState::Active));
    inbound_stream(
        &g.h,
        20,
        init_msg(
            "b",
            "b.txt",
            2 * pebble_core::domain::transfer::CHUNK_SIZE as u64,
        ),
    )
    .await;
    assert_eq!(
        within_one_second("B Init ack", ack_for(&mut g.h, 20)).await,
        Ok(())
    );
    let first_chunk = vec![b'B'; pebble_core::domain::transfer::CHUNK_SIZE];
    inbound_stream(&g.h, 21, chunk_msg("b", 0, &first_chunk)).await;
    assert_eq!(
        within_one_second("B 청크 ack", ack_for(&mut g.h, 21)).await,
        Ok(())
    );
    g.gate.open();
    wait_event_count(&g.h, "transfer-completed", 1).await;
}

#[tokio::test]
async fn ac_d1_2_transfer_state_is_active_while_committing_then_ended_completed() {
    let mut g = gated_engine(Gate::blocking());
    start_incoming(&mut g.h, "a", "a.txt", 4, 10).await;
    inbound_stream(&g.h, 11, chunk_msg("a", 0, b"AAAA")).await;
    g.gate.entered(1).await;
    assert!(matches!(state_now(&g.h, "a").await, TransferState::Active));
    g.gate.open();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if matches!(
                state_now(&g.h, "a").await,
                TransferState::Ended(UiEvent::TransferCompleted { .. })
            ) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("저장이 끝났는데 Ended(Completed) 가 되지 않음");
}

#[tokio::test]
async fn ac_d1_3_last_chunk_is_acked_only_after_commit_and_events_keep_order() {
    let mut g = gated_engine(Gate::blocking());
    start_incoming(&mut g.h, "a", "a.txt", 4, 10).await;
    inbound_stream(&g.h, 11, chunk_msg("a", 0, b"AAAA")).await;
    g.gate.entered(1).await;
    let _ = state_now(&g.h, "a").await;
    let early_ack = drain(&mut g.h).iter().any(
        |c| matches!(c, NetCommand::RespondStream { inbound, .. } if *inbound == InboundId(11)),
    );
    assert!(!early_ack, "commit 이 끝나기 전에 마지막 청크를 ack 함");
    assert_eq!(g.h.ui.count("transfer-completed"), 0);
    g.gate.open();
    assert_eq!(ack_for(&mut g.h, 11).await, Ok(()));
    let names = event_names(&g.h.ui);
    let progress = names
        .iter()
        .rposition(|n| n == "transfer-progress")
        .unwrap();
    let completed = names
        .iter()
        .position(|n| n == "transfer-completed")
        .unwrap();
    assert!(progress < completed, "{names:?}");
    assert_eq!(g.h.ui.count("transfer-completed"), 1);
}

#[tokio::test]
async fn ac_d1_4_commit_failure_is_acked_reported_recorded_and_leaves_no_part_file() {
    let mut g = gated_engine(Gate::failing());
    start_incoming(&mut g.h, "a", "a.txt", 4, 10).await;
    inbound_stream(&g.h, 11, chunk_msg("a", 0, b"AAAA")).await;
    let ack = ack_for(&mut g.h, 11).await;
    assert!(ack.is_err(), "commit 실패인데 ack 가 성공");
    assert_eq!(g.h.ui.count("transfer-failed"), 1);
    assert_eq!(g.h.ui.count("transfer-completed"), 0);
    let records: Vec<ActivityRecord> = g.activity.snapshot();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].status, TransferStatus::Failed);
    assert!(part_files(&g.h.shared).is_empty());
    assert!(!g.h.shared.join("a.txt").exists());
    assert!(matches!(
        state_now(&g.h, "a").await,
        TransferState::Ended(UiEvent::TransferFailed { .. })
    ));
}

#[tokio::test]
async fn ac_d1_5a_shutdown_during_commit_is_handled_at_once_and_waits_for_the_commit() {
    let mut g = gated_engine(Gate::blocking());
    start_incoming(&mut g.h, "a", "a.txt", 4, 10).await;
    start_incoming(&mut g.h, "b", "b.txt", 8, 20).await;
    inbound_stream(&g.h, 11, chunk_msg("a", 0, b"AAAA")).await;
    g.gate.entered(1).await;
    g.h.handle.send(EngineCommand::Shutdown).await.unwrap();
    tokio::time::timeout(Duration::from_secs(1), async {
        while g.h.ui.count("transfer-failed") < 1 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("저장 중에도 Shutdown 을 바로 처리해 다른 전송을 정리해야 함");
    assert_eq!(
        g.h.ui.count("p2p-stopped"),
        0,
        "commit 을 기다리지 않고 멈춤"
    );
    g.gate.open();
    tokio::time::timeout(Duration::from_secs(5), g.task)
        .await
        .expect("엔진이 끝나지 않음")
        .unwrap();
    let names = event_names(&g.h.ui);
    let completed = names
        .iter()
        .position(|n| n == "transfer-completed")
        .unwrap();
    let stopped = names.iter().position(|n| n == "p2p-stopped").unwrap();
    assert!(completed < stopped, "{names:?}");
    assert_eq!(g.h.ui.count("transfer-failed"), 1);
}

#[tokio::test]
async fn ac_d1_5b_commit_that_never_finishes_is_failed_with_engine_stopped_after_three_seconds() {
    let mut g = gated_engine(Gate::blocking());
    start_incoming(&mut g.h, "a", "a.txt", 4, 10).await;
    inbound_stream(&g.h, 11, chunk_msg("a", 0, b"AAAA")).await;
    g.gate.entered(1).await;
    tokio::time::pause();
    let shutdown_at = tokio::time::Instant::now();
    g.h.handle.send(EngineCommand::Shutdown).await.unwrap();
    let ended = tokio::time::timeout(Duration::from_secs(10), g.task)
        .await
        .expect("영영 안 끝나는 commit 때문에 엔진이 종료되지 않음");
    ended.expect("엔진 태스크가 패닉 없이 끝나야 함");
    let waited = shutdown_at.elapsed();
    assert!(
        waited >= Duration::from_secs(3) && waited <= Duration::from_secs(4),
        "commit 을 기다린 시간이 3~4초가 아님: {waited:?}"
    );
    let records = g.activity.snapshot();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].status, TransferStatus::Failed);
    let failed = payloads(&g.h.ui, "transfer-failed");
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert_eq!(failed[0]["error"], "P2P 엔진이 종료되어 전송을 멈췄습니다.");
    assert_eq!(g.h.ui.count("transfer-completed"), 0);
    assert_eq!(g.h.ui.count("p2p-stopped"), 1);
    assert!(part_files(&g.h.shared).is_empty());
}

#[tokio::test]
async fn ac_d1_6_two_commits_of_the_same_name_in_parallel_get_distinct_files() {
    let mut g = gated_engine(Gate::blocking());
    start_incoming(&mut g.h, "a", "a.txt", 4, 10).await;
    start_incoming(&mut g.h, "b", "a.txt", 4, 20).await;
    inbound_stream(&g.h, 11, chunk_msg("a", 0, b"AAAA")).await;
    inbound_stream(&g.h, 21, chunk_msg("b", 0, b"BBBB")).await;
    g.gate.entered(2).await;
    g.gate.open();
    wait_event_count(&g.h, "transfer-completed", 2).await;
    let first = std::fs::read(g.h.shared.join("a.txt")).unwrap();
    let second = std::fs::read(g.h.shared.join("a (1).txt")).unwrap();
    let mut both = vec![first, second];
    both.sort();
    assert_eq!(both, vec![b"AAAA".to_vec(), b"BBBB".to_vec()]);
    assert_eq!(g.h.ui.count("transfer-failed"), 0);
    assert!(part_files(&g.h.shared).is_empty());
}

#[tokio::test]
async fn ac_d1_7_cancel_and_disconnect_during_commit_still_end_as_one_completed() {
    let mut g = gated_engine(Gate::blocking());
    start_incoming(&mut g.h, "a", "a.txt", 4, 10).await;
    inbound_stream(&g.h, 11, chunk_msg("a", 0, b"AAAA")).await;
    g.gate.entered(1).await;
    inbound_stream(
        &g.h,
        30,
        StreamMsg::Cancel {
            transfer_id: wire("a"),
        },
    )
    .await;
    assert_eq!(
        within_one_second("상대 Cancel ack", ack_for(&mut g.h, 30)).await,
        Ok(())
    );
    g.h.handle
        .send(EngineCommand::Cancel {
            transfer_id: wire("a"),
        })
        .await
        .unwrap();
    g.h.events
        .send(NetEvent::Disconnected {
            peer: PeerId::parse(PEER).unwrap(),
        })
        .await
        .unwrap();
    wait_event_count(&g.h, "connection-closed", 1).await;
    assert_eq!(g.h.ui.count("transfer-failed"), 0);
    assert_eq!(g.h.ui.count("transfer-cancelled"), 0);
    g.gate.open();
    wait_event_count(&g.h, "transfer-completed", 1).await;
    assert_eq!(ack_for(&mut g.h, 11).await, Ok(()));
    let _ = peers_now(&g.h).await;
    assert_eq!(g.h.ui.count("transfer-completed"), 1);
    assert_eq!(g.h.ui.count("transfer-failed"), 0);
    assert_eq!(g.h.ui.count("transfer-cancelled"), 0);
    assert_eq!(g.activity.snapshot().len(), 1);
    assert!(g.h.shared.join("a.txt").exists());
}

#[tokio::test]
async fn ac_d1_7_zero_byte_file_commit_is_offloaded_too() {
    let mut g = gated_engine(Gate::blocking());
    inbound_stream(&g.h, 40, init_msg("z", "empty.txt", 0)).await;
    g.gate.entered(1).await;
    let _ = peers_now(&g.h).await;
    assert!(matches!(state_now(&g.h, "z").await, TransferState::Active));
    let early_ack = drain(&mut g.h).iter().any(
        |c| matches!(c, NetCommand::RespondStream { inbound, .. } if *inbound == InboundId(40)),
    );
    assert!(!early_ack, "0바이트 파일을 commit 전에 ack 함");
    g.gate.open();
    assert_eq!(ack_for(&mut g.h, 40).await, Ok(()));
    assert_eq!(g.h.ui.count("transfer-completed"), 1);
    assert_eq!(
        std::fs::metadata(g.h.shared.join("empty.txt"))
            .unwrap()
            .len(),
        0
    );
}

#[tokio::test]
async fn ac_d1_8_panicking_commit_task_fails_the_transfer_and_is_acked_with_error() {
    let mut g = gated_engine(Gate::panicking());
    start_incoming(&mut g.h, "a", "a.txt", 4, 10).await;
    inbound_stream(&g.h, 11, chunk_msg("a", 0, b"AAAA")).await;
    let ack = ack_for(&mut g.h, 11).await;
    assert!(ack.is_err(), "commit 이 패닉했는데 ack 가 성공");
    assert_eq!(g.h.ui.count("transfer-failed"), 1);
    assert_eq!(g.h.ui.count("transfer-completed"), 0);
    let records = g.activity.snapshot();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].status, TransferStatus::Failed);
    assert!(matches!(
        state_now(&g.h, "a").await,
        TransferState::Ended(UiEvent::TransferFailed { .. })
    ));
    assert!(part_files(&g.h.shared).is_empty());
}

#[tokio::test]
async fn ac_d4_4_6_unpair_fails_an_incoming_transfer_and_tells_the_peer_to_cancel() {
    let mut g = gated_engine(Gate::blocking());
    start_incoming(
        &mut g.h,
        "i",
        "i.txt",
        2 * pebble_core::domain::transfer::CHUNK_SIZE as u64,
        10,
    )
    .await;
    let first_chunk = vec![b'I'; pebble_core::domain::transfer::CHUNK_SIZE];
    inbound_stream(&g.h, 11, chunk_msg("i", 0, &first_chunk)).await;
    assert_eq!(ack_for(&mut g.h, 11).await, Ok(()));
    drain(&mut g.h);
    unpair(&g.h, PEER).await.expect("해제가 성공해야 함");
    let cancelled: Vec<TransferId> = drain(&mut g.h)
        .into_iter()
        .filter_map(|c| match c {
            NetCommand::SendStream {
                msg: StreamMsg::Cancel { transfer_id },
                ..
            } => Some(transfer_id),
            _ => None,
        })
        .collect();
    assert_eq!(cancelled, vec![wire("i")]);
    let failed = payloads(&g.h.ui, "transfer-failed");
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert_eq!(failed[0]["error"], "페어링이 해제되었습니다.");
    assert!(part_files(&g.h.shared).is_empty());
}

#[tokio::test]
async fn break_double_unpair_concurrently_saves_once() {
    for round in 0..50 {
        let trust = ListedTrust::new(&[PEER], false, false);
        let h = harness_with(trust.clone());
        let (first, second) = tokio::join!(unpair(&h, PEER), unpair(&h, PEER));
        let mut results = [first, second];
        results.sort_by_key(|r| r.is_err());
        assert!(results[0].is_ok(), "{round}회차: 둘 다 실패");
        let err = results[1].as_ref().unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound, "{round}회차");
        assert_eq!(trust.saves().len(), 1, "{round}회차: 저장이 한 번이 아님");
    }
}

#[tokio::test]
async fn break_shutdown_and_commit_finish_race_emits_exactly_one_terminal_event() {
    for round in 0..100 {
        let mut g = gated_engine(Gate::blocking());
        start_incoming(&mut g.h, "a", "a.txt", 4, 10).await;
        inbound_stream(&g.h, 11, chunk_msg("a", 0, b"AAAA")).await;
        g.gate.entered(1).await;
        g.gate.open();
        g.h.handle.send(EngineCommand::Shutdown).await.unwrap();
        tokio::time::timeout(Duration::from_secs(6), g.task)
            .await
            .unwrap_or_else(|_| panic!("{round}회차: 엔진이 6초 안에 끝나지 않음"))
            .unwrap();
        let terminal = g.h.ui.count("transfer-completed") + g.h.ui.count("transfer-failed");
        assert_eq!(terminal, 1, "{round}회차: 종료 이벤트가 한 건이 아님");
        assert_eq!(g.h.ui.count("p2p-stopped"), 1, "{round}회차");
        let acks = drain(&mut g.h)
            .iter()
            .filter(|c| {
                matches!(c, NetCommand::RespondStream { inbound, .. } if *inbound == InboundId(11))
            })
            .count();
        assert_eq!(acks, 1, "{round}회차: 마지막 청크 ack 가 한 번이 아님");
        let leftovers = part_files(&g.h.shared);
        assert!(leftovers.is_empty(), "{round}회차: {leftovers:?}");
    }
}

#[tokio::test]
async fn unpair_during_commit_still_completes_the_saved_file() {
    let mut g = gated_engine(Gate::blocking());
    start_incoming(&mut g.h, "a", "a.txt", 4, 10).await;
    inbound_stream(&g.h, 11, chunk_msg("a", 0, b"AAAA")).await;
    g.gate.entered(1).await;
    unpair(&g.h, PEER).await.expect("unpair");
    g.gate.open();
    let ack = ack_for(&mut g.h, 11).await;
    let _ = peers_now(&g.h).await;
    assert_eq!(ack, Ok(()));
    assert_eq!(g.h.ui.count("transfer-completed"), 1);
    assert_eq!(g.h.ui.count("transfer-failed"), 0);
    assert!(g.h.shared.join("a.txt").exists());
}
