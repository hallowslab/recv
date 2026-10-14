use clap::Parser;
use recv::HttpServer;
use recv::config::Args;
use tokio::sync::oneshot;

#[tokio::main]
pub async fn main() {
    let args = Args::parse();
    let record_store_path = args.record_store.clone();
    let show_qr = !args.no_qr;

    let mut server = match HttpServer::new(args) {
        Ok(server) => server,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    if let Some(path) = record_store_path {
        let store_path = std::path::PathBuf::from(&path);
        match exifflow_core::RecordStore::open(&store_path) {
            Ok(store) => {
                println!("[Record] Recording uploads to {}", store_path.display());
                server = server.with_record_store(store);
            }
            Err(e) => {
                eprintln!("Error: failed to open record store: {}", e);
                std::process::exit(1);
            }
        }
    }

    let info = match server.info() {
        Ok(info) => info,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let password = info
        .password
        .clone()
        .unwrap_or_else(|| "(none)".to_string());

    println!("Server Init");
    println!(
        "Config:\n\tURL: {}\n\tAddress: {}\n\tPort: {}\n\tUsername: {}\n\tPassword: {}\n",
        info.url, info.address, info.port, info.username, password
    );

    if show_qr {
        match recv::qr::render_terminal(&info.url) {
            Ok(qr) => println!("Scan to upload:\n{}\n", qr),
            Err(e) => eprintln!("Failed to render QR code: {}", e),
        }
    }

    let (_stop_tx, stop_rx) = oneshot::channel();

    if let Err(e) = server.run(stop_rx, None).await {
        eprintln!("Error running server: {}", e);
        std::process::exit(1);
    }
}
