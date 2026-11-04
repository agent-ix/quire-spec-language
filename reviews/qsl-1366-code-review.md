---
id: SR-2275001
title: "code-review (rust-review lane) of the QSpec protocol mutation manifest replay (quire-spec-language PR 2275)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; qsl-package/src/checked_v2/tests/protocol_mutation_manifest.rs; Cargo.lock; qsl-package/src/emit/tests/protocol_compensation.rs; qsl-package/src/emit/tests/protocol_controls.rs; src/checked_handoff/corpus.rs; tests/it/checked_handoff.rs; spec/functional/FR-518-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: references
---

# Code review (rust-review lane) of QSL-1366 manifest replay (quire-spec-language PR 2275)

## Summary

Ticket: QSL-1366. Head a552a19fc307d23cb132e264331e9c7e9aff5a8d, trial-merged with origin/main 32be5fad (head already contains it).
QSpec checkout at 860e08902 (>= b840f0985). Counts verified from the fixtures: 85 manifest rows (25 admit, 56 refuse, 4 property) plus 34 layout rows (all refuse) = 119 rows, 25/90/4, ids unique; 298 later; 27 pending (26 same-id layout rows, CH14a by CH14a-1..8, none without a row).
The runner asserts exact code, cause, locus and document_pointer per row and the row stage, validates the published schema first and reports the first error path as malformed_wire without calling read_v2. Eight scratch mutations of a copy of the fixtures were run: seven fail the test with the row id and the differing value (wire locus, reader cause, layout locus, schema-to-reader stage flip, property binder level, document_pointer, refuse-as-admit); the eighth (changing an admit row's added recursion_group name) legitimately still admits. Gates all green: fmt, check, clippy default and no-default-features, arch-lint qualified-core and api-surface, qsl-package 641+, qsl-semantics, qsl-eval, root `it` plain (882) and all-features (931). qsl-replay failing names are identical to a clean origin/main control run (20: 15 lib, 4 dispatch_link_compile, 1 doctest, including spine::lifecycle aggregate_package_accepts_temporal_selection_from_later_source).
Lock bump: Cargo.toml untouched (branch = main, no rev), Cargo.lock moves quire-contract-model to IR 35a96493 and quire-semantic-value to its main tip 30c24931 together. IR main has since moved to 6e73f067 (IR-1324/1325 only, not the IR-1322 reader).
The three restatements are consequences of the IR-1322 reader, not weakenings: each keeps or tightens its assertion (code, cause and pointer instead of code and pointer).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-518 row 7 and the definition-index corpus case state the locus rule as "first node that is a compensation template or carries a temporal_profile law", but the reader's trigger (and QSpec FR-464-AC-11) is the first node whose body is an await or compensate application. The two coincide on the one corpus package only; a package whose first compensation-form or law-carrying node is not an await/compensate application would get a different expected locus than IR gives. | spec/functional/FR-518-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md:row 7; src/checked_handoff/corpus.rs:238-258 | wrong-requirement |
| FND-002 | low | The pending-entry mapping (26 same-id rows, CH14a via CH14a-1..8) is computed and printed as "N of 27 pending" but never asserted, so a pending entry that loses its layout row passes silently; the exact 25/90/4 counts the AC states are not asserted either (only nonzero per class and total == rows). | qsl-package/src/checked_v2/tests/protocol_mutation_manifest.rs:509-540 | correct-requirement-no-evidence |
| FND-003 | low | The schema-first step depends on the first error of jsonschema 0.17.1, while the manifest names the validator it fixed the wire loci with as the jsonschema crate 0.55.0; all 40 or so wire rows agree today, but first-error ordering is validator specific and the test does not state or guard the assumption (a dev-dependency upgrade could move a row). | qsl-package/src/checked_v2/tests/protocol_mutation_manifest.rs:407-414 | correct-requirement-no-evidence |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 at 25b6ca591: the corpus helper picks the first protocol node whose body operation member operator is await or compensate (the reader's trigger) and FR-518 row 7 cites QSpec FR-464-AC-11 with the same rule. Mutating the helper to also accept compensation-form nodes, or to a wrong operator, fails every_case_gets_the_refusal_of_its_row with CaseMismatch definition-index; the row 8i test and the independent handoff table (node 7) still pass. |
| FND-002 | fixed | Round 1: the test asserts (admit, refuse, property, total) == (25, 90, 4, 119) and pending_without_row == 0. Mutations: an extra pending entry fails with 1 != 0; a dropped layout row fails with (25, 89, 4, 118); an extra manifest row fails with (26, 90, 4, 120). |
| FND-003 | fixed | Round 1: all schema errors are collected and the expected locus must be the first; a mismatch says whether the expected locus is a later error (validator order differs) or not an error at all, and the test doc comment records the 0.55.0 vs 0.17.1 assumption. A wrong expected locus fails with the "not among this validator's schema errors" message. The dependence on 0.17.1 first-error order remains by nature and now fails loudly with a diagnosis; the "later error" message branch was not exercised by a mutation. |

## Verdict

Round 1: PASS, mergeable. FND-001 to FND-003 are fixed. Head 25b6ca5913e37ec771e099299f3e42134499d4e9 contains origin/main (0c7da504d merge of 137f723a6), Cargo.lock equals main's with the earlier IR 35a96493 and semantic-value 30c24931 pair (no extra bump). Gates green: fmt, check, clippy default and no-default-features, arch-lint qualified-core and api-surface, qsl-package, qsl-semantics, qsl-eval, root it plain (882) and all-features (931); qsl-replay failing names (20) identical to the clean-main list. reviews/qsl-1366-*.md in the PR are the review-pass copies; this file supersedes them.
