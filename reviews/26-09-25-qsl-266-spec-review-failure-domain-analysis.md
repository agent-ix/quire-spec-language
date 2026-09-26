---
id: SR-640
title: "QSL-266 failure-domain review of per-operation requirement records"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@64ee12700cd66bb17767a8e9090cbca114308364; spec/functional/FR-057-admit-shared-capability-kinds.md; spec/functional/FR-062-implement-checked-family-contract.md; spec/functional/FR-075-compute-candidates-from-registered-backends.md; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md (unchanged); spec/functional/FR-097-classify-claim-extent-and-write-bounded-requests.md (unchanged); qsl-semantics/src/check/facts.rs; qsl-semantics/src/check/mod.rs; qsl-semantics/src/family/requirements.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: reviews
---

## Summary

Ticket: QSL-266. This review probes four areas:
- the claim semantics;
- identity: keying and the pairing of occurrences;
- dropped records: literals, `let`, query binders, narrow, occurrence scope;
- extent edge cases, with the conservative invariant "may say Unbounded
  where Bounded is true, never the reverse".

What holds:
- Literals contribute nothing, so `1 + 1` is `Bounded`.
- A `let` read is transitive through the bound value.
- Query, `count`, `sum`, `fold` and `reduce` binders contribute their checked
  type, which is sound because a binder with a finite type takes finitely
  many values.
- A projection or deref reaches its root parameter, whose transitive type
  closure is walked.
- Two structurally identical applications get distinct keys.

The narrow gap is recorded in SR-638 FND-002.

Four defects:
- The claim statement drops the path condition that `check` itself used,
  so a checked body yields false claims.
- Pairing a claim to its occurrence is unspecified where one node has two
  different extents.
- The occurrence scope is package-wide, not body-local.
- Two failure paths are unstated.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The claim form is: the application "is defined and its result lies in its result type, or in the target range of an enclosing narrowing conversion, for every assignment of the parameters and bound variables it reads". It has no path condition. `check` proves definedness and narrow ranges under the facts of enclosing guards, short-circuit operands and filters (qsl-semantics/src/check/facts.rs:753-790 for `Coerce` and `Divide`). Take `function f using v(n: Integer): Boolean pure { if n >= 0 and n < 10 then g(n + 1) else true }` with `g(p: Int[0, 10])`. `check` admits it, but the `+` claim as written ("n + 1 ∈ [0, 10] for every n: Integer") is false at n = 10. Likewise `if y != 0 then x / y else 0` gives a `/` claim that is false at y = 0. A backend refutes a claim for a body QSL proved, which is a spurious violation. Fix: make the claim the application's validity under its occurrence's path condition. Then the parameters and binders the path condition reads are extent roots too, or an application reading only `x` under a guard on `n: Integer` is labelled `Bounded` while its claim quantifies over unbounded `n`, which breaks the conservative invariant. | spec/functional/FR-057-admit-shared-capability-kinds.md:170; spec/functional/FR-062-implement-checked-family-contract.md:184-202; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md:236-248 |
| FND-002 | medium | FR-062 says "Each record is computed from the checked tree of its own occurrence", but it does not say how a claim from `requirements()` is paired with its occurrence. The existing `key_requirements` pairs by identity and position ("the first index to request one gets the first occurrence", qsl-semantics/src/check/mod.rs:1107-1157). One node can carry two different extents in one function. Sibling `let` binders reuse a level (FR-093: "lowering gives that sibling its enclosing binder's level"), so in `function f using v(x: Int[0, 9], n: Integer): Integer pure { (let t = x + 1 in t * 2) + (let t = n + 1 in t * 2) }` both `t` are the same parameter node (name `t`, same level, type `Integer`). Both `t * 2` are then one node with two `expression` occurrences, whose extents are `Bounded` (through `x`) and `Unbounded` (through `n`). Pairing by order can key the `Bounded` extent at the `n` occurrence, which is the forbidden direction. Require pairing by the site's own occurrence (see SR-639 FND-001), and add this fixture to FR-062-AC-13 and TC-160. | spec/functional/FR-062-implement-checked-family-contract.md:204-226; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:119-122 |
| FND-003 | medium | The record rule is "one requirement record for each `expression` occurrence of each scalar operation application node". Application nodes have no owner and are shared across the whole package (ADR-013 O-04; FR-093 "One node per checked expression"). So the rule also matches occurrences outside a `Value` function body: a `decreases` measure (FR-093 lowers "function bodies and measures"), and a Boolean clause once QSL-42 lands, where FR-057 says a nested expression "adds no kind". Given `function f using v(x: Int[0, 9]): Integer pure decreases(x + 1) { x + 1 }`, the spec does not say whether the unit has one `+` record or two. Scope the rule to occurrences whose region lies in a `Value` function body. | spec/functional/FR-062-implement-checked-family-contract.md:186-190, 204-205; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:72-76, 224 |
| FND-004 | low | The only drop guard named is `KeyFault::UnkeyableRequirements`, for an occurrence `check` "cannot key". A scalar operation application in a body that has only a `generated` occurrence and no `expression` occurrence (FR-093: "a node that no region denotes … has one `generated` occurrence") would give no record and no fault. State that every scalar operation application in a body has at least one `expression` occurrence, and that one without any is the same `KeyFault`. | spec/functional/FR-062-implement-checked-family-contract.md:228-230; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:83-87 |
| FND-005 | low | Per-application extent classification can fail. `classify_extent` stops with a node-count stage limit and faults on a composite missing from the environment (FR-097-AC-2; qsl-semantics/src/family/requirements.rs:204-240). ADR-012's `requirements()` returns an infallible `Vec`. The spec does not say that the classification runs in `check` (as the existing contract comment at qsl-semantics/src/family/contract.rs:307-309 does) and that its failure is the declaration's `StageFailure::Limit` or internal fault. A body with many applications multiplies the walks. State where the failure surfaces. | spec/functional/FR-062-implement-checked-family-contract.md:214-223; spec/decisions/ADR-012-semantic-family-extension-contracts.md:234 |
