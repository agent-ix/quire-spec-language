---
id: SR-2280001
title: "spec-review of quire-spec-language PR 2280 (QSL-1608 cross-check on both routes, construction refusals)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; FR-515; FR-516; TC-1310; TC-1311"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-515
    type: references
---

# Spec review of quire-spec-language PR 2280

## Summary

Ticket: QSL-1608. Head 59ac58830423a20a098c7309c4c9436d8898eda8, checked against PR 2263 code at 5fa074aee (qsl-eval EvidencePackage, qsl-package AdmittedPackageRead). Verified: ProfileSelections and Bindings are from_emission only (the read route takes both row sets from the bytes and the source CheckedPackage has no clock rows); AdmittedPackageRead::source() exists and is the source the bytes were emitted from; AC-29 can fail (always versus eventually, same interval); the FR-516 cause table keeps No other cause exists (PackageMismatch is stated to be FR-515's, not a call cause); FR-515-AC-25 to AC-29 are unused on main (AC-24 is main's last) and on the open PR heads; TC-1310 scope, trace tags, row 29, TC-1311 row 46 and the Delivery line (AC-18 to AC-29) are updated; no ticket number in any AC; no contradiction with FR-515-AC-13 to AC-23 or FR-337. quire validate exit 0 on all four files and on spec/** plus plan/**.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | AC-25 says `from_emission` and `from_read` each return `PackageMismatch::ClauseRow`, and the Behavior bullet and AC-26 do not say which route returns `AmbiguousName`. In the code on PR #2263 (qsl-eval/src/temporal/evidence/package.rs, head 5fa074aee) `ClauseRow` is returned by `from_emission` only: it checks each Emission clause reference's position against the name rows, while `from_read` builds its clause references from the name rows themselves (`PackageRows::of_read`), so a reference whose clause carries another name than its row cannot arise on the read route and the from_read half of AC-25 cannot fail. `AmbiguousName` is returned by `from_read` only (name pairing happens only there); `from_emission` pairs by `clause` position, so duplicate names do not affect it. State the route per variant in the Behavior bullet and make AC-25 emission-only and AC-26 read-only, or specify what the other route checks. `ProfileSelections` and `Bindings` as `from_emission` only is right: the read route takes both row sets from the bytes (the source CheckedPackage carries no clock rows or profile selections, FR-050), so there is nothing on that route to compare. | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md | wrong-requirement |
| FND-002 | medium | The paragraph says `from_emission` compares with 'the Emission's in-memory clause' and `from_read` with the admitted read's source graph. An `Emission` holds no clause (it holds rows), and in the code both routes compare against the clause at the reference's `clause` position in `AdmittedPackageRead::source()`, which is the only in-memory clause either route has, so there is one comparison, not two. AdmittedPackageRead is built only by emitting the source package and reading those bytes, so AC-29's 'package bytes hold `always` while the source holds `eventually`' can arise only from an edited Emission passed to `read_evaluation_emission`; the AC does not say how the test makes the bytes differ, and AC-24 and AC-29 then differ only in route. Name the single in-memory source (the source graph's clause), say how the differing bytes are produced (an edited Emission admitted through `read_evaluation_emission`), and say that the other route is the same comparison. | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md | wrong-requirement |

## Verdict

Not mergeable until FND-001 and FND-002 are fixed; both are small edits.
