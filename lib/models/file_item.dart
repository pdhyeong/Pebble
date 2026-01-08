class FileItem {
  final String id;
  final String name;
  final FileItemType type;
  final String? size;
  final String modifiedDate;
  final int? itemCount;

  FileItem({
    required this.id,
    required this.name,
    required this.type,
    this.size,
    required this.modifiedDate,
    this.itemCount,
  });
}

enum FileItemType {
  folder,
  document,
  image,
  video,
  audio,
  other,
}
