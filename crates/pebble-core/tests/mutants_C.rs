use std::sync::Arc;
use std::time::Duration;

use pebble_core::adapters::fs::LocalFileStore;
use pebble_core::adapters::p2p::Libp2pNetwork;
use pebble_core::application::engine::{spawn_engine, EngineDeps};
use pebble_core::application::events::UiEvent;
use pebble_core::application::net::{
    FileListReply, InboundId, InboundRequest, NetCommand, NetEvent, NetworkLink,
};
use pebble_core::application::ports::{
    CredentialStore, DeviceCredentials, FileStore, NetworkFactory, ServerApi, TrustStore, UiEvents,
};
use pebble_core::application::server_sync::{spawn_server_sync, ServerDeps};
use pebble_core::application::state::{ActivityStore, SettingsStore};
use pebble_core::domain::error::{AppError, AppResult, ErrorCode};
use pebble_core::domain::files::FileSnapshot;
use pebble_core::domain::peer::PeerId;
use pebble_core::domain::settings::Settings;
use tokio::sync::{mpsc, watch};
use tokio::time::timeout;

const WAIT: Duration = Duration::from_secs(30);

async fn next_matching<T>(
    link: &mut NetworkLink,
    mut pick: impl FnMut(NetEvent) -> Option<T>,
) -> T {
    timeout(WAIT, async {
        loop {
            let event = link.events.recv().await.expect("이벤트 채널이 닫혔습니다");
            if let Some(found) = pick(event) {
                return found;
            }
        }
    })
    .await
    .expect("기다리던 네트워크 이벤트가 오지 않았습니다")
}

async fn connected_pair() -> (NetworkLink, NetworkLink) {
    let mut a = Libp2pNetwork::ephemeral(0).start().unwrap();
    let mut b = Libp2pNetwork::ephemeral(0).start().unwrap();
    let address = next_matching(&mut b, |e| match e {
        NetEvent::ListeningOn { address }
            if address.starts_with("/ip4/127.0.0.1/") && address.contains("/tcp/") =>
        {
            Some(address)
        }
        _ => None,
    })
    .await;
    a.commands.send(NetCommand::Dial { address }).unwrap();
    next_matching(&mut a, |e| {
        matches!(e, NetEvent::Connected { .. }).then_some(())
    })
    .await;
    next_matching(&mut b, |e| {
        matches!(e, NetEvent::Connected { .. }).then_some(())
    })
    .await;
    (a, b)
}

async fn next_inbound(link: &mut NetworkLink) -> (InboundId, String) {
    next_matching(link, |e| match e {
        NetEvent::Inbound {
            id,
            request: InboundRequest::FileList { path },
            ..
        } => Some((id, path)),
        _ => None,
    })
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_inbound_request_gets_its_own_id_and_its_own_reply() {
    let (mut a, mut b) = connected_pair().await;
    let b_peer = b.local_peer.clone();

    for path in ["/one", "/two"] {
        a.commands
            .send(NetCommand::RequestFileList {
                peer: b_peer.clone(),
                path: path.into(),
            })
            .unwrap();
    }
    let first = next_inbound(&mut b).await;
    let second = next_inbound(&mut b).await;
    assert_ne!(first.0, second.0);

    for (id, path) in [&second, &first] {
        b.commands
            .send(NetCommand::RespondFileList {
                inbound: *id,
                reply: FileListReply {
                    path: path.clone(),
                    files: Vec::new(),
                    error: None,
                },
            })
            .unwrap();
    }
    let mut replied = Vec::new();
    for _ in 0..2 {
        let path = next_matching(&mut a, |e| match e {
            NetEvent::FileListReplied { reply, .. } => Some(reply.path),
            _ => None,
        })
        .await;
        replied.push(path);
    }
    replied.sort();
    assert_eq!(replied, vec!["/one".to_string(), "/two".to_string()]);

    a.commands.send(NetCommand::Shutdown).unwrap();
    b.commands.send(NetCommand::Shutdown).unwrap();
}

async fn connect_to(from: &mut NetworkLink, to: &mut NetworkLink, address: String) {
    from.commands.send(NetCommand::Dial { address }).unwrap();
    next_matching(from, |e| {
        matches!(e, NetEvent::Connected { .. }).then_some(())
    })
    .await;
    next_matching(to, |e| {
        matches!(e, NetEvent::Connected { .. }).then_some(())
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_requests_of_a_disconnected_peer_are_dropped_but_others_survive() {
    let mut a = Libp2pNetwork::ephemeral(0).start().unwrap();
    let mut c = Libp2pNetwork::ephemeral(0).start().unwrap();
    let mut b = Libp2pNetwork::ephemeral(0).start().unwrap();
    let address = next_matching(&mut b, |e| match e {
        NetEvent::ListeningOn { address }
            if address.starts_with("/ip4/127.0.0.1/") && address.contains("/tcp/") =>
        {
            Some(address)
        }
        _ => None,
    })
    .await;
    connect_to(&mut a, &mut b, address.clone()).await;
    connect_to(&mut c, &mut b, address).await;
    let b_peer = b.local_peer.clone();
    let a_peer = a.local_peer.clone();

    for (link, path) in [(&a, "/from-a"), (&c, "/from-c")] {
        link.commands
            .send(NetCommand::RequestFileList {
                peer: b_peer.clone(),
                path: path.into(),
            })
            .unwrap();
    }
    let mut seen_a = false;
    let mut from_c = None;
    while !seen_a || from_c.is_none() {
        let (id, path) = next_inbound(&mut b).await;
        if path == "/from-a" {
            seen_a = true;
        } else {
            from_c = Some(id);
        }
    }

    a.commands.send(NetCommand::Shutdown).unwrap();
    next_matching(&mut b, |e| match e {
        NetEvent::Disconnected { peer } if peer == a_peer => Some(()),
        _ => None,
    })
    .await;

    b.commands
        .send(NetCommand::RespondFileList {
            inbound: from_c.unwrap(),
            reply: FileListReply {
                path: "/from-c".into(),
                files: Vec::new(),
                error: None,
            },
        })
        .unwrap();
    let path = next_matching(&mut c, |e| match e {
        NetEvent::FileListReplied { reply, .. } => Some(reply.path),
        _ => None,
    })
    .await;
    assert_eq!(path, "/from-c");

    c.commands.send(NetCommand::Shutdown).unwrap();
    b.commands.send(NetCommand::Shutdown).unwrap();
}

#[derive(Default)]
struct DownServer {
    failures_left: std::sync::Mutex<u32>,
    registers: std::sync::Mutex<u32>,
    syncs: std::sync::Mutex<u32>,
}

#[async_trait::async_trait]
impl ServerApi for DownServer {
    async fn register(&self, peer: &PeerId, _: &str) -> AppResult<DeviceCredentials> {
        *self.registers.lock().unwrap() += 1;
        let mut left = self.failures_left.lock().unwrap();
        if *left > 0 {
            *left -= 1;
            return Err(AppError::new(ErrorCode::Network, "down"));
        }
        Ok(DeviceCredentials {
            peer_id: peer.clone(),
            device_db_id: "1".into(),
            device_secret: "s".into(),
        })
    }
    async fn heartbeat(&self, _: &DeviceCredentials) -> AppResult<()> {
        Ok(())
    }
    async fn sync_files(&self, _: &DeviceCredentials, _: Vec<FileSnapshot>) -> AppResult<()> {
        *self.syncs.lock().unwrap() += 1;
        Ok(())
    }
}

struct NoCreds;

impl CredentialStore for NoCreds {
    fn load(&self, _: &PeerId) -> AppResult<Option<DeviceCredentials>> {
        Ok(None)
    }
    fn save(&self, _: &DeviceCredentials) -> AppResult<()> {
        Ok(())
    }
}

struct NoTrust;

#[async_trait::async_trait]
impl TrustStore for NoTrust {
    async fn load(&self) -> AppResult<Vec<PeerId>> {
        Ok(Vec::new())
    }
    async fn save(&self, _: &[PeerId]) -> AppResult<()> {
        Ok(())
    }
}

#[tokio::test(start_paused = true)]
async fn registration_retry_wait_doubles_after_each_failure() {
    let root = std::env::temp_dir().join(format!("mutants-c-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let mut settings = Settings::with_home(Some(root.clone()));
    settings.shared_folder = root.clone();
    let settings = SettingsStore::new(settings);
    let files: Arc<dyn FileStore> = Arc::new(LocalFileStore::new(root.join("downloads")));
    let engine_deps = EngineDeps {
        ui: Arc::new(NullUi),
        files: files.clone(),
        settings: settings.clone(),
        activity: ActivityStore::default(),
        trust: Arc::new(NoTrust),
    };
    let (net_cmds, _net_rx) = mpsc::unbounded_channel::<NetCommand>();
    let (_net_events, events_rx) = mpsc::channel(8);
    let (handle, _task) = spawn_engine(engine_deps, net_cmds, events_rx);

    let server = Arc::new(DownServer {
        failures_left: std::sync::Mutex::new(3),
        ..DownServer::default()
    });
    let deps = ServerDeps {
        api: server.clone(),
        credentials: Arc::new(NoCreds),
        files,
        settings,
    };
    let (stop, stop_rx) = watch::channel(false);
    let (_sync, sync_rx) = mpsc::channel(1);
    let task = spawn_server_sync(
        deps,
        PeerId::parse("12D3KooWAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").unwrap(),
        handle,
        stop_rx,
        sync_rx,
    );

    tokio::time::sleep(Duration::from_secs(30)).await;
    assert_eq!(*server.registers.lock().unwrap(), 3);
    assert_eq!(*server.syncs.lock().unwrap(), 0);

    tokio::time::sleep(Duration::from_secs(10)).await;
    assert_eq!(*server.registers.lock().unwrap(), 4);
    assert_eq!(*server.syncs.lock().unwrap(), 1);

    stop.send(true).unwrap();
    timeout(Duration::from_secs(1), task)
        .await
        .unwrap()
        .unwrap();
}

struct NullUi;

impl UiEvents for NullUi {
    fn emit(&self, _: UiEvent) {}
}
