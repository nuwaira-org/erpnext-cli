mod cli;
mod client;
mod commands;
mod config;
mod error;

use clap::Parser;
use cli::{Commands, ConfigCommand, DoctypeCommand, OutputFormat};
use client::Client;
use colored::Colorize;
use serde_json::Value;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args = cli::Cli::parse();

    // Load config, then apply CLI flag overrides
    let mut config = config::Config::load().unwrap_or_default();
    if let Some(ref url) = args.url {
        config.url = url.clone();
    }
    if let Some(ref token) = args.token {
        if let Some((key, secret)) = token.split_once(':') {
            config.api_key = key.to_string();
            config.api_secret = secret.to_string();
            config.auth_type = "token".to_string();
        }
    }

    let verbose = args.verbose;

    match &args.command {
        // ---- Config subcommands (no client needed) ----
        Commands::Config { action } => match action {
            ConfigCommand::SetUrl { url } => {
                let mut cfg = config::Config::load().unwrap_or_default();
                cfg.url = url.clone();
                cfg.save().map_err(|e| color_eyre::eyre::eyre!("{e}"))?;
                println!("URL set to: {url}");
            }
            ConfigCommand::SetAuthToken {
                api_key,
                api_secret,
            } => {
                let mut cfg = config::Config::load().unwrap_or_default();
                cfg.auth_type = "token".to_string();
                cfg.api_key = api_key.clone();
                cfg.api_secret = api_secret.clone();
                cfg.session_id.clear();
                cfg.save().map_err(|e| color_eyre::eyre::eyre!("{e}"))?;
                println!("Auth set to token-based.");
            }
            ConfigCommand::SetAuthPassword => {
                let mut cfg = config::Config::load().unwrap_or_default();
                cfg.auth_type = "password".to_string();
                cfg.api_key.clear();
                cfg.api_secret.clear();
                cfg.session_id.clear();
                cfg.save().map_err(|e| color_eyre::eyre::eyre!("{e}"))?;
                println!("Auth set to password-based. Use 'erpnext login' to authenticate.");
            }
            ConfigCommand::Show => {
                let cfg = config::Config::load().unwrap_or_default();
                // Mask secrets
                let api_key_display = if cfg.api_key.is_empty() {
                    "(not set)".to_string()
                } else {
                    format!(
                        "{}...{}",
                        &cfg.api_key[..4.min(cfg.api_key.len())],
                        "(masked)"
                    )
                };
                let api_secret_display = if cfg.api_secret.is_empty() {
                    "(not set)".to_string()
                } else {
                    "(set, masked)".to_string()
                };

                println!("URL:        {}", cfg.url);
                println!("Auth type:  {}", cfg.auth_type);
                println!("API key:    {api_key_display}");
                println!("API secret: {api_secret_display}");
                let session_display = if cfg.session_id.is_empty() {
                    "(not set)".to_string()
                } else {
                    "(set, masked)".to_string()
                };
                println!("Session:    {session_display}");
                println!("Timeout:    {}s", cfg.timeout_secs);
            }
        },

        // ---- Login (password bootstrap; no API client yet) ----
        Commands::Login => {
            let mut cfg = config::Config::load().unwrap_or_default();
            if let Some(ref url) = args.url {
                cfg.url = url.clone();
            }
            match commands::auth::login(&mut cfg, verbose) {
                Ok(value) => {
                    print_output(value, &args.output);
                }
                Err(e) => {
                    eprintln!("{} {}", "error:".red().bold(), e);
                    std::process::exit(1);
                }
            }
        }

        // ---- Commands that require a client ----
        _ => {
            let client = Client::from_config(&config, verbose)
                .map_err(|e| color_eyre::eyre::eyre!("{e}"))?;

            let result = match &args.command {
                Commands::Whoami => commands::auth::whoami(&client),
                Commands::Doctype { action } => match action {
                    DoctypeCommand::List {
                        doctype,
                        fields,
                        filters,
                        order_by,
                        limit_start,
                        limit_page_length,
                    } => commands::doctype::list(
                        &client,
                        doctype,
                        fields,
                        filters.as_deref(),
                        order_by.as_deref(),
                        *limit_start,
                        *limit_page_length,
                    ),
                    DoctypeCommand::Get { doctype, name } => {
                        commands::doctype::get(&client, doctype, name)
                    }
                    DoctypeCommand::Create {
                        doctype,
                        data,
                        file,
                    } => commands::doctype::create(
                        &client,
                        doctype,
                        data.as_deref(),
                        file.as_deref(),
                    ),
                    DoctypeCommand::Update {
                        doctype,
                        name,
                        data,
                        file,
                    } => commands::doctype::update(
                        &client,
                        doctype,
                        name,
                        data.as_deref(),
                        file.as_deref(),
                    ),
                    DoctypeCommand::Delete { doctype, name } => {
                        commands::doctype::delete(&client, doctype, name)
                            .map(|_| serde_json::json!({"success": true}))
                    }
                },
                Commands::Call { method, data } => {
                    commands::method::call(&client, method, data.as_deref())
                }
                Commands::GenerateCompletions { shell } => {
                    use clap::CommandFactory;
                    use clap_complete::generate;
                    let mut cmd = cli::Cli::command();
                    let name = cmd.get_name().to_string();
                    generate(*shell, &mut cmd, &name, &mut std::io::stdout());
                    return Ok(());
                }
                Commands::Upload(args) => commands::upload::upload(&client, args),
                Commands::DownloadPdf(args) => commands::download_pdf::download(&client, args),
                // Already handled above
                Commands::Config { .. } | Commands::Login => unreachable!(),
            };

            match result {
                Ok(value) => print_output(value, &args.output),
                Err(e) => {
                    eprintln!("{} {}", "error:".red().bold(), e);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

/// Format and print a JSON value in the requested output format.
fn print_output(value: Value, format: &OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&value).unwrap_or_default()
            );
        }
        OutputFormat::Table => {
            // If the response has a "data" field (list results), render as table
            if let Some(data) = value.get("data").and_then(|d| d.as_array()) {
                if data.is_empty() {
                    println!("(empty)");
                    return;
                }

                // Collect all unique keys across all items
                let mut keys = Vec::new();
                let mut seen = std::collections::HashSet::new();
                for item in data {
                    if let Some(obj) = item.as_object() {
                        for k in obj.keys() {
                            if seen.insert(k) {
                                keys.push(k.clone());
                            }
                        }
                    }
                }

                // Build rows
                let mut builder = tabled::builder::Builder::new();
                builder.push_record(
                    keys.iter()
                        .map(|k| k.bold().to_string())
                        .collect::<Vec<_>>(),
                );

                for item in data {
                    let row: Vec<String> = keys
                        .iter()
                        .map(|k| {
                            item.get(k)
                                .map(|v| match v {
                                    Value::String(s) => s.clone(),
                                    Value::Null => "-".to_string(),
                                    other => other.to_string(),
                                })
                                .unwrap_or_else(|| "-".to_string())
                        })
                        .collect();
                    builder.push_record(row);
                }

                let table = builder.build();
                println!("{table}");
            } else {
                // Single document: display as vertical key-value pairs
                if let Some(obj) = value.as_object() {
                    let max_key_len = obj.keys().map(|k| k.len()).max().unwrap_or(0);
                    for (k, v) in obj {
                        let val_str = match v {
                            Value::String(s) => s.clone(),
                            Value::Null => "-".to_string(),
                            other => other.to_string(),
                        };
                        println!("{:>width$}  {}", k.bold(), val_str, width = max_key_len);
                    }
                } else {
                    // Fall back to JSON
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&value).unwrap_or_default()
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_output_json_format() {
        let value = serde_json::json!({"message": "hello"});
        print_output(value, &OutputFormat::Json);
        // Cannot easily capture stdout in unit test, but this validates no panic
    }
}
