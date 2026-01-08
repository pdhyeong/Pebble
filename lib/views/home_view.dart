import 'package:flutter/material.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import '../models/device.dart';
import '../widgets/quick_stats.dart';
import '../widgets/file_upload_zone.dart';
import '../widgets/device_card.dart';
import '../widgets/transfer_progress.dart';
import '../widgets/recent_files.dart';
import '../theme/app_colors.dart';
import 'device_connect_view.dart';

class HomeView extends StatelessWidget {
  const HomeView({super.key});

  @override
  Widget build(BuildContext context) {
    final devices = [
      Device(
        name: '내 아이폰',
        type: DeviceType.phone,
        status: DeviceStatus.connected,
        lastSync: '방금 전',
      ),
      Device(
        name: '맥북 프로',
        type: DeviceType.desktop,
        status: DeviceStatus.connected,
        lastSync: '5분 전',
      ),
      Device(
        name: '아이패드',
        type: DeviceType.tablet,
        status: DeviceStatus.offline,
        lastSync: '2시간 전',
      ),
    ];

    return SingleChildScrollView(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Hero Section
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 24, 16, 8),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: const [
                Text(
                  '안녕하세요! 👋',
                  style: TextStyle(
                    fontSize: 32,
                    fontWeight: FontWeight.w500,
                  ),
                ),
                SizedBox(height: 8),
                Text(
                  '내 PC의 저장공간을 어디서나 접근하세요',
                  style: TextStyle(
                    fontSize: 16,
                    color: AppColors.mutedForeground,
                  ),
                ),
              ],
            ),
          ),

          // File Upload Zone
          const FileUploadZone(),

          // Quick Stats
          const QuickStats(),

          // Active Transfers
          Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text(
                  '진행 중인 전송',
                  style: TextStyle(
                    fontSize: 20,
                    fontWeight: FontWeight.w500,
                  ),
                ),
                const SizedBox(height: 12),
                const TransferProgress(
                  fileName: 'design_assets.zip',
                  progress: 67,
                  status: TransferStatus.transferring,
                  size: '156 MB',
                  speed: '24.5 MB/s',
                ),
                const SizedBox(height: 12),
                const TransferProgress(
                  fileName: 'report_final.pdf',
                  progress: 100,
                  status: TransferStatus.completed,
                  size: '3.2 MB',
                ),
              ],
            ),
          ),

          // Connected Devices
          Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Column(
                  children: [
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        const Text(
                          '연결된 기기',
                          style: TextStyle(
                            fontSize: 20,
                            fontWeight: FontWeight.w500,
                          ),
                        ),
                        Row(
                          children: [
                            Container(
                              width: 8,
                              height: 8,
                              decoration: const BoxDecoration(
                                shape: BoxShape.circle,
                                color: Color(0xFF10B981),
                              ),
                            ),
                            const SizedBox(width: 8),
                            Text(
                              '${devices.where((d) => d.status == DeviceStatus.connected).length}개 활성',
                              style: const TextStyle(
                                fontSize: 14,
                                color: AppColors.mutedForeground,
                              ),
                            ),
                          ],
                        ),
                      ],
                    ),
                    const SizedBox(height: 12),
                    SizedBox(
                      width: double.infinity,
                      child: OutlinedButton.icon(
                        onPressed: () {
                          Navigator.push(
                            context,
                            MaterialPageRoute(
                              builder: (context) => const DeviceConnectView(),
                            ),
                          );
                        },
                        icon: const Icon(LucideIcons.qrCode, size: 18),
                        label: const Text('새 기기 연결'),
                        style: OutlinedButton.styleFrom(
                          foregroundColor: AppColors.primary,
                          side: const BorderSide(color: AppColors.primary),
                          padding: const EdgeInsets.symmetric(vertical: 12),
                          shape: RoundedRectangleBorder(
                            borderRadius: BorderRadius.circular(12),
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                ...devices.asMap().entries.map((entry) => Padding(
                      padding: const EdgeInsets.only(bottom: 12),
                      child: DeviceCard(
                        device: entry.value,
                        index: entry.key,
                      ),
                    )),
              ],
            ),
          ),

          // Recent Files
          const RecentFiles(),

          // Info Card
          Padding(
            padding: const EdgeInsets.all(16),
            child: Container(
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(24),
                gradient: const LinearGradient(
                  colors: [
                    AppColors.primary,
                    AppColors.chart2,
                    AppColors.chart3,
                  ],
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                ),
                boxShadow: [
                  BoxShadow(
                    color: AppColors.primary.withOpacity(0.25),
                    blurRadius: 20,
                    offset: const Offset(0, 4),
                  ),
                ],
              ),
              child: Stack(
                children: [
                  // Background decorations
                  Positioned(
                    top: 0,
                    right: 0,
                    child: Container(
                      width: 128,
                      height: 128,
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        color: Colors.white.withOpacity(0.1),
                      ),
                      transform: Matrix4.translationValues(64, -64, 0),
                    ),
                  ),
                  Positioned(
                    bottom: 0,
                    left: 0,
                    child: Container(
                      width: 96,
                      height: 96,
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        color: Colors.white.withOpacity(0.1),
                      ),
                      transform: Matrix4.translationValues(-48, 48, 0),
                    ),
                  ),

                  // Content
                  Padding(
                    padding: const EdgeInsets.all(24),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        const Text(
                          '💡 Pebble 사용 팁',
                          style: TextStyle(
                            fontSize: 20,
                            fontWeight: FontWeight.w500,
                            color: Colors.white,
                          ),
                        ),
                        const SizedBox(height: 8),
                        const Text(
                          '메인 PC를 항상 켜두면 언제 어디서나\n모든 파일에 접근할 수 있어요',
                          style: TextStyle(
                            fontSize: 14,
                            color: Colors.white70,
                          ),
                        ),
                        const SizedBox(height: 16),
                        ElevatedButton(
                          onPressed: () {},
                          style: ElevatedButton.styleFrom(
                            backgroundColor: Colors.white,
                            foregroundColor: AppColors.primary,
                            padding: const EdgeInsets.symmetric(
                              horizontal: 24,
                              vertical: 12,
                            ),
                            shape: RoundedRectangleBorder(
                              borderRadius: BorderRadius.circular(12),
                            ),
                            elevation: 4,
                          ),
                          child: const Text(
                            '설정 가이드',
                            style: TextStyle(
                              fontSize: 16,
                              fontWeight: FontWeight.w500,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ),

          const SizedBox(height: 24),
        ],
      ),
    );
  }
}
