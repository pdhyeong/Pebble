use libp2p::request_response::{self, ProtocolSupport};
use libp2p::swarm::NetworkBehaviour;
use libp2p::{identify, identity, mdns, ping, StreamProtocol};
use std::error::Error;
use std::time::Duration;

use super::codec::{Bincode, FramedCodec, Json};
use super::protocol::{
    DeviceInfoRequestDto, DeviceInfoResponseDto, DirectoryChangedDto, FileListRequestDto,
    FileListResponseDto, PairingRequestDto, PairingResponseDto, StreamRequestDto,
    StreamResponseDto,
};

pub const AGENT_PROTOCOL: &str = "/pebble/1.0.0";
pub const FILE_LIST_PROTOCOL: &str = "/pebble/file-list/1.0.0";
pub const STREAM_PROTOCOL: &str = "/pebble/file-stream/2.0.0";
pub const DEVICE_INFO_PROTOCOL: &str = "/pebble/device-info/1.0.0";
pub const PAIRING_PROTOCOL: &str = "/pebble/pairing/1.0.0";
pub const DIR_CHANGED_PROTOCOL: &str = "/pebble/dir-changed/1.0.0";

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
pub const STREAM_TIMEOUT: Duration = Duration::from_secs(60);
pub const PAIRING_TIMEOUT: Duration = Duration::from_secs(180);

pub type FileListCodec = FramedCodec<Json, FileListRequestDto, FileListResponseDto>;
pub type StreamCodec = FramedCodec<Bincode, StreamRequestDto, StreamResponseDto>;
pub type DeviceInfoCodec = FramedCodec<Json, DeviceInfoRequestDto, DeviceInfoResponseDto>;
pub type PairingCodec = FramedCodec<Json, PairingRequestDto, PairingResponseDto>;
pub type DirChangedCodec = FramedCodec<Json, DirectoryChangedDto, DirectoryChangedDto>;

#[derive(NetworkBehaviour)]
pub struct PebbleBehaviour {
    pub mdns: mdns::tokio::Behaviour,
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
    pub file_list: request_response::Behaviour<FileListCodec>,
    pub stream: request_response::Behaviour<StreamCodec>,
    pub device_info: request_response::Behaviour<DeviceInfoCodec>,
    pub pairing: request_response::Behaviour<PairingCodec>,
    pub directory_changed: request_response::Behaviour<DirChangedCodec>,
}

pub fn new_behaviour(
    key: &identity::Keypair,
) -> Result<PebbleBehaviour, Box<dyn Error + Send + Sync>> {
    let peer_id = key.public().to_peer_id();
    Ok(PebbleBehaviour {
        mdns: mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id)?,
        identify: identify::Behaviour::new(identify::Config::new(
            AGENT_PROTOCOL.into(),
            key.public(),
        )),
        ping: ping::Behaviour::new(ping::Config::new()),
        file_list: request_response_behaviour(FILE_LIST_PROTOCOL, DEFAULT_TIMEOUT),
        stream: request_response_behaviour(STREAM_PROTOCOL, STREAM_TIMEOUT),
        device_info: request_response_behaviour(DEVICE_INFO_PROTOCOL, DEFAULT_TIMEOUT),
        pairing: request_response_behaviour(PAIRING_PROTOCOL, PAIRING_TIMEOUT),
        directory_changed: request_response_behaviour(DIR_CHANGED_PROTOCOL, DEFAULT_TIMEOUT),
    })
}

fn request_response_behaviour<C>(
    protocol: &'static str,
    timeout: Duration,
) -> request_response::Behaviour<C>
where
    C: request_response::Codec<Protocol = StreamProtocol> + Default + Clone + Send + 'static,
{
    let mut config = request_response::Config::default();
    config.set_request_timeout(timeout);
    request_response::Behaviour::new(
        [(StreamProtocol::new(protocol), ProtocolSupport::Full)],
        config,
    )
}
