import { X, Shield, Loader2 } from "lucide-react";
import { useP2pContext } from "../contexts/P2pContext";

export default function PairingModal() {
    const { pairingRequest, sentPairingPin, respondPairing } = useP2pContext();

    // 수신 모달 핸들러
    const handleApprove = () => {
        if (pairingRequest) {
            respondPairing(pairingRequest.peerId, true);
        }
    };

    const handleReject = () => {
        if (pairingRequest) {
            respondPairing(pairingRequest.peerId, false);
        }
    };

    // 수신된 페어링 요청 모달 (상대방이 나에게 요청)
    if (pairingRequest) {
        return (
            <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4">
                <div className="bg-white dark:bg-gray-800 rounded-2xl shadow-2xl max-w-md w-full p-6 relative animate-in fade-in zoom-in duration-200">
                    <button
                        onClick={handleReject}
                        className="absolute top-4 right-4 p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                    >
                        <X className="w-5 h-5" />
                    </button>

                    <div className="flex flex-col items-center text-center">
                        <div className="w-16 h-16 bg-blue-100 dark:bg-blue-900/30 rounded-full flex items-center justify-center mb-4">
                            <Shield className="w-8 h-8 text-blue-600 dark:text-blue-400" />
                        </div>

                        <h2 className="text-2xl font-bold mb-2">페어링 요청</h2>
                        <p className="text-gray-600 dark:text-gray-400 mb-6">
                            <span className="font-semibold text-gray-900 dark:text-white">
                                {pairingRequest.deviceName}
                            </span>
                            에서 연결을 요청했습니다
                        </p>

                        <div className="bg-gradient-to-br from-blue-50 to-indigo-50 dark:from-blue-900/20 dark:to-indigo-900/20 rounded-xl p-6 mb-6 w-full">
                            <p className="text-sm text-gray-600 dark:text-gray-400 mb-2">
                                PIN 코드 확인
                            </p>
                            <div className="text-4xl font-mono font-bold tracking-widest text-blue-600 dark:text-blue-400">
                                {pairingRequest.pin}
                            </div>
                            <p className="text-xs text-gray-500 dark:text-gray-500 mt-2">
                                상대방 화면의 PIN과 일치하는지 확인하세요
                            </p>
                        </div>

                        <div className="flex gap-3 w-full">
                            <button
                                onClick={handleReject}
                                className="flex-1 px-4 py-3 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-xl font-medium hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
                            >
                                거절
                            </button>
                            <button
                                onClick={handleApprove}
                                className="flex-1 px-4 py-3 bg-blue-600 text-white rounded-xl font-medium hover:bg-blue-700 transition-colors shadow-lg shadow-blue-600/30"
                            >
                                승인
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        );
    }

    // 전송된 페어링 요청 모달 (내가 상대방에게 요청, 승인 대기 중)
    if (sentPairingPin) {
        return (
            <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4">
                <div className="bg-white dark:bg-gray-800 rounded-2xl shadow-2xl max-w-md w-full p-6 relative animate-in fade-in zoom-in duration-200">
                    <div className="flex flex-col items-center text-center">
                        <div className="w-16 h-16 bg-green-100 dark:bg-green-900/30 rounded-full flex items-center justify-center mb-4">
                            <Loader2 className="w-8 h-8 text-green-600 dark:text-green-400 animate-spin" />
                        </div>

                        <h2 className="text-2xl font-bold mb-2">페어링 대기 중</h2>
                        <p className="text-gray-600 dark:text-gray-400 mb-6">
                            상대방의 승인을 기다리는 중입니다
                        </p>

                        <div className="bg-gradient-to-br from-green-50 to-emerald-50 dark:from-green-900/20 dark:to-emerald-900/20 rounded-xl p-6 mb-4 w-full">
                            <p className="text-sm text-gray-600 dark:text-gray-400 mb-2">
                                PIN 코드
                            </p>
                            <div className="text-5xl font-mono font-bold tracking-widest text-green-600 dark:text-green-400">
                                {sentPairingPin.pin}
                            </div>
                            <p className="text-sm text-gray-500 dark:text-gray-500 mt-3">
                                상대방에게 이 PIN을 보여주세요
                            </p>
                        </div>

                        <p className="text-xs text-gray-400">
                            상대방이 PIN을 확인하고 승인하면 연결됩니다
                        </p>
                    </div>
                </div>
            </div>
        );
    }

    return null;
}
