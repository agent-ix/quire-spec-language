---
id: SR-1676002
title: "gap analysis of PR #1676 (QSL-580m) against FR-230, FR-181, FR-208 and QSpec FR-415"
type: SpecReview
analysis: gap-analysis
date: 2026-10-27
scope: "agent-ix/quire-spec-language; PR #1676 git diff 1ed0d0bee..27a6266a4; FR-230-AC-16, AC-17, AC-19, AC-21, AC-23, AC-26 (alias half); FR-181, FR-208 (new record kinds); QSpec FR-415 Cardinality bound and FR-415-AC-13; TC-685"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-208
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-181
    type: reviews
---
# SR-1676002: gap analysis of PR #1676 (QSL-580m)

## Summary

Ticket: QSL-580. Head reviewed: 27a6266a4. Method: each AC checked against its tests, the PR's claims re-measured with probes, and the gates run. Plan completion: not assessed.

| Requirement | Measured |
| --- | --- |
| FR-230-AC-16, sequence and alias halves | Met: `ks.[slot]` and `s`. The map half is deferred (no map value type, IR-1046). |
| FR-230-AC-17 | **Partial.** The alias, sequence and field halves are met. Not built: the parameter and result typed by the alias, and the ghost-unit refusal (FND-003). |
| FR-230-AC-19 / AC-23, sequence type | Met: `q.[slot].x` with `max: None`, and `qs.[slot].[slot].x`. The map half is deferred. |
| FR-230-AC-21, families and paths | Met. Probes found family semantics correct, including the empty collection (SR-1676001). |
| FR-230-AC-26, alias half | Met as briefed: an alias with constraints is refused `malformed-declaration`. The domain `0..10` waits for a bounded quantity in quire-exact. |
| QSpec FR-415, Cardinality bound | **Not met.** FND-001, FND-002, and SR-1676001 FND-001. |
| FR-208 / FR-181 | Met. The two new record kinds are `DeclarationKind::ValueType`, with no FR-208 construct meaning and no frame entry; every `DomainPackageRecord` match is extended, and identity follows the existing record-key path. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | **MaxClocks stops on a static listing, not on live clocks.** QSpec FR-415 says an analysis stops when "a state whose live clocks exceed the model checker's `max_clocks` limit" is reached. The PR instead fixes one `listed` for every unbounded collection, computed so that every bounded place counts at its declared maximum and every unbounded collection at `listed`. It stops when any one collection exceeds `listed`. So a state whose live clocks are well under `max_clocks` stops `MaxClocks` when bounded collections are declared large but held small, or when one unbounded collection is full and another is empty. The result is conservative, an early `incomplete`, but it is not the specified criterion. The PR's test at `max_clocks` 7 happens to coincide with the specified count. Count the live places of the state, and keep the static listing only to size the place vector, or state the static rule in FR-230. | qsl-eval/src/simulation/model/timed.rs:466-509,564-628 |
| FND-002 | medium | **`max_clocks` is not wired to the request or the result.** FR-415-AC-13 requires `limits.max_clocks` = 2048 to raise the bound, `0` to be refused, and a stop to settle `incomplete` `limit-reached` naming `max_clocks`, its value and `model_check.max_clocks`. EN-6, the zone certificate checker and replay all call `TimedSystem::new`, which is always 1024. EN-6 reports `ZoneUndecided::UndecidedSuccessor(MaxClocks)`, not FR-331's `limit-reached`. If this half is deferred, name the ticket that carries it. | qsl-analyze/src/zone_check/search.rs:660-662; qsl-replay/src/certificate/zone_certificate.rs:672; qsl-replay/src/model_check/replay/timed.rs:54 |
| FND-003 | low | **FR-230-AC-17's refusal half is not built.** An `alias` or `sequence` naming `ix://quire/unit/ghost` is admitted by intake: the probe gets `AliasType{target: Unit("ghost")}` and `SequenceType{items: Unit("ghost")}`. AC-17 requires `missing_declaration`/`missing-name` at `TypeDefinition.target` or `.items`. An unused ghost alias is never refused, and a used one fails later in assembly with `QuantityName`, not the AC's refusal. The parameter and result typed by the alias are untested. Also, AC-17 writes the unit as `ix://quire/unit/acme/rpc/ms`, which `read_value_type_ref` refuses because the name contains `/`. That is existing behaviour, but the AC and the reader disagree. The PR notes the refusal as "Not covered". Give it a ticket. | qsl-semantics/src/model/intake.rs:2959-2973,4222-4255; spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md (AC-17) |

## Verdict

**Not mergeable.** The QSpec FR-415 Cardinality bound is not met: SR-1676001 FND-001 (high) and FND-001 here. FND-002 may be deferred if a named ticket carries it. FND-003 needs a ticket.
