use crate::network::behavior::{DeviceInfoRequest, MyBehaviour};
use libp2p::{Multiaddr, PeerId, Swarm};
use tauri::Emitter;

use super::EngineContext;

impl EngineContext {
    // ─── Dial ───

    pub(crate) fn handle_dial(&self, swarm: &mut Swarm<MyBehaviour>, addr_str: String) {
        println!("📞 Dial 요청 수신: {}", addr_str);
        match addr_str.parse::<Multiaddr>() {
            Ok(addr) => match swarm.dial(addr.clone()) {
                Ok(_) => {
                    println!("Dial connecting~: {}", addr);
                    self.app.emit("dial-started", addr.to_string()).ok();
                }
                Err(e) => {
                    println!("Dial failed: {}", e);
                    self.app.emit("dial-failed", format!("{}", e)).ok();
                }
            },
            Err(e) => {
                println!("dial failed: {}", e);
                self.app
                    .emit("dial-failed", format!("주소 파싱 실패: {}", e))
                    .ok();
            }
        }
    }

    // ─── mDNS ───

    pub fn handle_mdns_discovered(&mut self, list: Vec<(PeerId, Multiaddr)>) {
        for (peer_id, addr) in list {
            let addr_str = addr.to_string();
            self.peer_addresses
                .entry(peer_id)
                .or_insert_with(Vec::new)
                .push(addr.clone());

            if !addr_str.contains("quic") {
                println!("peer-found ID: {}, 주소: {}", peer_id, addr);
                let payload = format!("{}:{}", peer_id, addr);
                self.app.emit("peer-found", payload).ok();
            } else {
                println!("📡 QUIC 주소 저장 (파일 전송용): {} -> {}", peer_id, addr);
            }
        }
    }

    pub fn handle_mdns_expired(&self, list: Vec<(PeerId, Multiaddr)>) {
        for (peer_id, _addr) in list {
            self.app.emit("peer-expired", peer_id.to_string()).ok();
        }
    }

    // ─── Identify ───

    pub fn handle_identify(&self, info: libp2p::identify::Info) {
        self.app.emit("peer-info", info.agent_version).ok();
    }

    // ─── Ping ───

    pub fn handle_ping(
        &self,
        peer: PeerId,
        result: Result<std::time::Duration, libp2p::ping::Failure>,
    ) {
        if let Ok(rtt) = result {
            let payload = format!("{}:{}", peer, rtt.as_millis());
            self.app.emit("ping-success", payload).ok();
        }
    }

    // ─── 연결 이벤트 ───

    pub fn handle_connection_established(&self, swarm: &mut Swarm<MyBehaviour>, peer_id: PeerId) {
        println!("connection-success PeerId: {}", peer_id);
        let request = DeviceInfoRequest {};
        swarm
            .behaviour_mut()
            .device_info
            .send_request(&peer_id, request);
        self.app
            .emit("connection-success", peer_id.to_string())
            .ok();
    }

    pub fn handle_connection_closed(&self, peer_id: PeerId) {
        self.app.emit("connection-closed", peer_id.to_string()).ok();
    }

    pub fn handle_new_listen_addr(&self, address: &Multiaddr) {
        if !address.to_string().contains("127.0.0.1") {
            println!("listening address: {}", address);
            self.app.emit("listening-on", address.to_string()).ok();
        }
    }
}
