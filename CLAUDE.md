# Planetary Particle Engine (PPE)

A simulation-first vehicle operating system in Rust.

## Build Commands

```bash
cargo build --workspace          # Build all crates
cargo test --workspace           # Run all tests
cargo clippy --workspace -- -D warnings  # Lint (must pass clean)
cargo fmt --check                # Check formatting
cargo run --bin ppe-daemon       # Run the main daemon
cargo run --bin ppe-diag         # Run the diagnostic CLI
```

## Architecture

```
Layer 7: TUI Dashboard           <- ppe-dashboard
Layer 6: Diagnostics (OBD-II)    <- ppe-diagnostics
Layer 5: Physics Simulation      <- ppe-sim
Layer 4: RT Scheduler            <- ppe-scheduler
Layer 3: Subsystems              <- ppe-subsystems
Layer 2: State Machine           <- ppe-state
Layer 1: CAN Bus + HAL           <- ppe-can, ppe-hal
Layer 0: Core Types              <- ppe-core
```

## Crate Responsibilities

- **ppe-core**: Unit newtypes (Voltage, Current, etc.), SimClock, PpeError, Dtc, ComponentId
- **ppe-can**: CanFrame, CanId, VirtualCanBus (crossbeam-based), BusNode, CanFilter
- **ppe-hal**: Sensor/Actuator traits, MockSensor with noise model, SensorHandle
- **ppe-state**: VehicleEvent, VehicleState FSM, sub-FSMs (BmsState, MotorState)
- **ppe-scheduler**: ScheduledTask, EDF-like scheduler loop, software Watchdog
- **ppe-subsystems**: Subsystem trait, BMS, MotorController, ThermalManagement
- **ppe-diagnostics**: ObdResponder (Mode 01/03), DtcManager, FreezeFrame
- **ppe-sim**: VehiclePhysics (dynamics, electrical, thermal), Scenarios
- **ppe-dashboard**: ratatui TUI with gauges, CAN monitor, DTC view

## Code Conventions

- `thiserror` in library crates, `anyhow` in binaries
- No `.unwrap()` in library code; use `?` or explicit error handling
- `tracing` for all logging (not `println!`)
- `crossbeam-channel` for CAN bus message passing
- `tokio` only at top-level (scheduler, dashboard, binaries)
- `heapless::Vec<u8, 8>` for CAN frame data
- Unit newtypes wrap f64 and implement Display

## Testing Conventions

- Unit tests in-file under `#[cfg(test)]`
- Integration tests in `tests/integration/`
- `proptest` for invariant testing (CAN arbitration, SOC bounds, state transitions)

## Git Conventions

- No co-author lines in commits
- Concise commit messages focused on "why"
