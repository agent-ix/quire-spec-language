---
id: SR-638
title: "QSL-266 integrity review of per-operation requirement records"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@64ee12700cd66bb17767a8e9090cbca114308364; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md (E7, unchanged); spec/decisions/ADR-012-semantic-family-extension-contracts.md; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md (O-04, O-07, unchanged); spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md; spec/functional/FR-057-admit-shared-capability-kinds.md; spec/functional/FR-062-implement-checked-family-contract.md; spec/functional/FR-075-compute-candidates-from-registered-backends.md; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md (unchanged); spec/functional/FR-097-classify-claim-extent-and-write-bounded-requests.md (unchanged); quire-specification FR-290 at origin/main (external)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: reviews
---

## Summary

Ticket: QSL-266. This review checks consistency between the diff and the
sections it leaves unchanged: ADR-011 E7, ADR-012, ADR-013 O-04/O-07,
ADR-014 §4, FR-057, FR-062, FR-075, FR-093, FR-097, and the external
FR-290.

What is consistent:
- The keying rule: (application node, `expression`, ordinal), with ordinals
  in source order.
- The ADR-012 §2 and §13.5 rows, the §7.2 step-1 rewrite, and the ADR-014 §4
  extent paragraph agree with FR-062's new section.
- The FR-062 status arithmetic adds up: 5 backed, 5 partly backed, 3
  unbacked, 13 in total.

What conflicts:
- The CG handoff in FR-075 conflicts with ADR-011 E7. It also drops
  what CG needs to tell two occurrences of one node apart.
- FR-057's claim-form SHALL still defers to FR-290's table, which has no
  such row.
- "Enclosing" and "wraps" give two readings of the narrow rule.
- FR-097-AC-6's "exactly when" no longer has a defined granularity.
- The driver cannot answer `requires-bound` from what FR-075 gives it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Under FR-075, the driver "hands CG generation each routed item's request index and node". This conflicts with ADR-011 E7, where the driver passes CG the requirement records keyed by occurrence key. It also loses what distinguishes two records at one node. Take `g(x + 1)` and `h(x + 1)` in one body, with `g(p: Int[0, 10])` and `h(q: Int[0, 20])`: that is one `+` node, two occurrences and two records. The result bound of each is a different enclosing narrow ([0, 10] and [0, 20]). CG gets the same node for both and must generate both obligations with one bound. The record (`Requirements`: kind, extent, authored bound) does not carry the result bound either, so nothing QSL hands over tells CG which narrow applies. There is outside evidence of the symptom: quire-driver `tests/drive.rs:66-82` (IR-302) says the narrow's bound "is not reachable from the arithmetic node itself" in IR. Fix: hand CG the occurrence key (and ADR-013 O-09's obligation identity gains it, as the ADR-011 E7 identity row already anticipates for clauses), and state where the result bound comes from. Either the record carries it, or CG reads the occurrence's enclosing narrow through the source map. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:138-144; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:260; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1016 |
| FND-002 | medium | "Enclosing" and "wraps" give two readings of the narrow rule. FR-057's claim form says the result lies "in the target range of an enclosing narrowing conversion". FR-062 and ADR-014 say the narrow supplies "the result bound of the application it wraps". The two differ for nested operations. In `function f using v(x: Int[0, 9]): Int[0, 20] pure { (x + 1) * 2 }` the narrow wraps the `*`. The `+` is also enclosed by it, so under FR-057 the `+` claim's result bound is [0, 20], which is the wrong value to bound, and under FR-062 it is `Integer`. Separately, the typing walk pushes the expected type through `if` and `let` and coerces at the leaf (qsl-semantics/src/check/check/typing.rs:497-525), so a narrow's operand is often not a scalar operation application: a literal (FR-093-AC-4 `c2`, `3` into `Int[0, 9]`), a parameter read (`if n >= 0 and n <= 10 then n else 0` into `Int[0, 10]`), a call, `sum`, `count`, `size`, a projection or `value`. For these, FR-057's narrow row ("it is the result bound of the application it wraps") is undefined, and the narrow's range is in no record. Use "wraps" in FR-057 too, and state what a narrow over any other operand gives. For example: no record, with its range discharged by `check`'s Coerce range obligation (FR-093). | spec/functional/FR-057-admit-shared-capability-kinds.md:170-171; spec/functional/FR-062-implement-checked-family-contract.md:198-202; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md:242-245 |
| FND-003 | medium | FR-057's claim-form SHALL still reads "the kind the FR-290 claim-form assignment table gives its clause's own claim form". The new sentence right after it says a `Value` function "carries no clause", and the two new rows are not in FR-290's table. FR-290 at quire-specification origin/main has only clause rows. It says "an expression nested in it … adds no kind" and defines `value-validity` as "Whether a Boolean clause … holds". So the SHALL is false for the new claim form, and FR-057 now names an external table as the authority for rows that table does not have. Fix: state FR-057's table as QSL's own claim-form table, which contains FR-290's rows plus the `Value` function rows, and cite FR-290 only for the kind vocabulary. If the ecosystem needs the row, raise it upstream separately. Do not block on it. | spec/functional/FR-057-admit-shared-capability-kinds.md:159-172 |
| FND-004 | medium | FR-097-AC-6, which is unchanged, says IR's `require_bounds` lowering returns `RequiresBound` "exactly when QSL's extent is `Unbounded`". ADR-014 §4 "IR's predicate" says the two are the same classification. Extents are now per application, while IR lowers the function. In `function f using v(x: Int[0, 9], n: Integer): Integer pure { (x + 1) + n }` the inner `+` is `Bounded` and the outer `+` is `Unbounded`. Nothing states which record IR's single answer must agree with, so TC-440's oracle is undefined for any body that mixes the two. State the granularity: IR's predicate per application node, or IR against the join of the function's records. | spec/functional/FR-097-classify-claim-extent-and-write-bounded-requests.md:74; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md:277-280 |
| FND-005 | medium | FR-075 says "The orchestrating driver (T-13) reads nothing else about items from QSL", but ADR-014 §4 "Bounded request" has the caller answer `requires-bound` with one `ProofBound` per `DomainKey`. The builder's item carries only the classification (`finite_bound_available`), not the domains. FR-075-AC-8's own `n + 1` fixture settles `requires-bound`, and with only these items the driver cannot write the bounded follow-up (`RequestWriter::bounded_item` needs the record's `Requirements`). Either the item carries its unbounded domains (or the record), or the spec states that the bounded follow-up is outside the driver's first version. Say which. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:138-144; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md:259-266 |
| FND-006 | low | FR-097's Dependencies, which are unchanged, still describe FR-062-AC-4 as "`FamilyContract::requirements()`, which returns the `Requirements` value carrying this requirement's `ClaimExtent`". That is singular. The new contract returns one per claim site (`Vec<(ClaimSite, Requirements)>`). | spec/functional/FR-097-classify-claim-extent-and-write-bounded-requests.md:84-86 |
| FND-007 | medium | Disposition pass, at accbac38. The fix for FND-004 creates a new contradiction. FR-097-AC-6 and ADR-014 §4 now require IR's `requires-bound` predicate "on each record's application node" to agree with that record's extent "exactly when". But the fixes also make a record's extent come from sources outside the node's own operands: roots reached through a `let` binder's bound value, and roots read only by a guard. IR's predicate looks at the lowered node and what it references. There are three cases. (1) RR-7 `let t = x + 1 in t * 2`: the `*` record is `Bounded` through `x`, but the `*` node references `t`'s binder node, whose type is `Integer` (FR-092 E12: `x + 1` is `Integer`), so IR sees an unbounded domain. (2) `if n = 0 then x + 1 else 0` with `n: Integer`: the `+` record is `Unbounded` at `n` because of the guard, but nothing reachable from the `+` node mentions `n`. (3) quire-driver `tests/drive.rs:66-82` notes IR's bounds closure is forward-only. TC-440 step 4 uses `(x + 1) + n`, which has neither shape, so it passes while the AC as written fails on RR-7. Either state the agreement only for records whose roots are all reachable from the node, or have IR evaluate the record (node, `let` context, path condition) rather than the node. | spec/functional/FR-097-classify-claim-extent-and-write-bounded-requests.md:74; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md:286-292; spec/functional/FR-062-implement-checked-family-contract.md:251-259, 296 |
| FND-008 | medium | Disposition pass, at accbac38. FR-075-AC-8 and TC-449 step 2 expect the bounded follow-up for RR-6 to be "written with its own request index, not 0". The builder returns `RequirementItem`s, not the `RequestWriter` that numbered them. ADR-014 §4 "Bounded request" says the caller answers "by submitting a new request". A driver that follows ADR-014 writes the follow-up with a fresh `RequestWriter` in a new FR-331 request, where its index is 0, and fails the AC. Nothing states whether the follow-up is appended to the builder's request, so it shares its index space, or is a new request. State which one, and how the driver reaches the writer (for example, the builder returns its `RequestWriter` with the items). | spec/functional/FR-075-compute-candidates-from-registered-backends.md:142-152, 239; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md:268-270; spec/test-cases/TC-449-request-builder-writes-one-item-per-requirement-record.md |
| FND-009 | low | Disposition pass, at accbac38. FR-062's new "The claim" paragraph cites "(FR-146, `check::facts`)" as where `check` proves definedness under guards. This repo has no FR-146. The only FR-146 in the ecosystem is quire-specification FR-146, the `case` exhaustiveness and termination obligation cited in FR-057 and ADR-012, which does not cover guards. Cite FR-093's definedness walk (or `check::facts` alone) instead. | spec/functional/FR-062-implement-checked-family-contract.md:228-232 |

## Dispositions

Disposition pass at `agent-ix/quire-spec-language@accbac3849a26f9f8206e655369105408b9f4a11` (fix commit `accbac38`, "QSL-266 spec: address spec review SR-636 to SR-640"). Each outcome was re-checked against the spec at that head, not taken from the commit message.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed accbac38 | FR-075 now has the driver hand CG "each routed item's request index, occurrence key, node and result bound, with the package's requirement records keyed by occurrence key (ADR-011 E7)". The ADR-011 E7 row now lists the result bound and the path condition. The ADR-013 obligation identity now uses the application's own occurrence key, so the `g(x + 1)` and `h(x + 1)` records are two obligations. Records and items carry the result bound, and RR-16 pins [0, 10] and [0, 20] (FR-075:142-152, ADR-011:260, ADR-013:277, FR-062:260-262). |
| FND-002 | fixed accbac38 | "Wraps" is now used in FR-057, FR-062 and ADR-014. A narrow over anything other than a scalar operation application gives no claim, and `check`'s `Coerce` range obligation discharges it. RR-10 and RR-11 pin this (FR-057:171-172, FR-062:213-216, 234-237, ADR-014:250-257). |
| FND-003 | fixed accbac38 | FR-057 now states that the table is QSL's own: FR-290's rows plus the `Value` function rows, with FR-290's kind vocabulary. The SHALL takes the kind from "this table", and the Dependencies point to quire-specification ticket STD-108 (Linear, Backlog) for the upstream row. That is not a blocker (FR-057:159-163, 368-373). |
| FND-004 | fixed accbac38 | The granularity is now stated: agreement is per application node (FR-097-AC-6, ADR-014 "IR's predicate", TC-440 step 4). The rule as stated contradicts the `let` and guard root rules; see the new FND-007. |
| FND-005 | fixed accbac38 | `RequirementItem` now carries its unbounded `DomainKey`s and their kinds. The driver answers `requires-bound` with one `FiniteBound` per key through `RequestWriter::bounded_item`, and FR-075-AC-8 and TC-449 step 2 test it. The index space of the follow-up is left open; see the new FND-008 (FR-075:130-131, 142-152). |
| FND-006 | fixed accbac38 | FR-097's Dependencies now read "AC-4 and AC-13: `FamilyContract::requirements()`, which returns one `(ClaimSite, Requirements)` pair per claim site" (FR-097:84-87). |

New in this pass: FND-007 (medium), FND-008 (medium) and FND-009 (low), recorded in `## Findings` above. They have no outcome yet.
