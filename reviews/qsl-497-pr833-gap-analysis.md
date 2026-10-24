---
id: SR-9092
title: "Gap analysis of quire-spec-language PR #833: FR-086, FR-084-AC-7 and FR-300-AC-3"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b23386bb470bd029d311ccf7ed7f50b857589ea6; PR #833: FR-086-AC-1, AC-2, AC-4, AC-5, FR-084-AC-7 and FR-300-AC-3 against qsl-semantics/src/model/systems.rs, xtask/src/seam_probe.rs and the tests tagged TC-233, TC-234, TC-236, TC-241, TC-410 and TC-791"
review_set: subset
---
# Gap analysis of quire-spec-language PR #833

## Summary

Ticket: QSL-497. Plan completion: not assessed. Planless and scoped to the criteria this PR moves to done in spec/tests.md.

Each criterion has a tagged test that can fail:

- FR-086-AC-1 (TC-233): two cascades (missing direction, unknown component), the Interface kind and 4 charges. A stop-at-first mutant fails the length assertion.
- FR-086-AC-2 (TC-234): port-direction and multiplicity both fail, and interface-type does not. The unmodified pipe admits.
- FR-086-AC-4 (TC-236): the Part and the Port share a display node but have different keys. They resolve independently, and the Port refuses as Part with `actual kind Port`.
- FR-086-AC-5 (TC-241): the order test makes the key order Part < Port < Interface and expects `Port` with 3 charges. The wrong-export test covers an ordinary object type, a Part and an undeclared key.
- FR-084-AC-7 (TC-410): three supertype components, an independent SHA-256 over the JCS preimage, three distinct universes and the authored object bytes.
- FR-300-AC-3 (TC-791): S1 and S3 are reported. S2 is reported as absent by design, and an injected `state_model::` arm fails the probe.

The removed "known gap" notes in FR-086 and TC-234 described the missing interface phase and the missing combined-failure fixture. Both now exist.

## Verdict

PASS. Every criterion in scope has a test that backs it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
