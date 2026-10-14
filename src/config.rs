use clap::Parser;

/// Command line arguments for the HTTP upload server.
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Server bind address
    #[arg(short, long, default_value = "0.0.0.0")]
    pub address: String,

    /// Port to run the HTTP server on
    #[arg(short = 'p', long, default_value = "41213")]
    pub port: u16,

    /// Directory where uploaded files will be stored
    #[arg(short, long, default_value = "./recv")]
    pub directory: String,

    /// Username reported for uploads and used when --require-password is set
    #[arg(short, long, value_parser = validate_username, default_value = "recv")]
    pub username: String,

    /// Password used when --require-password is set (auto-generated if omitted)
    #[arg(short = 'P', long)]
    pub password: Option<String>,

    /// Require username and password in addition to the pairing token
    #[arg(long)]
    pub require_password: bool,

    /// Enable HTTPS
    #[arg(long, value_parser = clap::builder::BoolishValueParser::new(), default_value = "false")]
    pub enable_https: Option<bool>,

    /// Certificate PEM file (HTTPS)
    #[arg(long)]
    pub cert_pem: Option<String>,

    /// Key PEM file (HTTPS)
    #[arg(long)]
    pub key_pem: Option<String>,

    /// Host advertised in the QR URL (NAT / container). Defaults to the LAN IP.
    #[arg(long)]
    pub external_host: Option<String>,

    /// Path to the durable replication store (SQLite); records uploads for the replicator
    #[arg(long)]
    pub record_store: Option<String>,

    /// Fixed pairing token (a random one is generated if omitted)
    #[arg(long)]
    pub token: Option<String>,

    /// Do not print the QR code on startup
    #[arg(long)]
    pub no_qr: bool,

    /// Maximum accepted upload size in bytes (unlimited if omitted)
    #[arg(long)]
    pub max_upload_size: Option<u64>,
}

pub fn validate_username(username: &str) -> Result<String, String> {
    if username.chars().all(|c| c.is_alphanumeric()) {
        Ok(username.to_string())
    } else {
        Err(String::from(
            "Username must contain only letters and numbers.",
        ))
    }
}
