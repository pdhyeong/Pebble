import 'device.dart';

class Activity {
  final String id;
  final ActivityType type;
  final String fileName;
  final DeviceInfo device;
  final String timestamp;
  final String? size;
  final ActivityStatus status;

  Activity({
    required this.id,
    required this.type,
    required this.fileName,
    required this.device,
    required this.timestamp,
    this.size,
    required this.status,
  });
}

class DeviceInfo {
  final String name;
  final DeviceType type;

  DeviceInfo({
    required this.name,
    required this.type,
  });
}

enum ActivityType {
  download,
  upload,
  delete,
  access,
  share,
}

enum ActivityStatus {
  completed,
  failed,
  inProgress,
}
