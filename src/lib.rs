//! HTTP upload server with QR pairing, and the browser-upload library behind
//! ExifFlow's desktop app.
//!
//! Ships as a standalone CLI (its own binary, like `rftps`) and as a library.
//! A sibling of `rftps`: same upload root, same event bus, same durable record
//! store. A phone scans a QR code, pairs with a URL-fragment token, and uploads
//! files from the browser. Uploads feed the same replication pipeline as FTP.

pub mod auth;
pub mod config;
pub mod qr;
pub mod server;

pub use config::Args;
pub use server::{HttpServer, HttpServerInfo};

#[cfg(feature = "include-pem-files")]
pub const EMBEDDED_CERT: &[u8] = include_bytes!("../cert.pem");
#[cfg(feature = "include-pem-files")]
pub const EMBEDDED_KEY: &[u8] = include_bytes!("../key.pem");
