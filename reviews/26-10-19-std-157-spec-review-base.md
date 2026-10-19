---
id: SR-13264
title: "Spec review of QSL PR #1039 (STD-157): FR-381-AC-10 expected-type rule"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@a6f4a796f; spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md, spec/test-cases/TC-1058-s3-checks-compensation-registration-retry-commit-and-recovery.md; compared against agent-ix/quire-specification@2c2918b spec/functional/type-model/FR-149-apply-complete-equality-matrix.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-381
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1058
    type: reviews
---

## Summary

Ticket: STD-157 (QSpec-side, Done; no QSL-side ticket exists for this PR). The PR adds an FR-381 behaviour bullet restating QSpec FR-149's expected-type rule ("Arithmetic operand against a bounded integer"), rewords FR-381-AC-10 and adds TC-1058 row 16a for unary `-`. The core rule matches FR-149: which node gets the expected type (binary `+`/`-`/`*` over `Integer`/`Int[..]` operands, parentheses ignored, either side), the obligation (FR-146 interval inside `[lo,hi]` under the facts where it executes), the refusal (`undefined_expression`/`unproved-range` at the arithmetic node, not the equality), and no clamp/wrap/widen. The AC-10 interval arithmetic is right (`earlier.tally < 3` gives `[0,2]`, the sum `[1,3]` inside `Int[0,3]`; without the guard `[1,4]`), and TC-1058 rows 16 and 22 already carry the unguarded and recovery-predicate refusals. Four defects: the row 16a span, an overstated no-coercion list, a no-coercion list narrower than FR-149's, and the rule's scope being tied to protocol positions.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-381 (new expected-type bullet) | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md | examined |
| FR-381-AC-10 | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md | examined |
| FR-381-AC-12 | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md | context_only |
| TC-1058 row 16 | spec/test-cases/TC-1058-s3-checks-compensation-registration-retry-commit-and-recovery.md | examined |
| TC-1058 row 16a | spec/test-cases/TC-1058-s3-checks-compensation-registration-retry-commit-and-recovery.md | examined |
| TC-1058 row 22 | spec/test-cases/TC-1058-s3-checks-compensation-registration-retry-commit-and-recovery.md | examined |
| QSpec FR-149 "Arithmetic operand against a bounded integer", FR-149-AC-19..21 | quire-specification spec/functional/type-model/FR-149-apply-complete-equality-matrix.md | context_only |
| FR-056-AC-36, FR-056-AC-41 (equality mismatch locus) | spec/functional/FR-056-admit-domain-package-model-declarations.md | context_only |
| checker equality path | qsl-semantics/src/check/check/typing.rs, qsl-semantics/src/check/check.rs | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-1058 row 16a locates the `ill_typed`/`type-mismatch` for `later.tally = -earlier.tally` at `-earlier.tally`. QSpec FR-149-AC-21 gives no locus, so QSL's own convention governs: FR-056-AC-36 and FR-056-AC-41 refuse an equality operand-type mismatch "at the comparison", and the checker does the same (unary `-` types `Integer`, check.rs:2431; `admit_sum_into_peer` leaves a non-arithmetic operand alone and `equality` maps the matrix refusal to the equality's own location, check.rs:2223). The guessed span disagrees with both, so a test written to the row fails or forces a checker change against convention. Fix: "at the comparison `later.tally = -earlier.tally`", and say so in FR-381-AC-10, which names no locus for this case. | spec/test-cases/TC-1058-s3-checks-compensation-registration-retry-commit-and-recovery.md:53; spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md:296 |
| FND-002 | medium | The no-coercion sentence says "a unary `-`, a path, a call, `div`, `rem`, `count` or an unannotated `let`-bound name against an `Int[..]` operand is `ill_typed`/`type-mismatch`". FR-149 limits this to `Integer`-typed operands ("and any other `Integer` operand"; AC-21 "an `Integer` path"). Read as written, an `Int[0,3]` path or call against `Int[0,3]` (`later.tally = earlier.tally`, the identical-type row) is refused, which FR-149 and the checker admit. Fix: "an `Integer`-typed unary `-`, path, call, ...". | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md:232-235 |
| FND-003 | medium | The restatement reads as a closed list but is narrower than FR-149: it omits `sum` from the non-coerced forms, omits that an arithmetic operand against a `Rational[..]` or `Decimal[..]` operand stays `ill_typed`, and says only "`<` supplies no expected type" where FR-149 says `<`, `<=`, `>` and `>=`. A reader of FR-381 alone could take `<=` or a `Rational[..]` peer as coercing. Fix: list `sum`, the Rational/Decimal case and all four ordering operators, or drop the list and defer wholly to FR-149. | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md:232-235 |
| FND-004 | medium | The bullet scopes the rule to "an operand of `=` or `!=` in a retry relation or a recovery predicate". FR-149 applies it to every `=`/`!=`, and the QSL checker applies it on the one general equality path (`admit_sum_into_peer`, typing.rs:2099,2113), so guards, captures, invariants and function bodies get it too. QSL states it nowhere for those positions, so the spec says less than the code does and a reader infers they refuse. Fix: state the rule once at the general clause/expression typer (for example FR-104) and have FR-381 reference it, or say FR-381's bullet is the protocol-position instance of a general rule. | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md:223-224 |

## Verdict

Not mergeable as is. FND-001 changes a TC expected result; FND-002 to FND-004 are wording fixes in one bullet. The rule's core matches merged QSpec FR-149: the coerced form, either-side and parenthesized matching, the obligation and its facts, the `undefined_expression`/`unproved-range` code, cause and locus at the arithmetic node, and the no-clamp/wrap/widen clause. The AC-10 intervals are correct. The unguarded and recovery-predicate refusals are already TC-1058 rows 16 and 22.

## New findings (disposition pass 1)

Against head 5dadb78aa.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | FR-104-AC-12 and TC-1084 step 3 require `earlier.tally + 1 <= later.tally` (an `Integer` sum against `Int[0,3]` under `<=`) to refuse `ill_typed`/`type-mismatch`, as QSpec FR-149-AC-21 says. The QSL checker accepts it: `ordering` admits any pair where both sides are `Integer` or `Int(_)` as `OrderedKind::Integers` (qsl-semantics/src/check/check.rs:1444, 2245), and the ordering peer path does no coercion (typing.rs:2048-2069). The spec now asks for a refusal the code does not make. A test written to TC-1084 step 3 fails, and turning on the refusal would reject every existing mixed `Integer`/`Int[..]` ordering. That needs a QSL code ticket, or a QSpec check that FR-149-AC-21 really means to ban mixed integer ordering. This is the planner's call, not the author's. | spec/functional/FR-104-check-state-clauses.md:309; spec/test-cases/TC-1084-equality-types-an-arithmetic-operand-against-a-bounded-integer.md:23 |
| FND-006 | low | The TC-1084 fixture is under-specified. It gives "parameters `earlier` and `later`" of the record but checks the same text "as a guard, as an invariant and as a function body". An invariant binds `self`, not parameters, and "a guard" does not say which clause's guard. FR-104's new bullet also lists "a function body" among positions "in a clause". Name the clause, or the binder, for each position. | spec/test-cases/TC-1084-equality-types-an-arithmetic-operand-against-a-bounded-integer.md:19-21; spec/functional/FR-104-check-state-clauses.md:141-144 |
| FND-007 | low | FR-104 now restates QSpec FR-149's rule as normative, but its frontmatter has no `ix://agent-ix/quire-specification/FR-149` relationship, so the cross-repo dependency is invisible to link and blast-radius queries. Add a `depends_on` edge. | spec/functional/FR-104-check-state-clauses.md:1-32 |

## New findings (disposition pass 2)

Against head 57228a451.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | FR-104-AC-12 and TC-1084 make `record Counter` the context of a `pre` guard ("an operation `bump` on `Counter`") and of an invariant "over `self`". FR-104:80-82 refuses a clause whose `T` names no object type, state machine or process, with `missing_declaration`/`missing-name`, and operations belong to object types and processes (FR-104:84-91, FR-104-AC-10's `Flow`). So the two guarded clause vectors refuse at the context before reaching the expected-type rule, and the AC contradicts its own FR. Fix: make `Counter` an object type of a model (clauses `on M::Counter`, operation `bump` declared on it); `f` can keep record or reference parameters. | spec/functional/FR-104-check-state-clauses.md:313; spec/test-cases/TC-1084-equality-types-an-arithmetic-operand-against-a-bounded-integer.md:19-21 |
| FND-009 | low | Step 3 checks `later.tally = n` "in `f`", but `f(earlier: Counter, later: Counter)` has no `n`. The fixture's free-standing "`n` of `Integer`" is not in scope in `f`'s body, so the vector refuses `missing_declaration`, not the `type-mismatch` it asserts. Fix: declare `f(earlier: Counter, later: Counter, n: Integer)`. | spec/test-cases/TC-1084-equality-types-an-arithmetic-operand-against-a-bounded-integer.md:19,23; spec/functional/FR-104-check-state-clauses.md:313 |
| FND-010 | medium | The fix round committed four Linear comment bodies as files under `reviews/`: `c-base.md`, `c-ears.md`, `d1-base.md` and `d1-ears.md`. They are not SpecReview artifacts, `quire validate` rejects them ("required 'type' is missing from frontmatter"), and their generic names will collide with later reviews. Only the two `26-10-19-std-157-spec-review-*.md` SR files belong in `reviews/`; delete the other four. | reviews/c-base.md; reviews/c-ears.md; reviews/d1-base.md; reviews/d1-ears.md |

## New findings (disposition pass 3)

Against head 6aab9a1c1.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | medium | In TC-1084, `f(earlier: Counter, later: Counter, n: Integer)` now takes object-type parameters, so `earlier` and `later` are references. Outside a state clause the checker treats `x.f` over a `Reference` as a record field read (`is_model_receiver` is true for a reference only `in_state_clause()`, qsl-semantics/src/check/check.rs:2482-2492; otherwise `field` needs a `Composite` record operand and returns `mismatch`, check.rs:2729-2737). So every `f` vector (steps 1 to 3) refuses `ill_typed`/`type-mismatch` at `earlier.tally` before the equality is typed: step 1's guarded form does not check, and steps 2 and 3 fail at the wrong span. FR-056-AC-9 and FR-098-AC-14 read a function's reference parameter as `deref(n).f`. The parameter type is also written unqualified (`Counter`), where object types are `M::Counter` (FR-056-AC-9; FR-108's `a: Config::ConfigVersion`). Fix: `function f using v(earlier: M::Counter, later: M::Counter, n: Integer): Boolean pure` with `deref(earlier).tally` and `deref(later).tally` throughout, or give `f` record parameters of a separate `record`. | spec/test-cases/TC-1084-equality-types-an-arithmetic-operand-against-a-bounded-integer.md:19-23; spec/functional/FR-104-check-state-clauses.md:313 |

## New findings (disposition pass 4)

Against head 05c1d8de6. Every form was run through the checker: a scratch test (probe-sr1039_probe.rs beside this file) in a clone at 05c1d8de6 adds `Tally` (`Int[0,3]`) and `Counter { tally, next }` with operation `bump` to the shared `config_version_document` fixture, then assembles and checks each body with `admit_and_assemble_with_body`.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | medium | FR-104-AC-12 and TC-1084 step 1 say `f` with body `deref(earlier).tally < 3 and deref(later).tally = deref(earlier).tally + 1` checks. Run through the checker, it refuses `undefined_expression`/`unproved-range` at the sum: in a function body the guard over a `deref(r).f` read supplies no fact. The other forms behave as written. `pre G` and `invariant I` check guarded and refuse at the sum unguarded. In `f`, `deref(later).tally = deref(earlier).tally` checks, `= -deref(earlier).tally` and `= n` refuse `ill_typed`/`type-mismatch` at the comparison, and the ordering checks. Forms that do check guarded: record parameters (`record Pair { tally: Int[0, 3]; }`, `f(earlier: Pair, later: Pair, n: Integer)` with `earlier.tally < 3 and later.tally = earlier.tally + 1`, with all five `f` vectors as AC-12 states), scalar parameters `a: Int[0, 3]`, and `let e = deref(earlier).tally in e < 3 and deref(later).tally = e + 1`. Fix: use record parameters for `f`. Whether a guard over `deref(r).f` should supply a fact in a function body, as `self.f` does in a clause, is a separate checker question for the planner. | spec/functional/FR-104-check-state-clauses.md:313; spec/test-cases/TC-1084-equality-types-an-arithmetic-operand-against-a-bounded-integer.md:19-21 |

## Dispositions

Round 1, against head 5dadb78aa.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | TC-1058 row 16a and FR-381-AC-10 now locate the refusal at the comparison `later.tally = -earlier.tally` |
| FND-002 | fixed | FR-104 bullet: "an `Integer`-typed unary `-`, path, call, ..." plus "An operand of the same `Int[..]` type as its peer is accepted unchanged" |
| FND-003 | fixed | FR-104 bullet lists `sum`, the `Rational[..]`/`Decimal[..]` case and `<`, `<=`, `>` and `>=` |
| FND-004 | fixed | The rule is stated once in FR-104 for every `=`/`!=` (FR-104-AC-12, TC-1084); FR-381 refers to it |

Round 2, against head 57228a451.

| FND | outcome | reason |
| --- | --- | --- |
| FND-005 | fixed | FR-104 and FR-381 now accept an ordering between `Integer` and `Int[..]`, matching the checker; the `<=` refusal vector is gone. STD-210 tracks the matching QSpec FR-149-AC-21 amendment |
| FND-006 | fixed | TC-1084 now names each position: the `pre` of `bump`, an invariant over `self` and `f(earlier, later)` (their validity is FND-008 and FND-009) |
| FND-007 | fixed | FR-104 frontmatter gains `ix://agent-ix/quire-specification/FR-149` `depends_on` |

Round 3, against head 6aab9a1c1.

| FND | outcome | reason |
| --- | --- | --- |
| FND-008 | still-open | The context is now the object type `M::Counter`, but the fixture uses "the header of FR-108's unit", whose only model line is `model Config = "example/config-version" ...`. That package declares no `Counter`, and the unit declares no `M`, so `pre G ... on M::Counter::bump` and `invariant I ... on M::Counter` refuse `missing_declaration`/`missing-name` at `M` (FR-104:80-82) before the rule applies. Fix: extend the Config package with `Counter` and `bump`, as TC-459 step 7 extends it with `Flow`, and write `Config::Counter`; or give the unit its own `model M = "<package>" digest ...` header over a package that declares them |
| FND-009 | fixed | `f(earlier: Counter, later: Counter, n: Integer)` binds `n` (the parameter typing is FND-011) |
| FND-010 | fixed | `reviews/c-base.md`, `c-ears.md`, `d1-base.md` and `d1-ears.md` deleted; only the two SR files remain |

Round 4, against head 05c1d8de6.

| FND | outcome | reason |
| --- | --- | --- |
| FND-008 | fixed | Verified by the checker probe: over the Config package extended with `Counter`/`bump`, `pre G using v on Config::Counter::bump { self.tally < 3 and self.next = self.tally + 1 }` and `invariant I using v on Config::Counter at current { ... }` check, and each unguarded form refuses `undefined_expression`/`unproved-range` at the sum |
| FND-011 | fixed | Verified by the checker probe: `f(earlier: Config::Counter, ...)` with `deref(...)` reads types; `deref(later).tally = deref(earlier).tally` checks and the `-` and `n` vectors refuse `ill_typed`/`type-mismatch` at the comparison (the guarded vector is FND-012) |

Round 5, against head e3ae4c594. Every vector was checked with the probe (probe-sr1039_probe.rs), its strings taken verbatim from TC-1084 at e3ae4c594. Results: `pre G` and `invariant I` check guarded and refuse `undefined_expression`/`unproved-range` at the sum unguarded. Over `record Pair { tally: Int[0,3]; }`, `f` checks guarded and refuses at the sum unguarded; `later.tally = earlier.tally` checks; `later.tally = -earlier.tally` and `later.tally = n` refuse `ill_typed`/`type-mismatch` at the comparison.

| FND | outcome | reason |
| --- | --- | --- |
| FND-012 | fixed | `f` takes `record Pair { tally: Int[0,3]; }` parameters with plain `earlier.tally`/`later.tally` reads; all five `f` vectors and the four `Config::Counter` clause vectors give the results FR-104-AC-12 states |
