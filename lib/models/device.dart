class Device {
  final String name;
  final DeviceType type;
  final DeviceStatus status;
  final String lastSync;

  Device({
    required this.name,
    required this.type,
    required this.status,
    required this.lastSync,
  });
}

enum DeviceType {
  phone,
  desktop,
  tablet,
}

enum DeviceStatus {
  connected,
  offline,
}
