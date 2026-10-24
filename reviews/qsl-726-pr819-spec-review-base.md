---
id: SR-9056
title: "QSL-726 spec review of PR #819 FR-302 and tests.md edits"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@e11f3fc5269d895d4cf8a97ca9f54082684ae538; spec/functional/FR-302-link-dispatch-during-unit-compile.md; spec/tests.md; context: spec/functional/FR-083-resolve-unique-most-specific-dispatch.md, spec/functional/FR-272-fail-on-a-second-definition-of-a-canonical-type.md, spec/test-cases/TC-794, TC-809; upstream QSpec FR-151, FR-272, TC-196 (read-only)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: reviews
---
## Summary

Ticket: QSL-726, AC-4. This review covers the FR-302 sentence that states the record shape and order of a failed link, and the TC-794 and TC-809 status flips in `spec/tests.md`.

The record shape, one record per failing subtype, agrees with QSpec FR-272's `ambiguous_dispatch` row ("the closed subtype for which selection fails") and with QSpec TC-196 D03 (four records for one operation). The tests.md flips are backed by tests for every TC step (see SR-9055).

## Verdict

**FINDINGS.** There are two medium spec defects in the one sentence the PR adds. Both are text fixes. FND-002 may also need a small code change, depending on which order is chosen.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new sentence cites "one FR-272 record" without a qualifier. In this repo, FR-272 is QSL's own `FR-272-fail-on-a-second-definition-of-a-canonical-type`, which is unrelated. The record meant here is QSpec FR-272's `ambiguous_dispatch` row. FR-302 itself writes "QSpec FR-151" everywhere else. Write "QSpec FR-272". The same bare citation appears in the new doc comments at `refusal.rs:648-652` and `dispatch_link.rs:537-539`. | spec/functional/FR-302-link-dispatch-during-unit-compile.md:126; qsl-semantics/src/check/refusal.rs:648-652; qsl-semantics/src/check/dispatch_link.rs:537-539 |
| FND-002 | medium | The order clause, "grouped by family in call order and by subtype in enumeration order", has two problems. First, it uses undefined terms: neither FR-302 nor FR-083 defines "call order" or "enumeration order". Second, the family order contradicts QSpec FR-151, which reports every failing subtype of every called operation "in charge order". QSL charges families in alias-then-root-key order (`called_families` sorts the roots), but `PendingLinks::report` emits each family at its first call. In the TC-794 step 3 unit, `area` is charged before `size`, yet `size` is reported first. Fix: report families in charge (link) order, or get the QSpec owner to accept call order. In either case, name the subtype order as ascending effective identity, which is what the bridge does and what QSpec TC-196 D01/D03 fix. | spec/functional/FR-302-link-dispatch-during-unit-compile.md:121-129; qsl-semantics/src/check/dispatch_link.rs:75-101; qsl-semantics/src/check/dispatch_link.rs:219-246 |
