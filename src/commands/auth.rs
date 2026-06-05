use crate::client::Client;
use crate::config::Config;
use crate::error::ClientError;
use serde_json::Value;
use std::io::{self, Write};

/// Handle login via password authentication and persist the session.
pub fn login(config: &mut Config, verbose: bool) -> Result<Value, ClientError> {
    let mut username = String::new();
    let mut password = String::new();

    print!("Username: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut username).unwrap();

    print!("Password: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut password).unwrap();

    let (value, session_id) =
        Client::login_with_password(config, username.trim(), password.trim(), verbose)?;

    config.session_id = session_id;
    config.save()?;

    Ok(value)
}

/// Show the currently authenticated user.
pub fn whoami(client: &Client) -> Result<Value, ClientError> {
    client.get("/api/method/frappe.auth.get_logged_user")
}
