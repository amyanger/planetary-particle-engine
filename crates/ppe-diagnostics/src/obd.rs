use ppe_can::{well_known, BusNode, CanFrame};
use tracing::{debug, warn};

use crate::DtcManager;

/// OBD-II PID definitions (Mode 01).
pub mod pid {
    pub const ENGINE_RPM: u8 = 0x0C;
    pub const VEHICLE_SPEED: u8 = 0x0D;
    pub const COOLANT_TEMP: u8 = 0x05;
    pub const FUEL_LEVEL: u8 = 0x2F; // We'll use this for SOC
    pub const CONTROL_MODULE_VOLTAGE: u8 = 0x42;
    pub const _BATTERY_CURRENT: u8 = 0x49; // Hybrid battery current (reserved)
}

/// Live vehicle data that the OBD responder reads.
#[derive(Debug, Clone, Default)]
pub struct ObdLiveData {
    pub rpm: f64,
    pub speed_kmh: f64,
    pub coolant_temp_c: f64,
    pub soc_pct: f64,
    pub battery_voltage: f64,
    pub battery_current: f64,
}

/// OBD-II protocol responder. Listens on CAN ID 0x7DF and responds on 0x7E8.
pub struct ObdResponder {
    can_node: BusNode,
    live_data: ObdLiveData,
}

impl ObdResponder {
    pub fn new(can_node: BusNode) -> Self {
        Self {
            can_node,
            live_data: ObdLiveData::default(),
        }
    }

    /// Update the live data snapshot.
    pub fn update_live_data(&mut self, data: ObdLiveData) {
        self.live_data = data;
    }

    /// Process incoming OBD requests and send responses.
    pub fn process(&mut self, dtc_manager: &DtcManager) {
        for frame in self.can_node.drain() {
            if frame.id != well_known::OBD_REQUEST {
                continue;
            }

            if frame.data.len() < 2 {
                continue;
            }

            let _length = frame.data[0];
            let mode = frame.data[1];

            match mode {
                0x01 => {
                    // Mode 01: Show current data
                    if frame.data.len() >= 3 {
                        let pid_val = frame.data[2];
                        self.handle_mode_01(pid_val);
                    }
                }
                0x03 => {
                    // Mode 03: Show stored DTCs
                    self.handle_mode_03(dtc_manager);
                }
                _ => {
                    warn!(mode, "unsupported OBD mode");
                }
            }
        }
    }

    fn handle_mode_01(&self, pid_val: u8) {
        let response_data = match pid_val {
            pid::ENGINE_RPM => {
                // RPM: value = RPM * 4, encoded in 2 bytes (A, B)
                let rpm_encoded = (self.live_data.rpm * 4.0) as u16;
                vec![
                    4, // response length
                    0x41,
                    pid::ENGINE_RPM,
                    (rpm_encoded >> 8) as u8,
                    (rpm_encoded & 0xFF) as u8,
                ]
            }
            pid::VEHICLE_SPEED => {
                // Speed in km/h, single byte
                let speed = self.live_data.speed_kmh.clamp(0.0, 255.0) as u8;
                vec![3, 0x41, pid::VEHICLE_SPEED, speed]
            }
            pid::COOLANT_TEMP => {
                // Temp: value = temp + 40 (offset)
                let temp = (self.live_data.coolant_temp_c + 40.0).clamp(0.0, 255.0) as u8;
                vec![3, 0x41, pid::COOLANT_TEMP, temp]
            }
            pid::FUEL_LEVEL => {
                // SOC as fuel level: value = SOC * 2.55
                let soc = (self.live_data.soc_pct * 2.55).clamp(0.0, 255.0) as u8;
                vec![3, 0x41, pid::FUEL_LEVEL, soc]
            }
            pid::CONTROL_MODULE_VOLTAGE => {
                // Voltage in mV, 2 bytes
                let mv = (self.live_data.battery_voltage * 1000.0) as u16;
                vec![
                    4,
                    0x41,
                    pid::CONTROL_MODULE_VOLTAGE,
                    (mv >> 8) as u8,
                    (mv & 0xFF) as u8,
                ]
            }
            _ => {
                debug!(pid = pid_val, "unsupported PID");
                return;
            }
        };

        let _ = self
            .can_node
            .send(CanFrame::new(well_known::OBD_RESPONSE, &response_data, 0));
    }

    fn handle_mode_03(&self, dtc_manager: &DtcManager) {
        let dtc_bytes = dtc_manager.encode_dtcs_for_obd();
        let mut response = vec![dtc_bytes.len() as u8 + 1, 0x43];
        response.extend_from_slice(&dtc_bytes);

        // Truncate to 8 bytes max (CAN frame limit)
        response.truncate(8);

        let _ = self
            .can_node
            .send(CanFrame::new(well_known::OBD_RESPONSE, &response, 0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ppe_can::{CanFilter, VirtualCanBus};
    use std::time::Duration;

    fn setup() -> (ObdResponder, BusNode, DtcManager) {
        let bus = VirtualCanBus::new(256);
        let obd_node = bus.connect(CanFilter::Exact(well_known::OBD_REQUEST), 64);
        let client_node = bus.connect(CanFilter::Exact(well_known::OBD_RESPONSE), 64);
        let responder = ObdResponder::new(obd_node);
        let dtc_mgr = DtcManager::new();
        (responder, client_node, dtc_mgr)
    }

    #[test]
    fn obd_mode_01_vehicle_speed() {
        let (mut responder, client, dtc_mgr) = setup();

        responder.update_live_data(ObdLiveData {
            speed_kmh: 100.0,
            ..Default::default()
        });

        // Send OBD request for vehicle speed
        let request = CanFrame::new(
            well_known::OBD_REQUEST,
            &[0x02, 0x01, pid::VEHICLE_SPEED],
            0,
        );
        client.send(request).unwrap();
        std::thread::sleep(Duration::from_millis(50));

        responder.process(&dtc_mgr);
        std::thread::sleep(Duration::from_millis(50));

        let response = client.recv_timeout(Duration::from_millis(100));
        assert!(response.is_some(), "Should receive OBD response");
        let response = response.unwrap();
        assert_eq!(response.id, well_known::OBD_RESPONSE);
        // Byte 3 is the speed value
        assert_eq!(response.data[3], 100);
    }

    #[test]
    fn obd_mode_01_rpm() {
        let (mut responder, client, dtc_mgr) = setup();

        responder.update_live_data(ObdLiveData {
            rpm: 3000.0,
            ..Default::default()
        });

        let request = CanFrame::new(well_known::OBD_REQUEST, &[0x02, 0x01, pid::ENGINE_RPM], 0);
        client.send(request).unwrap();
        std::thread::sleep(Duration::from_millis(50));

        responder.process(&dtc_mgr);
        std::thread::sleep(Duration::from_millis(50));

        let response = client.recv_timeout(Duration::from_millis(100));
        assert!(response.is_some());
        let response = response.unwrap();

        // RPM encoded as (RPM * 4), split into 2 bytes
        let rpm_raw = ((response.data[3] as u16) << 8) | (response.data[4] as u16);
        let rpm = rpm_raw as f64 / 4.0;
        assert!((rpm - 3000.0).abs() < 1.0, "RPM should be 3000, got {rpm}");
    }

    #[test]
    fn obd_mode_03_dtcs() {
        let (mut responder, client, mut dtc_mgr) = setup();

        use ppe_core::{ComponentId, Dtc, DtcSeverity};
        dtc_mgr.update(vec![Dtc::new(
            "P0A80",
            "test",
            DtcSeverity::Critical,
            ComponentId::Bms,
        )]);

        let request = CanFrame::new(well_known::OBD_REQUEST, &[0x01, 0x03], 0);
        client.send(request).unwrap();
        std::thread::sleep(Duration::from_millis(50));

        responder.process(&dtc_mgr);
        std::thread::sleep(Duration::from_millis(50));

        let response = client.recv_timeout(Duration::from_millis(100));
        assert!(response.is_some());
        let response = response.unwrap();
        assert_eq!(response.data[1], 0x43); // Mode 03 response
        assert_eq!(response.data[2], 1); // 1 DTC
    }
}
