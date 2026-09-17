---
id: Task-050
title: "Admit domain-package models and implement model graph binding, inheritance and closed dispatch"
type: Task
status: not_started
track: A04
priority: P0
relationships:
  - target: ix://agent-ix/quire-spec-language/Task-049
    type: depends_on
  - { target: ix://agent-ix/quire-spec-language/FR-056, type: references }
  - { target: ix://agent-ix/quire-spec-language/IT-012, type: references }
  - { target: ix://agent-ix/quire-specification/FR-150, type: references }
  - { target: ix://agent-ix/quire-specification/FR-151, type: references }
  - { target: ix://agent-ix/quire-specification/FR-152, type: references }
  - { target: ix://agent-ix/quire-specification/FR-153, type: references }
  - { target: ix://agent-ix/quire-spec-language/TC-145, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/TC-146, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/TC-147, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/TC-148, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-195, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-196, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-197, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-198, type: verifies }
---
# Task-050: Admit domain-package models and implement model graph binding, inheritance and closed dispatch

## Scope

Execute QSL #131 and #120 in order. #131 admits domain packages under
[FR-056](../../../spec/functional/FR-056-admit-domain-package-model-declarations.md):
a spec artifact bundle runs through `quire_rs::semantic::extract_semantic` with
the modules' real manifests, the `agent-ix-extraction-frontend` lift and the
`agent-ix-semantic-ir` reader, and each Semantic IR 2.0.0 type definition becomes
a Quire model declaration bound by its construct's `meaning` id, `shape` and
`identity`. #120 then implements, over those admitted declarations, original and
effective provenance, closed populations, typed references and traversal,
conformance and redefinition, and unique most-specific dispatch.

## Subtasks

- [ ] Select the merged agent-ix/filament-core-data#172 revision in `Cargo.lock`
      and write TC-145–148 and IT-012 first.
- [ ] Implement domain-package intake: constructs-table kind resolution,
      meaning-id binding, artifact-id identity and relationship exports.
- [ ] Implement closed lookup, specialization validation and ambiguity refusal,
      then TC-195–198.
- [ ] Pass local Rust gates and PR-time Rust/gap review; merge before Task-051.

## Deliverables

- Replayable model declarations keyed by domain package identity, IR node
  identity and `sha256-jcs`, with no ambient registry, kind-name dispatch or
  `displayName` identity.
- Typed intake, closure, cycle, weakening and dispatch-ambiguity failures with
  no substitute.

## Notes

filament-core-data owns the Semantic IR lowering and reader, and quire-rs owns
artifact extraction; this task consumes both crates by exact git revision and
edits neither. Component, endpoint, participant and configuration constructs
are outside this task until their Quire meanings are decided.
