mod app;
mod dialog;
mod display;
mod window;

use cgd1_rs::Backend;
use clap::Parser;

/// GTK 4 desktop application for the Qingping CGD1 Bluetooth alarm clock.
#[derive(Parser, Debug)]
#[command(name = "cgd1-controller", about = "GTK 4 desktop application for the Qingping CGD1 Bluetooth alarm clock")]
struct Cli {
    /// Verbosity level (0=warn, 1=info, 2=debug, 3=trace).
    #[arg(short, long, default_value_t = 0)]
    verbose: u8,

    /// BLE backend: `btleplug` (real hardware) or `virtual` (in-memory device for testing).
    #[arg(long, default_value_t)]
    backend: Backend,
}

fn main() {
    let cli = Cli::parse();

    let filter = match cli.verbose {
        0 => tracing_subscriber::EnvFilter::new("warn"),
        1 => tracing_subscriber::EnvFilter::new("info"),
        2 => tracing_subscriber::EnvFilter::new("debug"),
        _ => tracing_subscriber::EnvFilter::new("trace"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let app = app::ClockControllerApp::new(cli.backend);
    app.run();
}
