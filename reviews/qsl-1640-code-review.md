---
id: SR-1640001
title: "Code review of PR 2291: per-binder charge log (QSL-1640, FR-520-AC-11)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@0bd98250e0bfe4c9bfc353a23534e19126172115; QSL-1640; PR #2291 against origin/main 598cc99fb (trial merge is the identity, head already contains main); qsl-eval/src/value/expression/{evaluate.rs,mod.rs,protocol_slot.rs,protocol_slot/charges.rs}, qsl-eval/src/value/mod.rs, qsl-eval/tests/it/protocol_slots/**, Cargo.lock"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-520
    type: reviews
---
# Code review of PR 2291

## Summary

Ticket: QSL-1640. Rust lane folded in (rust-review, rust-style). The PR routes the quire-exact IR-1280 interval accounting (`enable_charge_accounting`, `take_charge_accounting`, `disable_charge_accounting`, `ChargeAccounting::amount`) onto binder reads in `Machine::eval_local`, returns a `BinderCharges` log on both `ProtocolSlotEvaluation` variants, and owns `BINDER_ACCOUNTING_VERSION`. Lock: quire-exact stays at 87fefed (contains IR-1280 commit d8e6411); quire-semantic-value moves 0d4bc2e to 30c2493 (current origin/main of that repo, two commits: a supplied-path-arena test and spec). Cargo.toml is unchanged, so `branch = "main"` is intact and the bump is lock-only, correct under the rule and not breaking (`cargo check --locked` and every suite below pass on it). The bump is not required by the feature.

Gates on the head (which equals the trial merge with origin/main): `cargo fmt --all -- --check` pass; `cargo check --locked --workspace --all-targets` pass; clippy `-D warnings` default and `--no-default-features` pass; `arch-lint qualified-core --qsl .` pass; `arch-lint api-surface --qsl . --qsl-only` pass; `qsl-eval` all targets pass (183 lib, 1068 `it` with 6 ignored, 43 doctests); root plain exit 0 (882 passed, 6 ignored in the main binary) and `--all-features` 0 failures (931 passed, 12 ignored); `qsl-replay` has 20 failures, 19 known by name plus one new name, see FND-004 (not caused by the PR).

Mutations (each reverted; tree clean afterwards): M1 attribute the closed interval to the binder being read instead of the previous one, killed by `the_binder_entries_and_the_unattributed_entry_combine_to_the_meter_total`; M2 `accounting_version()` returns "", killed by `a_slot_call_logs_each_binder_it_read_with_the_accounting_version`; M3 sum the eight high-water counters instead of max, killed by `a_high_water_counter_is_the_largest_binder_entry_not_the_sum`; M5 invert the capture restore condition, killed by `a_call_restores_the_meters_interval_capture_to_the_state_it_found`; M4 drop every charge routed to the unattributed entry, SURVIVED all four tests; M6 remove the `frames.len() != 1` root-frame gate, SURVIVED all four tests (FND-003).

Extra check: making the reader-backed/in-process equality helper `without_location` compare the `BinderCharges` as well leaves all 51 `protocol_slots` tests passing, so the log is equal across the in-process and read-back package (FR-520-AC-9) in the cases those tests cover.

## Verdict

**NOT MERGEABLE as specified**: the attribution rule contradicts FR-520-AC-11 as it was written (FND-001, spec decision needed); two medium items to fix or decide in the same PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The charge owner is "the binder read most recently". The evaluator reads every operand of an operator before applying it, so an operator charge lands on whichever binder was read last, and constants evaluated after a read land on that binder too. In the PR's own recovery fixture the equality over `recovered` and `target` is charged to `target` (entries: recovered all zero, target work 5 and one value occurrence, activated work 2 and result 2) and swapping the operand order would flip the charge between binders. FR-520-AC-11 as written says charges "while reading or evaluating a binder's supplied value" belong to that binder and charges made by no binder belong to none; here a binder-free subexpression after the first read (for example the right side of `a and (1 + 2 == 3)`) is recorded under `a`, and the unattributed entry can only ever hold the prefix before the first read. The PR rewrote the criterion and outcome-map text to the positional rule instead of changing the code, so the contradiction is hidden by a spec edit. | qsl-eval/src/value/expression/evaluate.rs:1818-1836; qsl-eval/src/value/expression/protocol_slot/charges.rs:132-165; qsl-eval/tests/it/protocol_slots/acceptance/state_compensation.rs:773-805 |
| FND-002 | medium | A meter interval the caller had enabled is drained at entry (`meter.take_charge_accounting()`) and its unread amounts are discarded; after the call the caller's interval is enabled but holds zero, so the call's own charges are also absent from it. This is documented only by a code comment inside `evaluate_protocol_slot`; the trait/variant docs, FR-520 and TC-1315 are silent, and the test only checks an empty pre-existing interval (`drained` all zero), never a non-empty one. Silent loss of caller accounting on a public entry point. Document it on `evaluate_protocol_slot` and in FR-520, and test a non-empty caller interval, or refuse such a call with a typed failure. | qsl-eval/src/value/expression/mod.rs:1966-1983; qsl-eval/tests/it/protocol_slots/acceptance/state_compensation.rs:844-871 |
| FND-003 | medium | Two behaviors the new code adds are pinned by no test: mutation M4 (charges made before the first binder read are dropped instead of recorded as unattributed) and M6 (removing the root-frame gate in `route_binder_read`) both survive every FR-520-AC-11 test, because every fixture charge happens after a binder read (`unattributed` asserted `[0; 10]`) and no fixture calls a function whose callee slot index equals a binder slot. The "charges made by no binder are recorded under none" clause of AC-11 therefore has no failing test. | qsl-eval/tests/it/protocol_slots/acceptance/state_compensation.rs:773-805; qsl-eval/src/value/expression/evaluate.rs:1819-1823 |
| FND-004 | low | `qsl-replay` lib test `spine::lifecycle::tests::aggregate_package_accepts_temporal_selection_from_later_source` fails (`MissingQualifiedName M::Widget`); it is not in the 19 known failures. It fails identically on origin/main 598cc99fb (control run), so it is not caused by this PR, but it is a 20th failure the gate comparison by name does not know. Report to the lead. | qsl-replay/src/spine/lifecycle/tests.rs:1344 |

## Style notes (not findings)

`ProtocolSlotEvaluation::Evaluated(Evaluation, BinderCharges)` is positional while `Unavailable` is a struct variant; the two types cannot be confused, so no rule is violated, and pre-release API breaks are allowed. The `saturating_add` in `ChargeAmounts::absorb` carries a comment that it is unreachable (interval sums are bounded by the checked global totals), which meets the saturation-policy rule. Adjacent and pre-existing, out of scope: `eval_local` consults `unavailable_slots` for any frame depth while the PR's own routing is gated on depth 1; a callee whose local slot index equals an unavailable root binder slot could read as Unavailable. Not reproduced here (no fixture calls a function from a slot expression); worth a ticket if a call-bearing slot is in scope.

## Coverage

Examined: FR-520-AC-11, FR-520-AC-9 (charge-log equality, probed), FR-520 outcome map row for per-binder accounting, quire-exact FR-358-AC-16 to AC-21 and the IR-1280 API (read), quire-protocol FR-045-AC-7 and AC-8 (consumer). Files examined with no finding: qsl-eval/src/value/mod.rs, qsl-eval/src/value/expression/protocol_slot.rs, Cargo.lock.
