use thiserror::Error;

/// All errors that can occur when interacting with the ERPNext API.
#[derive(Error, Debug)]
pub enum ClientError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("HTTP {status}: {body}")]
    Http { status: u16, body: String },

    #[error("authentication failed ({status}): {body}")]
    Auth { status: u16, body: String },

    #[error("server error ({status}): {messages:?}")]
    Server { status: u16, messages: Vec<String> },

    #[error("failed to deserialize response: {0}")]
    Deserialize(#[source] serde_json::Error),

    #[error("config error: {0}")]
    Config(String),
}

impl ClientError {
    /// Convenience constructor for HTTP errors from a response.
    pub fn from_response(status: u16, body: String) -> Self {
        match status {
            401 | 403 => ClientError::Auth { status, body },
            500.. => {
                // Try to parse _server_messages from the JSON body
                let messages = parse_server_messages(&body);
                ClientError::Server { status, messages }
            }
            _ => ClientError::Http { status, body },
        }
    }
}

/// Parse the `_server_messages` field from an ERPNext JSON response.
///
/// ERPNext wraps server-side messages in a JSON-encoded string within a list:
/// `{"_server_messages": ["[\"message\": \"Something went wrong\"]"]}`
fn parse_server_messages(body: &str) -> Vec<String> {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(body) else {
        return vec![body.to_string()];
    };

    let Some(messages) = json.get("_server_messages").and_then(|v| v.as_array()) else {
        return vec![body.to_string()];
    };

    messages
        .iter()
        .filter_map(|m| {
            let raw = m.as_str()?;
            // ERPNext encodes _server_messages as JSON within JSON.
            // Common format: ["[\"message1\"]", "[\"message2\"]"]
            // Each element is a JSON-encoded string or array of strings.
            let inner: serde_json::Value = serde_json::from_str(raw).ok()?;
            match &inner {
                // Case 1: "\"message text\"" -> a JSON string
                serde_json::Value::String(s) => Some(s.clone()),
                // Case 2: "[\"message text\"]" -> a JSON array of strings
                serde_json::Value::Array(arr) => {
                    arr.first().and_then(|v| v.as_str()).map(|s| s.to_string())
                }
                _ => None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_server_messages_valid() {
        let body = r#"{"_server_messages": ["[\"Duplicate entry\"]", "[\"Validation failed\"]"]}"#;
        let msgs = parse_server_messages(body);
        assert_eq!(msgs, vec!["Duplicate entry", "Validation failed"]);
    }

    #[test]
    fn parse_server_messages_no_field() {
        let body = r#"{"data": []}"#;
        let msgs = parse_server_messages(body);
        assert_eq!(msgs, vec!["{\"data\": []}"]);
    }

    #[test]
    fn parse_server_messages_invalid_json() {
        let body = "not json";
        let msgs = parse_server_messages(body);
        assert_eq!(msgs, vec!["not json"]);
    }
}
