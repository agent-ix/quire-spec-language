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

- FR-517-AC-23 (examined), spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md: Test |
- FR-531-AC-19 (examined), spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md: Test |
- FR-531-AC-20 (examined), spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md: Test |
- FR-531-AC-21 (examined), spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md: Test |
- FR-531-AC-13 (examined), spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md: Test (TC-1331) |
- FR-531 (examined), spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md: When identity-carrier acquisition is requested, its writer SHALL compare the exact final model-root bindings with the retained producer catalogs, not only their counts.
- TC-1331 (examined), spec/test-cases/TC-1331-evaluate-from-a-checked-package-read-from-its-bytes-alone.md: | 18 | 19 | a genuine final source-record emission, including renamed members and nested opaque types | required `/2` rows round-trip unchanged; native IR record join and `source_record_view` equal the producer row |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | A blank line separates new procedure rows 18-20 from the existing Markdown table, but the new block has no header/delimiter row. GFM renders those new procedures as prose rather than table rows. Remove the intervening blank line or provide a complete second table so the new input/expected-result cells remain readable. | spec/test-cases/TC-1331-evaluate-from-a-checked-package-read-from-its-bytes-alone.md:38-41 |

## Evidence and limits

Frozen PR #2428 reviewed without code changes, Cargo commands, tests, builds, gates or locks. Author evidence in /tmp/qsl1760-logs/focused-final.log records Clippy, five library controls, 22 bare-package tests and the monitor-root join passing. Only full-old-reader-red-control.log / old-unavailable-fallback-red-control.log establish the missing-witness regression; the earlier field-arm probes passed and are not red evidence. The field helper edits were removed. No applicable active AssuranceProfile is declared. The installed matrix CLI has repository scope only and no criterion filter; the requested scoped computed matrix is unavailable, so trace tags and assertions were checked manually rather than running a forbidden global census. Plan completion: not assessed. Driver #123 adoption/IR FR-357-AC-10 remains explicitly pending; no old-dependency result is credited. The lead owns the one final scoped gate. Fresh-main merges #2419/#2426/#2427 require merge-owner reconciliation after review; no rebase was performed.
