---
id: SR-160301
title: "Code review of PR #2229: restore the qsl-semantics lib suite on main, code-review with rust-review lane"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@c809ff263; git diff origin/main...c809ff263 (PR #2229, branch qsl-1603-main-red): qsl-semantics/src/check/lowering/model.rs and protocol test fixtures; ticket QSL-1603"
review_set: subset
date: "2026-11-03"
---

## Summary

Ticket: QSL-1603. PR: quire-spec-language#2229, head c809ff263, trial-merged on origin/main 3c12ee6dd. Method: code-review with the rust-review lane. The diff replaces three OnceLock uses in `AdmittedModel` with a systems classification computed at assembly, and restates stale protocol test fixtures to the grammar of #2184 (alias kinds, header order, shadowing, duplicate alias, one temporal profile).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| AdmittedModel.systems | qsl-semantics/src/check/lowering/model.rs:255-262 | examined |
| AdmittedModel::assemble | qsl-semantics/src/check/lowering/model.rs:872-880 | examined |
| the_rows_select_the_concrete_protocol_and_record_its_node | qsl-semantics/src/check/refinement/protocol/tests.rs:513-540 | examined |
| an_await_matches_an_event_in_a_closed_window_under_a_temporal_profile | qsl-semantics/src/check/protocol_clause/tests/graph.rs:405-530 | examined |
| check_refusals helper | qsl-semantics/src/check/protocol_clause/tests.rs:113-137 | examined |
| checks helper | qsl-semantics/src/check/protocol_slots/tests.rs:100-115 | examined |
| check_with_limits helper | qsl-semantics/src/check/protocol_controls/model_state_tests.rs:41-80 | examined |
| FR-136-AC-6, FR-135-AC-10, FR-383-AC-17 | spec/functional | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The emission-refusal filter in three test helpers drops every `CheckCause::ProtocolReference`, including `ProtocolReferenceCause::Repeated`, while the doc comments say only `NotEmitted` is ignored. | qsl-semantics/src/check/protocol_clause/tests.rs:119 |
| FND-002 | low | The restated assertion in `the_rows_select_the_concrete_protocol_and_record_its_node` drops the check of the refinement cause variant; only the code/tag pair and the text at the alias remain. | qsl-semantics/src/check/refinement/protocol/tests.rs:533-538 |
| FND-003 | low | `AdmittedModel` now classifies systems kinds eagerly on every assembly, although only connection-end queries read it; previously the cost was paid on first use. Results are identical (same limits, fresh meter). | qsl-semantics/src/check/lowering/model.rs:872-880 |

## Verdict

Mergeable. No high or medium findings. Evidence on the trial merge with origin/main 3c12ee6dd: fmt --check exit 0; cargo check --locked --workspace --all-targets exit 0; clippy -D warnings default and --no-default-features exit 0; arch-lint qualified-core exit 0; arch-lint api-surface --qsl-only exit 0 (no FAIL); cargo test -p qsl-semantics --lib 775 passed, 0 failed, 1 ignored; --test it 522 passed, 0 failed. The diff adds no OnceLock, thread_local, static cell, `#[allow]` or production unwrap. The two restated assertions agree with the spec: FR-136-AC-6 and FR-135-AC-10 say nothing about a unit selecting no protocol profile, and FR-383-AC-17 keeps `missing-selection` at an await with no temporal profile, which the TC-663 test still asserts alongside the new `MissingProfile` at the undeclared alias `e`.
