---
id: SR-947
title: "Integrity review of PR #557 (delete RevisionPin from ADR-013 and ADR-011)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@cdbc1fdb1077fe1432e7903b23af9af2bb1b1dc4; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md"
review_set: subset
---

## Summary

Ticket: QSL-361. Cross-reference consistency of the deletion across the whole
repo at cdbc1fdb.

- `RevisionPin`: the only remaining hit is `reviews/qsl-354-spec-review-base.md:42`,
  a committed review record (history, not a spec claim). No ADR, FR, doc or
  code names it.
- `T-9`: remaining spec hits are ADR-011:1045 (X-1 row, "T-9
  (agent-ix/quire-contract-runtime#55) then retargets RT `qsl-agreement`"),
  ADR-011:1400 (ADR-011 "Tickets to open" T-9 row, the same retarget) and
  ADR-013:1220 (TK-03, "the RT agreement retarget ADR-011 T-9"). All three are
  ADR-011's own T-9 ticket, a different id scheme from ADR-013 §3.1. Other hits
  are in `spec/reviews/` (history).
- Question counts: ADR-013 §3.1 now has eight rows T-1 to T-8, matching
  "leaves eight questions" (ADR-013:929) and "The eight ADR-011 questions ...
  (T-1 to T-8)" (ADR-013:1321-1322). ADR-011's answered paragraph lists T-1 to
  T-7 plus T-8, also eight. No other spec text counts these questions; the
  only other "nine"/"T-1 to T-9" text is `spec/reviews/type-ownership/base.md:81`
  and `spec/reviews/arch-g1-change-scenarios/base.md:54`, both review history.
- OBS-034 is still traced: ADR-013 O-23 heading keeps it, ADR-013 §9 row
  points to O-23, ADR-011 §9 keeps the secondary row, and ADR-010's
  definition is unchanged.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. No dangling `RevisionPin` or ADR-013 T-9 citation remains outside
review history, the eight-question count is consistent in both ADRs, and the
ADR-011 T-9 hits left in place refer to ADR-011's own qsl-agreement retarget
row.

## New findings (disposition pass 1)

Delta re-review of `git diff cdbc1fdb..74bb8f8e` at 74bb8f8e (round 0 at
cdbc1fdb was clean). The delta deletes the O-23 release-pins paragraph
(ADR-013:808-810) and the §5 bullet "Exact release pins are authoritative
(O-23)" (ADR-013:1017), and rewords the OBS-031 secondary row (ADR-013:1268).
No dangling citation of the removed text remains: `git grep -i 'release pin|qualified dependency|ecosystem lock'` at 74bb8f8e hits only review history,
FR-101:67 and Plan-013 plan.md:115 (both outside this delta). The new OBS-031
sentence is true: each lock resolves its own dependency revisions.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | O-23's kept row gives the authority as "Each repository's `Cargo.toml` and `Cargo.lock` exact `rev`", but first-party dependencies are `branch = "main"` with no `rev` (Cargo.toml:72-75 says "no rev" outright), which is the premise of this delta's ruling. With the release-pins paragraph gone this row is O-23's only statement, and the reworded OBS-031 row cites it. Round 0 examined this row and missed it. Fix: authority is each repository's `Cargo.toml` (`branch = "main"`) as resolved to one revision in its `Cargo.lock`. | ADR-013:806, ADR-013:1268, Cargo.toml:72-75 |
| FND-003 | low | ADR-010 OBS-031 observes that QI has no `heads/` workspace and no current-head integration lane. The reworded row ("each repository's `Cargo.toml` and `Cargo.lock` select its dependency revisions") is accurate but does not say how that answers the observation; the old row did, by naming release pins as the only qualified selection. Fix: say that first-party dependencies track `main`, so each lock already resolves current heads and no separate heads lane exists. | ADR-013:1268, ADR-010:988, ADR-011:1126 |
