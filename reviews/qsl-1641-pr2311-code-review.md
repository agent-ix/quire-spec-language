---
id: SR-20261104190829
title: "Code review of PR #2311: QSL-1641 identity file (identities.json, ReadIdentityRows), code-review with rust-review lane"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@2db5b5c3a trial-merged with origin/main 3800efb02; git diff pr2309...pr2311 (PR #2311, branch qsl-1641-identity-file): qsl-package/src/bare.rs, qsl-package/src/lib.rs, qsl-package/Cargo.toml, qsl-eval/src/temporal/evidence/package.rs, src/checked_handoff.rs; ticket QSL-1641"
review_set: subset
---

## Summary

Ticket: QSL-1641. PR: quire-spec-language#2311, head 2db5b5c3a (base: the A branch, 003b53ccd merged in). Method: code-review with the rust-review lane. The diff adds `BareSupply`, `BarePackage::with_supply`, `ReadIdentityRows` (read, `of_rows`, `to_bytes`), the typed `IdentityMember` and `IdentityFileRefusal`, writes `identities.json` in the handoff directory, and lets `from_package` use the supply's rows to project a record-typed capture.

Examined, clean:

- `IdentityMember` is a closed typed enum; the former scraping of the member name from serde error text is gone. `deny_unknown_fields` and the hand-written top-level visitor give `Unknown` and `Repeated` with the member named.
- `read` joins each row's `structural_node` (and the `node` references inside owners and units) against the package's nodes, matches the declaration form to the identity kind, and returns no rows on a refusal; a `declared_unit` with another `kind` is `Malformed`.
- No `unwrap`, `expect`, `panic!`, `unsafe`, `#[allow]` or indexing in the added production code; the only manifest change is `serde` `derive` in `qsl-package` (no Cargo.lock change).
- The writer is deterministic: files are inserted in a fixed order, `to_bytes` ends in one newline.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `EvidencePackage::from_package` ignores a supply whose rows `SelectedExactIdentities::from_stage_rows` refuses (`if let Ok`), so a mismatched identity file shows up only as a later `capture-type` refusal with no cause. | qsl-eval/src/temporal/evidence/package.rs:248-260 |
| FND-002 | low | `BareSupply`, `with_supply` and `supply()` have only the temporal capture projection as a reader until the FR-533 slot call lands (the coordinator's review FND-002). Accepted for the stack; not a defect. | qsl-package/src/bare.rs:52-86 |

## Verdict

Mergeable once A (#2309) is merged; only low findings. Gates on the head trial-merged with origin/main 3800efb02 are in the gap-analysis review of this PR.
