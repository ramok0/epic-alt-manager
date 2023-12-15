use std::sync::Arc;

use eframe::EventLoopBuilder;

use crate::{
    epic::EpicError,
    process::{get_process_pid, kill_process}, config,
};

use super::{
    gui_renderer::App,
    gui_workers_proc::{
        clone_settings_proc, link_egl_account_proc, remove_account_proc, swap_account_proc,
    },
};

impl App {
    pub fn link_egl_account(&self) {
        let event_sender = self.event_manager.0.clone();
        let configuration_mtx = Arc::clone(&self.configuration);
        tokio::spawn(async move {
            let toast = link_egl_account_proc(configuration_mtx, event_sender.clone())
                .await
                .unwrap_or_else(|error| error.to_toast());

            let _ = event_sender.send(super::window::EventKind::AddToast(toast)).await;
        });
    }

    pub fn swap_account(&self, display_name: impl Into<String>) {
        let configuration_mtx = Arc::clone(&self.configuration);
        let event_sender = self.event_manager.0.clone();

        let display_name = display_name.into();
        tokio::spawn(async move {
            let result = swap_account_proc(configuration_mtx, display_name.clone()).await;

            if result.is_ok() {
                let _ = event_sender.send(super::window::EventKind::CurrentAccount(Some(display_name))).await;
            }

            let toast = result.unwrap_or_else(|error| error.to_toast());

            let _ = event_sender.send(super::window::EventKind::AddToast(toast)).await;
        });
    }

    pub fn remove_account(&self, display_name: impl Into<String>) {
        let configuration_mtx = Arc::clone(&self.configuration);
        let event_sender = self.event_manager.0.clone();

        let display_name = display_name.into();
        tokio::spawn(async move {
            let toast = remove_account_proc(configuration_mtx, event_sender.clone(), display_name)
                .await
                .unwrap_or_else(|error| error.to_toast());

            let _ = event_sender.send(super::window::EventKind::AddToast(toast)).await;
        });
    }

    pub fn kill_epic_games_launcher() -> Result<(), EpicError> {
        unsafe {
            let pid = get_process_pid("EpicGamesLauncher.exe".to_string()).map_err(|_| {
                EpicError::new(
                    crate::epic::EpicErrorKind::NotFound,
                    Some("Failed to find EpicGamesLauncher.exe"),
                )
            })?;

            kill_process(pid).map_err(|_| {
                EpicError::new(
                    crate::epic::EpicErrorKind::Other,
                    Some("Failed to kill EpicGamesLauncher.exe"),
                )
            })?;
        }

        Ok(())
    }
}
