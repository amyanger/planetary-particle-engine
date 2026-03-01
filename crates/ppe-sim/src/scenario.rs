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
    AccelSynchro,
    TurboDuel,
    ReactorStress,
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
            Self::AccelSynchro => write!(f, "accel-synchro"),
            Self::TurboDuel => write!(f, "turbo-duel"),
            Self::ReactorStress => write!(f, "reactor-stress"),
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
            Self::AccelSynchro,
            Self::TurboDuel,
            Self::ReactorStress,
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
            "accel-synchro" => Some(Self::AccelSynchro),
            "turbo-duel" => Some(Self::TurboDuel),
            "reactor-stress" => Some(Self::ReactorStress),
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
            ScenarioKind::AccelSynchro => vec![
                ScenarioStep {
                    time: Duration::from_secs(0),
                    throttle: 0.3,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(10),
                    throttle: 0.5,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(20),
                    throttle: 0.7,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(35),
                    throttle: 0.85,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(50),
                    throttle: 1.0,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(70),
                    throttle: 0.3,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(85),
                    throttle: 0.6,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(100),
                    throttle: 0.0,
                    brake: 0.4,
                },
                ScenarioStep {
                    time: Duration::from_secs(105),
                    throttle: 0.0,
                    brake: 0.0,
                },
            ],
            ScenarioKind::TurboDuel => vec![
                ScenarioStep {
                    time: Duration::from_secs(0),
                    throttle: 1.0,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(25),
                    throttle: 1.0,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(40),
                    throttle: 0.0,
                    brake: 0.3,
                },
                ScenarioStep {
                    time: Duration::from_secs(48),
                    throttle: 0.0,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(55),
                    throttle: 1.0,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(75),
                    throttle: 0.5,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(90),
                    throttle: 0.0,
                    brake: 0.5,
                },
                ScenarioStep {
                    time: Duration::from_secs(95),
                    throttle: 0.0,
                    brake: 0.0,
                },
            ],
            ScenarioKind::ReactorStress => vec![
                ScenarioStep {
                    time: Duration::from_secs(0),
                    throttle: 0.5,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(15),
                    throttle: 0.7,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(30),
                    throttle: 0.85,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(90),
                    throttle: 1.0,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(120),
                    throttle: 0.5,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(150),
                    throttle: 0.7,
                    brake: 0.0,
                },
                ScenarioStep {
                    time: Duration::from_secs(180),
                    throttle: 0.0,
                    brake: 0.3,
                },
                ScenarioStep {
                    time: Duration::from_secs(195),
                    throttle: 0.0,
                    brake: 0.5,
                },
                ScenarioStep {
                    time: Duration::from_secs(210),
                    throttle: 0.0,
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

    #[test]
    fn accel_synchro_display_and_parse() {
        assert_eq!(ScenarioKind::AccelSynchro.to_string(), "accel-synchro");
        assert_eq!(
            ScenarioKind::parse("accel-synchro"),
            Some(ScenarioKind::AccelSynchro)
        );
    }

    #[test]
    fn turbo_duel_display_and_parse() {
        assert_eq!(ScenarioKind::TurboDuel.to_string(), "turbo-duel");
        assert_eq!(
            ScenarioKind::parse("turbo-duel"),
            Some(ScenarioKind::TurboDuel)
        );
    }

    #[test]
    fn reactor_stress_display_and_parse() {
        assert_eq!(ScenarioKind::ReactorStress.to_string(), "reactor-stress");
        assert_eq!(
            ScenarioKind::parse("reactor-stress"),
            Some(ScenarioKind::ReactorStress)
        );
    }

    #[test]
    fn accel_synchro_steps() {
        let mut scenario = Scenario::new(ScenarioKind::AccelSynchro);
        assert_eq!(scenario.steps.len(), 9);

        // Starts with gentle throttle
        let (t, b) = scenario.update(Duration::from_secs(0));
        assert_eq!(t, 0.3);
        assert_eq!(b, 0.0);

        // Full throttle overdrive push at 50s
        let (t, b) = scenario.update(Duration::from_secs(52));
        assert_eq!(t, 1.0);
        assert_eq!(b, 0.0);

        // Decel at 100s
        let (t, b) = scenario.update(Duration::from_secs(102));
        assert_eq!(t, 0.0);
        assert_eq!(b, 0.4);

        // Coast at 105s
        let (t, b) = scenario.update(Duration::from_secs(106));
        assert_eq!(t, 0.0);
        assert_eq!(b, 0.0);
    }

    #[test]
    fn turbo_duel_steps() {
        let mut scenario = Scenario::new(ScenarioKind::TurboDuel);
        assert_eq!(scenario.steps.len(), 8);

        // Launch at full throttle
        let (t, b) = scenario.update(Duration::from_secs(0));
        assert_eq!(t, 1.0);
        assert_eq!(b, 0.0);

        // Emergency cut at 40s
        let (t, b) = scenario.update(Duration::from_secs(42));
        assert_eq!(t, 0.0);
        assert_eq!(b, 0.3);

        // Second push at 55s
        let (t, b) = scenario.update(Duration::from_secs(58));
        assert_eq!(t, 1.0);
        assert_eq!(b, 0.0);

        // Slow down at 90s
        let (t, b) = scenario.update(Duration::from_secs(92));
        assert_eq!(t, 0.0);
        assert_eq!(b, 0.5);
    }

    #[test]
    fn reactor_stress_steps() {
        let mut scenario = Scenario::new(ScenarioKind::ReactorStress);
        assert_eq!(scenario.steps.len(), 9);

        // Warmup ramp
        let (t, b) = scenario.update(Duration::from_secs(0));
        assert_eq!(t, 0.5);
        assert_eq!(b, 0.0);

        // Sustained overdrive at 30s
        let (t, b) = scenario.update(Duration::from_secs(45));
        assert_eq!(t, 0.85);
        assert_eq!(b, 0.0);

        // Critical push at 90s
        let (t, b) = scenario.update(Duration::from_secs(100));
        assert_eq!(t, 1.0);
        assert_eq!(b, 0.0);

        // Full brake at 195s
        let (t, b) = scenario.update(Duration::from_secs(200));
        assert_eq!(t, 0.0);
        assert_eq!(b, 0.5);

        // Coast to stop at 210s
        let (t, b) = scenario.update(Duration::from_secs(215));
        assert_eq!(t, 0.0);
        assert_eq!(b, 0.0);
    }

    #[test]
    fn all_includes_new_scenarios() {
        let all = ScenarioKind::all();
        assert!(all.contains(&ScenarioKind::AccelSynchro));
        assert!(all.contains(&ScenarioKind::TurboDuel));
        assert!(all.contains(&ScenarioKind::ReactorStress));
        assert_eq!(all.len(), 10);
    }

    #[test]
    fn parse_rejects_unknown() {
        assert_eq!(ScenarioKind::parse("nonexistent"), None);
    }

    #[test]
    fn all_scenarios_roundtrip_display_parse() {
        for kind in ScenarioKind::all() {
            let display = kind.to_string();
            let parsed = ScenarioKind::parse(&display);
            assert_eq!(parsed, Some(*kind), "roundtrip failed for {display}");
        }
    }
}
