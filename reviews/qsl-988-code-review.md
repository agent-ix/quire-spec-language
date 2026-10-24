---
id: SR-17005
title: "Code review of quire-spec-language PR #1433: HP-4 settlement at negotiation and hyper test and tag gaps"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@23597d9fdbb46f1033f808c8bdd810aab973b3f5; PR #1433 (QSL-988) diff origin/main...HEAD: qsl-replay/src/model_check/hyper_settle.rs, qsl-replay/src/proof_result.rs, qsl-replay/src/outcome.rs, qsl-replay/src/lib.rs, qsl-semantics/src/check/hyper_clause.rs, qsl-semantics/tests/it/hyper_clauses.rs, qsl-analyze/tests/it/hyper.rs, hyper_projected.rs, hyper_settle.rs, hyper_swap.rs, hyper_witness.rs, hyper_world.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1433

## Summary

Ticket: QSL-988. This code review folds in the rust-review lane. It covers the
reviewed head 23597d9fd.

Production change: `settle_hyper_form(&HyperForm) -> Option<TerminalValue>`
has a total match with no wildcard. `HyperForm::Other { reason }` maps to
`Unsupported(UnsupportedHyperForm { reason })` and every admitted form to
`None`. The function has no unwrap, expect, indexing or arithmetic, so it
cannot panic. The new `UnavailabilityCause::UnsupportedHyperForm` has wire
cause `unsupported-requested-capability` and code `unsupported_projection`,
matched in `as_str`, `code` and `OutcomeItem`. `Hash` on `HyperUnsupported`
is needed by `UnavailabilityCause`'s `Hash` derive. `settle_hyper_form` has
no production caller yet. It is public groundwork, and the CG-side wiring is
still to be ticketed.

Each tag was compared, clause by clause, with the AC text on main (spec/ is
unchanged in this PR):

- FR-174-AC-5: complete. The first clause asserts stored whole, 4 states
  equal to the unreduced count, `Violated` (which names no reduction) and a
  fair replay. The second clause asserts `CopySwap{[[c, d]]}` alone. The
  helper checks only that the copy group is non-empty, which with two
  variables is `[[a, b]]`.
- FR-174-AC-6: complete. All three sentences are asserted, and the third
  (two classes, one acceptance-exchanging) is new. Reviewer check: the same
  four-copy clause with both classes acceptance-preserving does return
  `HoldsReduced`, so the third sentence's whole storage is caused by the
  failing class and is not vacuous.
- FR-177-AC-7: complete. Both sentences are in two helpers that one tagged
  test calls.
- FR-178-AC-7: complete. The test asserts `Holds` (the exhaustive variant),
  so `VacuousMatch` is excluded, and the leaky-vault control shows that the
  empty `match` decides the result.
- FR-178-AC-8: complete. Both sentences are in two helpers.
- FR-178-AC-9: complete (retagged from AC-5). FR-178-AC-5 is still tagged on
  `an_undefined_match_ends_the_paths_at_the_first_such_visible_step`.
- FR-183-AC-7: complete. The test asserts the code, the cause, the
  skipped-`mix` and visible `step(0)` paths, and the `HyperProjected`
  refusal (not a lockstep replay).
- FR-182-AC-9: complete in substance. FND-002 is a low-confidence
  antecedent question.
- FR-173-AC-2: partial. See FND-001.

Merged helpers: every `assert_*` helper is called from exactly one tagged
`#[test]` and runs unconditionally. Reviewer mutations confirmed that each
clause can fail on its own (see `## Verdict`).

No doc-comment or trace attribute names the deliberately untagged ACs
(FR-182-AC-3, FR-174-AC-8, FR-176-AC-1, FR-177-AC-8..12). This was checked
by grep and by `quire matrix`: all six are `untagged`. The stale
`FR-173-AC-2` doc id on the qsl-semantics classification test was removed.

`hyper_world.rs` fixture: `with_second_model` now also writes the second
model's population into each snapshot. Its only caller (the `SavesPower`
key-symmetry case in hyper_swap.rs) settles before expansion, so this is
safe. `second[index]` and `bounded`/`other_universe` are test-only. The
`["false"] => false` seam arm is reached only by the new FR-178-AC-7 test.

Gates run at the reviewed head: `cargo test -p qsl-analyze --test it hyper`
passed 174 tests, including the reviewer's temporary per-helper probes.
`cargo clippy -p qsl-replay -p qsl-semantics -p qsl-analyze --all-targets --
-D warnings` was clean, and so was `cargo run -p xtask -- string-edge`. On
GitHub, the `cla-check` job fails while `cla / gate` passes. No other CI job
runs cargo.

## Verdict

Changes requested on one medium finding (FND-001). The rest of the change is
correct.

Reviewer mutations (each one was killed by exactly the expected tests):

- Rejected rank 0 to 1. This killed FR-178-AC-8 and its second-sentence
  helper.
- MatchAt rank 0 to 1. This killed FR-178-AC-8 and its first-sentence
  helper. FR-178-AC-8 was not mutation-tested by the coder.
- `sooner` keeps the first refusal in the list. This killed FR-177-AC-7 and
  its second-sentence helper.
- The coder's "swap always applied" mutation was re-run. It killed
  FR-174-AC-6 and the third-sentence helper alone.
- The coder's "settle_hyper_form never settles" mutation was re-run. It
  killed FR-173-AC-2.
- The coder's "HyperProjected cause changed" mutation was re-run. It killed
  FR-183-AC-7.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-173-AC-2 tag claims the whole AC, but its last clause ("Each settles V-8 `unsupported-requested-capability` in negotiation with no engine run") is not exercised. The test calls the pure function `settle_hyper_form` directly. No negotiation path calls it: qsl-route has no form routing, and the engine still refuses `Other` with `HyperRefusal::FormNotDecided`. The PR body itself lists "FR-173-AC-2 negotiation/routing: CG-side" under "Not tagged". The repo's convention tags only complete ACs (FR-176-AC-1 is left untagged for one clause). The hybrid precedent (FR-250-AC-1) settles through real routing. The other clauses (classified `Other` with the rule named; V-8 `unsupported-requested-capability`) are complete. "Naming its rule" binds to the classification, as FR-173 says ("`Other` SHALL carry which rule"). `ItemCause` dropping the rule on the wire is therefore not a gap against this AC: the `TerminalValue` keeps it, as do the other unsupported causes. Fix: keep `TC-598` and drop `FR-173-AC-2` from the trace, then file the CG-side wiring ticket. Alternatively, the lead accepts the partial tag explicitly, with that ticket named in the doc comment. | qsl-analyze/tests/it/hyper_settle.rs:573-577; qsl-replay/src/model_check/hyper_settle.rs:79-94; qsl-analyze/src/model_check/hyper.rs:240-248 |
| FND-002 | low | FR-182-AC-9's last clause, "the same clause over a population with no universe is `RequiresBound`", most naturally refers to the `SavesPower` clause just named. The test asserts it with the one-parameter `NonInterference` instead. Either reading is defensible, so the confidence is low. Asserting `RequiresBound` for `SavesPower` too would close the ambiguity. | qsl-analyze/tests/it/hyper_settle.rs:696-706 |
| FND-003 | low | The hyper_settle.rs module doc still lists FR-182-AC-3 under "Not claimed here", "whose `proved` rows need FR-163's ... certificate", the SMT row and the HP-1 rows. With the "HP-4 negotiation rows" reason removed, none of the listed reasons fits AC-3. AC-3 has no `proved` row: it is VacuousMatch, max_witness_set, max_relation_tuples (HP-1) and HP-4 (FND-001). State AC-3's actual reason. | qsl-analyze/tests/it/hyper_settle.rs:8-11 |
