use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

use crate::DashboardState;

pub fn draw_dashboard(f: &mut Frame, state: &DashboardState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(6), // Subsystem panels
            Constraint::Length(8), // Ener-D Reactor panel
            Constraint::Length(5), // Vehicle dynamics
            Constraint::Min(6),    // Diagnostics + CAN
            Constraint::Length(1), // Footer
        ])
        .split(f.area());

    draw_header(f, chunks[0], state);
    draw_subsystems(f, chunks[1], state);
    draw_enerd_panel(f, chunks[2], state);
    draw_dynamics(f, chunks[3], state);
    draw_bottom(f, chunks[4], state);
    draw_footer(f, chunks[5]);
}

fn draw_header(f: &mut Frame, area: Rect, state: &DashboardState) {
    let status_color = match state.vehicle_state {
        ppe_state::VehicleState::Off => Color::DarkGray,
        ppe_state::VehicleState::Accessory => Color::Yellow,
        ppe_state::VehicleState::Ready => Color::Green,
        ppe_state::VehicleState::Driving => Color::Cyan,
        ppe_state::VehicleState::Charging => Color::Blue,
        ppe_state::VehicleState::Fault => Color::Red,
        ppe_state::VehicleState::SafeState => Color::Magenta,
    };

    let pause_indicator = if state.paused { " [PAUSED]" } else { "" };
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " PLANETARY PARTICLE ENGINE ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | State: "),
        Span::styled(
            format!("{}", state.vehicle_state),
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" | Gear: {} ", state.gear)),
        Span::raw(format!("| Scenario: {} ", state.current_scenario)),
        Span::raw(format!("| Uptime: {:.1}s", state.uptime_secs)),
        Span::styled(
            pause_indicator,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL));

    f.render_widget(header, area);
}

fn draw_subsystems(f: &mut Frame, area: Rect, state: &DashboardState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    draw_bms_panel(f, cols[0], state);
    draw_motor_panel(f, cols[1], state);
    draw_thermal_panel(f, cols[2], state);
}

fn draw_bms_panel(f: &mut Frame, area: Rect, state: &DashboardState) {
    let block = Block::default()
        .title(format!(" BMS [{}] ", state.bms_state))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(subsystem_color(state.bms_state)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

    // SOC gauge
    let soc_color = if state.soc_pct > 50.0 {
        Color::Green
    } else if state.soc_pct > 20.0 {
        Color::Yellow
    } else {
        Color::Red
    };
    let gauge = Gauge::default()
        .label(format!("SOC: {:.1}%", state.soc_pct))
        .ratio((state.soc_pct / 100.0).clamp(0.0, 1.0))
        .gauge_style(Style::default().fg(soc_color));
    f.render_widget(gauge, chunks[0]);

    let voltage = Paragraph::new(format!(" Voltage: {:.1}V", state.pack_voltage));
    f.render_widget(voltage, chunks[1]);

    let current = Paragraph::new(format!(" Current: {:.1}A", state.pack_current));
    f.render_widget(current, chunks[2]);

    let temp = Paragraph::new(format!(" Temp: {:.1}C", state.pack_temperature));
    f.render_widget(temp, chunks[3]);
}

fn draw_motor_panel(f: &mut Frame, area: Rect, state: &DashboardState) {
    let block = Block::default()
        .title(format!(" Motor [{}] ", state.motor_state))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(motor_color(state.motor_state)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

    let rpm = Paragraph::new(format!(" RPM: {:.0}", state.motor_rpm));
    f.render_widget(rpm, chunks[0]);

    let torque = Paragraph::new(format!(" Torque: {:.1} Nm", state.motor_torque));
    f.render_widget(torque, chunks[1]);

    let temp_color = if state.motor_temperature > 120.0 {
        Color::Red
    } else if state.motor_temperature > 80.0 {
        Color::Yellow
    } else {
        Color::Green
    };
    let temp = Paragraph::new(Span::styled(
        format!(" Temp: {:.1}C", state.motor_temperature),
        Style::default().fg(temp_color),
    ));
    f.render_widget(temp, chunks[2]);
}

fn draw_thermal_panel(f: &mut Frame, area: Rect, state: &DashboardState) {
    let block = Block::default()
        .title(format!(" Thermal [{}] ", state.cooling_state))
        .borders(Borders::ALL);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .split(inner);

    let temp = Paragraph::new(format!(" Coolant: {:.1}C", state.coolant_temp));
    f.render_widget(temp, chunks[0]);

    let fan = Gauge::default()
        .label(format!("Fan: {:.0}%", state.fan_speed_pct))
        .ratio((state.fan_speed_pct / 100.0).clamp(0.0, 1.0))
        .gauge_style(Style::default().fg(Color::Blue));
    f.render_widget(fan, chunks[1]);
}

fn draw_enerd_panel(f: &mut Frame, area: Rect, state: &DashboardState) {
    let enerd_color = enerd_state_color(state.reactor_state);
    let block = Block::default()
        .title(format!(" Ener-D Reactor [{}] ", state.reactor_state))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(enerd_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // 3 columns: left (25%) info, center (50%) gauges, right (25%) state indicator
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(inner);

    draw_enerd_info(f, cols[0], state);
    draw_enerd_gauges(f, cols[1], state);
    draw_enerd_state_indicator(f, cols[2], state);
}

fn draw_enerd_info(f: &mut Frame, area: Rect, state: &DashboardState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    let state_line = Paragraph::new(format!(" State: {}", state.reactor_state))
        .style(Style::default().fg(enerd_state_color(state.reactor_state)));
    f.render_widget(state_line, chunks[0]);

    let cont_color = if state.reactor_containment_pct > 70.0 {
        Color::Green
    } else if state.reactor_containment_pct > 50.0 {
        Color::Yellow
    } else {
        Color::Red
    };
    let containment = Paragraph::new(format!(
        " Containment: {:.1}%",
        state.reactor_containment_pct
    ))
    .style(Style::default().fg(cont_color));
    f.render_widget(containment, chunks[1]);

    let plasma_color = if state.reactor_plasma_temp < 50.0 {
        Color::Cyan
    } else if state.reactor_plasma_temp < 80.0 {
        Color::Yellow
    } else {
        Color::Red
    };
    let plasma = Paragraph::new(format!(" Plasma: {:.1} MK", state.reactor_plasma_temp))
        .style(Style::default().fg(plasma_color));
    f.render_widget(plasma, chunks[2]);
}

fn draw_enerd_gauges(f: &mut Frame, area: Rect, state: &DashboardState) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    // Spin rate gauge (max 800 rad/s)
    let spin_gauge = Gauge::default()
        .label(format!("Spin: {:.0} rad/s", state.reactor_spin_rate))
        .ratio((state.reactor_spin_rate / 800.0).clamp(0.0, 1.0))
        .gauge_style(Style::default().fg(Color::Magenta));
    f.render_widget(spin_gauge, rows[0]);

    // Power output gauge (max 250 kW for "safe", shows over)
    let power_ratio = (state.reactor_power_kw / 250.0).clamp(0.0, 1.0);
    let power_color = if state.reactor_power_kw > 200.0 {
        Color::Red
    } else if state.reactor_power_kw > 100.0 {
        Color::Yellow
    } else {
        Color::Green
    };
    let power_gauge = Gauge::default()
        .label(format!("Power: {:.1} kW", state.reactor_power_kw))
        .ratio(power_ratio)
        .gauge_style(Style::default().fg(power_color));
    f.render_widget(power_gauge, rows[1]);

    // Momentum flux gauge (max ~5000 N)
    let flux_gauge = Gauge::default()
        .label(format!("Flux: {:.0} N", state.reactor_momentum_flux))
        .ratio((state.reactor_momentum_flux / 5000.0).clamp(0.0, 1.0))
        .gauge_style(Style::default().fg(Color::Blue));
    f.render_widget(flux_gauge, rows[2]);

    // Containment gauge
    let cont_color = if state.reactor_containment_pct > 70.0 {
        Color::Green
    } else if state.reactor_containment_pct > 50.0 {
        Color::Yellow
    } else {
        Color::Red
    };
    let cont_gauge = Gauge::default()
        .label(format!(
            "Containment: {:.1}%",
            state.reactor_containment_pct
        ))
        .ratio((state.reactor_containment_pct / 100.0).clamp(0.0, 1.0))
        .gauge_style(Style::default().fg(cont_color));
    f.render_widget(cont_gauge, rows[3]);
}

fn draw_enerd_state_indicator(f: &mut Frame, area: Rect, state: &DashboardState) {
    let (text, color, modifier) = match state.reactor_state {
        ppe_state::EnerDState::Dormant => ("DORMANT", Color::DarkGray, Modifier::empty()),
        ppe_state::EnerDState::SpinUp => ("SPIN UP", Color::Yellow, Modifier::empty()),
        ppe_state::EnerDState::Sustaining => ("SUSTAIN", Color::Green, Modifier::BOLD),
        ppe_state::EnerDState::Overdrive => ("OVERDRIVE", Color::Magenta, Modifier::BOLD),
        ppe_state::EnerDState::Critical => (
            "CRITICAL",
            Color::Red,
            Modifier::BOLD | Modifier::SLOW_BLINK,
        ),
        ppe_state::EnerDState::Meltdown => ("MELTDOWN", Color::Red, Modifier::BOLD),
    };

    let indicator = Paragraph::new(text)
        .style(Style::default().fg(color).add_modifier(modifier))
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(indicator, area);
}

fn draw_dynamics(f: &mut Frame, area: Rect, state: &DashboardState) {
    let block = Block::default()
        .title(" Vehicle Dynamics ")
        .borders(Borders::ALL);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(inner);

    let speed_gauge = Gauge::default()
        .label(format!("{:.0} km/h", state.speed_kmh))
        .ratio((state.speed_kmh / 200.0).clamp(0.0, 1.0))
        .gauge_style(Style::default().fg(Color::Cyan));
    f.render_widget(speed_gauge, cols[0]);

    let throttle_gauge = Gauge::default()
        .label(format!("THR {:.0}%", state.throttle_pct))
        .ratio((state.throttle_pct / 100.0).clamp(0.0, 1.0))
        .gauge_style(Style::default().fg(Color::Green));
    f.render_widget(throttle_gauge, cols[1]);

    let brake_gauge = Gauge::default()
        .label(format!("BRK {:.0}%", state.brake_pct))
        .ratio((state.brake_pct / 100.0).clamp(0.0, 1.0))
        .gauge_style(Style::default().fg(Color::Red));
    f.render_widget(brake_gauge, cols[2]);

    let power = Paragraph::new(format!(" {:.1} kW", state.power_kw))
        .style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(power, cols[3]);
}

fn draw_bottom(f: &mut Frame, area: Rect, state: &DashboardState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // DTC panel
    let dtc_items: Vec<ListItem> = state
        .active_dtcs
        .iter()
        .map(|dtc| {
            let color = match dtc.severity {
                ppe_core::DtcSeverity::Critical => Color::Red,
                ppe_core::DtcSeverity::Fault => Color::LightRed,
                ppe_core::DtcSeverity::Warning => Color::Yellow,
                ppe_core::DtcSeverity::Info => Color::White,
            };
            ListItem::new(Span::styled(
                format!("[{}] {} - {}", dtc.severity, dtc.code, dtc.description),
                Style::default().fg(color),
            ))
        })
        .collect();

    let dtc_list = List::new(dtc_items).block(
        Block::default()
            .title(format!(" DTCs ({}) ", state.active_dtcs.len()))
            .borders(Borders::ALL),
    );
    f.render_widget(dtc_list, cols[0]);

    // CAN monitor
    let can_items: Vec<ListItem> = state
        .can_log
        .iter()
        .rev()
        .take(20)
        .map(|frame| ListItem::new(format!("{frame}")))
        .collect();

    let can_list = List::new(can_items).block(
        Block::default()
            .title(" CAN Bus Monitor ")
            .borders(Borders::ALL),
    );
    f.render_widget(can_list, cols[1]);
}

fn draw_footer(f: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" [Q]", Style::default().fg(Color::Yellow)),
        Span::raw("uit "),
        Span::styled("[P]", Style::default().fg(Color::Yellow)),
        Span::raw("ause "),
        Span::styled("[S]", Style::default().fg(Color::Yellow)),
        Span::raw("cenario "),
        Span::styled("[F]", Style::default().fg(Color::Yellow)),
        Span::raw("ault "),
        Span::styled("[D]", Style::default().fg(Color::Yellow)),
        Span::raw("TC Clear "),
        Span::styled("[+/-]", Style::default().fg(Color::Yellow)),
        Span::raw(" Throttle "),
        Span::styled("[R]", Style::default().fg(Color::Yellow)),
        Span::raw("eactor "),
        Span::styled("[C]", Style::default().fg(Color::Yellow)),
        Span::raw("ontainment "),
    ]));
    f.render_widget(footer, area);
}

fn subsystem_color(state: ppe_state::BmsState) -> Color {
    match state {
        ppe_state::BmsState::Active | ppe_state::BmsState::Charging => Color::Green,
        ppe_state::BmsState::Precharging | ppe_state::BmsState::Balancing => Color::Yellow,
        ppe_state::BmsState::Standby => Color::DarkGray,
        ppe_state::BmsState::Fault | ppe_state::BmsState::SafeState => Color::Red,
    }
}

fn motor_color(state: ppe_state::MotorState) -> Color {
    match state {
        ppe_state::MotorState::Ready | ppe_state::MotorState::Running => Color::Green,
        ppe_state::MotorState::Initializing | ppe_state::MotorState::Regenerating => Color::Yellow,
        ppe_state::MotorState::Derating => Color::LightRed,
        ppe_state::MotorState::Disabled => Color::DarkGray,
        ppe_state::MotorState::Fault => Color::Red,
    }
}

fn enerd_state_color(state: ppe_state::EnerDState) -> Color {
    match state {
        ppe_state::EnerDState::Dormant => Color::DarkGray,
        ppe_state::EnerDState::SpinUp => Color::Yellow,
        ppe_state::EnerDState::Sustaining => Color::Green,
        ppe_state::EnerDState::Overdrive => Color::Magenta,
        ppe_state::EnerDState::Critical | ppe_state::EnerDState::Meltdown => Color::Red,
    }
}
