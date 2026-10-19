---
id: SR-12187
title: "Code review of quire-spec-language PR #1027: S3 strong fairness and the unmarked kind over the mutex (FR-129, TC-530)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@355e95b6dcf5beee59a54379a49ce5c939070c4c; PR #1027 diff against origin/main: qsl-semantics/tests/it/strong_fairness.rs (new), qsl-semantics/tests/it/main.rs; read for context: qsl-semantics/src/check/temporal_clause.rs (checked_fairness, checked_constraint), qsl-semantics/src/check/lowering/temporal.rs (fairness node lowering), qsl-semantics/src/check/mod.rs (TemporalClauseNode), qsl-semantics/tests/it/temporal_fairness.rs, qsl-semantics/tests/it/model_operations.rs; spec FR-129, TC-530, ADR-019 sections 1 and 6"
review_set: subset
---
# Code review of quire-spec-language PR #1027

## Summary

Ticket: QSL-532 (partial: E10a, FR-129 at S3 only). One code-review pass
with the rust-review lane folded in, over `git diff origin/main...HEAD`
(2 files, +316). Tests only: one new integration module
`qsl-semantics/tests/it/strong_fairness.rs`, registered under
`#[cfg(feature = "test-support")]` in `tests/it/main.rs`. Read-only: nothing
was built or run, at the dispatcher's instruction.

Checked and clean:

- FR-129-AC-1, test `an_unmarked_kind_is_weak_and_the_kind_enters_the_identity`.
  It asserts `fair acquire;` checks to `[(Weak, acquire, Whole)]` and equals
  the set from `fair weak whole acquire;`; `strong` to `(Strong, acquire,
  Whole)`; `strong each` to `(Strong, acquire, Each)`; the unmarked and
  `weak whole` clauses have equal node identities; `strong each` and
  `weak each` (and `strong` and `weak`) have different ones. Each AC clause
  has an assertion that fails when the behaviour is wrong: an unmarked kind
  read as strong, a strong kind read as weak, or a wrong default granularity
  all change `members`.
- The identity assertions are meaningful. The node identity is
  `lowered.key` (`check/mod.rs` `TemporalClauseNode`), a structural key of
  the `temporal_clause` node whose fairness group references one `fairness`
  node per member carrying `Member::Fairness { kind, granularity,
  declaration, name }` (`check/lowering/temporal.rs:133-160`). Source spans
  are recorded as occurrences, not keyed. The `assert_eq!` between `fair
  acquire;` and `fair weak whole acquire;` passes only because spans and
  source text are outside the key (the two bodies differ in length). So the
  `assert_ne!` between `strong each` and `weak each`, whose bodies differ
  only in the kind keyword, fails if `kind` is dropped from
  `Member::Fairness`: nothing else differs between the two keys.
- FR-129-AC-2, test `weak_and_strong_stay_apart_and_strong_needs_an_infinite_trace`.
  Both source orders of `strong each` + `weak each` are checked, so a set
  sorted by kind or a set deduplicated without the kind fails. `strong`
  written twice checks to length 1, which fails if deduplication is removed.
  The event-position case asserts the exact
  `TemporalCause::FairnessUnderBoundedProfile { span }` with the span of
  `fair strong Config::Mutex::acquire;`, plus `Code::UnsupportedConstruct` and
  tag `expression-form`; it fails if a strong constraint were admitted under
  a bounded profile or refused at another span. The span convention matches
  FR-123's weak case (`temporal_fairness.rs` TC-518).
- Trace tags: two tests, each tagged with exactly the one AC it asserts,
  `#[trace("TC-530", "FR-129-AC-n")]` as TC-530 prescribes. Not over-tagged:
  each tagged test fails on its own AC's behaviour.
- Fixture fidelity to ADR-019 section 6: object type `Mutex` with
  `owner: Int[0, 2]` (value type `Owner`, min 0, max 2), population `locks`
  (closed, member `Mutex`), `acquire(p: Int[1, 2])` (value type `Process`,
  required, multiplicity 1..1) and `release()`, both with frame
  `modifies [owner]` and no result. Declared departures: no pre/post
  contracts (the module doc says so; S3 reads declarations only, which
  `checked_fairness` confirms), and the model alias `Config` for ADR-019's
  `M`, which the shared `config_unit_with_profiles` helper fixes.
- Rust lane: no `unsafe`, no production code, `panic!`/`expect` only in
  test helpers with a message naming the body, no integer conversions beyond
  `start + constraint.len()` over a found index. The import of `meaning`
  after the `crate::` group is rustfmt-accepted.

## Verdict

PASS. The tests exercise FR-129-AC-1 and FR-129-AC-2 as the spec states
them, the identity assertions fail if the kind were dropped from the node
identity, and the mutex fixture is faithful with its departures declared.
Two low findings are recorded, not blocking. Declared remainder, not
reviewed here: FR-130 (TC-531), FR-131 (TC-532) and UnfairLasso settlement,
blocked on E9 (QSL-530) and ticketed as QSL-896; FR-130-AC-4/5 also need
QSL-893.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new module copies its helpers from `temporal_fairness.rs`: `profile_header`, `clause_body`, `check_in` and `node_identity` are the same bodies with the document fixed to the mutex, and `bound_integer`'s constraint closure repeats `version_number_value_type`'s in `model_operations.rs`. Parameterising the existing helpers by document (or moving them to `model_operations`) would keep one copy of the check harness. | qsl-semantics/tests/it/strong_fairness.rs:31-66; qsl-semantics/tests/it/strong_fairness.rs:148-207; qsl-semantics/tests/it/temporal_fairness.rs:48-90 |
| FND-002 | low | Two departures from TC-530's procedure are not stated in the file: the alias `Config::Mutex` for ADR-019's `M::Mutex`, and the event-position case using `eventually[0,5] holds(m.owner = 1)` instead of TC-530's `always eventually holds(m.owner = 1)` (needed, since an unbounded operator would be refused under a bounded profile too). The module doc names only the missing contracts. | qsl-semantics/tests/it/strong_fairness.rs:1-7; qsl-semantics/tests/it/strong_fairness.rs:291 |
