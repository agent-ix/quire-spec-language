---
id: SR-2026111216592600000001
title: "code-review and rust-review of QSL-1891 and QSL-1885 (quire-spec-language PR 2691)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; qsl-semantics/src/check/protocol_controls.rs, qsl-semantics/src/check/refusal.rs, qsl-eval/src/simulation/protocol/steps.rs, qsl-eval/src/simulation/protocol/program.rs, qsl-eval/src/simulation/protocol.rs, qsl-eval/src/value/expression/mod.rs, test fixtures in qsl-semantics, qsl-package, qsl-analyze; spec/functional/FR-218 (PR 2691 diff, head df4efab42)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: references
---

# Code review of QSL-1891 and QSL-1885 (quire-spec-language PR 2691)

## Summary

Tickets: QSL-1891, QSL-1885. Head df4efab42, scoped to `git diff origin/main...HEAD`. The
rust-review lane is folded into this file.

QSL-1891 (S3 refuses a non-`all` join with no `outstanding` rule) is correct. `check_join`
pushes `invalid_package`/`invalid-value` at `parallel.join.span` with
`ProtocolAnchorCause::JoinList { fault: JoinFault::NoOutstandingRule }` before and independent
of the list checks. FR-219's weak-join refusal runs in its own pass, so a `memory tso` + `any`
join with no rule reports both, as FR-218 and FR-218-AC-40 require. A grep of qsl-semantics,
qsl-package, qsl-analyze, qsl-eval, tests/, examples/ and src/ finds no remaining non-`all`
join with no `outstanding` rule that reaches S3, other than the two deliberate negative cases.
The parser-level cases in qsl-cst and qsl-forms stop before S3. `Unsupported::JoinPredicate`
and `Unsupported::UnruledOutstanding` are removed with their `as_str` arms and leave no dead
references in code.

QSL-1885 (stepping `join predicate`) has two spec violations, FND-001 and FND-002.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `join_enabled` passes the predicate one Boolean per joined branch in branch declaration order (`join.branches` filtered by `joined`). FR-206 requires one argument per distinct joined label in ascending UTF-8 byte order. FR-218 "Predicate joins" and QSpec FR-171 say the same, and S4 binds that way (`an_out_of_order_predicate_join_binds_by_ascending_label`). For `branch b ... branch a ...` joined `predicate(f) [a, b]`, the evaluator gives `f` b's disposition first while the emitted package and IR give it a's. The tests do not catch this because every fixture declares a, b, c in alphabetical order. Fix: collect the distinct `join.joined` labels sorted by bytes (for example, a `BTreeSet<&str>`) and map each to its disposition. Add a fixture whose branches are declared out of order. | qsl-eval/src/simulation/protocol/steps.rs:1137-1145; spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md:96-99; spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:242-243 |
| FND-002 | high | Every predicate outcome other than `Completed(Boolean)` becomes `ProtocolRefusal::fault("a-join-predicate-is-boolean")`, which `From<ProtocolRefusal> for ExpansionStop` turns into `runtime_invariant`/`established-invariant-broken`. That covers undefined, refused and meter-exhausted (`Incomplete`) outcomes. FR-206 decides the coder's open question. An undefined result SHALL end the check with a violation with cause `UndefinedEvaluation` (QSpec FR-426) at the join's position, never `false`. An evaluation a resource limit stops SHALL stop with that limit's refusal (`resource_exhausted`/`insufficient-next-charge`). Neither is an internal fault. The choice also departs from how guards are handled: `decide` (settle.rs) and channel keys map the same outcomes through `verdict()` to `ModelFinding::ProtocolExpressionUndetermined`, the carrier `undefined_protocol_expression()` reads as FR-426's undefined protocol expression. Fix: read the outcome through `verdict()`. On `Undecided`, return the `undetermined(text, outcome)` finding to the caller (steps.rs:231 has `gathered.findings`), as guards do. Stop with `EXHAUSTED` on `Incomplete` if FR-206's resource-limit clause is applied literally. | qsl-eval/src/simulation/protocol/steps.rs:1146-1157; qsl-eval/src/simulation/protocol/settle.rs:249-265; qsl-eval/src/simulation/protocol.rs:1771-1784; spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md:100-108 |
| FND-003 | low | `policy_holds` keeps `CheckedJoinPolicy::Predicate { .. } => false`, now unreachable because `join_enabled` returns first for predicates. A future caller would get a silent `false`, not a fault. Fix: decide predicates inside one function, or make the arm unreachable by construction (for example, pass the completed count only for non-predicate policies). | qsl-eval/src/simulation/protocol/steps.rs:103-112 |

## Verdict

QSL-1891 is clean and mergeable. QSL-1885 is not: FND-001 (wrong argument order, against
three specs and S4) and FND-002 (an undefined or exhausted predicate is reported as a broken
internal invariant, against FR-206 and the guard precedent) must be fixed in this PR.
`evaluate_join_predicate` is sound. It resolves the function by identity, runs it with an
empty object environment and its formed units, and charges a fresh meter per evaluation, as
guards do. FR-216-AC-8's footprint needs no code change: the join footprint never reads the
function's model state, and the new test confirms this.
