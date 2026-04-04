/// 设置中心左侧大类（与 egui `SettingTab` 对齐）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum SettingsCategory {
    #[default]
    General,
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
    Theme(String),
    AccentColor(String),
    FontSize(f32),
    GpuAcceleration(bool),
    TargetFps(u32),
    AtlasResetOnPressure(bool),
    ColorScheme(String),
    /// Terminal text size (logical px); affects PTY grid when metrics are applied.
    TerminalFontSize(f32),
    LineHeight(f32),
    FontFamily(String),
    /// When true, `font_size` / `line_height` drive PTY cols/rows and Iced terminal rendering.
    ApplyTerminalMetrics(bool),
    GpuFontPath(String),
    GpuFontFaceIndex(String),
    RightClickPaste(bool),
    BracketedPaste(bool),
    KeepSelectionHighlight(bool),
    ScrollbackLimit(usize),
    HistorySearch(bool),
    PathCompletion(bool),
    IdleTimeoutMins(u32),
    LockOnSleep(bool),
    HostKeyPolicy(crate::settings::HostKeyPolicy),
    ConnectionSearch(String),
    /// When true: only one tab may hold SSH; switching tabs disconnects the previous session.
    SingleSharedSession(bool),
}

/// Type alias for connect result error info (error kind + optional host key error details).
pub type ConnectResultError = (crate::app_model::ConnectErrorKind, Option<crate::app_model::HostKeyErrorInfo>);

/// Wrapped session type for async connection result.
pub type ConnectSession = std::sync::Arc<Box<dyn crate::backend::ssh_session::AsyncSession>>;

/// Message type - manually implement Debug since AsyncSession doesn't implement it.
#[derive(Clone)]
pub(crate) enum Message {
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
    SessionEditorSave,
    TabSelected(usize),
    TabClose(usize),
    /// 标签芯片悬停：`None` 表示鼠标已离开当前芯片。
    TabChipHover(Option<usize>),
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
    /// Connection result callback (triggered by Task::perform).
    /// Contains: Result<session, (error_kind, host_key_error_info)>
    ConnectResult(Result<ConnectSession, ConnectResultError>),
    /// Keyboard-interactive: update answer field.
    QuickConnectInteractiveAnswerChanged(usize, String),
    /// Keyboard-interactive: submit current answers (advance auth state machine).
    QuickConnectInteractiveSubmit,
    /// Host key confirmation (Ask policy).
    HostKeyAcceptOnce,
    HostKeyAlwaysTrust,
    HostKeyReject,
    AutoProbeConsentOpen,
    AutoProbeConsentAllowOnce,
    AutoProbeConsentAlwaysAllow,
    AutoProbeConsentUsePassword,
    DisconnectPressed,
    ProfileConnect(crate::session::SessionProfile),
    VaultUnlockOpenConnect(crate::session::SessionProfile),
    VaultUnlockOpenDelete(String),
    VaultUnlockOpenSaveSession,
    VaultUnlockClose,
    VaultUnlockPasswordChanged(String),
    VaultUnlockSubmit,
    /// Async vault unlock result (callback from background KDF task).
    VaultUnlockComplete(Result<String, crate::vault::VaultUnlockError>),
    /// Clipboard contents for terminal paste (bracketed when DEC 2004 is on).
    ClipboardPaste(Option<String>),
    /// Ack for clipboard write task (copy).
    ClipboardWriteDone,
    SaveSettings,
    /// Toggle the debug overlay visibility (Ctrl+Shift+D).
    ToggleDebugOverlay,
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
            Message::QuickConnectQueryChanged(q) => write!(f, "Message::QuickConnectQueryChanged({})", q),
            Message::QuickConnectDirectSubmit => write!(f, "Message::QuickConnectDirectSubmit"),
            Message::QuickConnectPickRecent(rec) => write!(f, "Message::QuickConnectPickRecent({:?})", rec),
            Message::TopOpenSettings => write!(f, "Message::TopOpenSettings"),
            Message::SettingsDismiss => write!(f, "Message::SettingsDismiss"),
            Message::SettingsCategoryChanged(cat) => write!(f, "Message::SettingsCategoryChanged({:?})", cat),
            Message::SettingsSubTabChanged(i) => write!(f, "Message::SettingsSubTabChanged({})", i),
            Message::SettingsFieldChanged(field) => write!(f, "Message::SettingsFieldChanged({:?})", field),
            Message::BiometricsToggle(v) => write!(f, "Message::BiometricsToggle({})", v),
            Message::SettingsRestartAcknowledged => write!(f, "Message::SettingsRestartAcknowledged"),
            Message::VaultOpen => write!(f, "Message::VaultOpen"),
            Message::VaultClose => write!(f, "Message::VaultClose"),
            Message::VaultOldPasswordChanged(_) => write!(f, "Message::VaultOldPasswordChanged(...)"),
            Message::VaultNewPasswordChanged(_) => write!(f, "Message::VaultNewPasswordChanged(...)"),
            Message::VaultConfirmPasswordChanged(_) => write!(f, "Message::VaultConfirmPasswordChanged(...)"),
            Message::VaultSubmit => write!(f, "Message::VaultSubmit"),
            Message::DeleteSessionProfile(id) => write!(f, "Message::DeleteSessionProfile({})", id),
            Message::OpenSessionEditor(id) => write!(f, "Message::OpenSessionEditor({:?})", id),
            Message::SessionEditorClose => write!(f, "Message::SessionEditorClose"),
            Message::SessionEditorHostChanged(v) => write!(f, "Message::SessionEditorHostChanged({})", v),
            Message::SessionEditorPortChanged(v) => write!(f, "Message::SessionEditorPortChanged({})", v),
            Message::SessionEditorUserChanged(v) => write!(f, "Message::SessionEditorUserChanged({})", v),
            Message::SessionEditorAuthChanged(a) => write!(f, "Message::SessionEditorAuthChanged({:?})", a),
            Message::SessionEditorPasswordChanged(_) => write!(f, "Message::SessionEditorPasswordChanged(...)"),
            Message::SessionEditorClearPasswordToggled(v) => write!(f, "Message::SessionEditorClearPasswordToggled({})", v),
            Message::SessionEditorSave => write!(f, "Message::SessionEditorSave"),
            Message::TabSelected(i) => write!(f, "Message::TabSelected({})", i),
            Message::TabClose(i) => write!(f, "Message::TabClose({})", i),
            Message::TabChipHover(i) => write!(f, "Message::TabChipHover({:?})", i),
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
            Message::QuickConnectAuthChanged(a) => write!(f, "Message::QuickConnectAuthChanged({:?})", a),
            Message::QuickConnectKeyPathChanged(v) => write!(f, "Message::QuickConnectKeyPathChanged({})", v),
            Message::QuickConnectPassphraseChanged(_) => write!(f, "Message::QuickConnectPassphraseChanged(...)"),
            Message::ConnectPressed => write!(f, "Message::ConnectPressed"),
            Message::ConnectResult(_) => write!(f, "Message::ConnectResult(...)"),
            Message::QuickConnectInteractiveAnswerChanged(i, v) => write!(f, "Message::QuickConnectInteractiveAnswerChanged({}, {})", i, v),
            Message::QuickConnectInteractiveSubmit => write!(f, "Message::QuickConnectInteractiveSubmit"),
            Message::HostKeyAcceptOnce => write!(f, "Message::HostKeyAcceptOnce"),
            Message::HostKeyAlwaysTrust => write!(f, "Message::HostKeyAlwaysTrust"),
            Message::HostKeyReject => write!(f, "Message::HostKeyReject"),
            Message::AutoProbeConsentOpen => write!(f, "Message::AutoProbeConsentOpen"),
            Message::AutoProbeConsentAllowOnce => write!(f, "Message::AutoProbeConsentAllowOnce"),
            Message::AutoProbeConsentAlwaysAllow => write!(f, "Message::AutoProbeConsentAlwaysAllow"),
            Message::AutoProbeConsentUsePassword => write!(f, "Message::AutoProbeConsentUsePassword"),
            Message::DisconnectPressed => write!(f, "Message::DisconnectPressed"),
            Message::ProfileConnect(p) => write!(f, "Message::ProfileConnect({:?})", p.name),
            Message::VaultUnlockOpenConnect(_) => write!(f, "Message::VaultUnlockOpenConnect(...)"),
            Message::VaultUnlockOpenDelete(id) => write!(f, "Message::VaultUnlockOpenDelete({})", id),
            Message::VaultUnlockOpenSaveSession => write!(f, "Message::VaultUnlockOpenSaveSession"),
            Message::VaultUnlockClose => write!(f, "Message::VaultUnlockClose"),
            Message::VaultUnlockPasswordChanged(_) => write!(f, "Message::VaultUnlockPasswordChanged(...)"),
            Message::VaultUnlockSubmit => write!(f, "Message::VaultUnlockSubmit"),
            Message::VaultUnlockComplete(result) => write!(f, "Message::VaultUnlockComplete({:?})", result.is_ok()),
            Message::ClipboardPaste(t) => write!(f, "Message::ClipboardPaste({:?})", t.as_ref().map(|_| "...")),
            Message::ClipboardWriteDone => write!(f, "Message::ClipboardWriteDone"),
            Message::SaveSettings => write!(f, "Message::SaveSettings"),
            Message::ToggleDebugOverlay => write!(f, "Message::ToggleDebugOverlay"),
        }
    }
}
