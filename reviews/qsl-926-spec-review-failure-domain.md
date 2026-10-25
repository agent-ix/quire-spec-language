---
id: SR-17195
title: "QSL-926 PR #1524 failure-domain review of the on each binder and guard"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language; spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md, spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: reviews
---

## Summary

Ticket: QSL-926. PR: quire-spec-language#1524 (head a8b073ce6). This lens checks identity confusion, unstated failure modes and edge cases of the `on each` binder and its guard.

The guard's failure paths are stated and agree with the `holds` path: false, undefined (`absent-key`, kernel), refused, unobservable and meter. So is the guard-before-body refusal order, and the absent-guard vs `when (true)` identity. A probe confirmed that S3 today has no duplicate-name or type rule for the `on each` binder: `on each (c: Config::ConfigVersion)` over `(c: Config::ConfigVersion)` and `on each (q: Integer)` both reach `NotEmitted`.

Examined units: FR-337 "`on each` activation at S3" and "at S4", FR-337-AC-7, FR-125 "The `on each` activation guard".

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Q4. Nothing says what happens when the `on each` binder has the `over` parameter's name, `over (c: M::Counter)` `on each (c: M::Counter)`. As written, S3 admits it. A guard read of `c` then resolves to the `over` parameter, and S4 emits two `value`/`parameter` nodes named `c`, at levels 0 and 1. Under SR-17190 FND-002's alias ruling the two names would be in one scope. Two implementers could admit it or refuse it. Ruling: refuse `conflicting_declaration`/`duplicate-declaration` at the binder, as FR-447 refuses a repeated capture name, and add the case to FR-337-AC-7 and TC-1265 step 3. | FR-337:140-160, FR-337-AC-7 |

## Verdict

One medium identity gap. Fix it with SR-17190 FND-002.
