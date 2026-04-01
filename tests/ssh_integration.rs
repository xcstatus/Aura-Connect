use rust_ssh::backend::ssh_session::{AsyncSession, SshSession};
use std::path::Path;

fn load_env_test_file(path: &Path) {
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim();
            let val = v.trim().trim_matches('"');
            if std::env::var_os(key).is_none() {
                unsafe { std::env::set_var(key, val) };
            }
        }
    }
}

/// 可选的手工集成测试：验证 SSH 连接链路“能跑通”。
///
/// ## 测试内容
/// - 从环境变量（以及可选的 `.env.test`）读取 SSH 目标信息：host/port/user/password
/// - 发起一次真实的 SSH 连接
/// - 触发一次 `read_stream` 的非阻塞读取，确保接口在“刚建立连接/可能还没有数据”场景下也能安全返回
///
/// ## 测试原因
/// - 这是对 `SshSession::connect` 以及底层会话初始化的冒烟覆盖：能及早发现依赖库、握手、鉴权等集成问题
/// - `read_stream` 往往是 UI/事件循环的高频调用点；这里强调“不得 panic / 不得卡死”，避免回归造成终端渲染线程被阻塞
///
/// ## 为什么默认忽略
/// - 该测试依赖外部 SSH 服务与网络环境，无法保证在 CI 或任意开发机上可复现
/// - 因此用 `#[ignore]` 保持 `cargo test` 的可重复性（hermetic）
#[tokio::test]
#[ignore]
async fn ssh_session_connect_smoke() -> anyhow::Result<()> {
    load_env_test_file(Path::new(".env.test"));

    let host = std::env::var("RUSTSSH_TEST_SSH_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port: u16 = std::env::var("RUSTSSH_TEST_SSH_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(22);
    let user = std::env::var("RUSTSSH_TEST_SSH_USER").unwrap_or_else(|_| "xc".into());
    let password = std::env::var("RUSTSSH_TEST_SSH_PASSWORD").unwrap_or_default();

    let mut session = SshSession::new();
    session.connect(&host, port, &user, &password).await?;

    // 连接建立后不一定立刻有可读数据；此处只验证：
    // - `read_stream` 调用路径可达
    // - 在“可能无数据可读”的情况下能快速返回而不是 panic/阻塞
    let mut buf = [0u8; 1024];
    let _ = session.read_stream(&mut buf)?;

    Ok(())
}
