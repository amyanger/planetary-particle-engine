use ppe_core::Dtc;
use tracing::info;

use crate::FreezeFrame;

/// Central DTC aggregation and management.
pub struct DtcManager {
    active_dtcs: Vec<Dtc>,
    history: Vec<Dtc>,
    freeze_frames: Vec<FreezeFrame>,
}

impl DtcManager {
    pub fn new() -> Self {
        Self {
            active_dtcs: Vec::new(),
            history: Vec::new(),
            freeze_frames: Vec::new(),
        }
    }

    /// Update active DTCs from all subsystems.
    pub fn update(&mut self, dtcs: Vec<Dtc>) {
        // Add new DTCs to history
        for dtc in &dtcs {
            if !self.active_dtcs.iter().any(|d| d.code == dtc.code) {
                info!(code = %dtc.code, severity = %dtc.severity, "new DTC set");
                self.history.push(dtc.clone());
            }
        }
        self.active_dtcs = dtcs;
    }

    /// Add a freeze frame snapshot.
    pub fn add_freeze_frame(&mut self, frame: FreezeFrame) {
        self.freeze_frames.push(frame);
    }

    /// Get all active DTCs.
    pub fn active(&self) -> &[Dtc] {
        &self.active_dtcs
    }

    /// Get DTC history.
    pub fn history(&self) -> &[Dtc] {
        &self.history
    }

    /// Get freeze frames.
    pub fn freeze_frames(&self) -> &[FreezeFrame] {
        &self.freeze_frames
    }

    /// Clear a specific DTC by code.
    pub fn clear(&mut self, code: &str) {
        self.active_dtcs.retain(|d| d.code != code);
        info!(code, "DTC cleared");
    }

    /// Clear all DTCs.
    pub fn clear_all(&mut self) {
        self.active_dtcs.clear();
        self.freeze_frames.clear();
        info!("all DTCs cleared");
    }

    /// Total number of active DTCs.
    pub fn count(&self) -> usize {
        self.active_dtcs.len()
    }

    /// Encode active DTCs as OBD-II Mode 03 response bytes.
    /// Each DTC is encoded as 2 bytes.
    pub fn encode_dtcs_for_obd(&self) -> Vec<u8> {
        let mut bytes = vec![self.active_dtcs.len() as u8];
        for dtc in &self.active_dtcs {
            let encoded = encode_dtc_code(&dtc.code);
            bytes.push((encoded >> 8) as u8);
            bytes.push((encoded & 0xFF) as u8);
        }
        bytes
    }
}

impl Default for DtcManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Encode a DTC string (e.g., "P0A80") into 2 bytes per OBD-II standard.
fn encode_dtc_code(code: &str) -> u16 {
    if code.len() < 5 {
        return 0;
    }

    let chars: Vec<char> = code.chars().collect();

    let prefix = match chars[0] {
        'P' => 0b00,
        'C' => 0b01,
        'B' => 0b10,
        'U' => 0b11,
        _ => 0b00,
    };

    let d1 = chars[1].to_digit(16).unwrap_or(0) as u16;
    let d2 = chars[2].to_digit(16).unwrap_or(0) as u16;
    let d3 = chars[3].to_digit(16).unwrap_or(0) as u16;
    let d4 = chars[4].to_digit(16).unwrap_or(0) as u16;

    (prefix << 14) | (d1 << 12) | (d2 << 8) | (d3 << 4) | d4
}

#[cfg(test)]
mod tests {
    use super::*;
    use ppe_core::{ComponentId, DtcSeverity};

    #[test]
    fn dtc_manager_lifecycle() {
        let mut mgr = DtcManager::new();
        assert_eq!(mgr.count(), 0);

        let dtc = Dtc::new(
            "P0A80",
            "Over temp",
            DtcSeverity::Critical,
            ComponentId::Bms,
        );
        mgr.update(vec![dtc.clone()]);
        assert_eq!(mgr.count(), 1);
        assert_eq!(mgr.history().len(), 1);

        mgr.clear("P0A80");
        assert_eq!(mgr.count(), 0);
        // History preserved
        assert_eq!(mgr.history().len(), 1);
    }

    #[test]
    fn dtc_encoding() {
        assert_eq!(encode_dtc_code("P0A80"), 0x0A80);
        assert_eq!(encode_dtc_code("P0217"), 0x0217);
    }

    #[test]
    fn obd_mode_03_encoding() {
        let mut mgr = DtcManager::new();
        mgr.update(vec![
            Dtc::new("P0A80", "test", DtcSeverity::Critical, ComponentId::Bms),
            Dtc::new("P0217", "test2", DtcSeverity::Warning, ComponentId::Thermal),
        ]);

        let bytes = mgr.encode_dtcs_for_obd();
        assert_eq!(bytes[0], 2); // count
        assert_eq!(bytes.len(), 5); // 1 + 2*2
    }
}
