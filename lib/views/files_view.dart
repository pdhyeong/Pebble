import 'package:flutter/material.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import '../models/file_item.dart';
import '../theme/app_colors.dart';

class FilesView extends StatefulWidget {
  const FilesView({super.key});

  @override
  State<FilesView> createState() => _FilesViewState();
}

class _FilesViewState extends State<FilesView> {
  bool _isGridView = false;
  List<String> _currentPath = ['내 PC'];
  String _searchQuery = '';

  final List<FileItem> _files = [
    FileItem(
      id: '1',
      name: '문서',
      type: FileItemType.folder,
      modifiedDate: '오늘',
      itemCount: 24,
    ),
    FileItem(
      id: '2',
      name: '사진',
      type: FileItemType.folder,
      modifiedDate: '어제',
      itemCount: 156,
    ),
    FileItem(
      id: '3',
      name: '동영상',
      type: FileItemType.folder,
      modifiedDate: '2일 전',
      itemCount: 8,
    ),
    FileItem(
      id: '4',
      name: '다운로드',
      type: FileItemType.folder,
      modifiedDate: '오늘',
      itemCount: 43,
    ),
    FileItem(
      id: '5',
      name: '프로젝트_제안서_최종.pdf',
      type: FileItemType.document,
      size: '2.4 MB',
      modifiedDate: '5분 전',
    ),
    FileItem(
      id: '6',
      name: 'vacation_2025.jpg',
      type: FileItemType.image,
      size: '4.8 MB',
      modifiedDate: '1시간 전',
    ),
    FileItem(
      id: '7',
      name: 'tutorial_video.mp4',
      type: FileItemType.video,
      size: '128 MB',
      modifiedDate: '2시간 전',
    ),
    FileItem(
      id: '8',
      name: 'background_music.mp3',
      type: FileItemType.audio,
      size: '5.2 MB',
      modifiedDate: '어제',
    ),
  ];

  IconData _getFileIcon(FileItemType type) {
    switch (type) {
      case FileItemType.folder:
        return LucideIcons.folder;
      case FileItemType.document:
        return LucideIcons.fileText;
      case FileItemType.image:
        return LucideIcons.image;
      case FileItemType.video:
        return LucideIcons.film;
      case FileItemType.audio:
        return LucideIcons.music;
      case FileItemType.other:
        return LucideIcons.file;
    }
  }

  List<Color> _getIconGradient(FileItemType type) {
    switch (type) {
      case FileItemType.folder:
        return AppColors.orangeYellowGradient;
      case FileItemType.document:
        return AppColors.blueCyanGradient;
      case FileItemType.image:
        return AppColors.pinkRoseGradient;
      case FileItemType.video:
        return AppColors.purpleIndigoGradient;
      case FileItemType.audio:
        return AppColors.greenEmeraldGradient;
      case FileItemType.other:
        return const [Color(0xFF9CA3AF), Color(0xFF64748B)];
    }
  }

  List<FileItem> get _filteredFiles {
    return _files.where((file) {
      return file.name.toLowerCase().contains(_searchQuery.toLowerCase());
    }).toList();
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        // Search and Path Bar
        Container(
          decoration: BoxDecoration(
            color: AppColors.background.withOpacity(0.95),
            border: const Border(
              bottom: BorderSide(color: AppColors.border, width: 0.5),
            ),
          ),
          padding: const EdgeInsets.all(16),
          child: Column(
            children: [
              // Path breadcrumb
              Row(
                children: [
                  if (_currentPath.length > 1)
                    IconButton(
                      onPressed: () {
                        setState(() {
                          _currentPath.removeLast();
                        });
                      },
                      icon: const Icon(LucideIcons.arrowLeft, size: 16),
                      style: IconButton.styleFrom(
                        backgroundColor: AppColors.muted.withOpacity(0.5),
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(8),
                        ),
                        padding: const EdgeInsets.all(6),
                      ),
                    ),
                  if (_currentPath.length > 1) const SizedBox(width: 8),
                  Expanded(
                    child: SingleChildScrollView(
                      scrollDirection: Axis.horizontal,
                      child: Row(
                        children: _currentPath.asMap().entries.map((entry) {
                          final isLast = entry.key == _currentPath.length - 1;
                          return Row(
                            children: [
                              Text(
                                entry.value,
                                style: TextStyle(
                                  fontSize: 14,
                                  fontWeight: isLast ? FontWeight.w500 : FontWeight.w400,
                                  color: isLast
                                      ? AppColors.primary
                                      : AppColors.mutedForeground,
                                ),
                              ),
                              if (!isLast)
                                const Padding(
                                  padding: EdgeInsets.symmetric(horizontal: 4),
                                  child: Text(
                                    '/',
                                    style: TextStyle(
                                      fontSize: 14,
                                      color: AppColors.mutedForeground,
                                    ),
                                  ),
                                ),
                            ],
                          );
                        }).toList(),
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 12),

              // Search bar
              Row(
                children: [
                  Expanded(
                    child: TextField(
                      onChanged: (value) {
                        setState(() {
                          _searchQuery = value;
                        });
                      },
                      decoration: InputDecoration(
                        hintText: '파일 검색...',
                        hintStyle: const TextStyle(
                          color: AppColors.mutedForeground,
                        ),
                        prefixIcon: const Icon(
                          LucideIcons.search,
                          size: 16,
                          color: AppColors.mutedForeground,
                        ),
                        filled: true,
                        fillColor: AppColors.muted.withOpacity(0.5),
                        border: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(12),
                          borderSide: const BorderSide(
                            color: AppColors.border,
                            width: 0.5,
                          ),
                        ),
                        enabledBorder: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(12),
                          borderSide: const BorderSide(
                            color: AppColors.border,
                            width: 0.5,
                          ),
                        ),
                        focusedBorder: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(12),
                          borderSide: const BorderSide(
                            color: AppColors.primary,
                            width: 2,
                          ),
                        ),
                        contentPadding: const EdgeInsets.symmetric(
                          horizontal: 16,
                          vertical: 10,
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  IconButton(
                    onPressed: () {
                      setState(() {
                        _isGridView = !_isGridView;
                      });
                    },
                    icon: Icon(
                      _isGridView ? LucideIcons.list : LucideIcons.grid3x3,
                      size: 16,
                    ),
                    style: IconButton.styleFrom(
                      backgroundColor: AppColors.muted.withOpacity(0.5),
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(12),
                        side: const BorderSide(
                          color: AppColors.border,
                          width: 0.5,
                        ),
                      ),
                      padding: const EdgeInsets.all(10),
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),

        // Files Grid/List
        Expanded(
          child: _filteredFiles.isEmpty
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
                          LucideIcons.search,
                          size: 40,
                          color: AppColors.mutedForeground,
                        ),
                      ),
                      const SizedBox(height: 16),
                      const Text(
                        '검색 결과가 없습니다',
                        style: TextStyle(
                          color: AppColors.mutedForeground,
                        ),
                      ),
                    ],
                  ),
                )
              : _isGridView
                  ? _buildGridView()
                  : _buildListView(),
        ),

        // Storage Info Footer
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
            border: const Border(
              top: BorderSide(color: AppColors.border, width: 0.5),
            ),
          ),
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Text(
                    '저장공간 사용량',
                    style: TextStyle(
                      fontSize: 14,
                      color: AppColors.mutedForeground,
                    ),
                  ),
                  Text(
                    '256 GB / 512 GB',
                    style: TextStyle(
                      fontSize: 14,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 8),
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
                        widthFactor: 0.5,
                        child: Container(
                          decoration: const BoxDecoration(
                            gradient: LinearGradient(
                              colors: AppColors.primaryGradient,
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
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildListView() {
    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: _filteredFiles.length,
      itemBuilder: (context, index) {
        final file = _filteredFiles[index];
        final gradient = _getIconGradient(file.type);

        return Padding(
          padding: const EdgeInsets.only(bottom: 8),
          child: Container(
            decoration: BoxDecoration(
              color: AppColors.card,
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: AppColors.border, width: 1),
            ),
            padding: const EdgeInsets.all(12),
            child: Row(
              children: [
                Container(
                  width: 44,
                  height: 44,
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
                    _getFileIcon(file.type),
                    color: Colors.white,
                    size: 20,
                  ),
                ),
                const SizedBox(width: 12),
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
                            file.modifiedDate,
                            style: const TextStyle(
                              fontSize: 12,
                              color: AppColors.mutedForeground,
                            ),
                          ),
                          if (file.size != null) ...[
                            const Text(
                              ' • ',
                              style: TextStyle(
                                fontSize: 12,
                                color: AppColors.mutedForeground,
                              ),
                            ),
                            Text(
                              file.size!,
                              style: const TextStyle(
                                fontSize: 12,
                                color: AppColors.mutedForeground,
                              ),
                            ),
                          ],
                          if (file.itemCount != null) ...[
                            const Text(
                              ' • ',
                              style: TextStyle(
                                fontSize: 12,
                                color: AppColors.mutedForeground,
                              ),
                            ),
                            Text(
                              '${file.itemCount}개 항목',
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
                if (file.type != FileItemType.folder) ...[
                  IconButton(
                    onPressed: () {},
                    icon: const Icon(LucideIcons.download, size: 16),
                    style: IconButton.styleFrom(
                      backgroundColor: AppColors.muted.withOpacity(0.5),
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(8),
                      ),
                      padding: const EdgeInsets.all(8),
                    ),
                  ),
                  const SizedBox(width: 4),
                  IconButton(
                    onPressed: () {},
                    icon: const Icon(LucideIcons.share2, size: 16),
                    style: IconButton.styleFrom(
                      backgroundColor: AppColors.muted.withOpacity(0.5),
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(8),
                      ),
                      padding: const EdgeInsets.all(8),
                    ),
                  ),
                  const SizedBox(width: 4),
                ],
                IconButton(
                  onPressed: () {},
                  icon: const Icon(Icons.more_vert, size: 16),
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
          ),
        );
      },
    );
  }

  Widget _buildGridView() {
    return GridView.builder(
      padding: const EdgeInsets.all(16),
      gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
        crossAxisCount: 2,
        crossAxisSpacing: 12,
        mainAxisSpacing: 12,
        childAspectRatio: 0.85,
      ),
      itemCount: _filteredFiles.length,
      itemBuilder: (context, index) {
        final file = _filteredFiles[index];
        final gradient = _getIconGradient(file.type);

        return Container(
          decoration: BoxDecoration(
            color: AppColors.card,
            borderRadius: BorderRadius.circular(16),
            border: Border.all(color: AppColors.border, width: 1),
          ),
          padding: const EdgeInsets.all(12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: Container(
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
                  child: Center(
                    child: Icon(
                      _getFileIcon(file.type),
                      color: Colors.white,
                      size: 32,
                    ),
                  ),
                ),
              ),
              const SizedBox(height: 12),
              Text(
                file.name,
                style: const TextStyle(
                  fontSize: 14,
                  fontWeight: FontWeight.w500,
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              const SizedBox(height: 2),
              Text(
                file.size ?? '${file.itemCount}개 항목',
                style: const TextStyle(
                  fontSize: 12,
                  color: AppColors.mutedForeground,
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ],
          ),
        );
      },
    );
  }
}
