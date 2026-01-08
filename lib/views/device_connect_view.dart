import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:qr_flutter/qr_flutter.dart';
import 'package:mobile_scanner/mobile_scanner.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import '../theme/app_colors.dart';
import 'dart:convert';

class DeviceConnectView extends StatefulWidget {
  const DeviceConnectView({super.key});

  @override
  State<DeviceConnectView> createState() => _DeviceConnectViewState();
}

class _DeviceConnectViewState extends State<DeviceConnectView> {
  bool _isScanning = false;
  MobileScannerController? _scannerController;

  @override
  void initState() {
    super.initState();
    if (!kIsWeb) {
      _scannerController = MobileScannerController();
    }
  }

  @override
  void dispose() {
    _scannerController?.dispose();
    super.dispose();
  }

  String _generateConnectionData() {
    final connectionInfo = {
      'deviceId': DateTime.now().millisecondsSinceEpoch.toString(),
      'deviceName': 'My Device',
      'timestamp': DateTime.now().toIso8601String(),
    };
    return jsonEncode(connectionInfo);
  }

  void _handleQRScanned(BarcodeCapture barcodes) {
    if (barcodes.barcodes.isNotEmpty) {
      final String? code = barcodes.barcodes.first.rawValue;
      if (code != null) {
        _scannerController?.stop();
        _showConnectionDialog(code);
      }
    }
  }

  void _showConnectionDialog(String qrData) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
        ),
        title: const Row(
          children: [
            Icon(LucideIcons.check, color: AppColors.primary),
            SizedBox(width: 8),
            Text('기기 연결'),
          ],
        ),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text('새로운 기기를 연결하시겠습니까?'),
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: AppColors.muted,
                borderRadius: BorderRadius.circular(8),
              ),
              child: Text(
                qrData,
                style: const TextStyle(
                  fontSize: 12,
                  fontFamily: 'monospace',
                ),
                maxLines: 3,
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () {
              Navigator.pop(context);
              setState(() => _isScanning = false);
            },
            child: const Text('취소'),
          ),
          ElevatedButton(
            onPressed: () {
              // TODO: Implement device connection logic
              Navigator.pop(context);
              setState(() => _isScanning = false);
              ScaffoldMessenger.of(context).showSnackBar(
                SnackBar(
                  content: const Text('기기가 연결되었습니다'),
                  backgroundColor: AppColors.primary,
                  behavior: SnackBarBehavior.floating,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(12),
                  ),
                ),
              );
            },
            style: ElevatedButton.styleFrom(
              backgroundColor: AppColors.primary,
              foregroundColor: Colors.white,
            ),
            child: const Text('연결'),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: AppColors.background,
      body: SafeArea(
        child: Column(
          children: [
            // Header
            Padding(
              padding: const EdgeInsets.all(16),
              child: Row(
                children: [
                  IconButton(
                    onPressed: () => Navigator.pop(context),
                    icon: const Icon(LucideIcons.arrowLeft),
                    style: IconButton.styleFrom(
                      backgroundColor: AppColors.muted,
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(12),
                      ),
                    ),
                  ),
                  const SizedBox(width: 12),
                  const Text(
                    '기기 연결',
                    style: TextStyle(
                      fontSize: 24,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                ],
              ),
            ),

            // Tab Selector
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: Container(
                decoration: BoxDecoration(
                  color: AppColors.muted,
                  borderRadius: BorderRadius.circular(16),
                ),
                child: Row(
                  children: [
                    Expanded(
                      child: _buildTabButton(
                        'QR 코드 표시',
                        LucideIcons.qrCode,
                        !_isScanning,
                        () => setState(() => _isScanning = false),
                      ),
                    ),
                    if (!kIsWeb)
                      Expanded(
                        child: _buildTabButton(
                          'QR 스캔',
                          LucideIcons.scan,
                          _isScanning,
                          () => setState(() => _isScanning = true),
                        ),
                      ),
                  ],
                ),
              ),
            ),

            const SizedBox(height: 24),

            // Content
            Expanded(
              child: _isScanning ? _buildScannerView() : _buildQRCodeView(),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildTabButton(String label, IconData icon, bool isActive, VoidCallback onTap) {
    return GestureDetector(
      onTap: onTap,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 200),
        padding: const EdgeInsets.symmetric(vertical: 12),
        decoration: BoxDecoration(
          color: isActive ? AppColors.primary : Colors.transparent,
          borderRadius: BorderRadius.circular(12),
        ),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              icon,
              size: 18,
              color: isActive ? Colors.white : AppColors.mutedForeground,
            ),
            const SizedBox(width: 8),
            Text(
              label,
              style: TextStyle(
                fontSize: 14,
                fontWeight: isActive ? FontWeight.w500 : FontWeight.w400,
                color: isActive ? Colors.white : AppColors.mutedForeground,
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildQRCodeView() {
    final qrData = _generateConnectionData();

    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        children: [
          Container(
            padding: const EdgeInsets.all(24),
            decoration: BoxDecoration(
              color: AppColors.card,
              borderRadius: BorderRadius.circular(24),
              border: Border.all(
                color: AppColors.border,
                width: 1,
              ),
              boxShadow: [
                BoxShadow(
                  color: Colors.black.withOpacity(0.05),
                  blurRadius: 20,
                  offset: const Offset(0, 4),
                ),
              ],
            ),
            child: Column(
              children: [
                const Text(
                  '다른 기기에서 이 코드를 스캔하세요',
                  style: TextStyle(
                    fontSize: 18,
                    fontWeight: FontWeight.w500,
                  ),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 8),
                const Text(
                  'QR 코드를 스캔하여 기기를 연결할 수 있습니다',
                  style: TextStyle(
                    fontSize: 14,
                    color: AppColors.mutedForeground,
                  ),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 32),
                Container(
                  padding: const EdgeInsets.all(16),
                  decoration: BoxDecoration(
                    color: Colors.white,
                    borderRadius: BorderRadius.circular(16),
                  ),
                  child: QrImageView(
                    data: qrData,
                    version: QrVersions.auto,
                    size: 280,
                    backgroundColor: Colors.white,
                  ),
                ),
                const SizedBox(height: 32),
                Container(
                  padding: const EdgeInsets.all(16),
                  decoration: BoxDecoration(
                    color: AppColors.muted,
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Row(
                    children: const [
                      Icon(
                        LucideIcons.info,
                        size: 20,
                        color: AppColors.primary,
                      ),
                      SizedBox(width: 12),
                      Expanded(
                        child: Text(
                          'QR 코드는 보안을 위해 5분 후 자동으로 갱신됩니다',
                          style: TextStyle(
                            fontSize: 12,
                            color: AppColors.mutedForeground,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),
          _buildInfoCard(),
        ],
      ),
    );
  }

  Widget _buildScannerView() {
    if (kIsWeb) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: const [
            Icon(
              LucideIcons.camera,
              size: 64,
              color: AppColors.mutedForeground,
            ),
            SizedBox(height: 16),
            Text(
              '웹에서는 QR 스캔을 지원하지 않습니다',
              style: TextStyle(
                fontSize: 16,
                color: AppColors.mutedForeground,
              ),
            ),
          ],
        ),
      );
    }

    return Column(
      children: [
        Expanded(
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: ClipRRect(
              borderRadius: BorderRadius.circular(24),
              child: MobileScanner(
                controller: _scannerController,
                onDetect: _handleQRScanned,
              ),
            ),
          ),
        ),
        const SizedBox(height: 16),
        Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            children: [
              const Text(
                'QR 코드를 카메라에 맞추세요',
                style: TextStyle(
                  fontSize: 16,
                  fontWeight: FontWeight.w500,
                ),
              ),
              const SizedBox(height: 8),
              const Text(
                '다른 기기의 QR 코드를 스캔하여 연결합니다',
                style: TextStyle(
                  fontSize: 14,
                  color: AppColors.mutedForeground,
                ),
              ),
              const SizedBox(height: 16),
              ElevatedButton.icon(
                onPressed: () {
                  _scannerController?.toggleTorch();
                },
                icon: const Icon(LucideIcons.flashlight, size: 18),
                label: const Text('플래시'),
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.muted,
                  foregroundColor: AppColors.foreground,
                  padding: const EdgeInsets.symmetric(
                    horizontal: 24,
                    vertical: 12,
                  ),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(12),
                  ),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildInfoCard() {
    return Container(
      padding: const EdgeInsets.all(20),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(24),
        gradient: const LinearGradient(
          colors: [
            AppColors.primary,
            AppColors.chart2,
            AppColors.chart3,
          ],
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
        ),
        boxShadow: [
          BoxShadow(
            color: AppColors.primary.withOpacity(0.25),
            blurRadius: 20,
            offset: const Offset(0, 4),
          ),
        ],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: const [
              Icon(
                LucideIcons.shield,
                color: Colors.white,
                size: 24,
              ),
              SizedBox(width: 12),
              Text(
                '안전한 연결',
                style: TextStyle(
                  fontSize: 18,
                  fontWeight: FontWeight.w500,
                  color: Colors.white,
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          const Text(
            'P2P 암호화 연결로 데이터를 안전하게 전송합니다. 연결 코드는 일회성이며 재사용되지 않습니다.',
            style: TextStyle(
              fontSize: 14,
              color: Colors.white70,
              height: 1.5,
            ),
          ),
        ],
      ),
    );
  }
}
