use anyhow::Result;
use clap::{Parser, Subcommand};

use ppe_can::{well_known, CanFilter, CanId, VirtualCanBus};
use ppe_diagnostics::obd;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "ppe-diag", about = "PPE Diagnostic CLI Tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Query an OBD-II PID
    Pid {
        /// PID to query (rpm, speed, coolant-temp, soc, voltage)
        name: String,
    },
    /// Read stored DTCs
    Dtcs,
    /// Sniff CAN bus traffic
    Sniff {
        /// Duration in seconds
        #[arg(short, long, default_value_t = 5)]
        duration: u64,
        /// Filter by CAN ID (hex, e.g. 0x100)
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// List supported PIDs
    ListPids,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pid { name } => {
            let pid = match name.as_str() {
                "rpm" => obd::pid::ENGINE_RPM,
                "speed" => obd::pid::VEHICLE_SPEED,
                "coolant-temp" => obd::pid::COOLANT_TEMP,
                "soc" => obd::pid::FUEL_LEVEL,
                "voltage" => obd::pid::CONTROL_MODULE_VOLTAGE,
                _ => {
                    eprintln!("Unknown PID '{name}'. Use 'list-pids' to see available PIDs.");
                    std::process::exit(1);
                }
            };

            println!("Querying PID 0x{pid:02X} ({name})...");
            println!("Note: Requires ppe-daemon to be running on the same CAN bus.");
            println!("In standalone mode, this is a reference for the OBD request format:");
            println!(
                "  Request:  CAN ID {} | 02 01 {:02X}",
                well_known::OBD_REQUEST,
                pid
            );
            println!("  Response: CAN ID {} | ...", well_known::OBD_RESPONSE);
        }
        Commands::Dtcs => {
            println!("Querying stored DTCs...");
            println!("Request:  CAN ID {} | 01 03", well_known::OBD_REQUEST);
            println!("Response: CAN ID {} | ...", well_known::OBD_RESPONSE);
            println!();
            println!("Note: Connect to a running ppe-daemon for live DTC reads.");
        }
        Commands::Sniff { duration, filter } => {
            println!("CAN bus sniffer mode ({duration}s)");
            if let Some(ref f) = filter {
                println!("Filtering: {f}");
            }
            println!("Note: Requires shared CAN bus with ppe-daemon.");
            println!();

            // Demo mode: show what sniffing would look like
            let bus = VirtualCanBus::new(256);
            let monitor = if let Some(f) = filter {
                let id_val = u16::from_str_radix(f.trim_start_matches("0x"), 16).unwrap_or(0);
                if let Some(id) = CanId::new(id_val) {
                    bus.connect(CanFilter::Exact(id), 256)
                } else {
                    bus.connect(CanFilter::AcceptAll, 256)
                }
            } else {
                bus.connect(CanFilter::AcceptAll, 256)
            };

            let deadline = std::time::Instant::now() + Duration::from_secs(duration);
            println!("Listening...");
            while std::time::Instant::now() < deadline {
                if let Some(frame) = monitor.recv_timeout(Duration::from_millis(100)) {
                    println!("  {frame}");
                }
            }
            println!("Sniff complete.");
        }
        Commands::ListPids => {
            println!("Supported OBD-II PIDs:");
            println!("  rpm          (0x0C) - Engine/Motor RPM");
            println!("  speed        (0x0D) - Vehicle Speed (km/h)");
            println!("  coolant-temp (0x05) - Coolant Temperature (C)");
            println!("  soc          (0x2F) - State of Charge (%)");
            println!("  voltage      (0x42) - Battery Voltage (V)");
        }
    }

    Ok(())
}
