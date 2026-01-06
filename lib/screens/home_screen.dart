import 'package:flutter/material.dart';

class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key});

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
              'Visible Security & Simplicity',
              style: TextStyle(
                fontSize: 32,
                fontWeight: FontWeight.bold,
              ),
            ),
            const SizedBox(height: 32),
            Card(
              child: Padding(
                padding: const EdgeInsets.all(24.0),
                child: Row(
                  children: [
                    const Icon(
                      Icons.laptop_mac,
                      size: 48,
                      color: Color(0xFF00BCD4),
                    ),
                    const SizedBox(width: 16),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          const Text(
                            'My Device:',
                            style: TextStyle(
                              fontSize: 14,
                              color: Colors.grey,
                            ),
                          ),
                          const SizedBox(height: 4),
                          const Text(
                            "Dohyeong's MacBook",
                            style: TextStyle(
                              fontSize: 20,
                              fontWeight: FontWeight.bold,
                            ),
                          ),
                          const SizedBox(height: 8),
                          Row(
                            children: [
                              Container(
                                width: 12,
                                height: 12,
                                decoration: const BoxDecoration(
                                  shape: BoxShape.circle,
                                  color: Colors.green,
                                ),
                              ),
                              const SizedBox(width: 8),
                              const Text(
                                'Online',
                                style: TextStyle(color: Colors.green),
                              ),
                            ],
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 32),
            Center(
              child: SizedBox(
                width: 300,
                height: 300,
                child: Stack(
                  alignment: Alignment.center,
                  children: [
                    TweenAnimationBuilder(
                      tween: Tween<double>(begin: 0, end: 1),
                      duration: const Duration(seconds: 2),
                      builder: (context, double value, child) {
                        return CustomPaint(
                          size: const Size(300, 300),
                          painter: RipplePainter(
                            animationValue: value,
                            color: const Color(0xFF00BCD4),
                          ),
                        );
                      },
                    ),
                    Container(
                      width: 100,
                      height: 100,
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        border: Border.all(
                          color: const Color(0xFF00BCD4),
                          width: 3,
                        ),
                      ),
                      child: const Icon(
                        Icons.search,
                        size: 48,
                        color: Color(0xFF00BCD4),
                      ),
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 32),
            const Text(
              'Recent Activity',
              style: TextStyle(
                fontSize: 24,
                fontWeight: FontWeight.bold,
              ),
            ),
            const SizedBox(height: 16),
            _buildActivityCard(
              Icons.description,
              'report.pdf',
              'Created',
              '14:30',
            ),
            const SizedBox(height: 12),
            _buildActivityCard(
              Icons.edit_note,
              'memo.txt',
              'Modified',
              '14:45',
            ),
            const SizedBox(height: 12),
            _buildActivityCard(
              Icons.delete_outline,
              'old_file.zip',
              'Deleted',
              '15:00',
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildActivityCard(
    IconData icon,
    String filename,
    String action,
    String time,
  ) {
    return Card(
      child: ListTile(
        leading: Icon(icon, color: const Color(0xFF00BCD4)),
        title: Text(filename),
        subtitle: Text(action),
        trailing: Text(time, style: const TextStyle(color: Colors.grey)),
      ),
    );
  }
}

// Custom Painter for ripple animation
class RipplePainter extends CustomPainter {
  final double animationValue;
  final Color color;

  RipplePainter({
    required this.animationValue,
    required this.color,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color.withAlpha((255 * 0.3 * (1 - animationValue)).toInt())
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2;

    final center = Offset(size.width / 2, size.height / 2);
    final radius = size.width / 2 * animationValue;

    canvas.drawCircle(center, radius, paint);

    paint.color = color.withAlpha((255 * 0.2 * (1 - animationValue)).toInt());
    canvas.drawCircle(center, radius * 0.7, paint);

    paint.color = color.withAlpha((255 * 0.1 * (1 - animationValue)).toInt());
    canvas.drawCircle(center, radius * 0.4, paint);
  }

  @override
  bool shouldRepaint(RipplePainter oldDelegate) {
    return oldDelegate.animationValue != animationValue;
  }
}
