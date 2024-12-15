use std::{str::FromStr, time::Duration};

use log::info;
use vex_v5_serial::{
    connection::{
        serial::{self, SerialError},
        Connection,
    },
    encode::Encode,
    packets::{
        device::{GetDeviceStatusPacket, GetDeviceStatusReplyPacket},
        file::{
            ExtensionType, FileInitAction, FileInitOption, FileMetadata, FileTransferTarget,
            FileVendor, InitFileTransferPacket, InitFileTransferPayload,
        },
    },
    string::FixedString,
    version::Version,
};

#[tokio::main]
async fn main() -> Result<(), SerialError> {
    simplelog::TermLogger::init(
        log::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Always,
    )
    .unwrap();

    let devices = serial::find_devices()?;

    // Open a connection to the device
    let mut connection = devices[0].connect(Duration::from_secs(30))?;

    let status = connection
        .packet_handshake::<GetDeviceStatusReplyPacket>(
            Duration::from_millis(500),
            10,
            GetDeviceStatusPacket::new(()),
        )
        .await?
        .try_into_inner()?;

    for device in status.devices {
        info!("{:?} on port: {}", device.device_type, device.port);
    }

    Ok(())
}
