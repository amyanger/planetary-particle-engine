use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! unit_newtype {
    ($name:ident, $unit:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
        pub struct $name(pub f64);

        impl $name {
            pub fn new(value: f64) -> Self {
                Self(value)
            }

            pub fn value(self) -> f64 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{:.2} {}", self.0, $unit)
            }
        }

        impl From<f64> for $name {
            fn from(v: f64) -> Self {
                Self(v)
            }
        }

        impl std::ops::Add for $name {
            type Output = Self;
            fn add(self, rhs: Self) -> Self {
                Self(self.0 + rhs.0)
            }
        }

        impl std::ops::Sub for $name {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self {
                Self(self.0 - rhs.0)
            }
        }

        impl std::ops::Mul<f64> for $name {
            type Output = Self;
            fn mul(self, rhs: f64) -> Self {
                Self(self.0 * rhs)
            }
        }

        impl std::ops::Div<f64> for $name {
            type Output = Self;
            fn div(self, rhs: f64) -> Self {
                Self(self.0 / rhs)
            }
        }
    };
}

unit_newtype!(Voltage, "V");
unit_newtype!(Current, "A");
unit_newtype!(Temperature, "°C");
unit_newtype!(Speed, "km/h");
unit_newtype!(Rpm, "RPM");
unit_newtype!(Percent, "%");
unit_newtype!(Power, "kW");
unit_newtype!(Torque, "Nm");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voltage_display() {
        let v = Voltage::new(3.7);
        assert_eq!(format!("{v}"), "3.70 V");
    }

    #[test]
    fn unit_arithmetic() {
        let a = Voltage::new(3.7);
        let b = Voltage::new(0.3);
        assert!(((a + b).value() - 4.0).abs() < 1e-10);
        assert!(((a - b).value() - 3.4).abs() < 1e-10);
        assert!(((a * 2.0).value() - 7.4).abs() < 1e-10);
        assert!(((a / 2.0).value() - 1.85).abs() < 1e-10);
    }

    #[test]
    fn percent_default() {
        let p = Percent::default();
        assert_eq!(p.value(), 0.0);
    }
}
