use std::time::Duration;

use ppe_can::{well_known, CanFilter, CanFrame, VirtualCanBus};
use ppe_state::EnerDState;
use ppe_subsystems::{EnerDConfig, EnerDHandles, EnerDReactor, Subsystem};

const DT: Duration = Duration::from_millis(10);

fn make_reactor() -> (EnerDReactor, EnerDHandles) {
    let bus = VirtualCanBus::new(256);
    let reactor_node = bus.connect(
        CanFilter::Any(vec![CanFilter::Exact(well_known::EMERGENCY_STOP)]),
        64,
    );
    let config = EnerDConfig::default();
    let (reactor, handles) = EnerDReactor::new(config, reactor_node);
    (reactor, handles)
}

fn setup_vehicle_sensors(handles: &EnerDHandles, speed: f64, accel: f64, drag: f64, mass: f64) {
    handles.vehicle_speed.set(speed);
    handles.vehicle_accel.set(accel);
    handles.vehicle_drag_force.set(drag);
    handles.vehicle_mass.set(mass);
}

/// Tick the reactor until it reaches the target state or max_ticks is exceeded.
/// Returns true if the target state was reached.
fn tick_until(
    reactor: &mut EnerDReactor,
    target: EnerDState,
    dt: Duration,
    max_ticks: usize,
) -> bool {
    for _ in 0..max_ticks {
        reactor.tick(dt).unwrap();
        if reactor.state() == target {
            return true;
        }
    }
    false
}

#[test]
fn spinup_to_sustaining_cycle() {
    let (mut reactor, handles) = make_reactor();
    reactor.init().unwrap();
    reactor.set_enabled(true);

    setup_vehicle_sensors(&handles, 25.0, 3.0, 800.0, 1800.0);

    // Should enter SpinUp within a couple of ticks
    assert!(
        tick_until(&mut reactor, EnerDState::SpinUp, DT, 2),
        "Reactor should enter SpinUp quickly when speed > min_activation_speed"
    );

    // Spin rate should still be low at the start of SpinUp
    assert!(
        reactor.spin_rate() < 50.0,
        "Spin rate should still be low right after entering SpinUp: got {:.1}",
        reactor.spin_rate()
    );

    // Should reach Sustaining within ~20s (2000 ticks at 10ms)
    assert!(
        tick_until(&mut reactor, EnerDState::Sustaining, DT, 2000),
        "Reactor should reach Sustaining within 20s, spin_rate={:.1}, net_power={:.1}",
        reactor.spin_rate(),
        reactor.net_power_kw()
    );

    assert!(
        reactor.spin_rate() >= 150.0,
        "Spin rate must be >= sustain threshold (150) in Sustaining: got {:.1}",
        reactor.spin_rate()
    );
    assert!(
        reactor.net_power_kw() > 0.0,
        "Net power must be positive in Sustaining: got {:.1}",
        reactor.net_power_kw()
    );
}

#[test]
fn overdrive_entry_and_exit() {
    let (mut reactor, handles) = make_reactor();
    reactor.init().unwrap();
    reactor.set_enabled(true);

    // Use moderate inputs for spinup to preserve containment.
    // flux = 0.2 * (800 + 1800*3) = 0.2 * 6200 = 1240
    // Equilibrium spin = 1240 / 0.5 = 2480, clamped to 800.
    // This will reach Sustaining quickly.
    setup_vehicle_sensors(&handles, 25.0, 3.0, 800.0, 1800.0);

    assert!(
        tick_until(&mut reactor, EnerDState::Sustaining, DT, 3000),
        "Should reach Sustaining, state={}, spin={:.1}",
        reactor.state(),
        reactor.spin_rate()
    );

    // Gradually increase inputs to push spin above 400 (overdrive threshold).
    // Use inputs that produce high flux but won't destroy containment too fast.
    // flux = 0.2 * (3000 + 1800*8) = 0.2 * 17400 = 3480
    // Tick manually and break as soon as Overdrive is entered.
    setup_vehicle_sensors(&handles, 30.0, 8.0, 3000.0, 1800.0);

    let mut entered_overdrive = false;
    for _ in 0..3000 {
        reactor.tick(DT).unwrap();
        if reactor.state() == EnerDState::Overdrive {
            entered_overdrive = true;
            break;
        }
    }
    assert!(
        entered_overdrive,
        "Should reach Overdrive, state={}, spin={:.1}, containment={:.1}",
        reactor.state(),
        reactor.spin_rate(),
        reactor.containment()
    );

    // Immediately reduce inputs to near-zero so spin decays quickly.
    // This prevents containment from degrading further while in Overdrive.
    // Overdrive -> Sustaining transition: spin_rate <= overdrive_spin_threshold (400).
    setup_vehicle_sensors(&handles, 10.0, 0.0, 0.0, 1800.0);

    // Tick manually checking for Critical -> if we enter Critical, that's okay as long
    // as we eventually get to Sustaining (via recovery). But to avoid Meltdown, the
    // reduced inputs should let the reactor stabilize.
    assert!(
        tick_until(&mut reactor, EnerDState::Sustaining, DT, 15_000),
        "Should return to Sustaining after reducing inputs, state={}, spin={:.1}, containment={:.1}",
        reactor.state(),
        reactor.spin_rate(),
        reactor.containment()
    );

    assert_eq!(
        reactor.state(),
        EnerDState::Sustaining,
        "Final state should be Sustaining"
    );
}

#[test]
fn critical_state_and_recovery() {
    let (mut reactor, handles) = make_reactor();
    reactor.init().unwrap();
    reactor.set_enabled(true);

    // High inputs to push through SpinUp -> Sustaining -> Overdrive -> Critical
    setup_vehicle_sensors(&handles, 40.0, 20.0, 8000.0, 1800.0);

    assert!(
        tick_until(&mut reactor, EnerDState::Critical, DT, 10_000),
        "Should reach Critical, state={}, containment={:.1}, plasma={:.1}",
        reactor.state(),
        reactor.containment(),
        reactor.plasma_temp()
    );

    // Immediately reduce inputs to very low values.
    // Recovery requires: containment > 70 AND plasma_temp < 60 for 3 consecutive seconds.
    // We must recover before state_time > 15s (which triggers Meltdown).
    // Very low inputs let spin decay, reducing power output, allowing containment regen
    // and plasma cooling.
    setup_vehicle_sensors(&handles, 6.0, 0.0, 0.0, 1800.0);

    // Tick and check if we recover to Sustaining within the 15s meltdown window.
    // 15s = 1500 ticks, plus extra for the 3s debounce.
    let reached_sustaining = tick_until(&mut reactor, EnerDState::Sustaining, DT, 5000);

    assert!(
        reached_sustaining,
        "Should recover to Sustaining from Critical, state={}, containment={:.1}, plasma={:.1}",
        reactor.state(),
        reactor.containment(),
        reactor.plasma_temp()
    );
}

#[test]
fn meltdown_is_terminal() {
    // Use easier-to-hit meltdown thresholds
    let config = EnerDConfig {
        meltdown_containment: 40.0,
        critical_containment: 60.0,
        ..EnerDConfig::default()
    };

    let bus = VirtualCanBus::new(256);
    let reactor_node = bus.connect(
        CanFilter::Any(vec![CanFilter::Exact(well_known::EMERGENCY_STOP)]),
        64,
    );
    let (mut reactor, handles) = EnerDReactor::new(config, reactor_node);
    reactor.init().unwrap();
    reactor.set_enabled(true);

    // Very aggressive inputs
    setup_vehicle_sensors(&handles, 40.0, 20.0, 8000.0, 1800.0);

    assert!(
        tick_until(&mut reactor, EnerDState::Meltdown, DT, 20_000),
        "Should reach Meltdown, state={}, containment={:.1}",
        reactor.state(),
        reactor.containment()
    );

    let spin_at_meltdown = reactor.spin_rate();

    // Try to escape Meltdown via enable/disable toggling
    reactor.set_enabled(false);
    reactor.tick(DT).unwrap();
    assert_eq!(
        reactor.state(),
        EnerDState::Meltdown,
        "Disabling should not exit Meltdown"
    );

    reactor.set_enabled(true);
    reactor.tick(DT).unwrap();
    assert_eq!(
        reactor.state(),
        EnerDState::Meltdown,
        "Re-enabling should not exit Meltdown"
    );

    // Try SCRAM (should have no effect in Meltdown)
    reactor.request_scram();
    reactor.tick(DT).unwrap();
    assert_eq!(
        reactor.state(),
        EnerDState::Meltdown,
        "SCRAM should not exit Meltdown"
    );

    // Continue ticking; state must remain Meltdown
    for _ in 0..100 {
        reactor.tick(DT).unwrap();
    }
    assert_eq!(
        reactor.state(),
        EnerDState::Meltdown,
        "Meltdown is terminal; continued ticking should not exit it"
    );

    // Verify spin rate is decaying
    assert!(
        reactor.spin_rate() < spin_at_meltdown,
        "Spin rate should decay in Meltdown: was {:.1}, now {:.1}",
        spin_at_meltdown,
        reactor.spin_rate()
    );
}

#[test]
fn emergency_scram_via_can() {
    let bus = VirtualCanBus::new(256);
    // Use a large rx_capacity so published CAN frames from the reactor don't fill
    // the channel and cause the EMERGENCY_STOP frame to be dropped.
    let reactor_node = bus.connect(
        CanFilter::Any(vec![CanFilter::Exact(well_known::EMERGENCY_STOP)]),
        4096,
    );
    let sender_node = bus.connect(CanFilter::AcceptAll, 64);
    let (mut reactor, handles) = EnerDReactor::new(EnerDConfig::default(), reactor_node);
    reactor.init().unwrap();
    reactor.set_enabled(true);

    // Push to Critical with high inputs
    setup_vehicle_sensors(&handles, 40.0, 20.0, 8000.0, 1800.0);

    assert!(
        tick_until(&mut reactor, EnerDState::Critical, DT, 10_000),
        "Should reach Critical for SCRAM test, state={}",
        reactor.state()
    );

    // Reduce inputs to prevent Meltdown from firing on the same tick as SCRAM.
    setup_vehicle_sensors(&handles, 6.0, 0.0, 0.0, 1800.0);

    // Tick once with reduced inputs to clear any pending CAN frames and give
    // the router thread a chance to process the backlog.
    reactor.tick(DT).unwrap();

    // Send EMERGENCY_STOP via CAN (not request_scram())
    let estop_frame = CanFrame::new(well_known::EMERGENCY_STOP, &[0xFF], 0);
    sender_node.send(estop_frame).unwrap();

    // Give the CAN bus router thread time to deliver the frame
    std::thread::sleep(Duration::from_millis(50));

    // Tick to process the CAN message and execute SCRAM
    reactor.tick(DT).unwrap();

    assert_eq!(
        reactor.state(),
        EnerDState::Dormant,
        "SCRAM via CAN EMERGENCY_STOP should return reactor to Dormant"
    );
    assert_eq!(
        reactor.spin_rate(),
        0.0,
        "Spin rate should be zero after SCRAM"
    );
    assert_eq!(
        reactor.containment(),
        100.0,
        "Containment should be restored after SCRAM"
    );
}

#[test]
fn spinup_timeout_at_low_speed() {
    let (mut reactor, handles) = make_reactor();
    reactor.init().unwrap();
    reactor.set_enabled(true);

    // Speed just above min_activation_speed (5.0), with zero force inputs.
    // flux = 0.2 * (0 + 1800 * 0) = 0. Spinup torque = 50.
    // Equilibrium spin = 50 / 0.5 = 100 (below sustain threshold of 150).
    // Also need net_power < parasitic_load for spinup to fail:
    // At spin=100, gross_power = 0.0012 * 10000 = 12 kW, net = 10 kW >= 2 kW.
    // So spin threshold is the limiting factor (100 < 150). Timeout at 30s.
    setup_vehicle_sensors(&handles, 6.0, 0.0, 0.0, 1800.0);

    // Should enter SpinUp quickly
    assert!(
        tick_until(&mut reactor, EnerDState::SpinUp, DT, 5),
        "Should enter SpinUp at speed > min_activation_speed"
    );

    // Tick for 31 seconds (3100 ticks at 10ms) to exceed spinup_timeout_secs (30)
    let mut timeout_dtc_found = false;
    for _ in 0..3100 {
        reactor.tick(DT).unwrap();

        // Check for P0ED6 DTC on each tick (DTCs are cleared each tick)
        let dtcs = reactor.active_dtcs();
        if dtcs.iter().any(|d| d.code == "P0ED6") {
            timeout_dtc_found = true;
        }

        // If we already returned to Dormant, the timeout fired
        if reactor.state() == EnerDState::Dormant {
            break;
        }
    }

    assert_eq!(
        reactor.state(),
        EnerDState::Dormant,
        "Reactor should return to Dormant after spinup timeout"
    );
    assert!(
        timeout_dtc_found,
        "P0ED6 (spin-up timeout) DTC should have been emitted"
    );
}

#[test]
fn feedback_loop_stability() {
    let (mut reactor, handles) = make_reactor();
    reactor.init().unwrap();
    reactor.set_enabled(true);

    // Aggressive but realistic inputs, sustained for a long time
    setup_vehicle_sensors(&handles, 35.0, 5.0, 3000.0, 1800.0);

    // 30000 ticks at 10ms = 300 seconds
    for _ in 0..30_000 {
        reactor.tick(DT).unwrap();
    }

    let spin = reactor.spin_rate();
    let containment = reactor.containment();
    let plasma = reactor.plasma_temp();
    let power = reactor.net_power_kw();

    // All values must be finite
    assert!(spin.is_finite(), "Spin rate must be finite: got {spin}");
    assert!(
        containment.is_finite(),
        "Containment must be finite: got {containment}"
    );
    assert!(
        plasma.is_finite(),
        "Plasma temp must be finite: got {plasma}"
    );
    assert!(power.is_finite(), "Net power must be finite: got {power}");

    // Spin rate bounded by max_spin_rate (800)
    assert!(
        spin <= 800.0,
        "Spin rate must not exceed max_spin_rate (800): got {spin:.1}"
    );
    assert!(spin >= 0.0, "Spin rate must be non-negative: got {spin:.1}");

    // Containment bounded [0, 100]
    assert!(
        (0.0..=100.0).contains(&containment),
        "Containment must be in [0, 100]: got {containment:.1}"
    );

    // Plasma temperature bounded to reasonable range
    assert!(
        plasma >= 0.0,
        "Plasma temp must be non-negative: got {plasma:.1}"
    );
    assert!(
        plasma <= 120.0,
        "Plasma temp must not exceed 120 MK: got {plasma:.1}"
    );
}

#[test]
fn reactor_disabled_preserves_dormant() {
    let (mut reactor, handles) = make_reactor();
    reactor.init().unwrap();
    // Do NOT enable the reactor

    // Set high inputs that would normally cause spinup
    setup_vehicle_sensors(&handles, 40.0, 20.0, 8000.0, 1800.0);

    for _ in 0..1000 {
        reactor.tick(DT).unwrap();
    }

    assert_eq!(
        reactor.state(),
        EnerDState::Dormant,
        "Disabled reactor must stay Dormant regardless of inputs"
    );
    assert_eq!(
        reactor.spin_rate(),
        0.0,
        "Disabled reactor spin rate must remain 0"
    );
    assert_eq!(
        reactor.containment(),
        100.0,
        "Disabled reactor containment must remain 100%"
    );
}
