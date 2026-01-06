/// Phase 3 테스트: 암호화된 파일 전송 (Secure File Transfer)
///
/// # 사용법
/// ```bash
/// # 터미널 1 - 수신자
/// cargo run --release --bin test_transfer -- receiver
///
/// # 터미널 2 - 송신자
/// cargo run --release --bin test_transfer -- sender 127.0.0.1 /tmp/test_file.bin
///
/// # 테스트 파일 생성
/// dd if=/dev/urandom of=/tmp/test_file.bin bs=1048576 count=10  # 10MB
/// ```

use native::api::certificate::CertificateManager;
use native::api::transfer::{TransferClient, TransferServer, TRANSFER_PORT};
use std::env;
use std::fs;
use std::net::SocketAddr;

const CERT_DIR: &str = "/tmp/pebble_certs";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let mode = &args[1];

    match mode.as_str() {
        "receiver" => run_receiver().await?,
        "sender" => {
            if args.len() < 4 {
                println!("❌ Error: Missing arguments");
                println!("Usage: cargo run --bin test_transfer -- sender <ip> <file_path>");
                return Ok(());
            }
            let server_ip = &args[2];
            let file_path = &args[3];
            run_sender(server_ip, file_path).await?;
        }
        _ => {
            println!("❌ Unknown mode: {}", mode);
            print_usage();
        }
    }

    Ok(())
}

fn print_usage() {
    println!("\n{}", "=".repeat(70));
    println!("  Pebble Secure File Transfer Test");
    println!("{}", "=".repeat(70));
    println!("\nUsage:");
    println!("  Receiver: cargo run --bin test_transfer -- receiver");
    println!("  Sender:   cargo run --bin test_transfer -- sender <ip> <file>");
    println!("\nExample:");
    println!("  # Terminal 1");
    println!("  cargo run --release --bin test_transfer -- receiver");
    println!("\n  # Terminal 2");
    println!("  cargo run --release --bin test_transfer -- sender 127.0.0.1 /tmp/test_file.bin");
    println!("\nCreate test file:");
    println!("  dd if=/dev/urandom of=/tmp/test_file.bin bs=1048576 count=10");
    println!("{}\n", "=".repeat(70));
}

async fn run_receiver() -> anyhow::Result<()> {
    println!("\n{}", "=".repeat(70));
    println!("  📥 RECEIVER MODE");
    println!("{}\n", "=".repeat(70));

    fs::create_dir_all(CERT_DIR)?;

    println!("🔐 Loading TLS certificate...");
    let manager = CertificateManager::new(CERT_DIR.to_string());
    let cert = manager.get_or_create_certificate("receiver-id", "Test Receiver")?;

    println!("✅ Certificate loaded");
    println!("📋 Fingerprint: {}", cert.fingerprint);
    println!("   (Copy this for Certificate Pinning)\n");

    let bind_addr: SocketAddr = format!("0.0.0.0:{}", TRANSFER_PORT).parse()?;
    let server = TransferServer::new(cert);

    println!("📡 Transfer server listening on {}", bind_addr);
    println!("🔄 Waiting for files...");
    println!("   Press Ctrl+C to stop\n");

    server.start(bind_addr).await?;

    Ok(())
}

async fn run_sender(server_ip: &str, file_path: &str) -> anyhow::Result<()> {
    println!("\n{}", "=".repeat(70));
    println!("  📤 SENDER MODE");
    println!("{}\n", "=".repeat(70));

    if !std::path::Path::new(file_path).exists() {
        println!("❌ Error: File not found: {}", file_path);
        println!("\n💡 Create test file:");
        println!("   dd if=/dev/urandom of={} bs=1048576 count=10", file_path);
        return Ok(());
    }

    let file_size = fs::metadata(file_path)?.len();
    println!("📁 File: {}", file_path);
    println!("📊 Size: {:.2} MB", file_size as f64 / 1_048_576.0);
    println!();

    let server_addr: SocketAddr = format!("{}:{}", server_ip, TRANSFER_PORT).parse()?;
    println!("🎯 Target: {}", server_addr);

    println!("\n🔐 Certificate Pinning (optional):");
    println!("   Enter fingerprint or press Enter to skip:");
    print!("   > ");

    use std::io::{self, Write};
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let fingerprint = input.trim();

    let server_fingerprint = if fingerprint.is_empty() {
        println!("   ⚠️  Skipping certificate verification");
        None
    } else {
        println!("   ✅ Using Certificate Pinning");
        Some(fingerprint.to_string())
    };

    println!("\n🚀 Starting transfer...\n");

    let client = TransferClient::new(server_fingerprint);

    match client.send_file(server_addr, file_path).await {
        Ok(_) => {
            println!("\n{}", "=".repeat(70));
            println!("  ✅ FILE TRANSFER COMPLETED");
            println!("{}\n", "=".repeat(70));
        }
        Err(e) => {
            println!("\n{}", "=".repeat(70));
            println!("  ❌ FILE TRANSFER FAILED");
            println!("{}", "=".repeat(70));
            println!("Error: {}\n", e);
        }
    }

    Ok(())
}
