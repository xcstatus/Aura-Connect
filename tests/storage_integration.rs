use RustSsh::storage::StorageManager;
use tempfile::tempdir;

#[test]
fn test_storage_paths_validity() {
    // 验证路径生成逻辑不崩溃
    let config_dir = StorageManager::get_config_dir();
    assert!(config_dir.is_some());
    
    let sessions_path = StorageManager::get_sessions_path();
    assert!(sessions_path.unwrap().to_str().unwrap().contains("sessions.json"));
}

#[test]
fn test_atomic_write_simulation() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("atomic_test.json");
    
    // 模拟原子写入：先写临时文件再重命名
    let content = b"{\"test\": true}";
    let tmp_path = file_path.with_extension("tmp");
    std::fs::write(&tmp_path, content).unwrap();
    std::fs::rename(&tmp_path, &file_path).unwrap();
    
    assert!(file_path.exists());
    let read_back = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(read_back, "{\"test\": true}");
}
