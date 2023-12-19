use egui::RichText;
use egui_toast::{Toast, ToastKind, ToastOptions};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    config::Configuration,
    egl::{self, epic_get_remember_me_data},
    epic::{AccountDescriptor, EpicError, EpicErrorKind},
};

use super::window::{EventKind, EventSender};

pub(crate) async fn link_egl_account_proc(
    configuration_mtx: Arc<Mutex<Configuration>>,
    event_sender: EventSender,
) -> Result<Toast, EpicError> {
    let data = epic_get_remember_me_data().map_err(|_| {
        EpicError::new(
            EpicErrorKind::Other,
            Some("Failed to get EGL account to config"),
        )
    })?;

    let mut configuration = configuration_mtx.lock().await;

    let _ = configuration
        .add_account(crate::config::AddAccountProvider::RememberMeEntry(&data))
        .await
        .map_err(|_| {
            EpicError::new(
                EpicErrorKind::Other,
                Some("Failed to add account to configuration"),
            )
        })?;

    let _ = event_sender
        .send(
            EventKind::Accounts(configuration
                .accounts
                .iter()
                .map(|x| x.display_name.clone())
                .collect())
        )
        .await;

    let _ = configuration.flush();

    Ok(Toast {
        kind: ToastKind::Info,
        text: RichText::new(format!("Linked {} succesfully, you may have to reconnect on EpicGamesLauncher.", data.display_name.clone())).into(),
        options: ToastOptions::default()
            .duration_in_seconds(10.0)
            .show_progress(true)
            .show_icon(true),
    })
}

pub(crate) async fn clone_settings_proc(
    configuration_mtx: Arc<Mutex<Configuration>>,
    clone_from_username: String,
    clone_to_username: String,
) -> Result<Toast, EpicError> {
    let configuration = configuration_mtx.lock().await;

    let clone_from_opt = configuration
        .accounts
        .iter()
        .find(|x| x.display_name == clone_from_username);
    let clone_to_opt = configuration
        .accounts
        .iter()
        .find(|x| x.display_name == clone_to_username);

    let check_account = |x: Option<&AccountDescriptor>| -> Result<(), EpicError> {
        //check if account exists and device_auth is not null, otherwise, return an EpicError

        if x.is_none() {
            return Err(EpicError::new(
                EpicErrorKind::NotFound,
                Some("Failed to find account"),
            ));
        }

        let account = x.unwrap();

        if account.device_auth.is_none() {
            return Err(EpicError::new(
                EpicErrorKind::Other,
                Some("Account has no device_auth"),
            ));
        }

        Ok(())
    };

    check_account(clone_from_opt.clone())?;
    check_account(clone_to_opt.clone())?;

    let mut clone_from_device_auth = clone_from_opt.unwrap().device_auth.clone().unwrap();
    let mut clone_to_device_auth = clone_to_opt.unwrap().device_auth.clone().unwrap();

    let clone_from_account = clone_from_device_auth.login().await?;
    let clone_to_account = clone_to_device_auth.login().await?;


    let _ = clone_to_account.accept_eula().await;
    let _ = clone_to_account.grant_access().await;

    let client_settings = clone_from_account
        .get_user_file_content("ClientSettings.Sav")
        .await?;
    clone_to_account
        .insert_or_edit("ClientSettings.Sav", client_settings)
        .await?;

    Ok(Toast {
        kind: ToastKind::Info,
        text: RichText::new("Your configuration has been applied successfully").into(),
        options: ToastOptions::default()
            .duration_in_seconds(10.0)
            .show_progress(true)
            .show_icon(true),
    })
}

pub(crate) async fn swap_account_proc(
    configuration_mtx: Arc<Mutex<Configuration>>,
    display_name: String,
) -> Result<Toast, EpicError> {
    let configuration = configuration_mtx.lock().await;

    let account = configuration
        .accounts
        .iter()
        .find(|x| x.display_name == display_name);

    let descriptor = account.ok_or(EpicError::new(
        EpicErrorKind::NotFound,
        Some("Failed to find account"),
    ))?;
    let account = descriptor.login_as_launcher().await?;
    let infos = account.get_infos().await?;
    let remember_me_entry = infos.to_remember_me_entry(&account.refresh_token.unwrap());
    let _ = egl::epic_set_remember_me_data(remember_me_entry)?;

    Ok(Toast {
        text: RichText::new(format!(
            "Swapped to {} successfully, restart EpicGamesLauncher to apply changes",
            descriptor.display_name.clone()
        ))
        .into(),
        kind: ToastKind::Info,
        options: ToastOptions::default()
            .duration_in_seconds(10.0)
            .show_progress(true)
            .show_icon(true),
    })
}

pub async fn remove_account_proc(
    configuration_mtx: Arc<Mutex<Configuration>>,
    event_sender: EventSender,
    display_name: String,
) -> Result<Toast, EpicError> {
    let mut configuration = configuration_mtx.lock().await;
    let position_opt = configuration
        .accounts
        .iter()
        .position(|x| x.display_name == display_name);

    position_opt.ok_or(EpicError::new(
        EpicErrorKind::NotFound,
        Some("Failed to find account"),
    ))?;

    configuration.accounts.remove(position_opt.unwrap());
    let _ = event_sender
        .send(
            EventKind::Accounts(configuration
                .accounts
                .iter()
                .map(|x| x.display_name.clone())
                .collect()),
        )
        .await;
    let _ = configuration.flush();

    Ok(Toast {
        text: RichText::new("Removed account successfully").into(),
        kind: ToastKind::Success,
        options: ToastOptions::default()
            .duration_in_seconds(10.0)
            .show_progress(true)
            .show_icon(true),
    })
}
