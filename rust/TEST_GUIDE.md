# Pebble P2P 파일 동기화 테스트 가이드

## 🎯 개요

Pebble의 Phase 1~3 기능을 같은 맥북에서 2개의 프로세스로 테스트하는 가이드입니다.

## 📋 테스트 시나리오

### 준비사항

```bash
cd /Users/dohyeong/Dohyeong_Storage/Pebble/pebble/rust

# 프로젝트 빌드
cargo build --release --bin test_discovery
```

## 🔍 Phase 2 테스트: 기기 탐색 (Discovery)

같은 LAN에서 두 기기가 서로를 발견하는지 테스트합니다.

### 실행 방법

**터미널 1 (Device A)**
```bash
cargo run --release --bin test_discovery -- "MacBook-A"
```

**터미널 2 (Device B)**
```bash
cargo run --release --bin test_discovery -- "MacBook-B"
```

### 예상 출력

```
============================================================
  Pebble Discovery Test
============================================================
Device Name: MacBook-A
============================================================

🔍 Starting discovery...
✅ Device ID: 550e8400-e29b-41d4-a716-446655440000

🔎 Scanning for 30 seconds...

[2s] Found 1 device(s):
  📱 MacBook-B - 127.0.0.1 (Online)

[4s] Found 1 device(s):
  📱 MacBook-B - 127.0.0.1 (Online)
...

✅ Done
```

### 검증 사항

- ✅ 두 프로세스가 서로를 발견하는가?
- ✅ Device ID가 각각 다른가?
- ✅ IP 주소가 올바르게 표시되는가?
- ✅ 타임아웃 (15초) 후 기기가 사라지는가?

## 🔐 Phase 3 테스트: 암호화된 파일 전송

TLS로 암호화된 파일 전송을 테스트합니다.

### 1단계: 테스트 파일 생성

```bash
# 10MB 테스트 파일 생성
dd if=/dev/urandom of=/tmp/test_file.bin bs=1048576 count=10

# 또는 100MB 테스트 파일
dd if=/dev/urandom of=/tmp/large_file.bin bs=1048576 count=100
```

### 2단계: 수신자 시작

**터미널 1 (Receiver)**
```bash
# 수신자 모드로 서버 시작
cargo run --release --bin test_transfer -- receiver
```

출력 예시:
```
==================================================================
  📥 RECEIVER MODE
==================================================================

🔐 Loading TLS certificate...
✅ Certificate loaded
📋 Fingerprint: a8f5f167f44f4964e6c998dee827110c161f25478228d9e28e58cd9d21a4d6f3
   (Share this with the sender for Certificate Pinning)

📡 Starting transfer server on 0.0.0.0:37846...
🔄 Waiting for incoming files...
   Press Ctrl+C to stop
```

**중요**: Fingerprint를 복사해두세요!

### 3단계: 송신자 시작

**터미널 2 (Sender)**
```bash
# 송신자 모드로 파일 전송
cargo run --release --bin test_transfer -- sender 127.0.0.1 /tmp/test_file.bin
```

Certificate Pinning 입력 (선택):
```
🔐 Certificate Pinning:
   Enter receiver's certificate fingerprint (or press Enter to skip):
   > a8f5f167f44f4964e6c998dee827110c161f25478228d9e28e58cd9d21a4d6f3
```

### 예상 출력

**Sender (터미널 2)**
```
==================================================================
  📤 SENDER MODE
==================================================================

📁 File: /tmp/test_file.bin
📊 Size: 10.00 MB

🎯 Target: 127.0.0.1:37846

🔐 Certificate Pinning:
   ✅ Using Certificate Pinning: a8f5f167...

🚀 Starting file transfer...

==================================================================
  ✅ FILE TRANSFER COMPLETED SUCCESSFULLY
==================================================================
```

**Receiver (터미널 1)**
```
📥 Receiving file: /tmp/test_file.bin (10485760 bytes)
🔄 Chunk 1/10 received (10.0%)
🔄 Chunk 2/10 received (20.0%)
...
🔄 Chunk 10/10 received (100.0%)
✅ File received successfully
```

### 검증 사항

- ✅ TLS 핸드셰이크가 성공하는가?
- ✅ Certificate Pinning이 작동하는가? (잘못된 fingerprint 입력 시 실패)
- ✅ 파일이 완전히 전송되는가?
- ✅ 전송된 파일의 해시가 일치하는가?

### 파일 해시 검증

```bash
# 원본 파일 해시
blake3 /tmp/test_file.bin

# 수신된 파일 해시 (기본 저장 위치)
blake3 /tmp/test_file.bin
```

두 해시가 동일하면 ✅ 전송 성공!

## 🔄 이어받기 (Resume) 테스트

### 시나리오

1. 대용량 파일 전송 중 중단 (Ctrl+C)
2. 다시 시작하면 이어서 받기

### 실행

```bash
# 1. 100MB 파일 생성
dd if=/dev/urandom of=/tmp/large_file.bin bs=1048576 count=100

# 2. 수신자 시작
cargo run --release --bin test_transfer -- receiver

# 3. 송신자 시작
cargo run --release --bin test_transfer -- sender 127.0.0.1 /tmp/large_file.bin

# 4. 전송 중 Ctrl+C로 중단 (예: 50% 진행 시)

# 5. 수신자와 송신자 다시 시작
# 수신자
cargo run --release --bin test_transfer -- receiver

# 송신자
cargo run --release --bin test_transfer -- sender 127.0.0.1 /tmp/large_file.bin
```

### 예상 동작

```
📥 Resuming transfer from chunk 50/100 (50.0%)
🔄 Chunk 51/100 received (51.0%)
...
```

## 🐛 문제 해결

### 포트가 이미 사용 중

```
Error: Address already in use (os error 48)
```

**해결**: 기존 프로세스를 종료하거나 포트를 변경합니다.

```bash
# 포트 사용 확인
lsof -i :37845  # Discovery
lsof -i :37846  # Transfer

# 프로세스 종료
kill -9 <PID>
```

### Certificate Fingerprint 불일치

```
Error: Certificate fingerprint mismatch
```

**해결**: 수신자의 올바른 fingerprint를 입력하거나, Enter를 눌러 검증을 skip합니다.

### 파일을 찾을 수 없음

```
Error: File not found: /tmp/test_file.bin
```

**해결**: 테스트 파일을 먼저 생성합니다.

```bash
dd if=/dev/urandom of=/tmp/test_file.bin bs=1048576 count=10
```

## 📊 성능 벤치마크

### 로컬 전송 속도 (Loopback)

```bash
# 1GB 파일 전송 테스트
dd if=/dev/urandom of=/tmp/1gb_file.bin bs=1048576 count=1024
time cargo run --release --bin test_transfer -- sender 127.0.0.1 /tmp/1gb_file.bin
```

**예상 속도**: ~100-500 MB/s (로컬 loopback)

### LAN 전송 속도

실제 네트워크에서 테스트하려면:

```bash
# 수신자 (다른 맥북)
cargo run --release --bin test_transfer -- receiver

# 송신자 (현재 맥북)
cargo run --release --bin test_transfer -- sender <RECEIVER_IP> /tmp/test_file.bin
```

**예상 속도**: ~10-100 MB/s (Gigabit LAN)

## 🎓 학습 포인트

### Phase 1: 파일 감시
- ✅ SQLite로 파일 메타데이터 관리
- ✅ notify로 실시간 파일 변경 감지
- ✅ blake3로 파일 해시 계산

### Phase 2: 기기 탐색
- ✅ UDP 브로드캐스트로 LAN 탐색
- ✅ HMAC-SHA256으로 메시지 인증
- ✅ 타임스탬프로 재생 공격 방지

### Phase 3: 암호화 전송
- ✅ TLS 1.3으로 통신 암호화
- ✅ 자기 서명 인증서 생성 및 관리
- ✅ Certificate Pinning으로 MITM 방지
- ✅ 1MB 청크로 대용량 파일 전송
- ✅ DB 기반 이어받기 (Resume)

## 🚀 다음 단계

1. **NAT 트래버설**: STUN/TURN 서버로 인터넷 넘어 연결
2. **멀티 피어**: 여러 기기에 동시 전송
3. **충돌 해결**: CRDT 또는 Vector Clock
4. **압축**: zstd로 전송 데이터 압축
5. **Flutter UI**: 실제 앱에 통합

---

## 📝 테스트 체크리스트

- [ ] Discovery 테스트: 두 프로세스가 서로 발견
- [ ] Transfer 테스트: 파일 전송 성공
- [ ] Certificate Pinning 테스트: 잘못된 fingerprint 거부
- [ ] Resume 테스트: 중단된 전송 재개
- [ ] 대용량 파일 테스트: 100MB+ 파일 전송
- [ ] 해시 검증: 전송 전후 파일 해시 일치

모든 테스트가 통과하면 Pebble의 핵심 기능이 정상 작동하는 것입니다! 🎉
