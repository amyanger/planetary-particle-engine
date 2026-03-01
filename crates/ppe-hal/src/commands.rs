use ppe_core::Percent;

/// Commands for the motor controller actuator.
#[derive(Debug, Clone)]
pub enum MotorCommand {
    SetTorque(f64),
    Enable,
    Disable,
    EmergencyStop,
}

/// Commands for the cooling system actuator.
#[derive(Debug, Clone)]
pub enum CoolingCommand {
    SetFanSpeed(Percent),
    SetPumpSpeed(Percent),
    EnableCooling,
    DisableCooling,
}

/// Commands for the battery contactor actuator.
#[derive(Debug, Clone)]
pub enum ContactorCommand {
    Close,
    Open,
}
