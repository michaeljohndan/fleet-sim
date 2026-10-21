//! Rough throughput benchmark. Not a test; numbers vary by machine.
//!
//!   cargo run --release --example bench -- <vessels> <ticks>
//!
//! Each tick sends one set_destination per vessel (most are rejected while vessels
//! are in transit) and then a tick. Reports time per command and bytes per
//! response, which shows that output size, O(vessels), dominates.

use std::time::Instant;

use fleet_sim::{handle_line, Simulation};

fn main() {
    let args: Vec<usize> = std::env::args()
        .skip(1)
        .filter_map(|a| a.parse().ok())
        .collect();
    let vessels = args.first().copied().unwrap_or(1_000);
    let ticks = args.get(1).copied().unwrap_or(100);
    let n = 20;

    let travel: Vec<Vec<u32>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| {
                    if i == j {
                        0
                    } else {
                        1 + ((i * 7 + j * 3) % 5) as u32
                    }
                })
                .collect()
        })
        .collect();
    let vs: Vec<String> = (0..vessels)
        .map(|i| format!(r#"{{"id":"v{i}","location":{}}}"#, i % n))
        .collect();
    let init = format!(
        r#"{{"command":"init","parameters":{{"travel_times":{},"dock_throughput":[{}],"vessels":[{}]}}}}"#,
        serde_json::to_string(&travel).unwrap(),
        vec!["5"; n].join(","),
        vs.join(",")
    );

    let mut sim = Simulation::new();
    handle_line(&mut sim, &init).unwrap();

    let start = Instant::now();
    let (mut commands, mut bytes) = (0u64, 0u64);
    for t in 0..ticks {
        for i in 0..vessels {
            let cmd = format!(
                r#"{{"command":"set_destination","parameters":{{"vessel_id":"v{i}","destination":{}}}}}"#,
                (i + t) % n
            );
            bytes += handle_line(&mut sim, &cmd).unwrap().len() as u64;
            commands += 1;
        }
        bytes += handle_line(&mut sim, r#"{"command":"tick","parameters":{}}"#)
            .unwrap()
            .len() as u64;
        commands += 1;
    }
    let secs = start.elapsed().as_secs_f64();
    println!("vessels={vessels} ticks={ticks} commands={commands}");
    println!(
        "total {secs:.3}s  |  {:.1} us/command  |  {:.0} bytes/response (avg)",
        secs * 1e6 / commands as f64,
        bytes as f64 / commands as f64
    );
}
