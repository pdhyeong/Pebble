use anyhow::{Context, Result};
use bytes::{BufMut, Bytes, BytesMut};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::SocketAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_rustls::{TlsAcceptor, TlsConnector};
use uuid::Uuid;

use super::certificate::TlsCertificate;
use super::integrity;

/// 청크 크기 (1MB)
pub const CHUNK_SIZE: usize = 1024 * 1024;

/// 전송 포트
pub const TRANSFER_PORT: u16 = 37846;

/// 최대 전송 속도 (bytes/sec) - 기본값: 무제한 (0)
pub const MAX_TRANSFER_RATE: u64 = 0;

/// 전송 프로토콜 메시지 타입
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TransferMessage {
    /// 전송 요청
    TransferRequest {
        transfer_id: String,
        file_path: String,
        file_size: u64,
        file_hash: String,
        total_chunks: u64,
    },

    /// 전송 수락
    TransferAccept {
        transfer_id: String,
        resume_from_chunk: u64,
    },

    /// 전송 거부
    TransferReject {
        transfer_id: String,
        reason: String,
    },

    /// 청크 데이터
    ChunkData {
        transfer_id: String,
        chunk_index: u64,
        chunk_hash: String,
        data: Vec<u8>,
    },

    /// 청크 확인
    ChunkAck {
        transfer_id: String,
        chunk_index: u64,
    },

    /// 전송 완료
    TransferComplete {
        transfer_id: String,
    },

    /// 에러
    Error {
        transfer_id: String,
        message: String,
    },
}

impl TransferMessage {
    /// 메시지를 바이트로 직렬화합니다.
    pub fn to_bytes(&self) -> Result<Bytes> {
        let json = serde_json::to_vec(self)
            .context("Failed to serialize transfer message")?;

        let mut buf = BytesMut::with_capacity(4 + json.len());
        buf.put_u32(json.len() as u32);
        buf.put_slice(&json);

        Ok(buf.freeze())
    }

    /// 바이트에서 메시지를 역직렬화합니다.
    pub async fn from_stream<S>(stream: &mut S) -> Result<Self>
    where
        S: AsyncReadExt + Unpin,
    {
        // 메시지 길이 읽기
        let msg_len = stream.read_u32().await
            .context("Failed to read message length")? as usize;

        // 메시지 데이터 읽기
        let mut buf = vec![0u8; msg_len];
        stream.read_exact(&mut buf).await
            .context("Failed to read message data")?;

        // 역직렬화
        let msg: TransferMessage = serde_json::from_slice(&buf)
            .context("Failed to deserialize transfer message")?;

        Ok(msg)
    }
}

/// 전송 진행률 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgress {
    pub transfer_id: String,
    pub file_path: String,
    pub total_chunks: u64,
    pub completed_chunks: u64,
    pub progress_percent: f64,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub transfer_rate_mbps: f64,
}

/// 전송 상태
#[derive(Debug, Clone, PartialEq)]
pub enum TransferStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl TransferStatus {
    pub fn to_string(&self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::InProgress => "InProgress",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }
}

/// 파일 전송 서버
///
/// TLS로 암호화된 TCP 연결을 통해 파일을 수신합니다.
pub struct TransferServer {
    cert: TlsCertificate,
    progress_tx: Option<mpsc::UnboundedSender<TransferProgress>>,
}

impl TransferServer {
    /// 새로운 전송 서버를 생성합니다.
    pub fn new(cert: TlsCertificate) -> Self {
        Self {
            cert,
            progress_tx: None,
        }
    }

    /// 진행률 채널을 설정합니다.
    pub fn set_progress_channel(&mut self, tx: mpsc::UnboundedSender<TransferProgress>) {
        self.progress_tx = Some(tx);
    }

    /// 서버를 시작합니다.
    pub async fn start(&self, bind_addr: SocketAddr) -> Result<()> {
        let server_config = self.cert.build_server_config()?;
        let acceptor = TlsAcceptor::from(server_config);

        let listener = TcpListener::bind(bind_addr).await
            .with_context(|| format!("Failed to bind to {}", bind_addr))?;

        log::info!("Transfer server listening on {}", bind_addr);

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    log::info!("Accepting connection from {}", peer_addr);

                    let acceptor = acceptor.clone();
                    let progress_tx = self.progress_tx.clone();

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(stream, acceptor, progress_tx).await {
                            log::error!("Error handling client {}: {}", peer_addr, e);
                        }
                    });
                }
                Err(e) => {
                    log::error!("Error accepting connection: {}", e);
                }
            }
        }
    }

    /// 클라이언트 연결을 처리합니다.
    async fn handle_client(
        stream: TcpStream,
        acceptor: TlsAcceptor,
        progress_tx: Option<mpsc::UnboundedSender<TransferProgress>>,
    ) -> Result<()> {
        // TLS 핸드셰이크
        let mut tls_stream = acceptor.accept(stream).await
            .context("TLS handshake failed")?;

        log::info!("TLS handshake successful");

        // 전송 요청 수신
        let msg = TransferMessage::from_stream(&mut tls_stream).await?;

        let (transfer_id, file_path, file_size, total_chunks) = match msg {
            TransferMessage::TransferRequest {
                transfer_id,
                file_path,
                file_size,
                file_hash: _,
                total_chunks,
            } => {
                log::info!("Received transfer request: {} ({} bytes, {} chunks)",
                    file_path, file_size, total_chunks);

                (transfer_id, file_path, file_size, total_chunks)
            }
            _ => {
                anyhow::bail!("Expected TransferRequest, got {:?}", msg);
            }
        };

        // 이어받기 지원: 기존 전송 상태 확인
        let resume_from_chunk = Self::get_resume_chunk(&transfer_id)?;

        // 전송 수락
        let accept_msg = TransferMessage::TransferAccept {
            transfer_id: transfer_id.clone(),
            resume_from_chunk,
        };

        tls_stream.write_all(&accept_msg.to_bytes()?).await?;

        log::info!("Transfer accepted. Resuming from chunk {}", resume_from_chunk);

        // 파일 수신
        Self::receive_file(
            &mut tls_stream,
            &transfer_id,
            &file_path,
            file_size,
            total_chunks,
            resume_from_chunk,
            progress_tx,
        )
        .await?;

        Ok(())
    }

    /// 이어받기 청크 인덱스를 가져옵니다.
    fn get_resume_chunk(transfer_id: &str) -> Result<u64> {
        let conn = Connection::open("pebble.db")?;

        let mut stmt = conn.prepare(
            "SELECT received_chunks FROM transfer_state WHERE transfer_id = ?1"
        )?;

        let result: Result<i64, _> = stmt.query_row(params![transfer_id], |row| row.get(0));

        Ok(result.unwrap_or(0) as u64)
    }

    /// 파일을 수신합니다.
    async fn receive_file<S>(
        stream: &mut S,
        transfer_id: &str,
        file_path: &str,
        file_size: u64,
        total_chunks: u64,
        resume_from: u64,
        progress_tx: Option<mpsc::UnboundedSender<TransferProgress>>,
    ) -> Result<()>
    where
        S: AsyncReadExt + AsyncWriteExt + Unpin,
    {
        // 파일 열기 (이어받기 지원)
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(file_path)
            .with_context(|| format!("Failed to open file: {}", file_path))?;

        // 이어받기 위치로 이동
        if resume_from > 0 {
            let offset = resume_from * CHUNK_SIZE as u64;
            file.seek(SeekFrom::Start(offset))?;
            log::info!("Resuming from offset {}", offset);
        }

        let mut received_chunks = resume_from;
        let start_time = SystemTime::now();

        // 청크 수신 루프
        while received_chunks < total_chunks {
            let msg = TransferMessage::from_stream(stream).await?;

            match msg {
                TransferMessage::ChunkData {
                    chunk_index,
                    chunk_hash,
                    data,
                    ..
                } => {
                    // 청크 해시 검증
                    let computed_hash = {
                        use sha2::{Digest, Sha256};
                        let mut hasher = Sha256::new();
                        hasher.update(&data);
                        hex::encode(hasher.finalize())
                    };

                    if computed_hash != chunk_hash {
                        anyhow::bail!("Chunk hash mismatch at index {}", chunk_index);
                    }

                    // 파일에 쓰기
                    file.write_all(&data)?;

                    received_chunks += 1;

                    // 청크 확인 전송
                    let ack_msg = TransferMessage::ChunkAck {
                        transfer_id: transfer_id.to_string(),
                        chunk_index,
                    };
                    stream.write_all(&ack_msg.to_bytes()?).await?;

                    // DB 업데이트
                    Self::update_transfer_state(transfer_id, received_chunks)?;

                    // 진행률 전송
                    if let Some(ref tx) = progress_tx {
                        let elapsed = start_time.elapsed().unwrap_or(Duration::from_secs(1));
                        let bytes_transferred = received_chunks * CHUNK_SIZE as u64;
                        let transfer_rate = (bytes_transferred as f64 / elapsed.as_secs_f64()) / 1_000_000.0;

                        let progress = TransferProgress {
                            transfer_id: transfer_id.to_string(),
                            file_path: file_path.to_string(),
                            total_chunks,
                            completed_chunks: received_chunks,
                            progress_percent: (received_chunks as f64 / total_chunks as f64) * 100.0,
                            bytes_transferred,
                            total_bytes: file_size,
                            transfer_rate_mbps: transfer_rate,
                        };

                        let _ = tx.send(progress);
                    }

                    log::debug!("Received chunk {}/{} ({:.1}%)",
                        received_chunks, total_chunks,
                        (received_chunks as f64 / total_chunks as f64) * 100.0);
                }
                TransferMessage::TransferComplete { .. } => {
                    log::info!("Transfer completed");
                    break;
                }
                _ => {
                    log::warn!("Unexpected message: {:?}", msg);
                }
            }
        }

        file.flush()?;

        log::info!("File received successfully: {}", file_path);

        Ok(())
    }

    /// 전송 상태를 DB에 업데이트합니다.
    fn update_transfer_state(transfer_id: &str, received_chunks: u64) -> Result<()> {
        let conn = Connection::open("pebble.db")?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs() as i64;

        conn.execute(
            "INSERT OR REPLACE INTO transfer_state
             (transfer_id, file_path, file_size, total_chunks, received_chunks, transfer_status, peer_device_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(transfer_id) DO UPDATE SET
                received_chunks = excluded.received_chunks,
                updated_at = excluded.updated_at",
            params![
                transfer_id,
                "",
                0i64,
                0i64,
                received_chunks as i64,
                TransferStatus::InProgress.to_string(),
                "",
                now,
                now
            ],
        )?;

        Ok(())
    }
}

/// 파일 전송 클라이언트
///
/// TLS로 암호화된 TCP 연결을 통해 파일을 송신합니다.
pub struct TransferClient {
    server_fingerprint: Option<String>,
    progress_tx: Option<mpsc::UnboundedSender<TransferProgress>>,
}

impl TransferClient {
    /// 새로운 전송 클라이언트를 생성합니다.
    pub fn new(server_fingerprint: Option<String>) -> Self {
        Self {
            server_fingerprint,
            progress_tx: None,
        }
    }

    /// 진행률 채널을 설정합니다.
    pub fn set_progress_channel(&mut self, tx: mpsc::UnboundedSender<TransferProgress>) {
        self.progress_tx = Some(tx);
    }

    /// 파일을 전송합니다.
    pub async fn send_file(
        &self,
        server_addr: SocketAddr,
        file_path: &str,
    ) -> Result<()> {
        // 파일 정보 가져오기
        let file_metadata = std::fs::metadata(file_path)
            .with_context(|| format!("Failed to get file metadata: {}", file_path))?;

        let file_size = file_metadata.len();
        let total_chunks = (file_size + CHUNK_SIZE as u64 - 1) / CHUNK_SIZE as u64;

        // 파일 해시 계산
        let file_hash = integrity::calculate_file_hash(file_path)?;

        let transfer_id = Uuid::new_v4().to_string();

        log::info!("Starting file transfer: {} ({} bytes, {} chunks)",
            file_path, file_size, total_chunks);

        // TCP 연결
        let tcp_stream = TcpStream::connect(server_addr).await
            .with_context(|| format!("Failed to connect to {}", server_addr))?;

        // TLS 핸드셰이크
        let client_config = TlsCertificate::build_client_config(self.server_fingerprint.clone())?;
        let connector = TlsConnector::from(client_config);

        let domain = rustls::pki_types::ServerName::try_from("pebble.local")
            .map_err(|_| anyhow::anyhow!("Invalid DNS name"))?;

        let mut tls_stream = connector.connect(domain, tcp_stream).await
            .context("TLS handshake failed")?;

        log::info!("TLS handshake successful");

        // 전송 요청 전송
        let request_msg = TransferMessage::TransferRequest {
            transfer_id: transfer_id.clone(),
            file_path: file_path.to_string(),
            file_size,
            file_hash: file_hash.clone(),
            total_chunks,
        };

        tls_stream.write_all(&request_msg.to_bytes()?).await?;

        // 전송 수락 대기
        let response = TransferMessage::from_stream(&mut tls_stream).await?;

        let resume_from_chunk = match response {
            TransferMessage::TransferAccept { resume_from_chunk, .. } => {
                log::info!("Transfer accepted. Resuming from chunk {}", resume_from_chunk);
                resume_from_chunk
            }
            TransferMessage::TransferReject { reason, .. } => {
                anyhow::bail!("Transfer rejected: {}", reason);
            }
            _ => {
                anyhow::bail!("Expected TransferAccept or TransferReject");
            }
        };

        // 파일 전송
        self.send_file_chunks(
            &mut tls_stream,
            &transfer_id,
            file_path,
            file_size,
            total_chunks,
            resume_from_chunk,
        )
        .await?;

        // 전송 완료 메시지
        let complete_msg = TransferMessage::TransferComplete {
            transfer_id: transfer_id.clone(),
        };

        tls_stream.write_all(&complete_msg.to_bytes()?).await?;

        log::info!("File transfer completed successfully");

        Ok(())
    }

    /// 파일 청크를 전송합니다.
    async fn send_file_chunks<S>(
        &self,
        stream: &mut S,
        transfer_id: &str,
        file_path: &str,
        file_size: u64,
        total_chunks: u64,
        resume_from: u64,
    ) -> Result<()>
    where
        S: AsyncReadExt + AsyncWriteExt + Unpin,
    {
        let mut file = File::open(file_path)
            .with_context(|| format!("Failed to open file: {}", file_path))?;

        // 이어보내기 위치로 이동
        if resume_from > 0 {
            let offset = resume_from * CHUNK_SIZE as u64;
            file.seek(SeekFrom::Start(offset))?;
            log::info!("Resuming from chunk {}", resume_from);
        }

        let start_time = SystemTime::now();
        let mut buffer = vec![0u8; CHUNK_SIZE];

        for chunk_index in resume_from..total_chunks {
            // 청크 읽기
            let bytes_read = file.read(&mut buffer)?;

            if bytes_read == 0 {
                break;
            }

            let chunk_data = &buffer[..bytes_read];

            // 청크 해시 계산
            let chunk_hash = {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(chunk_data);
                hex::encode(hasher.finalize())
            };

            // 청크 전송
            let chunk_msg = TransferMessage::ChunkData {
                transfer_id: transfer_id.to_string(),
                chunk_index,
                chunk_hash,
                data: chunk_data.to_vec(),
            };

            stream.write_all(&chunk_msg.to_bytes()?).await?;

            // ACK 대기
            let ack = TransferMessage::from_stream(stream).await?;

            match ack {
                TransferMessage::ChunkAck { chunk_index: ack_idx, .. } => {
                    if ack_idx != chunk_index {
                        anyhow::bail!("Chunk ACK mismatch: expected {}, got {}", chunk_index, ack_idx);
                    }
                }
                _ => {
                    anyhow::bail!("Expected ChunkAck");
                }
            }

            // 진행률 전송
            if let Some(ref tx) = self.progress_tx {
                let elapsed = start_time.elapsed().unwrap_or(Duration::from_secs(1));
                let bytes_transferred = (chunk_index + 1) * CHUNK_SIZE as u64;
                let transfer_rate = (bytes_transferred as f64 / elapsed.as_secs_f64()) / 1_000_000.0;

                let progress = TransferProgress {
                    transfer_id: transfer_id.to_string(),
                    file_path: file_path.to_string(),
                    total_chunks,
                    completed_chunks: chunk_index + 1,
                    progress_percent: ((chunk_index + 1) as f64 / total_chunks as f64) * 100.0,
                    bytes_transferred,
                    total_bytes: file_size,
                    transfer_rate_mbps: transfer_rate,
                };

                let _ = tx.send(progress);
            }

            // Flow Control: 전송 속도 제한
            if MAX_TRANSFER_RATE > 0 {
                let elapsed = start_time.elapsed().unwrap_or(Duration::from_secs(1));
                let bytes_transferred = (chunk_index + 1) * CHUNK_SIZE as u64;
                let expected_duration = Duration::from_secs_f64(bytes_transferred as f64 / MAX_TRANSFER_RATE as f64);

                if elapsed < expected_duration {
                    tokio::time::sleep(expected_duration - elapsed).await;
                }
            }

            log::debug!("Sent chunk {}/{} ({:.1}%)",
                chunk_index + 1, total_chunks,
                ((chunk_index + 1) as f64 / total_chunks as f64) * 100.0);
        }

        Ok(())
    }
}
