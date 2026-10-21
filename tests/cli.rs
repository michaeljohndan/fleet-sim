//! End-to-end: runs the real binary over stdin/stdout.

use std::io::Write;
use std::process::{Command, Stdio};

fn run(input: &[u8]) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_fleet_sim"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("binary starts");
    child.stdin.take().unwrap().write_all(input).unwrap();
    let out = child.wait_with_output().expect("binary exits");
    assert!(out.status.success(), "binary exited with {}", out.status);
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn binary_reproduces_both_spec_walkthroughs() {
    for (input, expected) in [
        (
            include_str!("fixtures/base.in.jsonl"),
            include_str!("fixtures/base.expected.jsonl"),
        ),
        (
            include_str!("fixtures/cargo_and_history.in.jsonl"),
            include_str!("fixtures/cargo_and_history.expected.jsonl"),
        ),
    ] {
        assert_eq!(run(input.as_bytes()), expected);
    }
}

#[test]
fn blank_lines_are_skipped_and_crlf_is_accepted() {
    let input = "\n{\"command\":\"get_state\",\"parameters\":{}}\r\n\n";
    assert_eq!(
        run(input.as_bytes()),
        "{\"status\":\"error\",\"payload\":{\"message\":\"simulation not initialized\"}}\n"
    );
}

#[test]
fn invalid_utf8_gets_an_error_line_and_the_loop_continues() {
    let mut input = vec![0xff, 0xfe, b'\n'];
    input.extend_from_slice(b"{\"command\":\"tick\",\"parameters\":{}}\n");
    let out = run(&input);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("input is not valid UTF-8"));
    assert!(lines[1].contains("simulation not initialized"));
}

#[test]
fn final_line_without_newline_is_processed() {
    let out = run(b"{\"command\":\"tick\",\"parameters\":{}}");
    assert_eq!(out.lines().count(), 1);
}
