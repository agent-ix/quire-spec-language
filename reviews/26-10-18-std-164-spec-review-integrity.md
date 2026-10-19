---
id: SR-12813
title: "integrity review of STD-164 state-free footprints (FR-155, FR-162)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@8b413f340493dc7b760c011401a08ed358196e58; spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md, spec/functional/FR-162-offer-reductions-through-transition-system-hooks.md"
review_set: subset
---

## Summary

Ticket: STD-164 (PR quire-spec-language#969). Integrity analysis of the
amended FR-155 and FR-162 against FR-101 (hook signature), FR-120
(transition identity and reference encoding), FR-156, ADR-021 POR-1, EI-4 and
QS-4, and QSpec FR-383.

## Verdict

FAIL on two medium findings. These parts are consistent: FR-101:124 already
declares `fn footprint(&self, transition)` with no state; ADR-021 POR-1 calls
footprints "static"; EI-4 gives `footprint(transition) -> Footprint`; and
FR-155's existing Determinism bullet already makes footprints functions of
the package and the identity. The state-free hook in FR-162:77-81 therefore
aligns with FR-101, POR-1 and EI-4. The problem is the declared-type
coarsening and the premise given for it.

## Examined Scope

```yaml
scope:
  - {id: FR-155, path: spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md, role: examined, excerpt: "A footprint is also state-free: it is a function of the checked package and the transition identity alone, keyed on the declared types of the operation's receiver and reference parameters. ... and an object's most-specific type is a fact of the state."}
  - {id: FR-162, path: spec/functional/FR-162-offer-reductions-through-transition-system-hooks.md, role: examined, excerpt: "State-free footprint hook. The `footprint` hook takes a transition identity and no state, and SHALL return `Some` for every transition identity of the system, enabled or disabled and whether or not the objects it names exist in any state (FR-155)."}
  - {id: FR-120, path: spec/functional/FR-120-simulate-a-checked-package-s-state-family.md, role: context_only, excerpt: "A reference encodes as FR-181's triple: `universe` is the population's `UniverseId` as 64 lowercase hex digits, `object_type` the most-specific type's `EffectiveId` as 64 lowercase hex digits, and `identity` the lowercase hex of the key's UTF-8 bytes"}
  - {id: FR-101, path: spec/functional/FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md, role: context_only, excerpt: "fn footprint(&self, transition: &Self::TransitionId) -> Option<Footprint> { None }"}
  - {id: ADR-021, path: spec/decisions/ADR-021-state-space-reduction.md, role: context_only, excerpt: "POR-1 ... has a static write footprint W(t) and read footprint R(t) ... EI-4 ... footprint(transition) -> Footprint for POR"}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The stated premise ("an object's most-specific type is a fact of the state") contradicts FR-120. The transition identity's receiver and each reference argument are FR-181 reference triples whose `object_type` is the most-specific type's `EffectiveId`, so the type is a fact of the identity and needs no state. PR #967's `ModelSystem::footprint` (1ad86fa88, qsl-eval/src/simulation/model/footprints.rs:333-365) already parses `receiver.object_type()` from the identity text with no state, and `bump(d)` names `d`'s type in its own identity. Failure scenario: the coarsening is unnecessary yet becomes normative, it makes footprints less precise than FR-383, and it carries SR-12812 FND-001's unsoundness. Fix: state that the footprint keys on the `object_type` of the identity's references and apply FR-383 exactly, or give a concrete identity that has no `object_type`. | spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md:31-37,101-114; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:209-212,334-344 |
| FND-002 | medium | FR-155 now contradicts itself and its normative source. The Derivation bullet says S3 computes footprints "exactly by QSpec FR-383", and the Description says it implements FR-383 and "does not restate" it (lines 37-39, 87-88). The new State-free bullet permits footprints that "may contain more" than FR-383 derives. ADR-021 QS-4 and FR-155's Dependencies make FR-383 the normative owner of the derivation, and FR-383 has no declared-type rule. No paired QSpec change is linked. Failure scenario: the FR-383 conformance vectors (FR-383-AC-3 "derived ... as stated", AC-4's exact 7-state count) and an implementer who reads the Derivation bullet disagree with this bullet for any subject with subtyped populations. Fix: drop the coarsening (FND-001), or amend QSpec FR-383 under STD and reword the Derivation bullet. | spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md:37-39,87-88,112-114,158-159 |
