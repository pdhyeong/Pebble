# Pebble IPC 계약 (프론트엔드 ↔ Rust 백엔드)

이 문서가 정본이다. 프론트엔드(`src/`)와 Rust(`src-tauri/`)는 이 문서에 맞춘다. 바꿀 때는 이 문서부터 고친다.

## 공통 규칙

- 모든 이벤트 payload 는 **객체**다. JSON 문자열로 한 번 더 감싸지 않는다. 프론트는 `JSON.parse` 하지 않는다.
- 필드 이름은 `snake_case`.
- 명령 인자 이름은 Tauri 규칙대로 프론트에서 `camelCase` 로 넘긴다(예: `peerId`).
- 명령이 실패하면 아래 `AppError` 객체가 reject 된다.

```ts
type ErrorCode =
  | "P2P_NOT_RUNNING" | "P2P_ALREADY_RUNNING" | "INVALID_PEER_ID" | "INVALID_PATH"
  | "ACCESS_DENIED" | "NOT_FOUND" | "IO" | "NETWORK" | "ENGINE_UNAVAILABLE" | "INTERNAL";
interface AppError { code: ErrorCode; message: string; }
```

## 명령 (invoke)

| 명령 | 인자 | 반환 | 비고 |
|---|---|---|---|
| `start_p2p` | — | `null` | 이미 실행 중이면 `P2P_ALREADY_RUNNING` |
| `stop_p2p` | — | `null` | 엔진이 완전히 멈춘 뒤 돌아온다. `p2p-stopped` 이벤트는 돌아오기 전에 온다 |
| `get_p2p_status` | — | `boolean` | |
| `connect_to_peer` | `addr: string` | `null` | multiaddr |
| `request_file_list` | `peerId, path` | `null` | 결과는 `remote-files` 이벤트 |
| `download_file` | `peerId, path` | `string` (transfer_id) | 받은 파일은 OS 다운로드 폴더에 저장. 페어링 안 된 기기면 `ACCESS_DENIED` |
| `upload_file` | `peerId, filePath, remotePath` | `string` (transfer_id) | `filePath` 는 내 기기의 절대 경로, `remotePath` 는 상대 공유 폴더 안 상대 경로. 페어링 안 된 기기면 `ACCESS_DENIED` |
| `cancel_transfer` | `transferId` | `null` | |
| `request_pairing` | `peerId` | `string` (PIN 6자리) | `pairing-sent` 이벤트도 온다 |
| `respond_pairing` | `peerId, approved: boolean` | `null` | 요청을 받은 지 170초가 지났으면 `NOT_FOUND`. 믿는 기기 목록을 저장하지 못하면 `IO` 이고 상대에게는 거절이 간다 |
| `set_device_name` / `get_device_name` | `name` / — | `null` / `string` | |
| `set_shared_folder` / `get_shared_folder` | `path` / — | `null` / `string` | 설정하면 폴더를 만들고 **파일 감시도 다시 시작**한다 |
| `set_shared_folder_display_name` / `get_shared_folder_display_name` | `name` / — | `null` / `string` | |
| `set_show_folder_name` / `get_show_folder_name` | `show` / — | `null` / `boolean` | |
| `get_local_shared_files` | `relativePath` | `LocalFileInfo[]` | 공유 폴더 밖은 `ACCESS_DENIED` |
| `get_local_ip` | — | `string` | |
| `get_activity_history` | — | `ActivityRecord[]` | 최근 100개, 오래된 것부터 |
| `get_shared_folder_stats` | — | `{ total_size: number, file_count: number }` | |

없어진 명령: `greet`, `get_file_list`, `refresh_watcher` (→ `set_shared_folder` 가 감시를 다시 시작한다).

앱 시작 순서: 프론트가 저장된 설정을 읽어 `set_device_name` → `set_shared_folder_display_name` → `set_show_folder_name` → `set_shared_folder` 를 부른 뒤 `start_p2p`.

```ts
interface LocalFileInfo { name: string; path: string; is_dir: boolean; size: number; }
interface ActivityRecord {
  id: string; transfer_type: "upload" | "download"; file_name: string; peer_id: string;
  device_name: string | null; size: number; timestamp: number;
  status: "completed" | "failed" | "cancelled"; speed: number | null;
}
```

## 이벤트 (listen)

### 엔진·연결
| 이벤트 | payload |
|---|---|
| `p2p-stopped` | `{ message: string }` |
| `listening-on` | `{ address: string }` |
| `peer-found` | `{ peer_id: string, address: string }` (mDNS 로 찾았거나, Tailscale 자동 발견으로 연결된 기기) |
| `peer-expired` | `{ peer_id: string }` |
| `peer-info` | `{ peer_id: string, agent_version: string }` |
| `ping-success` | `{ peer_id: string, rtt_ms: number }` |
| `dial-started` | `{ address: string }` |
| `dial-failed` | `{ address: string, error: string }` |
| `connection-success` | `{ peer_id: string }` |
| `connection-closed` | `{ peer_id: string }` |
| `device-info-received` | `{ peer_id: string, device_name: string }` |

### 파일 목록·폴더
| 이벤트 | payload |
|---|---|
| `file-list-request-sent` | `{ peer_id: string, path: string }` |
| `remote-files` | `{ peer_id: string, path: string, files: LocalFileInfo[], error: string \| null }` |
| `file-list-error` | `{ peer_id: string, error: string }` |
| `remote-directory-changed` | `{ peer_id: string }` |
| `local-files-changed` | `{ path: string, kind: "modified" }` |
| `shared-folder-changed` | `{ path: string }` |

### 전송 (업로드·다운로드 공통)
옛 `file-download-*`, `file-upload-*`, `file-received`, `transfer-cancelled` 이벤트는 모두 아래 다섯 개로 바뀐다.
`direction` 은 **내 기기 기준**이다. 내가 보내면 `"upload"`, 내가 받으면 `"download"`(상대가 나에게 올린 것도 `"download"`).

```ts
type Direction = "upload" | "download";
```

| 이벤트 | payload |
|---|---|
| `transfer-started` | `{ transfer_id, direction, peer_id, file_name, total_size: number }` |
| `transfer-progress` | `{ transfer_id, direction, bytes_done: number, total_size: number, progress: number /*0~100*/ }` |
| `transfer-completed` | `{ transfer_id, direction, peer_id, file_name, total_size: number, saved_path: string \| null, speed_bps: number \| null }` |
| `transfer-failed` | `{ transfer_id, direction, peer_id, file_name, error: string }` |
| `transfer-cancelled` | `{ transfer_id, direction, peer_id, file_name, reason: "user_cancelled" \| "peer_cancelled" }` |

- 다운로드는 `download_file` 이 돌려준 transfer_id 로 `transfer-started`(total_size 0)가 먼저 오고, 상대가 보내기 시작하면 같은 transfer_id 로 `transfer-started` 가 실제 크기와 함께 한 번 더 온다. 프론트는 같은 id 면 덮어쓴다.
- `saved_path` 는 `download` 일 때만 값이 있다. 같은 이름 파일이 있으면 `이름 (1).확장자` 처럼 바뀐 실제 경로다.

### 페어링
| 이벤트 | payload |
|---|---|
| `pairing-request` | `{ peer_id, device_name, pin }` |
| `pairing-sent` | `{ peer_id, pin }` |
| `pairing-approved` | `{ peer_id, device_name, shared_folder_name: string \| null }` — 내 요청이 승인됐을 때, 내가 승인했을 때, 이미 페어링한 기기가 다시 연결됐을 때 온다. 앞의 경우 말고는 `shared_folder_name` 이 null |
| `pairing-rejected` | `{ peer_id, error: string \| null }` |

## 기기 사이 통신 (참고 — 프론트와 무관)

- `/pebble/file-transfer/1.0.0`(파일 통째 전송) 은 없어졌다. 다운로드도 청크 스트림으로 한다.
- 스트림 프로토콜은 `/pebble/file-stream/2.0.0` 이다. 옛 버전 앱과는 파일을 주고받지 못한다.
- 한 메시지 최대 크기: JSON 8 MiB, 스트림 1 MiB + 64 KiB. 넘으면 읽기 전에 끊는다.
- 스트림 `Init` 에는 `for_pull`(다운로드 요청에 대한 응답인지)이 들어 있다. 요청하지 않았거나 이미 취소한 다운로드의 `Init` 은 거절한다.
- 조각 크기는 마지막 조각을 빼고 정확히 1 MiB 여야 한다. 크기가 다르면 그 전송은 실패한다.
- 기본 포트는 TCP·UDP 47123 이다. 기기 키는 설정 폴더 `identity.key` 에 남아서 PeerId 가 바뀌지 않는다. 페어링은 `trusted-peers.json` 에 남는다.
- `trusted-peers.json` 을 읽지 못하면(손상·권한) 그 파일을 덮어쓰지 않도록 새 페어링 저장을 막는다. 파일을 고치거나 지우고 다시 시작한다.
- 상대가 보낸 기기 이름은 64자까지만 쓴다.
- CLI 연결 통로는 `docs/cli.md` 를 본다.
