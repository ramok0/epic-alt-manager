use std::sync::Arc;

use crate::{
    epic::EpicError,
    process::{get_process_pid, kill_process},
};

use super::{
    gui_renderer::App,
    gui_workers_proc::{
        clone_settings_proc, link_egl_account_proc, remove_account_proc, swap_account_proc,
    },
};

impl App {
    pub fn link_egl_account(&self) {
        let tx = self.data.account_communication.0.clone();
        let configuration_mtx = Arc::clone(&self.configuration);
        let toast_communication = self.data.toasts_communication.0.clone();
        tokio::spawn(async move {
            let toast = link_egl_account_proc(configuration_mtx, tx)
                .await
                .unwrap_or_else(|error| error.to_toast());

            let _ = toast_communication.send(toast).await;
        });
    }

    pub fn clone_settings(
        &self,
        clone_from_username: impl Into<String>,
        clone_to_username: impl Into<String>,
    ) {
        let configuration_mtx = Arc::clone(&self.configuration);
        let toast_communication = self.data.toasts_communication.0.clone();
        let clone_from_username = clone_from_username.into();
        let clone_to_username = clone_to_username.into();

        tokio::spawn(async move {
            let toast =
                clone_settings_proc(configuration_mtx, clone_from_username, clone_to_username)
                    .await
                    .unwrap_or_else(|error| error.to_toast());

            let _ = toast_communication.send(toast).await;
        });
    }

    pub fn swap_account(&self, display_name: impl Into<String>) {
        let configuration_mtx = Arc::clone(&self.configuration);
        let current_account_tx = self.data.current_account_communication.0.clone();
        let toast_communication = self.data.toasts_communication.0.clone();

        let display_name = display_name.into();
        tokio::spawn(async move {
            let result = swap_account_proc(configuration_mtx, display_name.clone()).await;

            if result.is_ok() {
                let _ = current_account_tx.send(Some(display_name)).await;
            }

            let toast = result.unwrap_or_else(|error| error.to_toast());

            let _ = toast_communication.send(toast).await;
        });
    }

    pub fn remove_account(&self, display_name: impl Into<String>) {
        let configuration_mtx = Arc::clone(&self.configuration);
        let accounts_tx = self.data.account_communication.0.clone();
        let toast_communication = self.data.toasts_communication.0.clone();

        let display_name = display_name.into();
        tokio::spawn(async move {
            let toast = remove_account_proc(configuration_mtx, accounts_tx, display_name)
                .await
                .unwrap_or_else(|error| error.to_toast());

            let _ = toast_communication.send(toast).await;
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
