---
id: SR-12816
title: "Code review (with rust-review) of QSL-550 hyper and relation forms (PR #971)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@6c7915ebf (qsl-550-e14fe-hyper, diff origin/main...HEAD); qsl-cst/src, qsl-forms/src, qsl-semantics/src/check, qsl-replay/src/spine/refinement.rs and their tests"
review_set: subset
---

## Summary

Ticket: QSL-550 (PR quire-spec-language#971). Code review with the
rust-review lane folded in, of the S1 grammar (BehavioursDomain, Quantifier
with `of` and per-variable fairness, AlignSkip, MatchBlock, StepLabel,
IndexedRead and the derived Hyper* expression tree), the S2 builders
(`qsl-forms/src/hyper_clause.rs`, the `holds(HyperExpression)` gate in
`temporal_clause.rs`, the value-mapping arms in `value.rs`) and the S3
`TemporalTrace` hyper family (`qsl-semantics/src/check/hyper_clause.rs`).

Checks run (own target dir): `cargo test -p qsl-cst -p qsl-forms --test it`
(59 and 113 passed); `cargo test -p qsl-semantics --features test-support
--test it` (987 passed, 1 failed: the known macOS-only
`model_identity_full_size::ten_thousand_edge_chain`); `cargo clippy` on the
four touched crates with `--all-targets -D warnings` (clean); `cargo fmt
--check` (clean); `cargo check --workspace --all-targets` (clean). Probe tests
were run in a scratch copy of the tree, not in the worktree.

## Verdict

PASS with medium findings; no high finding. The grammar keeps `@`, step
labels and `stutter` unreachable outside a `hyper` clause's `match` block and
`holds(...)` conditions. A `temporal` clause that writes one is refused
`UnrepresentedConstruct` at S2, and the Typer refuses
`OutsideHyperClause` as a backstop. The parser memoises per production and
position, so the derived Hyper* tree and the `holds(Expression) |
holds(HyperExpression)` alternative do not blow up on nesting. The HP-1..HP-6
classifier follows FR-173's order exactly, including the n+m >= 2 and
existential-fairness rules. Copy-swap detection matches FR-174's S3 rule
(same alias, equal fairness sets, normal form sorting `and`/`or`/`=`/`!=`,
flattening `and`/`or`, opaque nodes that mention a trace blocking a swap).

Author decisions: (1) a separate Hyper* tree derived by `copy_rule` (not
hand-copied) is sound. It is cheaper than admitting `@` in every expression
context and refusing it everywhere at S3. The merge hazard with #968 and #970
is textual only: `productions!`, the identical `recursion_limit = "256"`
line, the exhaustive `Production`, `DeclarationForm` and `CheckCause`
matches, and `check_limit` all fail to compile until reconciled, so nothing
merges silently. (2) A separate `CheckCause::Hyper` is acceptable. The codes
and tags match FR-172-AC-2, and fairness refusals reuse
`TemporalCause::UnresolvedFairnessOperation`, as the AC table's
`missing_declaration` expects. (3) `StepRead::Argument` is FND-005.

The ticket's "delete the FR-171 S2 depth cap" item has nothing to delete: no
such cap exists on origin/main.

## Examined Scope

```yaml
scope:
  - {id: FR-171, path: spec/functional/FR-171-build-forms-for-hyper-clauses-over-behaviours.md, role: examined, excerpt: "The S2 value form builder SHALL build a form for a hyper clause whose domain is behaviours and for a relation clause whose execution bindings name model operations"}
  - {id: FR-172, path: spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md, role: examined, excerpt: "bind each trace variable to a model alias, bind object parameters to population universes, type indexed atoms and step labels, split the match block ..., check each execution binding."}
  - {id: FR-173, path: spec/functional/FR-173-classify-hyper-clauses-into-forms.md, role: examined, excerpt: "S3 SHALL classify every checked hyper clause over behaviours and every relation over model executions into exactly one form, HP-1 to HP-6"}
  - {id: FR-174, path: spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md, role: examined, excerpt: "S3 SHALL test every pair of universal variables a, b of an HP-2 or HP-6 clause."}
  - {id: FR-180, path: spec/functional/FR-180-hand-over-a-step-relation-code-claim.md, role: examined, excerpt: "S3 SHALL record the code claim only when every execution variable's operation has an abstraction-relation binding"}
  - {id: ADR-023, path: spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md, role: context_only, excerpt: "HS-5 Input matching. An optional match { mu } block ... a.step.O.p is the argument p of operation O, of type Option<T> for p: T"}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `check_relation` never reads `form.body`, and `CheckedModelRelation` has no body field. FR-172 says the body "SHALL check under the value profile" and lists `body: CheckedExpression` in its Outputs. The author's scope limits do not list this. Probe: `relation Bad using v over (x: V::ConfigVersion::step, y: V::ConfigVersion::step) { x.nonexistent = zzz and q.r = 3 }` checks Ok, so it gets `NotEmitted` instead of `ill_typed`/`missing_declaration`. When S4 lands, nothing checked will exist to emit. | qsl-semantics/src/check/hyper_clause.rs:1996-2049 |
| FND-002 | medium | Duplicate names are accepted silently. `Variables::by_name` is a `BTreeMap` collected from the quantifiers, so the later quantifier wins. Probe: `forall trace a of V exists trace a of V { always holds(v.versionNumber @ a = 1) }` checks as `ForallExistsSafety`, with every `@ a` and `a.step` read bound to the existential. Duplicate object parameters (`(v: V::ConfigVersion, v: V::ConfigVersion)`) and duplicate execution variables (`(x: ..., x: ...)`) also check Ok. The `match` split and classification then rest on a shadowed binding. | qsl-semantics/src/check/hyper_clause.rs:812-817, 1075-1121, 2005-2032 |
| FND-003 | medium | Body atoms and `match` conjuncts are not typed beyond the trace binding, but FR-172 says S3 "SHALL type indexed atoms and step labels". Probes that all check Ok: `holds(w.versionNumber = 1)` (unknown root, not even refused as unindexed), `holds(zzz)`, `holds(v.nope @ a = 1)` (no such field), `holds(v.versionNumber @ a)` (non-Boolean), `match { a.step.op = a.step.receiver }` (operation compared with receiver key), and `match { a.step.op }` (non-Boolean conjunct). The `temporal` precedent also leaves `holds` conditions to FR-337, hence medium confidence, but FR-172 states the typing for hyper explicitly. | qsl-semantics/src/check/hyper_clause.rs:1178-1225, 1231-1339 |
| FND-004 | low | `SingleExistentialMatch` is located at the condition inside the braces (`block.spans.root()`), but FR-173 says "at the block". The test pins the inner span (`b.step.op = b.step.op`). | qsl-semantics/src/check/hyper_clause.rs:862-865; qsl-semantics/tests/it/hyper_clauses.rs:633-637 |
| FND-005 | low | `ArgumentRead::value_type` holds `T`, and the read's real type `Option<T>` (FR-172, ADR-023 HS-5) appears only in a doc comment. A later typer or evaluator reading `value_type` as the read's type would type `a.step.O.p = 1` as `T = Integer` and skip the absent case. Rename the field to `parameter_type` and add a `read_type()` that returns `Option<T>`. | qsl-semantics/src/check/hyper_clause.rs:146-157 |

## Coverage

Not applicable to this method; see SR-12817 (gap-analysis).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | The new body typer is a second, hand-written typer that is narrower than the value profile FR-172 names ("the body SHALL check under the value profile"), and at one point wider. It refuses valid bodies. With `enum Phase { OPEN, SHUT }` in the unit, `holds(Phase::OPEN = Phase::OPEN)` in a hyper body and `{ Phase::OPEN != Phase::SHUT }` in a relation body both refuse `missing_declaration` naming `Phase::OPEN`, because a qualified enum member is an `ExprNode::Name` that is not among the typer's roots. The cause is wrong too: a declared name is reported as missing. With a declared `function f`, `f(x.i) = f(y.i)` refuses `BodyUnsupported`, as do `abs(...)` and `let k = x.i in k = y.i`. In the other direction, `Ty::of` folds Integer, Int, Rational, Decimal, Float and Quantity into one `Numeric`, so a quantity can be compared with a bare integer, which the value typer would refuse. Fix: type the body through the existing value `Typer` (`check/check/typing.rs`), with the execution variables and object parameters bound as roots. Or admit enum members, declared calls and `let`, and keep numeric kinds and dimensions apart. | qsl-semantics/src/check/hyper_typing.rs:62-76, 150-157, 251-276 |
| FND-007 | low | `CheckedModelRelation::body()` returns the S2 `Expression`, not the `CheckedExpression` that FR-172's Outputs name, and the doc says S4 lowers it to FR-322 terms. That is acceptable while every clause is refused NotEmitted, but the Outputs now disagree with the code. Record the S4 typing as remaining work in FR-172, or amend the Outputs. | qsl-semantics/src/check/hyper_clause.rs:364, 427-432 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | check_relation types form.body through hyper_typing::type_condition and keeps it (CheckedModelRelation::body). Probe `{ x.nonexistent = zzz and q.r = 3 }` now refuses BodyMissing `nonexistent`; Det checks (26a263afd). |
| FND-002 | fixed | Duplicate trace variables, object parameters and execution variables refuse DuplicateName (conflicting_declaration/duplicate-declaration) at the second name. Probes `forall trace a ... exists trace a`, `(v: ..., v: ...)` and `(x: ..., x: ...)` all refuse (26a263afd). |
| FND-003 | fixed | Atoms and match conjuncts are typed. All six probes refuse: `w.versionNumber` and `zzz` BodyMissing, `v.nope @ a` BodyMissing, non-Boolean `holds(v.versionNumber @ a)` BodyIllTyped, `match { a.step.op = a.step.receiver }` and `match { a.step.op }` BodyIllTyped. Valid arithmetic, `if`, `not`, `stutter` and argument-pair bodies still check. The over-strict side is FND-006 (26a263afd). |
| FND-004 | fixed | The refusal is now at form.match_span, the whole `match { ... }` block (26a263afd). |
| FND-005 | fixed | The field is renamed `parameter_type`, and `ArgumentRead::read_type()` returns `ValueType::Option(T)`. The match typer types the label `Ty::Argument(T)`, which compares only with another argument, so `a.step.O.i = 1` refuses BodyIllTyped (26a263afd). |
