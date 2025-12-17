use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use distx_api::{GrpcApi, RestApi};
use distx_storage::StorageManager;

/// A simple, fast, in-memory vector database
#[derive(Parser, Debug)]
#[command(name = "distx")]
#[command(about = "A simple, fast vector database", long_about = None)]
struct Args {
    /// Path to the data directory
    #[arg(short, long, default_value = "./data")]
    data_dir: PathBuf,

    /// HTTP API port
    #[arg(long, default_value_t = 6333)]
    http_port: u16,

    /// gRPC API port
    #[arg(long, default_value_t = 6334)]
    grpc_port: u16,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let log_level = match args.log_level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting DistX v{}", env!("CARGO_PKG_VERSION"));
    info!("Data directory: {:?}", args.data_dir);
    info!("HTTP API port: {}", args.http_port);
    info!("gRPC API port: {}", args.grpc_port);

    let storage = Arc::new(StorageManager::new(&args.data_dir)?);
    info!("Storage initialized");

    let storage_http = storage.clone();
    let http_port = args.http_port;
    let http_handle = std::thread::spawn(move || {
        info!("Starting HTTP server on port {}", http_port);
        let sys = actix_web::rt::System::new();
        sys.block_on(async {
            if let Err(e) = RestApi::start(storage_http, http_port).await {
                eprintln!("HTTP server error: {}", e);
            }
        })
    });

    let storage_grpc = storage.clone();
    let grpc_port = args.grpc_port;
    let grpc_handle = tokio::spawn(async move {
        info!("Starting gRPC server on port {}", grpc_port);
        if let Err(e) = GrpcApi::start(storage_grpc, grpc_port).await {
            eprintln!("gRPC server error: {}", e);
        } else {
            info!("gRPC server stopped");
        }
    });

    info!("DistX started successfully");
    info!("HTTP API: http://localhost:{}/", args.http_port);
    info!("gRPC API: localhost:{}", args.grpc_port);

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown signal received");
        }
        _ = tokio::task::spawn_blocking(move || {
            http_handle.join().ok();
        }) => {
            info!("HTTP server stopped");
        }
        _ = grpc_handle => {
            info!("gRPC server stopped");
        }
    }

    info!("Shutting down...");
    Ok(())
}

