use std::time::Duration;

use ppe_can::{well_known, BusNode, CanFrame};
use ppe_core::{ComponentId, Dtc, DtcSeverity};
use ppe_hal::{MockSensor, Sensor, SensorHandle};
use ppe_state::EnerDState;
use tracing::{info, warn};

use crate::{Subsystem, SubsystemHealth};

/// Configuration for the Ener-D Reactor subsystem.
#[derive(Debug, Clone)]
pub struct EnerDConfig {
    pub coupling_efficiency: f64,
    pub core_inertia: f64,
    pub core_drag: f64,
    pub max_spin_rate: f64,
    pub power_coefficient: f64,
    pub parasitic_load_kw: f64,
    pub max_safe_output_kw: f64,
    pub sustain_spin_threshold: f64,
    pub overdrive_spin_threshold: f64,
    pub min_activation_speed: f64,
    pub containment_degrade_coeff: f64,
    pub containment_regen_rate: f64,
    pub plasma_heat_rate: f64,
    pub plasma_cool_rate: f64,
    pub critical_containment: f64,
    pub meltdown_containment: f64,
    pub critical_plasma_temp: f64,
    pub meltdown_plasma_temp: f64,
    pub spinup_timeout_secs: f64,
}

impl Default for EnerDConfig {
    fn default() -> Self {
        Self {
            coupling_efficiency: 0.20,
            core_inertia: 8.0,
            core_drag: 0.5,
            max_spin_rate: 800.0,
            power_coefficient: 0.0012,
            parasitic_load_kw: 2.0,
            max_safe_output_kw: 250.0,
            sustain_spin_threshold: 150.0,
            overdrive_spin_threshold: 400.0,
            min_activation_speed: 5.0,
            containment_degrade_coeff: 15.0,
            containment_regen_rate: 5.0,
            plasma_heat_rate: 0.008,
            plasma_cool_rate: 2.5,
            critical_containment: 50.0,
            meltdown_containment: 10.0,
            critical_plasma_temp: 80.0,
            meltdown_plasma_temp: 100.0,
            spinup_timeout_secs: 30.0,
        }
    }
}

/// Sensor handles for physics to write values into the reactor.
pub struct EnerDHandles {
    pub vehicle_speed: SensorHandle,
    pub vehicle_accel: SensorHandle,
    pub vehicle_drag_force: SensorHandle,
    pub vehicle_mass: SensorHandle,
}

/// Ener-D Reactor subsystem.
pub struct EnerDReactor {
    config: EnerDConfig,
    state: EnerDState,
    spin_rate: f64,
    containment: f64,
    plasma_temp: f64,
    gross_power_kw: f64,
    net_power_kw: f64,
    momentum_flux: f64,
    can_node: BusNode,
    dtcs: Vec<Dtc>,
    speed_sensor: MockSensor,
    accel_sensor: MockSensor,
    drag_sensor: MockSensor,
    mass_sensor: MockSensor,
    state_time: f64,
    critical_stable_time: f64,
    enabled: bool,
    scram_requested: bool,
}

impl EnerDReactor {
    pub fn new(config: EnerDConfig, can_node: BusNode) -> (Self, EnerDHandles) {
        let (speed_sensor, speed_handle) = MockSensor::new_clean("enerd_vehicle_speed", 0.0);
        let (accel_sensor, accel_handle) = MockSensor::new_clean("enerd_vehicle_accel", 0.0);
        let (drag_sensor, drag_handle) = MockSensor::new_clean("enerd_vehicle_drag_force", 0.0);
        let (mass_sensor, mass_handle) = MockSensor::new_clean("enerd_vehicle_mass", 1800.0);

        let handles = EnerDHandles {
            vehicle_speed: speed_handle,
            vehicle_accel: accel_handle,
            vehicle_drag_force: drag_handle,
            vehicle_mass: mass_handle,
        };

        let reactor = Self {
            config,
            state: EnerDState::Dormant,
            spin_rate: 0.0,
            containment: 100.0,
            plasma_temp: 0.1,
            gross_power_kw: 0.0,
            net_power_kw: 0.0,
            momentum_flux: 0.0,
            can_node,
            dtcs: Vec::new(),
            speed_sensor,
            accel_sensor,
            drag_sensor,
            mass_sensor,
            state_time: 0.0,
            critical_stable_time: 0.0,
            enabled: false,
            scram_requested: false,
        };

        (reactor, handles)
    }

    pub fn state(&self) -> EnerDState {
        self.state
    }

    pub fn spin_rate(&self) -> f64 {
        self.spin_rate
    }

    pub fn net_power_kw(&self) -> f64 {
        self.net_power_kw
    }

    pub fn gross_power_kw(&self) -> f64 {
        self.gross_power_kw
    }

    pub fn containment(&self) -> f64 {
        self.containment
    }

    pub fn plasma_temp(&self) -> f64 {
        self.plasma_temp
    }

    pub fn momentum_flux(&self) -> f64 {
        self.momentum_flux
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn request_scram(&mut self) {
        self.scram_requested = true;
    }

    fn read_sensors(&self) -> Result<(f64, f64, f64, f64), ppe_core::PpeError> {
        let speed = self
            .speed_sensor
            .read()
            .map_err(|e| ppe_core::PpeError::SubsystemFault {
                subsystem: "ENER-D".into(),
                detail: format!("speed read failed: {e}"),
            })?;

        let accel = self
            .accel_sensor
            .read()
            .map_err(|e| ppe_core::PpeError::SubsystemFault {
                subsystem: "ENER-D".into(),
                detail: format!("accel read failed: {e}"),
            })?;

        let drag_force =
            self.drag_sensor
                .read()
                .map_err(|e| ppe_core::PpeError::SubsystemFault {
                    subsystem: "ENER-D".into(),
                    detail: format!("drag read failed: {e}"),
                })?;

        let mass = self
            .mass_sensor
            .read()
            .map_err(|e| ppe_core::PpeError::SubsystemFault {
                subsystem: "ENER-D".into(),
                detail: format!("mass read failed: {e}"),
            })?;

        Ok((speed, accel, drag_force, mass))
    }

    fn update_physics(&mut self, dt: f64, speed: f64, accel: f64, drag_force: f64, mass: f64) {
        // Guard against non-finite sensor inputs
        if !dt.is_finite()
            || !speed.is_finite()
            || !accel.is_finite()
            || !drag_force.is_finite()
            || !mass.is_finite()
            || mass <= 0.0
        {
            warn!("Ener-D: non-finite or invalid sensor input, skipping physics tick");
            return;
        }

        // Momentum flux from vehicle dynamics
        let flux = self.config.coupling_efficiency * (drag_force.abs() + mass * accel.max(0.0));
        self.momentum_flux = flux;

        // Spinup assist torque
        let spinup_torque = if self.state == EnerDState::SpinUp {
            50.0
        } else {
            0.0
        };

        // Core spin dynamics
        let angular_accel = (flux + spinup_torque - self.config.core_drag * self.spin_rate)
            / self.config.core_inertia;
        self.spin_rate =
            (self.spin_rate + angular_accel * dt).clamp(0.0, self.config.max_spin_rate);

        // Power output
        self.gross_power_kw = self.config.power_coefficient * self.spin_rate * self.spin_rate;
        self.net_power_kw = (self.gross_power_kw - self.config.parasitic_load_kw).max(0.0);

        // Containment field integrity
        let ratio = self.gross_power_kw / self.config.max_safe_output_kw;
        let degrade = self.config.containment_degrade_coeff * ratio * ratio * ratio * dt;
        let regen = if ratio < 0.8 {
            self.config.containment_regen_rate * dt
        } else {
            0.0
        };
        self.containment = (self.containment - degrade + regen).clamp(0.0, 100.0);

        // Plasma temperature
        let heat = self.config.plasma_heat_rate * self.gross_power_kw * dt;
        let containment_factor = (self.containment / 100.0).max(0.1);
        let cool = self.config.plasma_cool_rate
            * containment_factor
            * (self.plasma_temp - 0.1).max(0.0)
            * dt;
        self.plasma_temp = (self.plasma_temp + heat - cool).max(0.1);

        let _ = speed; // Used only in state transitions
    }

    fn transition_state(&mut self, dt: f64, speed: f64) {
        self.state_time += dt;

        // SCRAM takes priority in all non-terminal states
        if self.scram_requested && self.state != EnerDState::Meltdown {
            self.state = EnerDState::Dormant;
            self.state_time = 0.0;
            self.spin_rate = 0.0;
            self.containment = 100.0;
            self.plasma_temp = 0.1;
            self.scram_requested = false;
            self.dtcs.push(Dtc::new(
                "P0EDA",
                "Ener-D reactor SCRAM executed",
                DtcSeverity::Warning,
                ComponentId::EnerD,
            ));
            return;
        }

        match self.state {
            EnerDState::Dormant => {
                if speed > self.config.min_activation_speed && self.enabled {
                    self.state = EnerDState::SpinUp;
                    self.state_time = 0.0;
                    self.dtcs.push(Dtc::new(
                        "P0ED0",
                        "Ener-D reactor entering spin-up",
                        DtcSeverity::Info,
                        ComponentId::EnerD,
                    ));
                }
            }
            EnerDState::SpinUp => {
                if self.spin_rate >= self.config.sustain_spin_threshold
                    && self.net_power_kw >= self.config.parasitic_load_kw
                {
                    self.state = EnerDState::Sustaining;
                    self.state_time = 0.0;
                    self.dtcs.push(Dtc::new(
                        "P0ED1",
                        "Ener-D reactor sustaining",
                        DtcSeverity::Info,
                        ComponentId::EnerD,
                    ));
                } else if speed < 3.0 {
                    self.state = EnerDState::Dormant;
                    self.state_time = 0.0;
                } else if self.state_time > self.config.spinup_timeout_secs {
                    self.state = EnerDState::Dormant;
                    self.state_time = 0.0;
                    self.dtcs.push(Dtc::new(
                        "P0ED6",
                        "Ener-D spin-up timeout",
                        DtcSeverity::Warning,
                        ComponentId::EnerD,
                    ));
                }
            }
            EnerDState::Sustaining => {
                if self.spin_rate > self.config.overdrive_spin_threshold && self.net_power_kw > 0.0
                {
                    self.state = EnerDState::Overdrive;
                    self.state_time = 0.0;
                    self.dtcs.push(Dtc::new(
                        "P0ED2",
                        "Ener-D reactor in overdrive",
                        DtcSeverity::Info,
                        ComponentId::EnerD,
                    ));
                } else if self.spin_rate < self.config.sustain_spin_threshold * 0.5 {
                    self.state = EnerDState::Dormant;
                    self.state_time = 0.0;
                }
                // If speed < min_activation_speed, let physics handle spindown naturally
            }
            EnerDState::Overdrive => {
                if self.containment < self.config.critical_containment
                    || self.plasma_temp > self.config.critical_plasma_temp
                    || self.net_power_kw > self.config.max_safe_output_kw
                {
                    self.state = EnerDState::Critical;
                    self.state_time = 0.0;
                    self.dtcs.push(Dtc::new(
                        "P0ED5",
                        "Ener-D reactor critical",
                        DtcSeverity::Critical,
                        ComponentId::EnerD,
                    ));
                } else if self.spin_rate <= self.config.overdrive_spin_threshold {
                    self.state = EnerDState::Sustaining;
                    self.state_time = 0.0;
                }
            }
            EnerDState::Critical => {
                // Recovery check
                if self.containment > 70.0 && self.plasma_temp < 60.0 {
                    self.critical_stable_time += dt;
                    if self.critical_stable_time >= 3.0 {
                        self.state = EnerDState::Sustaining;
                        self.state_time = 0.0;
                        self.critical_stable_time = 0.0;
                        self.dtcs.push(Dtc::new(
                            "P0EDB",
                            "Ener-D reactor recovered from critical",
                            DtcSeverity::Info,
                            ComponentId::EnerD,
                        ));
                    }
                } else {
                    self.critical_stable_time = 0.0;
                }

                // Meltdown check
                if self.containment < self.config.meltdown_containment
                    || self.plasma_temp > self.config.meltdown_plasma_temp
                    || self.state_time > 15.0
                {
                    self.state = EnerDState::Meltdown;
                    self.state_time = 0.0;
                    self.scram_requested = false;
                    self.dtcs.push(Dtc::new(
                        "P0ED7",
                        "Ener-D reactor MELTDOWN",
                        DtcSeverity::Critical,
                        ComponentId::EnerD,
                    ));
                }
            }
            EnerDState::Meltdown => {
                // Terminal state: spin decays slowly
                self.spin_rate *= 0.99;
            }
        }
    }

    fn check_warning_dtcs(&mut self) {
        if self.containment < 70.0 {
            self.dtcs.push(Dtc::new(
                "P0ED3",
                format!("Containment low: {:.1}%", self.containment),
                DtcSeverity::Warning,
                ComponentId::EnerD,
            ));
        }

        if self.plasma_temp > 50.0 {
            self.dtcs.push(Dtc::new(
                "P0ED4",
                format!("Plasma temp elevated: {:.1} MK", self.plasma_temp),
                DtcSeverity::Warning,
                ComponentId::EnerD,
            ));
        }

        if self.spin_rate > self.config.max_spin_rate * 0.9 {
            self.dtcs.push(Dtc::new(
                "P0ED8",
                format!("Spin rate near max: {:.1} rad/s", self.spin_rate),
                DtcSeverity::Warning,
                ComponentId::EnerD,
            ));
        }

        if self.net_power_kw > self.config.max_safe_output_kw * 0.8 {
            self.dtcs.push(Dtc::new(
                "P0ED9",
                format!("Power output near limit: {:.1} kW", self.net_power_kw),
                DtcSeverity::Warning,
                ComponentId::EnerD,
            ));
        }
    }

    fn publish_can(&self) {
        // Status: state as u8
        let state_byte = match self.state {
            EnerDState::Dormant => 0u8,
            EnerDState::SpinUp => 1,
            EnerDState::Sustaining => 2,
            EnerDState::Overdrive => 3,
            EnerDState::Critical => 4,
            EnerDState::Meltdown => 5,
        };
        let _ = self
            .can_node
            .send(CanFrame::new(well_known::ENERD_STATUS, &[state_byte], 0));

        // Spin rate as i32 millirad/s
        let spin_mrad = (self.spin_rate * 1000.0) as i32;
        let _ = self.can_node.send(CanFrame::new(
            well_known::ENERD_SPIN_RATE,
            &spin_mrad.to_le_bytes(),
            0,
        ));

        // Power output as i32 milliwatts
        let power_mw = (self.net_power_kw * 1000.0) as i32;
        let _ = self.can_node.send(CanFrame::new(
            well_known::ENERD_POWER_OUTPUT,
            &power_mw.to_le_bytes(),
            0,
        ));

        // Containment as u16 hundredths of percent
        let containment_encoded = (self.containment * 100.0) as u16;
        let _ = self.can_node.send(CanFrame::new(
            well_known::ENERD_CONTAINMENT,
            &containment_encoded.to_le_bytes(),
            0,
        ));

        // Plasma temp as i32 hundredths of MK
        let plasma_encoded = (self.plasma_temp * 100.0) as i32;
        let _ = self.can_node.send(CanFrame::new(
            well_known::ENERD_PLASMA_TEMP,
            &plasma_encoded.to_le_bytes(),
            0,
        ));

        // Momentum flux as i32 millinewtons
        let flux_mn = (self.momentum_flux * 1000.0) as i32;
        let _ = self.can_node.send(CanFrame::new(
            well_known::ENERD_MOMENTUM_FLUX,
            &flux_mn.to_le_bytes(),
            0,
        ));
    }

    fn process_can_messages(&mut self) {
        for frame in self.can_node.drain() {
            if frame.id == well_known::EMERGENCY_STOP {
                warn!("Ener-D received emergency stop");
                self.scram_requested = true;
            }
        }
    }
}

impl Subsystem for EnerDReactor {
    fn init(&mut self) -> Result<(), ppe_core::PpeError> {
        info!("Ener-D reactor initializing");
        Ok(())
    }

    fn tick(&mut self, dt: Duration) -> Result<(), ppe_core::PpeError> {
        self.process_can_messages();

        let (speed, accel, drag_force, mass) = self.read_sensors()?;
        let dt_secs = dt.as_secs_f64();

        self.dtcs.clear();

        if !self.enabled || self.state == EnerDState::Meltdown {
            if self.state == EnerDState::Meltdown {
                self.state_time += dt_secs;
                self.spin_rate *= 0.99;
                self.scram_requested = false; // SCRAM cannot save a meltdown
            }
            self.publish_can();
            return Ok(());
        }

        self.update_physics(dt_secs, speed, accel, drag_force, mass);
        self.transition_state(dt_secs, speed);
        self.check_warning_dtcs();
        self.publish_can();

        Ok(())
    }

    fn active_dtcs(&self) -> Vec<Dtc> {
        self.dtcs.clone()
    }

    fn shutdown(&mut self) -> Result<(), ppe_core::PpeError> {
        info!("Ener-D reactor shutting down");
        self.state = EnerDState::Dormant;
        self.spin_rate = 0.0;
        self.containment = 100.0;
        self.plasma_temp = 0.1;
        Ok(())
    }

    fn health(&self) -> SubsystemHealth {
        match self.state {
            EnerDState::Meltdown | EnerDState::Critical => SubsystemHealth::Fault,
            EnerDState::Dormant if !self.dtcs.is_empty() => SubsystemHealth::Degraded,
            _ => SubsystemHealth::Ok,
        }
    }

    fn name(&self) -> &str {
        "ENER-D"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ppe_can::{CanFilter, CanId, VirtualCanBus};

    fn make_reactor() -> (EnerDReactor, EnerDHandles, BusNode) {
        let bus = VirtualCanBus::new(256);
        let reactor_node = bus.connect(
            CanFilter::Any(vec![CanFilter::Exact(well_known::EMERGENCY_STOP)]),
            64,
        );
        let monitor_node = bus.connect(
            CanFilter::Range {
                low: CanId::new(0x500).unwrap(),
                high: CanId::new(0x50F).unwrap(),
            },
            64,
        );
        let config = EnerDConfig::default();
        let (reactor, handles) = EnerDReactor::new(config, reactor_node);
        (reactor, handles, monitor_node)
    }

    #[test]
    fn reactor_starts_dormant() {
        let (reactor, _handles, _monitor) = make_reactor();
        assert_eq!(reactor.state(), EnerDState::Dormant);
        assert_eq!(reactor.spin_rate(), 0.0);
        assert_eq!(reactor.containment(), 100.0);
    }

    #[test]
    fn reactor_spins_up_with_speed() {
        let (mut reactor, handles, _monitor) = make_reactor();
        reactor.init().unwrap();
        reactor.set_enabled(true);

        handles.vehicle_speed.set(20.0);
        handles.vehicle_accel.set(2.0);
        handles.vehicle_drag_force.set(500.0);
        handles.vehicle_mass.set(1800.0);

        // Tick until we leave Dormant
        let mut entered_spinup = false;
        let mut entered_sustaining = false;

        for _ in 0..2000 {
            reactor.tick(Duration::from_millis(10)).unwrap();

            if reactor.state() == EnerDState::SpinUp {
                entered_spinup = true;
            }
            if reactor.state() == EnerDState::Sustaining {
                entered_sustaining = true;
                break;
            }
        }

        assert!(entered_spinup, "Reactor should have entered SpinUp");
        assert!(
            entered_sustaining,
            "Reactor should have reached Sustaining, spin_rate={:.1}",
            reactor.spin_rate()
        );
    }

    #[test]
    fn reactor_stays_dormant_when_disabled() {
        let (mut reactor, handles, _monitor) = make_reactor();
        reactor.init().unwrap();
        // Do NOT enable

        handles.vehicle_speed.set(20.0);
        handles.vehicle_accel.set(2.0);
        handles.vehicle_drag_force.set(500.0);
        handles.vehicle_mass.set(1800.0);

        for _ in 0..100 {
            reactor.tick(Duration::from_millis(10)).unwrap();
        }

        assert_eq!(
            reactor.state(),
            EnerDState::Dormant,
            "Reactor should stay Dormant when disabled"
        );
    }

    #[test]
    fn reactor_scram() {
        let (mut reactor, handles, _monitor) = make_reactor();
        reactor.init().unwrap();
        reactor.set_enabled(true);

        handles.vehicle_speed.set(20.0);
        handles.vehicle_accel.set(2.0);
        handles.vehicle_drag_force.set(500.0);
        handles.vehicle_mass.set(1800.0);

        // Get to Sustaining
        for _ in 0..2000 {
            reactor.tick(Duration::from_millis(10)).unwrap();
            if reactor.state() == EnerDState::Sustaining {
                break;
            }
        }
        assert_eq!(reactor.state(), EnerDState::Sustaining);

        // SCRAM works from any non-terminal state
        reactor.request_scram();
        reactor.tick(Duration::from_millis(10)).unwrap();
        assert_eq!(
            reactor.state(),
            EnerDState::Dormant,
            "Reactor should return to Dormant after SCRAM"
        );
        assert_eq!(reactor.spin_rate(), 0.0);
        assert_eq!(reactor.containment(), 100.0);
    }

    #[test]
    fn reactor_containment_degrades_at_high_power() {
        let (mut reactor, handles, _monitor) = make_reactor();
        reactor.init().unwrap();
        reactor.set_enabled(true);

        handles.vehicle_speed.set(40.0);
        handles.vehicle_accel.set(10.0);
        handles.vehicle_drag_force.set(3000.0);
        handles.vehicle_mass.set(1800.0);

        let initial_containment = reactor.containment();

        // Run for many ticks to build up spin and power
        for _ in 0..5000 {
            reactor.tick(Duration::from_millis(10)).unwrap();
        }

        assert!(
            reactor.containment() < initial_containment,
            "Containment should degrade at high power: {:.1}% vs initial {:.1}%",
            reactor.containment(),
            initial_containment
        );
    }
}
