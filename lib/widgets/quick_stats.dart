import 'package:flutter/material.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import '../theme/app_colors.dart';

class QuickStats extends StatelessWidget {
  const QuickStats({super.key});

  @override
  Widget build(BuildContext context) {
    final stats = [
      _StatData(
        icon: LucideIcons.hardDrive,
        label: '총 전송량',
        value: '42.8 GB',
        gradient: AppColors.purplePinkGradient,
      ),
      _StatData(
        icon: LucideIcons.zap,
        label: '평균 속도',
        value: '125 MB/s',
        gradient: AppColors.blueCyanGradient,
      ),
      _StatData(
        icon: LucideIcons.clock,
        label: '활성 시간',
        value: '3h 24m',
        gradient: AppColors.greenEmeraldGradient,
      ),
    ];

    return Padding(
      padding: const EdgeInsets.all(16),
      child: Row(
        children: stats.asMap().entries.map((entry) {
          return Expanded(
            child: Padding(
              padding: EdgeInsets.only(
                left: entry.key == 0 ? 0 : 6,
                right: entry.key == stats.length - 1 ? 0 : 6,
              ),
              child: _StatCard(stat: entry.value),
            ),
          );
        }).toList(),
      ),
    );
  }
}

class _StatCard extends StatelessWidget {
  final _StatData stat;

  const _StatCard({required this.stat});

  @override
  Widget build(BuildContext context) {
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
      child: Column(
        children: [
          Padding(
            padding: const EdgeInsets.all(12),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Container(
                  width: 32,
                  height: 32,
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(8),
                    gradient: LinearGradient(
                      colors: stat.gradient,
                      begin: Alignment.topLeft,
                      end: Alignment.bottomRight,
                    ),
                    boxShadow: [
                      BoxShadow(
                        color: stat.gradient[0].withOpacity(0.2),
                        blurRadius: 4,
                        offset: const Offset(0, 2),
                      ),
                    ],
                  ),
                  child: Icon(
                    stat.icon,
                    color: Colors.white,
                    size: 16,
                  ),
                ),
                const SizedBox(height: 8),
                Text(
                  stat.label,
                  style: const TextStyle(
                    fontSize: 10,
                    color: AppColors.mutedForeground,
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                const SizedBox(height: 2),
                Text(
                  stat.value,
                  style: const TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
          Container(
            height: 2,
            decoration: BoxDecoration(
              gradient: LinearGradient(
                colors: stat.gradient.map((c) => c.withOpacity(0.5)).toList(),
                begin: Alignment.centerLeft,
                end: Alignment.centerRight,
              ),
              borderRadius: const BorderRadius.only(
                bottomLeft: Radius.circular(16),
                bottomRight: Radius.circular(16),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _StatData {
  final IconData icon;
  final String label;
  final String value;
  final List<Color> gradient;

  _StatData({
    required this.icon,
    required this.label,
    required this.value,
    required this.gradient,
  });
}
