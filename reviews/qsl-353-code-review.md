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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The `NOT_EMITTED_BY_QSL` reason for `value/collection_value` says "a collection literal lowers to an expression/collection node". That is only true of a collection constructor expression. A constant collection value (`Value::Collection`) is refused as `UnbuiltLiteral` (lowering.rs:3710-3718), the same reason `option_value` gives. The classification is right, but the reason is incomplete. Name both paths. | qsl-package/src/emit/tests/admission_corpus.rs:394-396 |

## Dispositions

Round 1, reviewed at c29e7b3397a0cb9bcec144a5ec4db210c170ff10 (fix commit c29e7b33 on 940aa601, rebased onto 8b0c1ffe). The coder's make ci log on c29e7b33 reports `exit=0`; I did not re-run it. The fix changes no existing test: `golden::float_add` now delegates to `float_add_of(Float64, mode)`, which builds the same package, and `modelled` is local to the corpus. `tests/fixtures/systems-interface.semantic-ir.json` is a fresh fixture: its `acme/systems` identity appears in no other checkout under ~/dev. The `reviews/` files the coder committed are byte-identical to the review-pass SR files.

I spot-checked the 48 `NOT_EMITTED_BY_QSL` reasons against `qsl-semantics/src/check`:
- `option_value`: confirmed. `Value::Option` is refused as `UnbuiltLiteral` (lowering.rs:3714).
- `alias`: confirmed. No lowering arm writes `alias`.
- `collection_value`: the classification holds, but the reason is incomplete (FND-004).
- `dimension`/`unit`: no lowering writes either form. The only `unit` string is a binding name inside `compound_unit` (lowering/model.rs:423).
- Every `NO_TAG` kind (temporal, protocol, claim, correspondence, state transition and snapshot): no production `insert` of those tags. `CheckedClauseKind::node_tag_and_semantic_form` in identity.rs only classifies; it builds no node.
- The model and relation `NO_RECORD_FORM` kinds: `record_form` (lowering/model.rs:233) maps only object types, systems interfaces and relationships.

A residual limit, not a finding: `NOT_EMITTED_BY_QSL` is a claim the test cannot prove. If QSL starts writing a kind listed there and no fixture emits it, the test still passes. QSL forms are `&'static str`, so there is no QSL-side closed enumeration to check against. Every IR kind now needs an explicit decision, and a fixture that emits a listed kind fails. That is as closed as the code allows.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | c29e7b33: the test walks `CheckedNodeKind::all()` and requires each kind to be exactly one of an admitted row, a gap row or a `NOT_EMITTED_BY_QSL` entry with a reason. Fixtures come from `Fixture::ALL`, a fixture backing no row fails, and an emitted family with no row fails even when it is listed as not emitted |
| FND-002 | fixed | c29e7b33: the STD-129 row requires `refusal.cause == Some(OperationLawMissing)` only |
| FND-003 | fixed | c29e7b33: `units_named_by` reads the compound unit's own `unit` bindings, and the row requires `NamesAbsentNode(absent)` with `absent` among them |
