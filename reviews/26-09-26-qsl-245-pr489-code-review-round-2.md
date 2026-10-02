---
id: SR-735
title: "QSL-245 code review of the PR 489 fix round"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-cst/src/diagnostic.rs; qsl-foundation/src/diagnostic.rs; qsl-foundation/src/source.rs; src/command/output.rs; src/command/output/types.rs; src/main.rs; tests/it/cli.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: reviews
---
## Summary

Ticket: QSL-245. PR: quire-spec-language#489. This round is a code and Rust
review of the fix delta only; it records new findings
that the fix introduced. Dispositions of SR-732, SR-733 and SR-734 are in
those files.

Checked sound:

- **Public API.** Three items are added: `SourceReadCause::ReferenceInvariant`,
  `BLANK_LABEL_TAG`/`EMPTY_PATH_TAG` and `IdentityCauseFields`. Three items are
  removed: `identity_tag`, `blank_label` and `with_identity_cause`, all of which
  this PR had itself added. Every `match` on `SourceReadCause` is still
  exhaustive.
- **Layer direction.** The new items live in qsl-foundation and are read by
  qsl-cst and the root crate. No dependency points upward. qsl-foundation
  already derives `Serialize` on source types.
- **Run output.** `types::Diagnostic` keeps its field order, and `flatten` of
  `None` adds no key, so the run output's shape does not change for existing
  consumers. The cli and standalone tests pass: 22 passed.

Measured by building the CLI before and after the fix round and running both on
a blank label, invalid UTF-8 and a blank identity.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The CLI refusal line's key order changed for every refusal, not only identity ones. The old line was a `serde_json::Value`, so its keys were sorted: `cause, code, label, message, path, phase, source, span, status`. The new line follows the order of the `Line` fields: `status, phase, code, source, path, span, message, cause, label`. The nested `source` object is now in declaration order, with `revision_namespace` before `revision`. Example: `parse` of a file with invalid UTF-8 gives the same keys and values, but the bytes differ. The JSON means the same thing, and no spec or test pins the key order. Still, this is a byte-level change to the wire output for refusals outside this PR's scope, and the commit does not mention it. A consumer that compares bytes breaks. Either accept it and say so in the commit or FR-010, or keep the old order. | src/main.rs:30-66 |
| FND-002 | low | The `QSL-236` comment about the byte ceiling now sits above the new `ReferenceInvariant` arm instead of the `ByteBudget` arm it describes, in both crates. | qsl-cst/src/diagnostic.rs:304-307; qsl-foundation/src/diagnostic.rs:561-563 |

## Verdict

Approve. Both findings are low and neither blocks a merge. The
`ReferenceInvariant` mapping has no test, and a mutation back to the invented
label survives. The path is unreachable: provenance refuses only empty labels,
which are excluded first, and the digest domain is fixed. That is accepted,
not a finding.
