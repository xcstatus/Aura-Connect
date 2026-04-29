/// 设置中心左侧大类（与 egui `SettingTab` 对齐）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum SettingsCategory {
    #[default]
    General,
    ColorScheme,
    Terminal,
    Connection,
    Security,
    Backup,
}

/// 设置项变更（写入 `Settings` 并 `save`，部分项同步 i18n / 重启提示）。
#[derive(Debug, Clone)]
pub(crate) enum SettingsField {
    Language(String),
    AutoCheckUpdate(bool),
    AccentColor(String),
    FontSize(f32),
    TargetFps(u32),
    /// 配色方案 ID
    ColorScheme(String),
    /// Terminal text size (logical px); affects PTY grid when metrics are applied.
    TerminalFontSize(f32),
    LineHeight(f32),
    FontFamily(String),
    /// When true, `font_size` / `line_height` drive PTY cols/rows and Iced terminal rendering.
    ApplyTerminalMetrics(bool),
    RightClickPaste(bool),
    BracketedPaste(bool),
    KeepSelectionHighlight(bool),
    ScrollbackLimit(usize),
    HistorySearch(bool),
    PathCompletion(bool),
    IdleTimeoutMins(u32),
    LockOnSleep(bool),
    /// Vault KDF 内存级别
    KdfMemoryLevel(crate::settings::KdfMemoryLevel),
    HostKeyPolicy(crate::settings::HostKeyPolicy),
    /// 删除已信任主机
    DeleteKnownHost {
        host: String,
        port: u16,
    },
    /// 展开/折叠主机详情
    ToggleKnownHostDetail {
        host: String,
        port: u16,
    },
    ConnectionSearch(String),
    /// When true: only one tab may hold SSH; switching tabs disconnects the previous session.
    SingleSharedSession(bool),
    /// 连接断开时自动重连
    AutoReconnect(bool),
    /// 最大重连次数
    ReconnectMaxAttempts(u8),
    /// 重连基础延迟秒数
    ReconnectBaseDelay(u32),
    /// 是否使用指数退避
    ReconnectExponential(bool),
    /// 启动时恢复上次会话
    RestoreLastSession(bool),
    /// 添加端口转发配置
    AddPortForward,
    /// 删除端口转发配置
    RemovePortForward(usize),
}

/// Type alias for connect result error info (error kind + optional host key error details).
pub type ConnectResultError = (
    crate::app::model::ConnectErrorKind,
    Option<crate::app::model::HostKeyErrorInfo>,
);

/// SFTP 右键菜单目标
#[derive(Debug, Clone)]
pub enum SftpContextMenuTarget {
    /// 文件
    File {
        name: String,
        path: String,
        is_dir: bool,
    },
    /// 空白区域
    EmptyArea,
}

/// SFTP 面板标签级消息
#[derive(Debug, Clone)]
pub enum SftpTabMessage {
    /// 切换面板显隐
    SftpToggle,
    /// 切换布局（上下 ↔ 左右）
    SftpToggleLayout,
    /// 进入目录
    SftpNavigate(String),
    /// 返回上级目录
    SftpNavigateToParent,
    /// 下载文件
    SftpDownload(String),
    /// 下载文件夹
    SftpDownloadFolder(String),
    /// 删除文件/文件夹
    SftpDelete(String),
    /// 创建文件夹
    SftpCreateFolder,
    /// 复制路径到剪贴板
    SftpCopyPath(String),
    /// 排序字段变更
    SftpSortBy(crate::sftp::SftpSortBy),
    /// 排序方向变更
    SftpSortDirection(crate::sftp::SortDirection),
    /// 切换隐藏文件显示
    SftpToggleHidden,
    /// 刷新目录
    SftpRefresh,
    /// 显示右键菜单
    ShowContextMenu {
        x: f32,
        y: f32,
        target: SftpContextMenuTarget,
    },
    /// 隐藏右键菜单
    HideContextMenu,
    /// 确认删除
    SftpDeleteConfirm,
    /// 取消删除
    SftpDeleteCancel,
    /// 确认创建文件夹
    SftpCreateFolderConfirm,
    /// 取消创建文件夹
    SftpCreateFolderCancel,
    /// 新建文件夹名称变更
    SftpCreateFolderNameChanged(String),
    /// 上传文件
    SftpUpload,
    /// 切换传输列表显示
    SftpToggleTransferList,
    /// 打开下载目录
    SftpOpenDownloadDir,
    /// 清除已完成的传输任务
    SftpClearCompletedTransfers,
}

/// Wrapped session type for async connection result.
pub type ConnectSession = std::sync::Arc<crate::backend::ssh_session::SshChannel>;

/// Message type - manually implement Debug since AsyncSession doesn't implement it.
#[derive(Clone)]
pub(crate) enum Message {
    /// 无操作（用于异步任务取消等场景）
    Noop,
    Tick,
    /// 主窗口尺寸（逻辑像素），用于判断标签条是否横向溢出。
    WindowResized(iced::Size),
    EventOccurred(iced::event::Event),
    TopAddTab,
    TopQuickConnect,
    /// 保存快速连接表单中的连接信息为会话。
    QuickConnectSaveSession,
    /// 关闭快速连接弹窗（遮罩、标题栏 ×、Esc）。
    QuickConnectDismiss,
    /// 快速连接：进入新建连接表单。
    QuickConnectNewConnection,
    /// 从表单返回连接列表。
    QuickConnectBackToList,
    /// 快速连接：搜索已保存会话 / 输入直连字符串。
    QuickConnectQueryChanged(String),
    /// 快速连接：对当前输入执行"直连"动作（填入 Draft 并进入表单/或触发连接）。
    QuickConnectDirectSubmit,
    /// 点击「最近」条目：已保存则直连；否则填入草稿并打开表单。
    QuickConnectPickRecent(crate::settings::RecentConnectionRecord),
    TopOpenSettings,
    /// 关闭设置中心弹窗（遮罩、×、Esc）。
    SettingsDismiss,
    SettingsCategoryChanged(SettingsCategory),
    /// 当前大类下的顶部子页签索引。
    SettingsSubTabChanged(usize),
    SettingsFieldChanged(SettingsField),
    /// 生物识别开关（需系统校验成功后落盘）。
    BiometricsToggle(bool),
    /// 清除「部分设置需重启」提示条。
    SettingsRestartAcknowledged,
    /// 主密码 / 保险箱（Iced 仅提示，完整流程见 egui）。
    VaultOpen,
    VaultClose,
    VaultOldPasswordChanged(String),
    VaultNewPasswordChanged(String),
    VaultConfirmPasswordChanged(String),
    VaultSubmit,
    /// 从连接管理中删除已保存会话。
    DeleteSessionProfile(String),
    /// 会话编辑器：新建（None）/编辑（Some(profile_id)）。
    OpenSessionEditor(Option<String>),
    SessionEditorClose,
    SessionEditorHostChanged(String),
    SessionEditorPortChanged(String),
    SessionEditorUserChanged(String),
    SessionEditorAuthChanged(crate::session::AuthMethod),
    SessionEditorPasswordChanged(String),
    SessionEditorClearPasswordToggled(bool),
    SessionEditorNameChanged(String),
    SessionEditorGroupChanged(Option<String>),
    SessionEditorPrivateKeyPathChanged(String),
    SessionEditorPassphraseChanged(String),
    SessionEditorTestConnection,
    SessionEditorSave,
    TabSelected(usize),
    TabClose(usize),
    /// 标签芯片悬停：`None` 表示鼠标已离开当前芯片。
    TabChipHover(Option<usize>),
    /// 滚动条区域 hover：用于透明度动画过渡。
    ScrollbarHover(bool),
    /// 标签栏横向滚动回调：传递 scrollable 的当前水平偏移。
    TabScrollTick(f32),
    /// 滚动到指定标签索引（同时选中标签）。
    TabScrollTo(usize),
    /// 切换溢出菜单显隐。
    TabOverflowToggle,
    /// 标签栏区域：垂直滚轮映射为横向滚动（无需 Shift）。
    TabStripWheel(iced::mouse::ScrollDelta),
    #[cfg(not(target_os = "macos"))]
    WinClose,
    #[cfg(not(target_os = "macos"))]
    WinMinimize,
    #[cfg(not(target_os = "macos"))]
    WinToggleMaximize,
    HostChanged(String),
    PortChanged(String),
    UserChanged(String),
    PasswordChanged(String),
    QuickConnectAuthChanged(crate::session::AuthMethod),
    QuickConnectKeyPathChanged(String),
    QuickConnectPassphraseChanged(String),
    ConnectPressed,
    /// Breadcrumb 固定状态切换
    BreadcrumbTogglePin,
    /// Breadcrumb 临时显示（鼠标悬停浮动图标）
    BreadcrumbShowTemp,
    /// Breadcrumb 隐藏临时显示
    BreadcrumbHideTemp,
    /// 打开 SFTP 传输
    BreadcrumbSftp,
    /// 打开端口转发
    BreadcrumbPortForward,
    /// Connection result callback (triggered by Task::perform).
    /// Contains: Result<session, (error_kind, host_key_error_info)>
    ConnectResult(Result<ConnectSession, ConnectResultError>),
    /// Keyboard-interactive: update answer field.
    QuickConnectInteractiveAnswerChanged(usize, String),
    /// Keyboard-interactive: submit current answers (advance auth state machine).
    QuickConnectInteractiveSubmit,
    /// Inline password/passphrase submitted from the terminal overlay when the
    /// quick-connect modal is closed (e.g. saved-session needs a password).
    QuickConnectInlinePasswordSubmit(String),
    /// Inline password field changed (live update while typing).
    QuickConnectInlinePasswordChanged(String),
    /// Toggle inline password visibility.
    QuickConnectInlinePasswordToggleVisibility,
    /// Close inline password overlay (e.g., cancel auth).
    QuickConnectInlinePasswordClose,
    /// Host key confirmation (Ask policy).
    HostKeyAcceptOnce,
    HostKeyAlwaysTrust,
    HostKeyReject,
    AutoProbeConsentOpen,
    AutoProbeConsentAllowOnce,
    AutoProbeConsentAlwaysAllow,
    AutoProbeConsentUsePassword,
    /// 切换认证方式（Failed/AuthLocked 状态）：返回 NewConnection 表单让用户选择其他认证方式。
    QuickConnectSwitchAuth,
    DisconnectPressed,
    /// SSH 会话正常退出（exit）：自动关闭对应页签
    SessionExited(usize),
    ProfileConnect(crate::session::SessionProfile),
    VaultUnlockOpenConnect(crate::session::SessionProfile),
    VaultUnlockOpenDelete(String),
    VaultUnlockOpenSaveSession,
    VaultUnlockClose,
    VaultUnlockPasswordChanged(String),
    VaultUnlockSubmit,
    /// Async vault unlock result (callback from background KDF task).
    /// `(master_password, preloaded_credentials)` where preloaded_credentials is
    /// `(password, passphrase)` for the pending session, loaded during the same KDF pass.
    VaultUnlockComplete(
        Result<(String, Option<(String, Option<String>)>), crate::vault::VaultUnlockError>,
    ),
    /// Clipboard contents for terminal paste (bracketed when DEC 2004 is on).
    ClipboardPaste(Option<String>),
    /// Ack for clipboard write task (copy).
    ClipboardWriteDone,
    SaveSettings,
    /// Toggle the debug overlay visibility (Ctrl+Shift+D).
    ToggleDebugOverlay,
    /// 自动重连倒计时更新（每秒触发一次）
    ReconnectTick,
    /// 自动重连结果（异步任务回调）
    ReconnectResult(
        Result<
            std::sync::Arc<crate::backend::ssh_session::SshChannel>,
            (
                crate::app::model::ConnectErrorKind,
                Option<crate::app::model::HostKeyErrorInfo>,
            ),
        >,
    ),
    /// 用户手动取消重连
    ReconnectCancel,
    /// 重启后恢复会话弹窗：确认恢复
    RestoreSessionConfirm,
    /// 重启后恢复会话弹窗：忽略
    RestoreSessionDismiss,
    /// 会话项悬停：开始预热计时
    SessionHoverStart(String),
    /// 会话项悬停结束：取消预热
    SessionHoverEnd,
    /// 预热 tick（每 500ms 检查）
    PrewarmTick,
    /// 预热结果回调
    PrewarmResult(
        Result<
            std::sync::Arc<crate::backend::ssh_session::SshChannel>,
            (
                crate::app::model::ConnectErrorKind,
                Option<crate::app::model::HostKeyErrorInfo>,
            ),
        >,
    ),
    /// SFTP 标签消息
    SftpTab(SftpTabMessage),
    /// SFTP session 初始化完成
    SftpInitComplete {
        tab_index: usize,
        session: std::sync::Arc<std::sync::Mutex<Option<crate::sftp::SftpSession>>>,
    },
    /// SFTP session 初始化失败
    SftpInitError {
        tab_index: usize,
        error: String,
    },
    /// SFTP 目录加载完成回调（异步任务返回）
    SftpDirLoaded {
        tab_index: usize,
        entries: Vec<crate::sftp::RemoteFileEntry>,
    },
    /// SFTP 目录加载失败回调
    SftpDirError {
        tab_index: usize,
        error: String,
    },
    /// SFTP 操作完成，需要刷新目录
    SftpRefreshComplete {
        tab_index: usize,
    },
    /// SFTP 传输完成
    SftpTransferComplete {
        tab_index: usize,
        transfer_id: uuid::Uuid,
    },
    /// SFTP 传输失败
    SftpTransferFailed {
        tab_index: usize,
        transfer_id: uuid::Uuid,
        error: String,
    },
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Tick => write!(f, "Message::Tick"),
            Message::WindowResized(size) => write!(f, "Message::WindowResized({:?})", size),
            Message::EventOccurred(_) => write!(f, "Message::EventOccurred(...)"),
            Message::TopAddTab => write!(f, "Message::TopAddTab"),
            Message::TopQuickConnect => write!(f, "Message::TopQuickConnect"),
            Message::QuickConnectDismiss => write!(f, "Message::QuickConnectDismiss"),
            Message::QuickConnectNewConnection => write!(f, "Message::QuickConnectNewConnection"),
            Message::QuickConnectSaveSession => write!(f, "Message::QuickConnectSaveSession"),
            Message::QuickConnectBackToList => write!(f, "Message::QuickConnectBackToList"),
            Message::QuickConnectQueryChanged(q) => {
                write!(f, "Message::QuickConnectQueryChanged({})", q)
            }
            Message::QuickConnectDirectSubmit => write!(f, "Message::QuickConnectDirectSubmit"),
            Message::QuickConnectPickRecent(rec) => {
                write!(f, "Message::QuickConnectPickRecent({:?})", rec)
            }
            Message::TopOpenSettings => write!(f, "Message::TopOpenSettings"),
            Message::SettingsDismiss => write!(f, "Message::SettingsDismiss"),
            Message::SettingsCategoryChanged(cat) => {
                write!(f, "Message::SettingsCategoryChanged({:?})", cat)
            }
            Message::SettingsSubTabChanged(i) => write!(f, "Message::SettingsSubTabChanged({})", i),
            Message::SettingsFieldChanged(field) => {
                write!(f, "Message::SettingsFieldChanged({:?})", field)
            }
            Message::BiometricsToggle(v) => write!(f, "Message::BiometricsToggle({})", v),
            Message::SettingsRestartAcknowledged => {
                write!(f, "Message::SettingsRestartAcknowledged")
            }
            Message::VaultOpen => write!(f, "Message::VaultOpen"),
            Message::VaultClose => write!(f, "Message::VaultClose"),
            Message::VaultOldPasswordChanged(_) => {
                write!(f, "Message::VaultOldPasswordChanged(...)")
            }
            Message::VaultNewPasswordChanged(_) => {
                write!(f, "Message::VaultNewPasswordChanged(...)")
            }
            Message::VaultConfirmPasswordChanged(_) => {
                write!(f, "Message::VaultConfirmPasswordChanged(...)")
            }
            Message::VaultSubmit => write!(f, "Message::VaultSubmit"),
            Message::DeleteSessionProfile(id) => write!(f, "Message::DeleteSessionProfile({})", id),
            Message::OpenSessionEditor(id) => write!(f, "Message::OpenSessionEditor({:?})", id),
            Message::SessionEditorClose => write!(f, "Message::SessionEditorClose"),
            Message::SessionEditorHostChanged(v) => {
                write!(f, "Message::SessionEditorHostChanged({})", v)
            }
            Message::SessionEditorPortChanged(v) => {
                write!(f, "Message::SessionEditorPortChanged({})", v)
            }
            Message::SessionEditorUserChanged(v) => {
                write!(f, "Message::SessionEditorUserChanged({})", v)
            }
            Message::SessionEditorAuthChanged(a) => {
                write!(f, "Message::SessionEditorAuthChanged({:?})", a)
            }
            Message::SessionEditorPasswordChanged(_) => {
                write!(f, "Message::SessionEditorPasswordChanged(...)")
            }
            Message::SessionEditorClearPasswordToggled(v) => {
                write!(f, "Message::SessionEditorClearPasswordToggled({})", v)
            }
            Message::SessionEditorNameChanged(v) => {
                write!(f, "Message::SessionEditorNameChanged({})", v)
            }
            Message::SessionEditorGroupChanged(v) => {
                write!(f, "Message::SessionEditorGroupChanged({:?})", v)
            }
            Message::SessionEditorPrivateKeyPathChanged(v) => {
                write!(f, "Message::SessionEditorPrivateKeyPathChanged({})", v)
            }
            Message::SessionEditorPassphraseChanged(_) => {
                write!(f, "Message::SessionEditorPassphraseChanged(...)")
            }
            Message::SessionEditorTestConnection => {
                write!(f, "Message::SessionEditorTestConnection")
            }
            Message::SessionEditorSave => write!(f, "Message::SessionEditorSave"),
            Message::TabSelected(i) => write!(f, "Message::TabSelected({})", i),
            Message::TabClose(i) => write!(f, "Message::TabClose({})", i),
            Message::TabChipHover(i) => write!(f, "Message::TabChipHover({:?})", i),
            Message::ScrollbarHover(v) => write!(f, "Message::ScrollbarHover({})", v),
            Message::TabScrollTick(v) => write!(f, "Message::TabScrollTick({:.1})", v),
            Message::TabScrollTo(i) => write!(f, "Message::TabScrollTo({})", i),
            Message::TabOverflowToggle => write!(f, "Message::TabOverflowToggle"),
            Message::TabStripWheel(_) => write!(f, "Message::TabStripWheel(...)"),
            #[cfg(not(target_os = "macos"))]
            Message::WinClose => write!(f, "Message::WinClose"),
            #[cfg(not(target_os = "macos"))]
            Message::WinMinimize => write!(f, "Message::WinMinimize"),
            #[cfg(not(target_os = "macos"))]
            Message::WinToggleMaximize => write!(f, "Message::WinToggleMaximize"),
            Message::HostChanged(v) => write!(f, "Message::HostChanged({})", v),
            Message::PortChanged(v) => write!(f, "Message::PortChanged({})", v),
            Message::UserChanged(v) => write!(f, "Message::UserChanged({})", v),
            Message::PasswordChanged(_) => write!(f, "Message::PasswordChanged(...)"),
            Message::QuickConnectAuthChanged(a) => {
                write!(f, "Message::QuickConnectAuthChanged({:?})", a)
            }
            Message::QuickConnectKeyPathChanged(v) => {
                write!(f, "Message::QuickConnectKeyPathChanged({})", v)
            }
            Message::QuickConnectPassphraseChanged(_) => {
                write!(f, "Message::QuickConnectPassphraseChanged(...)")
            }
            Message::ConnectPressed => write!(f, "Message::ConnectPressed"),
            Message::BreadcrumbTogglePin => write!(f, "Message::BreadcrumbTogglePin"),
            Message::BreadcrumbShowTemp => write!(f, "Message::BreadcrumbShowTemp"),
            Message::BreadcrumbHideTemp => write!(f, "Message::BreadcrumbHideTemp"),
            Message::BreadcrumbSftp => write!(f, "Message::BreadcrumbSftp"),
            Message::BreadcrumbPortForward => write!(f, "Message::BreadcrumbPortForward"),
            Message::ConnectResult(_) => write!(f, "Message::ConnectResult(...)"),
            Message::QuickConnectInteractiveAnswerChanged(i, v) => write!(
                f,
                "Message::QuickConnectInteractiveAnswerChanged({}, {})",
                i, v
            ),
            Message::QuickConnectInteractiveSubmit => {
                write!(f, "Message::QuickConnectInteractiveSubmit")
            }
            Message::QuickConnectInlinePasswordSubmit(_) => {
                write!(f, "Message::QuickConnectInlinePasswordSubmit(...)")
            }
            Message::QuickConnectInlinePasswordChanged(_) => {
                write!(f, "Message::QuickConnectInlinePasswordChanged(...)")
            }
            Message::QuickConnectInlinePasswordToggleVisibility => {
                write!(f, "Message::QuickConnectInlinePasswordToggleVisibility")
            }
            Message::QuickConnectInlinePasswordClose => {
                write!(f, "Message::QuickConnectInlinePasswordClose")
            }
            Message::HostKeyAcceptOnce => write!(f, "Message::HostKeyAcceptOnce"),
            Message::HostKeyAlwaysTrust => write!(f, "Message::HostKeyAlwaysTrust"),
            Message::HostKeyReject => write!(f, "Message::HostKeyReject"),
            Message::AutoProbeConsentOpen => write!(f, "Message::AutoProbeConsentOpen"),
            Message::AutoProbeConsentAllowOnce => write!(f, "Message::AutoProbeConsentAllowOnce"),
            Message::AutoProbeConsentAlwaysAllow => {
                write!(f, "Message::AutoProbeConsentAlwaysAllow")
            }
            Message::AutoProbeConsentUsePassword => {
                write!(f, "Message::AutoProbeConsentUsePassword")
            }
            Message::QuickConnectSwitchAuth => write!(f, "Message::QuickConnectSwitchAuth"),
            Message::DisconnectPressed => write!(f, "Message::DisconnectPressed"),
            Message::SessionExited(i) => write!(f, "Message::SessionExited({})", i),
            Message::ProfileConnect(p) => write!(f, "Message::ProfileConnect({:?})", p.name),
            Message::VaultUnlockOpenConnect(_) => write!(f, "Message::VaultUnlockOpenConnect(...)"),
            Message::VaultUnlockOpenDelete(id) => {
                write!(f, "Message::VaultUnlockOpenDelete({})", id)
            }
            Message::VaultUnlockOpenSaveSession => write!(f, "Message::VaultUnlockOpenSaveSession"),
            Message::VaultUnlockClose => write!(f, "Message::VaultUnlockClose"),
            Message::VaultUnlockPasswordChanged(_) => {
                write!(f, "Message::VaultUnlockPasswordChanged(...)")
            }
            Message::VaultUnlockSubmit => write!(f, "Message::VaultUnlockSubmit"),
            Message::VaultUnlockComplete(result) => {
                write!(f, "Message::VaultUnlockComplete({:?})", result.is_ok())
            }
            Message::ClipboardPaste(t) => write!(
                f,
                "Message::ClipboardPaste({:?})",
                t.as_ref().map(|_| "...")
            ),
            Message::ClipboardWriteDone => write!(f, "Message::ClipboardWriteDone"),
            Message::SaveSettings => write!(f, "Message::SaveSettings"),
            Message::ToggleDebugOverlay => write!(f, "Message::ToggleDebugOverlay"),
            Message::ReconnectTick => write!(f, "Message::ReconnectTick"),
            Message::ReconnectResult(_) => write!(f, "Message::ReconnectResult(...)"),
            Message::ReconnectCancel => write!(f, "Message::ReconnectCancel"),
            Message::RestoreSessionConfirm => write!(f, "Message::RestoreSessionConfirm"),
            Message::RestoreSessionDismiss => write!(f, "Message::RestoreSessionDismiss"),
            Message::SessionHoverStart(id) => write!(f, "Message::SessionHoverStart({})", id),
            Message::SessionHoverEnd => write!(f, "Message::SessionHoverEnd"),
            Message::PrewarmTick => write!(f, "Message::PrewarmTick"),
            Message::PrewarmResult(_) => write!(f, "Message::PrewarmResult(...)"),
            Message::SftpTab(msg) => write!(f, "Message::SftpTab({:?})", msg),
            Message::SftpInitComplete { tab_index, .. } => write!(
                f,
                "Message::SftpInitComplete {{ tab_index: {} }}",
                tab_index
            ),
            Message::SftpInitError { tab_index, error } => write!(
                f,
                "Message::SftpInitError {{ tab_index: {}, error: {} }}",
                tab_index, error
            ),
            Message::SftpDirLoaded { tab_index, entries } => write!(
                f,
                "Message::SftpDirLoaded {{ tab_index: {}, entries_count: {} }}",
                tab_index,
                entries.len()
            ),
            Message::SftpDirError { tab_index, error } => write!(
                f,
                "Message::SftpDirError {{ tab_index: {}, error: {} }}",
                tab_index, error
            ),
            Message::SftpRefreshComplete { tab_index } => write!(
                f,
                "Message::SftpRefreshComplete {{ tab_index: {} }}",
                tab_index
            ),
            Message::Noop => write!(f, "Message::Noop"),
            Message::SftpTransferComplete { tab_index, transfer_id } => write!(
                f,
                "Message::SftpTransferComplete {{ tab_index: {}, transfer_id: {} }}",
                tab_index, transfer_id
            ),
            Message::SftpTransferFailed { tab_index, transfer_id, error } => write!(
                f,
                "Message::SftpTransferFailed {{ tab_index: {}, transfer_id: {}, error: {} }}",
                tab_index, transfer_id, error
            ),
        }
    }
}
