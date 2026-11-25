use serde::{Deserialize, Serialize};

use crate::application::net::{FileListReply, PairingAsk, PairingReply, StreamAck, StreamMsg};
use crate::domain::files::FileEntry;
use crate::domain::transfer::TransferId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileListRequestDto {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfoDto {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileListResponseDto {
    pub path: String,
    pub files: Vec<FileInfoDto>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamRequestDto {
    Pull {
        transfer_id: String,
        path: String,
    },
    Init {
        transfer_id: String,
        file_name: String,
        total_size: u64,
        remote_path: String,
        for_pull: bool,
    },
    Chunk {
        transfer_id: String,
        index: u32,
        data: Vec<u8>,
    },
    Cancel {
        transfer_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResponseDto {
    pub transfer_id: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfoRequestDto {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfoResponseDto {
    pub device_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingRequestDto {
    pub device_name: String,
    pub pin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingResponseDto {
    pub approved: bool,
    pub device_name: String,
    pub shared_folder_name: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryChangedDto {}

impl From<FileEntry> for FileInfoDto {
    fn from(e: FileEntry) -> Self {
        Self {
            name: e.name,
            path: e.path,
            is_dir: e.is_dir,
            size: e.size,
        }
    }
}

impl From<FileInfoDto> for FileEntry {
    fn from(d: FileInfoDto) -> Self {
        Self {
            name: d.name,
            path: d.path,
            is_dir: d.is_dir,
            size: d.size,
        }
    }
}

impl From<FileListReply> for FileListResponseDto {
    fn from(r: FileListReply) -> Self {
        Self {
            path: r.path,
            files: r.files.into_iter().map(Into::into).collect(),
            error: r.error,
        }
    }
}

impl From<FileListResponseDto> for FileListReply {
    fn from(d: FileListResponseDto) -> Self {
        Self {
            path: d.path,
            files: d.files.into_iter().map(Into::into).collect(),
            error: d.error,
        }
    }
}

impl From<StreamMsg> for StreamRequestDto {
    fn from(m: StreamMsg) -> Self {
        match m {
            StreamMsg::Pull { transfer_id, path } => Self::Pull {
                transfer_id: transfer_id.to_string(),
                path,
            },
            StreamMsg::Init {
                transfer_id,
                file_name,
                total_size,
                remote_path,
                for_pull,
            } => Self::Init {
                transfer_id: transfer_id.to_string(),
                file_name,
                total_size,
                remote_path,
                for_pull,
            },
            StreamMsg::Chunk {
                transfer_id,
                index,
                data,
            } => Self::Chunk {
                transfer_id: transfer_id.to_string(),
                index,
                data,
            },
            StreamMsg::Cancel { transfer_id } => Self::Cancel {
                transfer_id: transfer_id.to_string(),
            },
        }
    }
}

impl From<StreamRequestDto> for StreamMsg {
    fn from(d: StreamRequestDto) -> Self {
        match d {
            StreamRequestDto::Pull { transfer_id, path } => Self::Pull {
                transfer_id: TransferId::from_wire(transfer_id),
                path,
            },
            StreamRequestDto::Init {
                transfer_id,
                file_name,
                total_size,
                remote_path,
                for_pull,
            } => Self::Init {
                transfer_id: TransferId::from_wire(transfer_id),
                file_name,
                total_size,
                remote_path,
                for_pull,
            },
            StreamRequestDto::Chunk {
                transfer_id,
                index,
                data,
            } => Self::Chunk {
                transfer_id: TransferId::from_wire(transfer_id),
                index,
                data,
            },
            StreamRequestDto::Cancel { transfer_id } => Self::Cancel {
                transfer_id: TransferId::from_wire(transfer_id),
            },
        }
    }
}

impl From<StreamAck> for StreamResponseDto {
    fn from(a: StreamAck) -> Self {
        let (success, error) = match a.result {
            Ok(()) => (true, None),
            Err(e) => (false, Some(e)),
        };
        Self {
            transfer_id: a.transfer_id.to_string(),
            success,
            error,
        }
    }
}

impl From<StreamResponseDto> for StreamAck {
    fn from(d: StreamResponseDto) -> Self {
        let result = if d.success {
            Ok(())
        } else {
            Err(d.error.unwrap_or_else(|| "상대 기기가 전송을 거부했습니다.".to_string()))
        };
        Self {
            transfer_id: TransferId::from_wire(d.transfer_id),
            result,
        }
    }
}

impl From<PairingAsk> for PairingRequestDto {
    fn from(a: PairingAsk) -> Self {
        Self {
            device_name: a.device_name,
            pin: a.pin,
        }
    }
}

impl From<PairingRequestDto> for PairingAsk {
    fn from(d: PairingRequestDto) -> Self {
        Self {
            device_name: d.device_name,
            pin: d.pin,
        }
    }
}

impl From<PairingReply> for PairingResponseDto {
    fn from(r: PairingReply) -> Self {
        Self {
            approved: r.approved,
            device_name: r.device_name,
            shared_folder_name: r.shared_folder_name,
            error: r.error,
        }
    }
}

impl From<PairingResponseDto> for PairingReply {
    fn from(d: PairingResponseDto) -> Self {
        Self {
            approved: d.approved,
            device_name: d.device_name,
            shared_folder_name: d.shared_folder_name,
            error: d.error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_ack_roundtrip_keeps_error() {
        let ack = StreamAck {
            transfer_id: TransferId::from_wire("t1".into()),
            result: Err("거부".into()),
        };
        let back: StreamAck = StreamResponseDto::from(ack).into();
        assert_eq!(back.transfer_id.as_str(), "t1");
        assert_eq!(back.result, Err("거부".to_string()));
    }

    #[test]
    fn json_protocols_keep_v1_wire_shape() {
        let json = serde_json::to_value(PairingRequestDto {
            device_name: "a".into(),
            pin: "123456".into(),
        })
        .unwrap();
        assert_eq!(json, serde_json::json!({"device_name": "a", "pin": "123456"}));
        assert_eq!(serde_json::to_string(&DeviceInfoRequestDto {}).unwrap(), "{}");
    }
}
