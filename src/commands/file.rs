use crate::{v5::meta::{
    FileTransferFunction,
    FileTransferTarget,
    FileTransferVID,
    FileTransferOptions,
    FileTransferType, FileTransferComplete
}, checks::VexExtPacketChecks};

use super::Command;


/// Initializes a file transfer between the brain and host
#[derive(Copy, Clone)]
pub struct FileTransferInit {
    pub function: FileTransferFunction,
    pub target: FileTransferTarget,
    pub vid: FileTransferVID,
    pub options: FileTransferOptions,
    pub file_type: FileTransferType,
    pub length: u32,
    pub addr: u32,
    pub crc: u32,
    pub timestamp: u32,
    pub version: u32,
    pub name: [u8; 24]
}

impl Command for FileTransferInit {
    type Response = FileTransferInitResponse;

    fn encode_request(self) -> Result<(u8, Vec<u8>), crate::errors::DecodeError> {
        
        // Create the empty payload
        let mut payload = Vec::<u8>::new();

        // Load the function, target, vid, and options
        payload.extend([
            self.function as u8,
            self.target as u8,
            self.vid as u8,
            self.options.bits(),
        ]);

        // Add the length
        payload.extend(self.length.to_le_bytes());

        // Add the addr
        payload.extend(self.addr.to_le_bytes());

        // Add the crc
        payload.extend(self.crc.to_le_bytes());

        // Add the type
        payload.extend(self.file_type.to_bytes());

        // Add the timestamp
        payload.extend(self.timestamp.to_le_bytes());

        // Add the version
        payload.extend(self.version.to_le_bytes());

        // Add the file name to the payload
        payload.extend(self.name);

        // Encode an extended command with id 0x11
        super::Extended(0x11, &payload).encode_request()
    }

    fn decode_response(command_id: u8, data: Vec<u8>) -> Result<Self::Response, crate::errors::DecodeError> {
        // Decode the extended command
        let payload = super::Extended::decode_response(command_id, data)?;

        // Ensure that it is a response to 0x11
        if payload.0 != 0x11 {
            return Err(crate::errors::DecodeError::ExpectedCommand(0x11, payload.0));
        }

        // Get the max_packet_size (bytes 0..1)
        // We can unwrap the try_into because we know that get will return 2 bytes
        let max_packet_size = u16::from_le_bytes(payload.1.get(0..2).ok_or(crate::errors::DecodeError::PacketLengthError)?.try_into().unwrap());

        // Get the file_size (bytes 2..3)
        let file_size = u16::from_le_bytes(payload.1.get(2..4).ok_or(crate::errors::DecodeError::PacketLengthError)?.try_into().unwrap());

        // Get the crc (bytes 4..8)
        let crc = u32::from_le_bytes(payload.1.get(4..8).ok_or(crate::errors::DecodeError::PacketLengthError)?.try_into().unwrap());

        // Return the result
        Ok(FileTransferInitResponse {
            max_packet_size,
            file_size,
            crc
        })
    }
}

#[derive(Copy, Clone)]
pub struct FileTransferInitResponse {
    pub max_packet_size: u16,
    pub file_size: u16,
    pub crc: u32
}



/// Exit a file transfer between the brain and host
/// 
/// # Members
/// 
/// * `0` - The action to complete when the transfer is finished
#[derive(Copy, Clone)]
pub struct FileTransferExit(pub FileTransferComplete);

impl Command for FileTransferExit {
    type Response = ();

    fn encode_request(self) -> Result<(u8, Vec<u8>), crate::errors::DecodeError> {
        
        // Create the empty payload
        let mut payload = Vec::<u8>::new();

        // Add the file transfer complete byte
        payload.push(self.0 as u8);

        // Encode an extended command with id 0x12
        super::Extended(0x12, &payload).encode_request()
    }

    

    fn decode_response(command_id: u8, data: Vec<u8>) -> Result<Self::Response, crate::errors::DecodeError> {
        
        // Decode the extended command
        let payload = super::Extended::decode_response(command_id, data)?;

        // Ensure that it is a response to 0x12
        if payload.0 != 0x12 {
            return Err(crate::errors::DecodeError::ExpectedCommand(0x12, payload.0));
        }

        // Do nothing
        Ok(())
    }
}


/// Sets the linked file for the current transfer
/// 
/// # Members
/// 
/// * `0` - The linked file name
/// * `1` - The file VID
/// * `2` - The file options
#[derive(Copy, Clone)]
pub struct FileTransferSetLink (pub [u8; 24], pub FileTransferVID, pub FileTransferOptions);

impl Command for FileTransferSetLink {
    type Response = ();

    fn encode_request(self) -> Result<(u8, Vec<u8>), crate::errors::DecodeError> {
        
        // Create the packet
        let mut packet = Vec::<u8>::new();

        // Add the vid
        packet.push(self.1 as u8);

        // Add the options
        packet.push(self.2.bits());

        // Add the name
        packet.extend(self.0);

        super::Extended(0x15, &packet).encode_request()
    }

    fn decode_response(command_id: u8, data: Vec<u8>) -> Result<Self::Response, crate::errors::DecodeError> {
        
        // Decode the extended command
        let payload = super::Extended::decode_response(command_id, data)?;

        // Ensure that it is a response to 0x15
        if payload.0 != 0x15 {
            return Err(crate::errors::DecodeError::ExpectedCommand(0x15, payload.0));
        }
        
        Ok(())
    }
}



/// Read data from a file transfer
/// 
/// # Members
/// 
/// * `0` - The address to read data from
/// * `1` - The number of bytes to read, will be padded to 4 bytes
#[derive(Copy, Clone)]
pub struct FileTransferRead(pub u32, pub u16);

impl Command for FileTransferRead {
    type Response = Vec<u8>;

    fn encode_request(self) -> Result<(u8, Vec<u8>), crate::errors::DecodeError> {
        
        // Pad nbytes to a 4 byte barrier
        let nbytes = if self.1 % 4 == 0 {
            self.1
        } else {
            self.1 + 4 - (self.1 % 4)
        };

        // Create the payload
        let mut payload = Vec::<u8>::new();

        // Add the address
        payload.extend(self.0.to_le_bytes());

        // Add the data length
        payload.extend(nbytes.to_le_bytes());

        // Return the encoded extended packet with id 0x14
        super::Extended(0x14, &payload).encode_request()
    }

    fn decode_response(command_id: u8, data: Vec<u8>) -> Result<Self::Response, crate::errors::DecodeError> {
        
        // Read the extended command
        let payload = super::Extended::decode_extended(
            command_id, data,
            VexExtPacketChecks::LENGTH | VexExtPacketChecks::CRC 
        )?;

        // Ensure that it is a response to 0x14
        if payload.0 != 0x14 {
            return Err(crate::errors::DecodeError::ExpectedCommand(0x14, payload.0));
        }

        
        // Return the data
        Ok(payload.1)
    }
}



/// Write data to a file transfer
/// 
/// # Members
/// 
/// * `0` - The address to write at
/// * `1` - The data to write
#[derive(Copy, Clone)]
pub struct FileTransferWrite<'a>(pub u32, pub &'a[u8]);

impl<'a> Command for FileTransferWrite<'a> {
    type Response = ();

    fn encode_request(self) -> Result<(u8, Vec<u8>), crate::errors::DecodeError> {
        
        // Create the payload vec
        let mut packet = Vec::<u8>::new();

        // Pad the payload to 4 bytes
        let mut payload = self.1.to_vec();
        payload.resize(if payload.len() % 4 == 0 {
            payload.len()
        } else {
            payload.len() + 4 - (payload.len() % 4)
        },0);

        // Add the address to the packet
        packet.extend(self.0.to_le_bytes());

        // Add the payload to the packet
        packet.extend(payload);

        // Return the parsed extended packet with id 0x13
        super::Extended(0x13, &packet).encode_request()
    }

    fn decode_response(command_id: u8, data: Vec<u8>) -> Result<Self::Response, crate::errors::DecodeError> {
        // Read the extended command
        let payload = super::Extended::decode_response(command_id, data)?;

        // Ensure that it is a response to 0x13
        if payload.0 != 0x13 {
            return Err(crate::errors::DecodeError::ExpectedCommand(0x13, payload.0));
        }

        
        // Return Ok
        Ok(())
    }
}