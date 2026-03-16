pub mod behavior;
pub mod handlers;
pub mod helpers;
pub mod types;

pub use types::{
    CancelTransferMsg, DeviceInfoMsg, FileListRequestMsg, FileTransferRequestMsg,
    FileTransferStreamRequestMsg, IncomingTransferState, OutgoingTransferState, P2pCommand,
    PairingApprovalMsg, PairingRequestMsg, CHUNK_SIZE,
};

use behavior::MyBehaviour;
use futures::StreamExt;
use handlers::EngineContext;
use libp2p::{request_response, swarm::SwarmEvent, StreamProtocol, SwarmBuilder};
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;
use tokio::sync::mpsc;

/// P2P 엔진 메인 루프
///
/// 리팩토링 전: 820줄의 단일 함수, 9개 채널 수신, 7단계 중첩 match
/// 리팩토링 후: ~120줄, 단일 커맨드 채널, EngineContext로 핸들러 위임
pub async fn run_p2p_engine(
    app: AppHandle,
    mut cmd_rx: mpsc::UnboundedReceiver<P2pCommand>,
    initial_shared_folder: PathBuf,
    device_name: String,
    state: Arc<crate::state::P2pState>,
) -> Result<(), Box<dyn Error>> {
    // ─── Swarm 초기화 ───
    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_quic()
        .with_behaviour(|key| {
            let mdns =
                libp2p::mdns::tokio::Behaviour::new(Default::default(), key.public().to_peer_id())?;
            let identify = libp2p::identify::Behaviour::new(libp2p::identify::Config::new(
                "/pebble/1.0.0".into(),
                key.public(),
            ));
            let ping = libp2p::ping::Behaviour::new(libp2p::ping::Config::new());

            let file_list = request_response::Behaviour::new(
                [(
                    StreamProtocol::new("/pebble/file-list/1.0.0"),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            );

            let file_transfer = request_response::Behaviour::new(
                [(
                    StreamProtocol::new("/pebble/file-transfer/1.0.0"),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            );

            let file_stream = request_response::Behaviour::new(
                [(
                    StreamProtocol::new("/pebble/file-upload/1.0.0"),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            );

            let device_info = request_response::Behaviour::new(
                [(
                    StreamProtocol::new("/pebble/device-info/1.0.0"),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            );

            let pairing = request_response::Behaviour::new(
                [(
                    StreamProtocol::new("/pebble/pairing/1.0.0"),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            );

            Ok(MyBehaviour {
                mdns,
                identify,
                ping,
                file_list,
                file_transfer,
                file_stream,
                device_info,
                pairing,
            })
        })?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(60 * 60)))
        .build();

    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;

    let local_peer_id = swarm.local_peer_id().to_string();
    println!("내 기기 PeerId: {}", local_peer_id);
    println!("기기 이름: {}", device_name);
    println!("공유 폴더: {:?}", initial_shared_folder);

    // ─── EngineContext 생성 ───
    let mut ctx = EngineContext {
        app,
        shared_folder: initial_shared_folder,
        device_name,
        state,
        outgoing_transfers: HashMap::new(),
        incoming_transfers: HashMap::new(),
        req_id_to_transfer_id: HashMap::new(),
        peer_addresses: HashMap::new(),
        trusted_peers: std::collections::HashSet::new(),
        pending_pairings: HashMap::new(),
    };

    // ─── 이벤트 루프 ───
    loop {
        tokio::select! {
            // 커맨드 수신 (통합 채널)
            Some(cmd) = cmd_rx.recv() => {
                // 페어링 승인은 비동기 처리 필요 (shared_folder_display_name 접근)
                if matches!(&cmd, P2pCommand::ApprovePairing(_)) {
                    if let P2pCommand::ApprovePairing(approval) = cmd {
                        ctx.handle_pairing_approval_async(&mut swarm, approval).await;
                    }
                } else {
                    let should_stop = ctx.handle_command(cmd, &mut swarm);
                    if should_stop {
                        break;
                    }
                }
            }

            // Swarm 이벤트 수신
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::Mdns(
                        libp2p::mdns::Event::Discovered(list),
                    )) => ctx.handle_mdns_discovered(list),

                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::Mdns(
                        libp2p::mdns::Event::Expired(list),
                    )) => ctx.handle_mdns_expired(list),

                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::Identify(
                        libp2p::identify::Event::Received { info, .. },
                    )) => ctx.handle_identify(info),

                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::Ping(
                        libp2p::ping::Event { peer, result, .. },
                    )) => ctx.handle_ping(peer, result),

                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::FileList(event)) => {
                        ctx.handle_file_list_event(&mut swarm, event);
                    }

                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::FileTransfer(event)) => {
                        ctx.handle_file_transfer_event(&mut swarm, event);
                    }

                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::FileStream(event)) => {
                        match event {
                            request_response::Event::Message { peer, message } => {
                                match message {
                                    request_response::Message::Request { request, channel, .. } => {
                                        ctx.handle_file_stream_request(&mut swarm, peer, request, channel).await;
                                    }
                                    request_response::Message::Response { response, request_id } => {
                                        ctx.handle_file_stream_response(&mut swarm, response, request_id).await;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::Pairing(event)) => {
                        ctx.handle_pairing_event(&mut swarm, event);
                    }

                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        ctx.handle_connection_established(&mut swarm, peer_id);
                    }
                    SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        ctx.handle_connection_closed(peer_id);
                    }
                    SwarmEvent::NewListenAddr { address, .. } => {
                        ctx.handle_new_listen_addr(&address);
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
