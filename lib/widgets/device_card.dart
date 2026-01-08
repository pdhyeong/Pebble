import 'package:flutter/material.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import '../models/device.dart';
import '../theme/app_colors.dart';

class DeviceCard extends StatelessWidget {
  final Device device;
  final int index;

  const DeviceCard({
    super.key,
    required this.device,
    this.index = 0,
  });

  IconData _getDeviceIcon() {
    switch (device.type) {
      case DeviceType.phone:
        return LucideIcons.smartphone;
      case DeviceType.desktop:
        return LucideIcons.monitor;
      case DeviceType.tablet:
        return LucideIcons.tablet;
    }
  }

  List<Color> _getGradientColors() {
    final gradients = [
      AppColors.purplePinkGradient,
      AppColors.blueCyanGradient,
      AppColors.greenEmeraldGradient,
      AppColors.orangeYellowGradient,
    ];
    return gradients[index % gradients.length];
  }

  @override
  Widget build(BuildContext context) {
    final gradient = _getGradientColors();
    final isConnected = device.status == DeviceStatus.connected;

    return Container(
      decoration: BoxDecoration(
        color: AppColors.card,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: AppColors.border, width: 1),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withOpacity(0.02),
            blurRadius: 4,
            offset: const Offset(0, 2),
          ),
        ],
      ),
      child: Column(
        children: [
          Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                // Device icon
                Container(
                  width: 48,
                  height: 48,
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(12),
                    gradient: LinearGradient(
                      colors: gradient,
                      begin: Alignment.topLeft,
                      end: Alignment.bottomRight,
                    ),
                    boxShadow: [
                      BoxShadow(
                        color: gradient[0].withOpacity(0.3),
                        blurRadius: 8,
                        offset: const Offset(0, 2),
                      ),
                    ],
                  ),
                  child: Icon(
                    _getDeviceIcon(),
                    color: Colors.white,
                    size: 24,
                  ),
                ),
                const SizedBox(width: 12),

                // Device info
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Expanded(
                            child: Text(
                              device.name,
                              style: const TextStyle(
                                fontSize: 16,
                                fontWeight: FontWeight.w500,
                              ),
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                          const SizedBox(width: 8),
                          if (isConnected) ...[
                            Container(
                              width: 8,
                              height: 8,
                              decoration: const BoxDecoration(
                                shape: BoxShape.circle,
                                color: Color(0xFF10B981),
                              ),
                            ),
                            const SizedBox(width: 4),
                            const Icon(
                              LucideIcons.wifi,
                              size: 12,
                              color: Color(0xFF10B981),
                            ),
                          ] else
                            const Icon(
                              LucideIcons.wifiOff,
                              size: 12,
                              color: AppColors.mutedForeground,
                            ),
                        ],
                      ),
                      const SizedBox(height: 4),
                      Text(
                        isConnected ? '연결됨' : '오프라인',
                        style: const TextStyle(
                          fontSize: 12,
                          color: AppColors.mutedForeground,
                        ),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        '마지막 동기화: ${device.lastSync}',
                        style: const TextStyle(
                          fontSize: 12,
                          color: AppColors.mutedForeground,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),

          // Bottom gradient line
          Container(
            height: 4,
            decoration: BoxDecoration(
              gradient: LinearGradient(
                colors: gradient,
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
