use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{
    crc::VEX_CRC32,
    devices::{device::Device, DeviceError},
    packets::file::{
        ExitFileTransferPacket, ExitFileTransferReplyPacket, FileDownloadTarget, FileExitAtion,
        FileInitAction, FileInitOption, FileVendor, InitFileTransferPacket,
        InitFileTransferPayload, InitFileTransferReplyPacket, LinkFilePacket, LinkFilePayload,
        LinkFileReplyPacket, ReadFilePacket, ReadFilePayload, ReadFileReplyPacket, WriteFilePacket,
        WriteFilePayload, WriteFileReplyPacket,
    },
    string::FixedLengthString,
    timestamp::j2000_timestamp,
    version::Version,
};

use super::Command;

pub const COLD_START: u32 = 0x3800000;
const USER_PROGRAM_CHUNK_SIZE: u16 = 4096;

pub struct DownloadFile {
    pub filename: FixedLengthString<23>,
    pub filetype: FixedLengthString<3>,
    pub size: u32,
    pub vendor: FileVendor,
    pub target: Option<FileDownloadTarget>,
    pub load_addr: u32,

    pub progress_callback: Option<Box<dyn FnMut(f32) + Send>>,
}
impl Command for DownloadFile {
    type Output = Vec<u8>;

    async fn execute(
        &mut self,
        device: &mut crate::devices::device::Device,
    ) -> Result<Self::Output, DeviceError> {
        let target = self.target.unwrap_or(FileDownloadTarget::Qspi);

        device
            .send_packet(InitFileTransferPacket::new(InitFileTransferPayload {
                operation: FileInitAction::Read,
                target,
                vendor: self.vendor,
                options: FileInitOption::None,
                write_file_size: self.size,
                load_address: self.load_addr,
                write_file_crc: 0,
                file_extension: self.filetype.clone(),
                timestamp: j2000_timestamp(),
                version: Version {
                    major: 1,
                    minor: 0,
                    build: 0,
                    beta: 0,
                },
                file_name: self.filename.clone(),
            }))
            .await?;
        let transfer_response = device
            .recieve_packet::<InitFileTransferReplyPacket>(Duration::from_millis(100))
            .await?;
        let transfer_response = transfer_response.payload.try_into_inner()?;

        let max_chunk_size = if transfer_response.window_size > 0
            && transfer_response.window_size <= USER_PROGRAM_CHUNK_SIZE
        {
            transfer_response.window_size
        } else {
            USER_PROGRAM_CHUNK_SIZE
        };

        let mut data = Vec::with_capacity(transfer_response.file_size as usize);
        let mut offset = 0;
        loop {
            device
                .send_packet(ReadFilePacket::new(ReadFilePayload {
                    address: self.load_addr + offset,
                    size: max_chunk_size,
                }))
                .await?;
            let read = device
                .recieve_packet::<ReadFileReplyPacket>(Duration::from_millis(100))
                .await?;
            let read = read.payload.unwrap().map_err(DeviceError::Nack)?;
            let chunk_data = read.1.into_inner();
            offset += chunk_data.len() as u32;
            let last = transfer_response.file_size <= offset;
            let progress = (offset as f32 / transfer_response.file_size as f32) * 100.0;
            data.extend(chunk_data);
            if let Some(callback) = &mut self.progress_callback {
                callback(progress);
            }
            if last {
                break;
            }
        }

        Ok(data)
    }
}

pub struct LinkedFile {
    pub filename: FixedLengthString<23>,
    pub vendor: Option<FileVendor>,
}

pub struct UploadFile {
    pub filename: FixedLengthString<23>,
    pub filetype: FixedLengthString<3>,
    pub vendor: Option<FileVendor>,
    pub data: Vec<u8>,
    pub target: Option<FileDownloadTarget>,
    pub load_addr: u32,
    pub linked_file: Option<LinkedFile>,
    pub after_upload: FileExitAtion,

    pub progress_callback: Option<Box<dyn FnMut(f32) + Send>>,
}
impl Command for UploadFile {
    type Output = ();
    async fn execute(
        &mut self,
        device: &mut crate::devices::device::Device,
    ) -> Result<Self::Output, DeviceError> {
        let vendor = self.vendor.unwrap_or(FileVendor::User);
        let target = self.target.unwrap_or(FileDownloadTarget::Qspi);

        let crc = VEX_CRC32.checksum(&self.data);

        device
            .send_packet(InitFileTransferPacket::new(InitFileTransferPayload {
                operation: FileInitAction::Write,
                target,
                vendor,
                options: FileInitOption::Overwrite,
                write_file_size: self.data.len() as u32,
                load_address: self.load_addr,
                write_file_crc: crc,
                file_extension: self.filetype.clone(),
                timestamp: j2000_timestamp(),
                version: Version {
                    major: 1,
                    minor: 0,
                    build: 0,
                    beta: 0,
                },
                file_name: self.filename.clone(),
            }))
            .await?;
        let transfer_response = device
            .recieve_packet::<InitFileTransferReplyPacket>(Duration::from_millis(100))
            .await?;
        println!("transfer init responded");
        let transfer_response = transfer_response.payload.try_into_inner()?;

        if let Some(linked_file) = &self.linked_file {
            device
                .send_packet(LinkFilePacket::new(LinkFilePayload {
                    vendor: linked_file.vendor.unwrap_or(FileVendor::User),
                    option: 0,
                    required_file: linked_file.filename.clone(),
                }))
                .await?;
            device
                .recieve_packet::<LinkFileReplyPacket>(Duration::from_millis(100))
                .await?;
        }

        let max_chunk_size = if transfer_response.window_size > 0
            && transfer_response.window_size <= USER_PROGRAM_CHUNK_SIZE
        {
            // Align to 4 bytes
            if transfer_response.window_size % 4 != 0 {
                transfer_response.window_size + (4 - transfer_response.window_size % 4)
            } else {
                transfer_response.window_size
            }
        } else {
            USER_PROGRAM_CHUNK_SIZE
        };
        println!(
            "max_chunk_size: {} from {}",
            max_chunk_size, transfer_response.window_size
        );

        let mut offset = 0;
        for chunk in self.data.chunks(max_chunk_size as _) {
            let progress = (offset as f32 / self.data.len() as f32) * 100.0;
            if let Some(callback) = &mut self.progress_callback {
                callback(progress);
            }
            device
                .send_packet(WriteFilePacket::new(WriteFilePayload {
                    address: (self.load_addr + offset) as _,
                    chunk_data: chunk.to_vec(),
                }))
                .await?;
            device
                .recieve_packet::<WriteFileReplyPacket>(Duration::from_millis(100))
                .await?;
            offset += chunk.len() as u32;
        }
        if let Some(callback) = &mut self.progress_callback {
            callback(100.0);
        }

        device
            .send_packet(ExitFileTransferPacket::new(self.after_upload))
            .await?;
        device
            .recieve_packet::<ExitFileTransferReplyPacket>(Duration::from_millis(200))
            .await?;

        Ok(())
    }
}

pub enum ProgramData {
    Hot(Vec<u8>),
    Cold(Vec<u8>),
    Both { hot: Vec<u8>, cold: Vec<u8> },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Program {
    pub name: String,
    pub slot: u8,
    pub icon: String,
    pub iconalt: String,
    pub description: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Project {
    // version: String,
    pub ide: String,
    // file: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProgramIniConfig {
    pub project: Project,
    pub program: Program,
}

pub struct UploadProgram {
    pub name: String,
    pub description: String,
    pub icon: String,
    pub program_type: String,
    /// 0-indexed slot
    pub slot: u8,
    pub data: ProgramData,
    pub after_upload: FileExitAtion,
}
impl Command for UploadProgram {
    type Output = ();

    async fn execute(&mut self, device: &mut Device) -> Result<Self::Output, DeviceError> {
        let base_file_name = format!("slot{}", self.slot);

        let ini = ProgramIniConfig {
            program: Program {
                description: self.description.clone(),
                icon: self.icon.clone(),
                iconalt: String::new(),
                slot: self.slot,
                name: self.name.clone(),
            },
            project: Project {
                ide: self.program_type.clone(),
            },
        };
        let ini = serde_ini::to_vec(&ini).unwrap();

        let file_transfer = UploadFile {
            filename: FixedLengthString::new(format!("{}.ini", base_file_name))?,
            filetype: FixedLengthString::new("ini".to_string())?,
            vendor: None,
            data: ini,
            target: None,
            load_addr: COLD_START,
            linked_file: None,
            after_upload: FileExitAtion::Halt,
            progress_callback: Some(Box::new(|progress| {
                println!("Uploading INI: {:.2}%", progress)
            })),
        };
        device.execute_command(file_transfer).await.unwrap();

        let (cold, hot) = match &self.data {
            ProgramData::Cold(cold) => (Some(cold), None),
            ProgramData::Hot(hot) => (None, Some(hot)),
            ProgramData::Both { hot, cold } => (Some(cold), Some(hot)),
        };

        if let Some(cold) = cold {
            let after_upload = if hot.is_some() {
                FileExitAtion::Halt
            } else {
                self.after_upload
            };

            device
                .execute_command(UploadFile {
                    filename: FixedLengthString::new(format!("{}.bin", base_file_name))?,
                    filetype: FixedLengthString::new("bin".to_string())?,
                    vendor: None,
                    data: cold.clone(),
                    target: None,
                    load_addr: COLD_START,
                    linked_file: None,
                    after_upload,
                    progress_callback: Some(Box::new(|progress| {
                        println!("Uploading cold: {:.2}%", progress)
                    })),
                })
                .await?;
        }

        if let Some(hot) = hot {
            let linked_file = Some(LinkedFile {
                filename: FixedLengthString::new(format!("{}_lib.bin", base_file_name))?,
                vendor: None,
            });
            device
                .execute_command(UploadFile {
                    filename: FixedLengthString::new(format!("{}.bin", base_file_name))?,
                    filetype: FixedLengthString::new("bin".to_string())?,
                    vendor: None,
                    data: hot.clone(),
                    target: None,
                    load_addr: 0x07800000,
                    linked_file,
                    after_upload: self.after_upload,
                    progress_callback: Some(Box::new(|progress| {
                        println!("Uploading hot: {:.2}%", progress)
                    })),
                })
                .await?;
        }

        Ok(())
    }
}
