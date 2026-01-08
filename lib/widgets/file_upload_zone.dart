import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:file_picker/file_picker.dart';
import 'package:desktop_drop/desktop_drop.dart';
import '../theme/app_colors.dart';

class FileUploadZone extends StatefulWidget {
  const FileUploadZone({super.key});

  @override
  State<FileUploadZone> createState() => _FileUploadZoneState();
}

class _FileUploadZoneState extends State<FileUploadZone> {
  List<String> selectedFiles = [];
  bool _isDragging = false;

  Future<void> _pickFiles() async {
    final result = await FilePicker.platform.pickFiles(
      allowMultiple: true,
    );

    if (result != null) {
      setState(() {
        selectedFiles.addAll(result.files.map((file) => file.name));
      });
    }
  }

  void _handleDroppedFiles(List<String> filePaths) {
    setState(() {
      selectedFiles.addAll(filePaths.map((path) => path.split(Platform.pathSeparator).last));
    });
  }

  @override
  Widget build(BuildContext context) {
    final dropZoneContent = InkWell(
      onTap: _pickFiles,
      borderRadius: BorderRadius.circular(24),
      child: Container(
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(24),
          border: Border.all(
            color: _isDragging ? AppColors.primary : AppColors.border,
            width: 2,
            strokeAlign: BorderSide.strokeAlignCenter,
          ),
          gradient: LinearGradient(
            colors: _isDragging
                ? [
                    AppColors.primary.withOpacity(0.15),
                    AppColors.chart2.withOpacity(0.15),
                  ]
                : const [
                    Color(0x0D7C6BFF),
                    Color(0x0DE0F2FE),
                  ],
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
          ),
        ),
        child: Column(
        children: [
          Padding(
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
                    LucideIcons.upload,
                    size: 40,
                    color: AppColors.primary,
                  ),
                ),
                const SizedBox(height: 16),
                const Text(
                  '파일을 드래그하거나',
                  style: TextStyle(
                    fontSize: 20,
                    fontWeight: FontWeight.w500,
                  ),
                ),
                const SizedBox(height: 4),
                const Text(
                  '클릭해서 업로드하세요',
                  style: TextStyle(
                    fontSize: 14,
                    color: AppColors.mutedForeground,
                  ),
                ),
                const SizedBox(height: 16),
                ElevatedButton.icon(
                  onPressed: _pickFiles,
                  icon: const Icon(LucideIcons.upload, size: 16),
                  label: const Text('파일 선택'),
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
          if (selectedFiles.isNotEmpty)
            Container(
              margin: const EdgeInsets.fromLTRB(16, 0, 16, 16),
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: AppColors.background.withOpacity(0.8),
                borderRadius: BorderRadius.circular(16),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    '선택된 파일 (${selectedFiles.length})',
                    style: const TextStyle(
                      fontSize: 12,
                      color: AppColors.mutedForeground,
                    ),
                  ),
                  const SizedBox(height: 8),
                  ...selectedFiles.take(3).map((file) => Padding(
                        padding: const EdgeInsets.only(bottom: 8),
                        child: Container(
                          padding: const EdgeInsets.all(8),
                          decoration: BoxDecoration(
                            color: AppColors.muted.withOpacity(0.5),
                            borderRadius: BorderRadius.circular(12),
                          ),
                          child: Row(
                            children: [
                              Container(
                                width: 32,
                                height: 32,
                                decoration: BoxDecoration(
                                  color: AppColors.primary.withOpacity(0.1),
                                  borderRadius: BorderRadius.circular(8),
                                ),
                                child: const Icon(
                                  LucideIcons.fileText,
                                  size: 16,
                                  color: AppColors.primary,
                                ),
                              ),
                              const SizedBox(width: 12),
                              Expanded(
                                child: Text(
                                  file,
                                  style: const TextStyle(fontSize: 14),
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                            ],
                          ),
                        ),
                      )),
                  if (selectedFiles.length > 3)
                    Center(
                      child: Padding(
                        padding: const EdgeInsets.only(top: 8),
                        child: Text(
                          '+${selectedFiles.length - 3}개 더 보기',
                          style: const TextStyle(
                            fontSize: 12,
                            color: AppColors.mutedForeground,
                          ),
                        ),
                      ),
                    ),
                ],
              ),
            ),
        ],
        ),
      ),
    );

    // Wrap with DropTarget for desktop platforms
    final Widget uploadZone = (!kIsWeb && (Platform.isWindows || Platform.isMacOS || Platform.isLinux))
        ? DropTarget(
            onDragEntered: (_) {
              setState(() => _isDragging = true);
            },
            onDragExited: (_) {
              setState(() => _isDragging = false);
            },
            onDragDone: (details) {
              setState(() => _isDragging = false);
              _handleDroppedFiles(details.files.map((f) => f.path).toList());
            },
            child: dropZoneContent,
          )
        : dropZoneContent;

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 24),
      child: uploadZone,
    );
  }
}
