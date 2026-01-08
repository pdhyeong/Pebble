import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:provider/provider.dart';
import 'package:pebble/src/rust/frb_generated.dart';
import 'theme/app_theme.dart';
import 'app.dart';
import 'providers/shared_folder_provider.dart';

/// 앱 진입점
/// Rust 라이브러리를 초기화하고 Flutter 앱을 실행합니다.
Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // ⚠️ CRITICAL: Rust 바인딩 초기화 (기존 코드 유지)
  await RustLib.init();

  // System UI 설정 (pebble_app 스타일)
  SystemChrome.setSystemUIOverlayStyle(
    const SystemUiOverlayStyle(
      statusBarColor: Colors.transparent,
      statusBarIconBrightness: Brightness.dark,
    ),
  );

  runApp(const MyApp());
}

/// 메인 앱 위젯
/// pebble_app의 Material 3 라이트 테마와 새로운 네비게이션을 적용합니다.
class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MultiProvider(
      providers: [
        ChangeNotifierProvider(create: (_) => SharedFolderProvider()),
      ],
      child: MaterialApp(
        title: 'Pebble',
        debugShowCheckedModeBanner: false,
        theme: AppTheme.lightTheme, // pebble_app의 라이트 테마
        home: const PebbleApp(), // pebble_app의 BottomNav 네비게이션
      ),
    );
  }
}
