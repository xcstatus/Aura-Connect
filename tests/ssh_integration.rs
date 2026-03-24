use rust_ssh::backend::ssh_session::{SshSession, AsyncSession};
use tokio;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("--- RustSSH 协议层验证工具 ---");
    
    // 请根据您的本地测试环境修改以下参数
    let host = "127.0.0.1";
    let port = 22;
    let user = "xc";
    let password = ""; // 建议使用 SSH Key 或本地测试环境

    println!("正在尝试连接到 {}@{}...", user, host);
    
    let mut session = SshSession::new();
    match session.connect(host, port, user, password).await {
        Ok(_) => println!("✅ 连接并认证成功！"),
        Err(e) => {
            println!("❌ 连接失败: {}", e);
            return Ok(());
        }
    }

    println!("正在启动读写监听 (按 Ctrl+C 退出)...");
    
    let mut session_arc = session;
    
    // 启动一个简单的输出打印任务
    tokio::spawn(async move {
        let mut buf = [0u8; 1024];
        loop {
            if let Ok(n) = session_arc.read_stream(&mut buf).await {
                if n > 0 {
                    print!("{}", String::from_utf8_lossy(&buf[..n]));
                    io::stdout().flush().unwrap();
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    });

    // 可以在这里添加交互式输入模拟，或者直接等待
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    println!("\n验证完成，会话正常。");
    
    Ok(())
}
