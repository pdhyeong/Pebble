use serde_json::Value;
use std::io::Write;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use pebble_core::adapters::control::{ControlReply, ControlRequest};

use crate::log::printable;

pub async fn run(socket: &Path, request: ControlRequest, json: bool) -> Result<(), String> {
    let stream = UnixStream::connect(socket).await.map_err(|_| {
        format!(
            "Pebble 이 켜져 있지 않습니다 ({}). `pebble-cli daemon` 을 먼저 실행하거나 Pebble 앱을 켜세요.",
            socket.display()
        )
    })?;
    let (read, mut write) = stream.into_split();
    let mut line = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
    line.push(b'\n');
    write.write_all(&line).await.map_err(|e| e.to_string())?;

    let mut lines = BufReader::new(read).lines();
    while let Some(raw) = lines.next_line().await.map_err(|e| e.to_string())? {
        if json {
            println!("{raw}");
        }
        let reply: ControlReply =
            serde_json::from_str(&raw).map_err(|e| format!("응답을 읽지 못했습니다: {e}"))?;
        match reply {
            ControlReply::Event { event } => {
                if !json {
                    show_event(&event);
                }
            }
            ControlReply::Ok { data } => {
                if !json {
                    show_result(&request, &data);
                }
                return Ok(());
            }
            ControlReply::Error { code, message } => {
                return Err(if json {
                    String::new()
                } else {
                    format!("{message} ({})", text(&code))
                });
            }
        }
    }
    Err("Pebble 이 응답 없이 연결을 끊었습니다.".to_string())
}

fn text(value: &Value) -> String {
    printable(value.as_str().unwrap_or(""))
}

fn show_event(event: &Value) {
    let payload = &event["payload"];
    match text(&event["event"]).as_str() {
        "pairing-sent" => println!(
            "PIN {} — 상대 기기에 같은 PIN 이 보이면 상대에서 승인하세요. 기다리는 중…",
            text(&payload["pin"])
        ),
        "transfer-started" if payload["total_size"].as_u64().unwrap_or(0) > 0 => println!(
            "전송 시작 [{}] {} ({} B)",
            text(&payload["transfer_id"]),
            text(&payload["file_name"]),
            payload["total_size"]
        ),
        "transfer-progress" => {
            eprint!(
                "\r진행 {:>5.1}%",
                payload["progress"].as_f64().unwrap_or(0.0)
            );
            let _ = std::io::stderr().flush();
        }
        _ => {}
    }
}

fn show_result(request: &ControlRequest, data: &Value) {
    match request {
        ControlRequest::Status => {
            println!("실행 중   {}", data["running"]);
            println!("이름      {}", text(&data["device_name"]));
            println!("PeerId    {}", text(&data["peer_id"]));
            println!("공유 폴더 {}", text(&data["shared_folder"]));
        }
        ControlRequest::Peers => show_peers(data),
        ControlRequest::Ls { .. } => {
            for file in data.as_array().into_iter().flatten() {
                let kind = if file["is_dir"].as_bool().unwrap_or(false) {
                    "폴더"
                } else {
                    "파일"
                };
                println!("{kind}  {:>12}  {}", file["size"], text(&file["path"]));
            }
        }
        ControlRequest::Pair { .. } => println!("페어링됨: {}", text(&data["device_name"])),
        ControlRequest::Send { .. } | ControlRequest::Get { .. } => {
            let payload = &data["payload"];
            eprintln!();
            match payload["saved_path"].as_str() {
                Some(path) => println!("완료: {}", printable(path)),
                None => println!("완료: {}", text(&payload["file_name"])),
            }
        }
        ControlRequest::Connect { .. }
        | ControlRequest::Respond { .. }
        | ControlRequest::Cancel { .. }
        | ControlRequest::Unpair { .. } => {
            println!("완료")
        }
    }
}

fn show_peers(data: &Value) {
    let peers = data.as_array().cloned().unwrap_or_default();
    if peers.is_empty() {
        println!("보이는 기기가 없습니다.");
        return;
    }
    println!(
        "{:<20} {:<6} {:<6} {:<8} PeerId",
        "이름", "연결", "페어링", "PIN"
    );
    for peer in peers {
        let flag = |key: &str| {
            if peer[key].as_bool().unwrap_or(false) {
                "예"
            } else {
                "-"
            }
        };
        println!(
            "{:<20} {:<6} {:<6} {:<8} {}",
            text(&peer["device_name"]),
            flag("connected"),
            flag("trusted"),
            peer["pending_pin"].as_str().unwrap_or("-"),
            text(&peer["peer_id"]),
        );
    }
}
