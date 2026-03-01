use serde::{Deserialize, Serialize};
use std::fmt;

use crate::ComponentId;

/// Diagnostic Trouble Code severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DtcSeverity {
    Info,
    Warning,
    Fault,
    Critical,
}

impl fmt::Display for DtcSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARN"),
            Self::Fault => write!(f, "FAULT"),
            Self::Critical => write!(f, "CRIT"),
        }
    }
}

/// A Diagnostic Trouble Code following OBD-II-like naming.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Dtc {
    pub code: String,
    pub description: String,
    pub severity: DtcSeverity,
    pub source: ComponentId,
}

impl Dtc {
    pub fn new(
        code: impl Into<String>,
        description: impl Into<String>,
        severity: DtcSeverity,
        source: ComponentId,
    ) -> Self {
        Self {
            code: code.into(),
            description: description.into(),
            severity,
            source,
        }
    }
}

impl fmt::Display for Dtc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} - {} ({})",
            self.severity, self.code, self.description, self.source
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dtc_display() {
        let dtc = Dtc::new(
            "P0A80",
            "Battery pack over-temperature",
            DtcSeverity::Critical,
            ComponentId::Bms,
        );
        assert!(format!("{dtc}").contains("P0A80"));
        assert!(format!("{dtc}").contains("CRIT"));
    }
}
