//! 会话编辑器模态框
//! 统一的新建/编辑会话表单，支持快速连接和设置中心两种入口模式

use crate::app::components::session_form_fields::session_form_dialog;

/// Build the session editor modal for creating/editing SSH session profiles.
/// This delegates to the unified session_form_dialog component.
pub(crate) fn session_editor_modal(
    state: &crate::app::state::IcedState,
) -> iced::Element<'_, crate::app::message::Message> {
    session_form_dialog(state)
}
