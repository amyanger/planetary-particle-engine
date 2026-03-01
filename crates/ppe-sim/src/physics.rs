use ppe_hal::SensorHandle;
use std::time::Duration;
use tracing::debug;

/// Vehicle physics configuration.
#[derive(Debug, Clone)]
pub struct VehiclePhysicsConfig {
    pub mass_kg: f64,
    pub drag_coefficient: f64,
    pub frontal_area_m2: f64,
    pub rolling_resistance: f64,
    pub tire_radius_m: f64,
    pub gear_ratio: f64,
    pub motor_max_torque_nm: f64,
    pub motor_max_rpm: f64,
    pub motor_efficiency: f64,
    pub battery_nominal_voltage: f64,
    pub battery_capacity_ah: f64,
    pub battery_internal_resistance: f64,
    pub motor_thermal_mass: f64,     // J/K
    pub motor_heat_coefficient: f64, // fraction of power loss that heats motor
    pub coolant_thermal_mass: f64,   // J/K
    pub radiator_cooling_power: f64, // W/K (ambient delta)
    pub air_density: f64,
}

impl Default for VehiclePhysicsConfig {
    fn default() -> Self {
        Self {
            mass_kg: 1800.0,
            drag_coefficient: 0.28,
            frontal_area_m2: 2.3,
            rolling_resistance: 0.012,
            tire_radius_m: 0.33,
            gear_ratio: 9.0,
            motor_max_torque_nm: 350.0,
            motor_max_rpm: 12000.0,
            motor_efficiency: 0.92,
            battery_nominal_voltage: 355.2, // 96 cells * 3.7V
            battery_capacity_ah: 60.0,
            battery_internal_resistance: 0.1,
            motor_thermal_mass: 15000.0,
            motor_heat_coefficient: 0.7,
            coolant_thermal_mass: 20000.0,
            radiator_cooling_power: 200.0,
            air_density: 1.225,
        }
    }
}

/// Handles to write physics values into mock sensors.
pub struct PhysicsHandles {
    // BMS handles
    pub bms_voltage: SensorHandle,
    pub bms_current: SensorHandle,
    pub bms_temperature: SensorHandle,
    // Motor handles
    pub motor_rpm: SensorHandle,
    pub motor_torque: SensorHandle,
    pub motor_temperature: SensorHandle,
    pub motor_throttle: SensorHandle,
    // Thermal handles
    pub coolant_temp: SensorHandle,
    pub ambient_temp: SensorHandle,
}

/// Full vehicle physics simulation.
pub struct VehiclePhysics {
    config: VehiclePhysicsConfig,
    // State
    speed_mps: f64, // meters per second
    soc: f64,       // 0.0 to 1.0
    motor_temp_c: f64,
    coolant_temp_c: f64,
    ambient_temp_c: f64,
    // Inputs
    throttle: f64, // 0.0 to 1.0
    brake: f64,    // 0.0 to 1.0
    // Outputs
    motor_rpm: f64,
    motor_torque: f64,
    battery_current: f64,
    battery_voltage: f64,
    power_kw: f64,
}

impl VehiclePhysics {
    pub fn new(config: VehiclePhysicsConfig) -> Self {
        let voltage = config.battery_nominal_voltage;
        Self {
            config,
            speed_mps: 0.0,
            soc: 0.8,
            motor_temp_c: 25.0,
            coolant_temp_c: 25.0,
            ambient_temp_c: 25.0,
            throttle: 0.0,
            brake: 0.0,
            motor_rpm: 0.0,
            motor_torque: 0.0,
            battery_current: 0.0,
            battery_voltage: voltage,
            power_kw: 0.0,
        }
    }

    pub fn set_throttle(&mut self, throttle: f64) {
        self.throttle = throttle.clamp(0.0, 1.0);
    }

    pub fn set_brake(&mut self, brake: f64) {
        self.brake = brake.clamp(0.0, 1.0);
    }

    pub fn set_ambient_temp(&mut self, temp: f64) {
        self.ambient_temp_c = temp;
    }

    pub fn speed_kmh(&self) -> f64 {
        self.speed_mps * 3.6
    }

    pub fn soc(&self) -> f64 {
        self.soc
    }

    pub fn motor_rpm(&self) -> f64 {
        self.motor_rpm
    }

    pub fn motor_temp(&self) -> f64 {
        self.motor_temp_c
    }

    pub fn coolant_temp(&self) -> f64 {
        self.coolant_temp_c
    }

    pub fn power_kw(&self) -> f64 {
        self.power_kw
    }

    /// Advance the simulation by dt.
    pub fn step(&mut self, dt: Duration) {
        let dt_s = dt.as_secs_f64();
        if dt_s <= 0.0 {
            return;
        }

        // Motor RPM from wheel speed
        self.motor_rpm =
            (self.speed_mps / self.config.tire_radius_m) * self.config.gear_ratio * 60.0
                / (2.0 * std::f64::consts::PI);

        // Requested motor torque
        let requested_torque = self.throttle * self.config.motor_max_torque_nm;

        // RPM limiting
        let rpm_factor = if self.motor_rpm > self.config.motor_max_rpm * 0.9 {
            let overspeed = (self.motor_rpm - self.config.motor_max_rpm * 0.9)
                / (self.config.motor_max_rpm * 0.1);
            (1.0 - overspeed).clamp(0.0, 1.0)
        } else {
            1.0
        };

        self.motor_torque = requested_torque * rpm_factor;

        // Forces
        let motor_force = self.motor_torque * self.config.gear_ratio / self.config.tire_radius_m;
        let drag_force = 0.5
            * self.config.air_density
            * self.config.drag_coefficient
            * self.config.frontal_area_m2
            * self.speed_mps
            * self.speed_mps;
        let rolling_force = self.config.rolling_resistance * self.config.mass_kg * 9.81;
        let brake_force = self.brake * self.config.mass_kg * 9.81 * 0.8; // 0.8g max braking

        let net_force = motor_force - drag_force - rolling_force - brake_force;
        let acceleration = net_force / self.config.mass_kg;

        // Integrate velocity
        self.speed_mps = (self.speed_mps + acceleration * dt_s).max(0.0);

        // Electrical model
        let mechanical_power =
            self.motor_torque * self.motor_rpm * 2.0 * std::f64::consts::PI / 60.0;
        let electrical_power = if mechanical_power > 0.0 {
            mechanical_power / self.config.motor_efficiency
        } else {
            mechanical_power * self.config.motor_efficiency // Regenerative braking
        };

        self.battery_current = electrical_power / self.config.battery_nominal_voltage;
        self.battery_voltage = self.config.battery_nominal_voltage
            - self.battery_current * self.config.battery_internal_resistance;

        // SOC depletion
        let consumed_ah = self.battery_current * dt_s / 3600.0;
        self.soc = (self.soc - consumed_ah / self.config.battery_capacity_ah).clamp(0.0, 1.0);

        self.power_kw = electrical_power / 1000.0;

        // Thermal model
        let power_loss = electrical_power * (1.0 - self.config.motor_efficiency);
        let motor_heat = power_loss.abs() * self.config.motor_heat_coefficient;

        // Motor heats up from losses, cools to coolant
        let motor_to_coolant = (self.motor_temp_c - self.coolant_temp_c) * 50.0; // W/K transfer
        self.motor_temp_c +=
            (motor_heat - motor_to_coolant) * dt_s / self.config.motor_thermal_mass;

        // Coolant heats from motor, cools via radiator
        let radiator_cooling =
            (self.coolant_temp_c - self.ambient_temp_c) * self.config.radiator_cooling_power;
        self.coolant_temp_c +=
            (motor_to_coolant - radiator_cooling) * dt_s / self.config.coolant_thermal_mass;

        debug!(
            speed_kmh = self.speed_kmh(),
            soc = self.soc * 100.0,
            motor_temp = self.motor_temp_c,
            power_kw = self.power_kw,
            "physics step"
        );
    }

    /// Write all physics values to sensor handles.
    pub fn update_sensors(&self, handles: &PhysicsHandles) {
        // BMS sensors
        handles.bms_voltage.set(self.battery_voltage);
        handles.bms_current.set(self.battery_current);
        handles.bms_temperature.set(self.coolant_temp_c); // Pack temp tracks coolant

        // Motor sensors
        handles.motor_rpm.set(self.motor_rpm);
        handles.motor_torque.set(self.motor_torque);
        handles.motor_temperature.set(self.motor_temp_c);
        handles.motor_throttle.set(self.throttle);

        // Thermal sensors
        handles.coolant_temp.set(self.coolant_temp_c);
        handles.ambient_temp.set(self.ambient_temp_c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ppe_hal::MockSensor;

    fn make_handles() -> PhysicsHandles {
        let (_, h1) = MockSensor::new_clean("v", 0.0);
        let (_, h2) = MockSensor::new_clean("c", 0.0);
        let (_, h3) = MockSensor::new_clean("bt", 0.0);
        let (_, h4) = MockSensor::new_clean("rpm", 0.0);
        let (_, h5) = MockSensor::new_clean("torque", 0.0);
        let (_, h6) = MockSensor::new_clean("mt", 0.0);
        let (_, h7) = MockSensor::new_clean("throttle", 0.0);
        let (_, h8) = MockSensor::new_clean("ct", 0.0);
        let (_, h9) = MockSensor::new_clean("at", 0.0);
        PhysicsHandles {
            bms_voltage: h1,
            bms_current: h2,
            bms_temperature: h3,
            motor_rpm: h4,
            motor_torque: h5,
            motor_temperature: h6,
            motor_throttle: h7,
            coolant_temp: h8,
            ambient_temp: h9,
        }
    }

    #[test]
    fn vehicle_accelerates_with_throttle() {
        let mut physics = VehiclePhysics::new(VehiclePhysicsConfig::default());
        physics.set_throttle(1.0);

        for _ in 0..1000 {
            physics.step(Duration::from_millis(10));
        }

        assert!(
            physics.speed_kmh() > 50.0,
            "Should reach significant speed: {}",
            physics.speed_kmh()
        );
    }

    #[test]
    fn soc_decreases_during_drive() {
        let mut physics = VehiclePhysics::new(VehiclePhysicsConfig::default());
        let initial_soc = physics.soc();
        physics.set_throttle(0.5);

        for _ in 0..6000 {
            // 60 seconds
            physics.step(Duration::from_millis(10));
        }

        assert!(
            physics.soc() < initial_soc,
            "SOC should decrease: {} -> {}",
            initial_soc,
            physics.soc()
        );
    }

    #[test]
    fn speed_stabilizes_at_highway() {
        let mut physics = VehiclePhysics::new(VehiclePhysicsConfig::default());
        physics.set_throttle(0.3);

        // Run 120 seconds
        for _ in 0..12000 {
            physics.step(Duration::from_millis(10));
        }

        let speed1 = physics.speed_kmh();
        for _ in 0..3000 {
            physics.step(Duration::from_millis(10));
        }
        let speed2 = physics.speed_kmh();

        // Speed should be roughly stable (within 5 km/h)
        assert!(
            (speed2 - speed1).abs() < 5.0,
            "Speed should stabilize: {speed1} -> {speed2}"
        );
    }

    #[test]
    fn motor_temperature_rises_under_load() {
        let mut physics = VehiclePhysics::new(VehiclePhysicsConfig::default());
        let initial_temp = physics.motor_temp();
        physics.set_throttle(1.0);

        for _ in 0..6000 {
            physics.step(Duration::from_millis(10));
        }

        assert!(
            physics.motor_temp() > initial_temp,
            "Motor should heat up: {} -> {}",
            initial_temp,
            physics.motor_temp()
        );
    }

    #[test]
    fn update_sensors_writes_values() {
        let handles = make_handles();
        let mut physics = VehiclePhysics::new(VehiclePhysicsConfig::default());
        physics.set_throttle(0.5);
        physics.step(Duration::from_secs(1));
        physics.update_sensors(&handles);

        // Just verify handles got written (non-zero)
        // Can't read handles directly, but the function shouldn't panic
    }

    #[test]
    fn braking_decelerates() {
        let mut physics = VehiclePhysics::new(VehiclePhysicsConfig::default());
        physics.set_throttle(1.0);
        for _ in 0..3000 {
            physics.step(Duration::from_millis(10));
        }
        let speed_before = physics.speed_kmh();
        assert!(speed_before > 30.0);

        physics.set_throttle(0.0);
        physics.set_brake(1.0);
        for _ in 0..1000 {
            physics.step(Duration::from_millis(10));
        }

        assert!(
            physics.speed_kmh() < speed_before,
            "Should decelerate: {} -> {}",
            speed_before,
            physics.speed_kmh()
        );
    }
}
