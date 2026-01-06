/// Phase 2 테스트: 기기 탐색 (Discovery)
///
/// # 사용법
/// ```bash
/// # 터미널 1 (Device A)
/// cargo run --bin test_discovery device-a
///
/// # 터미널 2 (Device B)
/// cargo run --bin test_discovery device-b
/// ```

use native::api::discovery;
use std::env;
use tokio::time::{sleep, Duration};

const SECRET_KEY: &str = "pebble-test-key-2024";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = env::args().collect();
    let device_name = if args.len() > 1 {
        args[1].clone()
    } else {
        "Test Device".to_string()
    };

    println!("\n{}", "=".repeat(60));
    println!("  Pebble Discovery Test");
    println!("{}", "=".repeat(60));
    println!("Device Name: {}", device_name);
    println!("{}\n", "=".repeat(60));

    println!("🔍 Starting discovery...");
    let device_id = discovery::start_discovery(device_name, SECRET_KEY.to_string()).await?;
    println!("✅ Device ID: {}\n", device_id);

    println!("🔎 Scanning for 30 seconds...\n");

    for i in 1..=30 {
        sleep(Duration::from_secs(1)).await;

        let devices = discovery::get_discovered_devices()?;

        if !devices.is_empty() && i % 2 == 0 {
            println!("\n[{}s] Found {} device(s):", i, devices.len());
            for d in &devices {
                println!("  📱 {} - {} ({})", d.device_name, d.ip_address, if d.is_online { "Online" } else { "Offline" });
            }
        }
    }

    discovery::stop_discovery()?;
    println!("\n✅ Done");

    Ok(())
}
