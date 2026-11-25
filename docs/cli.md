# Pebble CLI (화면 없이 쓰기)

화면 없는 Linux 서버나 원격 터미널에서 Pebble 을 쓰는 방법이다. Mac 에서 Pebble 앱이 켜져 있으면 같은 `pebble-cli` 명령이 그 앱에 붙어서 동작한다.

## 구조

```
crates/pebble-core   도메인 · 엔진(액터) · 포트 · 어댑터(libp2p, 파일, Tailscale, 제어 소켓)
crates/pebble-cli    화면 없는 입구: daemon + 명령
src-tauri            화면 입구: Tauri 앱 (pebble-core + server 기능)
```

- `pebble-cli` 는 Tauri·WebKit·OpenSSL 없이 빌드된다.
- daemon 이나 앱은 `<설정 폴더>/control.sock` 을 연다. 그 파일은 나만 읽고 쓸 수 있다(0600). `pebble-cli` 명령은 이 소켓으로 켜져 있는 Pebble 에게 일을 시킨다.
- 설정 폴더는 Mac 이 `~/Library/Application Support/pebble`, Linux 가 `~/.config/pebble` 이다.
  - `identity.key`: 기기 키. 지우면 PeerId 가 바뀌고 다시 페어링해야 한다.
  - `trusted-peers.json`: 페어링한 기기 목록.
  - `control.sock`: CLI 연결 통로.

## 설치

Rust(1.88 이상)와 C 컴파일러(Linux 는 `build-essential`)가 필요하다.

```bash
git clone <저장소> pebble && cd pebble
cargo install --path crates/pebble-cli
```

설치하면 `~/.cargo/bin/pebble-cli` 가 생긴다.

개발할 때 저장소 루트의 `cargo build`·`cargo test` 는 `pebble-core`·`pebble-cli` 만 다룬다. 화면 앱(`src-tauri`)은 서버 등록 키(`src-tauri/.env`)가 있어야 빌드되므로 `pnpm tauri dev` 나 `cargo test --workspace` 로 따로 돌린다.

## 켜 두기

```bash
pebble-cli daemon --name done --shared ~/Pebble/Shared
```

| 옵션 | 기본값 | 뜻 |
|---|---|---|
| `--name` | 호스트 이름 | 다른 기기에 보이는 이름 |
| `--shared` | `~/Pebble/Shared` | 다른 기기가 보고·올리는 폴더 |
| `--downloads` | 다운로드 폴더(없으면 `~/Downloads`) | `get` 으로 받은 파일 위치 |
| `--port` | 47123 | 기기끼리 붙는 포트(TCP·UDP). 세 기기 모두 같아야 자동 발견된다 |
| `--no-tailscale` | 켜짐 | Tailscale 자동 발견 끄기 |
| `--config-dir` | 위 설정 폴더 | 경로는 100자 이하여야 한다(소켓 경로 길이 제한) |

### Linux 에서 로그인 없이 계속 켜 두기 (systemd 사용자 서비스)

`~/.config/systemd/user/pebble.service`:

```ini
[Unit]
Description=Pebble daemon
After=network-online.target tailscaled.service

[Service]
ExecStart=%h/.cargo/bin/pebble-cli daemon --name done --shared %h/Pebble/Shared
Restart=on-failure

[Install]
WantedBy=default.target
```

```bash
systemctl --user daemon-reload
systemctl --user enable --now pebble
loginctl enable-linger $USER
journalctl --user -u pebble -f
```

## 기기 찾기

- **Tailscale**: 30초마다 `tailscale status --json` 을 읽는다. **나와 같은 Tailscale 계정이고 켜져 있는 기기**에만 `100.x.x.x:47123` 으로 연결을 시도한다. 같은 망의 다른 사람 기기에는 시도하지 않는다. 태그가 붙은 기기(서버 등)는 계정이 아니라 태그로 묶여 주인을 가릴 수 없으므로 자동 대상에서 뺀다. 그런 기기는 `connect` 로 직접 붙인다. 실패는 화면이나 로그에 남기지 않는다.
- **같은 와이파이**: 예전처럼 mDNS 로 찾는다. 찾기만 하고 자동으로 붙지는 않으니 daemon 로그의 주소로 `connect` 한다.
- **직접**: `pebble-cli connect /ip4/100.64.0.2/tcp/47123`

## 처음 한 번: 페어링

```bash
pebble-cli peers
pebble-cli pair done
```

받는 쪽 화면(Mac 앱)이나 daemon 로그에 PIN 이 뜬다. 화면 없는 쪽에서는 이렇게 승인한다.

```bash
pebble-cli peers
pebble-cli accept office-mac
```

두 기기의 PIN 이 같은지 확인하고 승인한다. 페어링은 `trusted-peers.json` 에 남아서, 껐다 켜도 다시 하지 않는다.

## 페어링 해제

```bash
pebble-cli peers
pebble-cli unpair office-mac
```

- 내 쪽 믿는 기기 목록(`trusted-peers.json`)에서만 뺀다. 상대 기기의 목록은 그대로이니, 상대도 끊으려면 그쪽에서 `unpair` 한다.
- 해제하면 그 기기와 진행 중이던 전송은 실패로 끝나고, 그 기기에서 온 파일·목록 요청은 `페어링이 필요합니다.` 로 거절된다. 연결 자체는 남아 있을 수 있으며 `peers` 에서 `페어링` 칸이 `-` 로 바뀐다.
- 이미 다 받아 저장(마무리) 중이던 파일은 해제와 상관없이 저장을 끝낸다.
- 다시 쓰려면 `pair` 로 PIN 을 새로 승인한다. Tailscale 자동 발견은 연결만 시도하고 페어링은 하지 않는다.
- 믿는 목록에 없는 기기를 해제하면 `NOT_FOUND`(종료 코드 1). `trusted-peers.json` 을 못 읽는 상태면 `IO`.
- 실행 중인 화면 앱에는 해제 알림이 가지 않는다. 앱의 페어링 표시는 다음 연결·재시작 때 맞춰진다.

## 파일 주고받기

```bash
pebble-cli ls done
pebble-cli ls done /docs
pebble-cli send ./report.pdf done /inbox
pebble-cli get done /docs/big.bin
pebble-cli cancel <전송ID>
```

- `send` 는 상대 공유 폴더의 `<폴더>` 에 넣는다. `get` 은 내 다운로드 폴더에 받는다.
- 둘 다 끝날 때까지 기다리고, 실패하면 종료 코드 1 을 돌려준다.
- 페어링한 기기와만 주고받는다. 페어링 요청은 170초 안에 승인해야 한다.
- 명령을 Ctrl-C 로 끊어도 전송은 켜져 있는 Pebble 에서 계속된다. 멈추려면 `cancel` 을 쓴다.
- 진행 알림을 너무 많이 놓치면(동시에 큰 전송 여러 개) 명령이 "알림을 놓쳤습니다" 로 끝난다. 전송 자체는 계속되니 받은 폴더나 `peers` 로 확인한다.
- 같은 이름의 파일이 이미 있으면 덮어쓰지 않고 `이름 (1).확장자` 로 저장한다.
- `<기기>` 에는 기기 이름, PeerId, PeerId 앞 6자 이상을 쓸 수 있다.

## 에이전트에서 쓰기

`--json` 을 붙이면 한 줄에 JSON 하나씩 나온다.

- `{"type":"event","event":{...}}`: 진행 중 알림(전송 시작·진행률, 페어링 PIN)
- `{"type":"ok","data":...}`: 성공. 이 줄이 마지막이다.
- `{"type":"error","code":"NOT_FOUND","message":"..."}`: 실패. 종료 코드는 1 이다.

```bash
pebble-cli send ./out.tar.gz done /inbox --json | tail -1
```
