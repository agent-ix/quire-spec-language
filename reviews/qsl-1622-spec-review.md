---
id: SR-2259001
title: "spec-review of quire-spec-language PR 2259 (QSL-1622, QSL-1541, QSL-1542, QSL-1366, QSL-1492)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-285-map-every-outcome-category-to-one-exit-code.md; spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md; spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md; spec/functional/FR-529-settle-a-temporal-guarantee-of-implementation-rust-natively.md; spec/functional/FR-530-carry-and-check-liveness-and-fairness-premises-in-native-composition.md; spec/test-cases/TC-770-library-outcomes-serialize-to-one-outcome-document-with-the-undefined-label-on-non-proof-outcomes-only.md; spec/test-cases/TC-1063-protocol-emission-refusals-and-read-back.md; spec/test-cases/TC-1329-implementation-temporal-guarantee-settles-from-checked-premises.md; spec/test-cases/TC-1330-native-composition-carries-and-checks-premises.md; plan/Plan-013-complete-v1-delivery/plan.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-283
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: references
---

# Spec review of quire-spec-language PR 2259

## Summary

Ticket: QSL-1622 (pointers on QSL-1541, QSL-1542, QSL-1366, QSL-1492). One spec-review pass over
`git diff origin/main...HEAD` at head 0809c1baef7faba51efcdb9e4c6a3b9a2da3d54e, with the EARS,
integrity, criterion-strength, dependency and object lenses applied by hand (no separate sub-analysis
artifact; the quoin sub-analysis skills were not run as separate tools). `quire validate` exit 0 on all
ten changed files and on `spec/**` plus `plan/**`; no EARS or AC-shape warning on a changed file.

Verified clean: ids FR-529, FR-530, TC-1329, TC-1330 are unused on origin/main (maxima FR-528, TC-1328)
and on all six other open PR heads (#2258, #2257, #2256, #2255, #2253, #2250). QSpec main 78dcf2a90:
the manifest `protocol-node-mutation-manifest.json` holds 85 rows (56 refuse, 25 admit, 4 property), 298
`later` and 27 `pending` entries, version `quire.protocol-node-mutation-manifest/v1`; `protocol-node-layout-rows.json`
holds 34 rows (26 same-id replacements of pending entries plus CH14a-1..8 replacing pending CH14a), so the
counts in FR-383-AC-20's brief are right after QSpec #615. QSpec FR-488, FR-489, TC-982 (nine cases) and
TC-983 (six cases) exist only in open QSpec #616; every label named (transfer-not-preserving, premise-mismatch,
premise-missing) is in that diff. Plan-013: all six remapped rows checked against QSpec docs/v1-capability-inventory.md
and the FRs: V1-TYPE-010/011 (presence, null, multiplicity) are FR-019-AC-2 (TC-007) and FR-143; V1-TYPE-019/020 FR-143 TC-188;
V1-TYPE-030 (reject implicit casts) FR-019-AC-4 (TC-009, "implicit lossy conversion refuses"), FR-149, FR-145, FR-143;
V1-EXPR-024/025 FR-146-AC-3 and FR-019-AC-5 (TC-010, reflection, I/O, mutation refuse). All TC ids exist and verify those ACs.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-286 contradicts itself and FR-283 on the `undefined` label. The new table row "undefined (FR-283's InsufficientData) -> undefined, cause insufficient-data" gives a monitor item the label `undefined`, but FR-286's Description ("A `monitor` clause ... never carries `undefined`"; label only on a non-proof outcome such as `execute`) and the Behavior bullet "The serialization shall write the label `undefined` on no `prove`, `analyze` or `monitor` item" forbid it, while FR-283's InsufficientData row and AC-11 require it. A conforming implementation cannot satisfy FR-286 Behavior and the table. Resolution: the label is tied to category undefined, not to the operation. Reword the Description sentence and the Behavior bullet to "no item whose category is not undefined carries `undefined`; a `monitor` item for FR-283's InsufficientData (category undefined, reason insufficient-data) carries it", keeping FR-286-AC-4 (undefined evaluation is refuted, no `undefined`) and AC-8 (pending, no `undefined`), and retitle TC-770 only if its wording needs it. Do not delete the table row: FR-283-AC-11 and FR-285's category-undefined rule require the label. | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:52-58,120,158 | wrong-requirement |
| FND-002 | medium | FR-286's Description says each `prove`/`analyze`/`monitor` item "carries its QSpec FR-331 terminal record", but the new section and AC-8 say pending and indeterminate items "carry no record". The always-present statement is not narrowed, so the two disagree for these two categories. Add the exception to the `items` bullet. | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:45-48,119-123 | wrong-requirement |
| FND-003 | medium | FR-283's MonitorVerdict table has no Indeterminate row and no `clause` member, yet FR-286-AC-8 says the same run "through the FR-283 `monitor` operation gives the same item", AC-9 and TC-770 step 8 build indeterminate monitor items, and the new section says "No other code path builds an item for a clause run" while FR-283 defines monitor records. Add an Indeterminate row (category indeterminate, exit 11, FR-501's reason) and state that the monitor operation's items are built by `from_clause_runs`. | spec/functional/FR-283-monitor-a-supplied-trace-offline.md:150-167 | missing-requirement |
| FND-004 | medium | FR-383-AC-20 and TC-1063 say `pending` entries "count as not executed", but 26 of the 27 pending entries are executed through `protocol-node-layout-rows.json` (same ids) and the 27th (CH14a) through CH14a-1..8, so the statement is false of QSpec main. The per-class count and the "fails when a row was not executed" check read only the manifest `rows`, so the 34 layout rows (all refuse) are neither counted nor required. Define the executed set as manifest rows plus layout rows (25 admit, 90 refuse, 4 property), treat a pending entry with a layout replacement as executed through it, and keep only entries with no replacement as not executed. | spec/test-cases/TC-1063-protocol-emission-refusals-and-read-back.md:63-87; spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md:565 | wrong-requirement |
| FND-005 | medium | The six remapped Plan-013 rows hold several FRs and TCs in cells whose preamble says "`Delivery test` selects exactly one existing central TestCase" and whose rows are "copied by identity from QSpec's accepted Agent-A manifest"; no other row holds a list. Either state that a row may name more than one owner or keep one central FR and TC per row and put the rest in the task files. The mappings themselves are correct. | plan/Plan-013-complete-v1-delivery/plan.md:188-193,221-222,230,241,266-267 | wrong-requirement |
| FND-006 | medium | FR-529 and FR-530 depend on QSpec FR-488, FR-489 and TC-982/TC-983, which exist only in open QSpec #616; until it merges the `ix://agent-ix/quire-specification/FR-488` and `FR-489` links dangle and TC-1329/TC-1330 read cases that do not exist. Merge QSpec #616 first (validate passes only because cross-repo targets are not resolved). | spec/functional/FR-529-settle-a-temporal-guarantee-of-implementation-rust-natively.md:1-20 | missing-requirement |
| FND-007 | low | FR-285-AC-9 cites TC-770 as verifying it, but TC-770 `verifies` only FR-286 and its Scope line lists FR-286 only, so the trace does not follow the declared relationship. Add FR-285 to TC-770's Scope and relationships. | spec/functional/FR-285-map-every-outcome-category-to-one-exit-code.md:156; spec/test-cases/TC-770-library-outcomes-serialize-to-one-outcome-document-with-the-undefined-label-on-non-proof-outcomes-only.md:15 | wrong-requirement |
| FND-008 | low | TC-1063 says the run applies the row's `edit` "(or `source_edit` or `limits` form)", but every executable row uses only a list `edit`; `source_edit` appears only in `later` entries. It also lists five `locus_basis` values and omits `not_applicable`, which 29 of the 85 rows use. Name the forms and values the executable rows actually use. | spec/test-cases/TC-1063-protocol-emission-refusals-and-read-back.md:66-82 | wrong-requirement |
| FND-009 | low | FR-529-AC-5, FR-530-AC-3 and FR-530-AC-5 each bundle several independent checks (compile, read-back, admission, forged identity, referenced-result, finite trace; or three cases), so one failing sub-check hides behind the AC row; FR-530-AC-5 says "tests reject", naming the test not the system. FR-529 cites FR-350 and FR-380 and FR-530 cites FR-177 in prose but not in `relationships`. | spec/functional/FR-529-settle-a-temporal-guarantee-of-implementation-rust-natively.md:98-104 | wrong-requirement |
| FND-010 | low | TC-1330 asks the test to "record in the test output which behaviour is existing coverage ... and which is newly delivered", which is a build-log instruction with no expected result that can fail. Remove it or turn it into an assertion on a named case. | spec/test-cases/TC-1330-native-composition-carries-and-checks-premises.md:20-24 | wrong-requirement |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-011 | low | The monitor section still says pending and indeterminate items "carry no record" while the items bullet now says they carry the clause-run record with `clause`; say "no FR-331 terminal record" there, and move the "for example `undefined-evaluation{where, cause}` and `replay-parity`" clause back beside the terminal-record sentence in the items bullet. Wording only, non-blocking. | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:46-55,125-128 | wrong-requirement |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Verified in fix commit f8528ef6e against the current text. |
| FND-002 | fixed | Verified in fix commit f8528ef6e against the current text. |
| FND-003 | fixed | Verified in fix commit f8528ef6e against the current text. |
| FND-004 | fixed | Verified in fix commit f8528ef6e against the current text. |
| FND-005 | fixed | Verified in fix commit f8528ef6e against the current text. |
| FND-006 | fixed | Verified in fix commit f8528ef6e against the current text. |
| FND-007 | fixed | Verified in fix commit f8528ef6e against the current text. |
| FND-008 | fixed | Verified in fix commit f8528ef6e against the current text. |
| FND-009 | fixed | Verified in fix commit f8528ef6e against the current text. |
| FND-010 | fixed | Verified in fix commit f8528ef6e against the current text. |

## Verdict

Not mergeable as is: FND-001 is a real contradiction (high) and four medium findings need a fix round. Merge order: QSpec #616 before this PR. Everything else listed in the Summary verified.

Round 1: FND-001 to FND-010 are fixed; FND-011 is a new low, non-blocking wording finding. Verified: executed set re-derived from QSpec main 78dcf2a90 (manifest 25 admit, 56 refuse, 4 property plus 34 layout refuse rows = 25, 90, 4; 26 pending ids equal layout ids; CH14a replaced by CH14a-1..8); `quire validate` exit 0 on all changed files and on spec/** plus plan/**. Mergeable once QSpec #616 has merged (it is still open).
