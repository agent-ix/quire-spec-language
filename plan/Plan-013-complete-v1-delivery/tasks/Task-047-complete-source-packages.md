---
id: Task-047
title: "Implement complete source packages, grammar and lossless CST"
type: Task
status: done
track: A01
priority: P0
relationships:
  - target: ix://agent-ix/quire-spec-language/Task-046
    type: depends_on
  - { target: ix://agent-ix/quire-specification/FR-131, type: references }
  - { target: ix://agent-ix/quire-specification/FR-134, type: references }
  - { target: ix://agent-ix/quire-specification/FR-302, type: references }
  - { target: ix://agent-ix/quire-specification/FR-303, type: references }
  - { target: ix://agent-ix/quire-specification/FR-339, type: references }
  - { target: ix://agent-ix/quire-specification/TC-180, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-184, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-222, type: verifies }
---
# Task-047: Implement complete source packages, grammar and lossless CST

## Scope

Execute QSL #117: complete the edition/profile/package selection, normative
grammar, identities, imports, lossless CST, formatter and incremental diagnostic
surface over the existing diagnostic code type, without allowing a backend to
change admission.

## Subtasks

- [x] Write the TC-180/184 and TC-222 corpus plus formatter/editor reparse,
      exact-token, limit, parity and refusal vectors first; leave checked
      semantic identity and concrete TC-223 to Task-054.
- [x] Implement all grammar productions, exact source-package identities and source-locus preservation.
- [x] Implement byte-exact CST round-trip, recovery separation and incremental parity.
- [x] Pass local Rust gates and PR-time Rust/gap review; prepare the merge that
      must land before Task-048.

## Deliverables

- Complete source/package semantic graph and editor-grade CST APIs.
- Exact typed refusal for unknown syntax, profiles, features and stale edits.

## Notes

This is QSL #117. Existing parser and composed-linking evidence remains credited
but cannot substitute for the complete grammar and CST corpus.
TC-181 remains allocated to Task-049; TC-220/221 remain allocated to Task-052.
FR-132/133 and TC-182/183 require the concrete complete checked-package and
extension-semantic authorities produced only after semantic/runtime delivery,
so they remain allocated to Task-054; syntax/package-graph evidence must not
self-attest those later authorities. Undeclared syntax still refuses here under
FR-339-AC-4. FR-270–272 and TC-047 likewise remain in Task-054: the proposed
native diagnostic interpretation requires exact catalog-source bytes and closed
typed causes, neither of which this source-admission task may self-attest. The
formatter provides exact-catalog reparse and token correspondence here; Task-054
adds an unforgeable checked-package identity and runs TC-223.
