import { motion, AnimatePresence } from "motion/react";
import { X, Undo2 } from "lucide-react";
import { useEffect, useState } from "react";

interface UndoToastProps {
    isOpen: boolean;
    message: string;
    onUndo: () => void;
    onClose: () => void;
    duration?: number; // seconds
}

export function UndoToast({
    isOpen,
    message,
    onUndo,
    onClose,
    duration = 5,
}: UndoToastProps) {
    const [countdown, setCountdown] = useState(duration);

    useEffect(() => {
        if (!isOpen) {
            setCountdown(duration);
            return;
        }

        const interval = setInterval(() => {
            setCountdown((prev) => {
                if (prev <= 1) {
                    clearInterval(interval);
                    onClose();
                    return 0;
                }
                return prev - 1;
            });
        }, 1000);

        return () => clearInterval(interval);
    }, [isOpen, duration, onClose]);

    if (!isOpen) return null;

    const handleUndo = () => {
        onUndo();
        onClose();
    };

    return (
        <AnimatePresence>
            <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: 20 }}
                className="fixed bottom-4 left-4 right-4 mx-auto max-w-md bg-white dark:bg-gray-800 rounded-2xl shadow-2xl border border-border/50 p-4 z-50"
            >
                <div className="flex items-center gap-3">
                    {/* Message */}
                    <div className="flex-1 min-w-0">
                        <p className="text-sm font-medium">{message}</p>
                        <div className="mt-2 h-1 bg-muted/50 rounded-full overflow-hidden">
                            <motion.div
                                initial={{ width: "100%" }}
                                animate={{ width: "0%" }}
                                transition={{ duration: duration, ease: "linear" }}
                                className="h-full bg-gradient-to-r from-blue-600 to-cyan-600"
                            />
                        </div>
                        <p className="text-xs text-muted-foreground mt-1">
                            {countdown}초 후 자동으로 전송됩니다
                        </p>
                    </div>

                    {/* Actions */}
                    <div className="flex items-center gap-2 flex-shrink-0">
                        <button
                            onClick={handleUndo}
                            className="px-3 py-1.5 rounded-lg bg-gradient-to-r from-blue-600 to-cyan-600 text-white text-sm font-medium hover:shadow-lg transition-shadow flex items-center gap-1"
                        >
                            <Undo2 className="w-4 h-4" />
                            취소
                        </button>
                        <button
                            onClick={onClose}
                            className="p-1.5 hover:bg-muted/50 rounded-lg transition-colors"
                        >
                            <X className="w-4 h-4" />
                        </button>
                    </div>
                </div>
            </motion.div>
        </AnimatePresence>
    );
}
