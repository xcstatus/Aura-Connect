//! SFTP 消息处理模块
//!
//! 处理所有 SFTP 相关的用户交互和状态更新。

use std::path::Path;

use iced::Task;

use crate::app::message::Message;
use crate::app::message::SftpTabMessage;
use crate::app::state::{IcedState, PaneLayout, SftpContextMenuState, SftpPanel};
use crate::sftp::SftpSession;
use crate::theme::layout;

/// 脱敏路径，仅保留文件名
fn sanitize_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "***".to_string())
}

/// 启动异步目录加载任务
fn start_async_dir_load(
    tab_index: usize,
    path: String,
    session: Option<std::sync::Arc<std::sync::Mutex<Option<SftpSession>>>>,
) -> Task<Message> {
    let session = match session {
        Some(s) => s,
        None => {
            return Task::none();
        }
    };

    // 先同步获取 session 的克隆（用于异步操作）
    // 注意：这里先锁住，克隆需要异步操作的句柄，然后释放锁
    // 由于 SftpSession::read_dir 需要 &self，我们需要在同一把锁内完成所有操作
    // 但 std::sync::Mutex 不是 Send，所以需要换一种方式

    // 方案：使用 tokio::sync::Mutex 或者将整个 SftpSession 克隆出来
    // 这里采用克隆 SftpSession 的方式（SftpSession 内部使用 Arc，只需要克隆 Arc）
    let sftp_session_arc = {
        let guard = session.lock().unwrap();
        guard.clone()
    };

    Task::perform(
        async move {
            // 现在 sftp_session_arc 是 Option<SftpSession>，不持有任何锁
            match sftp_session_arc {
                Some(sftp) => {
                    match sftp.read_dir(&path).await {
                        Ok(entries) => Message::SftpDirLoaded { tab_index, entries },
                        Err(e) => Message::SftpDirError { tab_index, error: e.to_string() },
                    }
                }
                None => Message::SftpDirError {
                    tab_index,
                    error: "SFTP session not initialized".into(),
                },
            }
        },
        |msg| msg,
    )
}

/// 处理 SFTP 标签消息
pub fn handle(state: &mut IcedState, msg: SftpTabMessage) -> Task<Message> {
    let tab_index = state.active_tab;

    match msg {
        SftpTabMessage::SftpToggle => {
            return handle_sftp_toggle(state, tab_index);
        }
        SftpTabMessage::SftpToggleLayout => {
            handle_sftp_toggle_layout(state, tab_index);
        }
        SftpTabMessage::SftpNavigate(path) => {
            return handle_sftp_navigate(state, tab_index, path);
        }
        SftpTabMessage::SftpNavigateToParent => {
            return handle_sftp_navigate_to_parent(state, tab_index);
        }
        SftpTabMessage::SftpDownload(path) => {
            handle_sftp_download(state, tab_index, path);
        }
        SftpTabMessage::SftpDownloadFolder(path) => {
            handle_sftp_download_folder(state, tab_index, path);
        }
        SftpTabMessage::SftpDelete(path) => {
            handle_sftp_delete(state, tab_index, path);
        }
        SftpTabMessage::SftpCreateFolder => {
            handle_sftp_create_folder(state, tab_index);
        }
        SftpTabMessage::SftpCopyPath(path) => {
            return handle_sftp_copy_path(state, tab_index, path);
        }
        SftpTabMessage::SftpSortBy(sort_by) => {
            handle_sftp_sort_by(state, tab_index, sort_by);
        }
        SftpTabMessage::SftpSortDirection(direction) => {
            handle_sftp_sort_direction(state, tab_index, direction);
        }
        SftpTabMessage::SftpToggleHidden => {
            handle_sftp_toggle_hidden(state, tab_index);
        }
        SftpTabMessage::SftpRefresh => {
            return handle_sftp_refresh(state, tab_index);
        }
        SftpTabMessage::ShowContextMenu { x, y, target } => {
            handle_show_context_menu(state, tab_index, x, y, target);
        }
        SftpTabMessage::HideContextMenu => {
            handle_hide_context_menu(state, tab_index);
        }
    }

    Task::none()
}

/// 切换 SFTP 面板显隐
fn handle_sftp_toggle(state: &mut IcedState, tab_index: usize) -> Task<Message> {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return Task::none(),
    };

    if pane.sftp_panel.is_some() {
        // 关闭面板
        pane.sftp_panel = None;
        pane.pane_layout = PaneLayout::TerminalOnly;
        Task::none()
    } else {
        // 打开面板（使用 Default）
        pane.sftp_panel = Some(SftpPanel::default());
        pane.pane_layout =
            PaneLayout::TerminalAboveSftp { sftp_ratio: layout::SFTP_PANEL_DEFAULT_RATIO };

        // 尝试初始化 SFTP session 并加载根目录
        init_sftp_and_load_root(state, tab_index)
    }
}

/// 初始化 SFTP session 并加载根目录
pub fn init_sftp_and_load_root(state: &mut IcedState, tab_index: usize) -> Task<Message> {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return Task::none(),
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return Task::none(),
    };

    // 如果已有 session，直接加载
    if let Some(ref session) = sftp.session {
        let tab_index_clone = tab_index;
        let path = sftp.current_path.clone();
        sftp.is_loading = true;
        return start_async_dir_load(tab_index_clone, path, Some(session.clone()));
    }

    // 获取 SSH channel 并创建 SFTP session
    let ssh_channel = match state.tab_manager.get_channel(tab_index) {
        Some(ch) => ch,
        None => {
            sftp.is_loading = false;
            sftp.error = Some("SSH session not connected".into());
            return Task::none();
        }
    };

    // 获取 BaseSshConnection
    let base_conn = ssh_channel.base_connection();
    let tab_index_clone = tab_index;

    Task::perform(
        async move {
            match crate::sftp::SftpSession::new(base_conn).await {
                Ok(sftp_session) => {
                    Message::SftpInitComplete {
                        tab_index: tab_index_clone,
                        session: std::sync::Arc::new(std::sync::Mutex::new(Some(sftp_session))),
                    }
                }
                Err(e) => {
                    Message::SftpInitError {
                        tab_index: tab_index_clone,
                        error: e.to_string(),
                    }
                }
            }
        },
        |msg| msg,
    )
}

/// 切换布局（上下 ↔ 左右）
fn handle_sftp_toggle_layout(state: &mut IcedState, tab_index: usize) {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return,
    };

    if pane.sftp_panel.is_none() {
        return;
    }

    match pane.pane_layout {
        PaneLayout::TerminalOnly => {
            pane.pane_layout =
                PaneLayout::TerminalAboveSftp { sftp_ratio: layout::SFTP_PANEL_DEFAULT_RATIO };
        }
        PaneLayout::TerminalAboveSftp { .. } => {
            pane.pane_layout =
                PaneLayout::SftpBesideTerminal { sftp_ratio: layout::SFTP_PANEL_SIDE_RATIO };
        }
        PaneLayout::SftpBesideTerminal { .. } => {
            pane.pane_layout =
                PaneLayout::TerminalAboveSftp { sftp_ratio: layout::SFTP_PANEL_DEFAULT_RATIO };
        }
    }
}

/// 进入目录
fn handle_sftp_navigate(state: &mut IcedState, tab_index: usize, path: String) -> Task<Message> {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return Task::none(),
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return Task::none(),
    };

    // 保存当前路径到历史
    sftp.path_history.push(sftp.current_path.clone());

    // 更新当前路径
    sftp.current_path = path.clone();

    // 清空当前列表并标记加载状态
    sftp.entries.clear();
    sftp.is_loading = true;
    sftp.error = None;

    // 启动异步加载
    start_async_dir_load(tab_index, path, sftp.session.clone())
}

/// 返回上级目录
fn handle_sftp_navigate_to_parent(state: &mut IcedState, tab_index: usize) -> Task<Message> {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return Task::none(),
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return Task::none(),
    };

    // 获取父目录路径
    if let Some(parent) = std::path::Path::new(&sftp.current_path).parent() {
        let parent_str = parent.to_string_lossy().to_string();
        if parent_str != sftp.current_path {
            let path = if parent_str.is_empty() { "/".to_string() } else { parent_str };

            // 保存当前路径到历史
            sftp.path_history.push(sftp.current_path.clone());

            // 更新当前路径
            sftp.current_path = path.clone();

            // 清空列表并标记加载状态
            sftp.entries.clear();
            sftp.is_loading = true;
            sftp.error = None;

            // 启动异步加载
            return start_async_dir_load(tab_index, path, sftp.session.clone());
        }
    }

    Task::none()
}

/// 下载文件
fn handle_sftp_download(state: &mut IcedState, tab_index: usize, path: String) {
    let _ = state;
    let _ = tab_index;

    // TODO: 触发系统文件保存对话框并开始下载
    log::info!(target: "sftp", "Download file: {}", sanitize_path(&path));
}

/// 下载文件夹
fn handle_sftp_download_folder(state: &mut IcedState, tab_index: usize, path: String) {
    let _ = state;
    let _ = tab_index;

    // TODO: 触发系统文件夹选择对话框并开始递归下载
    log::info!(target: "sftp", "Download folder: {}", sanitize_path(&path));
}

/// 删除文件或文件夹
fn handle_sftp_delete(state: &mut IcedState, tab_index: usize, path: String) {
    let _ = state;
    let _ = tab_index;

    // TODO: 显示确认对话框并执行删除
    log::info!(target: "sftp", "Delete: {}", sanitize_path(&path));
}

/// 创建文件夹
fn handle_sftp_create_folder(state: &mut IcedState, tab_index: usize) {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return,
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return,
    };

    // TODO: 显示输入对话框并创建文件夹
    log::info!(target: "sftp", "Create folder in: {}", sanitize_path(&sftp.current_path));
}

/// 复制路径到剪贴板
fn handle_sftp_copy_path(state: &mut IcedState, tab_index: usize, path: String) -> Task<Message> {
    use iced::clipboard;

    let _ = state;
    let _ = tab_index;
    log::info!(target: "sftp", "Copy path: {}", sanitize_path(&path));

    // 将路径复制到剪贴板
    clipboard::write(path).map(|_: ()| Message::ClipboardWriteDone)
}

/// 排序字段变更
fn handle_sftp_sort_by(state: &mut IcedState, tab_index: usize, sort_by: crate::sftp::SftpSortBy) {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return,
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return,
    };

    sftp.sort_by = sort_by;
}

/// 排序方向变更
fn handle_sftp_sort_direction(
    state: &mut IcedState,
    tab_index: usize,
    direction: crate::sftp::SortDirection,
) {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return,
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return,
    };

    sftp.sort_direction = direction;
}

/// 切换隐藏文件显示
fn handle_sftp_toggle_hidden(state: &mut IcedState, tab_index: usize) {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return,
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return,
    };

    sftp.show_hidden = !sftp.show_hidden;
}

/// 刷新目录
fn handle_sftp_refresh(state: &mut IcedState, tab_index: usize) -> Task<Message> {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return Task::none(),
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return Task::none(),
    };

    sftp.entries.clear();
    sftp.is_loading = true;
    sftp.error = None;

    let path = sftp.current_path.clone();
    log::info!(target: "sftp", "Refresh: {}", sanitize_path(&path));

    // 启动异步加载
    start_async_dir_load(tab_index, path, sftp.session.clone())
}

/// 显示右键菜单
fn handle_show_context_menu(
    state: &mut IcedState,
    tab_index: usize,
    x: f32,
    y: f32,
    target: crate::app::message::SftpContextMenuTarget,
) {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return,
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return,
    };

    sftp.context_menu = Some(SftpContextMenuState { x, y, target });
}

/// 隐藏右键菜单
fn handle_hide_context_menu(state: &mut IcedState, tab_index: usize) {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return,
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return,
    };

    sftp.context_menu = None;
}

// ============================================================================
// 异步回调处理函数（由 update/mod.rs 调用）
// ============================================================================

/// 处理 SFTP session 初始化成功
pub fn handle_sftp_init_complete(
    state: &mut IcedState,
    tab_index: usize,
    session: std::sync::Arc<std::sync::Mutex<Option<SftpSession>>>,
) -> Task<Message> {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return Task::none(),
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return Task::none(),
    };

    // 存储 session
    sftp.session = Some(session);
    sftp.is_loading = true;
    sftp.error = None;

    // 加载根目录
    let path = sftp.current_path.clone();
    start_async_dir_load(tab_index, path, sftp.session.clone())
}

/// 处理 SFTP session 初始化失败
pub fn handle_sftp_init_error(state: &mut IcedState, tab_index: usize, error: String) {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return,
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return,
    };

    sftp.is_loading = false;
    sftp.error = Some(error);
}

/// 处理目录加载成功
pub fn handle_sftp_dir_loaded(
    state: &mut IcedState,
    tab_index: usize,
    entries: Vec<crate::sftp::RemoteFileEntry>,
) {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return,
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return,
    };

    sftp.entries = entries;
    sftp.is_loading = false;
    sftp.error = None;
}

/// 处理目录加载失败
pub fn handle_sftp_dir_error(state: &mut IcedState, tab_index: usize, error: String) {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return,
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return,
    };

    sftp.is_loading = false;
    sftp.error = Some(error);
}
