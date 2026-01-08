import 'package:flutter/material.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import '../theme/app_colors.dart';

class TransferProgress extends StatelessWidget {
  final String fileName;
  final int progress;
  final TransferStatus status;
  final String size;
  final String? speed;

  const TransferProgress({
    super.key,
    required this.fileName,
    required this.progress,
    required this.status,
    required this.size,
    this.speed,
  });

  Widget _getStatusIcon() {
    switch (status) {
      case TransferStatus.transferring:
        return const SizedBox(
          width: 16,
          height: 16,
          child: CircularProgressIndicator(
            strokeWidth: 2,
            valueColor: AlwaysStoppedAnimation(AppColors.primary),
          ),
        );
      case TransferStatus.completed:
        return const Icon(
          Icons.check_circle,
          size: 16,
          color: Color(0xFF10B981),
        );
      case TransferStatus.failed:
        return const Icon(
          LucideIcons.x,
          size: 16,
          color: AppColors.destructive,
        );
    }
  }

  List<Color> _getStatusGradient() {
    switch (status) {
      case TransferStatus.transferring:
        return AppColors.primaryGradient;
      case TransferStatus.completed:
        return AppColors.greenEmeraldGradient;
      case TransferStatus.failed:
        return AppColors.redPinkGradient;
    }
  }

  @override
  Widget build(BuildContext context) {
    final gradient = _getStatusGradient();

    return Container(
      decoration: BoxDecoration(
        color: AppColors.card,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: AppColors.border, width: 1),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withOpacity(0.02),
            blurRadius: 4,
            offset: const Offset(0, 1),
          ),
        ],
      ),
      padding: const EdgeInsets.all(16),
      child: Column(
        children: [
          Row(
            children: [
              // File icon
              Container(
                width: 40,
                height: 40,
                decoration: BoxDecoration(
                  borderRadius: BorderRadius.circular(12),
                  gradient: LinearGradient(
                    colors: [
                      AppColors.primary.withOpacity(0.1),
                      AppColors.chart2.withOpacity(0.1),
                    ],
                    begin: Alignment.topLeft,
                    end: Alignment.bottomRight,
                  ),
                ),
                child: const Icon(
                  LucideIcons.fileText,
                  size: 20,
                  color: AppColors.primary,
                ),
              ),
              const SizedBox(width: 12),

              // File info
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Expanded(
                          child: Text(
                            fileName,
                            style: const TextStyle(
                              fontSize: 14,
                              fontWeight: FontWeight.w500,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                        const SizedBox(width: 8),
                        _getStatusIcon(),
                      ],
                    ),
                    const SizedBox(height: 4),
                    Row(
                      children: [
                        Text(
                          size,
                          style: const TextStyle(
                            fontSize: 12,
                            color: AppColors.mutedForeground,
                          ),
                        ),
                        if (speed != null && status == TransferStatus.transferring) ...[
                          const Text(
                            ' • ',
                            style: TextStyle(
                              fontSize: 12,
                              color: AppColors.mutedForeground,
                            ),
                          ),
                          Text(
                            speed!,
                            style: const TextStyle(
                              fontSize: 12,
                              color: AppColors.mutedForeground,
                            ),
                          ),
                        ],
                        if (status == TransferStatus.completed) ...[
                          const Text(
                            ' • ',
                            style: TextStyle(
                              fontSize: 12,
                              color: AppColors.mutedForeground,
                            ),
                          ),
                          const Text(
                            '완료',
                            style: TextStyle(
                              fontSize: 12,
                              color: Color(0xFF10B981),
                            ),
                          ),
                        ],
                      ],
                    ),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),

          // Progress bar
          ClipRRect(
            borderRadius: BorderRadius.circular(8),
            child: SizedBox(
              height: 8,
              child: Stack(
                children: [
                  Container(
                    decoration: const BoxDecoration(
                      color: AppColors.muted,
                    ),
                  ),
                  FractionallySizedBox(
                    widthFactor: progress / 100,
                    child: Container(
                      decoration: BoxDecoration(
                        gradient: LinearGradient(
                          colors: gradient,
                          begin: Alignment.centerLeft,
                          end: Alignment.centerRight,
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),

          if (status == TransferStatus.transferring) ...[
            const SizedBox(height: 8),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(
                  '$progress%',
                  style: const TextStyle(
                    fontSize: 12,
                    color: AppColors.mutedForeground,
                  ),
                ),
                const Text(
                  '전송 중...',
                  style: TextStyle(
                    fontSize: 12,
                    color: AppColors.mutedForeground,
                  ),
                ),
              ],
            ),
          ],
        ],
      ),
    );
  }
}

enum TransferStatus {
  transferring,
  completed,
  failed,
}
