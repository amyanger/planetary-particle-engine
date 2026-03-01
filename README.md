# Planetary Particle Engine (PPE)

A simulation-first vehicle operating system written in Rust.

PPE models a complete electric vehicle software stack — from CAN bus communication and hardware abstraction up through physics simulation and a real-time TUI dashboard — all without requiring physical hardware.

![PPE Dashboard](assets/dashboard.png)

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

## Crates

| Crate | Description |
|-------|-------------|
| `ppe-core` | Unit newtypes (Voltage, Current, etc.), SimClock, error types, DTCs |
| `ppe-can` | CAN frames, virtual CAN bus (crossbeam-based), bus nodes, filtering |
| `ppe-hal` | Sensor/Actuator traits, mock sensors with noise model |
| `ppe-state` | Vehicle FSM, sub-FSMs (BMS, Motor) |
| `ppe-scheduler` | EDF-like scheduler, software watchdog |
| `ppe-subsystems` | BMS, Motor Controller, Thermal Management |
| `ppe-diagnostics` | OBD-II responder (Mode 01/03), DTC manager, freeze frames |
| `ppe-sim` | Vehicle physics (dynamics, electrical, thermal), scenarios |
| `ppe-dashboard` | ratatui TUI with gauges, CAN monitor, DTC viewer |

## Getting Started

```bash
# Build
cargo build --workspace

# Run tests
cargo test --workspace

# Launch the dashboard
cargo run --bin ppe-daemon

# Headless mode (no TUI)
cargo run --bin ppe-daemon -- --headless

# Diagnostic CLI
cargo run --bin ppe-diag -- list-pids
cargo run --bin ppe-diag -- sniff --duration 5
```

## Dashboard Controls

| Key | Action |
|-----|--------|
| `q` | Quit |
| `p` | Pause / Resume |
| `s` | Cycle scenarios |
| `+` / `-` | Adjust throttle |
| `f` | Inject fault |
| `d` | Clear DTCs |

## License

MIT
