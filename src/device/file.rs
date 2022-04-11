use serde::{Serialize, Deserialize};
use chrono::TimeZone;


/// The filesystem target when reading from the brain
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum VexFileTarget {
    DDR = 0,
    FLASH = 1,
    SCREEN = 2,
}

/// The mode to open a file on the V5 device with
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum VexFileMode {
    /// Open the file for uploading to the brain
    Upload(VexFileTarget, bool),
    /// Open the file for downloading fromt he brain
    Download(VexFileTarget, bool),
}

/// Represents vex file metadata when initiating
/// a transfer
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct VexInitialFileMetadata {
    pub function: VexFileMode,
    pub vid: super::VexVID,
    pub options: u8,
    pub length: u32,
    pub addr: u32,
    pub crc: u32,
    pub r#type: [u8; 4],
    pub timestamp: u32,
    pub version: u32,
}

impl Default for VexInitialFileMetadata {
    fn default() -> Self {
        VexInitialFileMetadata {
            function: VexFileMode::Upload(VexFileTarget::FLASH, true),
            vid: super::VexVID::USER,
            options: 0,
            length: 0,
            addr: 0x3800000,
            crc: 0,
            r#type: *b"bin\0",
            // Default timestamp to number of seconds after Jan 1 2000
            timestamp: (chrono::Utc::now().timestamp() - chrono::Utc.ymd(2000, 1, 1)
                            .and_hms(0, 0, 0).timestamp()).try_into().unwrap(),
            version: 0,
        }
    }
}


/// Represents a flag that tells the brain what to do
/// after a file transfer is complete
pub enum VexFiletransferFinished {
    DoNothing = 0b0,
    RunProgram = 0b1,
    ShowRunScreen = 0b11,
}

impl Default for VexFiletransferFinished {
    fn default() -> Self {
        VexFiletransferFinished::DoNothing
    }
}

/// Metadata for a file transfer
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VexFiletransferMetadata {
    pub max_packet_size: u16,
    pub file_size: u32,
    pub crc: u32,
}

