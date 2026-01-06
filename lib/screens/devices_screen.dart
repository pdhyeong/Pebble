import 'package:flutter/material.dart';

class DevicesScreen extends StatelessWidget {
  const DevicesScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: const BoxDecoration(
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [
            Color.fromRGBO(26, 35, 126, 0.3),
            Colors.black,
          ],
        ),
      ),
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(32.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text(
              'Devices',
              style: TextStyle(
                fontSize: 32,
                fontWeight: FontWeight.bold,
              ),
            ),
            const SizedBox(height: 32),
            Center(
              child: Card(
                child: Padding(
                  padding: const EdgeInsets.all(32.0),
                  child: Column(
                    children: [
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          const Text(
                            'Real-time Watcher',
                            style: TextStyle(fontSize: 18),
                          ),
                          Switch(
                            value: true,
                            onChanged: (value) {
                              // TODO: 실시간 감시 토글
                            },
                            activeThumbColor: const Color(0xFF00BCD4),
                          ),
                        ],
                      ),
                      const Divider(height: 32),
                      const Text(
                        'Connection & Settings',
                        style: TextStyle(
                          fontSize: 20,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                      const SizedBox(height: 24),
                      Container(
                        padding: const EdgeInsets.all(24),
                        decoration: BoxDecoration(
                          color: Colors.white,
                          borderRadius: BorderRadius.circular(16),
                          border: Border.all(
                            color: const Color(0xFF00BCD4),
                            width: 3,
                          ),
                        ),
                        child: Column(
                          children: [
                            Container(
                              width: 200,
                              height: 200,
                              color: Colors.black,
                              child: const Center(
                                child: Icon(
                                  Icons.qr_code_2,
                                  size: 150,
                                  color: Color(0xFF00BCD4),
                                ),
                              ),
                            ),
                            const SizedBox(height: 16),
                            const Text(
                              'Scan to Pair New Device',
                              style: TextStyle(
                                color: Colors.black87,
                                fontSize: 16,
                                fontWeight: FontWeight.bold,
                              ),
                            ),
                          ],
                        ),
                      ),
                      const SizedBox(height: 32),
                      const Text(
                        'My Devices',
                        style: TextStyle(
                          fontSize: 20,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                      const SizedBox(height: 16),
                      _buildDeviceCard(
                        Icons.phone_iphone,
                        'iPhone 15 Pro',
                        'Last Sync: Just now',
                      ),
                      const SizedBox(height: 12),
                      _buildDeviceCard(
                        Icons.computer,
                        'Windows-PC',
                        'Last Sync: 2 hours ago',
                      ),
                    ],
                  ),
                ),
              ),
            ),
            const SizedBox(height: 32),
            const Center(
              child: Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(
                    Icons.shield,
                    color: Color(0xFF00BCD4),
                    size: 24,
                  ),
                  SizedBox(width: 8),
                  Text(
                    'Encrypted Tunnel Active',
                    style: TextStyle(
                      fontSize: 16,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildDeviceCard(IconData icon, String name, String status) {
    return Card(
      color: const Color(0xFF1A237E),
      child: ListTile(
        leading: Icon(icon, color: const Color(0xFF00BCD4), size: 32),
        title: Text(
          name,
          style: const TextStyle(fontWeight: FontWeight.bold),
        ),
        subtitle: Text(status),
      ),
    );
  }
}
