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

## Dispositions

Round 1, against head 5dadb78aa.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | TC-1058 row 16a and FR-381-AC-10 now locate the refusal at the comparison `later.tally = -earlier.tally` |
| FND-002 | fixed | FR-104 bullet: "an `Integer`-typed unary `-`, path, call, ..." plus "An operand of the same `Int[..]` type as its peer is accepted unchanged" |
| FND-003 | fixed | FR-104 bullet lists `sum`, the `Rational[..]`/`Decimal[..]` case and `<`, `<=`, `>` and `>=` |
| FND-004 | fixed | The rule is stated once in FR-104 for every `=`/`!=` (FR-104-AC-12, TC-1084); FR-381 refers to it |
