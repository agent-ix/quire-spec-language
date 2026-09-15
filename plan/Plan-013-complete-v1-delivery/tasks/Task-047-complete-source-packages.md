---
id: Task-047
title: "Implement complete source packages, grammar and lossless CST"
type: Task
status: not_started
track: A01
priority: P0
relationships:
  - target: ix://agent-ix/quire-spec-language/Task-046
    type: depends_on
  - { target: ix://agent-ix/quire-specification/FR-131, type: references }
  - { target: ix://agent-ix/quire-specification/FR-132, type: references }
  - { target: ix://agent-ix/quire-specification/FR-133, type: references }
  - { target: ix://agent-ix/quire-specification/FR-134, type: references }
  - { target: ix://agent-ix/quire-specification/FR-302, type: references }
  - { target: ix://agent-ix/quire-specification/FR-303, type: references }
  - { target: ix://agent-ix/quire-specification/FR-339, type: references }
  - { target: ix://agent-ix/quire-specification/TC-180, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-181, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-182, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-183, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-184, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-220, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-221, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-222, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-223, type: verifies }
---
# Task-047: Implement complete source packages, grammar and lossless CST

## Scope

Execute QSL #117: complete the edition/profile/package selection, normative
grammar, identities, imports, lossless CST, formatter and incremental diagnostic
surface without allowing a backend to change admission.

## Subtasks

- [ ] Write the TC-180–184 and TC-220–223 positive/boundary/refusal corpus first.
- [ ] Implement all grammar productions, exact manifests and source-locus preservation.
- [ ] Implement byte-exact CST round-trip, recovery separation and incremental parity.
- [ ] Pass local Rust gates and PR-time Rust/gap review; merge before Task-048.

## Deliverables

- Complete source/package semantic graph and editor-grade CST APIs.
- Exact typed refusal for unknown syntax, profiles, features and stale edits.

## Notes

This is QSL #117. Existing parser and composed-linking evidence remains credited
but cannot substitute for the complete grammar and CST corpus.
