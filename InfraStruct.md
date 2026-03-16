# [최종 설계안] Pebble: 하이브리드 P2P 및 Oracle Cloud 스토리지 시스템

## 1. 프로젝트 정의

Pebble은 **"언제 어디서나 내 기기 간 연결"**을 목표로 합니다.
직접 연결(P2P)이 가능할 때는 비용 없이 고속으로 전송하고, 네트워크 환경(NAT)이나 기기 전원 상태로 인해 직접 연결이 불가능할 때는 **Oracle Cloud 인프라(OCI Compute / Object Storage)**를 통해 끊김 없는 서비스를 제공합니다.

> **플랫폼 선택 이유**: AWS 대비 OCI는 Always Free Tier(VM 2대, Object Storage 20GB 포함)를 제공하여 초기 운영 비용을 크게 절감할 수 있습니다.

---

## 2. 네트워크 및 전송 아키텍처

네트워크 환경에 따라 총 3단계의 전송 전략을 자동으로 선택합니다.

| 전송 단계 | 기술 방식 | 작동 조건 | 특징 |
|---|---|---|---|
| **Phase 1: Direct** | Local / Remote P2P | 동일 네트워크 또는 홀펀칭 성공 시 | 서버 비용 0원, 최대 대역폭 활용 |
| **Phase 2: Relay** | Server Relay (OCI VM) | 홀펀칭 실패(Symmetric NAT 등) 시 | 서버를 거쳐 실시간 중계, 100% 연결 보장 |
| **Phase 3: Cloud** | Storage Sync (Object Storage) | 수신 기기가 오프라인일 때 | 서버에 파일 보관 후 나중에 전송 |

---

## 3. 핵심 인프라 구성 (Oracle Cloud — 서울 리전)

### 3.1 제어 및 중계부 (OCI Compute VM)

- **역할**: 시그널링(IP 교환), 릴레이(데이터 중계), 메타데이터 관리
- **서버**: NestJS (TypeScript) — `pebble-server`
- **데이터베이스**: PostgreSQL (VM 내 Docker Compose로 실행)
- **실시간 통신**: Socket.IO WebSocket Gateway (`SignalingGateway`)를 통해 기기 온/오프라인 상태 실시간 관리

**OCI VM 권장 스펙 (Always Free)**
- Shape: `VM.Standard.A1.Flex` (ARM, 최대 4 OCPU / 24 GB RAM 무료)
- OS: Ubuntu 22.04
- 보안 목록(Security List)에서 포트 개방: `3000` (HTTP API), `3001` (WebSocket 선택)

### 3.2 저장부 (OCI Object Storage)

- **역할**: "나만의 클라우드" 우체통. 수신 기기가 꺼진 상태를 위한 임시 파일 저장소
- **접근 방식**: S3 호환 API + `@aws-sdk/client-s3` (SDK 교체 없음)
  - 엔드포인트: `https://<namespace>.compat.objectstorage.ap-seoul-1.oraclecloud.com`
  - `forcePathStyle: true` 필수 설정
- **인증**: OCI IAM → Customer Secret Keys (Access Key / Secret Key)
- **보안**: Presigned URL (PUT/GET, 10분 유효)로 클라이언트가 서버를 거치지 않고 직접 안전하게 업로드/다운로드
- **비용 절감**: 전송 완료 후 Object Lifecycle Policy로 자동 삭제

### 3.3 전체 구성도

```
[Tauri 앱 (Rust/React)]
        │
        ├── P2P 가능 ──────────────────→ [상대 기기] (libp2p QUIC)
        │
        ├── 시그널링/릴레이 ──────────→ [OCI VM: pebble-server (NestJS)]
        │                                      │
        │                                 [PostgreSQL]
        │                                 (기기/파일 메타데이터)
        │
        └── 오프라인 전송 ────────────→ [OCI Object Storage]
                                          (Presigned URL 방식)
```

---

## 4. 핵심 데이터 흐름

### 4.1 P2P 연결 흐름
1. Tauri 앱 시작 → `peer-online` WebSocket 이벤트로 서버에 PeerID + IP 등록
2. 파일 전송 시작 → 서버에서 상대 PeerID의 최신 IP 조회 (`GET /devices/:peerId`)
3. libp2p QUIC으로 직접 연결 시도 → 성공 시 P2P 전송
4. 홀펀칭 실패 시 → 서버 릴레이(Phase 2) 또는 Object Storage(Phase 3)로 자동 Fallback

### 4.2 오프라인 전송 흐름 (Object Storage)
1. 수신자 오프라인 확인 (`isOnline: false`)
2. 서버에 `POST /storage/upload-url` 요청 → Presigned PUT URL 발급
3. Tauri 앱이 Object Storage에 직접 PUT 업로드
4. 서버에 `POST /transfers` 전송 예약 저장 (s3Key 포함)
5. 수신자 온라인 복귀 시 → `GET /transfers/pending`으로 예약 확인 → Presigned GET URL로 다운로드

### 4.3 파일 메타데이터 동기화
- 로컬 폴더 변경(notify 크레이트 감지) → `POST /files/sync`로 메타데이터 upsert
- 상대방은 내 기기가 꺼진 상태에서도 `GET /files?deviceId=xxx`로 폴더 구조 확인 가능

---

## 5. 단계별 구현 로드맵

### 1단계: OCI 인프라 기초 공사
- OCI 콘솔에서 VM 인스턴스 생성 (서울 리전, Always Free ARM)
- 고정 공인 IP 연결 (Reserved Public IP)
- Object Storage 버킷 생성 + CORS 정책 설정
- IAM → Customer Secret Keys 발급

### 2단계: pebble-server 배포 (완료 ✅)
- NestJS 기반 서버 모듈 구조 완성 (auth / devices / signaling / files / storage / transfers)
- `.env` 작성 후 `pnpm run start:prod` 또는 Docker Compose로 배포
- PostgreSQL 컨테이너와 함께 실행

### 3단계: Tauri 클라이언트 — 서버 연동 (🔧 진행 필요)
→ 아래 **"Pebble 클라이언트 작업 목록"** 참조

### 4단계: 하이브리드 전송 엔진 완성
- libp2p로 P2P 직접 연결 우선 시도
- 실패 시 자동 Fallback: 서버 릴레이 → Object Storage 업로드
- 전송 완료 후 예약 상태 업데이트 (`PATCH /transfers/:id/status`)

### 5단계: 사용자 UI/UX 개선
- "클라우드 보관 중", "P2P 전송 중", "릴레이 전송 중" 상태 표시
- 오프라인 수신 파일 알림 및 다운로드 UI

---

## 6. 기술 스택 최종 요약

| 계층 | 기술 | 이유 |
|---|---|---|
| 클라이언트 | Tauri (Rust + React) | 네이티브 성능, 크로스 플랫폼 |
| 백엔드 (WAS) | NestJS (TypeScript) | 모듈 구조, 빠른 개발, WebSocket 내장 |
| 데이터베이스 | PostgreSQL | 메타데이터 관계형 저장 |
| 저장소 | OCI Object Storage | S3 호환 API, Always Free 20GB |
| P2P 통신 | libp2p (QUIC) | UDP 홀펀칭, 고속 전송 |
| 서버 호스팅 | OCI Compute (Always Free) | AWS 대비 비용 절감 |
| 배포 | Docker Compose | VM 내 서버+DB 통합 관리 |

---

---

# 📋 Pebble 클라이언트 작업 목록 (pebble — Tauri/Rust)

서버(`pebble-server`) 연동을 위해 Tauri 앱에서 진행해야 할 작업 목록입니다.

## A. HTTP API 클라이언트 연동

### A-1. 인증 (auth)
- [ ] 회원가입 / 로그인 UI 및 Tauri Command 구현
  - `POST /auth/register`, `POST /auth/login`
- [ ] JWT 토큰을 Tauri 앱의 안전한 저장소에 보관 (예: `tauri-plugin-store` 또는 OS keychain)
- [ ] 모든 API 요청 헤더에 `Authorization: Bearer <token>` 자동 첨부

### A-2. 기기 등록 (devices)
- [ ] 앱 최초 실행 또는 로그인 완료 시 `POST /devices/register` 호출
  - 현재 libp2p PeerID와 기기명 전송
- [ ] 주기적 heartbeat: `PATCH /devices/heartbeat`로 현재 IP + 온라인 상태 갱신
  - 권장 주기: 30초 ~ 1분 (`tokio::time::interval`)

### A-3. 파일 메타데이터 동기화 (files)
- [ ] 기존 `notify` 크레이트 폴더 감시 로직에 훅 추가
  - 파일 생성/수정/삭제 감지 시 `POST /files/sync` 호출
- [ ] 원격 기기 파일 목록 조회: `GET /files?deviceId=xxx`
  - 상대방이 오프라인이어도 서버에서 메타데이터 가져오기

### A-4. 오프라인 전송 (transfers + storage)
- [ ] P2P 연결 실패 + 상대방 오프라인 감지 시 오프라인 전송 흐름 구현
  1. `POST /storage/upload-url` → Presigned PUT URL 받기
  2. `reqwest`로 Object Storage에 직접 PUT 업로드
  3. `POST /transfers` 전송 예약 서버에 저장
- [ ] 앱 시작 시 `GET /transfers/pending` 호출하여 미수신 파일 확인
  1. `POST /storage/download-url` → Presigned GET URL 받기
  2. 파일 다운로드 후 `PATCH /transfers/:id/status` → `COMPLETED`

## B. WebSocket 시그널링 연동 (signaling)

- [ ] Socket.IO 클라이언트를 Rust에서 연결 (또는 Tauri 프론트엔드에서 연결 후 IPC 전달)
  - 권장: `tokio-tungstenite` 또는 프론트엔드 `socket.io-client`
- [ ] 앱 시작 시 `peer-online` 이벤트 emit (PeerID + 현재 IP 포함)
- [ ] `peer-online` / `peer-offline` 이벤트 수신 → P2P 연결 가능 여부 실시간 업데이트
- [ ] P2P 연결 시도 전 `get-peer-address` 이벤트로 상대방 최신 IP 조회

## C. 하이브리드 전송 엔진 (Fallback 로직)

- [ ] 전송 우선순위 결정 함수 구현:
  ```
  1. 서버에서 수신자 isOnline 확인
  2. isOnline=true  → libp2p QUIC P2P 시도 (타임아웃: 5~10초)
     └ 성공 → Phase 1 직접 전송
     └ 실패 → Phase 2 서버 릴레이 (libp2p relay 또는 직접 중계)
  3. isOnline=false → Phase 3 Object Storage 업로드 + 전송 예약
  ```
- [ ] 전송 상태 이벤트를 프론트엔드로 emit하여 UI 업데이트

## D. 환경변수 / 설정

- [ ] Tauri 앱에 `pebble-server` 기본 URL 설정 추가
  - 예: `PEBBLE_SERVER_URL=http://<OCI VM 공인 IP>:3000`
  - `tauri.conf.json`의 `allowlist` 또는 CSP에 서버 도메인 추가
- [ ] 로컬 개발용 URL과 프로덕션 URL 분리 (`build.target` 환경별 `.env` 방식)