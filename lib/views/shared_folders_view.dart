import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:provider/provider.dart';
import 'package:file_picker/file_picker.dart';
import 'package:desktop_drop/desktop_drop.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import '../providers/shared_folder_provider.dart';
import '../models/shared_folder.dart';
import '../theme/app_colors.dart';

class SharedFoldersView extends StatefulWidget {
  const SharedFoldersView({super.key});

  @override
  State<SharedFoldersView> createState() => _SharedFoldersViewState();
}

class _SharedFoldersViewState extends State<SharedFoldersView> {
  bool _isDragging = false;

  Future<void> _pickFolder() async {
    String? selectedDirectory = await FilePicker.platform.getDirectoryPath();

    if (selectedDirectory != null && mounted) {
      _addFolder(selectedDirectory);
    }
  }

  void _addFolder(String folderPath) {
    final provider = context.read<SharedFolderProvider>();
    final folderName = folderPath.split(Platform.pathSeparator).last;

    final newFolder = SharedFolder(
      id: DateTime.now().millisecondsSinceEpoch.toString(),
      name: folderName,
      path: folderPath,
      addedAt: DateTime.now(),
    );

    provider.addFolder(newFolder);

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text('$folderName 폴더가 추가되었습니다'),
        backgroundColor: AppColors.primary,
        behavior: SnackBarBehavior.floating,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Consumer<SharedFolderProvider>(
      builder: (context, provider, child) {
        return SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Header
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 24, 16, 8),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: const [
                    Text(
                      '공유 폴더 관리',
                      style: TextStyle(
                        fontSize: 32,
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    SizedBox(height: 8),
                    Text(
                      'P2P로 동기화할 폴더를 선택하세요',
                      style: TextStyle(
                        fontSize: 16,
                        color: AppColors.mutedForeground,
                      ),
                    ),
                  ],
                ),
              ),

              // Drag and Drop Zone (Desktop only)
              if (!kIsWeb && (Platform.isWindows || Platform.isMacOS || Platform.isLinux))
                Padding(
                  padding: const EdgeInsets.all(16),
                  child: DropTarget(
                    onDragEntered: (_) {
                      setState(() => _isDragging = true);
                    },
                    onDragExited: (_) {
                      setState(() => _isDragging = false);
                    },
                    onDragDone: (details) {
                      setState(() => _isDragging = false);
                      for (final file in details.files) {
                        if (FileSystemEntity.isDirectorySync(file.path)) {
                          _addFolder(file.path);
                        }
                      }
                    },
                    child: Container(
                      decoration: BoxDecoration(
                        borderRadius: BorderRadius.circular(24),
                        border: Border.all(
                          color: _isDragging
                              ? AppColors.primary
                              : AppColors.border,
                          width: 2,
                          strokeAlign: BorderSide.strokeAlignCenter,
                        ),
                        gradient: LinearGradient(
                          colors: _isDragging
                              ? [
                                  AppColors.primary.withOpacity(0.2),
                                  AppColors.chart2.withOpacity(0.2),
                                ]
                              : [
                                  const Color(0x0D7C6BFF),
                                  const Color(0x0DE0F2FE),
                                ],
                          begin: Alignment.topLeft,
                          end: Alignment.bottomRight,
                        ),
                      ),
                      child: Padding(
                        padding: const EdgeInsets.all(32),
                        child: Column(
                          children: [
                            Container(
                              width: 80,
                              height: 80,
                              decoration: BoxDecoration(
                                borderRadius: BorderRadius.circular(24),
                                gradient: LinearGradient(
                                  colors: [
                                    AppColors.primary.withOpacity(0.2),
                                    AppColors.chart2.withOpacity(0.2),
                                  ],
                                  begin: Alignment.topLeft,
                                  end: Alignment.bottomRight,
                                ),
                              ),
                              child: const Icon(
                                LucideIcons.folderPlus,
                                size: 40,
                                color: AppColors.primary,
                              ),
                            ),
                            const SizedBox(height: 16),
                            Text(
                              _isDragging
                                  ? '폴더를 여기에 놓으세요'
                                  : '폴더를 드래그하거나',
                              style: TextStyle(
                                fontSize: 20,
                                fontWeight: FontWeight.w500,
                                color: _isDragging
                                    ? AppColors.primary
                                    : AppColors.foreground,
                              ),
                            ),
                            const SizedBox(height: 4),
                            const Text(
                              '클릭해서 선택하세요',
                              style: TextStyle(
                                fontSize: 14,
                                color: AppColors.mutedForeground,
                              ),
                            ),
                            const SizedBox(height: 16),
                            ElevatedButton.icon(
                              onPressed: _pickFolder,
                              icon: const Icon(LucideIcons.folderOpen, size: 16),
                              label: const Text('폴더 선택'),
                              style: ElevatedButton.styleFrom(
                                backgroundColor: AppColors.primary,
                                foregroundColor: Colors.white,
                                padding: const EdgeInsets.symmetric(
                                  horizontal: 24,
                                  vertical: 12,
                                ),
                                shape: RoundedRectangleBorder(
                                  borderRadius: BorderRadius.circular(16),
                                ),
                                elevation: 4,
                                shadowColor: AppColors.primary.withOpacity(0.3),
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                )
              else
                // Mobile: Just show folder picker button
                Padding(
                  padding: const EdgeInsets.all(16),
                  child: Container(
                    width: double.infinity,
                    decoration: BoxDecoration(
                      borderRadius: BorderRadius.circular(24),
                      border: Border.all(
                        color: AppColors.border,
                        width: 2,
                      ),
                      gradient: const LinearGradient(
                        colors: [
                          Color(0x0D7C6BFF),
                          Color(0x0DE0F2FE),
                        ],
                        begin: Alignment.topLeft,
                        end: Alignment.bottomRight,
                      ),
                    ),
                    child: Padding(
                      padding: const EdgeInsets.all(32),
                      child: Column(
                        children: [
                          Container(
                            width: 80,
                            height: 80,
                            decoration: BoxDecoration(
                              borderRadius: BorderRadius.circular(24),
                              gradient: LinearGradient(
                                colors: [
                                  AppColors.primary.withOpacity(0.2),
                                  AppColors.chart2.withOpacity(0.2),
                                ],
                                begin: Alignment.topLeft,
                                end: Alignment.bottomRight,
                              ),
                            ),
                            child: const Icon(
                              LucideIcons.folderPlus,
                              size: 40,
                              color: AppColors.primary,
                            ),
                          ),
                          const SizedBox(height: 16),
                          const Text(
                            '공유할 폴더 추가',
                            style: TextStyle(
                              fontSize: 20,
                              fontWeight: FontWeight.w500,
                            ),
                          ),
                          const SizedBox(height: 4),
                          const Text(
                            '버튼을 눌러 폴더를 선택하세요',
                            style: TextStyle(
                              fontSize: 14,
                              color: AppColors.mutedForeground,
                            ),
                          ),
                          const SizedBox(height: 16),
                          ElevatedButton.icon(
                            onPressed: _pickFolder,
                            icon: const Icon(LucideIcons.folderOpen, size: 20),
                            label: const Text('폴더 선택'),
                            style: ElevatedButton.styleFrom(
                              backgroundColor: AppColors.primary,
                              foregroundColor: Colors.white,
                              padding: const EdgeInsets.symmetric(
                                horizontal: 32,
                                vertical: 16,
                              ),
                              shape: RoundedRectangleBorder(
                                borderRadius: BorderRadius.circular(16),
                              ),
                              elevation: 4,
                              shadowColor: AppColors.primary.withOpacity(0.3),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),

              // Shared Folders List
              if (provider.folders.isNotEmpty)
                Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          const Text(
                            '공유 중인 폴더',
                            style: TextStyle(
                              fontSize: 20,
                              fontWeight: FontWeight.w500,
                            ),
                          ),
                          Text(
                            '${provider.folders.length}개',
                            style: const TextStyle(
                              fontSize: 14,
                              color: AppColors.mutedForeground,
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 12),
                      ...provider.folders.map((folder) => _buildFolderCard(folder, provider)),
                    ],
                  ),
                )
              else if (!provider.isLoading)
                Padding(
                  padding: const EdgeInsets.all(48),
                  child: Center(
                    child: Column(
                      children: [
                        Icon(
                          LucideIcons.folder,
                          size: 64,
                          color: AppColors.mutedForeground.withOpacity(0.5),
                        ),
                        const SizedBox(height: 16),
                        Text(
                          '아직 공유 폴더가 없습니다',
                          style: TextStyle(
                            fontSize: 16,
                            color: AppColors.mutedForeground.withOpacity(0.8),
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
      },
    );
  }

  Widget _buildFolderCard(SharedFolder folder, SharedFolderProvider provider) {
    return Container(
      margin: const EdgeInsets.only(bottom: 12),
      decoration: BoxDecoration(
        color: AppColors.card,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(
          color: AppColors.border,
          width: 1,
        ),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withOpacity(0.05),
            blurRadius: 10,
            offset: const Offset(0, 2),
          ),
        ],
      ),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          children: [
            Container(
              width: 48,
              height: 48,
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(12),
                gradient: LinearGradient(
                  colors: [
                    AppColors.primary.withOpacity(0.2),
                    AppColors.chart2.withOpacity(0.2),
                  ],
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                ),
              ),
              child: const Icon(
                LucideIcons.folder,
                color: AppColors.primary,
                size: 24,
              ),
            ),
            const SizedBox(width: 16),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    folder.name,
                    style: const TextStyle(
                      fontSize: 16,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    folder.path,
                    style: const TextStyle(
                      fontSize: 12,
                      color: AppColors.mutedForeground,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ],
              ),
            ),
            const SizedBox(width: 12),
            IconButton(
              onPressed: () {
                _showDeleteConfirmDialog(folder, provider);
              },
              icon: const Icon(
                LucideIcons.trash2,
                size: 20,
                color: AppColors.destructive,
              ),
              style: IconButton.styleFrom(
                backgroundColor: AppColors.destructive.withOpacity(0.1),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(8),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  void _showDeleteConfirmDialog(SharedFolder folder, SharedFolderProvider provider) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
        ),
        title: const Text('폴더 제거'),
        content: Text(
          '${folder.name} 폴더를 공유 목록에서 제거하시겠습니까?\n\n실제 폴더는 삭제되지 않습니다.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('취소'),
          ),
          ElevatedButton(
            onPressed: () {
              provider.removeFolder(folder.id);
              Navigator.pop(context);
              ScaffoldMessenger.of(context).showSnackBar(
                SnackBar(
                  content: Text('${folder.name} 폴더가 제거되었습니다'),
                  backgroundColor: AppColors.destructive,
                  behavior: SnackBarBehavior.floating,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(12),
                  ),
                ),
              );
            },
            style: ElevatedButton.styleFrom(
              backgroundColor: AppColors.destructive,
              foregroundColor: Colors.white,
            ),
            child: const Text('제거'),
          ),
        ],
      ),
    );
  }
}
