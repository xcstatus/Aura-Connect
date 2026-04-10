pub fn tr(key: &'static str) -> &'static str {
    match key {
        "iced.tab.terminal" => "Terminal",
        "iced.tab.settings" => "Settings",
        "iced.title.brand" => "RustSsh",
        "iced.title.subtitle" => "Terminal 2.0 (iced + libghostty)",
        "iced.field.host" => "Host",
        "iced.field.port" => "Port",
        "iced.field.user" => "User",
        "iced.field.password" => "Password",
        "iced.btn.connect" => "Connect",
        "iced.btn.disconnect" => "Disconnect",
        "iced.btn.save_session" => "Save session",
        "iced.btn.save_settings" => "Save settings",
        "iced.btn.switch_auth" => "Switch auth method",
        "iced.btn.confirm" => "Confirm",
        "iced.btn.cancel" => "Cancel",
        "iced.breadcrumb.connected" => "Connected",
        "iced.breadcrumb.disconnected" => "Disconnected",
        "iced.breadcrumb.not_connected" => "Not connected",
        "iced.btn.reconnect" => "Reconnect",
        "iced.btn.sftp" => "SFTP",
        "iced.btn.port_fwd" => "Ports",
        "iced.sidebar.sessions" => "Sessions",
        "iced.sidebar.connect" => "Connect",
        "iced.term.capture_keys" => "Terminal captures keyboard",
        "iced.term.command_placeholder" => "Type a command and press Enter",
        "iced.term.password_placeholder" => "Password",
        "iced.term.passphrase_placeholder" => "Passphrase",
        "iced.footer.status" => "Status",
        "iced.footer.hint" => "Hint",
        "iced.footer.vault" => "Vault",
        "iced.footer.vault_unlocked" => "Unlocked",
        "iced.settings.language" => "Language",
        "iced.settings.language.zh" => "简体中文",
        "iced.settings.language.en" => "English (US)",
        "iced.settings.language.help" => "Applies immediately; use Save settings to persist.",
        "iced.settings.title" => "Settings",
        "iced.settings.cat.general" => "General",
        "iced.settings.cat.terminal" => "Terminal",
        "iced.settings.cat.connection" => "Connections",
        "iced.settings.cat.security" => "Security",
        "iced.settings.cat.backup" => "Backup & sync",
        "iced.settings.sub.general.basic" => "Basic",
        "iced.settings.sub.general.appearance" => "Appearance",
        "iced.settings.sub.general.typography" => "Typography",
        "iced.settings.sub.terminal.render" => "Rendering",
        "iced.settings.sub.terminal.scheme" => "Color scheme",
        "iced.settings.sub.terminal.text" => "Text",
        "iced.settings.sub.terminal.interaction" => "Interaction",
        "iced.settings.sub.terminal.quick" => "Quick adjust",
        "iced.settings.sub.conn.ssh" => "SSH",
        "iced.settings.sub.conn.telnet" => "Telnet",
        "iced.settings.sub.conn.serial" => "Serial",
        "iced.settings.sub.conn.advanced" => "Advanced",
        "iced.settings.sub.security.policy" => "Vault & lock",
        "iced.settings.sub.security.hosts" => "Known hosts",
        "iced.settings.sub.backup.main" => "Backup",
        "iced.settings.section.startup" => "Startup & language",
        "iced.settings.row.language" => "Language",
        "iced.settings.row.auto_update" => "Check for updates automatically",
        "iced.settings.section.appearance" => "Appearance",
        "iced.settings.row.theme" => "Theme",
        "iced.settings.row.accent" => "Accent color (#RRGGBB)",
        "iced.settings.section.typography" => "UI font size",
        "iced.settings.row.ui_font_size" => "Size",
        "iced.settings.section.render_engine" => "Rendering engine",
        "iced.settings.row.target_fps" => "Target FPS",
        "iced.settings.section.color_scheme" => "Terminal color schemes",
        "iced.settings.section.terminal_theme" => "Terminal theme mode",
        "iced.settings.terminal.theme.auto" => "Follow UI theme",
        "iced.settings.terminal.theme.dark" => "Dark",
        "iced.settings.terminal.theme.light" => "Light",
        "iced.settings.section.quick_adjust" => "Quick color adjustment",
        "iced.settings.section.quick_adjust.desc" => "Customize terminal colors (leave empty to use preset)",
        "iced.settings.color.bg" => "Background",
        "iced.settings.color.fg" => "Foreground",
        "iced.settings.color.cursor" => "Cursor",
        "iced.settings.color.placeholder" => "#RRGGBB",
        "iced.settings.color.reset" => "Reset to preset",
        "iced.settings.section.preview" => "Preview",
        "iced.settings.scheme.applied" => "Applied",
        "iced.settings.scheme.apply" => "Apply",
        "iced.settings.section.text_render" => "Text rendering",
        "iced.settings.row.apply_terminal_metrics" => {
            "Apply font size & line height to terminal (PTY grid)"
        }
        "iced.settings.row.terminal_font_size" => "Terminal font size (px)",
        "iced.settings.row.line_height" => "Line height",
        "iced.settings.row.mono_font" => "Monospace font",
        "iced.settings.section.interaction" => "Interaction",
        "iced.settings.row.right_paste" => "Right-click to paste",
        "iced.settings.row.bracketed_paste" => "Bracketed paste (DEC 2004, if remote enables it)",
        "iced.settings.row.keep_selection" => "Keep selection highlight",
        "iced.settings.row.scrollback" => "Scrollback lines",
        "iced.settings.row.history_search" => "History search",
        "iced.settings.row.path_completion" => "Path completion",
        "iced.settings.conn.advanced_title" => "Advanced",
        "iced.settings.conn.session_model_title" => "Tabs & sessions",
        "iced.settings.row.single_shared_session" => "Single-session mode (recommended)",
        "iced.settings.hint.single_shared_session" => {
            "When on, at most one SSH connection: only the active tab holds the session; switching tabs disconnects the previous tab, avoiding a tab title/host mismatch with the live PTY. When off, each tab may keep its own connection and terminal buffer."
        }
        "iced.settings.conn.advanced_hint" => {
            "Proxy, jump host, certificates, and keepalive will arrive in a later release."
        }
        "iced.settings.row.auto_reconnect" => "Auto-reconnect on disconnect",
        "iced.settings.row.auto_reconnect.help" => {
            "Automatically attempt to reconnect when the connection is unexpectedly dropped."
        }
        "iced.settings.row.reconnect_max_attempts" => "Max reconnect attempts",
        "iced.settings.row.reconnect_max_attempts.help" => {
            "Set to 0 to disable auto-reconnect."
        }
        "iced.settings.row.reconnect_delay" => "Base reconnect delay (seconds)",
        "iced.settings.row.reconnect_delay.help" => {
            "Initial wait time before first reconnect. With exponential backoff, delay doubles each retry."
        }
        "iced.settings.row.reconnect_exponential" => "Use exponential backoff",
        "iced.settings.row.reconnect_exponential.help" => {
            "Double the wait time after each failed attempt (capped at 30 seconds)."
        }
        "iced.settings.row.restore_last_session" => "Restore last session on startup",
        "iced.settings.row.restore_last_session.help" => {
            "Automatically reconnect to the last session when the app starts."
        },
        "iced.settings.conn.manage_suffix" => "sessions",
        "iced.settings.conn.search_hint" => "Search name or address…",
        "iced.settings.conn.new" => "New",
        "iced.settings.conn.empty" => "No matching sessions.",
        "iced.settings.conn.edit" => "Edit",
        "iced.settings.conn.delete" => "Delete",
        "iced.settings.backup.title" => "Backup & sync",
        "iced.settings.backup.hint" => {
            "Export/import and cloud sync are not available in this build (placeholder)."
        }
        "iced.settings.restart.banner" => "Some display settings require restarting the app.",
        "iced.settings.restart.ok" => "OK",
        "iced.topbar.new_tab" => "New",
        "iced.topbar.quick_connect" => "Quick Connect",
        "iced.topbar.settings_center" => "Settings",
        "iced.tab.new" => "New session",
        "iced.placeholder.soon" => "Coming soon",
        "iced.quick_connect.recent" => "Recent",
        "iced.quick_connect.saved" => "Saved",
        "iced.quick_connect.new_connection" => "New connection",
        "iced.quick_connect.group_default" => "Uncategorized",
        "iced.quick_connect.empty_recent" => "No recent connections yet",
        "iced.quick_connect.search_or_direct" => "Search saved sessions, or enter user@host[:port]",
        "iced.quick_connect.direct_cta" => "Connect to {target}",
        "iced.quick_connect.back" => "Back",
        "iced.quick_connect.new_title" => "New connection",
        "iced.quick_connect.need_password" => "[rustssh] Password required. Enter your password below and click Connect.",
        "iced.quick_connect.need_passphrase" => "[rustssh] Passphrase required. Enter your passphrase below and click Connect.",
        "iced.quick_connect.need_auth" => "[rustssh] Authentication required. Enter your credential and click Connect.",
        "iced.vault_unlock.title" => "Unlock Vault",
        "iced.vault_unlock.title_save_credentials" => "Unlock Vault to save credentials",
        "iced.vault_unlock.hint_save_credentials" => {
            "Connection succeeded. Unlocking will save the password to Vault for next time."
        }
        "iced.vault_unlock.password_placeholder" => "Master password",
        "iced.vault_unlock.btn.confirm" => "Confirm",
        "iced.vault_unlock.btn.cancel" => "Cancel",
        "iced.vault_unlock.error.wrong_password" => "Incorrect password",
        "iced.vault_unlock.error.vault_not_initialized" => "Vault is not initialized",
        "iced.vault_unlock.error.vault_path_not_found" => "Cannot locate vault path",
        "iced.vault_unlock.error.unknown" => "Failed to unlock vault",
        "iced.vault.error.password_mismatch" => "Passwords do not match",
        "iced.vault.error.old_password_failed" => "Old password verification failed",
        "iced.vault.error.init_failed" => "Vault initialization failed",
        "iced.vault.error.vault_path_not_found" => "Cannot locate vault path",
        "iced.vault.error.file_lost" => "Vault file is lost. Existing credentials will be cleared after password change.",
        "iced.vault.error.save_failed" => "Vault save operation failed. Please check permissions or file corruption.",
        "iced.vault.title.initialize" => "Initialize Vault",
        "iced.vault.title.change_password" => "Change Master Password",
        "iced.vault.label.old_password" => "Current password",
        "iced.vault.label.new_password" => "New password",
        "iced.vault.label.confirm_password" => "Confirm new password",
        "iced.host_key_prompt.title" => "Host key confirmation",
        "iced.host_key_prompt.host_line" => "{host}:{port} ({algo})",
        "iced.host_key_prompt.old_fingerprint" => "Old fingerprint: {fp}",
        "iced.host_key_prompt.new_fingerprint" => "New fingerprint: {fp}",
        "iced.host_key_prompt.accept_once" => "Trust once",
        "iced.host_key_prompt.always_trust" => "Always trust",
        "iced.host_key_prompt.reject" => "Reject",
        "iced.term.connecting" => "[rustssh] Connecting…",
        "iced.term.connected" => "[rustssh] Connected.",
        "iced.term.connection_failed" => "[rustssh] Connection failed.",
        "iced.term.vault_needed" => {
            "[rustssh] This session requires Vault unlock to load credentials."
        }
        "iced.term.vault_unlock_to_continue" => "[rustssh] Unlock Vault to continue…",
        "iced.term.vault_unlocked" => "[rustssh] Vault unlocked.",
        "iced.term.ssh.connecting" => "SSH  Connecting to {target}",
        "iced.term.ssh.host_fingerprint" => "SSH  Host key: {algo} {fp}",
        "iced.term.ssh.auth_method" => "SSH  Auth: {method}",
        "iced.term.ssh.authenticating" => "SSH  Authenticating...",
        "iced.term.reconnecting" => "[rustssh] Reconnecting…",
        "iced.term.reconnect_attempt" => "[rustssh] Reconnecting ({n}/{max})…",
        "iced.term.reconnect_countdown" => "[rustssh] Retrying in {secs}s…",
        "iced.term.reconnect_failed" => "[rustssh] Reconnect failed: {reason}",
        "iced.term.reconnect_success" => "[rustssh] Reconnected.",
        "iced.term.connection_closed" => "[rustssh] Connection closed.",
        "iced.term.connection_timeout" => "[rustssh] Connection timed out.",
        "iced.term.connection_refused" => "[rustssh] Connection refused.",
        "iced.term.network_unreachable" => "[rustssh] Network unreachable.",
        "iced.term.host_unreachable" => "[rustssh] Host unreachable.",
        "iced.auth.password" => "password",
        "iced.auth.public_key" => "public key",
        "iced.auth.keyboard_interactive" => "keyboard-interactive",
        "iced.auth.agent" => "SSH Agent",
        "iced.stage.vault_loading" => "Decrypting credentials...",
        "iced.stage.ssh_connecting" => "Connecting to server",
        "iced.stage.authenticating" => "Verifying identity",
        "iced.stage.session_setup" => "Initializing session",
        "settings.security.vault.title" => "Vault Security",
        "settings.security.auto_lock.label" => "Auto-lock timeout",
        "settings.security.auto_lock.help" => {
            "Lock vault after inactivity. Choose Never to disable auto-lock."
        }
        "settings.security.timeout.never" => "Never",
        "settings.security.timeout.minute_1" => "1 minute",
        "settings.security.timeout.minute_5" => "5 minutes",
        "settings.security.timeout.minute_10" => "10 minutes",
        "settings.security.timeout.minute_30" => "30 minutes",
        "settings.security.lock_on_sleep.label" => "Lock when app is backgrounded",
        "settings.security.lock_on_sleep.help" => {
            "Lock vault immediately when minimized or moved to background."
        }
        "settings.security.kdf.title" => "Encryption Strength",
        "settings.security.kdf.help" => {
            "Trade-off between unlock speed and security. Takes effect on next unlock."
        }
        "settings.security.kdf.balanced" => "Balanced (Fast unlock, ~1-2 seconds)",
        "settings.security.kdf.security" => "Secure (Stronger protection, ~4-6 seconds)",
        "settings.security.master_password.change_title" => "Change master password",
        "settings.security.master_password.init_title" => "Initialize vault",
        "settings.security.master_password.change_help" => {
            "Change the root key used to decrypt the vault."
        }
        "settings.security.master_password.init_help" => {
            "Set a master password and create a local encrypted vault."
        }
        "settings.security.master_password.change_action" => "Change",
        "settings.security.master_password.init_action" => "Initialize",
        "settings.security.biometrics.title" => "System Biometrics",
        "settings.security.biometrics.label" => "Use fingerprint or face authentication",
        "settings.security.biometrics.help" => {
            "Supports Touch ID, Windows Hello, or system keychain. Verification is required before toggling."
        }
        "settings.security.biometrics.reason.toggle" => {
            "Please verify to change biometric settings"
        }
        "settings.security.hosts.title" => "Known Hosts",
        "settings.security.hosts.policy.label" => "Connection verification policy",
        "settings.security.hosts.policy.help" => {
            "How to handle unknown or mismatched host fingerprints"
        }
        "settings.security.hosts.policy.strict" => "Strict",
        "settings.security.hosts.policy.ask" => "Ask",
        "settings.security.hosts.policy.accept_new" => "Accept New",
        "settings.security.hosts.table.title" => "Trusted hosts",
        "settings.security.hosts.table.col.host" => "Host",
        "settings.security.hosts.table.col.algorithm" => "Algorithm",
        "settings.security.hosts.table.col.fingerprint" => "SHA256 Fingerprint",
        "toast.biometrics.in_progress" => "Biometric verification in progress. Please wait.",
        "toast.biometrics.updated" => "System verification passed. Setting updated.",
        "biometric.error.not_available" => "Biometrics are unavailable on this device",
        "biometric.error.not_enrolled" => "Biometrics are not enrolled on this device",
        "biometric.error.locked_out" => "Biometrics are locked. Unlock with system passcode first",
        "biometric.error.user_canceled" => "Biometric verification was canceled",
        "biometric.error.permission_denied" => {
            "Biometric permission denied. Please allow it in system settings"
        }
        "biometric.error.timeout" => "Biometric verification timed out. Please try again",
        "biometric.error.unknown" => "Biometric verification failed",
        "biometric.error.unknown_with_detail" => "Biometric verification failed: {detail}",

        // ========== Session Form (Unified) ==========
        "session_form.title_new" => "New Connection",
        "session_form.title_edit" => "Edit Connection",
        "session_form.field.name" => "Name",
        "session_form.field.name_placeholder" => "Enter connection name",
        "session_form.field.group" => "Group",
        "session_form.field.group_placeholder" => "Select group",
        "session_form.field.host" => "Host",
        "session_form.field.host_placeholder" => "192.168.1.100 or example.com",
        "session_form.field.port" => "Port",
        "session_form.field.user" => "Username",
        "session_form.field.password" => "Password",
        "session_form.field.auth_method" => "Auth Method",
        "session_form.field.private_key" => "Private Key",
        "session_form.field.private_key_placeholder" => "Select private key file",
        "session_form.field.passphrase" => "Passphrase",
        "session_form.field.passphrase_placeholder" => "Private key password (optional)",
        "session_form.field.browse" => "Browse",
        // Auth method options
        "session_form.auth.password" => "Password",
        "session_form.auth.private_key" => "Private Key",
        "session_form.auth.agent" => "SSH Agent",
        "session_form.auth.interactive" => "Keyboard Interactive",
        // Buttons
        "session_form.btn.test_connection" => "Test Connection",
        "session_form.btn.testing" => "Testing...",
        "session_form.btn.save" => "Save",
        "session_form.btn.cancel" => "Cancel",
        // Sidebar
        "session_form.sidebar.general" => "General",
        "session_form.sidebar.advanced" => "Advanced",
        "session_form.sidebar.port_forward" => "Port Forwarding",
        "session_form.sidebar.encryption" => "Encryption",
        // Group management
        "session_form.group.create_new" => "Create New Group",
        "session_form.group.placeholder" => "Enter group name",
        // Advanced settings
        "session_form.advanced.keep_alive" => "Keep-alive interval",
        "session_form.advanced.keep_alive_unit" => "seconds",
        "session_form.advanced.connection_timeout" => "Connection timeout",
        "session_form.advanced.proxy_type" => "Proxy type",
        "session_form.advanced.proxy_type.none" => "None",
        "session_form.advanced.proxy_type.socks5" => "SOCKS5",
        "session_form.advanced.proxy_type.http" => "HTTP",
        "session_form.advanced.proxy_host" => "Proxy host",
        "session_form.advanced.proxy_port" => "Proxy port",
        "session_form.advanced.heartbeat" => "Heartbeat",
        "session_form.advanced.heartbeat_interval" => "Heartbeat interval",
        // Error messages
        "session_form.error.required" => "This field is required",
        "session_form.error.port_range" => "Port range 1-65535",
        "session_form.error.host_invalid" => "Invalid host address format",
        "session_form.success.test" => "Connection test successful",
        "session_form.error.test" => "Connection test failed: {reason}",

        _ => key,
    }
}
