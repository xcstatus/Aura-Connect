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
                Some(sftp) => match sftp.read_dir(&path).await {
                    Ok(entries) => Message::SftpDirLoaded { tab_index, entries },
                    Err(e) => Message::SftpDirError {
                        tab_index,
                        error: e.to_string(),
                    },
                },
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
            return handle_sftp_download(state, tab_index, path);
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
        SftpTabMessage::SftpDeleteConfirm => {
            return handle_sftp_delete_confirm(state, tab_index);
        }
        SftpTabMessage::SftpDeleteCancel => {
            handle_sftp_delete_cancel(state, tab_index);
        }
        SftpTabMessage::SftpCreateFolderConfirm => {
            return handle_sftp_create_folder_confirm(state, tab_index);
        }
        SftpTabMessage::SftpCreateFolderCancel => {
            handle_sftp_create_folder_cancel(state, tab_index);
        }
        SftpTabMessage::SftpCreateFolderNameChanged(name) => {
            handle_sftp_create_folder_name_changed(state, tab_index, name);
        }
        SftpTabMessage::SftpUpload => {
            return handle_sftp_upload(state, tab_index);
        }
        SftpTabMessage::SftpToggleTransferList => {
            handle_sftp_toggle_transfer_list(state, tab_index);
        }
        SftpTabMessage::SftpOpenDownloadDir => {
            return handle_sftp_open_download_dir(state, tab_index);
        }
        SftpTabMessage::SftpClearCompletedTransfers => {
            handle_sftp_clear_completed_transfers(state, tab_index);
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
        pane.pane_layout = PaneLayout::TerminalAboveSftp {
            sftp_ratio: layout::SFTP_PANEL_DEFAULT_RATIO,
        };

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
                Ok(sftp_session) => Message::SftpInitComplete {
                    tab_index: tab_index_clone,
                    session: std::sync::Arc::new(std::sync::Mutex::new(Some(sftp_session))),
                },
                Err(e) => Message::SftpInitError {
                    tab_index: tab_index_clone,
                    error: e.to_string(),
                },
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
            pane.pane_layout = PaneLayout::TerminalAboveSftp {
                sftp_ratio: layout::SFTP_PANEL_DEFAULT_RATIO,
            };
        }
        PaneLayout::TerminalAboveSftp { .. } => {
            pane.pane_layout = PaneLayout::SftpBesideTerminal {
                sftp_ratio: layout::SFTP_PANEL_SIDE_RATIO,
            };
        }
        PaneLayout::SftpBesideTerminal { .. } => {
            pane.pane_layout = PaneLayout::TerminalAboveSftp {
                sftp_ratio: layout::SFTP_PANEL_DEFAULT_RATIO,
            };
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
            let path = if parent_str.is_empty() {
                "/".to_string()
            } else {
                parent_str
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
            return start_async_dir_load(tab_index, path, sftp.session.clone());
        }
    }

    Task::none()
}

/// 下载文件
fn handle_sftp_download(state: &mut IcedState, tab_index: usize, path: String) -> Task<Message> {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return Task::none(),
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return Task::none(),
    };

    let session = match sftp.session.clone() {
        Some(s) => s,
        None => return Task::none(),
    };

    // 获取文件大小（用于传输列表显示）
    let file_size = sftp.entries.iter()
        .find(|e| e.path.to_string_lossy() == path)
        .map(|e| e.size)
        .unwrap_or(0);

    let file_name = std::path::Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "download".to_string());

    // 添加传输任务到列表
    let transfer = crate::sftp::SftpTransfer::new(
        crate::sftp::TransferDirection::Download,
        path.clone(),
        String::new(), // 将在对话框选择后更新
        file_size,
    );
    let transfer_id = transfer.id;
    sftp.transfers.push(transfer);
    sftp.show_transfer_list = true;

    Task::perform(
        async move {
            // 打开保存文件对话框
            let file = rfd::AsyncFileDialog::new()
                .set_title("保存文件")
                .set_file_name(file_name.clone())
                .save_file()
                .await;

            let local_path = match file {
                Some(f) => f.path().to_path_buf(),
                None => return Message::Noop,
            };

            let sftp_session = {
                let guard = session.lock().unwrap();
                guard.clone()
            };

            match sftp_session {
                Some(sftp) => {
                    // 更新传输任务的目标路径和状态
                    // 注意：这里需要通过消息来更新状态，因为我们在异步上下文中
                    match sftp.download_file(&path, &local_path).await {
                        Ok(()) => {
                            log::info!(target: "sftp", "Download complete: {}", file_name);
                            Message::SftpTransferComplete { tab_index, transfer_id }
                        }
                        Err(e) => Message::SftpTransferFailed {
                            tab_index,
                            transfer_id,
                            error: format!("下载失败: {}", e),
                        },
                    }
                }
                None => Message::SftpTransferFailed {
                    tab_index,
                    transfer_id,
                    error: "SFTP session not initialized".into(),
                },
            }
        },
        |msg| msg,
    )
}

/// 下载文件夹
fn handle_sftp_download_folder(state: &mut IcedState, tab_index: usize, path: String) {
    let _ = state;
    let _ = tab_index;

    // TODO: 触发系统文件夹选择对话框并开始递归下载
    log::info!(target: "sftp", "Download folder: {}", sanitize_path(&path));
}

/// 删除文件或文件夹（显示确认对话框）
fn handle_sftp_delete(state: &mut IcedState, tab_index: usize, path: String) {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return,
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return,
    };

    sftp.pending_delete = Some(path);
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

    // 如果点击的是当前已激活的排序列，切换排序方向
    // 如果点击的是其他列，设为该列升序
    if sftp.sort_by == sort_by {
        sftp.sort_direction = sftp.sort_direction.toggle();
    } else {
        sftp.sort_by = sort_by;
        sftp.sort_direction = crate::sftp::SortDirection::Ascending;
    }
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
pub fn handle_sftp_refresh(state: &mut IcedState, tab_index: usize) -> Task<Message> {
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

/// 确认删除
fn handle_sftp_delete_confirm(state: &mut IcedState, tab_index: usize) -> Task<Message> {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return Task::none(),
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return Task::none(),
    };

    let path = match sftp.pending_delete.take() {
        Some(p) => p,
        None => return Task::none(),
    };

    let session = match sftp.session.clone() {
        Some(s) => s,
        None => return Task::none(),
    };

    Task::perform(
        async move {
            let sftp_session = {
                let guard = session.lock().unwrap();
                guard.clone()
            };

            match sftp_session {
                Some(sftp) => {
                    // 先尝试作为文件删除，失败则尝试作为目录删除
                    if let Err(e) = sftp.remove_file(&path).await {
                        if let Err(e2) = sftp.remove_dir(&path).await {
                            Message::SftpDirError {
                                tab_index,
                                error: format!("删除失败: {}", e2),
                            }
                        } else {
                            Message::SftpRefreshComplete { tab_index }
                        }
                    } else {
                        Message::SftpRefreshComplete { tab_index }
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

/// 取消删除
fn handle_sftp_delete_cancel(state: &mut IcedState, tab_index: usize) {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return,
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return,
    };

    sftp.pending_delete = None;
}

/// 创建文件夹（显示输入对话框）
fn handle_sftp_create_folder(state: &mut IcedState, tab_index: usize) {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return,
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return,
    };

    sftp.creating_folder = true;
    sftp.new_folder_name = String::new();
}

/// 确认创建文件夹
fn handle_sftp_create_folder_confirm(state: &mut IcedState, tab_index: usize) -> Task<Message> {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return Task::none(),
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return Task::none(),
    };

    let folder_name = sftp.new_folder_name.trim().to_string();
    if folder_name.is_empty() {
        sftp.creating_folder = false;
        return Task::none();
    }

    let current_path = sftp.current_path.clone();
    let session = sftp.session.clone();
    sftp.creating_folder = false;
    sftp.new_folder_name = String::new();

    let session = match session {
        Some(s) => s,
        None => return Task::none(),
    };

    let full_path = if current_path == "/" {
        format!("/{}", folder_name)
    } else {
        format!("{}/{}", current_path, folder_name)
    };

    Task::perform(
        async move {
            let sftp_session = {
                let guard = session.lock().unwrap();
                guard.clone()
            };

            match sftp_session {
                Some(mut sftp) => match sftp.make_dir(&full_path).await {
                    Ok(()) => Message::SftpRefreshComplete { tab_index },
                    Err(e) => Message::SftpDirError {
                        tab_index,
                        error: format!("创建文件夹失败: {}", e),
                    },
                },
                None => Message::SftpDirError {
                    tab_index,
                    error: "SFTP session not initialized".into(),
                },
            }
        },
        |msg| msg,
    )
}

/// 取消创建文件夹
fn handle_sftp_create_folder_cancel(state: &mut IcedState, tab_index: usize) {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return,
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return,
    };

    sftp.creating_folder = false;
    sftp.new_folder_name = String::new();
}

/// 新建文件夹名称变更
fn handle_sftp_create_folder_name_changed(
    state: &mut IcedState,
    tab_index: usize,
    name: String,
) {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return,
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return,
    };

    sftp.new_folder_name = name;
}

/// 上传文件
fn handle_sftp_upload(state: &mut IcedState, tab_index: usize) -> Task<Message> {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return Task::none(),
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return Task::none(),
    };

    let session = match sftp.session.clone() {
        Some(s) => s,
        None => return Task::none(),
    };

    let current_path = sftp.current_path.clone();

    // 使用 rfd 打开文件选择对话框
    Task::perform(
        async move {
            let file = rfd::AsyncFileDialog::new()
                .set_title("选择要上传的文件")
                .pick_file()
                .await;

            let local_path = match file {
                Some(f) => f.path().to_path_buf(),
                None => return Message::Noop,
            };

            let file_name = match local_path.file_name() {
                Some(n) => n.to_string_lossy().to_string(),
                None => return Message::SftpDirError {
                    tab_index,
                    error: "无法获取文件名".into(),
                },
            };

            let remote_path = if current_path == "/" {
                format!("/{}", file_name)
            } else {
                format!("{}/{}", current_path, file_name)
            };

            let sftp_session = {
                let guard = session.lock().unwrap();
                guard.clone()
            };

            match sftp_session {
                Some(sftp) => match sftp.upload_file(&local_path, &remote_path).await {
                    Ok(()) => {
                        log::info!(target: "sftp", "Upload complete: {}", file_name);
                        Message::SftpRefreshComplete { tab_index }
                    }
                    Err(e) => Message::SftpDirError {
                        tab_index,
                        error: format!("上传失败: {}", e),
                    },
                },
                None => Message::SftpDirError {
                    tab_index,
                    error: "SFTP session not initialized".into(),
                },
            }
        },
        |msg| msg,
    )
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

/// 切换传输列表显示
fn handle_sftp_toggle_transfer_list(state: &mut IcedState, tab_index: usize) {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return,
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return,
    };

    sftp.show_transfer_list = !sftp.show_transfer_list;
}

/// 打开下载目录
fn handle_sftp_open_download_dir(state: &mut IcedState, tab_index: usize) -> Task<Message> {
    let pane = match state.tab_panes.get(tab_index) {
        Some(p) => p,
        None => return Task::none(),
    };

    let sftp = match &pane.sftp_panel {
        Some(s) => s,
        None => return Task::none(),
    };

    // 获取下载目录路径
    let download_dir = if !sftp.local_download_path.is_empty() {
        std::path::PathBuf::from(&sftp.local_download_path)
    } else if let Some(dir) = dirs::download_dir() {
        dir
    } else {
        return Task::none();
    };

    // 使用系统文件管理器打开目录
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg(download_dir.to_string_lossy().to_string())
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(download_dir.to_string_lossy().to_string())
            .spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer")
            .arg(download_dir.to_string_lossy().to_string())
            .spawn();
    }

    Task::none()
}

/// 清除已完成的传输任务
fn handle_sftp_clear_completed_transfers(state: &mut IcedState, tab_index: usize) {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return,
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return,
    };

    sftp.transfers.retain(|t| !t.status.is_terminal());
}

/// 处理传输完成
pub fn handle_sftp_transfer_complete(
    state: &mut IcedState,
    tab_index: usize,
    transfer_id: uuid::Uuid,
) {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return,
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return,
    };

    if let Some(transfer) = sftp.transfers.iter_mut().find(|t| t.id == transfer_id) {
        transfer.complete();
    }
}

/// 处理传输失败
pub fn handle_sftp_transfer_failed(
    state: &mut IcedState,
    tab_index: usize,
    transfer_id: uuid::Uuid,
    error: String,
) {
    let pane = match state.tab_panes.get_mut(tab_index) {
        Some(p) => p,
        None => return,
    };

    let sftp = match &mut pane.sftp_panel {
        Some(s) => s,
        None => return,
    };

    if let Some(transfer) = sftp.transfers.iter_mut().find(|t| t.id == transfer_id) {
        transfer.fail(error);
    }
}
