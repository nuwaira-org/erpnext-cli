use crate::cli::UploadArgs;
use crate::client::Client;
use crate::error::ClientError;
use reqwest::blocking::multipart::{Form, Part};
use serde_json::Value;
use std::fs;
use std::io::{self, Read};

/// Upload a file to ERPNext via multipart/form-data POST to `/api/method/upload_file`.
pub fn upload(client: &Client, args: &UploadArgs) -> Result<Value, ClientError> {
    // 1. Read file content (from path or stdin)
    let (raw_data, filename) = if args.file == "-" {
        let mut buf = Vec::new();
        io::stdin()
            .read_to_end(&mut buf)
            .map_err(|e| ClientError::Config(format!("failed to read stdin: {e}")))?;
        let fname = args.filename.clone().unwrap_or_else(|| "stdin".to_string());
        (buf, fname)
    } else {
        let data = fs::read(&args.file)
            .map_err(|e| ClientError::Config(format!("failed to read file {}: {e}", args.file)))?;
        let fname = args
            .filename
            .clone()
            .or_else(|| {
                std::path::Path::new(&args.file)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| "unknown".to_string());
        (data, fname)
    };

    // 2. Decode base64 if requested
    let file_data = if args.base64 {
        let encoded = String::from_utf8(raw_data)
            .map_err(|e| ClientError::Config(format!("base64 input is not valid UTF-8: {e}")))?;
        let encoded = encoded.trim();
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| ClientError::Config(format!("base64 decode failed: {e}")))?
    } else {
        raw_data
    };

    // 3. Warn if only one of doctype/docname is given
    if args.doctype.is_some() != args.docname.is_some() {
        eprintln!(
            "warning: --doctype and --docname should both be specified to attach the file to a document"
        );
    }

    // 4. Detect MIME type
    let mime_type = mime_guess::from_path(&filename).first_or_octet_stream();

    // 5. Build multipart form
    let part = Part::bytes(file_data)
        .file_name(filename)
        .mime_str(mime_type.as_ref())
        .map_err(|e| ClientError::Config(format!("invalid MIME type: {e}")))?;

    let is_private = if args.public { "0" } else { "1" };

    let mut form = Form::new()
        .part("file", part)
        .text("is_private", is_private);

    if let Some(ref doctype) = args.doctype {
        form = form.text("doctype", doctype.clone());
    }
    if let Some(ref docname) = args.docname {
        form = form.text("docname", docname.clone());
    }

    // 6. Send
    client.post_multipart("/api/method/upload_file", form)
}

#[cfg(test)]
mod tests {
    // Integration tests are preferred; unit tests for helper logic go here.
}
