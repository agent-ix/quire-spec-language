---
id: SR-511
title: "failure-domain review of PR #354 (FR-091 OQ rulings)"
type: SpecReview
analysis: failure-domain
scope: "git diff origin/main...HEAD at a0e00cf9: ADR-011, ADR-012, ADR-013, FR-062, FR-091, spec.md, TC-259, TC-392 to TC-396, TC-400, TC-401, TC-405, TC-406, TC-412, tests.md"
review_set: subset
evaluated_revision: "a0e00cf90cadc3cb5bd5cccec787aaa557105932"
review_date: "2026-09-22"
---

## Summary

Failure-domain analysis of the PR #354 diff. It checks entity identity, unstated
failure modes, evaluation purity and topological robustness for the new owner
scope, `using` alias resolution and the alias-cycle refusal. The rulings are
taken as decided. The owner-scope change drops a uniqueness guarantee that
other ADR-013 rows still rely on. Alias resolution and owner provenance have no
stated failure modes. No purity issue was found: the new behaviour is pure
S2/E3 resolution.

## Verdict

**Changes needed.** FND-001 is high.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Identity confusion across packages. O-04 used to admit "one `package_id` per `name@version`". It now admits one `package_id` "per domain-package identity and version", and the preimage leaves out revisions and digests. No rule now stops one check or replay from admitting two library packages built from two revisions of the same `SourceOwner`. An unchanged declaration in each of them mints the same `NodeKey`. That breaks T-3 ("a bare `NodeKey` is unique across packages"), ADR-013:202 (`declaration` `NodeKey` unique across packages) and ADR-013:326 (equal type node ids in two packages mean the same declaration). Fix, without re-arguing the ruling: add an I2 admission rule to O-04 allowing at most one `package_id` per source or definition owner within one check and one replay, with a named refusal and a TC. Alternatively, amend T-3 and ADR-013:202 and 326 to require `PackageNodeKey` for every cross-package reference, and state that equal `NodeKey`s may cross `package_id`s of one owner. | ADR-013:178, 202, 326, 808 |
| FND-002 | medium | Duplicate alias. The assembler resolves `using` to "the one profile selection of the unit that declares that alias". QSpec says "Model and profile aliases are unique within their source unit" (shared-grammar.md:518). The FR defines no refusal, code or AC for two profile selections with one alias, or for a profile alias that equals a model or import alias. The word "one" assumes a uniqueness that nothing checks. Fix: add an assembler refusal (or state the S1/S2 stage that refuses) for a repeated alias in one unit, with its catalog code, and add an AC and a TC-412 step. | FR-091:326-330, 450; QSpec shared-grammar.md:518-519 |
| FND-003 | medium | Owner provenance has no failure mode. The assembler takes `SourceOwner{authority, identity}` "from the source reference the unit was compiled from". The FR does not say what happens when that reference has no authority (QSL's source identity has none today, per QSL-159) or when the source is unnamed (an in-memory or test compile). Without a stated rule, an implementation could fall back to a constant owner, which recreates `DEFAULT_PACKAGE_IDENTITY`. Fix: state that a unit with no complete source owner refuses before any key is minted, name the code (QSpec FR-322-AC-15: "an absent owner fails the package lock join"), and add an AC. | FR-091:101-103, 334-339 |
| FND-004 | medium | Topological robustness of alias resolution. The alias-cycle rule covers only "a chain of aliases back to itself". Termination and the refusal are unstated for a cycle through a type constructor or a composite, for example `type A = Option<A>;`, or `type A = Sequence<B>[0, 1];` with `record B { a: A; }`. Resolution of a qualified name to an alias's resolved type can recurse without bound on these. Fix: state that any cycle in the alias/record/tuple type-reference graph refuses as `invalid_package`/`definition-cycle`, retaining the edges in resolution order, or state the rule that admits it. Add one such case to AC-17 and TC-400. | FR-091:355-358, 375-377, 445 |
| FND-005 | low | The `pre(x)` refusal in AC-8 depends on the clause kind being `Body`. `check` refuses `pre` whenever `clause_kind != Postcondition` (`src/check/check.rs:965`). The FR fixes `Body` for functions (FR-091:319), but says nothing about the `decreases` measure, where `pre(..)` is also refused. Fix: add a measure case (`decreases(pre(x))`) to AC-8 and TC-396, or state that the measure is covered by the same rule. | FR-091:260-262, 436; src/check/check.rs:952-968 |
