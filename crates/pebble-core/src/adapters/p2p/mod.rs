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
        std::fs::create_dir_all(dir)
            .map_err(|e| AppError::io("설정 폴더를 만들지 못했습니다", &e))?;
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
