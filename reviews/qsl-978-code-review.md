---
id: SR-12348
title: "Code review (Rust lane) of QSL-978 published extension definitions in the checked-package v2 fixture reader (PR #1267)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6f03809f4b44576a162090839eb63af925fe3db3; git diff origin/main...HEAD over qsl-package/src/checked_v2/tests.rs and qsl-semantics/src/library/package_identity.rs"
review_set: subset
---
# SR-12348: Code review (Rust lane) of QSL-978 published extension definitions in the checked-package v2 fixture reader (PR #1267)

## Summary

Ticket: QSL-978. PR quire-spec-language#1267 at head 6f03809f4. Only the `origin/main...HEAD` diff was reviewed, against QSpec FR-469 ("Published definitions", AC-3, AC-9, AC-10) and TC-951 X18/X19 as they stand at QSpec PR #365 head 02cf63fd. This is a code review with the rust-review lane folded in.

Commands run through the build lane, with `QSPEC_DIR` at the QSpec #365 worktree: `cargo test -p qsl-package --lib checked_v2` passed (43 tests, including `extension_definition_vectors_xd_01_to_04`, c14, i2 and fr340). The same tests were run again with `QSPEC_DIR` at a directory holding every QSpec proposal file except `extension-definitions.json` (see the gap analysis SR-12349 for that result).

What holds:

- No vendoring. The definitions are read from `$QSPEC_DIR` at test time. Nothing is copied into QSL.
- The loader follows FR-469's conformance-reader rule. It holds a published definition only when a `lock.definition_selections` entry names it, holds nothing the file does not publish, and leaves a selection with no published definition unheld.
- A missing file is not hidden. When the file is absent, nothing is held, so XD-01 and XD-03 (`admitted`) and the c14/i2 positive reads fail loudly. They do not pass vacuously.
- A withheld reference that matches nothing would turn XD-02 or XD-04 into `admitted`, and the test then panics. So a typo in `withheld` cannot pass silently.
- The skip when `QSPEC_DIR` is unset follows the existing `qspec_v2_fixtures` convention.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `NODE_TAGS` copies IR's `CheckedNodeTag` wire vocabulary by hand, and nothing links the two. IR's `quire-contract-model` is pinned `branch = "main"`, so the next IR tag added fails again as `InvalidPreimage{NodeTag}` at fixture time, which is the same failure this PR fixes. The cure: a qsl-package test (qsl-package depends on both crates) asserting `NODE_TAGS` equals `CheckedNodeTag::ALL.iter().map(as_wire)`, with `NODE_TAGS` exposed under `test-support`. Alternatively, drop the duplicate membership check on the path IR has already validated. ADR-011 keeps `quire-contract-model` out of layer 3, so a direct dependency is not available. | qsl-semantics/src/library/package_identity.rs:289-304 |
| FND-002 | medium | The XD test does not assert the refusal's `locus` (the Audit node's `node_id`, digest `7aa76a0f...` in the vectors) or IR's `document_pointer` field. It compares only QSL's `Locus::Artifact` pointer, which is built from IR's `path`. The doc comment claims both are checked. A reader that put the wrong node in `locus`, or no `document_pointer`, would pass. FR-469-AC-3 and AC-10 (TC-951 X19 "the stated locus and document_pointer") are unverified. | qsl-package/src/checked_v2/tests.rs:2810-2832 |
| FND-003 | low | `let Ok(bytes) = std::fs::read(&path) else { return; }` swallows every I/O error, not only NotFound. A permission or EISDIR error reads as "no definitions published". Return only on `ErrorKind::NotFound` and panic otherwise, as the parse error already does. | qsl-package/src/checked_v2/tests.rs:1333-1335 |
| FND-004 | low | The test hard-codes the expected code instead of reading `expected.code`, ignores the vector file's own `definitions` member (the loader hard-codes the filename), and pins `assert_eq!(vectors.len(), 4)`, a count pin. `!vectors.is_empty()` keeps the non-vacuity guard without the pin. | qsl-package/src/checked_v2/tests.rs:2799-2817 |

## Verdict

Not mergeable as is. Two medium findings: FND-001, the unlinked parallel tag list, which is the repeat-failure cause, and FND-002, the unasserted locus and document_pointer. Both are small fixes inside this PR. FND-003 and FND-004 are low and are cheap to fold in during the same round.
