---
id: SR-368
title: "Failure-domain analysis of own-attempt Boolean choice admission"
type: SpecReview
analysis: failure-domain
scope: "spec/functional/FR-042-publish-compiled-protocol-artifacts.md; docs/compiled-protocol-v1.md; src/protocol_artifact/native/families/decisions/received.rs; tests/native_choice_emission.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

The extension preserves the trust boundary: an attempt is an observed record,
not evidence its operation succeeded or an effect occurred. Exact structural
role resolution, original binder/control anchor and the necessarily-produced
flow gate keep identity and topology strict.

## Verdict

**PASS (scoped)** — no new identity, purity, trust-boundary or topology failure
mode is demonstrated. SR-365 records the subsequently completed full gates and
separate Opus code/Rust recheck.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No attempt/effect identity confusion found: only Attempt role identity is admitted, other event kinds return false, and the positive fixture asserts no Effect binding/control is minted. | received.rs:137-152; native_choice_emission.rs:252-272; FR-042:102-104 |
| FND-002 | low | No same-model ownership widening found: exact structural role identity is compared to the choice owner and the foreign same-model test refuses FamilyProof. | received.rs:142-148; native_choice_emission.rs:356-378; FR-042:102-103 |
| FND-003 | low | References exhaustion, one-short refusal, exact success, source locus and fresh retry are tested; other AC-9 dimensions remain inherited general debt. | native_choice_emission.rs:57-164; FR-042:237-244 |

### Boundary and topology check

The resolver requires an EventRecord binder anchored to a real control and the
existing read/flow map to use the choice evaluation anchor. The attempt arm
resolves only the attempt role reference at that control. A parallel sibling
therefore cannot become visible by text order, while all-joined attempts remain
distinct atoms by binder/anchor identity. The formula is pure: it uses only the
Boolean record field and does not execute an operation or infer a result, effect,
delivery or compensation.
