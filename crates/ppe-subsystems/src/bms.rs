use std::time::Duration;

use ppe_can::{well_known, BusNode, CanFrame};
use ppe_core::{ComponentId, Dtc, DtcSeverity, Percent};
use ppe_hal::{MockSensor, Sensor, SensorHandle};
use ppe_state::BmsState;
use tracing::{info, warn};

use crate::{Subsystem, SubsystemHealth};

/// Configuration for the Battery Management System.
#[derive(Debug, Clone)]
pub struct BmsConfig {
    pub cell_count: u32,
    pub nominal_cell_voltage: f64,
    pub min_cell_voltage: f64,
    pub max_cell_voltage: f64,
    pub max_temperature: f64,
    pub min_temperature: f64,
    pub capacity_ah: f64,
}

impl Default for BmsConfig {
    fn default() -> Self {
        Self {
            cell_count: 96,
            nominal_cell_voltage: 3.7,
            min_cell_voltage: 3.0,
            max_cell_voltage: 4.2,
            max_temperature: 45.0,
            min_temperature: -10.0,
            capacity_ah: 60.0,
        }
    }
}

/// Sensor handles for physics to write into BMS sensors.
pub struct BmsHandles {
    pub pack_voltage: SensorHandle,
    pub pack_current: SensorHandle,
    pub pack_temperature: SensorHandle,
}

/// Battery Management System subsystem.
pub struct BatteryManagementSystem {
    config: BmsConfig,
    state: BmsState,
    soc: f64, // 0.0 to 100.0
    voltage_sensor: MockSensor,
    current_sensor: MockSensor,
    temp_sensor: MockSensor,
    can_node: BusNode,
    dtcs: Vec<Dtc>,
    coulomb_count_ah: f64,
}

impl BatteryManagementSystem {
    pub fn new(config: BmsConfig, can_node: BusNode) -> (Self, BmsHandles) {
        let initial_voltage = config.nominal_cell_voltage * config.cell_count as f64;
        let (voltage_sensor, voltage_handle) =
            MockSensor::new_clean("bms_pack_voltage", initial_voltage);
        let (current_sensor, current_handle) = MockSensor::new_clean("bms_pack_current", 0.0);
        let (temp_sensor, temp_handle) = MockSensor::new_clean("bms_pack_temperature", 25.0);

        let handles = BmsHandles {
            pack_voltage: voltage_handle,
            pack_current: current_handle,
            pack_temperature: temp_handle,
        };

        let bms = Self {
            config,
            state: BmsState::Standby,
            soc: 80.0,
            voltage_sensor,
            current_sensor,
            temp_sensor,
            can_node,
            dtcs: Vec::new(),
            coulomb_count_ah: 0.0,
        };

        (bms, handles)
    }

    pub fn state(&self) -> BmsState {
        self.state
    }

    pub fn soc(&self) -> Percent {
        Percent::new(self.soc)
    }

    fn check_faults(&mut self, voltage: f64, current: f64, temperature: f64) {
        self.dtcs.clear();

        let cell_voltage = voltage / self.config.cell_count as f64;

        if cell_voltage > self.config.max_cell_voltage {
            self.dtcs.push(Dtc::new(
                "P0A80",
                format!(
                    "Cell over-voltage: {cell_voltage:.2}V > {:.2}V",
                    self.config.max_cell_voltage
                ),
                DtcSeverity::Critical,
                ComponentId::Bms,
            ));
        }

        if cell_voltage < self.config.min_cell_voltage {
            self.dtcs.push(Dtc::new(
                "P0A81",
                format!(
                    "Cell under-voltage: {cell_voltage:.2}V < {:.2}V",
                    self.config.min_cell_voltage
                ),
                DtcSeverity::Fault,
                ComponentId::Bms,
            ));
        }

        if temperature > self.config.max_temperature {
            self.dtcs.push(Dtc::new(
                "P0A82",
                format!(
                    "Pack over-temperature: {temperature:.1}C > {:.1}C",
                    self.config.max_temperature
                ),
                DtcSeverity::Critical,
                ComponentId::Bms,
            ));
        }

        if temperature < self.config.min_temperature {
            self.dtcs.push(Dtc::new(
                "P0A83",
                format!(
                    "Pack under-temperature: {temperature:.1}C < {:.1}C",
                    self.config.min_temperature
                ),
                DtcSeverity::Warning,
                ComponentId::Bms,
            ));
        }

        let _ = current; // Current faults would go here

        if !self.dtcs.is_empty()
            && self
                .dtcs
                .iter()
                .any(|d| d.severity == DtcSeverity::Critical)
            && self.state != BmsState::Fault
        {
            warn!(dtc_count = self.dtcs.len(), "BMS entering fault state");
            self.state = BmsState::Fault;
        }
    }

    fn update_soc(&mut self, current: f64, dt: Duration) {
        // Coulomb counting: integrate current over time
        let dt_hours = dt.as_secs_f64() / 3600.0;
        self.coulomb_count_ah += current * dt_hours;

        // SOC = initial_soc - (consumed_ah / capacity) * 100
        // Negative current = discharging, positive = charging
        let consumed_pct = (self.coulomb_count_ah / self.config.capacity_ah) * 100.0;
        self.soc = (80.0 - consumed_pct).clamp(0.0, 100.0);
    }

    fn publish_can(&self, voltage: f64, current: f64, temperature: f64) {
        // SOC as u16 (0-10000 = 0.00-100.00%)
        let soc_encoded = (self.soc * 100.0) as u16;
        let _ = self.can_node.send(CanFrame::new(
            well_known::BMS_SOC,
            &soc_encoded.to_le_bytes(),
            0,
        ));

        // Voltage as i32 millivolts
        let mv = (voltage * 1000.0) as i32;
        let _ = self
            .can_node
            .send(CanFrame::new(well_known::BMS_VOLTAGE, &mv.to_le_bytes(), 0));

        // Current as i32 milliamps
        let ma = (current * 1000.0) as i32;
        let _ = self
            .can_node
            .send(CanFrame::new(well_known::BMS_CURRENT, &ma.to_le_bytes(), 0));

        // Temperature as i16 (tenths of degree)
        let temp_encoded = (temperature * 10.0) as i16;
        let _ = self.can_node.send(CanFrame::new(
            well_known::BMS_TEMPERATURE,
            &temp_encoded.to_le_bytes(),
            0,
        ));
    }

    fn process_can_messages(&mut self) {
        for frame in self.can_node.drain() {
            if frame.id == well_known::EMERGENCY_STOP {
                warn!("BMS received emergency stop");
                self.state = BmsState::SafeState;
            }
        }
    }
}

impl Subsystem for BatteryManagementSystem {
    fn init(&mut self) -> Result<(), ppe_core::PpeError> {
        info!("BMS initializing");
        self.state = BmsState::Precharging;
        // Simulate precharge completion
        self.state = BmsState::Active;
        info!(soc = self.soc, "BMS active");
        Ok(())
    }

    fn tick(&mut self, dt: Duration) -> Result<(), ppe_core::PpeError> {
        if self.state == BmsState::SafeState {
            return Ok(());
        }

        self.process_can_messages();

        let voltage =
            self.voltage_sensor
                .read()
                .map_err(|e| ppe_core::PpeError::SubsystemFault {
                    subsystem: "BMS".into(),
                    detail: format!("voltage read failed: {e}"),
                })?;

        let current =
            self.current_sensor
                .read()
                .map_err(|e| ppe_core::PpeError::SubsystemFault {
                    subsystem: "BMS".into(),
                    detail: format!("current read failed: {e}"),
                })?;

        let temperature =
            self.temp_sensor
                .read()
                .map_err(|e| ppe_core::PpeError::SubsystemFault {
                    subsystem: "BMS".into(),
                    detail: format!("temperature read failed: {e}"),
                })?;

        self.update_soc(current, dt);
        self.check_faults(voltage, current, temperature);
        self.publish_can(voltage, current, temperature);

        Ok(())
    }

    fn active_dtcs(&self) -> Vec<Dtc> {
        self.dtcs.clone()
    }

    fn shutdown(&mut self) -> Result<(), ppe_core::PpeError> {
        info!("BMS shutting down");
        self.state = BmsState::Standby;
        Ok(())
    }

    fn health(&self) -> SubsystemHealth {
        match self.state {
            BmsState::Fault | BmsState::SafeState => SubsystemHealth::Fault,
            _ if !self.dtcs.is_empty() => SubsystemHealth::Degraded,
            _ => SubsystemHealth::Ok,
        }
    }

    fn name(&self) -> &str {
        "BMS"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ppe_can::{CanFilter, CanId, VirtualCanBus};

    fn make_bms() -> (BatteryManagementSystem, BmsHandles, BusNode) {
        let bus = VirtualCanBus::new(256);
        let bms_node = bus.connect(
            CanFilter::Any(vec![CanFilter::Exact(well_known::EMERGENCY_STOP)]),
            64,
        );
        let monitor_node = bus.connect(
            CanFilter::Range {
                low: CanId::new(0x100).unwrap(),
                high: CanId::new(0x10F).unwrap(),
            },
            64,
        );
        let config = BmsConfig::default();
        let (bms, handles) = BatteryManagementSystem::new(config, bms_node);
        (bms, handles, monitor_node)
    }

    #[test]
    fn bms_init_and_tick() {
        let (mut bms, _handles, monitor) = make_bms();
        bms.init().unwrap();
        assert_eq!(bms.state(), BmsState::Active);

        for _ in 0..10 {
            bms.tick(Duration::from_millis(10)).unwrap();
        }

        // Should have published CAN frames
        std::thread::sleep(Duration::from_millis(50));
        let frames = monitor.drain();
        assert!(!frames.is_empty(), "BMS should publish CAN frames");
    }

    #[test]
    fn bms_soc_decreases_with_discharge() {
        let (mut bms, handles, _monitor) = make_bms();
        bms.init().unwrap();

        // Set a discharge current (positive = discharge in our convention)
        handles.pack_current.set(100.0); // 100A discharge

        let initial_soc = bms.soc().value();
        for _ in 0..100 {
            bms.tick(Duration::from_millis(100)).unwrap();
        }
        let final_soc = bms.soc().value();

        assert!(
            final_soc < initial_soc,
            "SOC should decrease during discharge: {initial_soc} -> {final_soc}"
        );
    }

    #[test]
    fn bms_detects_over_voltage() {
        let (mut bms, handles, _monitor) = make_bms();
        bms.init().unwrap();

        // Set dangerously high voltage (4.3V per cell * 96 cells)
        handles.pack_voltage.set(4.3 * 96.0);

        bms.tick(Duration::from_millis(10)).unwrap();

        let dtcs = bms.active_dtcs();
        assert!(!dtcs.is_empty(), "Should detect over-voltage");
        assert!(dtcs.iter().any(|d| d.code == "P0A80"));
        assert_eq!(bms.health(), SubsystemHealth::Fault);
    }

    #[test]
    fn bms_detects_over_temperature() {
        let (mut bms, handles, _monitor) = make_bms();
        bms.init().unwrap();

        handles.pack_temperature.set(50.0); // Over 45C limit

        bms.tick(Duration::from_millis(10)).unwrap();

        let dtcs = bms.active_dtcs();
        assert!(dtcs.iter().any(|d| d.code == "P0A82"));
    }

    #[test]
    fn bms_can_message_contains_soc() {
        let (mut bms, _handles, monitor) = make_bms();
        bms.init().unwrap();
        bms.tick(Duration::from_millis(10)).unwrap();

        std::thread::sleep(Duration::from_millis(50));
        let frames = monitor.drain();

        let soc_frame = frames.iter().find(|f| f.id == well_known::BMS_SOC);
        assert!(soc_frame.is_some(), "Should find SOC CAN frame");

        let soc_frame = soc_frame.unwrap();
        let soc_raw = u16::from_le_bytes([soc_frame.data[0], soc_frame.data[1]]);
        let soc_pct = soc_raw as f64 / 100.0;
        assert!(
            (soc_pct - 80.0).abs() < 1.0,
            "SOC should be ~80%: got {soc_pct}"
        );
    }
}
