// Module that contains all commands that can be sent to the v5

mod kv;
pub use kv::{KVRead, KVWrite};

mod extended;
pub use extended::{Extended, ExtendedResponse};

mod system;
pub use system::{GetSystemVersion, V5SystemVersion};

mod file;
pub use file::{
    FileTransferInit,
    FileTransferInitResponse,
    FileTransferExit,
    FileTransferSetLink,
    FileTransferWrite,
    FileTransferRead
};

/// A command trait that every command implements
pub trait Command {
    type Response;
    /// Encodes the client (computer) -> host (firmware) request
    /// 
    /// Implementation is specific to each command, but generally it returnes the data in the command's structure
    /// parsed into a (simple_command: u8, data: Vec<u8>)
    fn encode_request(self) -> Result<(u8, Vec<u8>), crate::errors::DecodeError>;

    /// Decodes a host (firmware) -> client (computer) response
    /// 
    /// # Arguments
    /// 
    /// * `command_id` - The command ID of the recieved command
    /// * `data` - The vector of data that was sent in the command
    fn decode_response(command_id: u8, data: Vec<u8>) -> Result<Self::Response, crate::errors::DecodeError>;
}