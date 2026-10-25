---
id: SR-17130
title: "Code review of PR #1511: FR-306-AC-3/4 inertness tests and FR-185-AC-1/2 tags (QSL-513, QSL-920, QSL-546), code-review with rust-review lane"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@e1fdc52178571c7e371769987290b387a5cc7a78; git diff origin/main...e1fdc5217 (PR #1511, branch qsl-tests-546-513-920): tests/it/abstraction_relation_inert.rs, tests/it/main.rs, qsl-semantics/tests/it/probabilistic_declarations.rs; context qsl-route/src/request.rs, qsl-route/src/routing.rs, qsl-replay/src/spine/clause.rs, qsl-semantics/src/check/mod.rs, examples/config-version/spine.rs"
review_set: subset
date: '2026-10-25'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-306
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-805
    type: reviews
---
## Summary

Tickets: QSL-513, QSL-920, QSL-546. PR: quire-spec-language#1511. Test-only
diff; the rust-review lane is folded into this file.

**FR-306-AC-3 (`a_relation_changes_no_requirement_record_or_route_result`).**
Compiles FR-108's ConfigVersion unit with and without FR-304-AC-1's relation
appended, asserts the relation reaches the graph (3 bindings), at least three
requirement records, `with.requirements() == without.requirements()`, and
equal `route` results over one registry built per record kind. The route
comparison goes through `items_from_requirements` + `route`, the same
production API the other route tests use; `route` takes only the records and
the registry, so its equality follows from record equality, which is what the
AC states. Reviewer mutation: removing one requirement record whenever a
relation is checked (qsl-semantics/src/check/mod.rs, after
`check_declared_abstraction`) is killed at line 128.

**FR-306-AC-4 (`a_relation_changes_no_clause_disposition`).** Runs every
FR-108 case through `run_clause` with the program bytes as written and with
the relation appended. Reviewer probe: 16 of 17 cases settle past compile
(Evaluate or Admit); only `missing-model` is compile-stage, compared by stage
and category, which is justified by the source digest in the refusal. The
non-compile dispositions are compared whole (Debug). Reviewer mutation:
`run_clause` returning `MissingName` when the checked package carries a
relation is killed at line 173. The test does not confirm the carried run
compiled the relation (FND-002).

**FR-185-AC-1.** The reward value type is now pinned to `Int[1, 12]`; d's
support and probabilities, outcome's probabilities and position, and
Steady's weights are asserted. Outcome's support *values* are not (FND-001).

**FR-185-AC-2 across four tests.** Every clause is asserted by one of the
four tagged tests: weight `0` and `rational(-1, 1)` `invalid-weight`, `1`
twice `repeated-support-value`, `true` `value-outside-type`, each located and
naming `d` (`random_parameter_variants_refuse_naming_the_parameter`); the
second distribution `duplicate-distribution` naming `d` (same test); the
precondition read `random-read-in-precondition` with the full cause
(`a_precondition_that_reads_a_random_parameter_refuses_at_the_read`); bare
`-1` `invalid_syntax` at S1 (`a_bare_negative_weight_is_refused_at_s1`); the
four over-ceiling values `StageLimitExceeded`/`rational-bits-exceeded` with
setting `s3.rational_bits` and limit 65,536, and the five admitted values with
their reduced values (`exact_values_are_bounded_as_written`). No overclaim:
the union covers the AC and no tagged test claims a clause it does not
assert.

**Not-covered calls (QSL-546 and siblings), checked against the code.**
FR-304-AC-6: qsl-semantics/src/check/abstraction.rs:16-19 states it checks no
Rust syntax and no `9lives`/`type` segment refusal exists anywhere; correct
not to tag. FR-185-AC-9: no reference-literal form exists in qsl-cst,
qsl-forms or qsl-semantics; correct. FR-139-AC-6: the `Int[2, 4]` row is
refused at S3 `ill_typed`/`type-mismatch` with no `R::slots` or universe size
in the cause (qsl-eval/tests/it/refinement_populations.rs:434-461); correct.
Tracking of these is covered in the gap-analysis SR.

**Style and gates.** `#[trace("TC-..", "FR-..-AC-n")]` plain style on every
new or changed tag; no `/// Trace:` lines; no tautologies (expected values
are literals or come from the plain compile, never from the value under
test). Reviewer gates at e1fdc5217 in a private worktree, separate
CARGO_TARGET_DIR per run under ~/.cache, all under `flock
/tmp/qsl-cargo.lock`: `cargo test --test it abstraction_relation_inert` 2
passed; `cargo test -p qsl-semantics --test it probabilistic_declarations` 52
passed; `cargo fmt --all -- --check` clean; `cargo clippy -p
quire-spec-language -p qsl-semantics --tests -- -D warnings` clean.

## Verdict

Request changes (small). The FR-306 tests are sound and both killed a
production mutation; the FR-185-AC-2 spread is complete and honest. The
FR-185-AC-1 tag overclaims one clause (FND-001, medium): swapping `Ok` and
`Fail` in the fixture still passes. FND-002 and FND-003 are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `the_service_model_checks_and_records_its_declarations` is now tagged FR-185-AC-1, whose text requires "`outcome` with `Ok` at `49/50` and `Fail` at `1/50`", but it asserts only outcome's probability list `[49/50, 1/50]` and position, never its support values. Reviewer mutation: `OUTCOME` rewritten to `{ Service::Outcome::Fail: 98, Service::Outcome::Ok: 2 }` passes the test. Fix: assert outcome's support values equal the `Ok` then `Fail` variant literals, as `d`'s values are asserted. | qsl-semantics/tests/it/probabilistic_declarations.rs:426-434 |
| FND-002 | low | `a_relation_changes_no_clause_disposition` never checks that the carried run compiled the relation, so a run that drops or ignores the appended declaration passes. Reviewer mutation: `*bytes = text.into_bytes()` (no relation) passes both tests. The AC-3 test checks `with.abstraction()`; this one has no analogue. Fix: for non-compile cases assert `carried.package_id != plain.package_id` (observed different in all 16), which FR-306's "the relation enters the `package_id`" guarantees. | tests/it/abstraction_relation_inert.rs:145-183 |
| FND-003 | low | The module doc still says "That is why FR-185-AC-1 and FR-185-AC-2, which name the quantity type, are not tagged here", which now contradicts the tags this PR adds (both ACs now name `Millis = Int[1, 12]`). Fix: drop or reword the sentence. | qsl-semantics/tests/it/probabilistic_declarations.rs:13-15 |

## Dispositions

Round 1, verified at PR #1511 head 2963ff6d6 (fix commit d4a6c26c4; the other non-test changes in the range come from the merge of origin/main, QSL-894). Reviewer gates at the head with a private CARGO_TARGET_DIR under ~/.cache, under `flock /tmp/qsl-cargo.lock`: `abstraction_relation_inert` 2 passed, `probabilistic_declarations` 52 passed, fmt and clippy (`-p quire-spec-language -p qsl-semantics --tests -D warnings`) clean. Re-run mutations: the Ok/Fail swap in `OUTCOME` now fails at probabilistic_declarations.rs:445; appending no relation in the AC-4 test now fails at abstraction_relation_inert.rs:173.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Outcome's support is asserted as ordered (member, probability) pairs: the first member has canonical rank 1 (`Ok`) at 49/50, the second rank 0 (`Fail`) at 1/50. Per ADR-013 O-14 an enum value is its `VariantId` plus its rank in the canonical member list, so the rank identifies the member within `Outcome`, and the swap mutant is killed. Residual, not blocking: the check matches `rank: N` in the `Debug` text and hard-codes the rank observed at check time; comparing to `EnumMember::new(mint_variant_id(<Outcome key>, "Ok"), 1)` would make it name-level. |
| FND-002 | fixed | `assert_ne!(carried.package_id, plain.package_id)` for every case that gets past compile (abstraction_relation_inert.rs:173-178). |
| FND-003 | fixed | The module doc now says FR-185-AC-1/2 are tagged on the `Millis` form of the checks (probabilistic_declarations.rs:13-15). |
