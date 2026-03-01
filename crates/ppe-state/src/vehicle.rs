use serde::{Deserialize, Serialize};
use std::fmt;
use tracing::info;

use crate::{Gear, VehicleEvent};

/// Top-level vehicle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VehicleState {
    Off,
    Accessory,
    Ready,
    Driving,
    Charging,
    Fault,
    SafeState,
}

impl fmt::Display for VehicleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Off => write!(f, "OFF"),
            Self::Accessory => write!(f, "ACCESSORY"),
            Self::Ready => write!(f, "READY"),
            Self::Driving => write!(f, "DRIVING"),
            Self::Charging => write!(f, "CHARGING"),
            Self::Fault => write!(f, "FAULT"),
            Self::SafeState => write!(f, "SAFE_STATE"),
        }
    }
}

/// Vehicle finite state machine.
pub struct VehicleFsm {
    state: VehicleState,
    gear: Gear,
}

impl VehicleFsm {
    pub fn new() -> Self {
        Self {
            state: VehicleState::Off,
            gear: Gear::Park,
        }
    }

    pub fn state(&self) -> VehicleState {
        self.state
    }

    pub fn gear(&self) -> Gear {
        self.gear
    }

    /// Process an event and return the new state.
    pub fn on_event(&mut self, event: &VehicleEvent) -> VehicleState {
        let old = self.state;
        self.state = match (self.state, event) {
            // Off transitions
            (VehicleState::Off, VehicleEvent::KeyToAccessory) => VehicleState::Accessory,

            // Accessory transitions
            (VehicleState::Accessory, VehicleEvent::KeyToStart) => VehicleState::Ready,
            (VehicleState::Accessory, VehicleEvent::KeyOff) => VehicleState::Off,
            (VehicleState::Accessory, VehicleEvent::ChargerConnected) => VehicleState::Charging,

            // Ready transitions
            (VehicleState::Ready, VehicleEvent::ThrottleApplied(_))
                if self.gear == Gear::Drive || self.gear == Gear::Reverse =>
            {
                VehicleState::Driving
            }
            (VehicleState::Ready, VehicleEvent::GearShift(gear)) => {
                self.gear = *gear;
                VehicleState::Ready
            }
            (VehicleState::Ready, VehicleEvent::KeyOff) => VehicleState::Off,
            (VehicleState::Ready, VehicleEvent::ChargerConnected) => VehicleState::Charging,

            // Driving transitions
            (VehicleState::Driving, VehicleEvent::ThrottleReleased)
                if self.gear == Gear::Park || self.gear == Gear::Neutral =>
            {
                VehicleState::Ready
            }
            (VehicleState::Driving, VehicleEvent::GearShift(Gear::Park)) => {
                self.gear = Gear::Park;
                VehicleState::Ready
            }
            (VehicleState::Driving, VehicleEvent::GearShift(gear)) => {
                self.gear = *gear;
                VehicleState::Driving
            }

            // Charging transitions
            (VehicleState::Charging, VehicleEvent::ChargerDisconnected) => VehicleState::Accessory,
            (VehicleState::Charging, VehicleEvent::ChargingComplete) => VehicleState::Accessory,

            // Fault from any state
            (_, VehicleEvent::FaultDetected(_)) => VehicleState::Fault,
            (_, VehicleEvent::EmergencyStop) => VehicleState::SafeState,

            // Fault recovery
            (VehicleState::Fault, VehicleEvent::FaultCleared) => VehicleState::Accessory,

            // SafeState recovery
            (VehicleState::SafeState, VehicleEvent::EmergencyReset) => VehicleState::Off,

            // No valid transition
            _ => self.state,
        };

        if old != self.state {
            info!(from = %old, to = %self.state, event = ?event, "vehicle state transition");
        }

        self.state
    }
}

impl Default for VehicleFsm {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_sequence() {
        let mut fsm = VehicleFsm::new();
        assert_eq!(fsm.state(), VehicleState::Off);

        fsm.on_event(&VehicleEvent::KeyToAccessory);
        assert_eq!(fsm.state(), VehicleState::Accessory);

        fsm.on_event(&VehicleEvent::KeyToStart);
        assert_eq!(fsm.state(), VehicleState::Ready);
    }

    #[test]
    fn driving_requires_gear_in_drive() {
        let mut fsm = VehicleFsm::new();
        fsm.on_event(&VehicleEvent::KeyToAccessory);
        fsm.on_event(&VehicleEvent::KeyToStart);

        // Throttle in Park should not transition to Driving
        fsm.on_event(&VehicleEvent::ThrottleApplied(0.5));
        assert_eq!(fsm.state(), VehicleState::Ready);

        // Shift to Drive, then throttle
        fsm.on_event(&VehicleEvent::GearShift(Gear::Drive));
        fsm.on_event(&VehicleEvent::ThrottleApplied(0.5));
        assert_eq!(fsm.state(), VehicleState::Driving);
    }

    #[test]
    fn emergency_stop_from_any_state() {
        let mut fsm = VehicleFsm::new();
        fsm.on_event(&VehicleEvent::KeyToAccessory);
        fsm.on_event(&VehicleEvent::KeyToStart);
        fsm.on_event(&VehicleEvent::GearShift(Gear::Drive));
        fsm.on_event(&VehicleEvent::ThrottleApplied(1.0));
        assert_eq!(fsm.state(), VehicleState::Driving);

        fsm.on_event(&VehicleEvent::EmergencyStop);
        assert_eq!(fsm.state(), VehicleState::SafeState);

        fsm.on_event(&VehicleEvent::EmergencyReset);
        assert_eq!(fsm.state(), VehicleState::Off);
    }

    #[test]
    fn fault_and_recovery() {
        let mut fsm = VehicleFsm::new();
        fsm.on_event(&VehicleEvent::KeyToAccessory);
        fsm.on_event(&VehicleEvent::KeyToStart);

        fsm.on_event(&VehicleEvent::FaultDetected("overtemp".into()));
        assert_eq!(fsm.state(), VehicleState::Fault);

        fsm.on_event(&VehicleEvent::FaultCleared);
        assert_eq!(fsm.state(), VehicleState::Accessory);
    }

    #[test]
    fn charging_flow() {
        let mut fsm = VehicleFsm::new();
        fsm.on_event(&VehicleEvent::KeyToAccessory);
        fsm.on_event(&VehicleEvent::ChargerConnected);
        assert_eq!(fsm.state(), VehicleState::Charging);

        fsm.on_event(&VehicleEvent::ChargingComplete);
        assert_eq!(fsm.state(), VehicleState::Accessory);
    }
}
