---
id: SR-202611071706001
title: "base review of QSL-1706 related-end position names (FR-383, FR-533, TC-1063, TC-1333)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; branch spec/qsl-1706-related-names diff origin/main...HEAD; FR-383 Protocol term locations and FR-383-AC-64; FR-533 slot naming and FR-533-AC-21; TC-1063 row L4e; TC-1333 row 21"
review_set: base
---

## Summary

The provisional-site rule is sound in its main line: the checker locates a `related` end at `protocol <P> run <path> related <start> <end>`, lowering re-locates the end's checked term to `related <k> from|to` before `check` returns, and both routes therefore read the same origin. FR-533-AC-13 is untouched and its exact parity is preserved; the new text strengthens it by putting the source route on the position name. Two medium gaps: the collapse rule keeps the occurrences and source-map rows of a collapsed repeat, which conflicts with FR-383-AC-13's "same bytes as the source without the repeat" and is not pinned by AC-64's third compile; and FR-533's dependency note still says no compiled unit can hold the `related` condition, which AC-64/AC-21 now disprove. The rest are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Collapse rule keeps each occurrence and source-map row of a later repeat's ends (FR-383:268-273), but a repeated `related by` must emit the same bytes as the source without it (FR-383-AC-13, FR-383:463-464); with a call end the repeat adds `expression` occurrences to the call node (FR-383:313-316). AC-64's third compile asserts only origins and per-origin row counts, so it does not catch this. Fix: state that a collapsed repeat's end terms record no occurrence and no source-map row (or that the kept ones are absent from the package), and add to AC-64/L4e that the repeat compile's package holds the same occurrences of the `same(view)` node and the same bytes as the compile without the repeat. | FR-383:263-273; FR-383:313-320; FR-383-AC-13; FR-383-AC-64; TC-1063 L4e |
| FND-002 | medium | FR-533 dependency note ends "The vectors of FR-533-AC-13 are on package copies because no compiled unit can hold the condition", but FR-383-AC-64 and FR-533-AC-21 compile a unit whose two records' `from` ends are one call node. That holds only for the case-guard half. Fix: reword to "The case-guard vectors of FR-533-AC-13 are on package copies because no compiled unit holds two case guards with one walk; the `related` half also holds on a compiled unit (FR-533-AC-21)." | FR-533:362-368; FR-533-AC-13; FR-533-AC-21 |
| FND-003 | low | The refusal clause works per unit ("no record position exists for a unit that does not check"), but results are per protocol (FR-383 Per-protocol results). A `Refused` aggregate's payload keeps the results of protocols that did emit (FR-383:549-552). Fix: say that a protocol refused before lowering keeps provisional sites in its refusal locus only, and that every protocol that lowers, including one in a unit whose aggregate is `Refused`, carries position names. | FR-383:278-280; FR-383:528-552 |
| FND-004 | low | The new text says "WHEN lowering writes a control's `related` records sorted and collapsed", while "Order, sets and collapse" gives the sort and collapse to the emitter (FR-383:442-444, 458-464). Node keys are minted in `check` (FR-383:388), so lowering has to build the sorted body, but the two sections name different actors. Fix: write "WHEN `check` lowers a control's `related` records in the order and collapse the emitter writes (Order, sets and collapse)". | FR-383:263; FR-383:388-393; FR-383:442-444 |
| FND-005 | low | FR-383-AC-64 is not atomic. It packs position names in two source orders, the leaf `to` having no row, order invariance of `k`, one-end-per-origin, absence of provisional origins in both the checked package and rows, and repeat invariance into one criterion, so one failure does not tell you which property broke. Fix: split it into (a) position names and order invariance, (b) no `related <start> <end>` origin in the checked package or rows, (c) repeat-compile invariance (merged with FND-001's byte assertion). | FR-383-AC-64; TC-1063 L4e |
| FND-006 | low | The two-record variant's function `same` is defined only in AC prose. TC-1070 fixture L has no such function, and the existing source-route test spells the identity function `identityRef` (`function identityRef using v(x: M::Node): M::Node pure { x }`, tests/it/bare_equality.rs:420). Fix: give the variant's exact source edit (the function declaration and both `related by` lines) in TC-1063 L4e and reference it from TC-1333 row 21, using one function name. | FR-383-AC-64; FR-533-AC-21; TC-1063 L4e; TC-1333 row 21 |

## Scope and evidence

No applicable AssuranceProfile was found in the repository, so this is a direct base review of the diff only. `quire validate --scope .` over the four changed files reported no errors (module-registry warnings only).

Checked and found consistent:
- FR-533-AC-13 text is unchanged. The added FR-533 paragraph and AC-21 strengthen parity by requiring the source route to carry the position name too. Nothing weakens it.
- Order invariance: both records share relationship `Owns` and `from` end `same(view)`, so their emitted order is decided by the `to` end's canonical bytes (`view` reference vs `same(view)` call). That order does not depend on source order, which makes AC-64's "same emitted position in both orders" well-defined.
- The `same(view)` node is held by three slots (two `from` ends and the call `to` end); by FR-383:313-316 it gets one occurrence per position, so each origin's rows form their own groups with `term_ordinal` 0. This is consistent with AC-58 and AC-61.
- The reference `to` end `view` is a leaf with no row (Assumption A1), as AC-64 states.
- The provisional form `related <start> <end>` matches the current checker site (qsl-semantics/src/check/lowering/protocol.rs:155-160). Re-location changes only the origin name and keeps child-index paths.
- TC-1063 scope now lists AC-63 and AC-64; TC-1333 scope and trace tag now run to AC-21.
