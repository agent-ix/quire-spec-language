---
id: SR-935
title: "Code review of PR #551 (delete ToolPin and the toolchain pin from qsl-replay)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@cfef8e790f508ebbe8ffbdba796072805841ef1f; qsl-replay/src/execute.rs, qsl-replay/src/execute/frame.rs, qsl-replay/src/execute/state_clause.rs, qsl-replay/src/execute/tests.rs, qsl-replay/src/lib.rs, qsl-replay/src/proof_result.rs, qsl-replay/src/result.rs"
review_set: subset
---

## Summary

Ticket: QSL-351 (ToolPin part only). Code review with the rust-review lane
over `git diff origin/main...HEAD` on the seven qsl-replay files.

Checked:

- `ToolPin` is gone from `proof_result.rs` and the `lib.rs` re-export;
  `ProofResultEnvelope::tool_pin` (field and accessor),
  `BackendProviderSource::tool_pin`, its share of
  `measured_encoded_bytes` and its `to_source` write are all removed
  together, so the reader, the size measurement and the round trip stay
  consistent.
- `WitnessArmResult`/`InputArmResult` lose `toolchain_pin` (field,
  accessor, `settle` parameter). All six `settle` call sites in
  `execute.rs`, `frame.rs` and `state_clause.rs` drop the argument; the
  `TOOLCHAIN` constant and its `use super::...TOOLCHAIN` imports are gone.
- `common_measured_bytes` now sums only resolved-region members. Its test
  expects 21 = `registry` 8 + `pkg-a` 5 + `git` 3 + `rev-1` 5; the old
  32 minus `kani-0.67.0` (11) is 21, so the change removes exactly the
  pin's bytes.
- Size bounds: both `read_bounded` (result.rs:359) and
  `read_backend_provider_envelope` (proof_result.rs:291) compare a
  self-measured byte count against `MAX_ENCODED_BYTES` (1 MiB). Their
  oversized tests build content of `MAX_ENCODED_BYTES + 1` bytes by
  themselves (result.rs:619, proof_result.rs:497); none relied on the pin's
  bytes to cross the bound, and no boundary-exact test exists that the
  11-byte drop could shift.
- No other field, check or invariant was removed with the pin: `backend`
  (identity + manifest digest), `inconclusive_cause`, `disagreement`,
  `record`, `resolved_regions` and `charges` are untouched, and
  TC-179/TC-189/TC-444 keep their remaining assertions.
- Dangling names: `grep -rniE 'tool_?pin|toolchain_pin|ToolPin|TOOLCHAIN\b'`
  over the tree (excluding `target/`, `.git`, `spec/reviews/`) finds no
  code hit.
- Consumers: the same grep over `/home/peter/dev/quire-contract-codegen`
  (all `.rs`/`.md`/`.toml`, excluding target) hits only historical review
  files in two CG worktrees, no code; `quire-contract-ir/src` has none.
- Gate: the coder's `make ci` log for c399a24d ends `exit=0`.
  `git diff c399a24d cfef8e79 -- . ':!spec'` differs only in three
  `reviews/qsl-354-*.md` files from #550, so the code under review is the
  code that passed. No focused re-run was made.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. A mechanical, complete deletion: every producer and reader of the pin
goes in the same commit, the size measurement subtracts exactly the pin's
bytes, and nothing with behaviour was removed alongside it. No new
`unwrap`, `unsafe`, integer conversion or allocation is introduced.
