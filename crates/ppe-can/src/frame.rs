use heapless::Vec;
use serde::{Deserialize, Serialize};
use std::fmt;

/// 11-bit CAN identifier. Lower value = higher priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CanId(u16);

impl CanId {
    /// Create a CAN ID from a raw 11-bit value.
    /// Returns None if the value exceeds 11 bits (0x7FF).
    pub fn new(raw: u16) -> Option<Self> {
        if raw <= 0x7FF {
            Some(Self(raw))
        } else {
            None
        }
    }

    /// Create a CAN ID without bounds checking.
    ///
    /// # Safety
    /// Caller must ensure raw <= 0x7FF.
    pub const fn new_unchecked(raw: u16) -> Self {
        Self(raw)
    }

    pub fn raw(self) -> u16 {
        self.0
    }
}

impl fmt::Display for CanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:03X}", self.0)
    }
}

/// A CAN bus frame with up to 8 bytes of data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanFrame {
    pub id: CanId,
    pub data: Vec<u8, 8>,
    pub timestamp_us: u64,
}

impl CanFrame {
    pub fn new(id: CanId, data: &[u8], timestamp_us: u64) -> Self {
        let mut frame_data = Vec::new();
        for &byte in data.iter().take(8) {
            let _ = frame_data.push(byte);
        }
        Self {
            id,
            data: frame_data,
            timestamp_us,
        }
    }

    pub fn dlc(&self) -> usize {
        self.data.len()
    }
}

impl fmt::Display for CanFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] ", self.id)?;
        for (i, byte) in self.data.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{byte:02X}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_id_valid() {
        assert!(CanId::new(0x100).is_some());
        assert!(CanId::new(0x7FF).is_some());
        assert!(CanId::new(0x800).is_none());
    }

    #[test]
    fn can_id_priority_ordering() {
        let high = CanId::new(0x010).unwrap();
        let low = CanId::new(0x700).unwrap();
        assert!(high < low); // Lower ID = higher priority
    }

    #[test]
    fn can_frame_display() {
        let frame = CanFrame::new(CanId::new(0x100).unwrap(), &[0xDE, 0xAD, 0xBE, 0xEF], 0);
        assert_eq!(format!("{frame}"), "[0x100] DE AD BE EF");
    }

    #[test]
    fn can_frame_truncates_to_8_bytes() {
        let frame = CanFrame::new(
            CanId::new(0x100).unwrap(),
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            0,
        );
        assert_eq!(frame.dlc(), 8);
    }
}
