use std::time::Duration;

use ppe_can::{well_known, BusNode, CanFrame};
use ppe_core::{ComponentId, Dtc, DtcSeverity, Rpm, Torque};
use ppe_hal::{MockSensor, Sensor, SensorHandle};
use ppe_state::MotorState;
use tracing::{info, warn};

use crate::{Subsystem, SubsystemHealth};

/// Sensor handles for physics to write into motor sensors.
pub struct MotorHandles {
    pub rpm: SensorHandle,
    pub torque: SensorHandle,
    pub temperature: SensorHandle,
    pub throttle: SensorHandle,
}

/// Motor controller subsystem.
pub struct MotorController {
    state: MotorState,
    rpm_sensor: MockSensor,
    torque_sensor: MockSensor,
    temp_sensor: MockSensor,
    throttle_sensor: MockSensor,
    can_node: BusNode,
    dtcs: Vec<Dtc>,
    max_temperature: f64,
    derate_temperature: f64,
}

impl MotorController {
    pub fn new(can_node: BusNode) -> (Self, MotorHandles) {
        let (rpm_sensor, rpm_handle) = MockSensor::new_clean("motor_rpm", 0.0);
        let (torque_sensor, torque_handle) = MockSensor::new_clean("motor_torque", 0.0);
        let (temp_sensor, temp_handle) = MockSensor::new_clean("motor_temperature", 25.0);
        let (throttle_sensor, throttle_handle) = MockSensor::new_clean("motor_throttle", 0.0);

        let handles = MotorHandles {
            rpm: rpm_handle,
            torque: torque_handle,
            temperature: temp_handle,
            throttle: throttle_handle,
        };

        let motor = Self {
            state: MotorState::Disabled,
            rpm_sensor,
            torque_sensor,
            temp_sensor,
            throttle_sensor,
            can_node,
            dtcs: Vec::new(),
            max_temperature: 150.0,
            derate_temperature: 120.0,
        };

        (motor, handles)
    }

    pub fn state(&self) -> MotorState {
        self.state
    }

    pub fn rpm(&self) -> Rpm {
        Rpm::new(self.rpm_sensor.read().unwrap_or(0.0))
    }

    pub fn torque(&self) -> Torque {
        Torque::new(self.torque_sensor.read().unwrap_or(0.0))
    }

    fn check_faults(&mut self, temperature: f64) {
        self.dtcs.clear();

        if temperature > self.max_temperature {
            self.dtcs.push(Dtc::new(
                "P0A78",
                format!(
                    "Motor over-temperature: {temperature:.1}C > {:.1}C",
                    self.max_temperature
                ),
                DtcSeverity::Critical,
                ComponentId::Motor,
            ));
            self.state = MotorState::Fault;
        } else if temperature > self.derate_temperature {
            self.dtcs.push(Dtc::new(
                "P0A79",
                format!("Motor temperature high, derating: {temperature:.1}C",),
                DtcSeverity::Warning,
                ComponentId::Motor,
            ));
            if self.state == MotorState::Running {
                self.state = MotorState::Derating;
            }
        } else if self.state == MotorState::Derating {
            self.state = MotorState::Running;
        }
    }

    fn publish_can(&self, rpm: f64, torque: f64, temperature: f64) {
        let rpm_encoded = rpm as i32;
        let _ = self.can_node.send(CanFrame::new(
            well_known::MOTOR_RPM,
            &rpm_encoded.to_le_bytes(),
            0,
        ));

        let torque_encoded = (torque * 100.0) as i32;
        let _ = self.can_node.send(CanFrame::new(
            well_known::MOTOR_TORQUE,
            &torque_encoded.to_le_bytes(),
            0,
        ));

        let temp_encoded = (temperature * 10.0) as i16;
        let _ = self.can_node.send(CanFrame::new(
            well_known::MOTOR_TEMPERATURE,
            &temp_encoded.to_le_bytes(),
            0,
        ));

        let status = self.state as u8;
        let _ = self
            .can_node
            .send(CanFrame::new(well_known::MOTOR_STATUS, &[status], 0));
    }

    fn process_can_messages(&mut self) {
        for frame in self.can_node.drain() {
            if frame.id == well_known::EMERGENCY_STOP {
                warn!("Motor received emergency stop");
                self.state = MotorState::Fault;
            }
        }
    }
}

impl Subsystem for MotorController {
    fn init(&mut self) -> Result<(), ppe_core::PpeError> {
        info!("Motor controller initializing");
        self.state = MotorState::Initializing;
        self.state = MotorState::Ready;
        info!("Motor controller ready");
        Ok(())
    }

    fn tick(&mut self, _dt: Duration) -> Result<(), ppe_core::PpeError> {
        if self.state == MotorState::Fault {
            return Ok(());
        }

        self.process_can_messages();

        let rpm = self.rpm_sensor.read().unwrap_or(0.0);
        let torque = self.torque_sensor.read().unwrap_or(0.0);
        let temperature = self.temp_sensor.read().unwrap_or(25.0);
        let throttle = self.throttle_sensor.read().unwrap_or(0.0);

        // Transition to Running if throttle applied and we're Ready
        if self.state == MotorState::Ready && throttle > 0.01 {
            self.state = MotorState::Running;
        } else if self.state == MotorState::Running && throttle < 0.01 && rpm < 10.0 {
            self.state = MotorState::Ready;
        }

        self.check_faults(temperature);
        self.publish_can(rpm, torque, temperature);

        Ok(())
    }

    fn active_dtcs(&self) -> Vec<Dtc> {
        self.dtcs.clone()
    }

    fn shutdown(&mut self) -> Result<(), ppe_core::PpeError> {
        info!("Motor controller shutting down");
        self.state = MotorState::Disabled;
        Ok(())
    }

    fn health(&self) -> SubsystemHealth {
        match self.state {
            MotorState::Fault => SubsystemHealth::Fault,
            MotorState::Derating => SubsystemHealth::Degraded,
            _ => SubsystemHealth::Ok,
        }
    }

    fn name(&self) -> &str {
        "Motor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ppe_can::{CanFilter, VirtualCanBus};

    #[test]
    fn motor_init_and_state_transitions() {
        let bus = VirtualCanBus::new(256);
        let node = bus.connect(CanFilter::Exact(well_known::EMERGENCY_STOP), 64);
        let (mut motor, handles) = MotorController::new(node);

        motor.init().unwrap();
        assert_eq!(motor.state(), MotorState::Ready);

        // Apply throttle -> Running
        handles.throttle.set(0.5);
        motor.tick(Duration::from_millis(10)).unwrap();
        assert_eq!(motor.state(), MotorState::Running);

        // Overheat -> Derating
        handles.temperature.set(125.0);
        motor.tick(Duration::from_millis(10)).unwrap();
        assert_eq!(motor.state(), MotorState::Derating);
    }

    #[test]
    fn motor_over_temperature_fault() {
        let bus = VirtualCanBus::new(256);
        let node = bus.connect(CanFilter::Exact(well_known::EMERGENCY_STOP), 64);
        let (mut motor, handles) = MotorController::new(node);

        motor.init().unwrap();
        handles.temperature.set(160.0);
        motor.tick(Duration::from_millis(10)).unwrap();
        assert_eq!(motor.state(), MotorState::Fault);
        assert_eq!(motor.health(), SubsystemHealth::Fault);
    }
}
