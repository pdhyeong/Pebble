import 'package:flutter/material.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import '../models/activity.dart';
import '../models/device.dart';
import '../theme/app_colors.dart';

class ActivityView extends StatefulWidget {
  const ActivityView({super.key});

  @override
  State<ActivityView> createState() => _ActivityViewState();
}

class _ActivityViewState extends State<ActivityView> {
  String _filter = 'all';

  final List<Activity> _activities = [
    Activity(
      id: '1',
      type: ActivityType.download,
      fileName: '프로젝트_제안서.pdf',
      device: DeviceInfo(name: '내 아이폰', type: DeviceType.phone),
      timestamp: '5분 전',
      size: '2.4 MB',
      status: ActivityStatus.completed,
    ),
    Activity(
      id: '2',
      type: ActivityType.upload,
      fileName: 'vacation_photos.zip',
      device: DeviceInfo(name: '맥북 프로', type: DeviceType.desktop),
      timestamp: '15분 전',
      size: '156 MB',
      status: ActivityStatus.completed,
    ),
    Activity(
      id: '3',
      type: ActivityType.access,
      fileName: '문서 폴더',
      device: DeviceInfo(name: '아이패드', type: DeviceType.tablet),
      timestamp: '1시간 전',
      status: ActivityStatus.completed,
    ),
    Activity(
      id: '4',
      type: ActivityType.download,
      fileName: 'presentation.pptx',
      device: DeviceInfo(name: '내 아이폰', type: DeviceType.phone),
      timestamp: '2시간 전',
      size: '12.5 MB',
      status: ActivityStatus.completed,
    ),
    Activity(
      id: '5',
      type: ActivityType.share,
      fileName: 'design_assets.zip',
      device: DeviceInfo(name: '맥북 프로', type: DeviceType.desktop),
      timestamp: '3시간 전',
      size: '89 MB',
      status: ActivityStatus.completed,
    ),
    Activity(
      id: '6',
      type: ActivityType.delete,
      fileName: 'old_backup.zip',
      device: DeviceInfo(name: '맥북 프로', type: DeviceType.desktop),
      timestamp: '어제',
      size: '2.1 GB',
      status: ActivityStatus.completed,
    ),
    Activity(
      id: '7',
      type: ActivityType.download,
      fileName: 'video_tutorial.mp4',
      device: DeviceInfo(name: '내 아이폰', type: DeviceType.phone),
      timestamp: '어제',
      size: '340 MB',
      status: ActivityStatus.failed,
    ),
    Activity(
      id: '8',
      type: ActivityType.access,
      fileName: '사진 폴더',
      device: DeviceInfo(name: '아이패드', type: DeviceType.tablet),
      timestamp: '2일 전',
      status: ActivityStatus.completed,
    ),
  ];

  IconData _getActivityIcon(ActivityType type) {
    switch (type) {
      case ActivityType.download:
        return LucideIcons.download;
      case ActivityType.upload:
        return LucideIcons.upload;
      case ActivityType.delete:
        return LucideIcons.trash2;
      case ActivityType.access:
        return LucideIcons.folderOpen;
      case ActivityType.share:
        return LucideIcons.share2;
    }
  }

  List<Color> _getActivityGradient(ActivityType type) {
    switch (type) {
      case ActivityType.download:
        return AppColors.blueCyanGradient;
      case ActivityType.upload:
        return AppColors.greenEmeraldGradient;
      case ActivityType.delete:
        return AppColors.redPinkGradient;
      case ActivityType.access:
        return AppColors.purpleIndigoGradient;
      case ActivityType.share:
        return AppColors.orangeYellowGradient;
    }
  }

  String _getActivityLabel(ActivityType type) {
    switch (type) {
      case ActivityType.download:
        return '다운로드';
      case ActivityType.upload:
        return '업로드';
      case ActivityType.delete:
        return '삭제';
      case ActivityType.access:
        return '접근';
      case ActivityType.share:
        return '공유';
    }
  }

  IconData _getDeviceIcon(DeviceType type) {
    switch (type) {
      case DeviceType.phone:
        return LucideIcons.smartphone;
      case DeviceType.desktop:
        return LucideIcons.monitor;
      case DeviceType.tablet:
        return LucideIcons.tablet;
    }
  }

  List<Activity> get _filteredActivities {
    if (_filter == 'all') return _activities;
    return _activities.where((activity) {
      return activity.type.toString().split('.').last == _filter;
    }).toList();
  }

  @override
  Widget build(BuildContext context) {
    final filters = [
      {'id': 'all', 'label': '전체'},
      {'id': 'download', 'label': '다운로드'},
      {'id': 'upload', 'label': '업로드'},
      {'id': 'access', 'label': '접근'},
    ];

    return Column(
      children: [
        // Filter Tabs
        Container(
          decoration: BoxDecoration(
            color: AppColors.background.withOpacity(0.95),
            border: const Border(
              bottom: BorderSide(color: AppColors.border, width: 0.5),
            ),
          ),
          padding: const EdgeInsets.all(16),
          child: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: filters.map((filter) {
                final isActive = _filter == filter['id'];
                return Padding(
                  padding: const EdgeInsets.only(right: 8),
                  child: GestureDetector(
                    onTap: () {
                      setState(() {
                        _filter = filter['id']!;
                      });
                    },
                    child: AnimatedContainer(
                      duration: const Duration(milliseconds: 200),
                      padding: const EdgeInsets.symmetric(
                        horizontal: 16,
                        vertical: 8,
                      ),
                      decoration: BoxDecoration(
                        borderRadius: BorderRadius.circular(12),
                        gradient: isActive
                            ? const LinearGradient(
                                colors: [
                                  Color(0x1A7C6BFF),
                                  Color(0x1AFF6BAF),
                                ],
                                begin: Alignment.topLeft,
                                end: Alignment.bottomRight,
                              )
                            : null,
                        border: isActive
                            ? Border.all(
                                color: AppColors.primary.withOpacity(0.2),
                                width: 1,
                              )
                            : null,
                      ),
                      child: Text(
                        filter['label']!,
                        style: TextStyle(
                          fontSize: 14,
                          fontWeight: isActive ? FontWeight.w500 : FontWeight.w400,
                          color: isActive
                              ? AppColors.primary
                              : AppColors.mutedForeground,
                        ),
                      ),
                    ),
                  ),
                );
              }).toList(),
            ),
          ),
        ),

        // Activity Stats
        Container(
          decoration: BoxDecoration(
            gradient: LinearGradient(
              colors: [
                AppColors.primary.withOpacity(0.05),
                AppColors.chart2.withOpacity(0.05),
              ],
              begin: Alignment.topLeft,
              end: Alignment.bottomRight,
            ),
          ),
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              Expanded(
                child: Container(
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: AppColors.card.withOpacity(0.5),
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(
                      color: AppColors.border.withOpacity(0.5),
                      width: 0.5,
                    ),
                  ),
                  child: const Column(
                    children: [
                      Text(
                        '오늘',
                        style: TextStyle(
                          fontSize: 12,
                          color: AppColors.mutedForeground,
                        ),
                      ),
                      SizedBox(height: 4),
                      Text(
                        '12',
                        style: TextStyle(
                          fontSize: 18,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Container(
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: AppColors.card.withOpacity(0.5),
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(
                      color: AppColors.border.withOpacity(0.5),
                      width: 0.5,
                    ),
                  ),
                  child: const Column(
                    children: [
                      Text(
                        '이번 주',
                        style: TextStyle(
                          fontSize: 12,
                          color: AppColors.mutedForeground,
                        ),
                      ),
                      SizedBox(height: 4),
                      Text(
                        '48',
                        style: TextStyle(
                          fontSize: 18,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Container(
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: AppColors.card.withOpacity(0.5),
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(
                      color: AppColors.border.withOpacity(0.5),
                      width: 0.5,
                    ),
                  ),
                  child: const Column(
                    children: [
                      Text(
                        '전송량',
                        style: TextStyle(
                          fontSize: 12,
                          color: AppColors.mutedForeground,
                        ),
                      ),
                      SizedBox(height: 4),
                      Text(
                        '2.4GB',
                        style: TextStyle(
                          fontSize: 18,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),

        // Activity Timeline
        Expanded(
          child: _filteredActivities.isEmpty
              ? Center(
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Container(
                        width: 80,
                        height: 80,
                        decoration: BoxDecoration(
                          color: AppColors.muted.withOpacity(0.5),
                          borderRadius: BorderRadius.circular(24),
                        ),
                        child: const Icon(
                          LucideIcons.clock,
                          size: 40,
                          color: AppColors.mutedForeground,
                        ),
                      ),
                      const SizedBox(height: 16),
                      const Text(
                        '활동 내역이 없습니다',
                        style: TextStyle(
                          color: AppColors.mutedForeground,
                        ),
                      ),
                    ],
                  ),
                )
              : ListView.builder(
                  padding: const EdgeInsets.all(16),
                  itemCount: _filteredActivities.length,
                  itemBuilder: (context, index) {
                    final activity = _filteredActivities[index];
                    final gradient = _getActivityGradient(activity.type);
                    final hasNextItem = index < _filteredActivities.length - 1;

                    return Padding(
                      padding: const EdgeInsets.only(bottom: 12),
                      child: Stack(
                        children: [
                          // Timeline line
                          if (hasNextItem)
                            Positioned(
                              left: 20,
                              top: 56,
                              bottom: -12,
                              child: Container(
                                width: 1,
                                color: AppColors.border.withOpacity(0.5),
                              ),
                            ),

                          Row(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              // Activity Icon
                              Container(
                                width: 40,
                                height: 40,
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
                                  _getActivityIcon(activity.type),
                                  color: Colors.white,
                                  size: 20,
                                ),
                              ),
                              const SizedBox(width: 12),

                              // Activity Content
                              Expanded(
                                child: Container(
                                  decoration: BoxDecoration(
                                    color: AppColors.card,
                                    borderRadius: BorderRadius.circular(12),
                                    border: Border.all(
                                      color: AppColors.border,
                                      width: 1,
                                    ),
                                  ),
                                  padding: const EdgeInsets.all(12),
                                  child: Column(
                                    crossAxisAlignment: CrossAxisAlignment.start,
                                    children: [
                                      Row(
                                        children: [
                                          Container(
                                            padding: const EdgeInsets.symmetric(
                                              horizontal: 8,
                                              vertical: 2,
                                            ),
                                            decoration: BoxDecoration(
                                              gradient: LinearGradient(
                                                colors: gradient,
                                                begin: Alignment.centerLeft,
                                                end: Alignment.centerRight,
                                              ),
                                              borderRadius: BorderRadius.circular(8),
                                            ),
                                            child: Text(
                                              _getActivityLabel(activity.type),
                                              style: const TextStyle(
                                                fontSize: 12,
                                                color: Colors.white,
                                              ),
                                            ),
                                          ),
                                          if (activity.status == ActivityStatus.failed) ...[
                                            const SizedBox(width: 8),
                                            Container(
                                              padding: const EdgeInsets.symmetric(
                                                horizontal: 8,
                                                vertical: 2,
                                              ),
                                              decoration: BoxDecoration(
                                                color: AppColors.destructive.withOpacity(0.1),
                                                borderRadius: BorderRadius.circular(8),
                                              ),
                                              child: const Text(
                                                '실패',
                                                style: TextStyle(
                                                  fontSize: 12,
                                                  color: AppColors.destructive,
                                                ),
                                              ),
                                            ),
                                          ],
                                        ],
                                      ),
                                      const SizedBox(height: 8),
                                      Text(
                                        activity.fileName,
                                        style: const TextStyle(
                                          fontSize: 14,
                                          fontWeight: FontWeight.w500,
                                        ),
                                        overflow: TextOverflow.ellipsis,
                                      ),
                                      const SizedBox(height: 8),
                                      Row(
                                        children: [
                                          Icon(
                                            _getDeviceIcon(activity.device.type),
                                            size: 12,
                                            color: AppColors.mutedForeground,
                                          ),
                                          const SizedBox(width: 4),
                                          Text(
                                            activity.device.name,
                                            style: const TextStyle(
                                              fontSize: 12,
                                              color: AppColors.mutedForeground,
                                            ),
                                          ),
                                          const Text(
                                            ' • ',
                                            style: TextStyle(
                                              fontSize: 12,
                                              color: AppColors.mutedForeground,
                                            ),
                                          ),
                                          const Icon(
                                            LucideIcons.clock,
                                            size: 12,
                                            color: AppColors.mutedForeground,
                                          ),
                                          const SizedBox(width: 4),
                                          Text(
                                            activity.timestamp,
                                            style: const TextStyle(
                                              fontSize: 12,
                                              color: AppColors.mutedForeground,
                                            ),
                                          ),
                                          if (activity.size != null) ...[
                                            const Text(
                                              ' • ',
                                              style: TextStyle(
                                                fontSize: 12,
                                                color: AppColors.mutedForeground,
                                              ),
                                            ),
                                            Text(
                                              activity.size!,
                                              style: const TextStyle(
                                                fontSize: 12,
                                                color: AppColors.mutedForeground,
                                              ),
                                            ),
                                          ],
                                        ],
                                      ),
                                    ],
                                  ),
                                ),
                              ),
                            ],
                          ),
                        ],
                      ),
                    );
                  },
                ),
        ),
      ],
    );
  }
}
