use super::App;
use crate::app_server_session::AppServerSession;
use codex_protocol::ThreadId;

impl App {
    pub(super) async fn open_requirement_view(
        &mut self,
        tui: &mut crate::tui::Tui,
        app_server: &mut AppServerSession,
        thread_id: ThreadId,
    ) {
        let result = app_server.thread_requirement_read(thread_id).await;
        if self.current_displayed_thread_id() != Some(thread_id) {
            return;
        }

        match result {
            Ok(response) => {
                let _ = tui.enter_alt_screen();
                self.overlay = Some(crate::requirement_view::requirement_overlay(
                    &response.requirement,
                    self.keymap.pager.clone(),
                ));
                tui.frame_requester().schedule_frame();
            }
            Err(err) => {
                self.chat_widget
                    .add_error_message(format!("Failed to read requirement outcome: {err}"));
            }
        }
    }
}
