use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_pebble-cli");

fn scratch() -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = PathBuf::from(format!(
        "/tmp/pbB-{}/{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_cli(dir: &Path, args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .arg("--config-dir")
        .arg(dir)
        .output()
        .unwrap()
}

fn plain(args: &[&str]) -> Output {
    Command::new(BIN).args(args).output().unwrap()
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).to_string()
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

fn fake_server(dir: &Path, replies: Vec<String>) -> JoinHandle<String> {
    let listener = UnixListener::bind(dir.join("control.sock")).unwrap();
    std::thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + Duration::from_secs(20);
        let stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        Instant::now() < deadline,
                        "클라이언트가 연결하지 않았습니다"
                    );
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(e) => panic!("{e}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        let mut line = String::new();
        BufReader::new(stream.try_clone().unwrap())
            .read_line(&mut line)
            .unwrap();
        let mut stream = stream;
        for reply in replies {
            writeln!(stream, "{reply}").unwrap();
        }
        line
    })
}

fn ok(data: &str) -> String {
    format!(r#"{{"type":"ok","data":{data}}}"#)
}

fn event(name: &str, payload: &str) -> String {
    format!(r#"{{"type":"event","event":{{"event":"{name}","payload":{payload}}}}}"#)
}

#[test]
fn help_forms_print_usage_and_succeed() {
    for args in [
        &[][..],
        &["-h"],
        &["--help"],
        &["help"],
        &["status", "-h"],
        &["status", "--help"],
        &["status", "help"],
    ] {
        let o = plain(args);
        assert!(o.status.success(), "{args:?}");
        assert!(out(&o).contains("사용법: pebble-cli"), "{args:?}");
    }
}

#[test]
fn unknown_command_exits_with_usage_error() {
    let o = plain(&["bogus"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(err(&o).contains("사용법"));
}

#[test]
fn request_runs_through_control_socket_and_prints_status() {
    let dir = scratch();
    let server = fake_server(
        &dir,
        vec![ok(
            r#"{"running":true,"device_name":"내기기","peer_id":"12D3","shared_folder":"/s"}"#,
        )],
    );
    let o = run_cli(&dir, &["status"]);
    assert!(o.status.success(), "{}", err(&o));
    let request = server.join().unwrap();
    assert!(request.contains(r#""cmd":"status""#), "{request}");
    let text = out(&o);
    assert!(text.contains("실행 중   true"), "{text}");
    assert!(text.contains("이름      내기기"), "{text}");
    assert!(text.contains("PeerId    12D3"), "{text}");
    assert!(text.contains("공유 폴더 /s"), "{text}");
}

#[test]
fn json_mode_prints_raw_lines_only() {
    let dir = scratch();
    let replies = vec![
        event("pairing-sent", r#"{"pin":"9"}"#),
        ok(r#"{"running":true,"device_name":"n","peer_id":"p","shared_folder":"/s"}"#),
    ];
    let server = fake_server(&dir, replies.clone());
    let o = run_cli(&dir, &["status", "--json"]);
    server.join().unwrap();
    assert!(o.status.success());
    let text = out(&o);
    assert_eq!(text, format!("{}\n{}\n", replies[0], replies[1]));
}

#[test]
fn text_mode_shows_events_and_progress() {
    let dir = scratch();
    let server = fake_server(
        &dir,
        vec![
            event("pairing-sent", r#"{"pin":"4242"}"#),
            event(
                "transfer-started",
                r#"{"transfer_id":"t1","file_name":"a.txt","total_size":7}"#,
            ),
            event(
                "transfer-started",
                r#"{"transfer_id":"t0","file_name":"empty.txt","total_size":0}"#,
            ),
            event("transfer-progress", r#"{"progress":42.5}"#),
            event("other", "{}"),
            ok("{}"),
        ],
    );
    let o = run_cli(&dir, &["connect", "/ip4/1.1.1.1/tcp/1"]);
    server.join().unwrap();
    assert!(o.status.success());
    let text = out(&o);
    assert!(
        text.contains("PIN 4242 — 상대 기기에 같은 PIN 이 보이면"),
        "{text}"
    );
    assert!(text.contains("전송 시작 [t1] a.txt (7 B)"), "{text}");
    assert!(!text.contains("empty.txt"), "{text}");
    assert!(text.contains("완료"));
    assert!(err(&o).contains("진행  42.5%"), "{}", err(&o));
}

#[test]
fn error_reply_is_reported_in_text_and_silent_in_json() {
    let reply = r#"{"type":"error","code":"not_found","message":"없음"}"#.to_string();
    let dir = scratch();
    let server = fake_server(&dir, vec![reply.clone()]);
    let o = run_cli(&dir, &["pair", "x"]);
    server.join().unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("오류: 없음 (not_found)"), "{}", err(&o));

    let dir = scratch();
    let server = fake_server(&dir, vec![reply.clone()]);
    let o = run_cli(&dir, &["pair", "x", "--json"]);
    server.join().unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(err(&o), "");
    assert_eq!(out(&o), format!("{reply}\n"));
}

#[test]
fn missing_daemon_and_dropped_connection_are_reported() {
    let dir = scratch();
    let o = run_cli(&dir, &["status"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("Pebble 이 켜져 있지 않습니다"));

    let dir = scratch();
    let server = fake_server(&dir, vec![]);
    let o = run_cli(&dir, &["status"]);
    server.join().unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("응답 없이 연결을 끊었습니다"));
}

#[test]
fn peers_table_and_empty_message() {
    let dir = scratch();
    let server = fake_server(&dir, vec![ok("[]")]);
    let o = run_cli(&dir, &["peers"]);
    server.join().unwrap();
    assert!(out(&o).contains("보이는 기기가 없습니다."));

    let dir = scratch();
    let server = fake_server(
        &dir,
        vec![ok(
            r#"[{"device_name":"A","connected":true,"trusted":false,"pending_pin":"555","peer_id":"P1"},{"device_name":"B","connected":false,"trusted":true,"peer_id":"P2"}]"#,
        )],
    );
    let o = run_cli(&dir, &["peers"]);
    server.join().unwrap();
    let text = out(&o);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3, "{text}");
    assert!(lines[0].contains("이름") && lines[0].contains("PeerId"));
    let cols: Vec<&str> = lines[1].split_whitespace().collect();
    assert_eq!(cols, ["A", "예", "-", "555", "P1"]);
    let cols: Vec<&str> = lines[2].split_whitespace().collect();
    assert_eq!(cols, ["B", "-", "예", "-", "P2"]);
}

#[test]
fn ls_pair_send_get_and_simple_results() {
    let dir = scratch();
    let server = fake_server(
        &dir,
        vec![ok(
            r#"[{"is_dir":true,"size":0,"path":"/d"},{"is_dir":false,"size":12,"path":"/f"}]"#,
        )],
    );
    let o = run_cli(&dir, &["ls", "dev"]);
    let request = server.join().unwrap();
    assert!(request.contains(r#""path":"/""#), "{request}");
    let text = out(&o);
    assert!(text.contains("폴더") && text.contains("/d"), "{text}");
    assert!(
        text.contains("파일") && text.contains("12") && text.contains("/f"),
        "{text}"
    );

    let dir = scratch();
    let server = fake_server(&dir, vec![ok(r#"{"device_name":"상대"}"#)]);
    let o = run_cli(&dir, &["pair", "dev"]);
    server.join().unwrap();
    assert!(out(&o).contains("페어링됨: 상대"));

    let dir = scratch();
    let server = fake_server(
        &dir,
        vec![ok(
            r#"{"payload":{"saved_path":"/dl/x.bin","file_name":"x.bin"}}"#,
        )],
    );
    let o = run_cli(&dir, &["get", "dev", "/x.bin"]);
    server.join().unwrap();
    assert!(out(&o).contains("완료: /dl/x.bin"), "{}", out(&o));

    let dir = scratch();
    let file = dir.join("up.txt");
    std::fs::write(&file, b"hi").unwrap();
    let server = fake_server(&dir, vec![ok(r#"{"payload":{"file_name":"up.txt"}}"#)]);
    let o = run_cli(&dir, &["send", file.to_str().unwrap(), "dev"]);
    let request = server.join().unwrap();
    assert!(request.contains(r#""cmd":"send""#));
    assert!(out(&o).contains("완료: up.txt"), "{}", out(&o));

    for args in [&["accept", "d"][..], &["reject", "d"], &["cancel", "t"]] {
        let dir = scratch();
        let server = fake_server(&dir, vec![ok("null")]);
        let o = run_cli(&dir, args);
        server.join().unwrap();
        assert_eq!(out(&o).trim(), "완료", "{args:?}");
    }
}

#[test]
fn control_characters_in_reply_are_neutralised() {
    let dir = scratch();
    let server = fake_server(&dir, vec![ok(r#"{"device_name":"a\nb"}"#)]);
    let o = run_cli(&dir, &["pair", "dev"]);
    server.join().unwrap();
    assert!(out(&o).contains("페어링됨: a?b"), "{}", out(&o));
}

fn wait_until(what: &str, mut cond: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(60);
    while !cond() {
        assert!(Instant::now() < deadline, "시간 초과: {what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

struct KillOnDrop(std::process::Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl std::ops::Deref for KillOnDrop {
    type Target = std::process::Child;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for KillOnDrop {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn local_hostname() -> String {
    let raw = Command::new("hostname").output().unwrap().stdout;
    String::from_utf8(raw)
        .unwrap()
        .trim()
        .split('.')
        .next()
        .unwrap()
        .to_string()
}

#[test]
fn daemon_serves_until_terminated_and_cleans_up() {
    let dir = scratch();
    let shared = dir.join("share");
    let downloads = dir.join("dl");
    std::fs::create_dir_all(&shared).unwrap();
    std::fs::create_dir_all(&downloads).unwrap();
    let port = free_port().to_string();
    let mut child = KillOnDrop(
        Command::new(BIN)
            .args(["daemon", "--no-tailscale", "--port", &port, "--shared"])
            .arg(&shared)
            .arg("--downloads")
            .arg(&downloads)
            .arg("--config-dir")
            .arg(&dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let socket = dir.join("control.sock");
    wait_until("제어 소켓", || socket.exists());

    let mut status = None;
    wait_until("status 응답", || {
        let o = run_cli(&dir, &["status"]);
        if o.status.success() {
            status = Some(out(&o));
            true
        } else {
            assert!(child.try_wait().unwrap().is_none(), "daemon 이 먼저 종료됨");
            false
        }
    });
    let status = status.unwrap();
    assert!(
        status.contains(&format!("이름      {}", local_hostname())),
        "{status}"
    );
    assert!(child.try_wait().unwrap().is_none());

    let killed = Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status()
        .unwrap();
    assert!(killed.success());
    wait_until("daemon 종료", || child.try_wait().unwrap().is_some());
    let mut stdout = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();
    assert!(stdout.contains("Pebble daemon 시작"), "{stdout}");
    assert!(stdout.contains(&format!("포트       {port}")), "{stdout}");
    assert!(stdout.contains("Tailscale  끔"), "{stdout}");
    assert!(stdout.contains("Pebble daemon 종료"), "{stdout}");
    assert!(stdout.contains("Z] 대기 주소 "), "{stdout}");
    assert!(!socket.exists());
}

#[test]
fn saved_path_from_a_peer_cannot_drive_the_terminal() {
    let dir = scratch();
    let server = fake_server(
        &dir,
        vec![ok(
            r#"{"payload":{"saved_path":"/dl/a\n[00:00:00Z] 페어링됨\u001b[2J.bin","file_name":"a.bin"}}"#,
        )],
    );
    let o = run_cli(&dir, &["get", "dev", "/a.bin"]);
    server.join().unwrap();
    let text = out(&o);
    assert!(!text.contains('\u{1b}'), "{text:?}");
    assert!(
        !text.lines().any(|l| l.starts_with("[00:00:00Z]")),
        "{text:?}"
    );
    assert!(
        text.contains("완료: /dl/a?[00:00:00Z] 페어링됨?[2J.bin"),
        "{text:?}"
    );
}
