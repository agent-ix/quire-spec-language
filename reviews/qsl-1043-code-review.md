---
id: SR-17009
title: "Code and Rust review of QSL-1043 model_check setting names and FR-127-AC-7 test"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #1436 (branch qsl1043-max-states, head 98b0baeb3); qsl-replay/src/model_check/limits.rs, qsl-analyze/tests/it/closure_certificate.rs, qsl-analyze/tests/it/hyper_witness.rs, qsl-analyze/tests/it/model_settle.rs, qsl-analyze/tests/it/partial_order.rs, qsl-analyze/tests/it/state_graph.rs, qsl-analyze/tests/it/state_graph_settle.rs"
review_set: subset
---

## Summary

Ticket: QSL-1043. Review pass (round 0) at frozen head 98b0baeb3 against the
origin/main merge base. Method: code-review with the rust-review lane folded
in. The root crate on origin/main does not compile until PR #1438 lands, so
gates ran on the qsl-* crates only, in a detached worktree with its own target
dir: `cargo test -p qsl-replay -p qsl-analyze -p qsl-foundation` passes (0
failures; qsl-analyze `it` 749 passed); `cargo clippy -p qsl-replay -p
qsl-analyze --all-targets -- -D warnings` clean; `cargo fmt --check` clean.

Setting names: all seven renamed `ModelCheckLimit::setting()` arms
(`model_check.max_states`, `.max_transitions`, `.max_automaton_states`,
`.witness_samples`, `.max_walk_steps`, `.max_witness_set`,
`.max_relation_tuples`) match FR-255's engine table rows exactly, in spelling
and prefix. FR-127's map, FR-127-AC-1/4/7, TC-522 and TC-1161 use the same
names.

Consumers: the only production reader of `ModelCheckLimit::setting()` is
`ItemCause.setting` in qsl-replay/src/outcome.rs:601, which is meant to carry
the FR-255 name. `CallerLimits::from_operands` parses through qsl-foundation's
`Setting` table, which has no `model_check.*` rows before or after this change
(QSL-1026), so no parser compared the old bare names. No JSON fixture, golden
or doc in the repo holds a bare setting string. No consumer breaks.

Red check: two mutations re-run. Reverting `MaxStates` to `"max_states"` fails
the new test at closure_certificate.rs:902. Fixing `Row::bound`'s cancelled
source to `Requested` fails it at closure_certificate.rs:908 (the deadline
case).

## Verdict

**APPROVED (round 1).** Round 1 at ee2b4bd3d fixes FND-001 to FND-004 and
defers FND-005 to QSL-1050. Tests, clippy and fmt pass, and four mutations
turned the tests red.

Round 0: **CHANGES REQUESTED.** The rename is correct and complete for EN-1
and breaks no consumer. One medium finding remains: the FR-127-AC-7 test does
not run the vector the AC names. The low findings can be fixed in this PR or
deferred. FND-005 is latent and is best handled as a follow-up ticket.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-127-AC-7 test uses the one-counter `{c}` world and removes the state with model value 1. The AC names FR-338-AC-1/2's TP-1 vector, whose certificate has model state `(1, 0)` removed. The repo's own FR-338 vector tests (closure_certificate.rs:360-416) run that on the two-counter `{a, b}` world from `(0, 0)`. As written, the test repeats FR-127-AC-1's test fixture rather than the AC's vector | qsl-analyze/tests/it/closure_certificate.rs:819-866 |
| FND-002 | low | AC-7 states the record's label and basis: `proved`/`closed-scope` for the accepted certificate and `inconclusive`/`unsettled` for the rejected one. The tagged test asserts only `.value` for both. The FR-127-AC-1 test asserts them for the same fixture (closure_certificate.rs:744-757, 790-794), but the AC-7 binding leaves them unasserted | qsl-analyze/tests/it/closure_certificate.rs:836-866 |
| FND-003 | low | AC-7's "caller/deadline cancellation retains requested/deadline" is exercised only through the package emission in `Row::bound`. A pre-cancelled handle stops before `check_closure` runs, so the certificate check's own cancellation, and `Row::of_check`'s `Stopped(Cancelled)` arm, are reached by no settle-level test. This is why a mutation of the checker's cancel arm survived. The checker arm is covered directly by TC-525 (closure_certificate.rs:978-985), and the settled value the AC requires is asserted, so this is acceptable. The doc comment should say the stop happens at package binding | qsl-analyze/tests/it/closure_certificate.rs:877-911; qsl-replay/src/model_check/settle.rs:588-597,619-633 |
| FND-004 | low | Stale doc comment: "The request member or FR-255 accounting setting name that raises this limit". Every arm now returns an FR-255 dotted setting name (model_check, expansion, hybrid, accounting or statistical); none is a bare request member | qsl-replay/src/model_check/limits.rs:148 |
| FND-005 | low | EN-5's product build stops with `ModelCheckLimit::MaxStates`, `MaxTransitions` and `MaxAutomatonStates`, so after this rename those stops name `model_check.*`. FR-255 says EN-5's product bounds take `exact_prob.<member>` and are raised separately from EN-1's. The error is latent: `ProductStop` is not yet settled into a terminal record, so nothing emits the name today. It needs an EN-5 limit identity, or an engine-aware setting, before FR-203 settlement lands | qsl-analyze/src/exact_probabilistic/build.rs:113-114,365,491 |

## Dispositions

Round 1, verified at head ee2b4bd3d. `cargo test -p qsl-analyze --test it -- closure_certificate`
passes (19 tests); clippy `-D warnings` and fmt are clean. All four mutations
turned the tests red: `MaxStates` setting reverted (fails at line 912),
`limit_reached` dropped in `Row::of_check` (line 911), `Requested` reported as
`Deadline` in `Row::bound` (line 918), and `of_check`'s `Stopped(Cancelled)`
arm mapped to `failed` (the new cancellation test fails at line 998). The last
mutation shows the new test reaches the check-time cancel arm. `quire matrix`
binds FR-127-AC-7 to the AC-7 test at closure_certificate.rs:820.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The AC-7 test now runs on the two-counter `{a, b}` world and removes product state `(1, 0)` under instance `a`'s carried root (closure_certificate.rs:821-861) |
| FND-002 | fixed | The test asserts `(Proved, ClosedScope)` for the accepted certificate and `(Inconclusive, Unsettled)` for the rejected one (closure_certificate.rs:846-849, 878-881) |
| FND-003 | fixed | The new TC-522 test `a_cancellation_during_the_certificate_check_settles_cancelled` cancels at the settlement's last meter poll, after package emission, and asserts `Incomplete(Cancelled{source})` with no limit reached, for both `Requested` and `Deadline` |
| FND-004 | fixed | The `setting()` doc comment now lists the FR-255 row families (limits.rs:148-153) |
| FND-005 | deferred | Deferred to QSL-1050 (EN-5 exact-build stops must name `exact_prob.*`). EN-5 is unchanged here; a note on `setting()` records the gap |
