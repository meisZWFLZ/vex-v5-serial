use crate::decode::{Decode, DecodeError};
use crate::encode::{Encode, EncodeError};

/// Variable-width u16 type.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct VarU16(u16);
impl VarU16 {
    /// Creates a new variable length u16.
    /// # Panics
    /// Panics if the value is too large to be encoded as a variable length u16.
    pub fn new(val: u16) -> Self {
        if val > (u16::MAX >> 1) {
            panic!("Value too large for variable length u16");
        }
        Self(val)
    }
    pub fn into_inner(self) -> u16 {
        self.0
    }
    /// Check if the variable length u16 will be wide from the first byte.
    pub fn check_wide(first: u8) -> bool {
        first > (u8::MAX >> 1) as _
    }
}
impl Encode for VarU16 {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        if self.0 > (u16::MAX >> 1) {
            return Err(EncodeError::VarShortTooLarge);
        }

        if self.0 > (u8::MAX >> 1) as _ {
            let first = (self.0 >> 8) as u8 | 0x80;
            let last = (self.0 & u8::MAX as u16) as u8;
            Ok([first, last].to_vec())
        } else {
            let val = self.0 as u8;
            Ok(vec![val])
        }
    }
}
impl Decode for VarU16 {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let first = u8::decode(&mut data)?;
        let wide = first & (1 << 7) != 0;

        if wide {
            let last = u8::decode(&mut data)?;
            let both = [first & u8::MAX >> 1, last];
            Ok(Self(u16::from_be_bytes(both)))
        } else {
            Ok(Self(first as u16))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{decode::Decode, encode::Encode, varint::VarU16};

    #[test]
    fn wide() {
        // A value that will be encoded as a wide variable length u16.
        const VAL: u16 = 0xF00;
        const ENCODED: [u8; 2] = [0x8f, 0x00];

        let var = super::VarU16::new(VAL);
        assert_eq!(ENCODED.to_vec(), var.encode().unwrap());
        assert_eq!(VAL, VarU16::decode(ENCODED).unwrap().into_inner())
    }

    #[test]
    fn thin() {
        // A value that will be encoded as a thin variable length u16.
        const VAL: u16 = 0x0F;
        const ENCODED: [u8; 1] = [0x0F];

        let var = super::VarU16::new(VAL);
        assert_eq!(ENCODED.to_vec(), var.encode().unwrap());
        assert_eq!(VAL, VarU16::decode(ENCODED).unwrap().into_inner())
    }
}
use std::time::SystemTime;

/// The epoch of the serial protocols timestamps
pub const J2000_EPOCH: u32 = 946684800;

pub(crate) fn j2000_timestamp() -> i32 {
    (SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis()
        - J2000_EPOCH as u128) as i32
}
