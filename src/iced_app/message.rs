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

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Tick,
    /// 主窗口尺寸（逻辑像素），用于判断标签条是否横向溢出。
    WindowResized(iced::Size),
    EventOccurred(iced::event::Event),
    TopAddTab,
    TopQuickConnect,
    /// 关闭快速连接弹窗（遮罩、标题栏 ×、Esc）。
    QuickConnectDismiss,
    /// 快速连接：进入新建连接表单。
    QuickConnectNewConnection,
    /// 从表单返回连接列表。
    QuickConnectBackToList,
    /// 快速连接：搜索已保存会话 / 输入直连字符串。
    QuickConnectQueryChanged(String),
    /// 快速连接：对当前输入执行“直连”动作（填入 Draft 并进入表单/或触发连接）。
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
    /// Clipboard contents for terminal paste (bracketed when DEC 2004 is on).
    ClipboardPaste(Option<String>),
    /// Ack for clipboard write task (copy).
    ClipboardWriteDone,
    SaveSettings,
}
