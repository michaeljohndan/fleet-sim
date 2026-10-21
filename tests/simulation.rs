//! Seeded randomized testing (in the spirit of TigerBeetle / FoundationDB
//! deterministic simulation testing).
//!
//! Each seed builds a random world and fires hundreds of random commands, valid and
//! invalid. After EVERY command it checks:
//!   1. all design invariants (World::check_invariants)
//!   2. a rejected command left the state byte-identical (D13)
//!
//! At the end it checks that history never changed and that the same seed produces
//! byte-identical output twice (determinism).
//!
//! Reproduce a failure:  FLEET_SIM_SEED=<seed> cargo test --test simulation
//! Run more seeds:       FLEET_SIM_SEEDS=5000 cargo test --release --test simulation

use fleet_sim::{handle_line, Simulation};

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + self.below((hi - lo + 1) as u64) as i64
    }
}

const STEPS: usize = 400;
const GET_STATE: &str = r#"{"command":"get_state","parameters":{}}"#;

/// Runs one seed and returns every output line (for the determinism check).
fn run_seed(seed: u64) -> Vec<String> {
    let mut rng = Rng::new(seed);
    let n = rng.range(2, 4) as usize;
    let vessels = rng.range(1, 4) as usize;
    let cargo_mode = !seed.is_multiple_of(4);

    let travel: Vec<Vec<i64>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| if i == j { 0 } else { rng.range(1, 5) })
                .collect()
        })
        .collect();
    let ids: Vec<String> = (0..vessels).map(|i| format!("v{i}")).collect();
    let vessel_json: Vec<String> = ids
        .iter()
        .map(|id| format!(r#"{{"id":"{id}","location":{}}}"#, rng.below(n as u64)))
        .collect();
    let throughput = if cargo_mode {
        let t: Vec<String> = (0..n).map(|_| rng.range(1, 3).to_string()).collect();
        format!(r#","dock_throughput":[{}]"#, t.join(","))
    } else {
        String::new()
    };
    let init = format!(
        r#"{{"command":"init","parameters":{{"travel_times":{}{throughput},"vessels":[{}]}}}}"#,
        serde_json::to_string(&travel).unwrap(),
        vessel_json.join(",")
    );

    let mut sim = Simulation::new();
    let mut outputs = Vec::new();
    let first = handle_line(&mut sim, &init).unwrap();
    assert!(
        first.contains(r#""status":"ok""#),
        "seed {seed}: init failed: {first}"
    );
    let mut history = vec![first.clone()];
    outputs.push(first);

    for step in 0..STEPS {
        let cmd = random_command(&mut rng, &ids, n, cargo_mode);
        let before = handle_line(&mut sim, GET_STATE).unwrap();
        let out = handle_line(&mut sim, &cmd).unwrap();
        let ctx = format!("seed {seed} step {step}: {cmd}\n  -> {out}");

        sim.world()
            .unwrap()
            .check_invariants()
            .unwrap_or_else(|e| panic!("{ctx}\n  invariant broken: {e}"));
        if out.contains(r#""status":"error""#) {
            let after = handle_line(&mut sim, GET_STATE).unwrap();
            assert_eq!(before, after, "{ctx}\n  rejected command changed state");
        } else if cmd.contains(r#""command":"tick""#) {
            history.push(out.clone());
        }
        outputs.push(out);
    }

    for (t, recorded) in history.iter().enumerate() {
        let q = format!(r#"{{"command":"get_state_at","parameters":{{"tick":{t}}}}}"#);
        let now = handle_line(&mut sim, &q).unwrap();
        assert_eq!(&now, recorded, "seed {seed}: history at tick {t} changed");
    }
    outputs
}

fn random_command(rng: &mut Rng, ids: &[String], n: usize, cargo_mode: bool) -> String {
    let vessel = if rng.below(20) == 0 {
        "ghost".to_string()
    } else {
        ids[rng.below(ids.len() as u64) as usize].clone()
    };
    let d = rng.range(-1, n as i64);
    let d2 = rng.range(0, n as i64 - 1);
    let loc = rng.range(0, n as i64 - 1);
    let qty = rng.range(0, 3);
    match rng.below(if cargo_mode { 100 } else { 45 }) {
        0..=19 => {
            if cargo_mode && rng.below(2) == 0 {
                format!(
                    r#"{{"command":"tick","parameters":{{"cargo_manifest":[{{"location":{loc},"destination":{d2},"quantity":{qty}}}]}}}}"#
                )
            } else {
                r#"{"command":"tick","parameters":{}}"#.to_string()
            }
        }
        20..=34 => format!(
            r#"{{"command":"set_destination","parameters":{{"vessel_id":"{vessel}","destination":{d}}}}}"#
        ),
        35..=37 => format!(
            r#"{{"command":"get_state_at","parameters":{{"tick":{}}}}}"#,
            rng.range(-1, 60)
        ),
        38..=39 => "{not json".to_string(),
        40..=44 => GET_STATE.to_string(),
        45..=59 => format!(r#"{{"command":"dock","parameters":{{"vessel_id":"{vessel}"}}}}"#),
        60..=69 => format!(r#"{{"command":"undock","parameters":{{"vessel_id":"{vessel}"}}}}"#),
        70..=82 => format!(
            r#"{{"command":"load","parameters":{{"vessel_id":"{vessel}","destination":{d}}}}}"#
        ),
        83..=94 => format!(
            r#"{{"command":"unload","parameters":{{"vessel_id":"{vessel}","destination":{d}}}}}"#
        ),
        _ => format!(
            r#"{{"command":"swap","parameters":{{"vessel_id":"{vessel}","load_destination":{d2},"unload_destination":{d}}}}}"#
        ),
    }
}

fn seeds() -> Vec<u64> {
    if let Ok(s) = std::env::var("FLEET_SIM_SEED") {
        return vec![s.parse().expect("FLEET_SIM_SEED must be a number")];
    }
    let count = std::env::var("FLEET_SIM_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300);
    (0..count).collect()
}

#[test]
fn random_command_sequences_keep_every_invariant() {
    for seed in seeds() {
        run_seed(seed);
    }
}

#[test]
fn same_seed_gives_byte_identical_output() {
    for seed in seeds().into_iter().take(50) {
        assert_eq!(
            run_seed(seed),
            run_seed(seed),
            "seed {seed} is not deterministic"
        );
    }
}

#[test]
fn random_runs_exercise_cargo_operations() {
    // Guards against a generator that silently stops reaching interesting states.
    let mut loads = 0;
    for seed in (1u64..40).filter(|s| !s.is_multiple_of(4)) {
        loads += run_seed(seed)
            .iter()
            .filter(|l| l.contains(r#""cargo":[{"destination""#))
            .count();
    }
    assert!(loads > 0, "random runs never put cargo on a vessel");
}
