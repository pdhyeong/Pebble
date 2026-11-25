import type { ContractEvent, Direction } from "./contract";
import type { P2pAction } from "./p2pState";

export const COMPLETED_TRANSFER_VISIBLE_MS = 1000;

export function shortPeerId(peerId: string): string {
  return `${peerId.slice(0, 12)}...`;
}

function transferLabel(direction: Direction): string {
  return direction === "upload" ? "업로드" : "다운로드";
}

export function eventToActions(event: ContractEvent, at: number): P2pAction[] {
  const log = (message: string): P2pAction => ({ type: "log", message, at });

  switch (event.name) {
    case "peer-found": {
      const { peer_id, address } = event.payload;
      return [
        log(`🔍 피어 발견: ${shortPeerId(peer_id)}`),
        { type: "peers/discovered", peerId: peer_id, address, at },
      ];
    }
    case "peer-expired":
      return [log(`⌛ 피어 만료: ${shortPeerId(event.payload.peer_id)}`)];
    case "connection-success":
      return [log(`🔗 전송 계층 연결: ${shortPeerId(event.payload.peer_id)} (페어링 필요)`)];
    case "device-info-received": {
      const { peer_id, device_name } = event.payload;
      return [
        log(`📱 기기 이름 수신: ${device_name}`),
        { type: "peers/deviceName", peerId: peer_id, deviceName: device_name },
      ];
    }
    case "connection-closed": {
      const { peer_id } = event.payload;
      return [log(`❌ 연결 종료: ${shortPeerId(peer_id)}`), { type: "peers/disconnected", peerId: peer_id }];
    }
    case "listening-on":
      return [log(`📡 리스닝: ${event.payload.address}`)];
    case "peer-info":
      return [log(`ℹ️ 피어 정보: ${shortPeerId(event.payload.peer_id)} (${event.payload.agent_version})`)];
    case "ping-success":
      return [log(`🏓 Ping: ${shortPeerId(event.payload.peer_id)} (${event.payload.rtt_ms}ms)`)];
    case "dial-started":
      return [log(`📞 연결 시도 중: ${event.payload.address}`)];
    case "dial-failed":
      return [log(`❌ 연결 시도 실패: ${event.payload.address} (${event.payload.error})`)];
    case "remote-files":
      return [
        log(`📂 파일 목록 수신: ${event.payload.files.length}개 파일`),
        { type: "remoteFiles/received", data: event.payload },
      ];
    case "file-list-request-sent":
      return [log(`📤 파일 목록 요청 전송: ${shortPeerId(event.payload.peer_id)} (${event.payload.path})`)];
    case "file-list-error":
      return [log(`❌ 파일 목록 요청 실패: ${event.payload.error}`), { type: "remoteFiles/loadFailed" }];
    case "p2p-stopped":
      return [log(`🛑 ${event.payload.message}`), { type: "engine/stopped" }];
    case "shared-folder-changed":
      return [{ type: "settings/sharedFolder", path: event.payload.path }];
    case "remote-directory-changed":
    case "local-files-changed":
      return [];
    case "transfer-started": {
      const { transfer_id, direction, peer_id, file_name, total_size } = event.payload;
      return [
        log(`${direction === "upload" ? "📤" : "📥"} 파일 ${transferLabel(direction)} 시작: ${file_name}`),
        {
          type: "transfer/started",
          transferId: transfer_id,
          direction,
          peerId: peer_id,
          fileName: file_name,
          totalBytes: total_size,
        },
      ];
    }
    case "transfer-progress": {
      const { transfer_id, bytes_done, total_size, progress } = event.payload;
      return [
        { type: "transfer/progress", transferId: transfer_id, bytesDone: bytes_done, totalBytes: total_size, progress },
      ];
    }
    case "transfer-completed": {
      const { transfer_id, direction, file_name, saved_path, speed_bps } = event.payload;
      const target = direction === "download" ? (saved_path ?? file_name) : file_name;
      return [
        log(`✅ 파일 ${transferLabel(direction)} 완료: ${target}`),
        { type: "transfer/completed", transferId: transfer_id, speedBps: speed_bps },
      ];
    }
    case "transfer-failed": {
      const { transfer_id, direction, error } = event.payload;
      return [
        log(`❌ 파일 ${transferLabel(direction)} 실패: ${error}`),
        { type: "transfer/removed", transferId: transfer_id },
      ];
    }
    case "transfer-cancelled": {
      const { transfer_id, direction, file_name } = event.payload;
      return [
        log(`❌ 파일 ${transferLabel(direction)} 취소: ${file_name}`),
        { type: "transfer/removed", transferId: transfer_id },
      ];
    }
    case "pairing-request": {
      const { peer_id, device_name, pin } = event.payload;
      return [
        log(`🔐 페어링 요청: ${device_name} (PIN: ${pin})`),
        { type: "pairing/requested", request: { peerId: peer_id, deviceName: device_name, pin } },
      ];
    }
    case "pairing-approved": {
      const { peer_id, device_name, shared_folder_name } = event.payload;
      return [
        log(`✅ 페어링 승인: ${device_name}`),
        { type: "peers/paired", peerId: peer_id, deviceName: device_name, sharedFolderName: shared_folder_name, at },
      ];
    }
    case "pairing-rejected":
      return [log(`❌ 페어링 거절: ${event.payload.error || "거절됨"}`), { type: "pairing/sentCleared" }];
    case "pairing-sent": {
      const { peer_id, pin } = event.payload;
      return [log(`🔐 페어링 요청 전송 (PIN: ${pin})`), { type: "pairing/sent", pin: { peerId: peer_id, pin } }];
    }
  }
}
