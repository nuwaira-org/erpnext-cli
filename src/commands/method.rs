use crate::client::Client;
use crate::error::ClientError;
use serde_json::Value;

/// Call a whitelisted server method.
pub fn call(client: &Client, method: &str, data: Option<&str>) -> Result<Value, ClientError> {
    let body = if let Some(json_str) = data {
        serde_json::from_str(json_str)
            .map_err(|e| ClientError::Config(format!("invalid JSON data: {e}")))?
    } else {
        serde_json::json!({})
    };

    let path = format!("/api/method/{method}");
    client.post(&path, &body)
}
