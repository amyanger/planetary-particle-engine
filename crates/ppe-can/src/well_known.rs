use crate::CanId;

// BMS CAN IDs (0x100-0x10F)
pub const BMS_SOC: CanId = CanId::new_unchecked(0x100);
pub const BMS_VOLTAGE: CanId = CanId::new_unchecked(0x101);
pub const BMS_CURRENT: CanId = CanId::new_unchecked(0x102);
pub const BMS_TEMPERATURE: CanId = CanId::new_unchecked(0x103);
pub const BMS_STATUS: CanId = CanId::new_unchecked(0x104);

// Motor CAN IDs (0x200-0x20F)
pub const MOTOR_RPM: CanId = CanId::new_unchecked(0x200);
pub const MOTOR_TORQUE: CanId = CanId::new_unchecked(0x201);
pub const MOTOR_TEMPERATURE: CanId = CanId::new_unchecked(0x202);
pub const MOTOR_STATUS: CanId = CanId::new_unchecked(0x203);

// Thermal CAN IDs (0x300-0x30F)
pub const THERMAL_COOLANT_TEMP: CanId = CanId::new_unchecked(0x300);
pub const THERMAL_FAN_SPEED: CanId = CanId::new_unchecked(0x301);
pub const THERMAL_STATUS: CanId = CanId::new_unchecked(0x302);

// Vehicle state CAN IDs (0x400-0x40F)
pub const VEHICLE_STATE: CanId = CanId::new_unchecked(0x400);
pub const VEHICLE_SPEED: CanId = CanId::new_unchecked(0x401);
pub const VEHICLE_THROTTLE: CanId = CanId::new_unchecked(0x402);
pub const VEHICLE_GEAR: CanId = CanId::new_unchecked(0x403);

// Emergency / high-priority (low IDs)
pub const EMERGENCY_STOP: CanId = CanId::new_unchecked(0x001);
pub const HEARTBEAT: CanId = CanId::new_unchecked(0x002);

// Ener-D Reactor CAN IDs (0x500-0x50F)
pub const ENERD_STATUS: CanId = CanId::new_unchecked(0x500);
pub const ENERD_SPIN_RATE: CanId = CanId::new_unchecked(0x501);
pub const ENERD_POWER_OUTPUT: CanId = CanId::new_unchecked(0x502);
pub const ENERD_CONTAINMENT: CanId = CanId::new_unchecked(0x503);
pub const ENERD_PLASMA_TEMP: CanId = CanId::new_unchecked(0x504);
pub const ENERD_MOMENTUM_FLUX: CanId = CanId::new_unchecked(0x505);

// OBD-II CAN IDs
pub const OBD_REQUEST: CanId = CanId::new_unchecked(0x7DF);
pub const OBD_RESPONSE: CanId = CanId::new_unchecked(0x7E8);
