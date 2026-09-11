---
id: SR-373
title: "Failure-domain review of domain-event Boolean choices"
type: SpecReview
analysis: failure-domain
scope: "FR-042 domain-event amendment; TC-121 procedure 5; docs/compiled-protocol-v1.md observed-Boolean fragment"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-042, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/TC-121, type: references }
---

## Summary

Applied the selected QUOIN failure-domain checklist to trust boundaries, entity
identity, evaluation purity and topology for same-owner domain-event facts,
including compensation-qualified events. The scope is static native family
admission and retained artifact obligations; runtime protocol execution remains
outside it. No new missing constraint requires an FR/StR/NFR addition.

## Verdict

**PASS** for this scoped specification lens. This does not waive SR-371's
inherited corpus gaps or replace the separate execution and code review evidence.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No missing failure-domain constraint found in the domain-event amendment. Exact role/provenance, independent registration prerequisites, pure Boolean evaluation and bounded explicit incomplete outcomes are specified. | FR-042 behavior and AC-5/6/9; TC-121 procedure 5; docs/compiled-protocol-v1.md |

## Analysis

Trust boundaries: the atom derives from an admitted event binder and model field,
not an arbitrary payload or external callback. Source/type/proof/family checks
remain prerequisites, and failed ownership/visibility/partition remains typed
FamilyProof. No extension-point exception policy is introduced. The qualified
event retains its exact registration binding; Boolean truth cannot satisfy that
binding or create activation, an operation effect, a send or a commit. Existing
runtime/compensations adds the event Invocation's registration prerequisite.

Entity identity: role SymbolId equality is distinct from shared model type;
binder, original control anchor, model and field identities distinguish otherwise
equal-typed observations. The event's compensation association is an exact local
handle rather than a guessed same-name obligation. The new source-owned tests
preserve it alongside registration/activation/attempt/effect prerequisites.

Purity: existing totality and the conservative independent-atom abstraction
remain explicit. Abstract failure is an unproved partition, not a fabricated
concrete violating trace. Original operands and visibility obligations survive
Boolean simplification; no side-effectful event execution is introduced.

Topology: existing flow admission controls necessary predecessor availability
and all-branch joins. Optional/sibling/await-local records cannot gain visibility
by textual ordering. Atom enumeration and per-choice temporary records consume
shared References/Entries limits; exhaustion is source-located incomplete, and
retry uses fresh counters. Twelve-event valuation and multiple-choice exact
budget controls supply concrete boundary evidence without changing those rules.

Reviewed together with the accepted choreography-surface event rule (ordinary
and qualified domain events) and protocol-contract role-visible choice rule.
The specification does not invent a blanket exclusion for qualified events.
Parent focused tests pass; broader gate status is recorded in SR-371.
