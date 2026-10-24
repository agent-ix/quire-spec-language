---
id: SR-15441
title: "Integrity review of the ADR-029 PL-7 and ADR-013 amendment edits for QSL-655 (PR #1210)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@1892b8760472c2080c2d81b54ec08f47a9ff6a87; spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
# SR-15441: Integrity review of the ADR-029 PL-7 and ADR-013 amendment edits for QSL-655 (PR #1210)

## Summary

Ticket: QSL-655. Integrity (completeness, consistency, atomicity) of the
diff `origin/main...1892b8760`. EARS analysis was not run: the diff is ADR
prose with no FR or NFR statement and no acceptance criterion.

What holds:

- The two new `relates_to` targets resolve (QSL FR-289, QSpec FR-462) and the
  gate passes.
- The ADR-013 amendment is correct for O-25 and C-09. O-25's replay-source
  paragraph calls an `Input` counterexample "a corpus counterexample" that
  "never counts as backend evidence", and C-09 says "an `Input`-arm result
  comes only from a corpus counterexample". The amendment widens the source
  to a plugin's counterexample, keeps the corpus rule, and says why a
  plugin `refuted` is still not CG backend evidence (PV-4 ruling 3). That
  agrees with PL-7 and PV-2 item 4.
- The rule 1 sub-bullets are atomic: one replay outcome each.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The amendment names O-25 and C-09 only, but ADR-013 QC-8 and QC-20 restate the same blanket rule ("an `Input`-sourced replay settles `reproduced-without-witness` and never counts as backend evidence"). A reader of QC-8 (the QSpec FR-323 and FR-331 handoff) or QC-20 (the AD-016 amendment) gets the old rule. Add QC-8 and QC-20 to the amendment bullet's heading. | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:915-920 |
| FND-002 | low | The last sentence of "Refused request body" ("A refused plugin result (QSL-only cause or core-only basis) settles the same way, with the refusal in `diagnostics`") states a rule for rules 2 and 3 inside rule 4's bullet. A reader of "Trusted proofs" or "QSL-only causes" does not find where the diagnostic goes. Move it into those two bullets, or into a bullet of its own. | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:633-636 |

## Verdict

No integrity blocker. Both findings are minor. The blocking findings for
this PR are in SR-15440.

## Dispositions

Disposition pass 1 at head `cd4a79867e4282166a30d899005e82b77b83043f`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | After: ADR-013 O-25, C-09, QC-8 and QC-20: a `ReplaySource::Input` counterexample is a corpus counterexample or a plugin's counterexample (PL-7). Each states its `Input`-sourced replay rule as: an `Input` replay settles `reproduced-without-witness`; a reproduced corpus counterexample never counts as backend evidence; a plugin's counterexample whose replay reproduces the violation settles `refuted` naming the plugin (PL-7) and is not CG backend evidence ... |
| FND-002 | fixed | The sentence is removed from "Refused request body", which now ends "cause, not in the terminal record."; the rule is its own "Refused plugin results" bullet, which "Trusted proofs", "QSL-only causes" and "Counterexamples" each point to by name. |
