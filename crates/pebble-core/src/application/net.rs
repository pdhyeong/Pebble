use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::domain::files::FileEntry;
use crate::domain::peer::PeerId;
use crate::domain::transfer::TransferId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InboundId(pub u64);

#[derive(Debug, Clone)]
pub struct FileListReply {
    pub path: String,
    pub files: Vec<FileEntry>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum StreamMsg {
    Pull {
        transfer_id: TransferId,
        path: String,
    },
    Init {
        transfer_id: TransferId,
        file_name: String,
        total_size: u64,
        remote_path: String,
        for_pull: bool,
    },
    Chunk {
        transfer_id: TransferId,
        index: u32,
        data: Vec<u8>,
    },
    Cancel {
        transfer_id: TransferId,
    },
}

impl StreamMsg {
    pub fn transfer_id(&self) -> &TransferId {
        match self {
            Self::Pull { transfer_id, .. }
            | Self::Init { transfer_id, .. }
            | Self::Chunk { transfer_id, .. }
            | Self::Cancel { transfer_id } => transfer_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StreamAck {
    pub transfer_id: TransferId,
    pub result: Result<(), String>,
}

#[derive(Debug, Clone)]
pub struct PairingAsk {
    pub device_name: String,
    pub pin: String,
}

#[derive(Debug, Clone)]
pub struct PairingReply {
    pub approved: bool,
    pub device_name: String,
    pub shared_folder_name: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug)]
pub enum NetCommand {
    Dial {
        address: String,
    },
    Probe {
        address: String,
    },
    RequestFileList {
        peer: PeerId,
        path: String,
    },
    RespondFileList {
        inbound: InboundId,
        reply: FileListReply,
    },
    SendStream {
        peer: PeerId,
        msg: StreamMsg,
    },
    RespondStream {
        inbound: InboundId,
        ack: StreamAck,
    },
    RequestPairing {
        peer: PeerId,
        ask: PairingAsk,
    },
    RespondPairing {
        inbound: InboundId,
        reply: PairingReply,
    },
    RequestDeviceInfo {
        peer: PeerId,
    },
    RespondDeviceInfo {
        inbound: InboundId,
        device_name: String,
    },
    NotifyDirectoryChanged {
        peers: Vec<PeerId>,
    },
    RespondDirectoryChanged {
        inbound: InboundId,
    },
    Shutdown,
}

#[derive(Debug)]
pub enum InboundRequest {
    FileList { path: String },
    Stream(StreamMsg),
    Pairing(PairingAsk),
    DeviceInfo,
    DirectoryChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestKind {
    FileList,
    Stream,
    Pairing,
    DeviceInfo,
    DirectoryChanged,
}

#[derive(Debug)]
pub enum NetEvent {
    ListeningOn {
        address: String,
    },
    PeerDiscovered {
        peer: PeerId,
        address: String,
    },
    PeerExpired {
        peer: PeerId,
    },
    Identified {
        peer: PeerId,
        agent_version: String,
    },
    Pinged {
        peer: PeerId,
        rtt: Duration,
    },
    DialStarted {
        address: String,
    },
    DialFailed {
        address: String,
        error: String,
    },
    Connected {
        peer: PeerId,
    },
    Disconnected {
        peer: PeerId,
    },
    Inbound {
        peer: PeerId,
        id: InboundId,
        request: InboundRequest,
    },
    FileListReplied {
        peer: PeerId,
        reply: FileListReply,
    },
    StreamAcked {
        peer: PeerId,
        ack: StreamAck,
    },
    PairingReplied {
        peer: PeerId,
        reply: PairingReply,
    },
    DeviceInfoReplied {
        peer: PeerId,
        device_name: String,
    },
    RequestFailed {
        peer: PeerId,
        kind: RequestKind,
        transfer_id: Option<TransferId>,
        error: String,
    },
}

pub struct NetworkLink {
    pub local_peer: PeerId,
    pub commands: mpsc::UnboundedSender<NetCommand>,
    pub events: mpsc::Receiver<NetEvent>,
    pub task: JoinHandle<()>,
}
