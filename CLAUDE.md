# Planetary Particle Engine (PPE)

A simulation-first vehicle operating system in Rust.

## Build Commands

```bash
cargo build --workspace          # Build all crates
cargo test --workspace           # Run all tests
cargo clippy --workspace -- -D warnings  # Lint (must pass clean)
cargo fmt --check                # Check formatting
cargo run --bin ppe-daemon       # Run the main daemon
cargo run --bin ppe-daemon -- --headless  # Run without TUI
cargo run --bin ppe-daemon -- --scenario reactor-stress  # Run specific scenario
cargo run --bin ppe-diag         # Run the diagnostic CLI
cargo run --bin ppe-diag -- list-pids    # List supported OBD PIDs
cargo run --bin ppe-diag -- sniff --duration 5  # Sniff CAN traffic
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

- **ppe-core**: Unit newtypes (Voltage, Current, SpinRate, MomentumFlux, Containment, PlasmaTemp, etc.), SimClock (AtomicU64-based), PpeError, Dtc, ComponentId
- **ppe-can**: CanFrame, CanId (11-bit), VirtualCanBus (crossbeam-based router thread), BusNode, CanFilter (AcceptAll/Exact/Range/Any), well-known CAN IDs (BMS 0x100, Motor 0x200, Thermal 0x300, Vehicle 0x400, Ener-D 0x500, OBD 0x7DF/0x7E8)
- **ppe-hal**: Sensor/Actuator traits (Send+Sync), MockSensor with NoiseModel (gaussian + drift + spike), SensorHandle (lock-free AtomicU64)
- **ppe-state**: VehicleEvent (22 variants), VehicleState FSM (7 states), Gear, sub-FSMs (BmsState, MotorState, EnerDState)
- **ppe-scheduler**: ScheduledTask, EDF-like async scheduler loop, software Watchdog
- **ppe-subsystems**: Subsystem trait (init/tick/shutdown/health/active_dtcs), BMS (coulomb counting SOC), MotorController (power derate), ThermalManagement (fan control), EnerDReactor (momentum-based energy source with containment field + plasma physics)
- **ppe-diagnostics**: ObdResponder (Mode 01/03 on CAN), DtcManager, FreezeFrame, ObdLiveData
- **ppe-sim**: VehiclePhysics (dynamics, electrical, thermal, reactor coupling), 10 Scenarios (Idle, CityDrive, HighwayCruise, FullThrottle, ThermalStress, RangeTest, FaultInjection, AccelSynchro, TurboDuel, ReactorStress)
- **ppe-dashboard**: ratatui TUI with BMS/Motor/Thermal/Ener-D panels, CAN monitor, DTC viewer, scenario cycling

## Ener-D Reactor

Momentum-based energy source subsystem with 6-state FSM: Dormant → SpinUp → Sustaining → Overdrive → Critical → Meltdown (terminal). Physics model includes spin rate dynamics, containment field (degrades cubically at high power), and plasma temperature. CAN IDs 0x500-0x505. Auto-enabled for reactor-specific scenarios (AccelSynchro, TurboDuel, ReactorStress).

## Code Conventions

- `thiserror` in library crates, `anyhow` in binaries
- No `.unwrap()` in library code; use `?` or explicit error handling
- `tracing` for all logging (not `println!`)
- `crossbeam-channel` for CAN bus message passing
- `tokio` only at top-level (scheduler, dashboard, binaries)
- `heapless::Vec<u8, 8>` for CAN frame data
- Unit newtypes wrap f64 and implement Display
- Subsystems implement the `Subsystem` trait (init/tick/shutdown/health/active_dtcs)
- Lock-free sensor updates via `SensorHandle` (AtomicU64 bit-reinterpreted as f64)

## Testing Conventions

- Unit tests in-file under `#[cfg(test)]`
- Integration tests in `tests/integration/` (CAN bus, BMS, scheduler, Ener-D reactor)
- `proptest` for invariant testing (CAN arbitration, SOC bounds, state transitions)

## Git Conventions

- No co-author lines in commits
- Concise commit messages focused on "why"
