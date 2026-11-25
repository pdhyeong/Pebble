use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use pebble_core::adapters::fs::LocalFileStore;
use pebble_core::application::engine::{
    spawn_engine, EngineCommand, EngineDeps, EngineHandle, PeerSummary, TransferState,
};
use pebble_core::application::events::UiEvent;
use pebble_core::application::folder_watch::FolderWatch;
use pebble_core::application::net::{
    InboundId, InboundRequest, NetCommand, NetEvent, NetworkLink, PairingAsk, RequestKind,
    StreamMsg,
};
use pebble_core::application::ports::{
    DeviceCredentials, FolderWatcher, NetworkFactory, TrustStore, UiEvents, WatchGuard,
};
use pebble_core::application::services::AppServices;
use pebble_core::application::state::{ActivityStore, SettingsStore};
use pebble_core::application::supervisor::P2pSupervisor;
use pebble_core::domain::error::AppResult;
use pebble_core::domain::path::RelativePath;
use pebble_core::domain::peer::PeerId;
use pebble_core::domain::settings::Settings;
use pebble_core::domain::transfer::TransferId;
use tokio::sync::mpsc;

const PEER_A: &str = "12D3KooWAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
const PEER_B: &str = "12D3KooWBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB";

fn tmp(label: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mutd-{label}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn peer(raw: &str) -> PeerId {
    PeerId::parse(raw).unwrap()
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
    fn named(&self, name: &str) -> Vec<serde_json::Value> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e["event"] == name)
            .cloned()
            .collect()
    }
    fn count(&self, name: &str) -> usize {
        self.named(name).len()
    }
}

async fn wait_until(what: &str, mut ready: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !ready() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("5초 안에 되지 않음: {what}"));
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
    activity: ActivityStore,
    root: PathBuf,
}

fn harness(trusted: &[&str]) -> Harness {
    let root = tmp("engine");
    let ui = Arc::new(Rec::default());
    let mut s = Settings::with_home(Some(root.clone()));
    s.shared_folder = root.join("shared");
    let activity = ActivityStore::default();
    let deps = EngineDeps {
        ui: ui.clone(),
        files: Arc::new(LocalFileStore::new(root.join("downloads"))),
        settings: SettingsStore::new(s),
        activity: activity.clone(),
        trust: Arc::new(MemTrust(trusted.iter().map(|p| peer(p)).collect())),
    };
    let (net, cmds) = mpsc::unbounded_channel();
    let (events, rx) = mpsc::channel(256);
    let (handle, _task) = spawn_engine(deps, net, rx);
    Harness {
        handle,
        cmds,
        events,
        ui,
        activity,
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

async fn state_of(h: &Harness, id: &TransferId) -> TransferState {
    h.handle
        .ask(|reply| EngineCommand::TransferState {
            transfer_id: id.clone(),
            reply,
        })
        .await
        .unwrap()
}

async fn state_until_ended(h: &Harness, id: &TransferId) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !matches!(state_of(h, id).await, TransferState::Ended(_)) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("전송이 5초 안에 끝나지 않음");
}

async fn peers_now(h: &Harness) -> Vec<PeerSummary> {
    h.handle
        .ask(|reply| EngineCommand::Peers { reply })
        .await
        .unwrap()
}

async fn download(h: &Harness, from: &str) -> TransferId {
    h.handle
        .ask(|reply| EngineCommand::Download {
            peer: peer(from),
            path: RelativePath::parse("/a.txt").unwrap(),
            reply,
        })
        .await
        .unwrap()
}

async fn upload(h: &Harness, to: &str, file: &std::path::Path) -> TransferId {
    h.handle
        .ask(|reply| EngineCommand::Upload {
            peer: peer(to),
            file: file.to_path_buf(),
            remote_dir: RelativePath::root(),
            reply,
        })
        .await
        .unwrap()
}

async fn inbound_init(h: &Harness, from: &str, n: u64, id: &str, for_pull: bool) {
    h.events
        .send(NetEvent::Inbound {
            peer: peer(from),
            id: InboundId(n),
            request: InboundRequest::Stream(StreamMsg::Init {
                transfer_id: TransferId::from_wire(id.into()),
                file_name: "in.bin".into(),
                total_size: 10,
                remote_path: "/".into(),
                for_pull,
            }),
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn loopback_listening_address_is_not_shown() {
    let h = harness(&[]);
    for address in ["/ip4/127.0.0.1/tcp/9", "/ip4/10.0.0.5/tcp/9"] {
        h.events
            .send(NetEvent::ListeningOn {
                address: address.into(),
            })
            .await
            .unwrap();
    }
    wait_until("listening-on", || h.ui.count("listening-on") >= 1).await;
    let shown: Vec<String> =
        h.ui.named("listening-on")
            .iter()
            .map(|e| e["payload"]["address"].as_str().unwrap().to_string())
            .collect();
    assert_eq!(shown, vec!["/ip4/10.0.0.5/tcp/9".to_string()]);
}

#[tokio::test]
async fn activity_record_is_stamped_with_current_unix_time() {
    let h = harness(&[PEER_A]);
    let id = download(&h, PEER_A).await;
    h.events
        .send(NetEvent::Disconnected { peer: peer(PEER_A) })
        .await
        .unwrap();
    state_until_ended(&h, &id).await;
    let records = h.activity.snapshot();
    assert_eq!(records.len(), 1);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    assert!(
        (now - 60..=now + 60).contains(&records[0].timestamp),
        "timestamp {} now {now}",
        records[0].timestamp
    );
}

#[tokio::test]
async fn engine_handle_reports_dead_after_shutdown() {
    let h = harness(&[]);
    assert!(h.handle.is_alive());
    h.handle.send(EngineCommand::Shutdown).await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), h.handle.closed())
        .await
        .expect("엔진이 5초 안에 닫히지 않음");
    assert!(!h.handle.is_alive());
}

#[tokio::test]
async fn pairing_with_bad_pin_is_rejected_and_replacement_rejects_old_request() {
    let mut h = harness(&[]);
    h.events
        .send(NetEvent::Inbound {
            peer: peer(PEER_A),
            id: InboundId(1),
            request: InboundRequest::Pairing(PairingAsk {
                device_name: "x".into(),
                pin: "12".into(),
            }),
        })
        .await
        .unwrap();
    let bad = next_command(
        &mut h,
        |c| matches!(c, NetCommand::RespondPairing { inbound, .. } if *inbound == InboundId(1)),
    )
    .await;
    assert!(matches!(bad, NetCommand::RespondPairing { reply, .. }
        if !reply.approved && reply.error.is_some()));

    for n in [2, 3] {
        h.events
            .send(NetEvent::Inbound {
                peer: peer(PEER_A),
                id: InboundId(n),
                request: InboundRequest::Pairing(PairingAsk {
                    device_name: "x".into(),
                    pin: "123456".into(),
                }),
            })
            .await
            .unwrap();
    }
    let replaced = next_command(
        &mut h,
        |c| matches!(c, NetCommand::RespondPairing { inbound, .. } if *inbound == InboundId(2)),
    )
    .await;
    assert!(matches!(replaced, NetCommand::RespondPairing { reply, .. }
        if !reply.approved && reply.error.is_some()));
}

#[tokio::test]
async fn disconnect_removes_connection_and_pending_pairing() {
    let h = harness(&[]);
    h.events
        .send(NetEvent::Connected { peer: peer(PEER_A) })
        .await
        .unwrap();
    h.events
        .send(NetEvent::Inbound {
            peer: peer(PEER_A),
            id: InboundId(1),
            request: InboundRequest::Pairing(PairingAsk {
                device_name: "x".into(),
                pin: "123456".into(),
            }),
        })
        .await
        .unwrap();
    wait_until("peer listed", || h.ui.count("pairing-request") == 1).await;
    assert_eq!(peers_now(&h).await.len(), 1);
    h.events
        .send(NetEvent::Disconnected { peer: peer(PEER_A) })
        .await
        .unwrap();
    wait_until("connection-closed", || h.ui.count("connection-closed") == 1).await;
    assert!(peers_now(&h).await.is_empty());
}

#[tokio::test]
async fn request_failures_are_reported_per_kind() {
    let h = harness(&[PEER_A]);
    let id = download(&h, PEER_A).await;
    let failed = |kind, transfer_id, error: &str| NetEvent::RequestFailed {
        peer: peer(PEER_A),
        kind,
        transfer_id,
        error: error.into(),
    };
    for ev in [
        failed(RequestKind::FileList, None, "fl"),
        failed(RequestKind::Pairing, None, "pr"),
        failed(RequestKind::DeviceInfo, None, "di"),
        failed(RequestKind::Stream, Some(id.clone()), "st"),
    ] {
        h.events.send(ev).await.unwrap();
    }
    state_until_ended(&h, &id).await;
    assert_eq!(h.ui.count("file-list-error"), 1);
    assert_eq!(h.ui.count("pairing-rejected"), 1);
    let tf = h.ui.named("transfer-failed");
    assert_eq!(tf.len(), 1);
    assert_eq!(tf[0]["payload"]["error"], "st");
}

#[tokio::test]
async fn init_for_pull_from_other_peer_is_refused() {
    let mut h = harness(&[PEER_A, PEER_B]);
    let id = download(&h, PEER_A).await;
    inbound_init(&h, PEER_B, 7, id.as_str(), true).await;
    let ack = next_command(
        &mut h,
        |c| matches!(c, NetCommand::RespondStream { inbound, .. } if *inbound == InboundId(7)),
    )
    .await;
    assert!(matches!(ack, NetCommand::RespondStream { ack, .. } if ack.result.is_err()));
    assert!(matches!(state_of(&h, &id).await, TransferState::Active));
    assert_eq!(h.ui.count("transfer-started"), 1);
}

#[tokio::test]
async fn disconnect_fails_only_that_peers_transfers() {
    let h = harness(&[PEER_A, PEER_B]);
    let file = h.root.join("up.bin");
    std::fs::write(&file, vec![1u8; 10]).unwrap();
    let up_a = upload(&h, PEER_A, &file).await;
    let up_b = upload(&h, PEER_B, &file).await;
    let dl_a = download(&h, PEER_A).await;
    let dl_b = download(&h, PEER_B).await;
    inbound_init(&h, PEER_A, 1, "in-a", false).await;
    inbound_init(&h, PEER_B, 2, "in-b", false).await;
    let in_a = TransferId::from_wire("in-a".into());
    let in_b = TransferId::from_wire("in-b".into());
    wait_until("both incoming started", || {
        h.ui.count("transfer-started") == 6
    })
    .await;
    h.events
        .send(NetEvent::Disconnected { peer: peer(PEER_A) })
        .await
        .unwrap();
    for id in [&up_a, &dl_a, &in_a] {
        state_until_ended(&h, id).await;
    }
    for id in [&up_b, &dl_b, &in_b] {
        assert!(
            matches!(state_of(&h, id).await, TransferState::Active),
            "다른 기기의 전송이 같이 끊김"
        );
    }
    assert_eq!(h.ui.count("transfer-failed"), 3);
}

#[tokio::test]
async fn engine_shutdown_fails_running_transfers_and_tells_peer() {
    let mut h = harness(&[PEER_A]);
    let file = h.root.join("up.bin");
    std::fs::write(&file, vec![1u8; 10]).unwrap();
    let id = upload(&h, PEER_A, &file).await;
    h.handle.send(EngineCommand::Shutdown).await.unwrap();
    let cancel = next_command(&mut h, |c| {
        matches!(
            c,
            NetCommand::SendStream {
                msg: StreamMsg::Cancel { .. },
                ..
            }
        )
    })
    .await;
    assert!(matches!(cancel, NetCommand::SendStream { msg, .. } if msg.transfer_id() == &id));
    wait_until("p2p-stopped", || h.ui.count("p2p-stopped") == 1).await;
    let failed = h.ui.named("transfer-failed");
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0]["payload"]["transfer_id"], id.as_str());
}

#[test]
fn device_credentials_debug_hides_secret() {
    let creds = DeviceCredentials {
        peer_id: peer(PEER_A),
        device_db_id: "db-1".into(),
        device_secret: "top-secret".into(),
    };
    let shown = format!("{creds:?}");
    assert!(shown.contains("DeviceCredentials"));
    assert!(shown.contains(PEER_A));
    assert!(shown.contains("db-1"));
    assert!(shown.contains("<hidden>"));
    assert!(!shown.contains("top-secret"));
}

struct DropFlag(Arc<AtomicBool>);
impl Drop for DropFlag {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

struct PendingNet {
    dropped: Arc<AtomicBool>,
}
impl NetworkFactory for PendingNet {
    fn start(&self) -> AppResult<NetworkLink> {
        let (commands, rx) = mpsc::unbounded_channel();
        let (tx, events) = mpsc::channel(8);
        let flag = DropFlag(self.dropped.clone());
        let task = tokio::spawn(async move {
            let _flag = flag;
            let _keep = (rx, tx);
            std::future::pending::<()>().await
        });
        Ok(NetworkLink {
            local_peer: peer(PEER_A),
            commands,
            events,
            task,
        })
    }
}

struct DyingNet;
impl NetworkFactory for DyingNet {
    fn start(&self) -> AppResult<NetworkLink> {
        let (commands, _rx) = mpsc::unbounded_channel();
        let (_tx, events) = mpsc::channel(8);
        let task = tokio::spawn(async {});
        Ok(NetworkLink {
            local_peer: peer(PEER_A),
            commands,
            events,
            task,
        })
    }
}

struct NoWatch;
impl FolderWatcher for NoWatch {
    fn watch(
        &self,
        _: &std::path::Path,
        _: mpsc::Sender<PathBuf>,
    ) -> AppResult<Box<dyn WatchGuard>> {
        Ok(Box::new(()))
    }
}

fn supervisor(net: Arc<dyn NetworkFactory>) -> (Arc<P2pSupervisor>, EngineDeps, Arc<Rec>) {
    let root = tmp("sup");
    let ui = Arc::new(Rec::default());
    let deps = EngineDeps {
        ui: ui.clone(),
        files: Arc::new(LocalFileStore::new(root.join("d"))),
        settings: SettingsStore::new(Settings::with_home(Some(root))),
        activity: ActivityStore::default(),
        trust: Arc::new(MemTrust(vec![])),
    };
    (
        Arc::new(P2pSupervisor::new(deps.clone(), net, None, None)),
        deps,
        ui,
    )
}

#[tokio::test(start_paused = true)]
async fn stop_waits_for_engine_and_aborts_stuck_network_task() {
    let dropped = Arc::new(AtomicBool::new(false));
    let (sup, _, ui) = supervisor(Arc::new(PendingNet {
        dropped: dropped.clone(),
    }));
    sup.start().await.unwrap();
    assert_eq!(sup.local_peer().await, Some(peer(PEER_A)));
    sup.stop().await.unwrap();
    assert_eq!(
        ui.count("p2p-stopped"),
        1,
        "stop 이 엔진 종료를 기다리지 않음"
    );
    for _ in 0..100 {
        if dropped.load(Ordering::SeqCst) {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        dropped.load(Ordering::SeqCst),
        "멈추지 않는 네트워크 작업이 강제 종료되지 않음"
    );
    assert_eq!(sup.local_peer().await, None);
    assert!(!sup.is_running().await);
}

#[tokio::test]
async fn dead_engine_is_not_served_by_supervisor() {
    let (sup, _, _) = supervisor(Arc::new(DyingNet));
    sup.start().await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while sup.is_running().await {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("엔진이 죽었는데 실행 중으로 보임");
    assert!(sup.engine().await.is_err());
    assert_eq!(sup.local_peer().await, None);
}

#[tokio::test]
async fn app_services_settings_round_trip() {
    let (sup, deps, ui) = supervisor(Arc::new(DyingNet));
    let folder_watch = Arc::new(FolderWatch::spawn(
        Arc::new(NoWatch),
        ui.clone(),
        sup.clone(),
    ));
    let initial_folder = deps.settings.get().shared_folder;
    let app = AppServices {
        supervisor: sup,
        folder_watch,
        settings: deps.settings.clone(),
        activity: deps.activity.clone(),
        files: deps.files.clone(),
        ui,
    };
    assert_eq!(app.shared_folder(), initial_folder);
    assert!(!app.shared_folder().as_os_str().is_empty());
    app.set_device_name("새 기기".into());
    assert_eq!(app.device_name(), "새 기기");
    assert_eq!(deps.settings.get().device_name, "새 기기");
    app.set_shared_folder_display_name("공유".into());
    assert_eq!(app.shared_folder_display_name(), "공유");
    assert!(app.show_folder_name());
    app.set_show_folder_name(false);
    assert!(!app.show_folder_name());
    app.set_show_folder_name(true);
    assert!(app.show_folder_name());
}
