import 'package:flutter/material.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import '../theme/app_colors.dart';

class BottomNav extends StatelessWidget {
  final int activeTab;
  final Function(int) onTabChange;

  const BottomNav({
    super.key,
    required this.activeTab,
    required this.onTabChange,
  });

  @override
  Widget build(BuildContext context) {
    final tabs = [
      _TabData(icon: Icons.home, label: '홈'),
      _TabData(icon: LucideIcons.folderOpen, label: '파일'),
      _TabData(icon: LucideIcons.activity, label: '활동'),
      _TabData(icon: LucideIcons.settings, label: '설정'),
    ];

    return Container(
      decoration: BoxDecoration(
        color: AppColors.background.withOpacity(0.8),
        border: const Border(
          top: BorderSide(color: AppColors.border, width: 0.5),
        ),
      ),
      child: SafeArea(
        top: false,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceAround,
            children: List.generate(
              tabs.length,
              (index) => _buildTabItem(tabs[index], index),
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildTabItem(_TabData tab, int index) {
    final isActive = activeTab == index;

    return GestureDetector(
      onTap: () => onTabChange(index),
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 200),
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(16),
          gradient: isActive
              ? const LinearGradient(
                  colors: [
                    Color(0x1A7C6BFF),
                    Color(0x1AFF6BAF),
                  ],
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                )
              : null,
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Stack(
              children: [
                Icon(
                  tab.icon,
                  size: 20,
                  color: isActive ? AppColors.primary : AppColors.mutedForeground,
                ),
                if (isActive)
                  Positioned(
                    top: -1,
                    right: -1,
                    child: Container(
                      width: 8,
                      height: 8,
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        gradient: const LinearGradient(
                          colors: AppColors.primaryGradient,
                          begin: Alignment.topLeft,
                          end: Alignment.bottomRight,
                        ),
                      ),
                    ),
                  ),
              ],
            ),
            const SizedBox(height: 4),
            Text(
              tab.label,
              style: TextStyle(
                fontSize: 12,
                fontWeight: isActive ? FontWeight.w500 : FontWeight.w400,
                color: isActive ? AppColors.primary : AppColors.mutedForeground,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _TabData {
  final IconData icon;
  final String label;

  _TabData({required this.icon, required this.label});
}
