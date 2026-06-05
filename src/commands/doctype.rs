use crate::client::Client;
use crate::error::ClientError;
use serde_json::Value;
use std::fs;
use url::Url;

/// Read JSON data from --data flag or --file flag.
/// --file takes precedence if both are provided.
fn read_data(data: Option<&str>, file: Option<&str>) -> Result<Value, ClientError> {
    if let Some(path) = file {
        let content = fs::read_to_string(path)
            .map_err(|e| ClientError::Config(format!("failed to read file {path}: {e}")))?;
        serde_json::from_str(&content)
            .map_err(|e| ClientError::Config(format!("invalid JSON in {path}: {e}")))
    } else if let Some(json_str) = data {
        serde_json::from_str(json_str)
            .map_err(|e| ClientError::Config(format!("invalid JSON: {e}")))
    } else {
        Err(ClientError::Config(
            "no data provided. Use --data '<JSON>' or --file <PATH>".into(),
        ))
    }
}

/// Build a percent-encoded `/api/resource/...` path for one or two path segments.
fn resource_path(doctype: &str, name: Option<&str>) -> Result<String, ClientError> {
    let mut url = Url::parse("http://_/api/resource")
        .map_err(|e| ClientError::Config(format!("invalid doctype name: {e}")))?;

    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| ClientError::Config("invalid resource path".into()))?;
        segments.push(doctype);
        if let Some(doc_name) = name {
            segments.push(doc_name);
        }
    }

    Ok(url.path().to_string())
}

/// List documents of a DocType.
pub fn list(
    client: &Client,
    doctype: &str,
    fields: &str,
    filters: Option<&str>,
    order_by: Option<&str>,
    limit_start: u64,
    limit_page_length: u64,
) -> Result<Value, ClientError> {
    let path = resource_path(doctype, None)?;
    let mut url = Url::parse(&format!("http://_{path}"))
        .map_err(|e| ClientError::Config(format!("invalid doctype name: {e}")))?;

    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("fields", fields);
        pairs.append_pair("limit_start", &limit_start.to_string());
        pairs.append_pair("limit_page_length", &limit_page_length.to_string());
        if let Some(f) = filters {
            pairs.append_pair("filters", f);
        }
        if let Some(o) = order_by {
            pairs.append_pair("order_by", o);
        }
    }

    let request_path = match url.query() {
        Some(query) => format!("{path}?{query}"),
        None => path,
    };

    client.get(&request_path)
}

/// Get a single document by name.
pub fn get(client: &Client, doctype: &str, name: &str) -> Result<Value, ClientError> {
    let path = resource_path(doctype, Some(name))?;
    client.get(&path)
}

/// Create a new document.
pub fn create(
    client: &Client,
    doctype: &str,
    data: Option<&str>,
    file: Option<&str>,
) -> Result<Value, ClientError> {
    let body = read_data(data, file)?;
    let path = resource_path(doctype, None)?;
    client.post(&path, &body)
}

/// Update an existing document.
pub fn update(
    client: &Client,
    doctype: &str,
    name: &str,
    data: Option<&str>,
    file: Option<&str>,
) -> Result<Value, ClientError> {
    let body = read_data(data, file)?;
    let path = resource_path(doctype, Some(name))?;
    client.put(&path, &body)
}

/// Delete a document.
pub fn delete(client: &Client, doctype: &str, name: &str) -> Result<(), ClientError> {
    let path = resource_path(doctype, Some(name))?;
    client.delete(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_path_encodes_spaces() {
        let path = resource_path("Sales Invoice", None).unwrap();
        assert_eq!(path, "/api/resource/Sales%20Invoice");
    }

    #[test]
    fn resource_path_encodes_doctype_and_name() {
        let path = resource_path("Sales Invoice", Some("SINV-0001")).unwrap();
        assert_eq!(path, "/api/resource/Sales%20Invoice/SINV-0001");
    }
}
