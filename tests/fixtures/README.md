# Golden-test fixtures (from the updated Saronic spec PDF)

Line N of `*.in.jsonl` is a stdin command; line N of `*.expected.jsonl` is the exact stdout line the spec shows for it.

- `base` - the base spec's Example (9 commands).
- `cargo_and_history` - the Follow-on 1 Example, followed by the Follow-on 2 Example, which the spec says continues from it (25 commands).

## Lines NOT copied from the PDF
The spec skips some ticks ("[ticks 2-3 omitted]", "[ticks 5-10 omitted]"). They were filled in as plain `tick` commands whose
expected output is the previous state with only the tick number changed (the vessel is mid-transit, so nothing else moves):
- base.expected.jsonl lines 6-7
- cargo_and_history.expected.jsonl lines 11-16

## Notes
- The standalone single-command examples in the PDF (e.g. `get_state_at` tick 5 showing arrives_at_tick 7 with cargo) come from
  different, partly impossible scenarios. Don't build tests from them. Only the two stdin/stdout Examples are used here.
- Compare outputs as parsed JSON, or byte-for-byte after you confirm the grader's format. Array order matters either way.
