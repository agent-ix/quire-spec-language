---
id: SR-60031
title: "spec-review of the FR-126, FR-281, FR-339 and TC-1085 edits of QSL-1099 (quire-spec-language PR 1603)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@3414559bd129af5c60dfe512bdbbe7a353031f59; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md, FR-281-analyze-claims-with-in-process-engines.md, FR-339-check-an-en-1-component-certificate.md, spec/test-cases/TC-1085-liveness-model-only-pass-and-unsupported-refusals.md"
review_set: subset
date: '2026-10-26'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-339
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: reviews
---
## Summary

Ticket: QSL-1099. The NotBuilt text is removed from FR-126 (Behavior, AC-13), FR-281's refusal row and TC-1085 step 3. A grep of src, spec, docs, plan and xtask for ComponentCertificateNotBuilt, component-checker-wired, "not built" near FR-339 or the component certificate finds no stale use outside historical reviews/ files; TC-1040 is an unrelated FR-107 case and carries none. The FR-339 each-order sentence (union over every binding's transitions, ascending FR-101 key bytes, producer and checker alike) matches the code (universe in component.rs, recorded.transitions in the checker). Budgets: FR-339 Behavior specifies that component work charges none of max_states, max_transitions, max_automaton_states and not the meter, and AC-6 verifies it. The liveness verdict changes (NotBuilt now Holds with a certificate) were checked: every converted test now runs the core checker on the certificate (Fixture::certified), except the tso Spin test, which does not (I ran it; Accepted). No previously refused case is silently passing: no test case changed its claim, only its expected shape.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-126 states a TP-4 item SHALL NOT settle Holds without the accepted ComponentCertificate, but FR-338/FR-339 do not say the checker refuses a ClosureCertificate for a TP-4 item, and it does not (SR-60029 FND-001). Add the refusal to FR-338 or FR-339 Behavior with an AC and TC row. | spec/functional/FR-338-check-an-en-1-closure-certificate.md | missing-requirement |
| FND-002 | low | FR-339 (protocol vectors) and FR-126 leave counted/interval protocol TP-4 clauses (translate versus translate_counted) unspecified for the producer; state the expected verdict or the refusal until it is built. | spec/functional/FR-339-check-an-en-1-component-certificate.md | missing-requirement |

## Verdict

PASS on the wording edits; FND-001 is the same defect as SR-60029 FND-001.
