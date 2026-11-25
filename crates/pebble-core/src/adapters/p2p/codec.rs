use async_trait::async_trait;
use futures::prelude::*;
use libp2p::{request_response, StreamProtocol};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io;
use std::marker::PhantomData;

use crate::domain::transfer::CHUNK_SIZE;

pub trait WireFormat: Send + Sync + 'static {
    const MAX_MESSAGE_BYTES: usize;
    fn encode<T: Serialize>(value: &T) -> io::Result<Vec<u8>>;
    fn decode<T: DeserializeOwned>(bytes: &[u8]) -> io::Result<T>;
}

pub struct Json;

impl WireFormat for Json {
    const MAX_MESSAGE_BYTES: usize = 8 * 1024 * 1024;

    fn encode<T: Serialize>(value: &T) -> io::Result<Vec<u8>> {
        serde_json::to_vec(value).map_err(invalid_data)
    }

    fn decode<T: DeserializeOwned>(bytes: &[u8]) -> io::Result<T> {
        serde_json::from_slice(bytes).map_err(invalid_data)
    }
}

pub struct Bincode;

impl WireFormat for Bincode {
    const MAX_MESSAGE_BYTES: usize = CHUNK_SIZE + 64 * 1024;

    fn encode<T: Serialize>(value: &T) -> io::Result<Vec<u8>> {
        bincode::serialize(value).map_err(invalid_data)
    }

    fn decode<T: DeserializeOwned>(bytes: &[u8]) -> io::Result<T> {
        bincode::deserialize(bytes).map_err(invalid_data)
    }
}

type Marker<F, Req, Resp> = PhantomData<fn() -> (F, Req, Resp)>;

pub struct FramedCodec<F, Req, Resp>(Marker<F, Req, Resp>);

impl<F, Req, Resp> Clone for FramedCodec<F, Req, Resp> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<F, Req, Resp> Default for FramedCodec<F, Req, Resp> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[async_trait]
impl<F, Req, Resp> request_response::Codec for FramedCodec<F, Req, Resp>
where
    F: WireFormat,
    Req: Serialize + DeserializeOwned + Send + Sync + 'static,
    Resp: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    type Protocol = StreamProtocol;
    type Request = Req;
    type Response = Resp;

    async fn read_request<T>(&mut self, _: &StreamProtocol, io: &mut T) -> io::Result<Req>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_frame::<F, _, _>(io).await
    }

    async fn read_response<T>(&mut self, _: &StreamProtocol, io: &mut T) -> io::Result<Resp>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_frame::<F, _, _>(io).await
    }

    async fn write_request<T>(&mut self, _: &StreamProtocol, io: &mut T, req: Req) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_frame::<F, _, _>(io, &req).await
    }

    async fn write_response<T>(&mut self, _: &StreamProtocol, io: &mut T, res: Resp) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_frame::<F, _, _>(io, &res).await
    }
}

async fn read_frame<F, T, R>(io: &mut R) -> io::Result<T>
where
    F: WireFormat,
    T: DeserializeOwned,
    R: AsyncRead + Unpin + Send,
{
    let mut len_buf = [0u8; 4];
    io.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > F::MAX_MESSAGE_BYTES {
        return Err(too_large(len, F::MAX_MESSAGE_BYTES));
    }
    let mut buf = vec![0u8; len];
    io.read_exact(&mut buf).await?;
    F::decode(&buf)
}

async fn write_frame<F, T, W>(io: &mut W, value: &T) -> io::Result<()>
where
    F: WireFormat,
    T: Serialize,
    W: AsyncWrite + Unpin + Send,
{
    let data = F::encode(value)?;
    if data.len() > F::MAX_MESSAGE_BYTES {
        return Err(too_large(data.len(), F::MAX_MESSAGE_BYTES));
    }
    let len = u32::try_from(data.len()).map_err(invalid_data)?;
    io.write_all(&len.to_be_bytes()).await?;
    io.write_all(&data).await?;
    io.flush().await
}

fn too_large(len: usize, max: usize) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("message of {len} bytes exceeds limit of {max} bytes"),
    )
}

fn invalid_data<E: Into<Box<dyn std::error::Error + Send + Sync>>>(err: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use futures::io::Cursor;
    use request_response::Codec;
    use serde::Deserialize;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Msg {
        text: String,
    }

    type TestCodec = FramedCodec<Json, Msg, Msg>;

    fn protocol() -> StreamProtocol {
        StreamProtocol::new("/test/1.0.0")
    }

    #[test]
    fn roundtrips_a_message() {
        let mut codec = TestCodec::default();
        let mut wire = Cursor::new(Vec::new());
        let msg = Msg { text: "안녕".into() };
        block_on(codec.write_request(&protocol(), &mut wire, Msg { text: "안녕".into() })).unwrap();
        wire.set_position(0);
        let read = block_on(codec.read_request(&protocol(), &mut wire)).unwrap();
        assert_eq!(read, msg);
    }

    #[test]
    fn rejects_oversized_length_before_allocating() {
        let mut codec = TestCodec::default();
        let mut wire = Cursor::new(u32::MAX.to_be_bytes().to_vec());
        let err = block_on(codec.read_request(&protocol(), &mut wire)).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn bincode_limit_fits_one_chunk() {
        let chunk = vec![7u8; CHUNK_SIZE];
        let encoded = Bincode::encode(&(String::from("transfer-id"), 3u32, chunk)).unwrap();
        assert!(encoded.len() <= Bincode::MAX_MESSAGE_BYTES);
    }
}
