import 'package:flutter/material.dart';

class ActivityScreen extends StatefulWidget {
  const ActivityScreen({super.key});

  @override
  State<ActivityScreen> createState() => _ActivityScreenState();
}

class _ActivityScreenState extends State<ActivityScreen> {
  bool _isWatching = true;
  final List<FileActivity> _activities = [
    FileActivity(
      filename: 'Project_Plan V2.pdf',
      action: 'Modified',
      time: '16:35:05',
      icon: Icons.description,
      folder: 'File me1n',
    ),
    FileActivity(
      filename: 'IMG-4821.png',
      action: 'Created',
      time: '16:35:05',
      icon: Icons.image,
      folder: 'File fio.ita',
    ),
    FileActivity(
      filename: 'meeting_notes.txt',
      action: 'Created',
      time: '16:34:38',
      icon: Icons.description,
      folder: 'File inie raid',
    ),
  ];

  @override
  void initState() {
    super.initState();
    // TODO: Rust Stream 연결 - 추후 Rust에서 파일 감시 이벤트를 받아올 때 사용
    // StreamBuilder를 사용해서 실시간 파일 변경 이벤트를 받아올 수 있습니다.
    // Example:
    // _fileWatchStream = RustLib.watchFiles();
    // _fileWatchStream.listen((event) {
    //   setState(() {
    //     _activities.insert(0, FileActivity.fromEvent(event));
    //   });
    // });
  }

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
      child: Column(
        children: [
          Container(
            padding: const EdgeInsets.all(32.0),
            child: Row(
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Text(
                        'Activity',
                        style: TextStyle(
                          fontSize: 32,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                      const SizedBox(height: 8),
                      Row(
                        children: [
                          const Text(
                            'Watching:',
                            style: TextStyle(
                              color: Colors.grey,
                              fontSize: 16,
                            ),
                          ),
                          const SizedBox(width: 8),
                          const Text(
                            '~/Downloads/Pebble_Test',
                            style: TextStyle(
                              color: Color(0xFF00BCD4),
                              fontSize: 16,
                            ),
                          ),
                          const SizedBox(width: 8),
                          TextButton(
                            onPressed: () {
                              // TODO: 폴더 선택 다이얼로그 열기
                            },
                            child: const Text('Change'),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
                Row(
                  children: [
                    const Text('Real-time Watcher'),
                    const SizedBox(width: 16),
                    Switch(
                      value: _isWatching,
                      onChanged: (value) {
                        setState(() {
                          _isWatching = value;
                        });
                        // TODO: Rust 파일 감시 시작/중지
                      },
                      activeThumbColor: const Color(0xFF00BCD4),
                    ),
                  ],
                ),
              ],
            ),
          ),
          Expanded(
            child: AnimatedList(
              padding: const EdgeInsets.symmetric(horizontal: 32.0),
              initialItemCount: _activities.length,
              itemBuilder: (context, index, animation) {
                final activity = _activities[index];
                return _buildActivityItem(activity, animation);
              },
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildActivityItem(FileActivity activity, Animation<double> animation) {
    return SizeTransition(
      sizeFactor: animation,
      child: FadeTransition(
        opacity: animation,
        child: Container(
          margin: const EdgeInsets.only(bottom: 16),
          child: Card(
            child: Padding(
              padding: const EdgeInsets.all(20.0),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    activity.folder,
                    style: const TextStyle(
                      fontSize: 12,
                      color: Colors.grey,
                    ),
                  ),
                  const SizedBox(height: 12),
                  Row(
                    children: [
                      Container(
                        padding: const EdgeInsets.all(12),
                        decoration: BoxDecoration(
                          color: const Color(0xFF1A237E),
                          borderRadius: BorderRadius.circular(8),
                        ),
                        child: Icon(
                          activity.icon,
                          color: const Color(0xFF00BCD4),
                        ),
                      ),
                      const SizedBox(width: 16),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              activity.filename,
                              style: const TextStyle(
                                fontSize: 16,
                                fontWeight: FontWeight.bold,
                              ),
                            ),
                            const SizedBox(height: 4),
                            Text(
                              activity.action,
                              style: const TextStyle(
                                color: Color(0xFF00BCD4),
                                fontSize: 14,
                              ),
                            ),
                          ],
                        ),
                      ),
                      Text(
                        activity.time,
                        style: const TextStyle(
                          color: Color(0xFF00BCD4),
                          fontSize: 14,
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class FileActivity {
  final String filename;
  final String action;
  final String time;
  final IconData icon;
  final String folder;

  FileActivity({
    required this.filename,
    required this.action,
    required this.time,
    required this.icon,
    required this.folder,
  });

  // TODO: Rust 이벤트로부터 FileActivity 객체 생성
  // factory FileActivity.fromEvent(RustFileEvent event) {
  //   return FileActivity(
  //     filename: event.filename,
  //     action: event.action,
  //     time: event.time,
  //     icon: _getIconForAction(event.action),
  //     folder: event.folder,
  //   );
  // }
}
