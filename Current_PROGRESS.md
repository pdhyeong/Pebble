## 완료된 기능

### 1. P2P 네트워킹 기반
- **mDNS 피어 발견**: 로컬 네트워크에서 자동으로 피어 발견
- **연결 관리**: 피어 연결/연결 해제 처리
- **Ping/Pong**: 연결 상태 확인

### 2. 파일 목록 교환 프로토콜
- **FileListRequest/Response**: 상대방 공유 폴더의 파일 목록 요청 및 수신
- **FileListCodec**: libp2p request-response 프로토콜 구현
- 파일 이름, 경로, 크기, 디렉토리 여부 정보 교환

### 3. 파일 전송 프로토콜
- **FileTransferRequest/Response**: 파일 다운로드 요청 및 수신
- **FileTransferCodec**: 파일 데이터 전송 프로토콜 구현


### 현재 진행상황 01-16

현재 = 파일 전송 및 다운로드 구현

하지만 문제점을 발견 단지 전송관련된 UI로 표현도 불가능하고 대용량 파일전송에 문제가 생길듯함

pub struct FileUploadRequest {
      pub data: Vec<u8>,  // 파일 전체를 메모리에 올림 -> 바로 메모리 로드 하여 대용량 파일시 문제가 될것같음
  }

  현재 방식:
  1. 메모리 문제: 파일 전체를 Vec<u8>로 메모리에 로드 → 1GB 파일 = 1GB 메모리 사용
  2. JSON 직렬화: serde_json으로 직렬화 → 바이너리를 base64로 변환하면 ~33% 크기 증가
  3. 전송 방식: libp2p 기본 설정인 TCP + noise + yamux 사용중
  4. 청크 없음: 한 번에 전송 → 실패 시 처음부터 다시
  5. 진행률 표시 불가

  개선 방안

  1. QUIC 지원 (libp2p에서 지원)

  # Cargo.toml
  libp2p = { version = "0.52", features = ["quic"] }

  QUIC의 장점:
  - UDP 기반으로 TCP보다 빠른 연결 설정 (0-RTT)
  - 내장 TLS 암호화
  - 멀티플렉싱 지원
  - 패킷 손실 복구 효율적

  2. 청크 기반 스트리밍 전송

  // 개선된 프로토콜 설계
  pub struct FileTransferStart {
      pub file_name: String,
      pub file_size: u64,
      pub chunk_size: u32,
      pub checksum: String,  // SHA256
  }

  pub struct FileChunk {
      pub transfer_id: String,
      pub chunk_index: u32,
      pub data: Vec<u8>,     // 1MB 청크
  }

  pub struct FileTransferComplete {
      pub transfer_id: String,
      pub total_chunks: u32,
  }