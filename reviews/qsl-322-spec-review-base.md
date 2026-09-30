---
id: SR-815
title: "QSL-322 spec review of PR 536's ADR-013 edit (O-25, OQ-H)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@b95b671a01f151632be03adb078b83d7280d51ae; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md (O-25 decode-types paragraph, OQ-H row); PR #534 head 17b7269c ADR-013 OQ-H (context, read only)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
## Summary

Ticket: QSL-322. PR: quire-spec-language#536 at b95b671a.

The ADR edit makes two changes. The O-25 paragraph now states the concrete
decode shape: a `WitnessBinding` names its parameter node id and a
`WitnessValueType` (`Boolean` or `I64`), `decode` returns one
`WitnessValue` (`Boolean` or `Integer`) per binding in binding order, and
the `Input` arm carries the same `WitnessValue`. The OQ-H Reason cell drops
"except the three `decode` types, which QSL-322 adds". Both match the code
at head.

I checked the edited text against the code (witness.rs, execute.rs) and
against PR #534 (QSL-323, open, head 17b7269c), which also edits the OQ-H
row.

## Verdict

The O-25 edit is accurate. There is one content conflict with PR #534 in
the OQ-H row, which whoever merges second must reconcile by hand (FND-001).
There is no other spec defect in the diff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | OQ-H content conflict with PR #534. This PR keeps the old Ruling ("IR deletes its copies and names QSL's; ... IR keeps `KaniOutcome` and maps it into `TerminalValue` (C-09)") and changes the Reason's last sentence to "QSL builds every one of these types (FR-069 to FR-072, O-25)". PR #534 rewrites both cells: the Ruling becomes "IR names no QSL type. CG ... maps IR's `KaniOutcome` into `TerminalValue` (C-09), builds the O-25 packet ...", with a new Reason. Its Reason still ends "except the three `decode` types, which QSL-322 adds (O-25)". The correct merged row is #534's Ruling and Reason with this PR's last sentence. Taking either side whole either restores the stale "QSL-322 adds" claim or drops the QSL-323 ruling. This PR's new identity.rs and lib.rs docs (SR-813 FND-001) already contradict #534's ruling. Fix: whichever PR lands second rebases and hand-merges the OQ-H row as above. Better, this PR stops touching the Ruling-adjacent text and #534 carries the sentence change. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:1221 |
