---
id: SR-1318
title: "IR-601 spec review of PR #639 (FR-096, FR-100, TC-428, TC-452 member-only division)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@1d27057837c187c1106d7445e7e806faa9aa669b; PR #639 diff against origin/main (merge base bcca43356); spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md; spec/functional/FR-100-run-a-named-function-through-the-spine.md; spec/test-cases/TC-428-a-refusal-record-carries-code-category-locus-and-fields.md; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md; QSpec spec/functional/expressions/FR-147-evaluate-integer-division-domains.md at quire-specification 2b2dd28 (context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
---
## Summary

Ticket: IR-601. PR: quire-spec-language#639. This is a spec review of the four
edited artifacts against QSpec FR-147 at quire-specification 2b2dd28 (QSpec
#188).

- **FR-096 is the right consuming FR.** It owns the kernel-refusal key table and
  `kernel_refusal_record` (the code, the cause and the `expected` field). The
  division law itself is consumed from QSpec FR-147 directly by the TC-192
  tests, and FR-096 already lists FR-147 among its references (line 54). No
  other QSL FR restates the law.
- **The text matches QSpec FR-147.** FR-096 now says that `DivisionOutOfDomain`
  takes its cause from the exposed member (`quotient-outside-domain` for
  `div`, `remainder-outside-domain` for `rem`), that both members are
  computed exactly, and that it refuses only when the exposed member is outside.
  This matches FR-147's member table and "A bounded consumer checks only the
  exposed member". The code is `division_out_of_domain`, as in FR-147-AC-9.
  The key-table row, the value-refusal list, AC-13 and the implementation note
  all agree.
- **FR-100, TC-428 and TC-452** say two causes and one record per member, with
  no leftover pair wording.
- **Pair-rule removal is complete.** A spec-wide grep for the pair-rule terms
  finds none at the head.
- **EARS and atomicity.** The FR-096 statement edit keeps its SHALL form.
  AC-13 is a single testable record-level claim. It states no operator-level
  evaluation that QSL has not built (qsl-eval has no `div`/`rem` arm,
  FR-091-OQ-5).

## Verdict

Approve. The edited spec text is consistent with QSpec FR-147 and with the
code, and has no defect.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
