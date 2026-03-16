use futures::prelude::*;
use libp2p::{identify, mdns, ping, request_response, swarm::NetworkBehaviour, StreamProtocol};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io;
use std::marker::PhantomData;

// ===== 제네릭 JSON 코덱 =====

pub struct JsonCodec<Req, Resp> {
    _marker: PhantomData<(Req, Resp)>,
}

impl<Req, Resp> Clone for JsonCodec<Req, Resp> {
    fn clone(&self) -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<Req, Resp> fmt::Debug for JsonCodec<Req, Resp> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JsonCodec").finish()
    }
}

impl<Req, Resp> Default for JsonCodec<Req, Resp> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

use std::fmt;

#[async_trait::async_trait]
impl<Req, Resp> request_response::Codec for JsonCodec<Req, Resp>
where
    Req: Serialize + DeserializeOwned + Send + 'static,
    Resp: Serialize + DeserializeOwned + Send + 'static,
{
    type Protocol = StreamProtocol;
    type Request = Req;
    type Response = Resp;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data =
            serde_json::to_vec(&req).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data =
            serde_json::to_vec(&res).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        Ok(())
    }
}

// ===== 파일 목록 교환 프로토콜 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileListRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileListResponse {
    pub path: String,
    pub files: Vec<FileInfo>,
    pub error: Option<String>,
}

pub type FileListCodec = JsonCodec<FileListRequest, FileListResponse>;

// ===== 파일 전송 프로토콜 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferResponse {
    pub path: String,
    pub data: Option<Vec<u8>>,
    pub error: Option<String>,
}

pub type FileTransferCodec = JsonCodec<FileTransferRequest, FileTransferResponse>;

// ===== 파일 스트림 전송 프로토콜 (양방향) =====

// 전송 메시지 타입 정의
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileTransferStreamMsg {
    Init {
        file_name: String,
        total_size: u64,
        total_chunks: u32,
        transfer_id: String, // UUID for tracking
        remote_path: String,
    },
    Chunk {
        transfer_id: String,
        chunk_index: u32,
        data: Vec<u8>,
    },
    Cancel {
        transfer_id: String,
    },
}

// Request Wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferStreamRequest {
    pub msg: FileTransferStreamMsg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferStreamResponse {
    pub transfer_id: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct FileTransferStreamCodec;

#[async_trait::async_trait]
impl request_response::Codec for FileTransferStreamCodec {
    type Protocol = StreamProtocol;
    type Request = FileTransferStreamRequest;
    type Response = FileTransferStreamResponse;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        // Length-prefixed bincode decoding
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;

        bincode::deserialize(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;

        bincode::deserialize(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data =
            bincode::serialize(&req).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data =
            bincode::serialize(&res).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        Ok(())
    }
}

// ===== 기기 정보 프로토콜 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfoRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfoResponse {
    pub device_name: String,
}

pub type DeviceInfoCodec = JsonCodec<DeviceInfoRequest, DeviceInfoResponse>;

// ===== 페어링 인증 프로토콜 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingRequest {
    pub device_name: String,
    pub pin: String, // 6자리 PIN
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingResponse {
    pub approved: bool,
    pub device_name: String,
    pub shared_folder_name: Option<String>,
    pub error: Option<String>,
}

pub type PairingCodec = JsonCodec<PairingRequest, PairingResponse>;

// ===== 디렉토리 변경 알림 프로토콜 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryChangedRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryChangedResponse {}

pub type DirectoryChangedCodec = JsonCodec<DirectoryChangedRequest, DirectoryChangedResponse>;

#[derive(NetworkBehaviour)]
pub struct MyBehaviour {
    pub mdns: mdns::tokio::Behaviour,
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
    pub file_list: request_response::Behaviour<FileListCodec>,
    pub file_transfer: request_response::Behaviour<FileTransferCodec>,
    pub file_stream: request_response::Behaviour<FileTransferStreamCodec>,
    pub device_info: request_response::Behaviour<DeviceInfoCodec>,
    pub pairing: request_response::Behaviour<PairingCodec>,
    pub directory_changed: request_response::Behaviour<DirectoryChangedCodec>,
}
