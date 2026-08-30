use std::path::PathBuf;

use cgd1_rs::Backend;
use cgd1_rs::MacAddress;
use cgd1_rs::RingtoneSignature;

use crate::connection::DeviceConnection;
use crate::error::CliError;

/// Arguments for the `ringtone-upload` command.
pub struct RingtoneUploadArgs {
    /// Device MAC address.
    pub address: MacAddress,
    /// Path to 8-bit PCM audio file (8 kHz, mono).
    pub file: PathBuf,
    /// 4-byte signature as hex string (e.g., "deadbeef").
    pub signature: RingtoneSignature,
    /// BLE backend to use.
    pub backend: Backend,
}

/// Run the `ringtone-upload` command.
pub async fn run(args: RingtoneUploadArgs) -> Result<(), CliError> {
    let audio = std::fs::read(&args.file).map_err(|e| CliError::AudioReadFailed {
        path: args.file.clone(),
        reason: e.to_string(),
    })?;

    let connection = DeviceConnection::connect(&args.address, args.backend).await?;
    connection.device().upload_ringtone(&audio, args.signature.bytes()).await?;

    println!("Ringtone uploaded (signature: {}).", args.signature);
    Ok(())
}
