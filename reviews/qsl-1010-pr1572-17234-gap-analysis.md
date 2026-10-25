---
id: SR-17234
title: "Gap analysis of QSL-1010 PR B(b): FR-315-AC-7 to AC-14, AC-43 to AC-52, FR-487-AC-7 and AC-8"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@c8ea7587198f5b68eb1c209078a713d40bdbe925; PR #1572 (branch qsl-1010-encode-hyper-query), QSL-1010 PR B(b) and QSL-1083: spec/functional/FR-315-encode-the-smt-lib-transition-relation.md, spec/test-cases TC-1120, TC-1190, TC-1264, qsl-replay/tests/it/hyper_query.rs, qsl-replay/tests/it/script_search.rs, tests/fixtures/hyper-query-consumer"
review_set: subset
date: '2026-10-25'
---

## Summary

Ticket: QSL-1010 (QSL-1083). Planless run (Plan completion: not assessed). `quire matrix` at the head: FR-315-AC-7, 8, 9, 10, 12, 13, 43, FR-487-AC-7 and AC-8 tagged; AC-11 `method-without-symbol` (its verification method is Analysis, the test is a bonus); AC-14 **untagged**. The matrix tool lists no row for FR-315-AC-44 to AC-52 at all (the listing stops at AC-43), so those were checked by reading the tags: AC-46, 47, 48 (hyper_query.rs), 49, 51, 52 and 50 carry TC-1120, TC-1190 and TC-1264 tags that match their assertions. No stub or tautology found; AC-48's final loop asserts nothing (SR-17233 FND-002). Production code with no owning requirement: none (`CheckedFormula::is_safety`, `Components` and `StateCopy::hyper` are owned by FR-315 "Hyper self-composition queries" and FR-487). Gaps are below.

## Verdict

**CONDITIONAL** — FND-001 and FND-002 should be closed in this PR; the rest are follow-ups.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-315-AC-14 is untagged and cannot be satisfied as written. It names `HyperSmtOutcome` and `settle_hyper_smt`, which do not exist (FR-182 has not delivered them; FR-182-AC-12 is untagged too). The consumer crate compiles the half that exists (the types, `encode_hyper_query`, `HyperProductIdentity::of`) and the three `compile_fail` doctests cover the struct-literal/serialize/deserialize half, but neither carries a `#[trace]` tag, so the matrix reports the AC as unverified and a reader of the PR description could think it is covered. Split AC-14 (encoder half stays here, settle half moves to FR-182) and bind the half that is delivered with a tag the matrix reads. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:1242; tests/fixtures/hyper-query-consumer/src/lib.rs:1-45; qsl-replay/src/model_check/hyper_smt_query.rs:54-82 |
| FND-002 | medium | No test encodes a clause with more than one alias. `SavesPower` reaches only refusals; the fixture world cannot encode it (I tried: both subjects' snapshots hold both models' vaults, so the object named `v` appears twice and the encoder correctly refuses `constant-name`). The densification of trace variables to subject readings (`used`, `dense`), the per-component layouts in `Components`, and the slot-to-object binding for two different subjects are therefore never run, and the wrong-output risk of a mixed-up component sits exactly there. Scripts with a repeated alias (the vault) and two copies of one subject are verified (SR-17233); add a two-alias fixture with disjoint universes. | qsl-eval/src/smt/hyper.rs:125-151, 523-562; qsl-replay/tests/it/vault_world.rs |
| FND-003 | low | AC-47's headline variant (conjunct 2, `quire::value::ieee::sqrt` under `Float64[exact]`, kernel refusal `ieee_not_exact`) and the `MatchReading::Refused` arm are untested; S2 refuses float literals so the source cannot be written. Judged acceptable to defer: the arm shares the `refused()` closure with the `Err(MatchIncomplete)` arm, which the exhausting-function conjunct drives (same tuple, conjunct, site; byte-equal on repeat). But the test's doc comment says "a conjunct the kernel refuses" while exercising only the exhausting case. Amend AC-47 (or the comment) to say which half is verified where, and file a follow-up for a `qsl-eval` unit test of the `Refused` mapping. | qsl-replay/tests/it/hyper_query.rs:432-470; qsl-eval/src/smt/hyper.rs:335-337 |
| FND-004 | low | Hyper `too-large` refusals (`constants` counting every copy, `applications` counting joint disjuncts, `script-bytes`, `candidates`) have no test; AC-44 and AC-45 cover the single-subject encoder only. I confirmed the first three ad hoc (see SR-17233) and they behave as FR-315 states. The AC-46 test compares only the `WorkUnits` consumption and `admission_count()` of the meter with nine sequential `universal` calls, not every counter. | qsl-replay/tests/it/hyper_query.rs:563-600 |
| FND-005 | low | AC-11's second half asserts only that the leaky vault's base query is satisfiable; it does not check the joint step `(step(0), step(0))` or the initial tuple `((0, 0), (1, 0))` the AC names. The semantic cross-check in SR-17233 (96 cases) covers this by other means; the committed test would pass for a satisfiable query of any shape. | qsl-replay/tests/it/hyper_query.rs:244-270 |
| FND-006 | low | AC-12 and AC-46 say "the same request at instance `w`"; the AC-46 test builds a universe {w} for that, while AC-12's vault has {v, w}. With {v, w} each component has 4 operation disjuncts and the stutter (25 joint tuples before `mu_U`), not nine; the AC-46 count is only true for a one-object universe. See SR-17235 FND-005. | qsl-replay/tests/it/hyper_query.rs:566-570 |

## Coverage

Examined: FR-315 "Hyper self-composition queries" (every bullet), FR-315-AC-7 to AC-14, AC-43 to AC-52, FR-487-AC-7 and AC-8, against `hyper_query.rs`, `hyper_identity.rs`, `script_search.rs`, the goldens and the consumer crate; `quire matrix --format tsv`. The solver-free checker is exact for the scripts it is used on (`assert_unsat` requires every Integer to be ranged inside the searched 0..8). Independent semantic check: scratch explicit-state oracle, 96 cases, 0 mismatches (SR-17233).

## Dispositions

(none yet)
