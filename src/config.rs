use crate::{
    egl::{RememberMeEntry, FORTNITE_IOS_GAME_CLIENT, LAUNCHER_APP_CLIENT_2},
    epic::{self, AccountDescriptor, EpicAccount}, launchers::Launchers,
};
use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Configuration {
    pub accounts: Vec<AccountDescriptor>,
    pub launcher:Launchers,
    pub legendary_path:String,
    pub version:String
}

pub enum AddAccountProvider<'a> {
    RememberMeEntry(&'a RememberMeEntry),
    EpicAccount(&'a EpicAccount),
}

pub enum AddAccountError {
    FailedToExchange,
    InvalidResponse,
    CipherError,
    FailedToLogin,
}

impl Display for AddAccountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AddAccountError::FailedToExchange => "FailedToExchange",
                AddAccountError::InvalidResponse => "InvalidResponse",
                AddAccountError::CipherError => "CipherError",
                AddAccountError::FailedToLogin => "FailedToLogin",
            }
        )?;

        Ok(())
    }
}

impl Configuration {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut configuration: Configuration = Configuration::default();
        configuration.read()?;

        Ok(configuration)
    }

    fn update(&mut self) {
        let version = self.version.to_string();
        println!("version : {}", version);
        match version.as_str() {
            "0.1.0" => {
                println!("Updating configuration from version 0.1.0 to 0.1.1");
                self.accounts.iter_mut().for_each(|x| {
                    if let Some(device_auth) = &mut x.device_auth {
                        if let Ok(_) = device_auth.uncipher_secret_xor() {
                            let _ = device_auth.cipher_secret();
                        }
                    }
                });

                let _ = self.flush();
            }
            _ => {}
        }

        self.version = crate::version::get_program_version().to_string();
    }

    fn get_path() -> PathBuf {
        std::path::PathBuf::from("config.json")
    }

    fn exists() -> bool {
        Path::exists(&Configuration::get_path())
    }

    fn apply_values(&mut self, data: &Configuration) {
        self.accounts = data.accounts.clone();
        self.launcher = data.launcher.clone();
        self.legendary_path = data.legendary_path.clone();
        self.version = data.version.clone();
    }

    fn read(&mut self) -> Result<(), Box<dyn std::error::Error>> {


        let path_buf = Configuration::get_path();
        if Configuration::exists() {
            let data_str = std::fs::read_to_string(path_buf)?;
            let data: Configuration = serde_json::from_str(&data_str)?;

            self.apply_values(&data);

            if data.version != crate::version::get_program_version() {
                self.update();
            }
        } else {
            let data = Self::default();
            data.flush()?;

            self.apply_values(&data);
        }

        Ok(())
    }

    pub fn flush(&self) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::write(
            Configuration::get_path(),
            serde_json::to_string_pretty(&self)?,
        )?;
        if cfg!(debug_assertions) {
            println!("Flushed configuration successfully !");
        }
        Ok(())
    }

    pub fn insert_or_edit(&mut self, data: &AccountDescriptor) -> () {
        if let Some(pos) = self
            .accounts
            .iter()
            .position(|x| x.display_name == data.display_name)
        {
            self.accounts[pos] = data.clone();
        } else {
            self.accounts.push(data.clone());
        }
    }

    pub async fn add_account<'a>(
        &mut self,
        account: AddAccountProvider<'a>,
    ) -> Result<AccountDescriptor, AddAccountError> {
        match account {
            AddAccountProvider::RememberMeEntry(entry) => {
                /*
                   - se connecter au compte
                   - exchange vers fortnite ios
                   - créer le device auth
                   - sauver le device auth
                */
                let account = epic::token(
                    epic::Token::RefreshToken(&entry.token),
                    LAUNCHER_APP_CLIENT_2,
                )
                .await
                .map_err(|_| AddAccountError::FailedToLogin)?;

                let account = match account.exchange_to(FORTNITE_IOS_GAME_CLIENT).await {
                    Ok(data) => Ok(data),
                    Err(_) => Err(AddAccountError::FailedToExchange),
                }?;

                let mut device_auth = match account.create_device_auth().await {
                    Ok(data) => Ok(data),
                    Err(_) => Err(AddAccountError::InvalidResponse),
                }?;

                if device_auth.cipher_secret().is_err() {
                    return Err(AddAccountError::CipherError);
                }

                let descriptor = AccountDescriptor {
                    display_name: entry.display_name.clone(),
                    device_auth: Some(device_auth),
                };

                self.insert_or_edit(&descriptor);

                return Ok(descriptor);
            }
            AddAccountProvider::EpicAccount(account) => {
                //fortnite ios ou new switch
                /*
                   - créer le device auth
                   - sauver le device auth
                */

                let mut account = account.clone();

                if account.client_id != FORTNITE_IOS_GAME_CLIENT.0 {
                    account = account
                        .exchange_to(FORTNITE_IOS_GAME_CLIENT)
                        .await
                        .map_err(|_| AddAccountError::FailedToExchange)?;
                }

                let mut device_auth = match account.create_device_auth().await {
                    Ok(data) => Ok(data),
                    Err(_) => Err(AddAccountError::InvalidResponse),
                }?;

                if device_auth.cipher_secret().is_err() {
                    return Err(AddAccountError::CipherError);
                }
                let descriptor = AccountDescriptor {
                    display_name: account.display_name.clone().unwrap(),
                    device_auth: Some(device_auth),
                };

                self.insert_or_edit(&descriptor);

                return Ok(descriptor);
            }
        }
    }
}

impl Drop for Configuration {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            accounts: Vec::new(),
            launcher: Launchers::EpicGamesLauncher,
            legendary_path:String::new(),
            version: crate::version::get_program_version().to_string()
        }
    }
}
