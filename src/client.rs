use crate::config::Config;
use crate::error::ClientError;
use reqwest::header::{HeaderMap, SET_COOKIE};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::time::Duration;

/// Blocking HTTP client for ERPNext REST API.
pub struct Client {
    inner: reqwest::blocking::Client,
    base_url: String,
    auth_header: Option<String>,
    session_id: Option<String>,
    verbose: bool,
}

impl Client {
    /// Create a new client from configuration.
    pub fn from_config(config: &Config, verbose: bool) -> Result<Self, ClientError> {
        if !config.is_ready() {
            return Err(ClientError::Config(
                "configuration is incomplete. Run 'erpnext config' to set up.".into(),
            ));
        }

        let base_url = config.url.trim_end_matches('/').to_string();
        let auth_header = if config.auth_type == "token" {
            Some(format!("token {}:{}", config.api_key, config.api_secret))
        } else {
            None
        };
        let session_id = if config.auth_type == "password" {
            Some(config.session_id.clone())
        } else {
            None
        };

        let inner = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()?;

        Ok(Self {
            inner,
            base_url,
            auth_header,
            session_id,
            verbose,
        })
    }

    /// Authenticate with username/password and return the session ID.
    pub fn login_with_password(
        config: &Config,
        username: &str,
        password: &str,
        verbose: bool,
    ) -> Result<(Value, String), ClientError> {
        if !config.can_login() {
            return Err(ClientError::Config(
                "password login requires a URL and auth_type \"password\". \
                 Run 'erpnext config set-auth-password' first."
                    .into(),
            ));
        }

        let base_url = config.url.trim_end_matches('/');
        let url = format!("{base_url}/api/method/login");
        let body = serde_json::json!({
            "usr": username,
            "pwd": password,
        });

        let inner = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()?;

        if verbose {
            eprintln!("[DEBUG] POST {url}");
            eprintln!(
                "[DEBUG] Body: {}",
                serde_json::to_string_pretty(&body).unwrap_or_default()
            );
        }

        let resp = inner.post(&url).json(&body).send()?;
        let status = resp.status().as_u16();
        let headers = resp.headers().clone();
        let body_text = resp.text()?;

        if verbose {
            eprintln!("[DEBUG] {status}");
            let truncated: String = body_text.chars().take(500).collect();
            eprintln!("[DEBUG] Response: {truncated}");
        }

        if !(200..300).contains(&status) {
            return Err(ClientError::from_response(status, body_text));
        }

        let session_id = extract_session_id(&headers).ok_or_else(|| {
            ClientError::Config(
                "login succeeded but no session cookie was returned by the server".into(),
            )
        })?;

        let value: Value = serde_json::from_str(&body_text).map_err(ClientError::Deserialize)?;
        Ok((value, session_id))
    }

    /// GET a resource or method endpoint.
    pub fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ClientError> {
        let url = format!("{}{path}", self.base_url);
        let req = self.apply_auth(self.inner.get(&url));

        if self.verbose {
            eprintln!("[DEBUG] GET {url}");
        }

        let resp = req.send()?;
        self.handle_response(resp)
    }

    /// POST JSON payload to an endpoint.
    pub fn post<T: DeserializeOwned>(&self, path: &str, body: &Value) -> Result<T, ClientError> {
        let url = format!("{}{path}", self.base_url);

        if self.verbose {
            eprintln!("[DEBUG] POST {url}");
            eprintln!(
                "[DEBUG] Body: {}",
                serde_json::to_string_pretty(body).unwrap_or_default()
            );
        }

        let resp = self.apply_auth(self.inner.post(&url)).json(body).send()?;
        self.handle_response(resp)
    }

    /// PUT JSON payload to an endpoint.
    pub fn put<T: DeserializeOwned>(&self, path: &str, body: &Value) -> Result<T, ClientError> {
        let url = format!("{}{path}", self.base_url);

        if self.verbose {
            eprintln!("[DEBUG] PUT {url}");
            eprintln!(
                "[DEBUG] Body: {}",
                serde_json::to_string_pretty(body).unwrap_or_default()
            );
        }

        let resp = self.apply_auth(self.inner.put(&url)).json(body).send()?;
        self.handle_response(resp)
    }

    /// DELETE a resource.
    pub fn delete(&self, path: &str) -> Result<(), ClientError> {
        let url = format!("{}{path}", self.base_url);

        if self.verbose {
            eprintln!("[DEBUG] DELETE {url}");
        }

        let resp = self.apply_auth(self.inner.delete(&url)).send()?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status().as_u16();
            let body = resp.text().unwrap_or_default();
            Err(ClientError::from_response(status, body))
        }
    }

    /// POST a multipart form to an endpoint.
    pub fn post_multipart<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        form: reqwest::blocking::multipart::Form,
    ) -> Result<T, ClientError> {
        let url = format!("{}{path}", self.base_url);

        if self.verbose {
            eprintln!("[DEBUG] POST (multipart) {url}");
        }

        let resp = self
            .apply_auth(self.inner.post(&url))
            .multipart(form)
            .send()?;
        self.handle_response(resp)
    }

    /// GET an endpoint and return the raw response bytes.
    /// Used for binary downloads (e.g. PDFs) where JSON parsing would fail.
    pub fn get_raw(&self, path: &str, query: &[(&str, &str)]) -> Result<Vec<u8>, ClientError> {
        let url = format!("{}{path}", self.base_url);

        if self.verbose {
            eprintln!("[DEBUG] GET (raw) {url}");
        }

        let resp = self.apply_auth(self.inner.get(&url)).query(query).send()?;

        let status = resp.status().as_u16();

        if self.verbose {
            eprintln!("[DEBUG] {status} ({:?})", resp.status());
        }

        if (200..300).contains(&status) {
            let bytes = resp.bytes()?;
            Ok(bytes.to_vec())
        } else {
            let body = resp.text().unwrap_or_default();
            if self.verbose {
                let truncated: String = body.chars().take(500).collect();
                eprintln!("[DEBUG] Response: {truncated}");
            }
            Err(ClientError::from_response(status, body))
        }
    }

    fn apply_auth(
        &self,
        mut req: reqwest::blocking::RequestBuilder,
    ) -> reqwest::blocking::RequestBuilder {
        if let Some(ref auth) = self.auth_header {
            req = req.header("Authorization", auth);
        }
        if let Some(ref sid) = self.session_id {
            req = req.header("Cookie", format!("sid={sid}"));
        }
        req
    }

    /// Handle an HTTP response: parse JSON on success, produce error on failure.
    fn handle_response<T: DeserializeOwned>(
        &self,
        resp: reqwest::blocking::Response,
    ) -> Result<T, ClientError> {
        let status = resp.status().as_u16();

        if self.verbose {
            eprintln!("[DEBUG] {status} ({:?})", resp.status());
        }

        let body = resp.text()?;

        if self.verbose {
            let truncated: String = body.chars().take(500).collect();
            eprintln!("[DEBUG] Response: {truncated}");
        }

        if (200..300).contains(&status) {
            serde_json::from_str(&body).map_err(ClientError::Deserialize)
        } else {
            Err(ClientError::from_response(status, body))
        }
    }

    /// Return the base URL (for display purposes).
    #[allow(dead_code)]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

/// Extract the ERPNext `sid` session cookie from response headers.
fn extract_session_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|cookie| {
            cookie.split(';').next().and_then(|pair| {
                let (name, value) = pair.split_once('=')?;
                if name.trim() == "sid" {
                    Some(value.to_string())
                } else {
                    None
                }
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    fn test_config(server: &MockServer) -> Config {
        Config {
            url: server.url(""),
            auth_type: "token".to_string(),
            api_key: "test-key".to_string(),
            api_secret: "test-secret".to_string(),
            session_id: String::new(),
            timeout_secs: 10,
        }
    }

    fn test_client(server: &MockServer) -> Client {
        Client::from_config(&test_config(server), false).unwrap()
    }

    #[test]
    fn get_success() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/api/method/test.endpoint")
                .header("Authorization", "token test-key:test-secret");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"message": "ok"}"#);
        });

        let client = test_client(&server);
        let result: serde_json::Value = client.get("/api/method/test.endpoint").unwrap();
        assert_eq!(result["message"], "ok");
        mock.assert();
    }

    #[test]
    fn get_auth_error() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/api/resource/Test");
            then.status(401)
                .header("content-type", "application/json")
                .body(r#"{"message": "Unauthorized"}"#);
        });

        let client = test_client(&server);
        let result: Result<serde_json::Value, _> = client.get("/api/resource/Test");
        assert!(result.is_err());
        match result.unwrap_err() {
            ClientError::Auth { status, .. } => assert_eq!(status, 401),
            other => panic!("expected Auth error, got {:?}", other),
        }
        mock.assert();
    }

    #[test]
    fn get_server_error_with_messages() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/api/resource/Test");
            then.status(500)
                .header("content-type", "application/json")
                .body(r#"{"_server_messages": ["[\"Something broke\"]"]}"#);
        });

        let client = test_client(&server);
        let result: Result<serde_json::Value, _> = client.get("/api/resource/Test");
        assert!(result.is_err());
        match result.unwrap_err() {
            ClientError::Server { status, .. } => {
                assert_eq!(status, 500);
            }
            other => panic!("expected Server error, got {:?}", other),
        }
        mock.assert();
    }

    #[test]
    fn post_success() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/api/resource/Test")
                .header("Authorization", "token test-key:test-secret")
                .json_body(serde_json::json!({"field": "value"}));
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"data": {"name": "TEST-001"}}"#);
        });

        let client = test_client(&server);
        let result: serde_json::Value = client
            .post("/api/resource/Test", &serde_json::json!({"field": "value"}))
            .unwrap();
        assert_eq!(result["data"]["name"], "TEST-001");
        mock.assert();
    }

    #[test]
    fn delete_success() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(DELETE)
                .path("/api/resource/Test/TEST-001")
                .header("Authorization", "token test-key:test-secret");
            then.status(202);
        });

        let client = test_client(&server);
        client.delete("/api/resource/Test/TEST-001").unwrap();
        mock.assert();
    }

    #[test]
    fn client_rejects_incomplete_config() {
        let config = Config::default();
        let result = Client::from_config(&config, false);
        assert!(matches!(result, Err(ClientError::Config(_))));
    }

    #[test]
    fn session_auth_sends_cookie_header() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/api/method/frappe.auth.get_logged_user")
                .header("Cookie", "sid=abc123");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"message": "Administrator"}"#);
        });

        let config = Config {
            url: server.url(""),
            auth_type: "password".to_string(),
            api_key: String::new(),
            api_secret: String::new(),
            session_id: "abc123".to_string(),
            timeout_secs: 10,
        };
        let client = Client::from_config(&config, false).unwrap();
        let result: serde_json::Value = client
            .get("/api/method/frappe.auth.get_logged_user")
            .unwrap();
        assert_eq!(result["message"], "Administrator");
        mock.assert();
    }

    #[test]
    fn login_with_password_extracts_session() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/api/method/login")
                .json_body(serde_json::json!({"usr": "admin", "pwd": "secret"}));
            then.status(200)
                .header("content-type", "application/json")
                .header("set-cookie", "sid=session789; Path=/; HttpOnly")
                .body(r#"{"message": "Logged In"}"#);
        });

        let config = Config {
            url: server.url(""),
            auth_type: "password".to_string(),
            api_key: String::new(),
            api_secret: String::new(),
            session_id: String::new(),
            timeout_secs: 10,
        };

        let (value, sid) = Client::login_with_password(&config, "admin", "secret", false).unwrap();
        assert_eq!(value["message"], "Logged In");
        assert_eq!(sid, "session789");
        mock.assert();
    }

    #[test]
    fn extract_session_id_parses_set_cookie() {
        use reqwest::header::HeaderValue;

        let mut headers = HeaderMap::new();
        headers.insert(
            SET_COOKIE,
            HeaderValue::from_static("sid=abc123; Path=/; HttpOnly"),
        );
        assert_eq!(extract_session_id(&headers).as_deref(), Some("abc123"));
    }
}
