use std::time::Duration;

/// Types of pre-built scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioKind {
    Idle,
    CityDrive,
    HighwayCruise,
    FullThrottle,
    ThermalStress,
    RangeTest,
    FaultInjection,
}

impl std::fmt::Display for ScenarioKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "idle"),
            Self::CityDrive => write!(f, "city-drive"),
            Self::HighwayCruise => write!(f, "highway-cruise"),
            Self::FullThrottle => write!(f, "full-throttle"),
            Self::ThermalStress => write!(f, "thermal-stress"),
            Self::RangeTest => write!(f, "range-test"),
            Self::FaultInjection => write!(f, "fault-injection"),
        }
    }
}

impl ScenarioKind {
    pub fn all() -> &'static [ScenarioKind] {
        &[
            Self::Idle,
            Self::CityDrive,
            Self::HighwayCruise,
            Self::FullThrottle,
            Self::ThermalStress,
            Self::RangeTest,
            Self::FaultInjection,
        ]
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "idle" => Some(Self::Idle),
            "city-drive" => Some(Self::CityDrive),
            "highway-cruise" => Some(Self::HighwayCruise),
            "full-throttle" => Some(Self::FullThrottle),
            "thermal-stress" => Some(Self::ThermalStress),
            "range-test" => Some(Self::RangeTest),
            "fault-injection" => Some(Self::FaultInjection),
            _ => None,
        }
    }
}

/// A single step in a scenario (throttle/brake at a given time).
#[derive(Debug, Clone)]
pub struct ScenarioStep {
    pub time: Duration,
    pub throttle: f64,
    pub brake: f64,
}

/// A scripted scenario with timed throttle/brake commands.
pub struct Scenario {
    pub kind: ScenarioKind,
    pub steps: Vec<ScenarioStep>,
    current_index: usize,
    pub current_throttle: f64,
    pub current_brake: f64,
}

impl Scenario {
    pub fn new(kind: ScenarioKind) -> Self {
        let steps = match kind {
            ScenarioKind::Idle => vec![ScenarioStep {
                time: Duration::ZERO,
                throttle: 0.0,
                brake: 0.0,
            }],
            ScenarioKind::CityDrive => vec![
                ScenarioStep {
                    time: Duration::from_secs(0),
                    throttle: 0.4,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(10),
                    throttle: 0.0,
                    brake: 0.3,
                },
                ScenarioStep {
                    time: Duration::from_secs(15),
                    throttle: 0.0,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(20),
                    throttle: 0.5,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(30),
                    throttle: 0.0,
                    brake: 0.5,
                },
                ScenarioStep {
                    time: Duration::from_secs(35),
                    throttle: 0.0,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(40),
                    throttle: 0.3,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(50),
                    throttle: 0.0,
                    brake: 0.4,
                },
                ScenarioStep {
                    time: Duration::from_secs(55),
                    throttle: 0.0,
                    brake: 0.0,
                },
            ],
            ScenarioKind::HighwayCruise => vec![
                ScenarioStep {
                    time: Duration::from_secs(0),
                    throttle: 0.8,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(20),
                    throttle: 0.3,
                    brake: 0.0,
                },
            ],
            ScenarioKind::FullThrottle => vec![ScenarioStep {
                time: Duration::ZERO,
                throttle: 1.0,
                brake: 0.0,
            }],
            ScenarioKind::ThermalStress => vec![
                ScenarioStep {
                    time: Duration::from_secs(0),
                    throttle: 1.0,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(30),
                    throttle: 0.0,
                    brake: 0.8,
                },
                ScenarioStep {
                    time: Duration::from_secs(35),
                    throttle: 1.0,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(65),
                    throttle: 0.0,
                    brake: 0.8,
                },
                ScenarioStep {
                    time: Duration::from_secs(70),
                    throttle: 1.0,
                    brake: 0.0,
                },
            ],
            ScenarioKind::RangeTest => vec![ScenarioStep {
                time: Duration::from_secs(0),
                throttle: 0.25,
                brake: 0.0,
            }],
            ScenarioKind::FaultInjection => vec![
                ScenarioStep {
                    time: Duration::from_secs(0),
                    throttle: 0.5,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(10),
                    throttle: 0.8,
                    brake: 0.0,
                },
            ],
        };

        let current_throttle = steps.first().map(|s| s.throttle).unwrap_or(0.0);
        let current_brake = steps.first().map(|s| s.brake).unwrap_or(0.0);

        Self {
            kind,
            steps,
            current_index: 0,
            current_throttle,
            current_brake,
        }
    }

    /// Update the scenario based on elapsed time. Returns (throttle, brake).
    pub fn update(&mut self, elapsed: Duration) -> (f64, f64) {
        // Advance to latest applicable step
        while self.current_index + 1 < self.steps.len()
            && elapsed >= self.steps[self.current_index + 1].time
        {
            self.current_index += 1;
            self.current_throttle = self.steps[self.current_index].throttle;
            self.current_brake = self.steps[self.current_index].brake;
        }

        (self.current_throttle, self.current_brake)
    }

    pub fn is_looping(&self) -> bool {
        // Scenarios repeat after their last step
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_city_drive_steps() {
        let mut scenario = Scenario::new(ScenarioKind::CityDrive);

        let (t, b) = scenario.update(Duration::from_secs(0));
        assert_eq!(t, 0.4);
        assert_eq!(b, 0.0);

        let (t, b) = scenario.update(Duration::from_secs(12));
        assert_eq!(t, 0.0);
        assert_eq!(b, 0.3);
    }

    #[test]
    fn scenario_kind_display() {
        assert_eq!(ScenarioKind::HighwayCruise.to_string(), "highway-cruise");
        assert_eq!(
            ScenarioKind::parse("highway-cruise"),
            Some(ScenarioKind::HighwayCruise)
        );
    }
}
