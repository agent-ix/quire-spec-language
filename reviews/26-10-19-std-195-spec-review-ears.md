---
id: SR-12929
title: "EARS conformance review of STD-195 FR-125 instancing statements (PR #1013)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@9a8a05e9d; spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:112-135"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
---
# SR-12929: EARS conformance review of STD-195 FR-125 instancing statements (PR #1013)

## Summary

Ticket: STD-195. PR quire-spec-language#1013 at head 9a8a05e9d. Checked the new and edited Behavior statements in FR-125 against EARS grammar: one system, one response, and an explicit trigger or state.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "The canonical order is the instance order everywhere QSL lists instances, including `ModelLetters::bindings`; no other order ... applies" is normative but has no `SHALL` and no system subject. Write it as `QSL SHALL list instances, including in ModelLetters::bindings, in the canonical order`. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:115-117 |
| FND-002 | low | The binding bullet has three `SHALL` responses (give the binding, match by population and key, return `absent-key`) plus a closing note. The third has "a read through the bound parameter" as its subject, not QSL. Split it into a ubiquitous binding rule and a state-driven `Where the population holds no object under the key whose most-specific type conforms to T, QSL SHALL return absent-key for a read through the bound parameter`, and move the note on type difference to rationale. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:125-135 |

## Verdict

The key-instancing statement ("QSL SHALL therefore instantiate a key, not a typed object") is a conformant ubiquitous statement. Its explanation reads as rationale. The two findings concern form only.
