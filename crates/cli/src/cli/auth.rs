use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose};
use clap::{Args, Subcommand};
use rand::Rng;
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use url::Url;

use localgpt_core::config::{Config, GeminiOAuthConfig, GitHubOAuthConfig};
use localgpt_core::env::{
    LOCALGPT_GITHUB_CLIENT_ID, LOCALGPT_GITHUB_CLIENT_SECRET, LOCALGPT_GOOGLE_CLIENT_ID,
    LOCALGPT_GOOGLE_CLIENT_SECRET,
};

const GEMINI_REDIRECT_URI: &str = "http://localhost:8085/oauth2callback";
const GEMINI_SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/cloud-platform",
    "https://www.googleapis.com/auth/userinfo.email",
    "https://www.googleapis.com/auth/userinfo.profile",
];

const GITHUB_DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const GITHUB_ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

#[derive(Args, Debug)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub command: AuthCommands,
}

#[derive(Subcommand, Debug)]
pub enum AuthCommands {
    /// Authenticate with Google Gemini
    Gemini {
        /// Google Cloud Project ID
        #[arg(short, long)]
        project: Option<String>,
    },
    /// Authenticate with GitHub (for Copilot models)
    Github,
}

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}

#[derive(Deserialize, Debug)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

pub async fn run(args: AuthArgs) -> Result<()> {
    match args.command {
        AuthCommands::Gemini { project } => run_gemini_auth(project).await,
        AuthCommands::Github => run_github_auth().await,
    }
}

async fn run_gemini_auth(project_id: Option<String>) -> Result<()> {
    // 1. Resolve credentials (require env vars)
    let client_id = std::env::var(LOCALGPT_GOOGLE_CLIENT_ID).map_err(|_| {
        anyhow::anyhow!(
            "Missing {} environment variable. Please set it with your Google OAuth Client ID.",
            LOCALGPT_GOOGLE_CLIENT_ID
        )
    })?;
    let client_secret = std::env::var(LOCALGPT_GOOGLE_CLIENT_SECRET).map_err(|_| {
        anyhow::anyhow!(
            "Missing {} environment variable. Please set it with your Google OAuth Client Secret.",
            LOCALGPT_GOOGLE_CLIENT_SECRET
        )
    })?;

    // 2. Generate PKCE
    let (code_challenge, code_verifier) = generate_pkce();

    // 3. Generate state
    let state = generate_random_string(32);

    // 4. Construct Authorization URL
    let mut auth_url = Url::parse("https://accounts.google.com/o/oauth2/v2/auth").unwrap();
    auth_url
        .query_pairs_mut()
        .append_pair("client_id", &client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", GEMINI_REDIRECT_URI)
        .append_pair("scope", &GEMINI_SCOPES.join(" "))
        .append_pair("code_challenge", &code_challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", &state)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent");

    println!("\nTo authenticate with Google Gemini, please visit the following URL:\n");
    println!("{}", auth_url);
    println!("\nWaiting for authorization...\n");

    // 5. Start local server to listen for callback
    let expected_state = state.clone();
    let code =
        tokio::task::spawn_blocking(move || wait_for_callback(8085, &expected_state)).await??;

    println!("Authorization code received. Exchanging for tokens...");

    // 6. Exchange code for tokens
    let tokens = exchange_code(&code, &code_verifier, &client_id, &client_secret).await?;

    // 7. Save tokens to config
    update_config_gemini(tokens, client_id, client_secret, project_id).await?;

    println!("Successfully authenticated with Gemini! Tokens saved to config.");
    println!("Default model set to 'gemini/gemini-3-pro-preview'.");
    Ok(())
}

async fn run_github_auth() -> Result<()> {
    // 1. Resolve Client ID
    let client_id = std::env::var(LOCALGPT_GITHUB_CLIENT_ID).map_err(|_| {
        anyhow::anyhow!(
            "Missing {} environment variable. Please set it with your GitHub OAuth Client ID.",
            LOCALGPT_GITHUB_CLIENT_ID
        )
    })?;
    let client_secret = std::env::var(LOCALGPT_GITHUB_CLIENT_SECRET).ok();

    println!("\nAuthenticating with GitHub...");
    let client = Client::new();

    // 2. Request device code
    let params = [
        ("client_id", &client_id),
        ("scope", &"read:user".to_string()),
    ];

    let response = client
        .post(GITHUB_DEVICE_CODE_URL)
        .header("Accept", "application/json")
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        anyhow::bail!("GitHub device code request failed: {}", error_text);
    }

    let device_code: DeviceCodeResponse = response.json().await?;

    println!("\nPlease visit: {}", device_code.verification_uri);
    println!("And enter code: {}", device_code.user_code);
    println!("\nWaiting for authorization...\n");

    // 3. Poll for access token
    let mut interval = std::time::Duration::from_secs(device_code.interval.max(1));
    let expires_at =
        std::time::Instant::now() + std::time::Duration::from_secs(device_code.expires_in);

    while std::time::Instant::now() < expires_at {
        let mut params = vec![
            ("client_id", client_id.clone()),
            ("device_code", device_code.device_code.clone()),
            (
                "grant_type",
                "urn:ietf:params:oauth:grant-type:device_code".to_string(),
            ),
        ];
        if let Some(ref secret) = client_secret {
            params.push(("client_secret", secret.clone()));
        }

        let response = client
            .post(GITHUB_ACCESS_TOKEN_URL)
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;

        if let Some(access_token) = json["access_token"].as_str() {
            // Success!
            let refresh_token = json["refresh_token"].as_str().map(|s| s.to_string());
            let expires_in = json["expires_in"].as_u64();

            update_config_github(
                access_token,
                refresh_token,
                expires_in,
                client_id,
                client_secret,
            )
            .await?;

            println!("Successfully authenticated with GitHub! Tokens saved to config.");
            println!("Default model set to 'github/copilot'.");
            return Ok(());
        }

        if let Some(error) = json["error"].as_str() {
            match error {
                "authorization_pending" => {
                    // Still waiting
                }
                "slow_down" => {
                    interval += std::time::Duration::from_secs(5);
                }
                "expired_token" => {
                    anyhow::bail!("GitHub device code expired. Please run auth again.");
                }
                "access_denied" => {
                    anyhow::bail!("GitHub login cancelled.");
                }
                _ => {
                    anyhow::bail!("GitHub OAuth error: {}", error);
                }
            }
        }

        tokio::time::sleep(interval).await;
    }

    anyhow::bail!("GitHub device code expired. Please run auth again.")
}

fn generate_random_string(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    rand::rng().fill_bytes(&mut bytes);
    general_purpose::URL_SAFE_NO_PAD.encode(&bytes)
}

fn generate_pkce() -> (String, String) {
    let verifier = generate_random_string(64);
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = general_purpose::URL_SAFE_NO_PAD.encode(hasher.finalize());
    (challenge, verifier)
}

fn wait_for_callback(port: u16, expected_state: &str) -> Result<String> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).with_context(|| {
        format!(
            "Failed to bind to port {}. Is another instance running?",
            port
        )
    })?;

    for stream in listener.incoming() {
        let mut stream = stream?;
        let mut reader = BufReader::new(&stream);
        let mut request_line = String::new();
        reader.read_line(&mut request_line)?;

        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        let path = parts[1];
        if let Ok(url) = Url::parse(&format!("http://localhost{}", path)) {
            let pairs: std::collections::HashMap<_, _> = url.query_pairs().collect();

            let mut state = None;
            let mut code = None;
            let mut error = None;

            for (key, value) in pairs {
                match key.as_ref() {
                    "state" => state = Some(value.to_string()),
                    "code" => code = Some(value.to_string()),
                    "error" => error = Some(value.to_string()),
                    _ => {}
                }
            }

            if let Some(state_val) = state
                && state_val != expected_state
            {
                let response = "HTTP/1.1 400 Bad Request\r\n\r\nInvalid state parameter.";
                stream.write_all(response.as_bytes())?;
                anyhow::bail!("Invalid state parameter received.");
            }

            if let Some(code_val) = code {
                let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html><body><h1>Authentication Successful</h1><p>You can close this window and return to the terminal.</p><script>window.close()</script></body></html>";
                stream.write_all(response.as_bytes())?;
                return Ok(code_val);
            } else if let Some(error_val) = error {
                let response = format!(
                    "HTTP/1.1 400 Bad Request\r\n\r\nAuthentication failed: {}",
                    error_val
                );
                stream.write_all(response.as_bytes())?;
                anyhow::bail!("Authentication failed: {}", error_val);
            }
        }
    }

    anyhow::bail!("Server closed without receiving code")
}

async fn exchange_code(
    code: &str,
    verifier: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<TokenResponse> {
    let client = Client::new();
    let params = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("code", code),
        ("grant_type", "authorization_code"),
        ("redirect_uri", GEMINI_REDIRECT_URI),
        ("code_verifier", verifier),
    ];

    let body = serde_urlencoded::to_string(params)?;

    let response = client
        .post("https://oauth2.googleapis.com/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        anyhow::bail!("Failed to exchange code for tokens: {}", error_text);
    }

    let tokens: TokenResponse = response.json().await?;
    Ok(tokens)
}

async fn update_config_github(
    access_token: &str,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    client_id: String,
    client_secret: Option<String>,
) -> Result<()> {
    let mut config = Config::load().unwrap_or_else(|_| Config::default());

    let expires_at = expires_in.map(|ei| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + ei
    });

    config.providers.github_copilot = Some(GitHubOAuthConfig {
        access_token: access_token.to_string(),
        refresh_token,
        client_id: Some(client_id),
        client_secret,
        expires_at,
    });

    config.agent.default_model = "github/copilot".to_string();

    config.save()?;
    Ok(())
}

async fn update_config_gemini(
    tokens: TokenResponse,
    client_id: String,
    client_secret: String,
    project_id: Option<String>,
) -> Result<()> {
    let mut config = Config::load().unwrap_or_else(|_| Config::default());

    let expires_at = Some(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + tokens.expires_in,
    );

    let gemini_config = GeminiOAuthConfig {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        client_id: Some(client_id),
        client_secret: Some(client_secret),
        expires_at,
        base_url: "https://generativelanguage.googleapis.com".to_string(),
        project_id,
    };

    config.providers.gemini_oauth = Some(gemini_config);
    config.agent.default_model = "gemini/gemini-3-pro-preview".to_string();

    config.save()?;
    Ok(())
}
