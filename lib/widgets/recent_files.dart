import 'package:flutter/material.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import '../theme/app_colors.dart';

class RecentFiles extends StatelessWidget {
  const RecentFiles({super.key});

  @override
  Widget build(BuildContext context) {
    final files = [
      _RecentFileData(
        name: '프로젝트_제안서.pdf',
        type: _FileType.document,
        size: '2.4 MB',
        time: '5분 전',
      ),
      _RecentFileData(
        name: 'vacation_photo.jpg',
        type: _FileType.image,
        size: '4.8 MB',
        time: '1시간 전',
      ),
      _RecentFileData(
        name: 'demo_video.mp4',
        type: _FileType.video,
        size: '128 MB',
        time: '2시간 전',
      ),
      _RecentFileData(
        name: 'presentation.pptx',
        type: _FileType.document,
        size: '12.5 MB',
        time: '어제',
      ),
    ];

    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              const Text(
                '최근 파일',
                style: TextStyle(
                  fontSize: 20,
                  fontWeight: FontWeight.w500,
                ),
              ),
              TextButton(
                onPressed: () {},
                child: const Text(
                  '전체보기',
                  style: TextStyle(
                    fontSize: 14,
                    color: AppColors.primary,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          ...files.map((file) => Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: _RecentFileItem(file: file),
              )),
        ],
      ),
    );
  }
}

class _RecentFileItem extends StatelessWidget {
  final _RecentFileData file;

  const _RecentFileItem({required this.file});

  IconData _getIcon() {
    switch (file.type) {
      case _FileType.document:
        return LucideIcons.fileText;
      case _FileType.image:
        return LucideIcons.image;
      case _FileType.video:
        return LucideIcons.film;
      case _FileType.audio:
        return LucideIcons.music;
    }
  }

  List<Color> _getGradient() {
    switch (file.type) {
      case _FileType.document:
        return AppColors.blueCyanGradient;
      case _FileType.image:
        return AppColors.pinkRoseGradient;
      case _FileType.video:
        return AppColors.purpleIndigoGradient;
      case _FileType.audio:
        return AppColors.greenEmeraldGradient;
    }
  }

  @override
  Widget build(BuildContext context) {
    final gradient = _getGradient();

    return Container(
      decoration: BoxDecoration(
        color: AppColors.card,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: AppColors.border, width: 1),
      ),
      padding: const EdgeInsets.all(12),
      child: Row(
        children: [
          // File icon
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
                  color: gradient[0].withOpacity(0.2),
                  blurRadius: 4,
                  offset: const Offset(0, 2),
                ),
              ],
            ),
            child: Icon(
              _getIcon(),
              color: Colors.white,
              size: 20,
            ),
          ),
          const SizedBox(width: 12),

          // File info
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  file.name,
                  style: const TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w500,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
                const SizedBox(height: 2),
                Row(
                  children: [
                    Text(
                      file.size,
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
                    Text(
                      file.time,
                      style: const TextStyle(
                        fontSize: 12,
                        color: AppColors.mutedForeground,
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),

          // More button
          IconButton(
            onPressed: () {},
            icon: const Icon(
              Icons.more_vert,
              size: 16,
              color: AppColors.mutedForeground,
            ),
            style: IconButton.styleFrom(
              backgroundColor: AppColors.muted.withOpacity(0.5),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(8),
              ),
              padding: const EdgeInsets.all(8),
            ),
          ),
        ],
      ),
    );
  }
}

class _RecentFileData {
  final String name;
  final _FileType type;
  final String size;
  final String time;

  _RecentFileData({
    required this.name,
    required this.type,
    required this.size,
    required this.time,
  });
}

enum _FileType {
  document,
  image,
  video,
  audio,
}
