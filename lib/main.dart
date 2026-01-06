import 'package:flutter/material.dart';
import 'package:pebble/src/rust/frb_generated.dart';
import 'package:pebble/screens/home_screen.dart';
import 'package:pebble/screens/activity_screen.dart';
import 'package:pebble/screens/devices_screen.dart';

/// 앱 진입점
/// Rust 라이브러리를 초기화하고 Flutter 앱을 실행합니다.
Future<void> main() async {
  // Rust 바인딩 초기화
  await RustLib.init();
  runApp(const MyApp());
}

/// 메인 앱 위젯
/// Material 3 다크 테마와 Indigo/Cyan 컬러 스킴을 적용합니다.
class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Pebble',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        useMaterial3: true,
        brightness: Brightness.dark,
        colorScheme: const ColorScheme.dark(
          primary: Color(0xFF00BCD4), // Cyan - 데이터 흐름, 속도
          secondary: Color(0xFF1A237E), // Deep Indigo - 신뢰, 보안
          surface: Color(0xFF263238),
        ),
        cardTheme: const CardThemeData(
          elevation: 4,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.all(Radius.circular(16)),
          ),
        ),
      ),
      home: const MainScreen(),
    );
  }
}

class MainScreen extends StatefulWidget {
  const MainScreen({super.key});

  @override
  State<MainScreen> createState() => _MainScreenState();
}

/// 메인 화면 상태 관리
/// NavigationRail을 통한 화면 전환을 담당합니다.
class _MainScreenState extends State<MainScreen> {
  int _selectedIndex = 0;

  // 각 탭에 표시할 화면 목록
  final List<Widget> _pages = const [
    HomeScreen(),
    ActivityScreen(),
    DevicesScreen(),
  ];

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Row(
        children: [
          NavigationRail(
            selectedIndex: _selectedIndex,
            onDestinationSelected: (int index) {
              setState(() {
                _selectedIndex = index;
              });
            },
            backgroundColor: const Color(0xFF1A237E),
            labelType: NavigationRailLabelType.all,
            leading: const Padding(
              padding: EdgeInsets.all(16.0),
              child: Column(
                children: [
                  Icon(
                    Icons.shield_outlined,
                    size: 40,
                    color: Color(0xFF00BCD4),
                  ),
                  SizedBox(height: 8),
                  Text(
                    'Pebble',
                    style: TextStyle(
                      fontSize: 18,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                ],
              ),
            ),
            destinations: const [
              NavigationRailDestination(
                icon: Icon(Icons.home_outlined),
                selectedIcon: Icon(Icons.home),
                label: Text('Home'),
              ),
              NavigationRailDestination(
                icon: Icon(Icons.timeline_outlined),
                selectedIcon: Icon(Icons.timeline),
                label: Text('Activity'),
              ),
              NavigationRailDestination(
                icon: Icon(Icons.devices_outlined),
                selectedIcon: Icon(Icons.devices),
                label: Text('Devices'),
              ),
            ],
          ),
          const VerticalDivider(thickness: 1, width: 1),
          Expanded(
            child: AnimatedSwitcher(
              duration: const Duration(milliseconds: 300),
              child: _pages[_selectedIndex],
            ),
          ),
        ],
      ),
    );
  }
}