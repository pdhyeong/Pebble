import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';
import 'app_colors.dart';

class AppTheme {
  static ThemeData get lightTheme {
    return ThemeData(
      useMaterial3: true,
      colorScheme: const ColorScheme.light(
        surface: AppColors.background,
        onSurface: AppColors.foreground,
        primary: AppColors.primary,
        onPrimary: AppColors.primaryForeground,
        secondary: AppColors.secondary,
        onSecondary: AppColors.secondaryForeground,
        error: AppColors.destructive,
        onError: AppColors.destructiveForeground,
      ),
      scaffoldBackgroundColor: AppColors.background,
      cardTheme: CardThemeData(
        color: AppColors.card,
        elevation: 0,
        shape: RoundedRectangleBorder(
          borderRadius: const BorderRadius.all(Radius.circular(16)),
          side: const BorderSide(color: AppColors.border, width: 1),
        ),
      ),
      textTheme: GoogleFonts.interTextTheme(
        const TextTheme(
          displayLarge: TextStyle(
            fontSize: 32,
            fontWeight: FontWeight.w500,
            height: 1.5,
            color: AppColors.foreground,
          ),
          displayMedium: TextStyle(
            fontSize: 24,
            fontWeight: FontWeight.w500,
            height: 1.5,
            color: AppColors.foreground,
          ),
          displaySmall: TextStyle(
            fontSize: 20,
            fontWeight: FontWeight.w500,
            height: 1.5,
            color: AppColors.foreground,
          ),
          bodyLarge: TextStyle(
            fontSize: 16,
            fontWeight: FontWeight.w400,
            height: 1.5,
            color: AppColors.foreground,
          ),
          bodyMedium: TextStyle(
            fontSize: 14,
            fontWeight: FontWeight.w400,
            height: 1.5,
            color: AppColors.foreground,
          ),
          bodySmall: TextStyle(
            fontSize: 12,
            fontWeight: FontWeight.w400,
            height: 1.5,
            color: AppColors.mutedForeground,
          ),
          labelLarge: TextStyle(
            fontSize: 16,
            fontWeight: FontWeight.w500,
            height: 1.5,
            color: AppColors.foreground,
          ),
        ),
      ),
      appBarTheme: const AppBarTheme(
        backgroundColor: Color(0xCCFAFAFF),
        elevation: 0,
        centerTitle: true,
      ),
      bottomNavigationBarTheme: const BottomNavigationBarThemeData(
        backgroundColor: Color(0xCCFAFAFF),
        selectedItemColor: AppColors.primary,
        unselectedItemColor: AppColors.mutedForeground,
        type: BottomNavigationBarType.fixed,
        elevation: 0,
      ),
    );
  }
}
