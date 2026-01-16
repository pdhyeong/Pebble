use libp2p::{mdns, identify, ping, request_response, swarm::NetworkBehaviour, StreamProtocol};
use serde::{Deserialize, Serialize};
use futures::prelude::*;
use std::io;

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

// ===== Codec 구현 =====

#[derive(Debug, Clone, Default)]
pub struct FileListCodec;

#[async_trait::async_trait]
impl request_response::Codec for FileListCodec {
    type Protocol = StreamProtocol;
    type Request = FileListRequest;
    type Response = FileListResponse;

    async fn read_request<T>(&mut self, _protocol: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        serde_json::from_slice(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(&mut self, _protocol: &Self::Protocol, io: &mut T) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        serde_json::from_slice(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(&mut self, _protocol: &Self::Protocol, io: &mut T, req: Self::Request) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = serde_json::to_vec(&req)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        Ok(())
    }

    async fn write_response<T>(&mut self, _protocol: &Self::Protocol, io: &mut T, res: Self::Response) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = serde_json::to_vec(&res)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        Ok(())
    }
}

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

#[derive(Debug, Clone, Default)]
pub struct FileTransferCodec;

#[async_trait::async_trait]
impl request_response::Codec for FileTransferCodec {
    type Protocol = StreamProtocol;
    type Request = FileTransferRequest;
    type Response = FileTransferResponse;

    async fn read_request<T>(&mut self, _protocol: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        serde_json::from_slice(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(&mut self, _protocol: &Self::Protocol, io: &mut T) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        serde_json::from_slice(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(&mut self, _protocol: &Self::Protocol, io: &mut T, req: Self::Request) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = serde_json::to_vec(&req)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        Ok(())
    }

    async fn write_response<T>(&mut self, _protocol: &Self::Protocol, io: &mut T, res: Self::Response) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = serde_json::to_vec(&res)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        Ok(())
    }
}

// ===== 파일 업로드 프로토콜 (로컬 → 원격) =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUploadRequest {
    pub file_name: String,        // 파일 이름
    pub remote_path: String,      // 저장할 원격 경로 (예: "/" 또는 "/subfolder")
    pub data: Vec<u8>,            // 파일 데이터
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUploadResponse {
    pub file_name: String,
    pub saved_path: String,       // 실제 저장된 경로
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct FileUploadCodec;

#[async_trait::async_trait]
impl request_response::Codec for FileUploadCodec {
    type Protocol = StreamProtocol;
    type Request = FileUploadRequest;
    type Response = FileUploadResponse;

    async fn read_request<T>(&mut self, _protocol: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        serde_json::from_slice(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(&mut self, _protocol: &Self::Protocol, io: &mut T) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        serde_json::from_slice(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(&mut self, _protocol: &Self::Protocol, io: &mut T, req: Self::Request) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = serde_json::to_vec(&req)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        Ok(())
    }

    async fn write_response<T>(&mut self, _protocol: &Self::Protocol, io: &mut T, res: Self::Response) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = serde_json::to_vec(&res)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
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

#[derive(Debug, Clone, Default)]
pub struct DeviceInfoCodec;

#[async_trait::async_trait]
impl request_response::Codec for DeviceInfoCodec {
    type Protocol = StreamProtocol;
    type Request = DeviceInfoRequest;
    type Response = DeviceInfoResponse;

    async fn read_request<T>(&mut self, _protocol: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        serde_json::from_slice(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(&mut self, _protocol: &Self::Protocol, io: &mut T) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        serde_json::from_slice(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(&mut self, _protocol: &Self::Protocol, io: &mut T, req: Self::Request) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = serde_json::to_vec(&req)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        Ok(())
    }

    async fn write_response<T>(&mut self, _protocol: &Self::Protocol, io: &mut T, res: Self::Response) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = serde_json::to_vec(&res)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        Ok(())
    }
}

// ===== NetworkBehaviour =====

#[derive(NetworkBehaviour)]
pub struct MyBehaviour {
    pub mdns: mdns::tokio::Behaviour,
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
    pub file_list: request_response::Behaviour<FileListCodec>,
    pub file_transfer: request_response::Behaviour<FileTransferCodec>,
    pub file_upload: request_response::Behaviour<FileUploadCodec>,
    pub device_info: request_response::Behaviour<DeviceInfoCodec>,
}
