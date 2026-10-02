---
id: SR-1072
title: "QSL-381 scope-boundary review of PR 577: overlap items IR, CG, RT, filament, quire-canonical"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language@9d240032be4eac1df486d3c224cbd48273053dc5; spec/decisions/ADR-030 D-4.5, D-7, D-8; spec/functional/FR-259, FR-260, FR-264"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-259
    type: reviews
---
## Summary

Ticket: QSL-381. Check 3 of the brief: overlap items are named, not
specified. O-1 (IR), O-2 (CG), O-3 (RT) and O-5 (QSpec) name the work and
the owner. FR-264's Overlap section names IR's work without designing it.
quire-canonical is a separate repository (git dependency in Cargo.toml).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | D-8 opens "Named here, not designed", but O-4 designs Filament's internals: cycle search "becomes an iterative three-colour depth-first search over an explicit stack. It visits each type once and reports a cycle when it meets a type still on the stack". FR-260's Overlap repeats the algorithm. Keep the outcome (cycles of any length reported, tested by FR-260-AC-3) and drop the algorithm. | spec/decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md:496-511; spec/functional/FR-260-admit-semantic-ir-documents-at-any-depth.md:70-73 |
| FND-002 | medium | quire-canonical is specified in QSL, not named. ADR-030 D-4.5 sits under "Components in QSL's lane" and designs the crate (event API method names, pull reader, arena tree, FixedShape marker). FR-259 "What QSL consumes" states that API with SHALL, and FR-259-AC-3 tests the crate's own reader behaviour (lone surrogate, 1e400 refusal with byte offset) from QSL. Move D-4.5 under D-8 O-6 as a named item, have FR-259 depend on the capability by name, and keep only QSL's own behaviour (byte error mapping, malformed-input mapping, identity routing) as QSL ACs. | spec/decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md:271-305; spec/functional/FR-259-encode-identities-and-read-json-through-quire-canonical-at-any-depth.md:29-49,81 |
| FND-003 | medium | D-7 lays out the ecosystem delivery order across QSpec, quire-canonical, the semantic-IR crate, IR, CG and RT (steps 1-5) in a public repository. Ecosystem delivery order belongs in quire-research. QSL's own sequencing (3a-3d) is fine; reduce steps 1-5 to the dependencies QSL's slices wait on. | spec/decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md:434-444 |

## Verdict

Changes requested. Check 3 fails for Filament (O-4) and quire-canonical
(D-4.5, FR-259). IR, CG and RT are correctly named only.

## Dispositions

Round 1, reviewed at 76e47d3a96aacde91f7964e872d50dcd87dfbc15.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 16a9154f: O-4 and FR-260 name cycle reporting as Filament-owned. |
| FND-002 | fixed | 16a9154f: ADR-030 D-4.5 and FR-259 name the quire-canonical capabilities QSL relies on. |
| FND-003 | fixed | 16a9154f: D-7 is QSL's slices and what each waits on. |
