use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::multipart::Field;
use axum::extract::{DefaultBodyLimit, Multipart, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use exifflow_core::event::{EventBus, FtpEvent};
use exifflow_core::record::RecordStore;

use crate::auth::{SessionStore, constant_time_eq};

const INDEX_HTML: &str = include_str!("../assets/index.html");
const SESSION_COOKIE: &str = "recv_session";

/// Runtime configuration for the HTTP upload server.
pub struct HttpServer {
    addr: SocketAddr,
    upload_root: PathBuf,
    username: String,
    password: Option<String>,
    require_password: bool,
    enable_https: bool,
    cert_pem: Option<String>,
    key_pem: Option<String>,
    token: String,
    advertised_host: String,
    event_bus: Option<EventBus>,
    record_store: Option<RecordStore>,
    max_upload_size: Option<u64>,
}

/// Everything the caller needs to display the server to the user.
#[derive(Debug, Clone, Serialize)]
pub struct HttpServerInfo {
    pub url: String,
    pub address: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub token: String,
    pub qr_data_url: String,
    pub https: bool,
}

struct AppState {
    upload_root: PathBuf,
    username: String,
    password: Option<String>,
    require_password: bool,
    token: String,
    sessions: SessionStore,
    event_bus: Option<EventBus>,
    record_store: Option<RecordStore>,
    secure_cookie: bool,
}

impl HttpServer {
    pub fn new(args: crate::config::Args) -> Result<Self, String> {
        let upload_root = exifflow_core::utils::verify_home(args.directory.clone())?;

        let addr: SocketAddr = format!("{}:{}", args.address, args.port)
            .parse()
            .map_err(|e| format!("Failed to parse address: {}", e))?;

        let enable_https = args.enable_https.unwrap_or(false);
        let require_password = args.require_password;

        let mut password = args.password.clone().filter(|p| !p.is_empty());
        if require_password && password.is_none() {
            password = Some(exifflow_core::utils::generate_random_string(10));
        }

        let token = args
            .token
            .clone()
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| exifflow_core::utils::generate_random_string(32));

        let advertised_host = match args.external_host.clone().filter(|h| !h.trim().is_empty()) {
            Some(host) => host,
            None => exifflow_core::utils::resolve_local_ip()
                .map(|s| s.ip().to_string())
                .unwrap_or_else(|_| "127.0.0.1".to_string()),
        };

        Ok(Self {
            addr,
            upload_root,
            username: args.username,
            password,
            require_password,
            enable_https,
            cert_pem: args.cert_pem,
            key_pem: args.key_pem,
            token,
            advertised_host,
            event_bus: None,
            record_store: None,
            max_upload_size: args.max_upload_size,
        })
    }

    pub fn with_event_bus(mut self, bus: EventBus) -> Self {
        self.event_bus = Some(bus);
        self
    }

    pub fn with_record_store(mut self, store: RecordStore) -> Self {
        self.record_store = Some(store);
        self
    }

    /// URL, credentials, and QR image for display. Safe to call before `run`.
    pub fn info(&self) -> Result<HttpServerInfo, String> {
        let scheme = if self.enable_https { "https" } else { "http" };
        let url = format!(
            "{}://{}:{}/#t={}",
            scheme,
            self.advertised_host,
            self.addr.port(),
            self.token
        );
        let qr_data_url = crate::qr::render_svg_data_url(&url)?;
        Ok(HttpServerInfo {
            url,
            address: self.advertised_host.clone(),
            port: self.addr.port(),
            username: self.username.clone(),
            password: self.password.clone(),
            token: self.token.clone(),
            qr_data_url,
            https: self.enable_https,
        })
    }

    /// Binds the listener, reports readiness, then serves until `stop_rx` fires.
    ///
    /// The listener is bound before `ready` is signalled, so a reserved or busy
    /// port surfaces as an error instead of a silent failure.
    pub async fn run(
        self,
        stop_rx: oneshot::Receiver<()>,
        mut ready: Option<oneshot::Sender<Result<(), String>>>,
    ) -> Result<(), String> {
        let rustls = if self.enable_https {
            install_crypto_provider();
            match build_rustls_config(&self.cert_pem, &self.key_pem).await {
                Ok(config) => Some(config),
                Err(e) => {
                    report_ready(&mut ready, Err(e.clone()));
                    return Err(e);
                }
            }
        } else {
            None
        };

        let listener = match std::net::TcpListener::bind(self.addr) {
            Ok(listener) => listener,
            Err(e) => {
                let message = format!("Failed to bind {}: {}", self.addr, bind_hint(e));
                report_ready(&mut ready, Err(message.clone()));
                return Err(message);
            }
        };

        if let Err(e) = listener.set_nonblocking(true) {
            let message = e.to_string();
            report_ready(&mut ready, Err(message.clone()));
            return Err(message);
        }

        report_ready(&mut ready, Ok(()));

        let state = Arc::new(AppState {
            upload_root: self.upload_root.clone(),
            username: self.username.clone(),
            password: self.password.clone(),
            require_password: self.require_password,
            token: self.token.clone(),
            sessions: SessionStore::new(),
            event_bus: self.event_bus.clone(),
            record_store: self.record_store.clone(),
            secure_cookie: self.enable_https,
        });

        let app = Router::new()
            .route("/", get(index))
            .route("/api/health", get(health))
            .route("/api/session", get(session_status).post(create_session))
            .route("/api/upload", post(upload))
            .layer(match self.max_upload_size {
                Some(limit) => DefaultBodyLimit::max(limit as usize),
                None => DefaultBodyLimit::disable(),
            })
            .with_state(state);

        let handle = axum_server::Handle::new();
        let shutdown = handle.clone();
        tokio::spawn(async move {
            let _ = stop_rx.await;
            shutdown.graceful_shutdown(Some(Duration::from_secs(2)));
        });

        match rustls {
            Some(config) => axum_server::from_tcp_rustls(listener, config)
                .map_err(|e| format!("Failed to create HTTPS server: {}", e))?
                .handle(handle)
                .serve(app.into_make_service())
                .await
                .map_err(|e| format!("HTTPS server error: {}", e)),
            None => axum_server::from_tcp(listener)
                .map_err(|e| format!("Failed to create HTTP server: {}", e))?
                .handle(handle)
                .serve(app.into_make_service())
                .await
                .map_err(|e| format!("HTTP server error: {}", e)),
        }
    }
}

/// Sends the startup outcome to the caller, if one is waiting.
fn report_ready(
    ready: &mut Option<oneshot::Sender<Result<(), String>>>,
    result: Result<(), String>,
) {
    if let Some(tx) = ready.take() {
        let _ = tx.send(result);
    }
}

/// Installs the ring crypto provider as the process default, matching libunftp.
///
/// rustls panics when a config is built with no unambiguous default provider, so
/// pin ring explicitly instead of relying on a crate feature.
fn install_crypto_provider() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

async fn build_rustls_config(
    cert_pem: &Option<String>,
    key_pem: &Option<String>,
) -> Result<axum_server::tls_rustls::RustlsConfig, String> {
    if let (Some(cert), Some(key)) = (cert_pem, key_pem) {
        return axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key)
            .await
            .map_err(|e| format!("Failed to load TLS certificate and key: {}", e));
    }

    #[cfg(feature = "include-pem-files")]
    {
        axum_server::tls_rustls::RustlsConfig::from_pem(
            crate::EMBEDDED_CERT.to_vec(),
            crate::EMBEDDED_KEY.to_vec(),
        )
        .await
        .map_err(|e| format!("Failed to load embedded TLS certificate and key: {}", e))
    }

    #[cfg(not(feature = "include-pem-files"))]
    {
        Err(
            "HTTPS enabled but no --cert-pem/--key-pem provided (this build has no embedded certs)"
                .to_string(),
        )
    }
}

/// Adds a Windows-specific hint when a bind fails because the port sits inside
/// a reserved range (WSAEACCES / os error 10013).
fn bind_hint(err: std::io::Error) -> String {
    #[cfg(windows)]
    {
        if err.raw_os_error() == Some(10013) {
            return format!(
                "{} (on Windows, error 10013 usually means the port is in a reserved range; check `netsh interface ipv4 show excludedportrange protocol=tcp`)",
                err
            );
        }
    }
    err.to_string()
}

async fn index() -> Response {
    (
        [(header::REFERRER_POLICY, "no-referrer")],
        Html(INDEX_HTML),
    )
        .into_response()
}

async fn health(State(state): State<Arc<AppState>>) -> Response {
    Json(serde_json::json!({
        "status": "ok",
        "password_required": state.require_password,
    }))
    .into_response()
}

#[derive(Deserialize)]
struct SessionRequest {
    token: String,
    username: Option<String>,
    password: Option<String>,
}

async fn session_status(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    match session_from_headers(&headers) {
        Some(session) if state.sessions.is_valid(&session) => StatusCode::NO_CONTENT.into_response(),
        _ => StatusCode::UNAUTHORIZED.into_response(),
    }
}

async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SessionRequest>,
) -> Response {
    if !constant_time_eq(request.token.as_bytes(), state.token.as_bytes()) {
        return (StatusCode::UNAUTHORIZED, "invalid pairing token").into_response();
    }

    if state.require_password {
        let user_ok = request.username.as_deref() == Some(state.username.as_str());
        let password_ok = match (&state.password, &request.password) {
            (Some(expected), Some(given)) => {
                constant_time_eq(expected.as_bytes(), given.as_bytes())
            }
            _ => false,
        };
        if !user_ok || !password_ok {
            return (StatusCode::UNAUTHORIZED, "invalid credentials").into_response();
        }
    }

    let session = state.sessions.create();
    let cookie = build_cookie(&session, state.secure_cookie);
    (StatusCode::NO_CONTENT, [(header::SET_COOKIE, cookie)]).into_response()
}

#[derive(Serialize)]
struct UploadSummary {
    uploaded: Vec<String>,
    failed: Vec<String>,
}

async fn upload(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Response {
    let Some(session) = session_from_headers(&headers) else {
        return (StatusCode::UNAUTHORIZED, "pairing required").into_response();
    };
    if !state.sessions.is_valid(&session) {
        return (StatusCode::UNAUTHORIZED, "pairing required").into_response();
    }

    let mut uploaded = Vec::new();
    let mut failed = Vec::new();

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(e) => {
                return (StatusCode::BAD_REQUEST, format!("malformed upload: {}", e))
                    .into_response();
            }
        };

        let Some(raw_name) = field.file_name().map(|s| s.to_string()) else {
            continue;
        };

        let name = match sanitize_filename(&raw_name) {
            Some(name) => name,
            None => {
                failed.push(raw_name);
                continue;
            }
        };

        let dest = state.upload_root.join(&name);
        match write_field(field, &dest).await {
            Ok(()) => {
                let abs = dest.to_string_lossy().into_owned();
                if let Some(bus) = state.event_bus.as_ref() {
                    bus.publish(&FtpEvent::FileUploaded {
                        username: state.username.clone(),
                        path: name.clone(),
                        timestamp: std::time::SystemTime::now(),
                    });
                }
                if let Some(store) = state.record_store.as_ref() {
                    if let Err(e) = store.insert(&abs, &name, &name) {
                        eprintln!("[Record] failed to record upload {}: {}", name, e);
                    }
                }
                uploaded.push(name);
            }
            Err(e) => {
                eprintln!("Upload failed for {}: {}", name, e);
                failed.push(name);
            }
        }
    }

    Json(UploadSummary { uploaded, failed }).into_response()
}

async fn write_field(mut field: Field<'_>, dest: &std::path::Path) -> Result<(), String> {
    use tokio::io::AsyncWriteExt;

    let mut file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| e.to_string())?;

    while let Some(chunk) = field.chunk().await.map_err(|e| e.to_string())? {
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
    }

    file.flush().await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Reduces a client-supplied filename to a safe base name, rejecting anything
/// that could escape the upload root.
fn sanitize_filename(raw: &str) -> Option<String> {
    let base = raw.rsplit(['/', '\\']).next().unwrap_or(raw).trim();

    if base.is_empty() || base == "." || base == ".." {
        return None;
    }
    if base
        .chars()
        .any(|c| c.is_control() || c == '/' || c == '\\')
    {
        return None;
    }

    Some(base.to_string())
}

fn build_cookie(session: &str, secure: bool) -> String {
    let mut cookie = format!(
        "{}={}; HttpOnly; SameSite=Strict; Path=/",
        SESSION_COOKIE, session
    );
    if secure {
        cookie.push_str("; Secure");
    }
    cookie
}

fn session_from_headers(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    let prefix = format!("{}=", SESSION_COOKIE);
    for part in raw.split(';') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix(&prefix) {
            return Some(value.to_string());
        }
    }
    None
}
