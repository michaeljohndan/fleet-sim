//! I/O loop: one JSON command per stdin line, exactly one JSON response per stdout line.
//! Single-threaded and synchronous on purpose: commands are strictly ordered and
//! cost microseconds, so threads or an async runtime would add nondeterminism and
//! overhead for no gain.

use std::io::{self, BufRead, BufWriter, Write};

use fleet_sim::{handle_line, render_error, SimError, Simulation};

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let mut sim = Simulation::new();
    let mut buf = Vec::new();
    loop {
        buf.clear();
        if input.read_until(b'\n', &mut buf)? == 0 {
            break;
        }
        let response = match std::str::from_utf8(&buf) {
            Ok(line) => handle_line(&mut sim, line),
            Err(_) => Some(render_error(&SimError::InvalidCommand(
                "input is not valid UTF-8".into(),
            ))),
        };
        if let Some(line) = response {
            out.write_all(line.as_bytes())?;
            out.write_all(b"\n")?;
            // Flush per response: a driver may wait for each answer before
            // sending the next command.
            out.flush()?;
        }
    }
    Ok(())
}
