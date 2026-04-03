# Iced UI 架构与异步限制

## 概述

本项目从 egui 迁移到 iced 作为 UI 框架。本文档记录架构决策和已知的限制。

## 当前架构

### 技术栈

| 组件 | 选型 | 说明 |
|------|------|------|
| **GUI 框架** | iced | 声明式 GUI，支持跨平台 |
| **终端引擎** | libghostty | GPU 纹理共享给 iced 渲染 |
| **SSH 协议** | russh | 纯 Rust 异步 SSH2 |
| **异步运行时** | tokio | 多线程异步 I/O |

### IcedState 结构

```rust
pub(crate) struct IcedState {
    pub model: AppModel,
    pub rt: tokio::runtime::Runtime,  // 用于文件 IO 和 SSH 连接
    pub tabs: Vec<IcedTab>,
    pub tab_panes: Vec<TabPane>,     // 包含 TerminalController
    // ... 其他字段
}
```

### TerminalController 嵌套链

```
IcedState
  └── tab_panes: Vec<TabPane>
        └── TerminalController
              └── GhosttyVtTerminal (包含 *mut GhosttyTerminal)
```

## 已知的架构限制

### 1. Task::perform 与 Send 约束

**问题**：尝试使用 `iced::Task::perform` 实现非阻塞 SSH 连接失败。

**原因**：
- `Task::perform` 的回调闭包必须是 `Send`
- `IcedState` 包含 `TabPane` → `TerminalController` → `GhosttyVtTerminal` → `*mut GhosttyTerminal`
- 裸指针 `*mut T` 不是 `Send`
- 因此 `&mut IcedState` 无法传递给 `Task::perform` 的回调

**影响**：
- SSH 连接必须使用 `rt.block_on` 同步执行
- UI 在连接过程中可能短暂阻塞
- 无法真正实现异步非阻塞的连接体验

## 解决方案讨论

### 方案 A：分离状态（推荐）

将 SSH 会话状态从 `IcedState` 中分离到独立的 `Arc<SessionState>`：

```rust
// 独立的会话状态，可以是 Send + Sync
pub struct SessionState {
    pub session: Option<Box<dyn AsyncSession>>,
    pub connecting: bool,
    pub error: Option<ConnectErrorKind>,
}

// IcedState 持有 Arc<SessionState>
pub(crate) struct IcedState {
    // ...
    pub session_state: Arc<RwLock<SessionState>>,
}
```

**优点**：
- 可以将 `Arc<RwLock<SessionState>>` 传递给 `Task::perform`
- UI 状态与 SSH 状态解耦

**缺点**：
- 需要大量重构
- 状态同步复杂度增加

### 方案 B：使用 tokio::spawn + channel

在 `boot()` 中创建 channel，将异步任务 spawn 到 tokio runtime，结果通过 channel 传回：

```rust
pub(crate) fn boot() -> (IcedState, Task<Message>) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime for iced app");

    // 创建 channel
    let (tx, rx) = tokio::sync::mpsc::channel(1);

    // Spawn 异步任务
    rt.spawn(async move {
        // SSH 连接逻辑
        let result = connect().await;
        let _ = tx.send(result).await;
    });

    // 返回 IcedState 和监听 channel 的 Task
    (state, listener_task(rx))
}
```

**优点**：
- 保持 UI 响应性
- 不需要重构整个状态结构

**缺点**：
- 需要管理 runtime 和 channel 生命周期
- 增加异步复杂度

### 方案 C：保持现状

保留 `rt.block_on` 用于 SSH 连接，因为：
- 连接操作通常是短暂的（几秒内完成）
- 文件 IO 操作非常快
- Interactive Auth 需要同步状态

**优点**：
- 代码简单，维护成本低
- 符合当前 MVP 需求

**缺点**：
- UI 在连接时可能轻微阻塞
- 不符合最佳异步实践

## 当前实现状态

| 操作 | 方式 | 说明 |
|------|------|------|
| 标准 SSH 连接 | `rt.block_on` | 同步执行，保持原行为 |
| Interactive Auth | `rt.block_on` | 需要多步骤状态 |
| Session Manager IO | `rt.block_on` | 文件操作，极快 |
| PTY pumping | tick 循环 | 每帧调用，不阻塞 UI |

## 性能影响评估

- **UI 阻塞时间**：SSH 连接通常在 1-5 秒内完成，UI 不会完全冻结
- **用户体验**：连接时显示 "Connecting..." 状态，用户可见进度
- **资源占用**：tokio runtime 持续运行，但开销很小

## 结论

对于 MVP 阶段，当前实现是可接受的。如果未来需要更好的用户体验（真正异步连接），建议采用**方案 A：分离状态**。

## 文档更新历史

- 2026-04-03：初始文档，记录 Task::perform 限制
