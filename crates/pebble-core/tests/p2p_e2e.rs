use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use pebble_core::adapters::fs::{JsonTrustStore, LocalFileStore, NotifyFolderWatcher};
use pebble_core::adapters::p2p::Libp2pNetwork;
use pebble_core::application::engine::EngineDeps;
use pebble_core::application::events::UiEvent;
use pebble_core::application::folder_watch::FolderWatch;
use pebble_core::application::ports::{FileStore, PeerDirectory, UiEvents};
use pebble_core::application::services::AppServices;
use pebble_core::application::state::{ActivityStore, SettingsStore};
use pebble_core::application::supervisor::P2pSupervisor;
use pebble_core::domain::error::{AppResult, ErrorCode};
use pebble_core::domain::settings::Settings;
use pebble_core::domain::transfer::{Direction, TransferStatus, CHUNK_SIZE};

const WAIT: Duration = Duration::from_secs(30);

#[derive(Default)]
struct Recorder {
    events: Mutex<Vec<serde_json::Value>>,
}

impl UiEvents for Recorder {
    fn emit(&self, event: UiEvent) {
        let value = serde_json::to_value(&event).unwrap();
        self.events.lock().unwrap().push(value);
    }
}

impl Recorder {
    fn len(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    async fn wait(
        &self,
        after: usize,
        name: &str,
        pred: impl Fn(&serde_json::Value) -> bool,
    ) -> serde_json::Value {
        let deadline = tokio::time::Instant::now() + WAIT;
        loop {
            let found = self.events.lock().unwrap()[after..]
                .iter()
                .find(|e| e["event"] == name && pred(&e["payload"]))
                .map(|e| e["payload"].clone());
            if let Some(payload) = found {
                return payload;
            }
            if tokio::time::Instant::now() > deadline {
                let seen: Vec<_> = self
                    .events
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|e| e["event"].clone())
                    .collect();
                panic!("{name} 이벤트가 오지 않았습니다. 받은 이벤트: {seen:?}");
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    fn count(&self, name: &str) -> usize {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e["event"] == name)
            .count()
    }
}

struct FixedDirectory(Mutex<Vec<String>>);

#[async_trait]
impl PeerDirectory for FixedDirectory {
    async fn addresses(&self) -> AppResult<Vec<String>> {
        Ok(self.0.lock().unwrap().clone())
    }
}

struct Node {
    services: AppServices,
    ui: Arc<Recorder>,
    shared: PathBuf,
    downloads: PathBuf,
    peer_id: Option<String>,
}

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pebble-e2e-{label}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

async fn node(label: &str) -> Node {
    node_with_directory(label, None).await
}

async fn node_with_directory(label: &str, directory: Option<Arc<dyn PeerDirectory>>) -> Node {
    let root = temp_dir(label);
    let shared = root.join("shared");
    let downloads = root.join("downloads");
    let ui = Arc::new(Recorder::default());
    let ui_port: Arc<dyn UiEvents> = ui.clone();
    let files: Arc<dyn FileStore> = Arc::new(LocalFileStore::new(downloads.clone()));
    let mut initial = Settings::with_home(Some(root.clone()));
    initial.device_name = label.to_string();
    let settings = SettingsStore::new(initial);
    let activity = ActivityStore::default();
    let deps = EngineDeps {
        ui: ui_port.clone(),
        files: files.clone(),
        settings: settings.clone(),
        activity: activity.clone(),
        trust: Arc::new(JsonTrustStore::new(root.join("config/trusted-peers.json"))),
    };
    let network = Arc::new(Libp2pNetwork::ephemeral(0));
    let supervisor = Arc::new(P2pSupervisor::new(deps, network, None, directory));
    let folder_watch = Arc::new(FolderWatch::spawn(
        Arc::new(NotifyFolderWatcher),
        ui_port.clone(),
        supervisor.clone(),
    ));
    let services = AppServices {
        supervisor,
        folder_watch,
        settings,
        activity,
        files,
        ui: ui_port,
    };
    services.set_shared_folder(shared.clone()).await.unwrap();
    Node {
        services,
        ui,
        shared,
        downloads,
        peer_id: None,
    }
}

async fn tcp_address(n: &Node) -> String {
    let payload =
        n.ui.wait(0, "listening-on", |p| {
            let a = p["address"].as_str().unwrap_or_default();
            a.contains("/tcp/") && a.starts_with("/ip4/")
        })
        .await;
    payload["address"].as_str().unwrap().to_string()
}

async fn connect(a: &mut Node, b: &mut Node) {
    let addr = tcp_address(b).await;
    let mark = a.ui.len();
    a.services.dial(addr).await.unwrap();
    let connected = a.ui.wait(mark, "connection-success", |_| true).await;
    b.peer_id = Some(connected["peer_id"].as_str().unwrap().to_string());
    let back = b.ui.wait(0, "connection-success", |_| true).await;
    a.peer_id = Some(back["peer_id"].as_str().unwrap().to_string());
}

async fn pair(a: &Node, b: &Node) {
    let b_id = b.peer_id.clone().unwrap();
    let a_id = a.peer_id.clone().unwrap();
    let mark_b = b.ui.len();
    let mark_a = a.ui.len();
    let pin = a.services.request_pairing(&b_id).await.unwrap();
    let request =
        b.ui.wait(mark_b, "pairing-request", |p| p["peer_id"] == a_id.as_str())
            .await;
    assert_eq!(request["pin"], pin.as_str());
    b.services.respond_pairing(&a_id, true).await.unwrap();
    let approved =
        a.ui.wait(mark_a, "pairing-approved", |p| {
            p["peer_id"] == b_id.as_str()
        })
        .await;
    assert_eq!(approved["shared_folder_name"], "Shared");
}

fn patterned(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i * 31 % 251) as u8).collect()
}

fn no_partial_files(dir: &Path) -> bool {
    walk(dir)
        .iter()
        .all(|p| !p.to_string_lossy().ends_with(".part"))
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(walk(&path));
            } else {
                out.push(path);
            }
        }
    }
    out
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_nodes_pair_browse_upload_download_cancel_and_restart() {
    let mut a = node("alice").await;
    let mut b = node("bob").await;

    let (first, second) = tokio::join!(a.services.start_p2p(), a.services.start_p2p());
    assert!(
        first.is_ok() != second.is_ok(),
        "동시에 두 번 시작하면 하나만 성공해야 합니다"
    );
    assert_eq!(
        first.err().or(second.err()).unwrap().code,
        ErrorCode::P2pAlreadyRunning
    );
    b.services.start_p2p().await.unwrap();

    connect(&mut a, &mut b).await;
    let a_id = a.peer_id.clone().unwrap();
    let b_id = b.peer_id.clone().unwrap();

    let names =
        b.ui.wait(0, "device-info-received", |p| p["peer_id"] == a_id.as_str())
            .await;
    assert_eq!(names["device_name"], "alice");

    let mark = a.ui.len();
    a.services
        .request_file_list(&b_id, "/".into())
        .await
        .unwrap();
    let denied = a.ui.wait(mark, "remote-files", |_| true).await;
    assert_eq!(denied["error"], "페어링이 필요합니다.");

    pair(&a, &b).await;

    std::fs::create_dir_all(b.shared.join("docs")).unwrap();
    std::fs::write(b.shared.join("docs/note.txt"), b"hello").unwrap();
    let big = patterned(3 * CHUNK_SIZE + 12_345);
    std::fs::write(b.shared.join("big.bin"), &big).unwrap();

    let mark = a.ui.len();
    a.services
        .request_file_list(&b_id, "/docs".into())
        .await
        .unwrap();
    let listed =
        a.ui.wait(mark, "remote-files", |p| p["path"] == "/docs")
            .await;
    assert_eq!(listed["error"], serde_json::Value::Null);
    assert_eq!(listed["files"][0]["path"], "/docs/note.txt");
    assert_eq!(listed["files"][0]["size"], 5);

    let mark = a.ui.len();
    a.services
        .request_file_list(&b_id, "/../..".into())
        .await
        .unwrap();
    let escaped =
        a.ui.wait(mark, "remote-files", |p| p["path"] == "/../..")
            .await;
    assert!(escaped["error"].as_str().unwrap().contains("공유 폴더 밖"));

    let local = temp_dir("alice-local").join("photo.raw");
    let upload_bytes = patterned(2 * CHUNK_SIZE + 7);
    std::fs::write(&local, &upload_bytes).unwrap();
    let mark_a = a.ui.len();
    let mark_b = b.ui.len();
    let upload_id = a
        .services
        .upload(&b_id, local.clone(), "/inbox")
        .await
        .unwrap();
    let id = upload_id.to_string();
    a.ui.wait(mark_a, "transfer-completed", |p| {
        p["transfer_id"] == id.as_str() && p["direction"] == "upload"
    })
    .await;
    let received =
        b.ui.wait(mark_b, "transfer-completed", |p| {
            p["transfer_id"] == id.as_str() && p["direction"] == "download"
        })
        .await;
    let saved = b.shared.join("inbox/photo.raw");
    let reported = PathBuf::from(received["saved_path"].as_str().unwrap());
    assert_eq!(
        reported.canonicalize().unwrap(),
        saved.canonicalize().unwrap()
    );
    assert_eq!(std::fs::read(&saved).unwrap(), upload_bytes);
    assert!(no_partial_files(&b.shared));

    let mark_a = a.ui.len();
    let download_id = a
        .services
        .download(&b_id, "/big.bin")
        .await
        .unwrap()
        .to_string();
    let done =
        a.ui.wait(mark_a, "transfer-completed", |p| {
            p["transfer_id"] == download_id.as_str()
        })
        .await;
    assert_eq!(done["direction"], "download");
    assert_eq!(done["total_size"], big.len());
    assert_eq!(std::fs::read(a.downloads.join("big.bin")).unwrap(), big);
    assert!(no_partial_files(&a.downloads));

    let empty = temp_dir("alice-empty").join("empty.txt");
    std::fs::write(&empty, b"").unwrap();
    let mark_b = b.ui.len();
    let empty_id = a
        .services
        .upload(&b_id, empty, "/")
        .await
        .unwrap()
        .to_string();
    b.ui.wait(mark_b, "transfer-completed", |p| {
        p["transfer_id"] == empty_id.as_str()
    })
    .await;
    assert_eq!(std::fs::read(b.shared.join("empty.txt")).unwrap(), b"");

    let err = a
        .services
        .download(&b_id, "/../../etc/passwd")
        .await
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidPath);

    let mark_a = a.ui.len();
    let missing_id = a
        .services
        .download(&b_id, "/nope.bin")
        .await
        .unwrap()
        .to_string();
    let failed =
        a.ui.wait(mark_a, "transfer-failed", |p| {
            p["transfer_id"] == missing_id.as_str()
        })
        .await;
    assert_eq!(failed["direction"], "download");

    let huge = temp_dir("alice-huge").join("huge.bin");
    std::fs::write(&huge, patterned(64 * CHUNK_SIZE)).unwrap();
    let mark_a = a.ui.len();
    let mark_b = b.ui.len();
    let huge_id = a.services.upload(&b_id, huge, "/").await.unwrap();
    let huge_str = huge_id.to_string();
    b.ui.wait(mark_b, "transfer-progress", |p| {
        p["transfer_id"] == huge_str.as_str()
    })
    .await;
    a.services.cancel(huge_str.clone()).await.unwrap();
    let mine =
        a.ui.wait(mark_a, "transfer-cancelled", |p| {
            p["transfer_id"] == huge_str.as_str()
        })
        .await;
    assert_eq!(mine["reason"], "user_cancelled");
    let theirs =
        b.ui.wait(mark_b, "transfer-cancelled", |p| {
            p["transfer_id"] == huge_str.as_str()
        })
        .await;
    assert_eq!(theirs["reason"], "peer_cancelled");
    assert!(!b.shared.join("huge.bin").exists());
    assert!(no_partial_files(&b.shared));

    let history = a.services.activity_history();
    let completed_uploads = history
        .iter()
        .filter(|r| r.transfer_type == Direction::Upload && r.status == TransferStatus::Completed)
        .count();
    assert_eq!(completed_uploads, 2);
    assert!(history
        .iter()
        .any(|r| r.status == TransferStatus::Cancelled && r.device_name.as_deref() == Some("bob")));

    let mark_b = b.ui.len();
    std::fs::write(a.shared.join("changed.txt"), b"x").unwrap();
    b.ui.wait(mark_b, "remote-directory-changed", |p| {
        p["peer_id"] == a_id.as_str()
    })
    .await;

    let mut c = node("carol").await;
    c.services.start_p2p().await.unwrap();
    connect(&mut c, &mut b).await;
    let c_upload = temp_dir("carol-local").join("intruder.txt");
    std::fs::write(&c_upload, b"nope").unwrap();
    let refused = c.services.upload(&b_id, c_upload, "/").await.unwrap_err();
    assert_eq!(refused.code, ErrorCode::AccessDenied);
    assert_eq!(refused.message, "페어링이 필요합니다.");
    assert!(!b.shared.join("intruder.txt").exists());

    let mark_a = a.ui.len();
    a.services.stop_p2p().await.unwrap();
    a.ui.wait(mark_a, "p2p-stopped", |_| true).await;
    assert!(!a.services.p2p_status().await);
    assert_eq!(
        a.services.stop_p2p().await.unwrap_err().code,
        ErrorCode::P2pNotRunning
    );
    assert_eq!(
        a.services
            .request_file_list(&b_id, "/".into())
            .await
            .unwrap_err()
            .code,
        ErrorCode::P2pNotRunning
    );
    a.services.start_p2p().await.unwrap();
    assert!(a.services.p2p_status().await);
    assert_eq!(a.ui.count("p2p-stopped"), 1);

    let mark_a = a.ui.len();
    let addr = tcp_address(&b).await;
    a.services.dial(addr).await.unwrap();
    a.ui.wait(mark_a, "pairing-approved", |p| {
        p["peer_id"] == b_id.as_str()
    })
    .await;
    let mark_a = a.ui.len();
    a.services
        .request_file_list(&b_id, "/docs".into())
        .await
        .unwrap();
    let again =
        a.ui.wait(mark_a, "remote-files", |p| p["path"] == "/docs")
            .await;
    assert_eq!(
        again["error"],
        serde_json::Value::Null,
        "다시 켠 뒤에도 페어링이 유지되어야 합니다"
    );
    let summary = a.services.peers().await.unwrap();
    assert!(summary
        .iter()
        .any(|p| p.peer_id.as_str() == b_id && p.trusted && p.connected));

    let directory = Arc::new(FixedDirectory(Mutex::new(Vec::new())));
    *directory.0.lock().unwrap() = vec![tcp_address(&b).await, "/ip4/127.0.0.1/tcp/1".to_string()];
    let d = node_with_directory("dave", Some(directory)).await;
    let mark_d = d.ui.len();
    d.services.start_p2p().await.unwrap();
    let found =
        d.ui.wait(mark_d, "peer-found", |p| p["peer_id"] == b_id.as_str())
            .await;
    assert!(found["address"].as_str().unwrap().contains("/tcp/"));
    d.ui.wait(mark_d, "connection-success", |p| {
        p["peer_id"] == b_id.as_str()
    })
    .await;
    assert_eq!(
        d.ui.count("dial-failed"),
        0,
        "자동 발견 실패는 화면에 알리지 않습니다"
    );
    d.services.stop_p2p().await.unwrap();

    b.services.stop_p2p().await.unwrap();
    c.services.stop_p2p().await.unwrap();
    a.services.stop_p2p().await.unwrap();
}
