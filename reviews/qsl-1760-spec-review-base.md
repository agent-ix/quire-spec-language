---
id: SR-20261105101050
title: Base specification review of required identity handoff v2
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language; QSL-1760; PR #2428; FR-517, FR-531, TC-1331
  and changed carrier/emitter/consumer/test code'
review_set: subset
---

## Summary

Concrete defects require fixes before merge.

## Verdict

**CONDITIONAL**. 

## Scope

- FR-517-AC-23 (examined), spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md: The writer emits the required `identities/2` source-record and selected-population lists from the same final emission as the package: original opaque record keys, final structural/type references, original and final member names and order round-trip through native record joins, and every selected root's original populations and normalized effective member types are retained, including unused populations. The reader requires both lists and refuses the old format or an omitted list; no optional compatibility path or source fallback supplies them.
- FR-531-AC-19 (examined), spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md: An authentic final emission with a source record round-trips its original opaque record key, source owner, final structural/type IDs, ordered original member identities, final admitted names, presence and exact value types, including nested opaque identities. The bytes-only read joins it through native IR and `source_record_view` equals the original row; no healthy source record succeeds by structural-key substitution.
- FR-531-AC-20 (examined), spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md: A file with `/1`, absent `source_records` or `selected_populations`, an unknown/repeated member, malformed opaque key/type, foreign structural/type node, stale admitted name or conflicting record row refuses with the named typed format/shape/native-join condition and no partial identity read. With a supplied empty record-row list, an actual request for that record preserves native `UnavailableIdentityWitness`, never succeeding by structural-key substitution. Whole-witness completeness is not asserted by this carrier read.
- FR-531-AC-21 (examined), spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md: A handoff containing multiple selected roots, an unused population and normalized effective member types round-trips every original population/member declaration and its genuine normalized effective TYPE identity with no owner, in deterministic producer order. The population catalog is acquired without a snapshot and includes declarations no source expression reaches.
- FR-531-AC-13 (examined), spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md: The identity file of **R** encodes every identity as the "Value encodings" state: a node reference is the wire node id string of the package, an `EffectiveId` has the `chain` list root first, a `MemberId` has `owner` and `member`, and a declared `UnitId` has `kind` `declared` and a `node` reference; `read` returns rows equal to the writer's, a unit with `kind` other than `declared` returns `Malformed`, a declared unit whose node reference the package does not hold returns `Unjoined`, and a `declared_unit` row whose `exact_identity` has another `kind` returns `Unjoined`. A forged row that passe
- FR-531 (examined), spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md: When identity-carrier acquisition is requested, its writer SHALL compare the
  exact final model-root bindings with the retained producer catalogs, not only
  their counts.
- TC-1331 (examined), spec/test-cases/TC-1331-evaluate-from-a-checked-package-read-from-its-bytes-alone.md: | 18 | 19 | a genuine final source-record emission, including renamed members and nested opaque types | required `/2` rows round-trip unchanged; native IR record join and `source_record_view` equal the producer row |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | A blank line separates new procedure rows 18-20 from the existing Markdown table, but the new block has no header/delimiter row. GFM renders those new procedures as prose rather than table rows. Remove the intervening blank line or provide a complete second table so the new input/expected-result cells remain readable. | spec/test-cases/TC-1331-evaluate-from-a-checked-package-read-from-its-bytes-alone.md:38-41 |

## Evidence and limits

Frozen PR #2428 reviewed without code changes, Cargo commands, tests, builds, gates or locks. Author evidence in /tmp/qsl1760-logs/focused-final.log records Clippy, five library controls, 22 bare-package tests and the monitor-root join passing. Only full-old-reader-red-control.log / old-unavailable-fallback-red-control.log establish the missing-witness regression; the earlier field-arm probes passed and are not red evidence. The field helper edits were removed. No applicable active AssuranceProfile is declared. The installed matrix CLI has repository scope only and no criterion filter; the requested scoped computed matrix is unavailable, so trace tags and assertions were checked manually rather than running a forbidden global census. Plan completion: not assessed. Driver #123 adoption/IR FR-357-AC-10 remains explicitly pending; no old-dependency result is credited. The lead owns the one final scoped gate. Fresh-main merges #2419/#2426/#2427 require merge-owner reconciliation after review; no rebase was performed.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | The new FR-531 acceptance rows 19-21 are separated from the existing Markdown table by a blank line and have no header/delimiter row of their own. They render as prose rather than table cells. Remove the intervening blank line or give the block its own complete table header, as was done for TC-1331. | spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md:723-726 |

## Dispositions

### Round 1

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | TC-1331 rows 18-20 now directly continue the existing table. |
| FND-002 | still-open | New disposition finding: FR-531 itself retains the same unheaded second table block at rows 19-21. |

### Round 2

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | fixed | FR-531 AC19-21 now directly continue the existing acceptance table; the only substantive round-2 change is deletion of that separator blank line. |

## Disposition evidence (round 1)

Reviewed fixed head 91c68d1df71f04254349acdf0d7d095663368425 and main merge e1e2909b5 without source edits or Cargo runs. Author confined logs record 35 tests passing, including all seven equality codec controls, genuine empty population round trip and isolated deep-key reader/writer refusals. Exact 32-key success and 33-key refusal are checked through the original NodeKey and Composite ValueType entry points. Scoped Quire validation is grammar-clean. Relative to merged main, QSL-1718 bare_slots changes remain only the two constructor argument migrations; its assertions and behavior are preserved. Driver adoption remains pending and the lead final gate has not run.

Round 1 current verdict: CONDITIONAL; new base FND-002 remains open.

## Disposition evidence (round 2)

Verified documentation-only fix 76284f611b5ea019c66a732f2b4bfd466e91067f against the exact table boundary. No source, Cargo, build, test or gate changes were made by the reviewer. Current verdict: **PASS** for base review. All findings across this reviewer's methods now have a latest fixed disposition. The lead final scoped gate and normal downstream driver adoption remain separate.
