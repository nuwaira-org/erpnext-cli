use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

/// CLI tool for interacting with ERPNext via its REST API.
#[derive(Parser)]
#[command(name = "erpnext", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output (logs HTTP requests and responses)
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Output format: json or table
    #[arg(short, long, global = true, value_enum, default_value_t = OutputFormat::default())]
    pub output: OutputFormat,

    /// Token to use for API auth (format: api_key:api_secret)
    #[arg(long, global = true, env = "ERPNEXT_TOKEN")]
    pub token: Option<String>,

    /// URL of the ERPNext instance
    #[arg(long, global = true, env = "ERPNEXT_URL")]
    pub url: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigCommand,
    },

    /// Authenticate with password (bootstrap only)
    Login,

    /// Show current authenticated user
    Whoami,

    /// Interact with ERPNext DocTypes
    Doctype {
        #[command(subcommand)]
        action: DoctypeCommand,
    },

    /// Call a whitelisted server method
    Call {
        /// Dotted method path (e.g. "frappe.auth.get_logged_user")
        method: String,

        /// JSON data to pass as keyword arguments
        #[arg(short, long)]
        data: Option<String>,
    },

    /// Upload a file to ERPNext
    Upload(UploadArgs),

    /// Download a document as PDF
    DownloadPdf(DownloadPdfArgs),

    /// Generate shell completion scripts
    GenerateCompletions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Set the ERPNext instance URL
    SetUrl { url: String },

    /// Set authentication method to token-based
    SetAuthToken {
        /// API key
        api_key: String,
        /// API secret
        api_secret: String,
    },

    /// Set authentication method to password-based (for bootstrap login)
    SetAuthPassword,

    /// Display current configuration
    Show,
}

#[derive(Subcommand)]
pub enum DoctypeCommand {
    /// List documents of a DocType
    List {
        /// DocType name (e.g. "Sales Invoice")
        doctype: String,

        /// Comma-separated list of fields to return (default: "name")
        #[arg(long, default_value = "name")]
        fields: String,

        /// JSON array of filters (e.g. '[["status","=","Open"]]')
        #[arg(long)]
        filters: Option<String>,

        /// Field to order by
        #[arg(long)]
        order_by: Option<String>,

        /// Pagination start index
        #[arg(long, default_value = "0")]
        limit_start: u64,

        /// Page size
        #[arg(long, default_value = "20")]
        limit_page_length: u64,
    },

    /// Get a single document by name
    Get {
        /// DocType name
        doctype: String,
        /// Document name
        name: String,
    },

    /// Create a new document
    Create {
        /// DocType name
        doctype: String,
        /// JSON document data (inline or --file)
        #[arg(short, long)]
        data: Option<String>,
        /// Read document data from a file
        #[arg(long)]
        file: Option<String>,
    },

    /// Update an existing document
    Update {
        /// DocType name
        doctype: String,
        /// Document name
        name: String,
        /// JSON data with fields to update (inline or --file)
        #[arg(short, long)]
        data: Option<String>,
        /// Read update data from a file
        #[arg(long)]
        file: Option<String>,
    },

    /// Delete a document
    Delete {
        /// DocType name
        doctype: String,
        /// Document name
        name: String,
    },
}

/// Arguments for the `upload` subcommand.
#[derive(Args)]
pub struct UploadArgs {
    /// Path to the file to upload, or "-" to read from stdin
    pub file: String,

    /// DocType to attach the file to
    #[arg(long)]
    pub doctype: Option<String>,

    /// Document name to attach the file to
    #[arg(long)]
    pub docname: Option<String>,

    /// File name to use (defaults to the source file name)
    #[arg(long)]
    pub filename: Option<String>,

    /// Make the uploaded file public (default: private)
    #[arg(long)]
    pub public: bool,

    /// Decode file content from base64 before uploading
    #[arg(long)]
    pub base64: bool,
}

/// Arguments for the `download-pdf` subcommand.
#[derive(Args)]
pub struct DownloadPdfArgs {
    /// DocType name (e.g. "Sales Invoice")
    pub doctype: String,

    /// Document name (e.g. "SINV-0001")
    pub name: String,

    /// Print format name
    #[arg(long, alias = "print-format", default_value = "Standard")]
    pub format: String,

    /// Omit letterhead
    #[arg(long)]
    pub no_letterhead: bool,

    /// Output file path (writes to stdout if omitted)
    #[arg(short, long)]
    pub output: Option<String>,
}

#[derive(Clone, ValueEnum)]
pub enum OutputFormat {
    /// Pretty-printed JSON (default when piped)
    Json,
    /// Formatted table (default for TTY)
    Table,
}

impl Default for OutputFormat {
    fn default() -> Self {
        use std::io::IsTerminal;
        if std::io::stdout().is_terminal() {
            OutputFormat::Table
        } else {
            OutputFormat::Json
        }
    }
}
