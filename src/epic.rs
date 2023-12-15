use std::{collections::HashMap, fmt::Display};

use crate::egl::{
    get_remember_me_data, RememberMeEntry, FORTNITE_IOS_GAME_CLIENT, LAUNCHER_APP_CLIENT_2,
};

use base64::{engine::general_purpose, Engine};
use egui_toast::{Toast, ToastOptions};
use lazy_static::lazy_static;
use reqwest::StatusCode;
use windows::{Win32::Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_LOCAL_MACHINE, CryptUnprotectData}, core::PCWSTR};

lazy_static! {
    static ref CLIENT: reqwest::Client = reqwest::Client::new();
}

const TOKEN: &'static str =
    "https://account-public-service-prod.ol.epicgames.com/account/api/oauth/token";
const EXCHANGE: &'static str =
    "https://account-public-service-prod.ol.epicgames.com/account/api/oauth/exchange";
const GET_DEVICE_AUTHORIZATION: &'static str =
    "https://account-public-service-prod03.ol.epicgames.com/account/api/oauth/deviceAuthorization";

const DEVICE_AUTH_SECRET_KEY: u8 = 0x85;

#[derive(Default, Debug, Clone, PartialEq, serde::Deserialize)]
pub struct FileEntry {
    #[serde(rename = "uniqueFilename")]
    pub unique_filename: String,
    pub filename: String,
    pub hash: String,
    pub hash256: String,
    pub length: i64,
    #[serde(rename = "contentType")]
    pub content_type: String,
    pub uploaded: String,
    #[serde(rename = "storageType")]
    pub storage_type: String,
    #[serde(rename = "accountId")]
    pub account_id: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DeviceAuthorization {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub prompt: String,
    pub expires_in: i64,
    pub interval: i64,
    pub client_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct DeviceAuth {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    pub secret: String,
}

impl DeviceAuth {
    //TODO : change the cipher algorithm with dpapi or AES
    pub fn cipher_secret(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let in_blob = CRYPT_INTEGER_BLOB {
            cbData: self.secret.len() as u32,
            pbData: self.secret.as_ptr() as *mut u8,
        };
        
        let mut out_blob = unsafe { std::mem::zeroed::<CRYPT_INTEGER_BLOB>() };

        unsafe {
            CryptProtectData(
                &in_blob,
                PCWSTR::null(),
                None,
                None,
                None,
                CRYPTPROTECT_LOCAL_MACHINE,
                &mut out_blob,
            )
        }?;

        self.secret = general_purpose::STANDARD.encode(unsafe { 
            std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize)
        });
        println!("Secret : {}", self.secret);
        Ok(())
    }

    pub fn uncipher_secret(&mut self) -> Result<(), EpicError> {
        let raw_data = general_purpose::STANDARD.decode(self.secret.as_bytes()).map_err(|_| {
            EpicError::new(
                EpicErrorKind::CipherError,
                Some("Failed to uncipher secret key"),
            )
        })?;

        let in_blob = CRYPT_INTEGER_BLOB {
            cbData: raw_data.len() as u32,
            pbData: raw_data.as_ptr() as *mut u8,
        };

        let mut out_blob = unsafe { std::mem::zeroed::<CRYPT_INTEGER_BLOB>() };

        unsafe {
            CryptUnprotectData(
                &in_blob,
                None,
                None,
                None,
                None,
                CRYPTPROTECT_LOCAL_MACHINE,
                &mut out_blob,
            )
        }.map_err(|_| EpicError::new(EpicErrorKind::CipherError, Some("Failed to uncipher secret key")))?;

        self.secret = String::from_utf8(unsafe {
            Vec::from_raw_parts(out_blob.pbData, out_blob.cbData as usize, out_blob.cbData as usize)
        }).map_err(|_| EpicError::new(EpicErrorKind::CipherError, Some("Failed to decode secret key")))?;

        Ok(())
    }

    pub fn uncipher_secret_xor(&mut self) -> Result<(), EpicError> {
        self.secret = self
            .secret
            .chars()
            .map(|x| (x as u8 ^ DEVICE_AUTH_SECRET_KEY) as char)
            .collect();

        Ok(())
    }

    pub async fn login(&mut self) -> Result<EpicAccount, EpicError> {
        self.uncipher_secret().map_err(|_| {
            EpicError::new(
                EpicErrorKind::CipherError,
                Some("Failed to uncipher secret key"),
            )
        })?;
        let response =
            crate::epic::token(Token::DeviceAuth(&self), FORTNITE_IOS_GAME_CLIENT).await?;

        Ok(response)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct AccountDescriptor {
    pub display_name: String,
    pub device_auth: Option<DeviceAuth>,
}

impl PartialEq for AccountDescriptor {
    fn eq(&self, other: &Self) -> bool {
        self.display_name == other.display_name
    }
}

impl AccountDescriptor {
    #[allow(dead_code)]
    pub fn is_currently_used(&self) -> bool {
        match get_remember_me_data() {
            Ok(data) => {
             //   println!("Self : {}, Data : {}", self.display_name, data.display_name);

                return *self.display_name == data.display_name;
            }
            Err(error) => {
                eprintln!("get_remember_me_data error : {}", error);
                return false;
            }
        }
    }

    pub async fn login_as_launcher(&self) -> Result<EpicAccount, EpicError> {
        let mut device_auth = self.device_auth.clone().ok_or(EpicError::new(
            EpicErrorKind::Other,
            Some("Your configuration does not contains a valid device_auth"),
        ))?;

        device_auth.uncipher_secret()?;
        let account = token(Token::DeviceAuth(&device_auth), FORTNITE_IOS_GAME_CLIENT).await?;
        let account = account.exchange_to(LAUNCHER_APP_CLIENT_2).await?;
        Ok(account)
    }
}

#[derive(Default, Debug, Clone, PartialEq, serde::Deserialize)]
pub struct EpicAccountDetails {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub email: String,
    pub name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ExchangeCode {
    pub code: String,
}

impl EpicAccountDetails {
    pub fn to_remember_me_entry(&self, refresh_token: &str) -> RememberMeEntry {
        RememberMeEntry {
            region: "Prod".to_string(),
            email: self.email.clone(),
            name: self.name.clone().unwrap_or("None".to_string()),
            last_name: self.last_name.clone().unwrap_or("None".to_string()),
            display_name: self.display_name.clone(),
            token: refresh_token.to_string(),
            b_has_password_auth: true,
        }
    }
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct EpicAccount {
    pub access_token: String,
    pub refresh_token: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub account_id: Option<String>,
    pub client_id: String,
}

impl EpicAccount {
    pub async fn get_device_authorization(
        &self,
    ) -> Result<DeviceAuthorization, EpicError> {
        let mut body = HashMap::new();
        body.insert("prompt", "login");

        let response = CLIENT
            .post(GET_DEVICE_AUTHORIZATION)
            .form(&body)
            .bearer_auth(&self.access_token)
            .send()
            .await.map_err(|_| EpicError::reqwest_internal_error())?;

        if !response.status().is_success() {
            // if cfg!(debug_assertions) {
            //     eprintln!("Error : {}", response.status());
            // eprintln!("Body : {}", response.text().await?);
            // }
            
            return Err(EpicError::reqwest_error(response.status()));
        }

        let authorization = response.json::<DeviceAuthorization>().await.map_err(|_| EpicError::new(EpicErrorKind::ParsingError, Some("Failed to parse JSON data")))?;

        Ok(authorization)
    }

    pub async fn get_user_files(&self) -> Result<Vec<FileEntry>, EpicError> {
        let response = CLIENT
            .get(format!(
                "https://fngw-mcp-gc-livefn.ol.epicgames.com/fortnite/api/cloudstorage/user/{}",
                self.account_id.clone().unwrap()
            ))
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|_| {
                EpicError::new(EpicErrorKind::HttpError, Some("Reqwest internal error"))
            })?;

        let status = response.status();

        if status.is_success() {
            let data = response.json::<Vec<FileEntry>>().await.unwrap_or(Vec::new());

            return Ok(data);
        } else {
            //     println!("Response : {}", response.text().await.unwrap());
            eprintln!("error while getting user files");
            return Err(EpicError::reqwest_error(status));
        }
    }

    pub async fn create_file(&self, unique_file_name: impl Into<String>, data:Vec<u8>) -> Result<(), EpicError> {
        let response = CLIENT
        .put(format!("https://fngw-mcp-gc-livefn.ol.epicgames.com/fortnite/api/cloudstorage/user/{}/{}",self.account_id.clone().unwrap(), unique_file_name.into()))
        .bearer_auth(&self.access_token)
        .body(data)
        .send()
        .await.map_err(|_| EpicError::reqwest_internal_error())?;

        if response.status().is_success() {
            return Ok(());
        } else {
            let status = response.status();
            eprintln!("error while creating file, body : {}", response.text().await.unwrap());
            return Err(EpicError::reqwest_error(status));
        }
    }

    pub async fn update_file(
        &self,
        unique_file_name: impl Into<String>,
        data: Vec<u8>,
    ) -> Result<(), EpicError> {
        let response = CLIENT
            .put(format!(
                "https://fngw-mcp-gc-livefn.ol.epicgames.com/fortnite/api/cloudstorage/user/{}/{}",
                self.account_id.clone().unwrap(),
                unique_file_name.into()
            ))
            .bearer_auth(&self.access_token)
            .body(data)
            .send()
            .await
            .map_err(|_| EpicError::reqwest_internal_error())?;

        if response.status().is_success() {
            return Ok(());
        } else {
            return Err(EpicError::reqwest_error(response.status()));
        }
    }

    pub async fn insert_or_edit(
        &self,
        unique_file_name: impl Into<String>,
        data: Vec<u8>,
    ) -> Result<(), EpicError> {
        let file_name: String = unique_file_name.into();

        let files = self.get_user_files().await?;
        if files
            .iter()
            .find(|x| x.unique_filename == file_name.clone())
            .is_none()
        {
            self.create_file(file_name.clone(), data).await?;
        } else {
            self.update_file(file_name, data).await?;
        }



        Ok(())
    }

    pub async fn get_user_file_content(
        &self,
        unique_file_name: impl Into<String>,
    ) -> Result<Vec<u8>, EpicError> {
        let response = CLIENT
            .get(format!(
                "https://fngw-mcp-gc-livefn.ol.epicgames.com/fortnite/api/cloudstorage/user/{}/{}",
                self.account_id.clone().unwrap(),
                unique_file_name.into()
            ))
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|_| EpicError::reqwest_internal_error())?;

        if response.status().is_success() {
            let bytes = response.bytes().await.map_err(|_| {
                EpicError::new(EpicErrorKind::ParsingError, Some("Failed to parse bytes"))
            })?;
            return Ok(bytes.to_vec());
        } else {
            println!("error while getting file");
            return Err(EpicError::reqwest_error(response.status()));
        }
    }

    pub async fn get_infos(&self) -> Result<EpicAccountDetails, EpicError> {
        let url = format!(
            "https://account-public-service-prod.ol.epicgames.com/account/api/public/account/{}",
            self.account_id.clone().unwrap_or(String::from("0"))
        );

        let response = CLIENT
            .get(url)
            .bearer_auth(self.access_token.clone())
            .send()
            .await
            .map_err(|_| EpicError::reqwest_internal_error())?;

        if !response.status().is_success() {
            return Err(EpicError::reqwest_error(response.status()));
        }

        Ok(response.json::<EpicAccountDetails>().await.map_err(|_| {
            EpicError::new(
                EpicErrorKind::ParsingError,
                Some("Failed to parse JSON data"),
            )
        })?)
    }

    pub async fn exchange_code(&self) -> Result<String, EpicError> {
        let response = CLIENT
            .get(EXCHANGE)
            .bearer_auth(self.access_token.clone())
            .send()
            .await
            .map_err(|_| EpicError::reqwest_internal_error())?;

        if !response.status().is_success() {
            return Err(EpicError::reqwest_error(response.status()));
        }

        let data = response.json::<ExchangeCode>().await.map_err(|_| {
            EpicError::new(
                EpicErrorKind::ParsingError,
                Some("Failed to parse JSON data."),
            )
        })?;
        Ok(data.code)
    }

    pub async fn exchange_to(
        &self,
        client: (&'static str, &'static str),
    ) -> Result<EpicAccount, EpicError> {
        let exchange_code = self.exchange_code().await?;
        let account = token(Token::ExchangeCode(&exchange_code), client).await?;
        Ok(account)
    }

    pub async fn create_device_auth(&self) -> Result<DeviceAuth, Box<dyn std::error::Error>> {
        let url = format!("https://account-public-service-prod.ol.epicgames.com/account/api/public/account/{}/deviceAuth", self.account_id.clone().unwrap());

        let response = CLIENT
            .post(url)
            .bearer_auth(self.access_token.clone())
            .send()
            .await?;

        if !response.status().is_success() {
            return Err("Failed to create device auth".into());
        }

        Ok(response.json::<DeviceAuth>().await?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenType {
    RefreshToken,
    AuthorizationCode,
    ExchangeCode,
    DeviceAuth,
    DeviceCode,
    None
}

pub fn token_types() -> [TokenType; 6] {
    [
        TokenType::RefreshToken,
        TokenType::AuthorizationCode,
        TokenType::ExchangeCode,
        TokenType::DeviceAuth,
        TokenType::DeviceCode,
        TokenType::None
    ]

}

impl Display for TokenType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            TokenType::RefreshToken => "RefreshToken",
            TokenType::AuthorizationCode => "AuthorizationCode",
            TokenType::ExchangeCode => "ExchangeCode",

            TokenType::DeviceAuth => "DeviceAuth",
            TokenType::DeviceCode => "DeviceCode",
            TokenType::None => "None",
        })?;

        Ok(())
    }
}

pub enum Token<'a> {
    RefreshToken(&'a str),
    #[allow(dead_code)]
    AuthorizationCode(&'a str),
    ExchangeCode(&'a str),
    ClientCredentials,
    DeviceAuth(&'a DeviceAuth),
    DeviceCode(&'a str),
}

#[derive(Debug, Clone)]
pub enum EpicErrorKind {
    IoError,
    NotFound,
    CipherError,
    EncodingError,
    HttpError,
    Authentification,
    ParsingError,
    InvalidResponse,
    Other,
}

impl Into<String> for EpicErrorKind {
    fn into(self) -> String {
        match self {
            EpicErrorKind::IoError => "IoError".to_string(),
            EpicErrorKind::Authentification => "Authentification".to_string(),
            EpicErrorKind::CipherError => "CipherError".to_string(),
            EpicErrorKind::EncodingError => "EncodingError".to_string(),
            EpicErrorKind::HttpError => "HttpError".to_string(),
            EpicErrorKind::NotFound => "NotFound".to_string(),
            EpicErrorKind::ParsingError => "ParsingError".to_string(),
            EpicErrorKind::InvalidResponse => "InvalidResponse".to_string(),
            EpicErrorKind::Other => "Other".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EpicError {
    kind: EpicErrorKind,
    message: Option<String>,
}

impl EpicError {
    pub fn new(kind: EpicErrorKind, message: Option<impl Into<String>>) -> Self {
        Self {
            kind,
            message: {
                if let Some(msg) = message {
                    Some(Into::<String>::into(msg))
                } else {
                    None
                }
            },
        }
    }

    pub fn to_toast(&self) -> Toast {
        Toast {
            kind: egui_toast::ToastKind::Error,
            text: egui::WidgetText::RichText(self.to_string().into()),
            options: ToastOptions::default().duration_in_seconds(10.),
        }
    }

    pub fn reqwest_error(status: StatusCode) -> Self {
        return Self {
            kind: EpicErrorKind::InvalidResponse,
            message: Some(format!("Invalid API Response, got response {}", status)),
        };
    }

    pub fn reqwest_internal_error() -> Self {
        return Self {
            kind: EpicErrorKind::HttpError,
            message: Some("Reqwest internal error".to_string()),
        };
    }
}

impl Display for EpicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = self.clone();

        write!(
            f,
            "An error of type {} occured, message:{}",
            Into::<String>::into(data.kind),
            data.message.unwrap_or("None".to_string())
        )?;

        Ok(())
    }
}

impl std::error::Error for EpicError {}

pub async fn token<'a>(
    token: Token<'a>,
    client: (&'static str, &'static str),
) -> Result<EpicAccount, EpicError> {
    let mut params = HashMap::new();

    match token {
        Token::RefreshToken(refresh_token) => {
            params.insert("token_type", "eg1");
            params.insert("grant_type", "refresh_token");
            params.insert("refresh_token", refresh_token);
        }
        Token::AuthorizationCode(code) => {
            params.insert("token_type", "eg1");
            params.insert("grant_type", "authorization_code");
            params.insert("code", code);
        }
        Token::ExchangeCode(code) => {
            params.insert("token_type", "eg1");
            params.insert("grant_type", "exchange_code");
            params.insert("exchange_code", code);
        }
        Token::DeviceAuth(device_auth) => {
            params.insert("token_type", "eg1");
            params.insert("grant_type", "device_auth");
            params.insert("account_id", &device_auth.account_id);
            params.insert("device_id", &device_auth.device_id);
            params.insert("secret", &device_auth.secret);
        }
        Token::ClientCredentials => {
            params.insert("grant_type", "client_credentials");
        }
        Token::DeviceCode(device_code) => {
            params.insert("grant_type", "device_code");
            params.insert("device_code", device_code);
        }
    }

    let response = CLIENT
        .post(TOKEN)
        .form(&params)
        .basic_auth(client.0, Some(client.1))
        .send()
        .await
        .map_err(|_| EpicError::reqwest_internal_error())?;

    let status = response.status();

    if !status.is_success() {
        if cfg!(debug_assertions) {
            eprintln!("Error URL : {}", response.url().to_string());
            if let Ok(body) = response.text().await {
                eprintln!("Error Body : {}", body);
            }
        }
        return Err(EpicError::reqwest_error(status));
    }

    let data: EpicAccount = response.json().await.map_err(|_| {
        EpicError::new(
            EpicErrorKind::ParsingError,
            Some("Failed to parse JSON error"),
        )
    })?;

    Ok(data)
}
