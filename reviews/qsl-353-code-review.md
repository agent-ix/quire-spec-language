---
id: SR-938
title: "QSL-353 code review (with rust-review lane) of PR 553, emission-to-admission corpus"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6d5e539395f42434e641fb9854eee4b94140931a; qsl-package/src/emit/tests/admission_corpus.rs, qsl-package/src/emit/tests.rs, qsl-package/src/emit/tests/golden.rs, qsl-package/src/emit/extent_agreement.rs, examples/config-version/spine.rs, examples/config-version/spine_unit.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
---
## Summary

Ticket: QSL-353. PR: quire-spec-language#553 at 6d5e5393, diff `d81193f9...6d5e5393`.
Gate: the coder's make ci log on 6d5e5393 ends `exit=0` and shows
`every_emitted_node_family_is_admitted_at_its_package_id ... ok`. I did not re-run it.

Oracle strength, checked against the code:
- Admission at the package_id is real. `read` pins the request at the emitted
  `package_id`. `read_checked_package_v2` recomputes the id from IR's preimage,
  cross-checks it against IR's digest, and `verify_binding` refuses a pin mismatch.
  `check_fixture` then compares the verified id with the emitted one.
- An `Admitted` row asserts the family is in `admitted.node_kinds()`, which is IR's
  own decoded kind list for the admitted graph, not QSL's view.
- The STD-129 row cannot absorb an unrelated refusal. It needs an envelope refusal
  with one of two causes, a locus that is an `expression`/`binary` node, and that node
  must be an application of `quire.op.structural.eq`. Only the `CyclicEquality`
  fixture skips the admission check.
- Omissions are closed. `UnsupportedForm`, `DeclarationOccurrenceMismatch` and
  `UnlockedOwner` always fail. `NamesAbsentNode` passes only when the absent node is
  one a `compound_unit` gap node names.

Refactors: `package_declaring`, `spine_compile_package`, `package_from_text` and the
`pub(super)` on `golden::Case`/`cases`/`records` are pure extractions. Every caller
does the same thing as before. No existing test changes behaviour.

`#[path]`: acceptable here. `tests/support/mod.rs:37` already includes
`examples/config-version/fixtures.rs` by `#[path]`, and `qsl-package` is
`publish = false`. It is a reference to the one datum, not a copy. The licence header
matches `spine.rs`.

Trace tag `#[trace("FR-093-AC-19", "TC-416")]` uses bare QSL ids, as the repo rule
requires. There are no QSpec ids in the diff.

Main moved (#550, #551). `git merge-tree` shows only `spec/tests.md` changed on both
sides, on different rows (TC-179 and TC-416), and it merges cleanly. FR-093 and TC-416
are untouched on main.

Rust lane: no production code changes. The test-only panics (`families` on an
undecodable kind, `document_digest`'s let-else) are fine in tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The "missing row fails" guard compares the table only with the families the listed fixtures emit (`emitted` is built from `outcomes`). So a family the lowering starts writing with no fixture passes silently. QSL-353 asked for the test to fail whenever a new family is emitted without a row. IR exposes a closed, exhaustive enumeration, `CheckedNodeKind::all()` (quire-contract-model `checked_package/v2/vocabulary.rs:554`). Iterate it, and require each kind to be either a row or an explicitly listed kind QSL does not emit. Then every unclassified family fails, and so does a new IR family. The fixture set is also derived from the rows (line 616), so a fixture that loses its last row is no longer emitted at all. | qsl-package/src/emit/tests/admission_corpus.rs:614-646 |
| FND-002 | low | The STD-129 row also accepts `OperatorIneligible` "after IR #238". The locked IR is ea63488 (IR #237), so that cause is for an upstream change that has not landed. Accepting it widens the oracle for a future state. Assert the cause the locked IR returns, and change it when the lock moves. | qsl-package/src/emit/tests/admission_corpus.rs:166, 536-539 |
| FND-003 | low | The `UndeclaredUnit` row accepts `NamesAbsentNode(_)` for any absent node. It does not check that the absent node is a unit the `compound_unit` body names (its `unit` bindings). So an unrelated absent-node defect on a compound unit would still read as the known gap. | qsl-package/src/emit/tests/admission_corpus.rs:519-523 |

## Verdict

The oracle is strong for what it covers. Admission at the package_id is proved through
IR, the gap rows are tight, and the refactors are behaviour-neutral. One medium
finding: the completeness guard is not closed over a real enumeration, so it does not
meet the ticket's "fails when a new family is emitted without a row". Two low
oracle-tightening findings. Not mergeable until the gap-analysis high finding (SR-939)
and FND-001 are fixed.
