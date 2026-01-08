class SharedFolder {
  final String id;
  final String name;
  final String path;
  final DateTime addedAt;
  final int fileCount;
  final String size;

  SharedFolder({
    required this.id,
    required this.name,
    required this.path,
    required this.addedAt,
    this.fileCount = 0,
    this.size = '0 MB',
  });

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'name': name,
      'path': path,
      'addedAt': addedAt.toIso8601String(),
      'fileCount': fileCount,
      'size': size,
    };
  }

  factory SharedFolder.fromJson(Map<String, dynamic> json) {
    return SharedFolder(
      id: json['id'] as String,
      name: json['name'] as String,
      path: json['path'] as String,
      addedAt: DateTime.parse(json['addedAt'] as String),
      fileCount: json['fileCount'] as int? ?? 0,
      size: json['size'] as String? ?? '0 MB',
    );
  }

  SharedFolder copyWith({
    String? id,
    String? name,
    String? path,
    DateTime? addedAt,
    int? fileCount,
    String? size,
  }) {
    return SharedFolder(
      id: id ?? this.id,
      name: name ?? this.name,
      path: path ?? this.path,
      addedAt: addedAt ?? this.addedAt,
      fileCount: fileCount ?? this.fileCount,
      size: size ?? this.size,
    );
  }
}
