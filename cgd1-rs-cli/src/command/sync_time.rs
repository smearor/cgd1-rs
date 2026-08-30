use cgd1_rs::Backend;
use cgd1_rs::MacAddress;
use cgd1_rs::TokenStore;
use tracing::info;

use crate::connection::DeviceConnection;
use crate::error::CliError;

/// Arguments for the `sync-time` command.
pub struct SyncTimeArgs {
    /// Device MAC address.
    pub address: MacAddress,
    /// BLE backend to use.
    pub backend: Backend,
}

/// Run the `sync-time` command.
pub async fn run(args: SyncTimeArgs) -> Result<(), CliError> {
    let (connection, store) = DeviceConnection::connect_with_store(&args.address, args.backend).await?;

    let device = connection.device();
    device.sync_time_now().await?;

    let token_result = store.load_or_generate(&args.address);
    if token_result.is_new() {
        store.save(&args.address, &token_result)?;
        info!(address = %args.address, "new token persisted after successful sync_time");
    }

    println!("Time synchronized for {}.", args.address);
    Ok(())
}
