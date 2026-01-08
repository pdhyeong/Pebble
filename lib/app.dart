import 'package:flutter/material.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:file_picker/file_picker.dart';
import 'widgets/header.dart';
import 'widgets/bottom_nav.dart';
import 'views/home_view.dart';
import 'views/files_view.dart';
import 'views/activity_view.dart';
import 'views/shared_folders_view.dart';
import 'theme/app_colors.dart';

class PebbleApp extends StatefulWidget {
  const PebbleApp({super.key});

  @override
  State<PebbleApp> createState() => _PebbleAppState();
}

class _PebbleAppState extends State<PebbleApp> {
  int _activeTab = 0;

  Widget _renderView() {
    switch (_activeTab) {
      case 0:
        return const HomeView();
      case 1:
        return const FilesView();
      case 2:
        return const ActivityView();
      case 3:
        return const SharedFoldersView();
      default:
        return const HomeView();
    }
  }

  void _showUploadOptions() {
    showModalBottomSheet(
      context: context,
      backgroundColor: Colors.transparent,
      builder: (context) => Container(
        decoration: const BoxDecoration(
          color: AppColors.background,
          borderRadius: BorderRadius.vertical(top: Radius.circular(24)),
        ),
        child: SafeArea(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const SizedBox(height: 12),
              Container(
                width: 40,
                height: 4,
                decoration: BoxDecoration(
                  color: AppColors.muted,
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
              const SizedBox(height: 24),
              const Padding(
                padding: EdgeInsets.symmetric(horizontal: 24),
                child: Text(
                  '파일 추가',
                  style: TextStyle(
                    fontSize: 20,
                    fontWeight: FontWeight.w500,
                  ),
                ),
              ),
              const SizedBox(height: 16),
              _buildUploadOption(
                icon: LucideIcons.fileText,
                title: '문서 파일',
                subtitle: 'PDF, Word, Excel 등',
                gradient: AppColors.blueCyanGradient,
                onTap: () async {
                  Navigator.pop(context);
                  final result = await FilePicker.platform.pickFiles(
                    type: FileType.custom,
                    allowedExtensions: ['pdf', 'doc', 'docx', 'xls', 'xlsx', 'txt'],
                    allowMultiple: true,
                  );
                  if (result != null && mounted) {
                    _showSuccessSnackBar('${result.files.length}개의 문서가 추가되었습니다');
                  }
                },
              ),
              _buildUploadOption(
                icon: LucideIcons.image,
                title: '이미지',
                subtitle: 'JPG, PNG, GIF 등',
                gradient: AppColors.pinkRoseGradient,
                onTap: () async {
                  Navigator.pop(context);
                  final result = await FilePicker.platform.pickFiles(
                    type: FileType.image,
                    allowMultiple: true,
                  );
                  if (result != null && mounted) {
                    _showSuccessSnackBar('${result.files.length}개의 이미지가 추가되었습니다');
                  }
                },
              ),
              _buildUploadOption(
                icon: LucideIcons.film,
                title: '동영상',
                subtitle: 'MP4, AVI, MOV 등',
                gradient: AppColors.purpleIndigoGradient,
                onTap: () async {
                  Navigator.pop(context);
                  final result = await FilePicker.platform.pickFiles(
                    type: FileType.video,
                    allowMultiple: true,
                  );
                  if (result != null && mounted) {
                    _showSuccessSnackBar('${result.files.length}개의 동영상이 추가되었습니다');
                  }
                },
              ),
              _buildUploadOption(
                icon: LucideIcons.music,
                title: '오디오',
                subtitle: 'MP3, WAV, FLAC 등',
                gradient: AppColors.greenEmeraldGradient,
                onTap: () async {
                  Navigator.pop(context);
                  final result = await FilePicker.platform.pickFiles(
                    type: FileType.audio,
                    allowMultiple: true,
                  );
                  if (result != null && mounted) {
                    _showSuccessSnackBar('${result.files.length}개의 오디오가 추가되었습니다');
                  }
                },
              ),
              _buildUploadOption(
                icon: LucideIcons.file,
                title: '모든 파일',
                subtitle: '모든 형식의 파일',
                gradient: const [Color(0xFF9CA3AF), Color(0xFF64748B)],
                onTap: () async {
                  Navigator.pop(context);
                  final result = await FilePicker.platform.pickFiles(
                    allowMultiple: true,
                  );
                  if (result != null && mounted) {
                    _showSuccessSnackBar('${result.files.length}개의 파일이 추가되었습니다');
                  }
                },
              ),
              const SizedBox(height: 24),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildUploadOption({
    required IconData icon,
    required String title,
    required String subtitle,
    required List<Color> gradient,
    required VoidCallback onTap,
  }) {
    return InkWell(
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 12),
        child: Row(
          children: [
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
              ),
              child: Icon(
                icon,
                color: Colors.white,
                size: 24,
              ),
            ),
            const SizedBox(width: 16),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    title,
                    style: const TextStyle(
                      fontSize: 16,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                  const SizedBox(height: 2),
                  Text(
                    subtitle,
                    style: const TextStyle(
                      fontSize: 12,
                      color: AppColors.mutedForeground,
                    ),
                  ),
                ],
              ),
            ),
            const Icon(
              LucideIcons.chevronRight,
              size: 20,
              color: AppColors.mutedForeground,
            ),
          ],
        ),
      ),
    );
  }

  void _showSuccessSnackBar(String message) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(message),
        backgroundColor: AppColors.primary,
        behavior: SnackBarBehavior.floating,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Column(
        children: [
          const Header(),
          Expanded(
            child: _renderView(),
          ),
        ],
      ),
      bottomNavigationBar: BottomNav(
        activeTab: _activeTab,
        onTabChange: (index) {
          setState(() {
            _activeTab = index;
          });
        },
      ),
      floatingActionButton: _activeTab == 1
          ? FloatingActionButton.extended(
              onPressed: _showUploadOptions,
              icon: const Icon(LucideIcons.plus),
              label: const Text('파일 추가'),
              backgroundColor: AppColors.primary,
              foregroundColor: Colors.white,
              elevation: 4,
            )
          : null,
    );
  }
}
