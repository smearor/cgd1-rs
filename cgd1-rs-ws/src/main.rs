use cgd1_rs::Backend;
use cgd1_rs_ws::ServerState;
use clap::Parser;

/// Command-line arguments for the WebSocket server.
#[derive(Parser, Debug)]
#[command(name = "cgd1-ws", about = "WebSocket and REST server for the Qingping CGD1 alarm clock")]
struct Cli {
    /// Bind address.
    #[arg(long, default_value = "0.0.0.0")]
    address: String,

    /// Bind port.
    #[arg(long, default_value_t = 3000)]
    port: u16,

    /// Verbosity level (0=warn, 1=info, 2=debug, 3=trace).
    #[arg(short, long, default_value_t = 0)]
    verbose: u8,

    /// BLE backend: `btleplug` (real hardware) or `virtual` (in-memory device for testing).
    #[arg(long, default_value_t)]
    backend: Backend,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let filter = match cli.verbose {
        0 => tracing_subscriber::EnvFilter::new("warn"),
        1 => tracing_subscriber::EnvFilter::new("info"),
        2 => tracing_subscriber::EnvFilter::new("debug"),
        _ => tracing_subscriber::EnvFilter::new("trace"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let state = ServerState::new(cli.backend).await?;
    let router = cgd1_rs_ws::build_router(state);

    let bind_addr = format!("{}:{}", cli.address, cli.port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!(addr = %bind_addr, "WebSocket server listening");

    axum::serve(listener, router).await?;

    Ok(())
}
