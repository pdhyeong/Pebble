import type { AppError, ErrorCode } from "../../core/contract";

const ERROR_LABELS: Record<ErrorCode, string> = {
  P2P_NOT_RUNNING: "P2P 엔진이 실행 중이 아닙니다",
  P2P_ALREADY_RUNNING: "P2P 엔진이 이미 실행 중입니다",
  INVALID_PEER_ID: "잘못된 기기 ID입니다",
  INVALID_PATH: "잘못된 경로입니다",
  ACCESS_DENIED: "접근할 수 없는 경로입니다",
  NOT_FOUND: "찾을 수 없습니다",
  IO: "파일 입출력 오류",
  NETWORK: "네트워크 오류",
  ENGINE_UNAVAILABLE: "P2P 엔진을 사용할 수 없습니다",
  INTERNAL: "내부 오류",
};

function isAppError(e: unknown): e is AppError {
  return (
    typeof e === "object" &&
    e !== null &&
    typeof (e as AppError).code === "string" &&
    typeof (e as AppError).message === "string"
  );
}

export function describeError(e: unknown): string {
  if (isAppError(e)) {
    const label = ERROR_LABELS[e.code] ?? e.code;
    return e.message ? `${label}: ${e.message}` : label;
  }
  if (e instanceof Error) return e.message;
  return String(e);
}
