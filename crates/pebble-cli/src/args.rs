use std::path::PathBuf;

use pebble_core::adapters::control::ControlRequest;

pub const USAGE: &str = "\
사용법: pebble-cli <명령> [옵션]

  daemon [--name 이름] [--shared 폴더] [--downloads 폴더] [--port 번호] [--no-tailscale]
                              화면 없이 Pebble 을 켜 둡니다 (Ctrl-C 로 끔)
  status                      켜져 있는지, 내 PeerId·이름·공유 폴더
  peers                       보이는 기기 목록 (연결·페어링·대기 중인 PIN)
  connect <주소>              주소로 직접 연결 (예: /ip4/100.64.0.2/tcp/47123)
  pair <기기>                 페어링 요청을 보내고 상대 승인을 기다림
  accept <기기> | reject <기기>
                              나에게 온 페어링 요청을 승인/거절
  ls <기기> [경로]            상대 공유 폴더 목록
  send <파일> <기기> [폴더]   내 파일을 상대 공유 폴더로 보냄 (끝날 때까지 기다림)
  get <기기> <경로>           상대 공유 폴더의 파일을 내 다운로드 폴더로 받음
  cancel <전송ID>             전송 취소

공통 옵션:
  --json                      결과를 JSON 줄로 출력 (에이전트용)
  --config-dir <폴더>         설정 폴더 (기본: 사용자 설정 폴더/pebble)

<기기> 는 기기 이름, PeerId, 또는 PeerId 앞 6자 이상입니다.";

pub struct DaemonOptions {
    pub name: Option<String>,
    pub shared: Option<PathBuf>,
    pub downloads: Option<PathBuf>,
    pub port: Option<u16>,
    pub tailscale: bool,
}

pub enum Command {
    Daemon(DaemonOptions),
    Request(ControlRequest),
}

pub struct Cli {
    pub command: Command,
    pub json: bool,
    pub config_dir: Option<PathBuf>,
}

pub fn parse(raw: Vec<String>) -> Result<Cli, String> {
    let mut json = false;
    let mut config_dir = None;
    let mut options: Vec<(String, String)> = Vec::new();
    let mut flags: Vec<String> = Vec::new();
    let mut words: Vec<String> = Vec::new();
    let mut iter = raw.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--json" => json = true,
            "--no-tailscale" => flags.push(arg),
            "--config-dir" | "--name" | "--shared" | "--downloads" | "--port" => {
                let value = iter
                    .next()
                    .ok_or_else(|| format!("{arg} 뒤에 값이 필요합니다"))?;
                if arg == "--config-dir" {
                    config_dir = Some(PathBuf::from(value));
                } else {
                    options.push((arg, value));
                }
            }
            s if s.starts_with("--") => return Err(format!("모르는 옵션: {s}")),
            _ => words.push(arg),
        }
    }
    let command = command(&words, &options, &flags)?;
    Ok(Cli {
        command,
        json,
        config_dir,
    })
}

fn command(
    words: &[String],
    options: &[(String, String)],
    flags: &[String],
) -> Result<Command, String> {
    let (name, rest) = words.split_first().ok_or("명령이 없습니다")?;
    if name != "daemon" && (!options.is_empty() || !flags.is_empty()) {
        return Err(
            "--name·--shared·--downloads·--port·--no-tailscale 는 daemon 에서만 씁니다".into(),
        );
    }
    let arg = |i: usize, what: &str| {
        rest.get(i)
            .cloned()
            .ok_or(format!("{name}: {what} 이(가) 필요합니다"))
    };
    let request = match (name.as_str(), rest.len()) {
        ("daemon", 0) => return daemon(options, flags).map(Command::Daemon),
        ("status", 0) => ControlRequest::Status,
        ("peers", 0) => ControlRequest::Peers,
        ("connect", 1) => ControlRequest::Connect {
            address: arg(0, "주소")?,
        },
        ("pair", 1) => ControlRequest::Pair {
            peer: arg(0, "기기")?,
        },
        ("accept", 1) => ControlRequest::Respond {
            peer: arg(0, "기기")?,
            approve: true,
        },
        ("reject", 1) => ControlRequest::Respond {
            peer: arg(0, "기기")?,
            approve: false,
        },
        ("ls", 1 | 2) => ControlRequest::Ls {
            peer: arg(0, "기기")?,
            path: rest.get(1).cloned().unwrap_or_else(|| "/".into()),
        },
        ("send", 2 | 3) => ControlRequest::Send {
            file: absolute(&arg(0, "파일")?)?,
            peer: arg(1, "기기")?,
            dir: rest.get(2).cloned().unwrap_or_else(|| "/".into()),
        },
        ("get", 2) => ControlRequest::Get {
            peer: arg(0, "기기")?,
            path: arg(1, "경로")?,
        },
        ("cancel", 1) => ControlRequest::Cancel {
            transfer: arg(0, "전송ID")?,
        },
        _ => {
            return Err(format!(
                "명령 '{name}' 의 인자 수가 맞지 않거나 모르는 명령입니다"
            ))
        }
    };
    Ok(Command::Request(request))
}

fn daemon(options: &[(String, String)], flags: &[String]) -> Result<DaemonOptions, String> {
    let get = |key: &str| {
        options
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    };
    let port = get("--port")
        .map(|p| {
            p.parse::<u16>()
                .map_err(|_| format!("포트 번호가 올바르지 않습니다: {p}"))
        })
        .transpose()?;
    Ok(DaemonOptions {
        name: get("--name"),
        shared: get("--shared").map(|p| absolute_dir(&p)).transpose()?,
        downloads: get("--downloads").map(|p| absolute_dir(&p)).transpose()?,
        port,
        tailscale: !flags.iter().any(|f| f == "--no-tailscale"),
    })
}

fn absolute_dir(path: &str) -> Result<PathBuf, String> {
    std::path::absolute(path).map_err(|e| format!("경로를 확인하지 못했습니다 {path}: {e}"))
}

fn absolute(path: &str) -> Result<PathBuf, String> {
    std::fs::canonicalize(path).map_err(|e| format!("파일을 찾지 못했습니다 {path}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(line: &str) -> Vec<String> {
        line.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn parses_client_commands_with_defaults() {
        let cli = parse(args("ls done --json")).unwrap();
        assert!(cli.json);
        assert!(
            matches!(cli.command, Command::Request(ControlRequest::Ls { ref peer, ref path }) if peer == "done" && path == "/")
        );
        let cli = parse(args("accept office")).unwrap();
        assert!(matches!(
            cli.command,
            Command::Request(ControlRequest::Respond { approve: true, .. })
        ));
    }

    #[test]
    fn parses_daemon_options() {
        let cli = parse(args(
            "daemon --name linux --port 4000 --no-tailscale --config-dir /tmp/p",
        ))
        .unwrap();
        let Command::Daemon(opts) = cli.command else {
            panic!("daemon 이어야 합니다")
        };
        assert_eq!(opts.name.as_deref(), Some("linux"));
        assert_eq!(opts.port, Some(4000));
        assert!(!opts.tailscale);
        assert_eq!(cli.config_dir, Some(PathBuf::from("/tmp/p")));
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse(args("")).is_err());
        assert!(parse(args("get done")).is_err());
        assert!(parse(args("peers --port 1")).is_err());
        assert!(parse(args("daemon --port abc")).is_err());
        assert!(parse(args("send /no/such/file done")).is_err());
    }

    fn request(line: &str) -> ControlRequest {
        match parse(args(line)).unwrap().command {
            Command::Request(r) => r,
            Command::Daemon(_) => panic!("요청이어야 합니다"),
        }
    }

    #[test]
    fn parses_every_zero_and_one_argument_command() {
        assert!(matches!(request("status"), ControlRequest::Status));
        assert!(matches!(request("peers"), ControlRequest::Peers));
        assert!(
            matches!(request("connect /ip4/1.2.3.4/tcp/1"), ControlRequest::Connect { address } if address == "/ip4/1.2.3.4/tcp/1")
        );
        assert!(matches!(request("pair dev"), ControlRequest::Pair { peer } if peer == "dev"));
        assert!(
            matches!(request("reject dev"), ControlRequest::Respond { peer, approve: false } if peer == "dev")
        );
        assert!(
            matches!(request("cancel t1"), ControlRequest::Cancel { transfer } if transfer == "t1")
        );
        assert!(
            matches!(request("get dev /a/b"), ControlRequest::Get { peer, path } if peer == "dev" && path == "/a/b")
        );
    }

    #[test]
    fn wrong_argument_counts_are_rejected() {
        for line in [
            "status x",
            "peers x",
            "connect",
            "connect a b",
            "pair",
            "pair a b",
            "reject",
            "reject a b",
            "get",
            "get a b c",
            "cancel",
            "cancel a b",
            "send a",
            "send a b c d",
            "bogus",
        ] {
            assert!(parse(args(line)).is_err(), "{line}");
        }
    }

    #[test]
    fn send_resolves_file_and_defaults_dir() {
        let file = std::env::temp_dir().join(format!("pbB-args-{}.txt", std::process::id()));
        std::fs::write(&file, b"x").unwrap();
        let canonical = std::fs::canonicalize(&file).unwrap();
        let line = format!("send {} dev", file.display());
        assert!(
            matches!(request(&line), ControlRequest::Send { file, peer, dir } if file == canonical && peer == "dev" && dir == "/")
        );
        let line = format!("send {} dev /up", file.display());
        assert!(matches!(request(&line), ControlRequest::Send { dir, .. } if dir == "/up"));
        std::fs::remove_file(&file).unwrap();
    }

    #[test]
    fn unknown_option_is_reported_as_option() {
        let err = parse(args("status --bogus")).err().unwrap();
        assert!(err.contains("모르는 옵션: --bogus"), "{err}");
    }

    #[test]
    fn daemon_folders_become_absolute() {
        let cli = parse(args("daemon --shared rel/share --downloads rel/dl")).unwrap();
        let Command::Daemon(opts) = cli.command else {
            panic!("daemon 이어야 합니다")
        };
        assert_eq!(opts.shared, Some(std::path::absolute("rel/share").unwrap()));
        assert_eq!(opts.downloads, Some(std::path::absolute("rel/dl").unwrap()));
        assert!(opts.tailscale);
    }
}
