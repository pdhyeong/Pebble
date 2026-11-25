use std::path::Path;
use std::time::{Duration, Instant};

use super::peers::NOT_PAIRED;
use super::Engine;
use crate::application::events::UiEvent;
use crate::application::net::{InboundId, NetCommand, StreamAck, StreamMsg};
use crate::application::ports::{ChunkSink, ChunkSource, IncomingTarget, OpenedFile};
use crate::domain::error::{AppError, AppResult, ErrorCode};
use crate::domain::path::{FileName, RelativePath};
use crate::domain::peer::PeerId;
use crate::domain::transfer::{
    speed_bps, CancelReason, ChunkCounter, Direction, TransferId, TransferStatus,
};

const ENGINE_STOPPED: &str = "P2P 엔진이 종료되어 전송을 멈췄습니다.";
const STALLED: &str = "상대 기기가 응답하지 않아 전송을 멈췄습니다.";
pub(super) const STALL_TIMEOUT: Duration = Duration::from_secs(120);
const EARLY_CANCEL_LIMIT: usize = 256;

pub(super) struct Outgoing {
    peer: PeerId,
    file_name: String,
    source: Box<dyn ChunkSource>,
    counter: ChunkCounter,
    started: Instant,
}

pub(super) struct Incoming {
    peer: PeerId,
    file_name: String,
    sink: Box<dyn ChunkSink>,
    counter: ChunkCounter,
    started: Instant,
    touched: Instant,
}

pub(super) struct PendingPull {
    peer: PeerId,
    file_name: String,
    touched: Instant,
}

enum Ending {
    Cancelled(CancelReason),
    Failed(String),
}

struct Taken {
    direction: Direction,
    peer: PeerId,
    file_name: String,
    size: u64,
    sink: Option<Box<dyn ChunkSink>>,
}

struct Step {
    peer: PeerId,
    index: u32,
    data: Vec<u8>,
    done: u64,
    total: u64,
    percent: f64,
}

impl Engine {
    pub(super) fn start_download(
        &mut self,
        peer: PeerId,
        path: &RelativePath,
    ) -> AppResult<TransferId> {
        self.require_trusted(&peer)?;
        let name = path.file_name().ok_or_else(|| {
            AppError::new(ErrorCode::InvalidPath, "다운로드할 파일 경로가 비었습니다.")
        })?;
        let transfer_id = TransferId::generate();
        let file_name = name.as_str().to_string();
        self.send_net(NetCommand::SendStream {
            peer: peer.clone(),
            msg: StreamMsg::Pull {
                transfer_id: transfer_id.clone(),
                path: path.to_display(),
            },
        });
        self.emit(UiEvent::TransferStarted {
            transfer_id: transfer_id.clone(),
            direction: Direction::Download,
            peer_id: peer.clone(),
            file_name: file_name.clone(),
            total_size: 0,
        });
        self.pulls.insert(
            transfer_id.clone(),
            PendingPull {
                peer,
                file_name,
                touched: Instant::now(),
            },
        );
        Ok(transfer_id)
    }

    pub(super) async fn start_upload(
        &mut self,
        peer: PeerId,
        file: &Path,
        remote_dir: &RelativePath,
    ) -> AppResult<TransferId> {
        self.require_trusted(&peer)?;
        let opened = self.deps.files.open_local(file).await?;
        let transfer_id = TransferId::generate();
        self.begin_outgoing(
            peer,
            transfer_id.clone(),
            opened,
            remote_dir.to_display(),
            false,
        );
        Ok(transfer_id)
    }

    fn begin_outgoing(
        &mut self,
        peer: PeerId,
        transfer_id: TransferId,
        opened: OpenedFile,
        remote_path: String,
        for_pull: bool,
    ) {
        let file_name = opened.name.as_str().to_string();
        self.send_net(NetCommand::SendStream {
            peer: peer.clone(),
            msg: StreamMsg::Init {
                transfer_id: transfer_id.clone(),
                file_name: file_name.clone(),
                total_size: opened.size,
                remote_path,
                for_pull,
            },
        });
        self.emit(UiEvent::TransferStarted {
            transfer_id: transfer_id.clone(),
            direction: Direction::Upload,
            peer_id: peer.clone(),
            file_name: file_name.clone(),
            total_size: opened.size,
        });
        self.outgoing.insert(
            transfer_id,
            Outgoing {
                peer,
                file_name,
                source: opened.source,
                counter: ChunkCounter::new(opened.size),
                started: Instant::now(),
            },
        );
    }

    pub(super) async fn on_stream_request(
        &mut self,
        peer: PeerId,
        inbound: InboundId,
        msg: StreamMsg,
    ) {
        if !self.trusted.contains(&peer) {
            self.ack(
                inbound,
                msg.transfer_id().clone(),
                Err(NOT_PAIRED.to_string()),
            );
            return;
        }
        match msg {
            StreamMsg::Pull { transfer_id, path } => {
                self.serve_pull(peer, inbound, transfer_id, &path).await
            }
            StreamMsg::Init {
                transfer_id,
                file_name,
                total_size,
                remote_path,
                for_pull,
            } => {
                let target = if for_pull {
                    None
                } else {
                    Some(remote_path.as_str())
                };
                self.on_init(peer, inbound, transfer_id, file_name, total_size, target)
                    .await
            }
            StreamMsg::Chunk {
                transfer_id,
                index,
                data,
            } => {
                self.on_chunk(&peer, inbound, transfer_id, index, &data)
                    .await
            }
            StreamMsg::Cancel { transfer_id } => {
                self.remember_early_cancel(&transfer_id, &peer);
                self.end_transfer(
                    &transfer_id,
                    Ending::Cancelled(CancelReason::PeerCancelled),
                    false,
                    Some(&peer),
                )
                .await;
                self.ack(inbound, transfer_id, Ok(()));
            }
        }
    }

    fn ack(&self, inbound: InboundId, transfer_id: TransferId, result: Result<(), String>) {
        self.send_net(NetCommand::RespondStream {
            inbound,
            ack: StreamAck {
                transfer_id,
                result,
            },
        });
    }

    async fn serve_pull(
        &mut self,
        peer: PeerId,
        inbound: InboundId,
        transfer_id: TransferId,
        path: &str,
    ) {
        if self.outgoing.contains_key(&transfer_id) {
            self.ack(
                inbound,
                transfer_id,
                Err("이미 진행 중인 전송 ID 입니다.".to_string()),
            );
            return;
        }
        let root = self.deps.settings.get().shared_folder;
        let opened = match RelativePath::parse(path) {
            Ok(rel) => self.deps.files.open_shared(&root, &rel).await,
            Err(e) => Err(e),
        };
        match opened {
            Ok(opened) => {
                self.ack(inbound, transfer_id.clone(), Ok(()));
                self.begin_outgoing(peer, transfer_id, opened, String::new(), true);
            }
            Err(e) => self.ack(inbound, transfer_id, Err(e.message)),
        }
    }

    async fn on_init(
        &mut self,
        peer: PeerId,
        inbound: InboundId,
        transfer_id: TransferId,
        file_name: String,
        total_size: u64,
        upload_dir: Option<&str>,
    ) {
        if self.incoming.contains_key(&transfer_id) {
            self.ack(
                inbound,
                transfer_id,
                Err("이미 진행 중인 전송 ID 입니다.".to_string()),
            );
            return;
        }
        if self.take_early_cancel(&transfer_id, &peer) {
            self.ack(
                inbound,
                transfer_id,
                Err("이미 취소한 전송입니다.".to_string()),
            );
            return;
        }
        let pull = match self.pulls.get(&transfer_id) {
            Some(p) if p.peer == peer => self.pulls.remove(&transfer_id),
            _ => None,
        };
        if upload_dir.is_none() && pull.is_none() {
            self.ack(
                inbound,
                transfer_id,
                Err("요청하지 않았거나 이미 취소한 다운로드입니다.".to_string()),
            );
            return;
        }
        let sink = match self.open_incoming(upload_dir, &file_name).await {
            Ok(sink) => sink,
            Err(e) => {
                if pull.is_some() {
                    self.emit_failed(
                        &transfer_id,
                        Direction::Download,
                        &peer,
                        &file_name,
                        &e.message,
                    );
                    self.record(
                        Direction::Download,
                        &peer,
                        &file_name,
                        total_size,
                        TransferStatus::Failed,
                        None,
                    );
                }
                self.ack(inbound, transfer_id, Err(e.message));
                return;
            }
        };
        self.emit(UiEvent::TransferStarted {
            transfer_id: transfer_id.clone(),
            direction: Direction::Download,
            peer_id: peer.clone(),
            file_name: file_name.clone(),
            total_size,
        });
        let counter = ChunkCounter::new(total_size);
        let complete = counter.is_complete();
        self.incoming.insert(
            transfer_id.clone(),
            Incoming {
                peer,
                file_name,
                sink,
                counter,
                started: Instant::now(),
                touched: Instant::now(),
            },
        );
        let result = if complete {
            self.finish_incoming(&transfer_id).await
        } else {
            Ok(())
        };
        self.ack(inbound, transfer_id, result);
    }

    async fn open_incoming(
        &self,
        upload_dir: Option<&str>,
        file_name: &str,
    ) -> AppResult<Box<dyn ChunkSink>> {
        let name = FileName::parse(file_name)?;
        let Some(upload_dir) = upload_dir else {
            return self
                .deps
                .files
                .create_incoming(IncomingTarget::Downloads, &name)
                .await;
        };
        let dir = RelativePath::parse(upload_dir)?;
        let root = self.deps.settings.get().shared_folder;
        self.deps
            .files
            .create_incoming(
                IncomingTarget::Shared {
                    root: &root,
                    dir: &dir,
                },
                &name,
            )
            .await
    }

    async fn on_chunk(
        &mut self,
        peer: &PeerId,
        inbound: InboundId,
        transfer_id: TransferId,
        index: u32,
        data: &[u8],
    ) {
        let Some(entry) = self
            .incoming
            .get_mut(&transfer_id)
            .filter(|t| &t.peer == peer)
        else {
            self.ack(
                inbound,
                transfer_id,
                Err("전송 세션을 찾을 수 없습니다.".to_string()),
            );
            return;
        };
        entry.touched = Instant::now();
        let written = match entry.counter.accept(index, data.len()) {
            Ok(()) => entry.sink.write(data).await.map_err(|e| e.message),
            Err(e) => Err(e.to_string()),
        };
        let (done, total, percent, complete) = (
            entry.counter.done_bytes(),
            entry.counter.total_size(),
            entry.counter.percent(),
            entry.counter.is_complete(),
        );
        if let Err(error) = written {
            self.end_transfer(
                &transfer_id,
                Ending::Failed(error.clone()),
                false,
                Some(peer),
            )
            .await;
            self.ack(inbound, transfer_id, Err(error));
            return;
        }
        self.emit(UiEvent::TransferProgress {
            transfer_id: transfer_id.clone(),
            direction: Direction::Download,
            bytes_done: done,
            total_size: total,
            progress: percent,
        });
        let result = if complete {
            self.finish_incoming(&transfer_id).await
        } else {
            Ok(())
        };
        self.ack(inbound, transfer_id, result);
    }

    async fn finish_incoming(&mut self, transfer_id: &TransferId) -> Result<(), String> {
        let Some(t) = self.incoming.remove(transfer_id) else {
            return Err("전송 세션을 찾을 수 없습니다.".to_string());
        };
        let size = t.counter.total_size();
        match t.sink.commit().await {
            Ok(saved) => {
                let speed = speed_bps(size, t.started.elapsed());
                self.emit(UiEvent::TransferCompleted {
                    transfer_id: transfer_id.clone(),
                    direction: Direction::Download,
                    peer_id: t.peer.clone(),
                    file_name: t.file_name.clone(),
                    total_size: size,
                    saved_path: Some(saved.display().to_string()),
                    speed_bps: speed,
                });
                self.record(
                    Direction::Download,
                    &t.peer,
                    &t.file_name,
                    size,
                    TransferStatus::Completed,
                    speed,
                );
                Ok(())
            }
            Err(e) => {
                self.emit_failed(
                    transfer_id,
                    Direction::Download,
                    &t.peer,
                    &t.file_name,
                    &e.message,
                );
                self.record(
                    Direction::Download,
                    &t.peer,
                    &t.file_name,
                    size,
                    TransferStatus::Failed,
                    None,
                );
                Err(e.message)
            }
        }
    }

    pub(super) async fn on_stream_ack(&mut self, peer: PeerId, ack: StreamAck) {
        let id = ack.transfer_id;
        if self.outgoing.get(&id).is_some_and(|t| t.peer == peer) {
            match ack.result {
                Ok(()) => self.send_next_chunk(&id).await,
                Err(error) => {
                    self.end_transfer(&id, Ending::Failed(error), false, Some(&peer))
                        .await
                }
            }
        } else if self.pulls.get(&id).is_some_and(|p| p.peer == peer) {
            if let Err(error) = ack.result {
                self.end_transfer(&id, Ending::Failed(error), false, Some(&peer))
                    .await;
            }
        }
    }

    async fn send_next_chunk(&mut self, transfer_id: &TransferId) {
        let step = {
            let Some(t) = self.outgoing.get_mut(transfer_id) else {
                return;
            };
            if t.counter.is_complete() {
                None
            } else {
                Some(read_step(t).await)
            }
        };
        match step {
            None => self.complete_outgoing(transfer_id),
            Some(Ok(step)) => {
                self.send_net(NetCommand::SendStream {
                    peer: step.peer,
                    msg: StreamMsg::Chunk {
                        transfer_id: transfer_id.clone(),
                        index: step.index,
                        data: step.data,
                    },
                });
                self.emit(UiEvent::TransferProgress {
                    transfer_id: transfer_id.clone(),
                    direction: Direction::Upload,
                    bytes_done: step.done,
                    total_size: step.total,
                    progress: step.percent,
                });
            }
            Some(Err(error)) => {
                self.end_transfer(transfer_id, Ending::Failed(error), true, None)
                    .await
            }
        }
    }

    fn complete_outgoing(&mut self, transfer_id: &TransferId) {
        let Some(t) = self.outgoing.remove(transfer_id) else {
            return;
        };
        let size = t.counter.total_size();
        let speed = speed_bps(size, t.started.elapsed());
        self.emit(UiEvent::TransferCompleted {
            transfer_id: transfer_id.clone(),
            direction: Direction::Upload,
            peer_id: t.peer.clone(),
            file_name: t.file_name.clone(),
            total_size: size,
            saved_path: None,
            speed_bps: speed,
        });
        self.record(
            Direction::Upload,
            &t.peer,
            &t.file_name,
            size,
            TransferStatus::Completed,
            speed,
        );
    }

    pub(super) async fn cancel_by_user(&mut self, transfer_id: &TransferId) {
        self.end_transfer(
            transfer_id,
            Ending::Cancelled(CancelReason::UserCancelled),
            true,
            None,
        )
        .await;
    }

    pub(super) async fn fail_transfer(&mut self, transfer_id: &TransferId, error: String) {
        self.end_transfer(transfer_id, Ending::Failed(error), false, None)
            .await;
    }

    pub(super) async fn fail_transfers_of(&mut self, peer: &PeerId, error: &str) {
        let ids: Vec<TransferId> = self
            .outgoing
            .iter()
            .filter(|(_, t)| &t.peer == peer)
            .map(|(id, _)| id.clone())
            .chain(
                self.incoming
                    .iter()
                    .filter(|(_, t)| &t.peer == peer)
                    .map(|(id, _)| id.clone()),
            )
            .chain(
                self.pulls
                    .iter()
                    .filter(|(_, t)| &t.peer == peer)
                    .map(|(id, _)| id.clone()),
            )
            .collect();
        for id in ids {
            self.end_transfer(&id, Ending::Failed(error.to_string()), false, None)
                .await;
        }
    }

    fn remember_early_cancel(&mut self, transfer_id: &TransferId, peer: &PeerId) {
        let known = self.incoming.contains_key(transfer_id)
            || self.outgoing.contains_key(transfer_id)
            || self.pulls.contains_key(transfer_id);
        if known {
            return;
        }
        if self.early_cancels.len() >= EARLY_CANCEL_LIMIT {
            self.early_cancels.pop_front();
        }
        self.early_cancels
            .push_back((transfer_id.clone(), peer.clone(), Instant::now()));
    }

    fn take_early_cancel(&mut self, transfer_id: &TransferId, peer: &PeerId) -> bool {
        let found = self
            .early_cancels
            .iter()
            .position(|(id, from, _)| id == transfer_id && from == peer);
        found.is_some_and(|i| self.early_cancels.remove(i).is_some())
    }

    pub(super) async fn sweep_stalled(&mut self) {
        self.early_cancels
            .retain(|(_, _, at)| at.elapsed() <= STALL_TIMEOUT);
        let stalled: Vec<TransferId> = self
            .incoming
            .iter()
            .filter(|(_, t)| t.touched.elapsed() > STALL_TIMEOUT)
            .map(|(id, _)| id.clone())
            .chain(
                self.pulls
                    .iter()
                    .filter(|(_, p)| p.touched.elapsed() > STALL_TIMEOUT)
                    .map(|(id, _)| id.clone()),
            )
            .collect();
        for id in stalled {
            self.end_transfer(&id, Ending::Failed(STALLED.to_string()), true, None)
                .await;
        }
    }

    pub(super) async fn abort_all_transfers(&mut self) {
        let ids: Vec<TransferId> = self
            .outgoing
            .keys()
            .chain(self.incoming.keys())
            .chain(self.pulls.keys())
            .cloned()
            .collect();
        for id in ids {
            self.end_transfer(&id, Ending::Failed(ENGINE_STOPPED.to_string()), true, None)
                .await;
        }
    }

    async fn end_transfer(
        &mut self,
        transfer_id: &TransferId,
        ending: Ending,
        notify_peer: bool,
        only_peer: Option<&PeerId>,
    ) {
        let Some(taken) = self.take_transfer(transfer_id, only_peer) else {
            return;
        };
        if let Some(sink) = taken.sink {
            sink.abort().await;
        }
        if notify_peer {
            self.send_net(NetCommand::SendStream {
                peer: taken.peer.clone(),
                msg: StreamMsg::Cancel {
                    transfer_id: transfer_id.clone(),
                },
            });
        }
        let status = match ending {
            Ending::Cancelled(reason) => {
                self.emit(UiEvent::TransferCancelled {
                    transfer_id: transfer_id.clone(),
                    direction: taken.direction,
                    peer_id: taken.peer.clone(),
                    file_name: taken.file_name.clone(),
                    reason,
                });
                TransferStatus::Cancelled
            }
            Ending::Failed(error) => {
                self.emit_failed(
                    transfer_id,
                    taken.direction,
                    &taken.peer,
                    &taken.file_name,
                    &error,
                );
                TransferStatus::Failed
            }
        };
        self.record(
            taken.direction,
            &taken.peer,
            &taken.file_name,
            taken.size,
            status,
            None,
        );
    }

    fn take_transfer(
        &mut self,
        transfer_id: &TransferId,
        only_peer: Option<&PeerId>,
    ) -> Option<Taken> {
        let matches = |peer: &PeerId| only_peer.is_none_or(|p| p == peer);
        if self
            .outgoing
            .get(transfer_id)
            .is_some_and(|t| matches(&t.peer))
        {
            let t = self.outgoing.remove(transfer_id)?;
            return Some(Taken {
                direction: Direction::Upload,
                peer: t.peer,
                file_name: t.file_name,
                size: t.counter.total_size(),
                sink: None,
            });
        }
        if self
            .incoming
            .get(transfer_id)
            .is_some_and(|t| matches(&t.peer))
        {
            let t = self.incoming.remove(transfer_id)?;
            return Some(Taken {
                direction: Direction::Download,
                peer: t.peer,
                file_name: t.file_name,
                size: t.counter.total_size(),
                sink: Some(t.sink),
            });
        }
        if self
            .pulls
            .get(transfer_id)
            .is_some_and(|t| matches(&t.peer))
        {
            let t = self.pulls.remove(transfer_id)?;
            return Some(Taken {
                direction: Direction::Download,
                peer: t.peer,
                file_name: t.file_name,
                size: 0,
                sink: None,
            });
        }
        None
    }

    fn emit_failed(
        &self,
        transfer_id: &TransferId,
        direction: Direction,
        peer: &PeerId,
        file_name: &str,
        error: &str,
    ) {
        self.emit(UiEvent::TransferFailed {
            transfer_id: transfer_id.clone(),
            direction,
            peer_id: peer.clone(),
            file_name: file_name.to_string(),
            error: error.to_string(),
        });
    }
}

async fn read_step(t: &mut Outgoing) -> Result<Step, String> {
    let data = match t.source.next_chunk().await {
        Ok(Some(data)) => data,
        Ok(None) => return Err("파일이 전송 중에 줄어들었습니다.".to_string()),
        Err(e) => return Err(e.message),
    };
    let index = t.counter.next_index();
    t.counter
        .accept(index, data.len())
        .map_err(|e| e.to_string())?;
    Ok(Step {
        peer: t.peer.clone(),
        index,
        data,
        done: t.counter.done_bytes(),
        total: t.counter.total_size(),
        percent: t.counter.percent(),
    })
}

#[cfg(test)]
mod mutants_d {
    use super::*;
    use crate::adapters::fs::LocalFileStore;
    use crate::application::engine::{Engine, EngineDeps};
    use crate::application::ports::UiEvents;
    use crate::application::state::{ActivityStore, SettingsStore};
    use crate::domain::settings::Settings;
    use async_trait::async_trait;
    use std::collections::{HashMap, HashSet, VecDeque};
    use std::sync::{Arc, Mutex};
    use tokio::sync::mpsc;

    const PEER_A: &str = "12D3KooWAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

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

    struct NullSink;
    #[async_trait]
    impl ChunkSink for NullSink {
        async fn write(&mut self, _: &[u8]) -> AppResult<()> {
            Ok(())
        }
        async fn commit(self: Box<Self>) -> AppResult<std::path::PathBuf> {
            Ok(std::path::PathBuf::new())
        }
        async fn abort(self: Box<Self>) {}
    }

    fn engine() -> (Engine, Arc<Rec>, mpsc::UnboundedReceiver<NetCommand>) {
        let root = std::env::temp_dir().join(format!("mutd-unit-{}", uuid::Uuid::new_v4()));
        let ui = Arc::new(Rec::default());
        let deps = EngineDeps {
            ui: ui.clone(),
            files: Arc::new(LocalFileStore::new(root.join("downloads"))),
            settings: SettingsStore::new(Settings::with_home(Some(root))),
            activity: ActivityStore::default(),
            trust: Arc::new(NoTrust),
        };
        let (net, cmds) = mpsc::unbounded_channel();
        let engine = Engine {
            deps,
            folder_dirty: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            finished: Mutex::new(VecDeque::new()),
            net,
            trusted: HashSet::new(),
            connected: HashSet::new(),
            device_names: HashMap::new(),
            pending_pairings: HashMap::new(),
            trust_unreadable: false,
            early_cancels: VecDeque::new(),
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
            pulls: HashMap::new(),
        };
        (engine, ui, cmds)
    }

    struct NoTrust;
    #[async_trait]
    impl crate::application::ports::TrustStore for NoTrust {
        async fn load(&self) -> AppResult<Vec<PeerId>> {
            Ok(Vec::new())
        }
        async fn save(&self, _: &[PeerId]) -> AppResult<()> {
            Ok(())
        }
    }

    fn ago(by: Duration) -> Instant {
        Instant::now().checked_sub(by).unwrap()
    }

    fn incoming(touched: Instant) -> Incoming {
        Incoming {
            peer: PeerId::parse(PEER_A).unwrap(),
            file_name: "f".into(),
            sink: Box::new(NullSink),
            counter: ChunkCounter::new(10),
            started: touched,
            touched,
        }
    }

    fn pull(touched: Instant) -> PendingPull {
        PendingPull {
            peer: PeerId::parse(PEER_A).unwrap(),
            file_name: "f".into(),
            touched,
        }
    }

    fn tid(raw: &str) -> TransferId {
        TransferId::from_wire(raw.into())
    }

    #[tokio::test]
    async fn sweep_fails_only_transfers_idle_past_the_timeout() {
        let (mut e, ui, mut cmds) = engine();
        let stale = STALL_TIMEOUT + Duration::from_secs(1);
        let fresh = STALL_TIMEOUT - Duration::from_secs(30);
        e.incoming.insert(tid("in-stale"), incoming(ago(stale)));
        e.incoming.insert(tid("in-fresh"), incoming(ago(fresh)));
        e.pulls.insert(tid("pull-stale"), pull(ago(stale)));
        e.pulls.insert(tid("pull-fresh"), pull(ago(fresh)));
        e.sweep_stalled().await;
        assert!(!e.incoming.contains_key(&tid("in-stale")));
        assert!(e.incoming.contains_key(&tid("in-fresh")));
        assert!(!e.pulls.contains_key(&tid("pull-stale")));
        assert!(e.pulls.contains_key(&tid("pull-fresh")));
        assert_eq!(ui.count("transfer-failed"), 2);
        let mut cancelled = 0;
        while let Ok(c) = cmds.try_recv() {
            if matches!(
                c,
                NetCommand::SendStream {
                    msg: StreamMsg::Cancel { .. },
                    ..
                }
            ) {
                cancelled += 1;
            }
        }
        assert_eq!(cancelled, 2);
    }

    #[tokio::test]
    async fn sweep_forgets_only_old_early_cancels() {
        let (mut e, _ui, _cmds) = engine();
        let peer = PeerId::parse(PEER_A).unwrap();
        let stale = STALL_TIMEOUT + Duration::from_secs(1);
        e.early_cancels
            .push_back((tid("old"), peer.clone(), ago(stale)));
        e.early_cancels
            .push_back((tid("new"), peer.clone(), Instant::now()));
        e.sweep_stalled().await;
        assert!(!e.take_early_cancel(&tid("old"), &peer));
        assert!(e.take_early_cancel(&tid("new"), &peer));
    }

    #[tokio::test(start_paused = true)]
    async fn run_sweeps_every_quarter_of_the_stall_timeout() {
        let (mut e, ui, _cmds) = engine();
        e.pulls.insert(
            tid("pull"),
            pull(ago(STALL_TIMEOUT - Duration::from_millis(500))),
        );
        let (_commands, inbox) = mpsc::channel(8);
        let (_events, net_events) = mpsc::channel(8);
        let task = tokio::spawn(e.run(inbox, net_events));
        for _ in 0..20 {
            tokio::task::yield_now().await;
        }
        assert_eq!(ui.count("transfer-failed"), 0);
        std::thread::sleep(Duration::from_millis(700));
        tokio::time::advance(STALL_TIMEOUT / 4 + Duration::from_secs(1)).await;
        for _ in 0..200 {
            if ui.count("transfer-failed") > 0 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(
            ui.count("transfer-failed"),
            1,
            "멈춘 전송을 주기적으로 정리하지 않음"
        );
        task.abort();
    }
}
