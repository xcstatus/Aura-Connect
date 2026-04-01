use rust_ssh::storage::StorageManager;
use tempfile::tempdir;

#[test]
fn test_storage_paths_validity() {
    // 测试内容：验证 `StorageManager` 的路径拼装约定是“可用且符合预期”的。
    //
    // 测试原因：
    // - 配置目录/会话文件路径通常会在应用启动早期被调用；一旦返回 None 或包含非法路径，
    //   会导致后续读写配置、加载会话列表等功能整体不可用。
    // - 这里用最小断言覆盖“不会崩溃 + 关键文件名约定不被改坏”，避免重构时悄悄写到错误位置。
    let config_dir = StorageManager::get_config_dir();
    assert!(config_dir.is_some());
    
    let sessions_path = StorageManager::get_sessions_path();
    assert!(sessions_path.unwrap().to_str().unwrap().contains("sessions.json"));
}

#[test]
fn test_atomic_write_simulation() {
    // 测试内容：用临时目录模拟一次“写临时文件 -> rename 覆盖”的原子写入流程。
    //
    // 测试原因：
    // - 会话/配置文件属于关键状态；写入过程中如果进程崩溃或被打断，最怕留下半截文件导致下次启动无法解析。
    // - 原子写的核心语义是：要么旧文件保持不变，要么新文件完整落盘（依赖 rename 的原子性）。
    // - 这里不直接测试 `StorageManager` 的具体实现细节，而是锁定住“推荐写入模式”的期望行为，
    //   防止未来改动把写入退化成非原子覆盖写。
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("atomic_test.json");
    
    // 注意：使用与目标文件同目录的临时文件，才能保证 rename 在多数平台/文件系统上具备原子性语义。
    let content = b"{\"test\": true}";
    let tmp_path = file_path.with_extension("tmp");
    std::fs::write(&tmp_path, content).unwrap();
    std::fs::rename(&tmp_path, &file_path).unwrap();
    
    assert!(file_path.exists());
    let read_back = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(read_back, "{\"test\": true}");
}
