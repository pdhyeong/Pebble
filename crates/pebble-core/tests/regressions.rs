use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use pebble_core::adapters::fs::{JsonTrustStore, LocalFileStore};
use pebble_core::application::engine::{spawn_engine, EngineCommand, EngineDeps, EngineHandle};
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
use pebble_core::domain::error::AppResult;
use pebble_core::domain::files::FileSnapshot;
use pebble_core::domain::path::{FileName, RelativePath};
use pebble_core::domain::peer::PeerId;
use pebble_core::domain::settings::Settings;
use pebble_core::domain::transfer::TransferId;
use tokio::sync::{mpsc, watch};

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
    let _ = h
        .handle
        .ask(|reply| EngineCommand::Upload {
            peer: peer.clone(),
            file: file.clone(),
            remote_dir: RelativePath::root(),
            reply,
        })
        .await;
    tokio::time::sleep(Duration::from_millis(200)).await;
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
    let h = harness(&[PEER]);
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
    tokio::time::sleep(Duration::from_millis(300)).await;
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
    let huge = "x".repeat(1024 * 1024);
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
    tokio::time::sleep(Duration::from_millis(300)).await;
    let peers = h
        .handle
        .ask(|reply| EngineCommand::Peers { reply })
        .await
        .unwrap();
    let name = peers[0].device_name.clone();
    assert!(
        name.as_ref().is_none_or(|n| n.len() <= 256),
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

struct HangingNet;
impl NetworkFactory for HangingNet {
    fn start(&self) -> AppResult<NetworkLink> {
        let (commands, _rx) = mpsc::unbounded_channel();
        let (_tx, events) = mpsc::channel(8);
        let task = tokio::spawn(std::future::pending::<()>());
        std::mem::forget(_rx);
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
    let sup = Arc::new(P2pSupervisor::new(deps, Arc::new(HangingNet), None, None));
    sup.start().await.unwrap();
    let s2 = sup.clone();
    let stopper = tokio::spawn(async move { s2.stop().await });
    tokio::time::sleep(Duration::from_millis(300)).await;
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
        let _ = Path::new(&p);
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
    tokio::time::sleep(Duration::from_millis(100)).await;
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
    tokio::time::sleep(Duration::from_millis(200)).await;
    let refused = drain(&mut h).iter().any(|c| {
        matches!(c, NetCommand::RespondStream { ack, .. } if ack.result == Err("페어링이 필요합니다.".to_string()))
    });
    assert!(refused);
    assert!(!h.shared.join("intruder.txt").exists());
    assert_eq!(h.ui.count("transfer-started"), 0);
}
