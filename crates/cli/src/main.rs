/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use std::{
    collections::HashMap,
    io::{BufRead, Write},
};

use clap::Parser;
use console::style;
use jmap_client::client::Credentials;
use modules::{
    UnwrapResult,
    cli::{Cli, Client, Commands},
    is_localhost,
};

use crate::modules::OAuthResponse;

pub mod modules;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Cli::parse();
    let url = args
        .url
        .or_else(|| std::env::var("URL").ok())
        .map(|url| url.trim_end_matches('/').to_string())
        .unwrap_or_else(|| {
            eprintln!("No URL specified. Use --url or set the URL environment variable.");
            std::process::exit(1);
        });
    let client = Client {
        credentials: if let Some(credentials) = args.credentials {
            parse_credentials(&credentials)
        } else if let Ok(credentials) = std::env::var("CREDENTIALS") {
            parse_credentials(&credentials)
        } else {
            let credentials = rpassword::prompt_password(
                "\nEnter administrator credentials or press [ENTER] to use OAuth: ",
            )
            .unwrap();
            if !credentials.is_empty() {
                parse_credentials(&credentials)
            } else {
                oauth(&url).await
            }
        },
        timeout: args.timeout,
        url,
    };

    match args.command {
        Commands::Account(command) => {
            command.exec(client).await;
        }
        Commands::Domain(command) => {
            command.exec(client).await;
        }
        Commands::List(command) => {
            command.exec(client).await;
        }
        Commands::Group(command) => {
            command.exec(client).await;
        }
        Commands::Import(command) => {
            command.exec(client).await;
        }
        Commands::Export(command) => {
            command.exec(client).await;
        }
        Commands::Server(command) => command.exec(client).await,
        Commands::Dkim(command) => command.exec(client).await,
        Commands::Queue(command) => command.exec(client).await,
        Commands::Report(command) => command.exec(client).await,
    }

    Ok(())
}

fn parse_credentials(credentials: &str) -> Credentials {
    if let Some((account, secret)) = credentials.split_once(':') {
        Credentials::basic(account, secret)
    } else {
        Credentials::basic("admin", credentials)
    }
}

async fn oauth(url: &str) -> Credentials {
    let metadata: HashMap<String, serde_json::Value> = serde_json::from_slice(
        &reqwest::Client::builder()
            .danger_accept_invalid_certs(is_localhost(url))
            .build()
            .unwrap_or_default()
            .get(format!("{}/.well-known/oauth-authorization-server", url))
            .send()
            .await
            .unwrap_result("send OAuth GET request")
            .bytes()
            .await
            .unwrap_result("fetch bytes"),
    )
    .unwrap_result("deserialize OAuth GET response");

    let token_endpoint = metadata.property("token_endpoint");
    let mut params: HashMap<String, String> =
        HashMap::from_iter([("client_id".to_string(), "Stalwart_CLI".to_string())]);
    let response: HashMap<String, serde_json::Value> = serde_json::from_slice(
        &reqwest::Client::builder()
            .danger_accept_invalid_certs(is_localhost(url))
            .build()
            .unwrap_or_default()
            .post(metadata.property("device_authorization_endpoint"))
            .form(&params)
            .send()
            .await
            .unwrap_result("send OAuth POST request")
            .bytes()
            .await
            .unwrap_result("fetch bytes"),
    )
    .unwrap_result("deserialize OAuth POST response");

    params.insert(
        "grant_type".to_string(),
        "urn:ietf:params:oauth:grant-type:device_code".to_string(),
    );
    params.insert(
        "device_code".to_string(),
        response.property("device_code").to_string(),
    );

    print!(
        "\nAuthenticate this request using code {} at {}. Please ENTER when done.",
        style(response.property("user_code")).bold(),
        style(response.property("verification_uri")).bold().dim()
    );

    std::io::stdout().flush().unwrap();
    std::io::stdin().lock().lines().next();

    let mut response: HashMap<String, serde_json::Value> = serde_json::from_slice(
        &reqwest::Client::builder()
            .danger_accept_invalid_certs(is_localhost(url))
            .build()
            .unwrap_or_default()
            .post(token_endpoint)
            .form(&params)
            .send()
            .await
            .unwrap_result("send OAuth POST request")
            .bytes()
            .await
            .unwrap_result("fetch bytes"),
    )
    .unwrap_result("deserialize OAuth POST response");

    if let Some(serde_json::Value::String(access_token)) = response.remove("access_token") {
        Credentials::Bearer(access_token)
    } else {
        eprintln!(
            "OAuth failed with code {}.",
            response
                .get("error")
                .and_then(|s| s.as_str())
                .unwrap_or("<unknown>")
        );
        std::process::exit(1);
    }
}

// Client implementation is now in modules/client.rs
