use std::time::Duration;

use ppe_can::{well_known, BusNode, CanFrame};
use ppe_core::{ComponentId, Dtc, DtcSeverity, Percent};
use ppe_hal::{MockSensor, Sensor, SensorHandle};
use tracing::{info, warn};

use crate::{Subsystem, SubsystemHealth};

/// Cooling system state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoolingState {
    Off,
    LowSpeed,
    HighSpeed,
    Emergency,
    Fault,
}

impl std::fmt::Display for CoolingState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Off => write!(f, "OFF"),
            Self::LowSpeed => write!(f, "LOW"),
            Self::HighSpeed => write!(f, "HIGH"),
            Self::Emergency => write!(f, "EMERGENCY"),
            Self::Fault => write!(f, "FAULT"),
        }
    }
}

/// Sensor handles for physics to write into thermal sensors.
pub struct ThermalHandles {
    pub coolant_temp: SensorHandle,
    pub ambient_temp: SensorHandle,
}

/// Thermal management subsystem.
pub struct ThermalManagement {
    state: CoolingState,
    coolant_sensor: MockSensor,
    ambient_sensor: MockSensor,
    can_node: BusNode,
    dtcs: Vec<Dtc>,
    fan_speed: Percent,
    // Thresholds
    low_threshold: f64,
    high_threshold: f64,
    emergency_threshold: f64,
    fault_threshold: f64,
}

impl ThermalManagement {
    pub fn new(can_node: BusNode) -> (Self, ThermalHandles) {
        let (coolant_sensor, coolant_handle) = MockSensor::new_clean("coolant_temp", 25.0);
        let (ambient_sensor, ambient_handle) = MockSensor::new_clean("ambient_temp", 25.0);

        let handles = ThermalHandles {
            coolant_temp: coolant_handle,
            ambient_temp: ambient_handle,
        };

        let thermal = Self {
            state: CoolingState::Off,
            coolant_sensor,
            ambient_sensor,
            can_node,
            dtcs: Vec::new(),
            fan_speed: Percent::new(0.0),
            low_threshold: 40.0,
            high_threshold: 60.0,
            emergency_threshold: 80.0,
            fault_threshold: 95.0,
        };

        (thermal, handles)
    }

    pub fn state(&self) -> CoolingState {
        self.state
    }

    pub fn fan_speed(&self) -> Percent {
        self.fan_speed
    }

    fn update_cooling(&mut self, coolant_temp: f64) {
        self.dtcs.clear();

        if coolant_temp >= self.fault_threshold {
            self.state = CoolingState::Fault;
            self.fan_speed = Percent::new(100.0);
            self.dtcs.push(Dtc::new(
                "P0217",
                format!("Coolant over-temperature: {coolant_temp:.1}C"),
                DtcSeverity::Critical,
                ComponentId::Thermal,
            ));
        } else if coolant_temp >= self.emergency_threshold {
            self.state = CoolingState::Emergency;
            self.fan_speed = Percent::new(100.0);
            self.dtcs.push(Dtc::new(
                "P0218",
                format!("Coolant temperature high: {coolant_temp:.1}C"),
                DtcSeverity::Warning,
                ComponentId::Thermal,
            ));
        } else if coolant_temp >= self.high_threshold {
            self.state = CoolingState::HighSpeed;
            let pct = ((coolant_temp - self.high_threshold)
                / (self.emergency_threshold - self.high_threshold))
                * 50.0
                + 50.0;
            self.fan_speed = Percent::new(pct.clamp(50.0, 100.0));
        } else if coolant_temp >= self.low_threshold {
            self.state = CoolingState::LowSpeed;
            let pct = ((coolant_temp - self.low_threshold)
                / (self.high_threshold - self.low_threshold))
                * 50.0;
            self.fan_speed = Percent::new(pct.clamp(0.0, 50.0));
        } else {
            self.state = CoolingState::Off;
            self.fan_speed = Percent::new(0.0);
        }
    }

    fn publish_can(&self, coolant_temp: f64) {
        let temp_encoded = (coolant_temp * 10.0) as i16;
        let _ = self.can_node.send(CanFrame::new(
            well_known::THERMAL_COOLANT_TEMP,
            &temp_encoded.to_le_bytes(),
            0,
        ));

        let fan_encoded = (self.fan_speed.value() * 100.0) as u16;
        let _ = self.can_node.send(CanFrame::new(
            well_known::THERMAL_FAN_SPEED,
            &fan_encoded.to_le_bytes(),
            0,
        ));

        let status = self.state as u8;
        let _ = self
            .can_node
            .send(CanFrame::new(well_known::THERMAL_STATUS, &[status], 0));
    }

    fn process_can_messages(&mut self) {
        for frame in self.can_node.drain() {
            if frame.id == well_known::EMERGENCY_STOP {
                warn!("Thermal received emergency stop");
                self.state = CoolingState::Emergency;
                self.fan_speed = Percent::new(100.0);
            }
        }
    }
}

impl Subsystem for ThermalManagement {
    fn init(&mut self) -> Result<(), ppe_core::PpeError> {
        info!("Thermal management initializing");
        self.state = CoolingState::Off;
        Ok(())
    }

    fn tick(&mut self, _dt: Duration) -> Result<(), ppe_core::PpeError> {
        self.process_can_messages();

        let coolant_temp = self.coolant_sensor.read().unwrap_or(25.0);
        let _ambient = self.ambient_sensor.read().unwrap_or(25.0);

        self.update_cooling(coolant_temp);
        self.publish_can(coolant_temp);

        Ok(())
    }

    fn active_dtcs(&self) -> Vec<Dtc> {
        self.dtcs.clone()
    }

    fn shutdown(&mut self) -> Result<(), ppe_core::PpeError> {
        info!("Thermal management shutting down");
        self.state = CoolingState::Off;
        self.fan_speed = Percent::new(0.0);
        Ok(())
    }

    fn health(&self) -> SubsystemHealth {
        match self.state {
            CoolingState::Fault => SubsystemHealth::Fault,
            CoolingState::Emergency => SubsystemHealth::Degraded,
            _ => SubsystemHealth::Ok,
        }
    }

    fn name(&self) -> &str {
        "Thermal"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ppe_can::{CanFilter, VirtualCanBus};

    #[test]
    fn thermal_cooling_stages() {
        let bus = VirtualCanBus::new(256);
        let node = bus.connect(CanFilter::Exact(well_known::EMERGENCY_STOP), 64);
        let (mut thermal, handles) = ThermalManagement::new(node);
        thermal.init().unwrap();

        // Cold - no cooling
        handles.coolant_temp.set(30.0);
        thermal.tick(Duration::from_millis(100)).unwrap();
        assert_eq!(thermal.state(), CoolingState::Off);

        // Warm - low speed
        handles.coolant_temp.set(50.0);
        thermal.tick(Duration::from_millis(100)).unwrap();
        assert_eq!(thermal.state(), CoolingState::LowSpeed);

        // Hot - high speed
        handles.coolant_temp.set(70.0);
        thermal.tick(Duration::from_millis(100)).unwrap();
        assert_eq!(thermal.state(), CoolingState::HighSpeed);

        // Critical - emergency
        handles.coolant_temp.set(85.0);
        thermal.tick(Duration::from_millis(100)).unwrap();
        assert_eq!(thermal.state(), CoolingState::Emergency);

        // Over fault threshold
        handles.coolant_temp.set(100.0);
        thermal.tick(Duration::from_millis(100)).unwrap();
        assert_eq!(thermal.state(), CoolingState::Fault);
        assert_eq!(thermal.health(), SubsystemHealth::Fault);
    }
}
