use iced::Task;
use iced::{Point, Size};

use crate::app_model::AppModel;
use crate::iced_app::session_manager::SessionManager;
use crate::iced_app::terminal_rich::RowWidgetCache;
use crate::settings::TerminalSettings;
use crate::storage::StorageManager;
use crate::terminal_core::TerminalController;
use crate::theme::layout;
use secrecy::SecretString;

use super::message::{Message, SettingsCategory};

/// Fixed-size circular buffer for sliding window statistics.
#[derive(Debug, Clone)]
pub(crate) struct RingBuffer<T> {
    data: Vec<T>,
    head: usize,
    count: usize,
}

impl<T: Clone + Default> RingBuffer<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: vec![T::default(); capacity],
            head: 0,
            count: 0,
        }
    }

    /// Push a new value (evicts oldest if full).
    pub fn push(&mut self, value: T) {
        self.data[self.head] = value;
        self.head = (self.head + 1) % self.data.len();
        self.count = self.count.saturating_add(1).min(self.data.len());
    }

    /// Return the most recent value (the one about to be evicted).
    pub fn last(&self) -> Option<&T> {
        if self.count == 0 {
            return None;
        }
        let idx = if self.head == 0 {
            self.data.len() - 1
        } else {
            self.head - 1
        };
        Some(&self.data[idx])
    }

    /// Maximum of all stored values.
    pub fn max(&self) -> Option<T>
    where
        T: Ord,
    {
        self.data.iter().take(self.count).cloned().max()
    }

    pub fn len(&self) -> usize {
        self.count
    }
}

impl RingBuffer<u64> {
    /// Average of all stored u64 values as f64.
    pub fn average_ns(&self) -> Option<f64> {
        if self.count == 0 {
            return None;
        }
        let sum: u64 = self
            .data
            .iter()
            .take(self.count)
            .map(|&v| v)
            .sum();
        Some(sum as f64 / self.count as f64)
    }
}

impl<T> Default for RingBuffer<T>
where
    T: Clone + Default,
{
    fn default() -> Self {
        Self::with_capacity(60)
    }
}

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

/// Connection progress stage shown in the quick-connect form while connecting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ConnectionStage {
    #[default]
    None,
    /// Vault credentials loading / decrypting.
    VaultLoading,
    /// SSH TCP connection + handshake.
    SshConnecting,
    /// SSH authentication phase.
    Authenticating,
    /// PTY allocation + session finalization.
    SessionSetup,
    /// Auto-reconnect in progress.
    Reconnecting { attempt: u8, max: u8, delay_secs: u32 },
}

/// 快速连接弹窗内：列表 / 新建连接表单。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum QuickConnectPanel {
    #[default]
    Picker,
    NewConnection,
}

/// 连接预热状态。
#[derive(Debug, Clone)]
pub(crate) struct SessionPrewarmState {
    /// 预热中的会话 profile_id
    pub profile_id: Option<String>,
    /// 预热状态
    pub status: PrewarmStatus,
    /// 开始预热的时间
    pub start_time: Option<std::time::Instant>,
    /// 预热超时时间（秒）
    pub timeout_secs: u32,
}

/// 连接预热状态枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PrewarmStatus {
    /// 未开始预热
    Idle,
    /// 预热中（等待鼠标悬停 500ms）
    WaitingHover,
    /// 正在建立连接
    Connecting,
    /// 预热成功，可直接使用
    Ready,
    /// 预热失败
    Failed,
}

impl Default for PrewarmStatus {
    fn default() -> Self {
        PrewarmStatus::Idle
    }
}

/// 重连上下文：保存连接断开后重连所需的全部信息。
#[derive(Debug, Clone)]
pub(crate) struct ReconnectContext {
    /// 连接草案（包含 host, port, user, auth, password 等）。
    pub draft: crate::app_model::ConnectionDraft,
    /// Vault 主密码（用于读取凭据）。
    pub vault_master_password: Option<secrecy::SecretString>,
    /// 会话配置 ID（若有）。
    pub profile_id: Option<String>,
    /// 重连开始时间（用于计算已耗时）。
    pub start_time: std::time::Instant,
}

/// Tab chrome: label and optional saved profile id (for draft / breadcrumb).
///
/// **Runtime binding** (SSH + terminal emulator) lives in [`TabPane`], index-aligned with [`IcedState::tabs`].
#[derive(Debug, Clone)]
pub(crate) struct IcedTab {
    pub title: String,
    pub profile_id: Option<String>,
}

/// Per-tab animation state for tab width expand/collapse.
#[derive(Debug, Clone)]
pub(crate) struct TabAnimEntry {
    /// Target width in pixels (126.0 when fully open, 0.0 when closed).
    pub target_w: f32,
    pub enter_tick: u64,
    pub done: bool,
}

impl TabAnimEntry {
    pub(crate) fn new(target_w: f32, enter_tick: u64) -> Self {
        Self { target_w, enter_tick, done: false }
    }
}

/// Modal overlay animation state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ModalAnimPhase {
    Closed,
    Opening,
    Open,
    Closing,
}

#[derive(Debug, Clone)]
pub(crate) struct ModalAnimState {
    pub phase: ModalAnimPhase,
    pub enter_tick: u64,
}

impl ModalAnimState {
    pub(crate) fn opening(tick_count: u64) -> Self {
        Self { phase: ModalAnimPhase::Opening, enter_tick: tick_count }
    }
    pub(crate) fn closing(tick_count: u64) -> Self {
        Self { phase: ModalAnimPhase::Closing, enter_tick: tick_count }
    }
}

impl Default for ModalAnimState {
    fn default() -> Self {
        Self { phase: ModalAnimPhase::Closed, enter_tick: 0 }
    }
}

/// Per-tab terminal controller and runtime state (**1:1** with one [`IcedTab`]).
///
/// SSH sessions are managed by [`SessionManager`] in [`IcedState`], shared across all tabs.
/// This keeps `TabPane` focused on terminal-local concerns (viewport, cache, focus latch).
pub(crate) struct TabPane {
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
        let mut terminal = TerminalController::new(terminal_settings)
            .expect("Failed to initialize terminal controller - libghostty VT initialization failed");
        terminal.apply_terminal_palette_for_scheme(&terminal_settings.color_scheme);
        Self {
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
    /// Unified SSH session registry: each tab may have an independent live session.
    pub session_manager: SessionManager,
    pub active_tab: usize,
    pub window_size: Size,
    /// 主窗口是否键盘焦点（用于 DEC 1004 与捕获勾选组合）。
    pub window_focused: bool,
    /// 顶栏「快速连接」弹窗是否打开。
    pub quick_connect_open: bool,
    pub quick_connect_panel: QuickConnectPanel,
    /// Quick connect: search query / direct input.
    pub quick_connect_query: String,
    /// 连接预热状态
    pub prewarm_state: Option<SessionPrewarmState>,
    /// 上次预热 tick 的时间戳（毫秒），用于每 500ms 检查悬停状态
    pub last_prewarm_tick_ms: i64,
    /// Quick connect state machine.
    pub quick_connect_flow: QuickConnectFlow,
    /// Connection progress stage shown in the quick-connect form while connecting.
    pub connection_stage: ConnectionStage,
    /// Quick connect: last stable error kind for UI branching (Failed/NeedAuthPassword).
    pub quick_connect_error_kind: Option<crate::app_model::ConnectErrorKind>,
    /// Raw password input while the inline password overlay is shown (decoupled from draft).
    pub inline_password_input: secrecy::SecretString,
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
    /// Pending vault unlock context stored during async KDF operations.
    /// Cleared after unlock completes or on error.
    pub pending_vault_unlock: Option<PendingVaultUnlock>,
    /// 首次自动探测授权提醒（一次性）。
    pub auto_probe_consent_modal: Option<AutoProbeConsentModalState>,
    /// 连接信息行数，用于连接成功后清理终端中的连接提示。
    pub preconnect_info_line_count: usize,
    /// Vault 提示行数，用于 vault 解锁后清理提示并显示 SSH info。
    pub vault_hint_line_count: usize,
    /// 自动重连上下文（连接断开时保存）。
    pub reconnect_context: Option<ReconnectContext>,
    /// 重启后恢复会话弹窗状态。
    pub restore_session_modal: Option<RestoreSessionModalState>,
    /// 上次重连 tick 的时间戳（毫秒），用于每 1 秒触发一次重连倒计时检查。
    pub last_reconnect_tick_ms: i64,

    /// Tab 宽度动画状态（index-aligned with tabs）。
    pub tab_anims: Vec<TabAnimEntry>,
    /// Quick connect 弹窗动画状态。
    pub quick_connect_anim: ModalAnimState,
    /// Settings 弹窗动画状态。
    pub settings_anim: ModalAnimState,
    /// 滚动条 hover 状态（用于透明度动画）。
    pub scrollbar_hovered: bool,
    /// 滚动条 hover 开始时的 tick（用于 alpha 插值动画）。
    pub scrollbar_hover_enter_tick: Option<u64>,
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
        self.session_manager
            .get_session(self.active_tab)
            .is_some_and(|s| s.is_connected())
    }

    /// Estimated tick interval in ms for use in animation interpolation.
    /// Must be kept in sync with `compute_tick_ms` in `update/tick.rs`.
    pub(crate) fn tick_ms(&self) -> f32 {
        let now = crate::settings::unix_time_ms();
        let idle_ms = now.saturating_sub(self.last_activity_ms).max(0) as u64;
        let target_fps = self.model.settings.terminal.target_fps.clamp(10, 240);
        if !self.window_focused {
            250.0
        } else if idle_ms <= 1_000 {
            (1000.0 / target_fps as f32).max(16.0)
        } else if idle_ms <= 5_000 {
            (1000.0 / target_fps.min(30) as f32).max(33.0)
        } else {
            (1000.0 / target_fps.min(10) as f32).max(100.0)
        }
    }

    /// 检查是否需要恢复上次会话。
    /// 仅当 restore_last_session 设置开启且存在上次会话时返回。
    pub(crate) fn check_last_session_restore(&self) -> Option<crate::settings::RecentConnectionRecord> {
        if !self.model.settings.quick_connect.restore_last_session {
            return None;
        }
        self.model
            .settings
            .quick_connect
            .recent
            .iter()
            .find(|r| r.is_last_session)
            .cloned()
    }

    /// 启动自动重连流程。
    /// 如果 auto_reconnect 关闭或已达最大次数，返回 None。
    pub(crate) fn start_auto_reconnect(&mut self) -> bool {
        let max_attempts = self.model.settings.quick_connect.reconnect_max_attempts;
        if max_attempts == 0 {
            return false;
        }
        // 构建重连上下文
        let draft = self.model.draft.clone();
        let vault_master_password = self.model.vault_master_password.clone();
        let profile_id = self.model.selected_session_id.clone();
        self.reconnect_context = Some(ReconnectContext {
            draft,
            vault_master_password,
            profile_id,
            start_time: std::time::Instant::now(),
        });
        true
    }

    /// 获取下次重连的延迟（秒）。
    pub(crate) fn next_reconnect_delay(&self, attempt: u8) -> u32 {
        self.model
            .settings
            .reconnect_delay_for_attempt(attempt)
    }

    /// Scrollbar track alpha for hover animation.
    pub(crate) fn scrollbar_alpha(&self, tick_ms: f32) -> f32 {
        use crate::theme::animation::{anim_t, ease_out};
        use crate::theme::layout;
        let target = if self.scrollbar_hovered { 0.25 } else { 0.08 };
        let enter_tick = self.scrollbar_hover_enter_tick.unwrap_or(self.tick_count);
        let t = anim_t(enter_tick, self.tick_count, tick_ms, layout::DURATION_HOVER_MS as f32);
        let eased = if self.scrollbar_hovered { ease_out(t) } else { 1.0 - ease_out(t) };
        layout::SCROLLBAR_HIDE_ALPHA + (target - layout::SCROLLBAR_HIDE_ALPHA) * eased
    }

    /// Returns the current animated width for tab at index `i`.
    /// Uses `tab_anims[i]` if animating, otherwise `target_w`.
    pub(crate) fn tab_animated_width(&self, i: usize, tick_ms: f32) -> f32 {
        if i >= self.tab_anims.len() {
            return 126.0;
        }
        let anim = &self.tab_anims[i];
        if anim.done {
            return anim.target_w;
        }
        use crate::theme::animation::{anim_t, ease_out};
        let t = anim_t(anim.enter_tick, self.tick_count, tick_ms, layout::DURATION_TAB_NEW_MS as f32);
        let t = ease_out(t);
        if anim.target_w > 0.0 {
            anim.target_w * t
        } else {
            126.0 * (1.0 - t)
        }
    }

    /// Advance tab animation states, marking done ones as complete.
    pub(crate) fn tick_tab_anims(&mut self, tick_ms: f32) {
        for anim in &mut self.tab_anims {
            if anim.done {
                continue;
            }
            use crate::theme::animation::anim_done;
            let duration = if anim.target_w > 0.0 {
                layout::DURATION_TAB_NEW_MS as f32
            } else {
                layout::DURATION_TAB_CLOSE_MS as f32
            };
            if anim_done(anim.enter_tick, self.tick_count, tick_ms, duration) {
                anim.done = true;
                anim.target_w = anim.target_w.max(0.0);
            }
        }
    }

    /// Quick connect modal alpha: 0.0 (fully transparent) to 1.0 (fully opaque).
    pub(crate) fn quick_connect_anim_alpha(&self, tick_ms: f32) -> f32 {
        use crate::theme::animation::{anim_t, ease_out, ease_in};
        match self.quick_connect_anim.phase {
            ModalAnimPhase::Closed => 0.0,
            ModalAnimPhase::Opening => {
                let t = anim_t(self.quick_connect_anim.enter_tick, self.tick_count, tick_ms, layout::DURATION_MODAL_MS as f32);
                ease_out(t)
            }
            ModalAnimPhase::Open => 1.0,
            ModalAnimPhase::Closing => {
                let t = anim_t(self.quick_connect_anim.enter_tick, self.tick_count, tick_ms, layout::DURATION_MODAL_CLOSE_MS as f32);
                ease_in(t)
            }
        }
    }

    /// Quick connect modal Y offset: 0.0 (normal) to -8.0 (up).
    pub(crate) fn quick_connect_anim_offset(&self, tick_ms: f32) -> f32 {
        use crate::theme::animation::{anim_t, ease_out, ease_in};
        match self.quick_connect_anim.phase {
            ModalAnimPhase::Closed => -8.0,
            ModalAnimPhase::Opening => {
                let t = anim_t(self.quick_connect_anim.enter_tick, self.tick_count, tick_ms, layout::DURATION_MODAL_MS as f32);
                -8.0 * (1.0 - ease_out(t))
            }
            ModalAnimPhase::Open => 0.0,
            ModalAnimPhase::Closing => {
                let t = anim_t(self.quick_connect_anim.enter_tick, self.tick_count, tick_ms, layout::DURATION_MODAL_CLOSE_MS as f32);
                -8.0 * ease_in(t)
            }
        }
    }

    /// Advance modal animation state, advancing Opening→Open and Closing→Closed.
    pub(crate) fn tick_modal_anims(&mut self, tick_ms: f32) {
        use crate::theme::animation::anim_done;

        if self.quick_connect_anim.phase == ModalAnimPhase::Opening
            && anim_done(self.quick_connect_anim.enter_tick, self.tick_count, tick_ms, layout::DURATION_MODAL_MS as f32)
        {
            self.quick_connect_anim.phase = ModalAnimPhase::Open;
        }
        if self.quick_connect_anim.phase == ModalAnimPhase::Closing
            && anim_done(self.quick_connect_anim.enter_tick, self.tick_count, tick_ms, layout::DURATION_MODAL_CLOSE_MS as f32)
        {
            self.quick_connect_anim.phase = ModalAnimPhase::Closed;
            self.quick_connect_open = false;
        }
        if self.settings_anim.phase == ModalAnimPhase::Opening
            && anim_done(self.settings_anim.enter_tick, self.tick_count, tick_ms, layout::DURATION_MODAL_MS as f32)
        {
            self.settings_anim.phase = ModalAnimPhase::Open;
        }
        if self.settings_anim.phase == ModalAnimPhase::Closing
            && anim_done(self.settings_anim.enter_tick, self.tick_count, tick_ms, layout::DURATION_MODAL_CLOSE_MS as f32)
        {
            self.settings_anim.phase = ModalAnimPhase::Closed;
            self.settings_modal_open = false;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VaultStatus {
    Uninitialized,
    Unlocked,
    Locked,
    Unavailable,
}

/// Performance counters and diagnostics data.
///
/// Tick/pump statistics are accumulated since app start. Per-tab and sliding-window
/// data are used by the DebugOverlay; accumulated data is used by CSV export.
#[derive(Debug, Clone)]
pub(crate) struct PerfCounters {
    // === Tick/pump accumulated counters ===
    pub ticks: u64,
    pub pump_calls: u64,
    pub bytes_in: u64,
    pub last_log_ms: i64,
    pub ticks_at_log: u64,
    pub pump_calls_at_log: u64,
    pub bytes_in_at_log: u64,

    // === Accumulated timing (nanoseconds) ===
    /// Total tick elapsed time since start (ns).
    pub tick_ns_total: u64,
    /// Slow tick count (> SLOW_TICK_NS threshold).
    pub slow_ticks: u64,
    /// `slow_ticks` value at last log (for delta calculation).
    pub slow_ticks_at_log: Option<u64>,

    // === Per-pump diagnostics ===
    /// Number of pump calls that returned 0 bytes (empty reads).
    pub pump_empty_reads: u64,
    /// `pump_empty_reads` value at last log (for delta calculation).
    pub pump_empty_reads_at_log: Option<u64>,

    // === Per-tab statistics ===
    /// Pump call counts per tab (index-aligned with tab_panes).
    pub tab_pump_calls: Vec<u64>,
    /// Bytes-in per tab.
    pub tab_bytes_in: Vec<u64>,
    /// VT pump time per tab (ns).
    pub tab_vt_ns: Vec<u64>,
    /// VT frame time per tab (ns).
    pub tab_vt_frame_ns: Vec<u64>,

    // === VT engine timing ===
    /// Accumulated `update_render_state` + `update_dirty_styled_rows` time (ns).
    pub vt_update_ns_total: u64,
    /// Accumulated `on_frame_tick` time (ns).
    pub vt_frame_ns_total: u64,

    // === Sliding window (last N ticks) ===
    /// Last 60 tick durations for DebugOverlay display.
    pub tick_durations_ns: RingBuffer<u64>,

    // === Diagnostics output ===
    /// Optional perf CSV dump path (env: `RUST_SSH_PERF_DUMP`).
    pub dump_path: Option<String>,
    pub dump_header_written: bool,

    // === Runtime control ===
    /// Whether the DebugOverlay widget is visible.
    pub debug_overlay_enabled: bool,
}

impl Default for PerfCounters {
    fn default() -> Self {
        Self {
            ticks: 0,
            pump_calls: 0,
            bytes_in: 0,
            last_log_ms: 0,
            ticks_at_log: 0,
            pump_calls_at_log: 0,
            bytes_in_at_log: 0,
            tick_ns_total: 0,
            slow_ticks: 0,
            slow_ticks_at_log: None,
            pump_empty_reads: 0,
            pump_empty_reads_at_log: None,
            tab_pump_calls: Vec::new(),
            tab_bytes_in: Vec::new(),
            tab_vt_ns: Vec::new(),
            tab_vt_frame_ns: Vec::new(),
            vt_update_ns_total: 0,
            vt_frame_ns_total: 0,
            tick_durations_ns: RingBuffer::with_capacity(60),
            dump_path: None,
            dump_header_written: false,
            debug_overlay_enabled: false,
        }
    }
}

impl PerfCounters {
    /// Ensure tab arrays have at least `n` slots (grows lazily with tab count).
    pub fn ensure_tabs(&mut self, n: usize) {
        if self.tab_pump_calls.len() < n {
            self.tab_pump_calls.resize(n, 0);
            self.tab_bytes_in.resize(n, 0);
            self.tab_vt_ns.resize(n, 0);
            self.tab_vt_frame_ns.resize(n, 0);
        }
    }

    /// Reset accumulated counters (keeps config like dump_path and debug_overlay_enabled).
    pub fn reset(&mut self) {
        self.ticks = 0;
        self.pump_calls = 0;
        self.bytes_in = 0;
        self.tick_ns_total = 0;
        self.slow_ticks = 0;
        self.slow_ticks_at_log = None;
        self.pump_empty_reads = 0;
        self.pump_empty_reads_at_log = None;
        for v in self.tab_pump_calls.iter_mut() {
            *v = 0;
        }
        for v in self.tab_bytes_in.iter_mut() {
            *v = 0;
        }
        for v in self.tab_vt_ns.iter_mut() {
            *v = 0;
        }
        for v in self.tab_vt_frame_ns.iter_mut() {
            *v = 0;
        }
        self.vt_update_ns_total = 0;
        self.vt_frame_ns_total = 0;
        self.tick_durations_ns = RingBuffer::with_capacity(60);
        self.last_log_ms = crate::settings::unix_time_ms();
        self.ticks_at_log = 0;
        self.pump_calls_at_log = 0;
        self.bytes_in_at_log = 0;
        self.dump_header_written = false;
    }

    /// Tick rate (ticks per second) since last log.
    pub fn tick_rate(&self, dt_ms: i64) -> f64 {
        let dt = dt_ms.max(1) as f64 / 1000.0;
        (self.ticks - self.ticks_at_log) as f64 / dt
    }

    /// Pump rate (calls per second) since last log.
    pub fn pump_rate(&self, dt_ms: i64) -> f64 {
        let dt = dt_ms.max(1) as f64 / 1000.0;
        (self.pump_calls - self.pump_calls_at_log) as f64 / dt
    }

    /// Bytes-in rate (B/s) since last log.
    pub fn bytes_rate(&self, dt_ms: i64) -> f64 {
        let dt = dt_ms.max(1) as f64 / 1000.0;
        (self.bytes_in - self.bytes_in_at_log) as f64 / dt
    }

    /// Collect raw tick duration values from the ring buffer for the overlay histogram.
    pub fn tick_histogram_ms(&self) -> Vec<f64> {
        self.tick_durations_ns
            .data
            .iter()
            .take(self.tick_durations_ns.len())
            .map(|&v| v as f64 / 1_000_000.0)
            .collect()
    }
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

/// 重启后恢复会话弹窗状态。
#[derive(Debug, Clone)]
pub(crate) struct RestoreSessionModalState {
    /// 要恢复的最近连接记录。
    pub record: crate::settings::RecentConnectionRecord,
}

/// Context stored during async vault unlock operations.
/// This is kept in IcedState while the background KDF task is running.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct PendingVaultUnlock {
    pub verifier_hash: String,
    pub pending_connect: Option<crate::session::SessionProfile>,
    pub pending_delete_profile_id: Option<String>,
    pub pending_save_session: bool,
    pub pending_save_credentials_profile_id: Option<String>,
}

pub(crate) fn boot() -> (IcedState, Task<Message>) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime for iced app");
    let model = AppModel::load();
    let first_title = model.i18n.tr("iced.tab.new").to_string();
    let tab_panes = vec![TabPane::new(&model.settings.terminal)];
    let session_manager = SessionManager::new(1); // one tab at boot
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
            session_manager,
            active_tab: 0,
            window_size: Size::new(1280.0, 800.0),
            window_focused: true,
            quick_connect_open: false,
            quick_connect_panel: QuickConnectPanel::default(),
            quick_connect_query: String::new(),
            prewarm_state: None,
            last_prewarm_tick_ms: now,
            quick_connect_flow: QuickConnectFlow::Idle,
            connection_stage: ConnectionStage::None,
            quick_connect_error_kind: None,
            inline_password_input: secrecy::SecretString::from(String::new()),
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
            pending_vault_unlock: None,
            auto_probe_consent_modal: None,
            preconnect_info_line_count: 0,
            vault_hint_line_count: 0,
            reconnect_context: None,
            restore_session_modal: None,
            last_reconnect_tick_ms: now,
            tab_anims: vec![TabAnimEntry { target_w: 126.0, enter_tick: 0, done: true }],
            quick_connect_anim: ModalAnimState::default(),
            settings_anim: ModalAnimState::default(),
            scrollbar_hovered: false,
            scrollbar_hover_enter_tick: None,
        },
        Task::none(),
    )
}
