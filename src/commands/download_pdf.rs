use crate::cli::DownloadPdfArgs;
use crate::client::Client;
use crate::error::ClientError;
use serde_json::Value;
use std::io::Write;

/// Download a document as PDF via `frappe.utils.print_format.download_pdf`.
pub fn download(client: &Client, args: &DownloadPdfArgs) -> Result<Value, ClientError> {
    let no_lh = if args.no_letterhead { "1" } else { "0" };

    let query: &[(&str, &str)] = &[
        ("doctype", &args.doctype),
        ("name", &args.name),
        ("format", &args.format),
        ("no_letterhead", no_lh),
    ];

    let bytes = client.get_raw("/api/method/frappe.utils.print_format.download_pdf", query)?;

    if let Some(output) = &args.output {
        std::fs::write(output, &bytes)
            .map_err(|e| ClientError::Config(format!("failed to write {}: {}", output, e)))?;
        eprintln!("PDF saved to {}", output);
    } else {
        std::io::stdout()
            .write_all(&bytes)
            .map_err(|e| ClientError::Config(format!("failed to write to stdout: {}", e)))?;
    }

    Ok(serde_json::json!({
        "success": true,
        "doctype": args.doctype,
        "name": args.name,
        "format": args.format,
        "size": bytes.len(),
    }))
}
