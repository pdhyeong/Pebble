use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use super::engine::{EngineCommand, EngineHandle};
use super::ports::PeerDirectory;

pub const DISCOVERY_INTERVAL: Duration = Duration::from_secs(30);

pub fn spawn_discovery(
    directory: Arc<dyn PeerDirectory>,
    engine: EngineHandle,
    mut stop: watch::Receiver<bool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(DISCOVERY_INTERVAL);
        let mut reported = false;
        loop {
            tokio::select! {
                _ = stop.changed() => break,
                _ = engine.closed() => break,
                _ = tick.tick() => match directory.addresses().await {
                    Ok(addresses) => {
                        reported = false;
                        for address in addresses {
                            if engine.send(EngineCommand::Probe { address }).await.is_err() {
                                return;
                            }
                        }
                    }
                    Err(e) if !reported => {
                        eprintln!("[discovery] 기기 목록을 읽지 못했습니다: {e}");
                        reported = true;
                    }
                    Err(_) => {}
                },
            }
        }
    })
}
