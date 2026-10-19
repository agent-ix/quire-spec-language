---
id: SR-12890
title: "Base review of PR #985: ADR-018 records the NoInstance cause"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@3e1272e03; ADR-018 V-6 row (line 151) and InconclusiveCause list (lines 163-167); FR-125 Outputs and Instancing; FR-127 InconclusiveCause sketch and V-6 row; FR-126 Undecided causes; FR-169 V-6 row; ADR-016 'Amended by FR-380' notes; agent-ix/quire-specification@4f65ed2 FR-331 inconclusive causes row and FR-331-AC-28"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: reviews
---

## Summary

Ticket: STD-174. PR quire-spec-language#985, diff `origin/main...3e1272e03`, one
file: two amendment notes in ADR-018, one at the end of the V-6 row and one
in the InconclusiveCause paragraph. Checked the note format against
ADR-018's existing "Amended by ..." sentences, the cited sources, and whether
an ADR or an FR should be cited.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The two notes cite different sources for the same change: the V-6 row says "Amended by FR-125 and FR-127", the causes paragraph says "Amended by FR-125". The paragraph adds an enum member, which FR-127's `InconclusiveCause gains` sketch (FR-127:92) declares, so FR-127 is the more direct source there. The paragraph's "it also gains" also follows a full stop, while the earlier "it also gains" (RU-5) follows a semicolon. Cite "FR-125 and FR-127" in both places and name `InconclusiveCause`. | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:151,167 |

## Verdict

Mergeable. One low finding.

Checked and clean:

- Format: "Amended by <source>: <member>, <gloss> (<wire/QSpec ref>)" matches
  the AE-4 and MV-1 notes in the same V-6 cell. An FR as the amending
  source has precedent: ADR-016:350 and :352 say "Amended by FR-380". No ADR
  defines Instancing. The only ADR-018 text near it is QS-5 (line 509), which
  is an open question, so citing FR-125 is correct.
- FR-125 Outputs (line 71) and Instancing (lines 112-116): `NoInstance` for
  an item with zero instances, reported before any state is explored, FR-127
  settles it `inconclusive`. The V-6 note says the same thing.
- FR-127:92 and :125 already list `NoInstance` and map it to V-6 `inconclusive`
  / `unsettled` / `Inconclusive(cause)`. This matches the row's other columns.
- QSpec FR-331 (quire-specification main, merged by #280 as 3b64766): line
  100 lists `no-instance` in the `inconclusive` causes, and FR-331-AC-28 pins
  the spelling. The wire citation is correct.
- No contradiction with ADR-018's decisions: SM-5's verdict and the V-6 row's
  `inconclusive` mapping are unchanged, and an item that was never evaluated
  is never read as holding, which matches V-6's intent.
