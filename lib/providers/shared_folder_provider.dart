import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:shared_preferences/shared_preferences.dart';
import '../models/shared_folder.dart';

class SharedFolderProvider extends ChangeNotifier {
  List<SharedFolder> _folders = [];
  bool _isLoading = false;

  List<SharedFolder> get folders => _folders;
  bool get isLoading => _isLoading;

  static const String _storageKey = 'shared_folders';

  SharedFolderProvider() {
    loadFolders();
  }

  Future<void> loadFolders() async {
    _isLoading = true;
    notifyListeners();

    try {
      final prefs = await SharedPreferences.getInstance();
      final foldersJson = prefs.getString(_storageKey);

      if (foldersJson != null) {
        final List<dynamic> decoded = jsonDecode(foldersJson);
        _folders = decoded.map((json) => SharedFolder.fromJson(json)).toList();
      }
    } catch (e) {
      debugPrint('Error loading folders: $e');
    } finally {
      _isLoading = false;
      notifyListeners();
    }
  }

  Future<void> _saveFolders() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final foldersJson = jsonEncode(_folders.map((f) => f.toJson()).toList());
      await prefs.setString(_storageKey, foldersJson);
    } catch (e) {
      debugPrint('Error saving folders: $e');
    }
  }

  Future<void> addFolder(SharedFolder folder) async {
    _folders.add(folder);
    notifyListeners();
    await _saveFolders();
  }

  Future<void> removeFolder(String folderId) async {
    _folders.removeWhere((folder) => folder.id == folderId);
    notifyListeners();
    await _saveFolders();
  }

  Future<void> updateFolder(SharedFolder updatedFolder) async {
    final index = _folders.indexWhere((f) => f.id == updatedFolder.id);
    if (index != -1) {
      _folders[index] = updatedFolder;
      notifyListeners();
      await _saveFolders();
    }
  }

  SharedFolder? getFolderById(String id) {
    try {
      return _folders.firstWhere((folder) => folder.id == id);
    } catch (e) {
      return null;
    }
  }
}
