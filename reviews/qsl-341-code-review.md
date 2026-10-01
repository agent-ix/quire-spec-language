---
id: SR-912
title: "QSL-341 code review of PR 543 (subtype admission for Reference<T> under QSpec FR-151)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@d0a4b6efe5007b0892ff8a01ba85ba2f22adf815; qsl-semantics/src/model/observation/document.rs; qsl-semantics/src/model/observation.rs; qsl-semantics/src/model/object_environment.rs; qsl-semantics/src/value/declaration.rs; qsl-semantics/tests/it/state_clauses.rs; qsl-semantics/src/value/containment.rs (read, context); qsl-semantics/src/check/check.rs and check/typing.rs (read, context); quire-exact/src/value.rs and collection.rs (read, context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
---
## Summary

Ticket: QSL-341. PR: quire-spec-language#543 at d0a4b6ef, diff
`origin/main...HEAD` (one commit). Code review with the rust-review lane
folded in. Gate: the coder's `make ci` log for d0a4b6ef ends
`head=d0a4b6efe5007b0892ff8a01ba85ba2f22adf815 exit=0`. The reviewer did not
re-run it.

What was checked:

- **References::resolve.** It now calls `TypeEnvironment::conforms(target.object_type(), declared)`.
  That is the existing FR-151 check: reflexive, or `declared` is a proper
  ancestor in the admitted supertypes graph. `target.object_type()` is the
  object's effective type, taken from the snapshot's own `type` member. This is
  the most-specific type the ruling names. The coder's claim holds.
- **fill_slots.** It now goes through the new `pub(crate) TypeEnvironment::admits`.
  That function sends a top-level `Reference` pair to `conforms` and every
  other pair to `ValueType::admits`. Without it, `ObjectEnvironment::new`
  would refuse a Sub object in a `Reference<ConfigVersion>` field after check
  6.5 had admitted it. `map_environment_refusal` would then turn that into the
  fault `object-environment-refused-after-admission-checks`. `TypeEnvironment::record`
  shares `fill_slots`, so it now admits by conformance too. The coder's claim
  holds.
- **Mutation checks**, reviewer runs on the focused `qsl-semantics --test it`
  binary. Each mutant was restored afterwards and `git status --porcelain` was
  clean:
  - M1: `resolve` uses exact equality. Two tests fail: the field-subtype test
    and the parameter-subtype test.
  - M2: `resolve` always admits. Two tests fail: the unrelated-type and the
    supertype-object tests. A full run of the `it` binary had 242 tests pass
    and 2 fail.
  - M3: `TypeEnvironment::admits` uses exact equality for references. The
    field-subtype test fails.
  - M4: `conforms` has its arguments reversed. Three tests fail.
  - Every mutant was killed.
- **Open question 2 (nested references).** No admission path is reachable
  where an outer exact check refuses a subtype element:
  - An optional object field unwraps to a bare `Reference`, which `resolve`
    admits by conformance and `fill_slots` checks through `admits`.
  - A sequence field admits each element through `resolve`.
    `quire_exact::from_admitted` then stamps the *declared* collection type.
    `ValueType::admits` on a collection compares only that type, so it passes.
  - Set, bag and ordered-set fields refuse earlier at check 6.2.
  - A parameter or result of type `Option`/`Collection` passes
    `field_kind_matches`, but `admit_scalar` has no arm for it, so it always
    refuses `wrong-value-kind`, whatever its element type. That limitation was
    there before this PR and is not a subtype problem.
  - So there is no finding and no test is needed for question 2.
- **Program-evaluation paths.** The checker types record-literal fields and
  tuple arguments with `Expected::Checked` (`coerce`). That refuses a
  `Reference<Sub>` where `Reference<ConfigVersion>` is required. Only call
  arguments upcast (`Expected::Argument`). `evaluate_record`/`evaluate_tuple`'s
  exact `admitted` check and the evaluator's `tuple` call therefore agree with
  the checker. This PR does not change that.
- **Rust idioms.** No new `unwrap`/`expect`/`unsafe` in shipped code. The new
  `types` field on `References` is a borrowed `&'a TypeEnvironment`. `admits`
  is `pub(crate)`. Doc comments were updated with the behaviour.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `TypeEnvironment::record` now admits a Sub reference into a `Reference<ConfigVersion>` field, through `fill_slots` and `admits`. Its sibling `TypeEnvironment::tuple` still uses exact `ValueType::admits` and refuses the same value in a `Reference<ConfigVersion>` position with `TypeMismatch`. Both are public, and the public FR-143 `ValueGraph::build` (`containment.rs`) reaches both with a `GraphSlot::Value(Value::Reference(..))`. So the same FR-151 value is admitted as a record field and refused as a tuple position. This is the same bug class the PR fixes. The fix is one line, `!self.admits(value_type, value)` at declaration.rs:1754, plus a unit test that builds a record and a tuple each holding a Sub reference (admits) and an unrelated reference (refuses `TypeMismatch`). | qsl-semantics/src/value/declaration.rs:1745-1762, qsl-semantics/src/value/containment.rs:180-202 |
| FND-002 | low | Weak oracles in the two admit tests. `a_subtype_object_binds_a_supertype_parameter` checks only `assert_ne!(target.object_type(), self_object.object_type())`. That passes for any type other than ConfigVersion, but the doc comment promises the type is `Sub`. `a_field_reference_to_an_admitted_subtype_object_admits` checks only `result.expect(..)`. It never checks that `child.parent` admitted as `k` with type `Sub`. Assert equality with Sub's effective id in both. | qsl-semantics/tests/it/state_clauses.rs:2157-2161, qsl-semantics/tests/it/state_clauses.rs:4059 |

## Verdict

The fix is correct. `References::resolve` and `fill_slots` both decide by
FR-151 conformance. Every mutant of the fix is killed, and the three tests
the ruling requires exist and pass. Open question 2 is unreachable (see
Summary). Open question 1 is FND-001: the same rule, a one-line fix, and in
scope because this PR is what made `record` and `tuple` diverge. FND-001
should be fixed in this PR.

## Dispositions

Round 1, reviewed at 7ba3a212430669eca995281d090a319420cbc4fe (fix commit 7ba3a212 over the rebased 877d9701; content diff d0a4b6ef..7ba3a212 restricted to qsl-semantics and spec, minus main's #542 changes). Coder's make ci on 7ba3a212: exit 0 (not re-run). Reviewer focused runs: qsl-semantics --features test-support --test it, 446 tests pass; mutants re-run (tuple exact, resolve always-admit, resolve exact, admits exact) each killed.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 7ba3a212: `TypeEnvironment::tuple` uses `!self.admits(value_type, value)` (declaration.rs:1754); `type_environment_model::record_and_tuple_admit_a_subtype_reference_by_conformance` builds a record and a tuple each admitting a Sub reference and refusing an unrelated one with `TypeMismatch`; reverting the line fails that test (reviewer mutant, test-support build) |
| FND-002 | fixed | 7ba3a212: both admit tests assert `environment.find(target.universe(), "k") == Some(target)`; `ObjectReference` derives `PartialEq` over its identity triple, type included, so this pins the target to archive's `k` with its own type `Sub`; the field test now reads `child.parent` from the admitted environment (state_clauses.rs:2157-2162, 4069-4096) |
