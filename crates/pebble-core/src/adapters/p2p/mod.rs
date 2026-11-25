mod behaviour;
mod codec;
mod protocol;

use futures::StreamExt;
use libp2p::identity::Keypair;
use libp2p::multiaddr::Protocol;
use libp2p::request_response::{self, Message, ResponseChannel};
use libp2p::swarm::dial_opts::DialOpts;
use libp2p::swarm::{ConnectionId, SwarmEvent};
use libp2p::{identify, mdns, noise, ping, tcp, yamux, Multiaddr, Swarm, SwarmBuilder};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::net::IpAddr;
use std::path::Path;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::application::net::{
    InboundId, InboundRequest, NetCommand, NetEvent, NetworkLink, RequestKind,
};
use crate::application::ports::NetworkFactory;
use crate::domain::error::{AppError, AppResult, ErrorCode};
use crate::domain::peer::PeerId;
use crate::domain::transfer::TransferId;
use behaviour::{PebbleBehaviour, PebbleBehaviourEvent};
use protocol::{
    DeviceInfoRequestDto, DeviceInfoResponseDto, DirectoryChangedDto, FileListRequestDto,
    FileListResponseDto, PairingResponseDto, StreamResponseDto,
};

pub const EVENT_BUFFER: usize = 256;
pub const IDLE_CONNECTION_TIMEOUT: Duration = Duration::from_secs(60 * 60);

pub const DEFAULT_PORT: u16 = 47123;

pub struct Libp2pNetwork {
    identity: Keypair,
    port: u16,
}

impl Libp2pNetwork {
    pub fn ephemeral(port: u16) -> Self {
        Self {
            identity: Keypair::generate_ed25519(),
            port,
        }
    }

    pub fn persistent(identity_file: &Path, port: u16) -> AppResult<Self> {
        Ok(Self {
            identity: load_or_create_identity(identity_file)?,
            port,
        })
    }
}

fn load_or_create_identity(path: &Path) -> AppResult<Keypair> {
    if let Ok(bytes) = std::fs::read(path) {
        return Keypair::from_protobuf_encoding(&bytes).map_err(|e| {
            AppError::new(
                ErrorCode::Internal,
                format!("기기 키 파일이 손상되었습니다 ({}): {e}", path.display()),
            )
        });
    }
    let identity = Keypair::generate_ed25519();
    let bytes = identity
        .to_protobuf_encoding()
        .map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))?;
    if let Some(dir) = path.parent() {
        create_private_dir(dir).map_err(|e| AppError::io("설정 폴더를 만들지 못했습니다", &e))?;
    }
    write_private(path, &bytes).map_err(|e| AppError::io("기기 키를 저장하지 못했습니다", &e))?;
    Ok(identity)
}

#[cfg(unix)]
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)
}

#[cfg(unix)]
fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
}

#[cfg(not(unix))]
fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)
}

#[cfg(not(unix))]
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, bytes)
}

impl NetworkFactory for Libp2pNetwork {
    fn start(&self) -> AppResult<NetworkLink> {
        let swarm = build_swarm(self.identity.clone(), self.port)
            .map_err(|e| AppError::new(ErrorCode::Network, format!("네트워크를 시작하지 못했습니다. 포트 {}를 다른 프로그램(다른 Pebble 포함)이 쓰고 있을 수 있습니다: {e:?}", self.port)))?;
        let local_peer = domain_peer(swarm.local_peer_id());
        let (commands, inbox) = mpsc::unbounded_channel();
        let (events, events_rx) = mpsc::channel(EVENT_BUFFER);
        let actor = NetworkActor {
            swarm,
            events,
            inbound: HashMap::new(),
            next_inbound: 0,
            stream_requests: HashMap::new(),
            quic_addresses: HashMap::new(),
            quic_dialed: HashSet::new(),
            connections: HashMap::new(),
            probes: HashMap::new(),
        };
        let task = tokio::spawn(actor.run(inbox));
        Ok(NetworkLink {
            local_peer,
            commands,
            events: events_rx,
            task,
        })
    }
}

fn build_swarm(identity: Keypair, port: u16) -> Result<Swarm<PebbleBehaviour>, Box<dyn Error>> {
    let mut swarm = SwarmBuilder::with_existing_identity(identity)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_behaviour(behaviour::new_behaviour)?
        .with_swarm_config(|c| c.with_idle_connection_timeout(IDLE_CONNECTION_TIMEOUT))
        .build();
    swarm.listen_on(format!("/ip4/0.0.0.0/tcp/{port}").parse()?)?;
    swarm.listen_on(format!("/ip4/0.0.0.0/udp/{port}/quic-v1").parse()?)?;
    Ok(swarm)
}

fn domain_peer(peer: &libp2p::PeerId) -> PeerId {
    PeerId::parse(&peer.to_base58()).expect("libp2p PeerId is always base58")
}

enum PendingResponse {
    FileList(ResponseChannel<FileListResponseDto>),
    Stream(ResponseChannel<StreamResponseDto>),
    Pairing(ResponseChannel<PairingResponseDto>),
    DeviceInfo(ResponseChannel<DeviceInfoResponseDto>),
    DirectoryChanged(ResponseChannel<DirectoryChangedDto>),
}

struct NetworkActor {
    swarm: Swarm<PebbleBehaviour>,
    events: mpsc::Sender<NetEvent>,
    inbound: HashMap<InboundId, (libp2p::PeerId, PendingResponse)>,
    next_inbound: u64,
    stream_requests: HashMap<request_response::RequestId, TransferId>,
    quic_addresses: HashMap<libp2p::PeerId, Multiaddr>,
    quic_dialed: HashSet<libp2p::PeerId>,
    connections: HashMap<ConnectionId, IpAddr>,
    probes: HashMap<ConnectionId, IpAddr>,
}

fn ip_of(addr: &Multiaddr) -> Option<IpAddr> {
    addr.iter().find_map(|p| match p {
        Protocol::Ip4(ip) => Some(IpAddr::V4(ip)),
        Protocol::Ip6(ip) => Some(IpAddr::V6(ip)),
        _ => None,
    })
}

impl NetworkActor {
    async fn run(mut self, mut inbox: mpsc::UnboundedReceiver<NetCommand>) {
        loop {
            let keep_going = tokio::select! {
                command = inbox.recv() => match command {
                    None | Some(NetCommand::Shutdown) => false,
                    Some(command) => self.on_command(command).await,
                },
                event = self.swarm.select_next_some() => self.on_swarm_event(event).await,
            };
            if !keep_going {
                break;
            }
        }
    }

    async fn emit(&mut self, event: NetEvent) -> bool {
        self.events.send(event).await.is_ok()
    }

    async fn on_command(&mut self, command: NetCommand) -> bool {
        match command {
            NetCommand::Dial { address } => return self.dial(address).await,
            NetCommand::Probe { address } => self.probe(&address),
            NetCommand::RequestFileList { peer, path } => {
                if let Some(p) = self.resolve(&peer, RequestKind::FileList, None).await {
                    self.swarm
                        .behaviour_mut()
                        .file_list
                        .send_request(&p, FileListRequestDto { path });
                }
            }
            NetCommand::RespondFileList { inbound, reply } => {
                if let Some(PendingResponse::FileList(ch)) = self.take_inbound(inbound) {
                    let _ = self
                        .swarm
                        .behaviour_mut()
                        .file_list
                        .send_response(ch, reply.into());
                }
            }
            NetCommand::SendStream { peer, msg } => {
                let transfer_id = msg.transfer_id().clone();
                let is_init = matches!(msg, crate::application::net::StreamMsg::Init { .. });
                if let Some(p) = self
                    .resolve(&peer, RequestKind::Stream, Some(transfer_id.clone()))
                    .await
                {
                    if is_init {
                        self.prefer_quic(p);
                    }
                    let request_id = self
                        .swarm
                        .behaviour_mut()
                        .stream
                        .send_request(&p, msg.into());
                    self.stream_requests.insert(request_id, transfer_id);
                }
            }
            NetCommand::RespondStream { inbound, ack } => {
                if let Some(PendingResponse::Stream(ch)) = self.take_inbound(inbound) {
                    let _ = self
                        .swarm
                        .behaviour_mut()
                        .stream
                        .send_response(ch, ack.into());
                }
            }
            NetCommand::RequestPairing { peer, ask } => {
                if let Some(p) = self.resolve(&peer, RequestKind::Pairing, None).await {
                    self.swarm
                        .behaviour_mut()
                        .pairing
                        .send_request(&p, ask.into());
                }
            }
            NetCommand::RespondPairing { inbound, reply } => {
                if let Some(PendingResponse::Pairing(ch)) = self.take_inbound(inbound) {
                    let _ = self
                        .swarm
                        .behaviour_mut()
                        .pairing
                        .send_response(ch, reply.into());
                }
            }
            NetCommand::RequestDeviceInfo { peer } => {
                if let Some(p) = self.resolve(&peer, RequestKind::DeviceInfo, None).await {
                    self.swarm
                        .behaviour_mut()
                        .device_info
                        .send_request(&p, DeviceInfoRequestDto {});
                }
            }
            NetCommand::RespondDeviceInfo {
                inbound,
                device_name,
            } => {
                if let Some(PendingResponse::DeviceInfo(ch)) = self.take_inbound(inbound) {
                    let _ = self
                        .swarm
                        .behaviour_mut()
                        .device_info
                        .send_response(ch, DeviceInfoResponseDto { device_name });
                }
            }
            NetCommand::NotifyDirectoryChanged { peers } => {
                for peer in peers {
                    if let Some(p) = self
                        .resolve(&peer, RequestKind::DirectoryChanged, None)
                        .await
                    {
                        self.swarm
                            .behaviour_mut()
                            .directory_changed
                            .send_request(&p, DirectoryChangedDto {});
                    }
                }
            }
            NetCommand::RespondDirectoryChanged { inbound } => {
                if let Some(PendingResponse::DirectoryChanged(ch)) = self.take_inbound(inbound) {
                    let _ = self
                        .swarm
                        .behaviour_mut()
                        .directory_changed
                        .send_response(ch, DirectoryChangedDto {});
                }
            }
            NetCommand::Shutdown => return false,
        }
        true
    }

    async fn dial(&mut self, address: String) -> bool {
        let result = address
            .parse::<Multiaddr>()
            .map_err(|e| format!("주소 파싱 실패: {e}"))
            .and_then(|addr| self.swarm.dial(addr).map_err(|e| e.to_string()));
        let event = match result {
            Ok(()) => NetEvent::DialStarted { address },
            Err(error) => NetEvent::DialFailed { address, error },
        };
        self.emit(event).await
    }

    fn probe(&mut self, address: &str) {
        let Ok(addr) = address.parse::<Multiaddr>() else {
            return;
        };
        let Some(ip) = ip_of(&addr) else {
            return;
        };
        let busy = self
            .connections
            .values()
            .chain(self.probes.values())
            .any(|known| *known == ip);
        if busy {
            return;
        }
        let opts = DialOpts::unknown_peer_id().address(addr).build();
        let id = opts.connection_id();
        if self.swarm.dial(opts).is_ok() {
            self.probes.insert(id, ip);
        }
    }

    async fn resolve(
        &mut self,
        peer: &PeerId,
        kind: RequestKind,
        transfer_id: Option<TransferId>,
    ) -> Option<libp2p::PeerId> {
        match peer.as_str().parse::<libp2p::PeerId>() {
            Ok(p) => Some(p),
            Err(e) => {
                self.emit(NetEvent::RequestFailed {
                    peer: peer.clone(),
                    kind,
                    transfer_id,
                    error: format!("잘못된 PeerId: {e}"),
                })
                .await;
                None
            }
        }
    }

    fn prefer_quic(&mut self, peer: libp2p::PeerId) {
        if self.quic_dialed.contains(&peer) {
            return;
        }
        if let Some(addr) = self.quic_addresses.get(&peer).cloned() {
            if self.swarm.dial(addr).is_ok() {
                self.quic_dialed.insert(peer);
            }
        }
    }

    fn store_inbound(&mut self, peer: libp2p::PeerId, pending: PendingResponse) -> InboundId {
        self.next_inbound += 1;
        let id = InboundId(self.next_inbound);
        self.inbound.insert(id, (peer, pending));
        id
    }

    fn take_inbound(&mut self, id: InboundId) -> Option<PendingResponse> {
        self.inbound.remove(&id).map(|(_, pending)| pending)
    }

    async fn on_swarm_event<E>(&mut self, event: SwarmEvent<PebbleBehaviourEvent, E>) -> bool {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                self.emit(NetEvent::ListeningOn {
                    address: address.to_string(),
                })
                .await
            }
            SwarmEvent::ConnectionEstablished {
                peer_id,
                connection_id,
                endpoint,
                num_established,
                ..
            } => {
                let remote = endpoint.get_remote_address().clone();
                if let Some(ip) = ip_of(&remote) {
                    self.connections.insert(connection_id, ip);
                }
                let probed = self.probes.remove(&connection_id).is_some();
                if probed {
                    let found = NetEvent::PeerDiscovered {
                        peer: domain_peer(&peer_id),
                        address: remote.to_string(),
                    };
                    if !self.emit(found).await {
                        return false;
                    }
                }
                if num_established.get() != 1 {
                    return true;
                }
                self.emit(NetEvent::Connected {
                    peer: domain_peer(&peer_id),
                })
                .await
            }
            SwarmEvent::OutgoingConnectionError { connection_id, .. } => {
                self.probes.remove(&connection_id);
                true
            }
            SwarmEvent::ConnectionClosed {
                peer_id,
                connection_id,
                num_established,
                ..
            } => {
                self.connections.remove(&connection_id);
                if num_established > 0 {
                    return true;
                }
                self.inbound.retain(|_, (p, _)| p != &peer_id);
                self.quic_dialed.remove(&peer_id);
                self.emit(NetEvent::Disconnected {
                    peer: domain_peer(&peer_id),
                })
                .await
            }
            SwarmEvent::Behaviour(event) => self.on_behaviour_event(event).await,
            _ => true,
        }
    }

    async fn on_behaviour_event(&mut self, event: PebbleBehaviourEvent) -> bool {
        let translated = match event {
            PebbleBehaviourEvent::Mdns(event) => return self.on_mdns(event).await,
            PebbleBehaviourEvent::Identify(identify::Event::Received { peer_id, info }) => {
                Some(NetEvent::Identified {
                    peer: domain_peer(&peer_id),
                    agent_version: info.agent_version,
                })
            }
            PebbleBehaviourEvent::Identify(_) => None,
            PebbleBehaviourEvent::Ping(ping::Event {
                peer,
                result: Ok(rtt),
                ..
            }) => Some(NetEvent::Pinged {
                peer: domain_peer(&peer),
                rtt,
            }),
            PebbleBehaviourEvent::Ping(_) => None,
            PebbleBehaviourEvent::FileList(event) => self.on_file_list(event),
            PebbleBehaviourEvent::Stream(event) => self.on_stream(event),
            PebbleBehaviourEvent::DeviceInfo(event) => self.on_device_info(event),
            PebbleBehaviourEvent::Pairing(event) => self.on_pairing(event),
            PebbleBehaviourEvent::DirectoryChanged(event) => self.on_directory_changed(event),
        };
        match translated {
            Some(event) => self.emit(event).await,
            None => true,
        }
    }

    async fn on_mdns(&mut self, event: mdns::Event) -> bool {
        match event {
            mdns::Event::Discovered(list) => {
                for (peer, addr) in list {
                    if addr.to_string().contains("quic") {
                        self.quic_addresses.insert(peer, addr);
                        continue;
                    }
                    let event = NetEvent::PeerDiscovered {
                        peer: domain_peer(&peer),
                        address: addr.to_string(),
                    };
                    if !self.emit(event).await {
                        return false;
                    }
                }
                true
            }
            mdns::Event::Expired(list) => {
                let expired: HashSet<libp2p::PeerId> =
                    list.into_iter().map(|(peer, _)| peer).collect();
                for peer in expired {
                    self.quic_addresses.remove(&peer);
                    if !self
                        .emit(NetEvent::PeerExpired {
                            peer: domain_peer(&peer),
                        })
                        .await
                    {
                        return false;
                    }
                }
                true
            }
        }
    }

    fn on_file_list(
        &mut self,
        event: request_response::Event<FileListRequestDto, FileListResponseDto>,
    ) -> Option<NetEvent> {
        match event {
            request_response::Event::Message {
                peer,
                message:
                    Message::Request {
                        request, channel, ..
                    },
            } => Some(NetEvent::Inbound {
                peer: domain_peer(&peer),
                id: self.store_inbound(peer, PendingResponse::FileList(channel)),
                request: InboundRequest::FileList { path: request.path },
            }),
            request_response::Event::Message {
                peer,
                message: Message::Response { response, .. },
            } => Some(NetEvent::FileListReplied {
                peer: domain_peer(&peer),
                reply: response.into(),
            }),
            request_response::Event::OutboundFailure { peer, error, .. } => {
                Some(failure(peer, RequestKind::FileList, None, &error))
            }
            _ => None,
        }
    }

    fn on_stream(
        &mut self,
        event: request_response::Event<protocol::StreamRequestDto, StreamResponseDto>,
    ) -> Option<NetEvent> {
        match event {
            request_response::Event::Message {
                peer,
                message:
                    Message::Request {
                        request, channel, ..
                    },
            } => Some(NetEvent::Inbound {
                peer: domain_peer(&peer),
                id: self.store_inbound(peer, PendingResponse::Stream(channel)),
                request: InboundRequest::Stream(request.into()),
            }),
            request_response::Event::Message {
                peer,
                message:
                    Message::Response {
                        request_id,
                        response,
                    },
            } => {
                self.stream_requests.remove(&request_id);
                Some(NetEvent::StreamAcked {
                    peer: domain_peer(&peer),
                    ack: response.into(),
                })
            }
            request_response::Event::OutboundFailure {
                peer,
                request_id,
                error,
            } => {
                let transfer_id = self.stream_requests.remove(&request_id);
                Some(failure(peer, RequestKind::Stream, transfer_id, &error))
            }
            _ => None,
        }
    }

    fn on_device_info(
        &mut self,
        event: request_response::Event<DeviceInfoRequestDto, DeviceInfoResponseDto>,
    ) -> Option<NetEvent> {
        match event {
            request_response::Event::Message {
                peer,
                message: Message::Request { channel, .. },
            } => Some(NetEvent::Inbound {
                peer: domain_peer(&peer),
                id: self.store_inbound(peer, PendingResponse::DeviceInfo(channel)),
                request: InboundRequest::DeviceInfo,
            }),
            request_response::Event::Message {
                peer,
                message: Message::Response { response, .. },
            } => Some(NetEvent::DeviceInfoReplied {
                peer: domain_peer(&peer),
                device_name: response.device_name,
            }),
            request_response::Event::OutboundFailure { peer, error, .. } => {
                Some(failure(peer, RequestKind::DeviceInfo, None, &error))
            }
            _ => None,
        }
    }

    fn on_pairing(
        &mut self,
        event: request_response::Event<protocol::PairingRequestDto, PairingResponseDto>,
    ) -> Option<NetEvent> {
        match event {
            request_response::Event::Message {
                peer,
                message:
                    Message::Request {
                        request, channel, ..
                    },
            } => Some(NetEvent::Inbound {
                peer: domain_peer(&peer),
                id: self.store_inbound(peer, PendingResponse::Pairing(channel)),
                request: InboundRequest::Pairing(request.into()),
            }),
            request_response::Event::Message {
                peer,
                message: Message::Response { response, .. },
            } => Some(NetEvent::PairingReplied {
                peer: domain_peer(&peer),
                reply: response.into(),
            }),
            request_response::Event::OutboundFailure { peer, error, .. } => {
                Some(failure(peer, RequestKind::Pairing, None, &error))
            }
            _ => None,
        }
    }

    fn on_directory_changed(
        &mut self,
        event: request_response::Event<DirectoryChangedDto, DirectoryChangedDto>,
    ) -> Option<NetEvent> {
        match event {
            request_response::Event::Message {
                peer,
                message: Message::Request { channel, .. },
            } => Some(NetEvent::Inbound {
                peer: domain_peer(&peer),
                id: self.store_inbound(peer, PendingResponse::DirectoryChanged(channel)),
                request: InboundRequest::DirectoryChanged,
            }),
            request_response::Event::OutboundFailure { peer, error, .. } => {
                Some(failure(peer, RequestKind::DirectoryChanged, None, &error))
            }
            _ => None,
        }
    }
}

fn failure(
    peer: libp2p::PeerId,
    kind: RequestKind,
    transfer_id: Option<TransferId>,
    error: &request_response::OutboundFailure,
) -> NetEvent {
    NetEvent::RequestFailed {
        peer: domain_peer(&peer),
        kind,
        transfer_id,
        error: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::net::{PairingAsk, StreamMsg};
    use libp2p::core::{ConnectedPoint, Endpoint};
    use libp2p::swarm::DialError;
    use std::num::NonZeroU32;
    use tokio::sync::mpsc::error::TryRecvError;

    fn actor() -> (NetworkActor, mpsc::Receiver<NetEvent>) {
        let swarm = build_swarm(Keypair::generate_ed25519(), 0).unwrap();
        let (events, rx) = mpsc::channel(EVENT_BUFFER);
        let actor = NetworkActor {
            swarm,
            events,
            inbound: HashMap::new(),
            next_inbound: 0,
            stream_requests: HashMap::new(),
            quic_addresses: HashMap::new(),
            quic_dialed: HashSet::new(),
            connections: HashMap::new(),
            probes: HashMap::new(),
        };
        (actor, rx)
    }

    fn temp_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("pebble-p2p-{label}-{}", uuid::Uuid::new_v4()))
    }

    fn dialer(addr: &str) -> ConnectedPoint {
        ConnectedPoint::Dialer {
            address: addr.parse().unwrap(),
            role_override: Endpoint::Dialer,
        }
    }

    fn established(
        peer: libp2p::PeerId,
        id: usize,
        count: u32,
    ) -> SwarmEvent<PebbleBehaviourEvent, ()> {
        SwarmEvent::ConnectionEstablished {
            peer_id: peer,
            connection_id: ConnectionId::new_unchecked(id),
            endpoint: dialer("/ip4/10.1.2.3/tcp/4001"),
            num_established: NonZeroU32::new(count).unwrap(),
            concurrent_dial_errors: None,
            established_in: Duration::from_millis(1),
        }
    }

    fn closed(
        peer: libp2p::PeerId,
        id: usize,
        remaining: u32,
    ) -> SwarmEvent<PebbleBehaviourEvent, ()> {
        SwarmEvent::ConnectionClosed {
            peer_id: peer,
            connection_id: ConnectionId::new_unchecked(id),
            endpoint: dialer("/ip4/10.1.2.3/tcp/4001"),
            num_established: remaining,
            cause: None,
        }
    }

    #[test]
    fn idle_connection_timeout_is_one_hour() {
        assert_eq!(IDLE_CONNECTION_TIMEOUT, Duration::from_secs(3600));
    }

    #[cfg(unix)]
    #[test]
    fn new_identity_file_is_private_and_reloads_the_same_key() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_path("identity");
        let file = dir.join("nested").join("identity.key");
        let first = load_or_create_identity(&file).unwrap();
        let on_disk = std::fs::read(&file).unwrap();
        assert!(!on_disk.is_empty());
        assert_eq!(
            Keypair::from_protobuf_encoding(&on_disk)
                .unwrap()
                .public()
                .to_peer_id(),
            first.public().to_peer_id()
        );
        let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        let dir_mode = std::fs::metadata(file.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(dir_mode, 0o700);
        let second = load_or_create_identity(&file).unwrap();
        assert_eq!(second.public().to_peer_id(), first.public().to_peer_id());
    }

    #[cfg(unix)]
    #[test]
    fn write_private_refuses_to_overwrite_and_stores_bytes() {
        let dir = temp_path("write-private");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("secret");
        write_private(&file, b"abc").unwrap();
        assert_eq!(std::fs::read(&file).unwrap(), b"abc");
        assert!(write_private(&file, b"xyz").is_err());
        assert_eq!(std::fs::read(&file).unwrap(), b"abc");
    }

    #[test]
    fn ip_of_reads_v4_and_v6_and_ignores_others() {
        let v4 = ip_of(&"/ip4/10.0.0.7/tcp/1".parse().unwrap());
        assert_eq!(v4, Some("10.0.0.7".parse().unwrap()));
        let v6 = ip_of(&"/ip6/::1/tcp/1".parse().unwrap());
        assert_eq!(v6, Some("::1".parse().unwrap()));
        assert_eq!(ip_of(&"/dns4/example.com/tcp/1".parse().unwrap()), None);
    }

    #[tokio::test]
    async fn probe_skips_known_ips_and_dials_new_ones() {
        let (mut actor, _rx) = actor();
        let known: IpAddr = "10.9.8.7".parse().unwrap();
        actor
            .connections
            .insert(ConnectionId::new_unchecked(1), known);
        actor.probe("/ip4/10.9.8.7/tcp/4001");
        assert!(actor.probes.is_empty());

        actor.probe("/ip4/10.9.8.8/tcp/4001");
        assert_eq!(actor.probes.len(), 1);
        assert_eq!(
            actor.probes.values().next().copied(),
            Some("10.9.8.8".parse().unwrap())
        );

        actor.probe("/ip4/10.9.8.8/tcp/4002");
        assert_eq!(actor.probes.len(), 1);
    }

    #[tokio::test]
    async fn prefer_quic_dials_the_known_quic_address_once() {
        let (mut actor, _rx) = actor();
        let peer = libp2p::PeerId::random();
        actor.prefer_quic(peer);
        assert!(actor.quic_dialed.is_empty());

        actor
            .quic_addresses
            .insert(peer, "/ip4/10.9.8.7/udp/4001/quic-v1".parse().unwrap());
        actor.prefer_quic(peer);
        assert!(actor.quic_dialed.contains(&peer));
    }

    #[tokio::test]
    async fn outgoing_connection_error_forgets_the_probe() {
        let (mut actor, _rx) = actor();
        let id = ConnectionId::new_unchecked(7);
        let other = ConnectionId::new_unchecked(8);
        actor.probes.insert(id, "10.0.0.1".parse().unwrap());
        actor.probes.insert(other, "10.0.0.2".parse().unwrap());
        let keep = actor
            .on_swarm_event(
                SwarmEvent::<PebbleBehaviourEvent, ()>::OutgoingConnectionError {
                    connection_id: id,
                    peer_id: None,
                    error: DialError::Aborted,
                },
            )
            .await;
        assert!(keep);
        assert!(!actor.probes.contains_key(&id));
        assert!(actor.probes.contains_key(&other));
    }

    #[tokio::test]
    async fn connected_is_announced_only_for_the_first_connection() {
        let (mut actor, mut rx) = actor();
        let peer = libp2p::PeerId::random();
        assert!(actor.on_swarm_event(established(peer, 1, 1)).await);
        assert!(matches!(rx.try_recv(), Ok(NetEvent::Connected { .. })));
        assert_eq!(
            actor.connections.get(&ConnectionId::new_unchecked(1)),
            Some(&"10.1.2.3".parse().unwrap())
        );

        assert!(actor.on_swarm_event(established(peer, 2, 2)).await);
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
    }

    #[tokio::test]
    async fn probed_connection_reports_the_discovered_peer() {
        let (mut actor, mut rx) = actor();
        let peer = libp2p::PeerId::random();
        actor
            .probes
            .insert(ConnectionId::new_unchecked(5), "10.1.2.3".parse().unwrap());
        assert!(actor.on_swarm_event(established(peer, 5, 1)).await);
        assert!(actor.probes.is_empty());
        assert!(matches!(
            rx.try_recv(),
            Ok(NetEvent::PeerDiscovered { address, .. }) if address.contains("10.1.2.3")
        ));
        assert!(matches!(rx.try_recv(), Ok(NetEvent::Connected { .. })));
    }

    #[tokio::test]
    async fn disconnected_is_announced_only_after_the_last_connection_closes() {
        let (mut actor, mut rx) = actor();
        let peer = libp2p::PeerId::random();
        actor
            .connections
            .insert(ConnectionId::new_unchecked(1), "10.1.2.3".parse().unwrap());
        actor
            .connections
            .insert(ConnectionId::new_unchecked(2), "10.1.2.3".parse().unwrap());
        actor.quic_dialed.insert(peer);

        assert!(actor.on_swarm_event(closed(peer, 1, 1)).await);
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
        assert!(!actor
            .connections
            .contains_key(&ConnectionId::new_unchecked(1)));
        assert!(actor.quic_dialed.contains(&peer));

        assert!(actor.on_swarm_event(closed(peer, 2, 0)).await);
        assert!(matches!(rx.try_recv(), Ok(NetEvent::Disconnected { .. })));
        assert!(actor.connections.is_empty());
        assert!(!actor.quic_dialed.contains(&peer));
    }

    #[tokio::test]
    async fn mdns_discovery_emits_tcp_peers_and_remembers_quic_addresses() {
        let (mut actor, mut rx) = actor();
        let tcp_peer = libp2p::PeerId::random();
        let quic_peer = libp2p::PeerId::random();
        let tcp_addr: Multiaddr = "/ip4/10.0.0.5/tcp/4001".parse().unwrap();
        let quic_addr: Multiaddr = "/ip4/10.0.0.6/udp/4001/quic-v1".parse().unwrap();
        let keep = actor
            .on_mdns(mdns::Event::Discovered(vec![
                (quic_peer, quic_addr.clone()),
                (tcp_peer, tcp_addr.clone()),
            ]))
            .await;
        assert!(keep);
        assert!(matches!(
            rx.try_recv(),
            Ok(NetEvent::PeerDiscovered { address, .. }) if address == tcp_addr.to_string()
        ));
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
        assert_eq!(actor.quic_addresses.get(&quic_peer), Some(&quic_addr));
        assert!(!actor.quic_addresses.contains_key(&tcp_peer));
    }

    #[tokio::test]
    async fn mdns_discovery_stops_when_the_event_receiver_is_gone() {
        let (mut actor, rx) = actor();
        drop(rx);
        let addr: Multiaddr = "/ip4/10.0.0.5/tcp/4001".parse().unwrap();
        let keep = actor
            .on_mdns(mdns::Event::Discovered(vec![(
                libp2p::PeerId::random(),
                addr,
            )]))
            .await;
        assert!(!keep);
    }

    #[tokio::test]
    async fn mdns_expiry_emits_once_per_peer_and_forgets_quic_address() {
        let (mut actor, mut rx) = actor();
        let peer = libp2p::PeerId::random();
        let addr: Multiaddr = "/ip4/10.0.0.6/udp/4001/quic-v1".parse().unwrap();
        actor.quic_addresses.insert(peer, addr.clone());
        let keep = actor
            .on_mdns(mdns::Event::Expired(vec![
                (peer, addr.clone()),
                (peer, "/ip4/10.0.0.6/tcp/4001".parse().unwrap()),
            ]))
            .await;
        assert!(keep);
        assert!(matches!(rx.try_recv(), Ok(NetEvent::PeerExpired { .. })));
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
        assert!(actor.quic_addresses.is_empty());
    }

    #[tokio::test]
    async fn mdns_expiry_stops_when_the_event_receiver_is_gone() {
        let (mut actor, rx) = actor();
        drop(rx);
        let addr: Multiaddr = "/ip4/10.0.0.6/tcp/4001".parse().unwrap();
        let keep = actor
            .on_mdns(mdns::Event::Expired(vec![(libp2p::PeerId::random(), addr)]))
            .await;
        assert!(!keep);
    }

    fn failure_error() -> request_response::OutboundFailure {
        request_response::OutboundFailure::ConnectionClosed
    }

    #[tokio::test]
    async fn outbound_failures_are_reported_per_protocol() {
        let (mut actor, _rx) = actor();
        let peer = libp2p::PeerId::random();
        let transfer = TransferId::from_wire("t-fail".into());

        let id = actor
            .swarm
            .behaviour_mut()
            .file_list
            .send_request(&peer, FileListRequestDto { path: "/".into() });
        let event = actor.on_file_list(request_response::Event::OutboundFailure {
            peer,
            request_id: id,
            error: failure_error(),
        });
        assert!(matches!(
            event,
            Some(NetEvent::RequestFailed {
                kind: RequestKind::FileList,
                transfer_id: None,
                ..
            })
        ));

        let id = actor.swarm.behaviour_mut().stream.send_request(
            &peer,
            StreamMsg::Cancel {
                transfer_id: transfer.clone(),
            }
            .into(),
        );
        actor.stream_requests.insert(id, transfer.clone());
        let event = actor.on_stream(request_response::Event::OutboundFailure {
            peer,
            request_id: id,
            error: failure_error(),
        });
        assert!(matches!(
            event,
            Some(NetEvent::RequestFailed {
                kind: RequestKind::Stream,
                transfer_id: Some(t),
                ..
            }) if t == transfer
        ));
        assert!(actor.stream_requests.is_empty());

        let id = actor
            .swarm
            .behaviour_mut()
            .device_info
            .send_request(&peer, DeviceInfoRequestDto {});
        let event = actor.on_device_info(request_response::Event::OutboundFailure {
            peer,
            request_id: id,
            error: failure_error(),
        });
        assert!(matches!(
            event,
            Some(NetEvent::RequestFailed {
                kind: RequestKind::DeviceInfo,
                ..
            })
        ));

        let id = actor.swarm.behaviour_mut().pairing.send_request(
            &peer,
            PairingAsk {
                device_name: "x".into(),
                pin: "1234".into(),
            }
            .into(),
        );
        let event = actor.on_pairing(request_response::Event::OutboundFailure {
            peer,
            request_id: id,
            error: failure_error(),
        });
        assert!(matches!(
            event,
            Some(NetEvent::RequestFailed {
                kind: RequestKind::Pairing,
                ..
            })
        ));

        let id = actor
            .swarm
            .behaviour_mut()
            .directory_changed
            .send_request(&peer, DirectoryChangedDto {});
        let event = actor.on_directory_changed(request_response::Event::OutboundFailure {
            peer,
            request_id: id,
            error: failure_error(),
        });
        assert!(matches!(
            event,
            Some(NetEvent::RequestFailed {
                kind: RequestKind::DirectoryChanged,
                ..
            })
        ));
    }
}
