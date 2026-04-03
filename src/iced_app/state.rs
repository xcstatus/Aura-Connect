use iced::Task;
use iced::{Point, Size};

use crate::app_model::AppModel;
use crate::backend::ssh_session::AsyncSession;
use crate::iced_app::terminal_rich::RowWidgetCache;
use crate::settings::TerminalSettings;
use crate::storage::StorageManager;
use crate::terminal_core::TerminalController;
use secrecy::SecretString;

use super::message::{Message, SettingsCategory};

#[derive(Debug, Clone)]
pub(crate) struct InteractivePromptState {
    pub name: String,
    pub instructions: String,
    pub prompts: Vec<crate::backend::ssh_session::KeyboardInteractivePrompt>,
    pub answers: Vec<String>,
    pub error: Option<String>,
}

pub(crate) struct InteractiveAuthFlow {
    pub session: crate::backend::ssh_session::InteractiveAuthSession,
    pub ui: InteractivePromptState,
}

#[derive(Debug, Clone)]
pub(crate) struct HostKeyPromptState {
    pub info: crate::app_model::HostKeyErrorInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum QuickConnectFlow {
    #[default]
    Idle,
    Connecting,
    NeedUser,
    NeedAuthPassword,
    NeedAuthInteractive,
    AuthLocked,
    Failed,
    Connected,
}

/// 快速连接弹窗内：列表 / 新建连接表单。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum QuickConnectPanel {
    #[default]
    Picker,
    NewConnection,
}

/// Tab chrome: label and optional saved profile id (for draft / breadcrumb).
///
/// **Runtime binding** (SSH + terminal emulator) lives in [`TabPane`], index-aligned with [`IcedState::tabs`].
#[derive(Debug, Clone)]
pub(crate) struct IcedTab {
    pub title: String,
    pub profile_id: Option<String>,
}

/// Per-tab SSH session and [`TerminalController`] (**1:1** with one [`IcedTab`]).
///
/// **Policy** — [`crate::settings::QuickConnectSettings::single_shared_session`]:
/// - `true` (**default**, rollback-friendly): at most one tab holds `session`; connect clears other
///   tabs' sessions; switching away from a connected tab **disconnects** that session so the strip
///   cannot imply a different host than the live PTY.
/// - `false`: each tab may retain its own `session`; all connected tabs are pumped each tick.
pub(crate) struct TabPane {
    pub session: Option<Box<dyn AsyncSession>>,
    pub terminal: TerminalController,
    /// Per-session focus latch (DECSET 1004): last effective focus we reported to this tab's PTY.
    pub last_terminal_focus_sent: Option<bool>,
    /// Last time we pumped output for this pane (for background throttling).
    pub last_pump_ms: i64,
    /// Per-row Iced widget cache for dirty-row incremental rendering in Styled mode.
    /// Grows lazily to viewport rows and is invalidated on resize/render mode change.
    pub styled_row_cache: RowWidgetCache,
}

impl TabPane {
    pub fn new(terminal_settings: &TerminalSettings) -> Self {
        let mut terminal = TerminalController::new(terminal_settings);
        terminal.apply_terminal_palette_for_scheme(&terminal_settings.color_scheme);
        Self {
            session: None,
            terminal,
            last_terminal_focus_sent: None,
            last_pump_ms: 0,
            styled_row_cache: RowWidgetCache::new(),
        }
    }
}

pub(crate) struct IcedState {
    pub model: AppModel,
    /// Tokio runtime for session manager file operations (fast IO).
    pub rt: tokio::runtime::Runtime,
    pub tabs: Vec<IcedTab>,
    /// Same length as `tabs`; `tab_panes[i]` is the runtime for `tabs[i]`.
    pub tab_panes: Vec<TabPane>,
    pub active_tab: usize,
    pub window_size: Size,
    /// 主窗口是否键盘焦点（用于 DEC 1004 与捕获勾选组合）。
    pub window_focused: bool,
    /// 顶栏「快速连接」弹窗是否打开。
    pub quick_connect_open: bool,
    pub quick_connect_panel: QuickConnectPanel,
    /// Quick connect: search query / direct input.
    pub quick_connect_query: String,
    /// Quick connect state machine.
    pub quick_connect_flow: QuickConnectFlow,
    /// Quick connect: last stable error kind for UI branching (Failed/NeedAuthPassword).
    pub quick_connect_error_kind: Option<crate::app_model::ConnectErrorKind>,
    /// Keyboard-interactive auth flow state (when `quick_connect_flow == NeedAuthInteractive`).
    pub quick_connect_interactive: Option<InteractiveAuthFlow>,
    /// Host key confirmation overlay (Ask policy).
    pub host_key_prompt: Option<HostKeyPromptState>,
    /// Runtime-only known host overrides ("accept once").
    pub runtime_known_hosts: Vec<crate::settings::KnownHostRecord>,
    /// 鼠标悬停的标签索引；用于仅在悬停时显示关闭按钮。
    pub tab_hover_index: Option<usize>,
    /// Last known window cursor position (for mouse press/release events without coordinates).
    pub last_cursor_pos: Option<Point>,
    pub settings_modal_open: bool,
    pub settings_category: SettingsCategory,
    pub settings_sub_tab: [usize; 5],
    pub settings_connection_search: String,
    pub settings_needs_restart: bool,
    /// 帧计数（定时订阅），用于光标闪烁等。
    pub tick_count: u64,

    /// Activity latch for dynamic tick rate (user input / PTY output).
    pub last_activity_ms: i64,
    /// Last time we ran an expensive cursor/blink refresh (ms).
    pub last_blink_tick_ms: i64,
    /// Aggregated counters for quick validation.
    pub perf: PerfCounters,

    /// Vault 状态（底栏 + 安全中心展示），由真实 vault/设置推导。
    pub vault_status: VaultStatus,
    /// 安全中心：初始化/改密流程弹窗状态。
    pub vault_flow: Option<VaultFlowState>,
    /// 会话编辑器弹窗状态（最小 SSH 表单）。
    pub session_editor: Option<SessionEditorState>,
    /// 连接前 vault 解锁弹窗（用于读取/写入凭据）。
    pub vault_unlock: Option<VaultUnlockState>,
    /// 首次自动探测授权提醒（一次性）。
    pub auto_probe_consent_modal: Option<AutoProbeConsentModalState>,
}

impl IcedState {
    #[inline]
    pub(crate) fn active_pane(&self) -> &TabPane {
        &self.tab_panes[self.active_tab]
    }

    #[inline]
    pub(crate) fn active_pane_mut(&mut self) -> &mut TabPane {
        &mut self.tab_panes[self.active_tab]
    }

    #[inline]
    pub(crate) fn active_terminal(&self) -> &TerminalController {
        &self.active_pane().terminal
    }

    /// Whether the **active** tab has a live SSH session that reports connected.
    pub(crate) fn active_session_is_connected(&self) -> bool {
        self.active_pane()
            .session
            .as_ref()
            .is_some_and(|s| s.is_connected())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VaultStatus {
    Uninitialized,
    Unlocked,
    Locked,
    Unavailable,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct PerfCounters {
    pub ticks: u64,
    pub pump_calls: u64,
    pub bytes_in: u64,
    pub last_log_ms: i64,
    pub ticks_at_log: u64,
    pub pump_calls_at_log: u64,
    pub bytes_in_at_log: u64,
    /// Optional perf CSV dump path (env: `RUST_SSH_PERF_DUMP`).
    pub dump_path: Option<String>,
    pub dump_header_written: bool,
}

impl VaultStatus {
    pub(crate) fn compute(settings: &crate::settings::Settings, runtime_unlocked: bool) -> Self {
        let Some(_meta) = settings.security.vault.as_ref() else {
            return VaultStatus::Uninitialized;
        };
        let Some(path) = StorageManager::get_vault_path() else {
            return VaultStatus::Unavailable;
        };
        if !path.exists() {
            // Meta is present but vault file is missing; treat as initialized but currently unusable.
            return VaultStatus::Unavailable;
        }
        if runtime_unlocked {
            VaultStatus::Unlocked
        } else {
            VaultStatus::Locked
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VaultFlowMode {
    Initialize,
    ChangePassword,
}

#[derive(Debug, Clone)]
pub(crate) struct VaultFlowState {
    pub mode: VaultFlowMode,
    pub old_password: SecretString,
    pub new_password: SecretString,
    pub confirm_password: SecretString,
    pub error: Option<String>,
}

impl VaultFlowState {
    pub(crate) fn new(mode: VaultFlowMode) -> Self {
        Self {
            mode,
            old_password: SecretString::new("".into()),
            new_password: SecretString::new("".into()),
            confirm_password: SecretString::new("".into()),
            error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SessionEditorState {
    pub profile_id: Option<String>,
    pub host: String,
    pub port: String,
    pub user: String,
    pub auth: crate::session::AuthMethod,
    pub password: SecretString,
    /// Existing credential id (if editing a profile that already has one).
    pub existing_credential_id: Option<String>,
    /// Whether password input was edited in this session.
    pub password_dirty: bool,
    /// Explicitly clear saved password (must not be inferred from empty input).
    pub clear_saved_password: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct VaultUnlockState {
    pub pending_connect: Option<crate::session::SessionProfile>,
    pub pending_delete_profile_id: Option<String>,
    pub pending_save_session: bool,
    /// Save credentials for this profile id after a successful connect (post-connect UX).
    pub pending_save_credentials_profile_id: Option<String>,
    pub password: SecretString,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct AutoProbeConsentModalState {
    // Reserved for future: allow other pending actions.
}

pub(crate) fn boot() -> (IcedState, Task<Message>) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime for iced app");
    let model = AppModel::load();
    let first_title = model.i18n.tr("iced.tab.new").to_string();
    let tab_panes = vec![TabPane::new(&model.settings.terminal)];
    let vault_status = VaultStatus::compute(&model.settings, model.vault_master_password.is_some());
    let now = crate::settings::unix_time_ms();
    let dump_path = std::env::var("RUST_SSH_PERF_DUMP")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    (
        IcedState {
            model,
            rt,
            tabs: vec![IcedTab {
                title: first_title,
                profile_id: None,
            }],
            tab_panes,
            active_tab: 0,
            window_size: Size::new(1280.0, 800.0),
            window_focused: true,
            quick_connect_open: false,
            quick_connect_panel: QuickConnectPanel::default(),
            quick_connect_query: String::new(),
            quick_connect_flow: QuickConnectFlow::Idle,
            quick_connect_error_kind: None,
            quick_connect_interactive: None,
            host_key_prompt: None,
            runtime_known_hosts: Vec::new(),
            tab_hover_index: None,
            last_cursor_pos: None,
            settings_modal_open: false,
            settings_category: SettingsCategory::General,
            settings_sub_tab: [0; 5],
            settings_connection_search: String::new(),
            settings_needs_restart: false,
            tick_count: 0,
            last_activity_ms: now,
            last_blink_tick_ms: 0,
            perf: PerfCounters {
                dump_path,
                dump_header_written: false,
                ..PerfCounters::default()
            },
            vault_status,
            vault_flow: None,
            session_editor: None,
            vault_unlock: None,
            auto_probe_consent_modal: None,
        },
        Task::none(),
    )
}
