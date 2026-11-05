---
id: SR-20261105001150
title: "Review of PR #2343: QSL-1641 B2a slot naming, equality file and equality witness (code-review with rust-review lane, gap-analysis lane)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@2067681e3 (PR #2343, branch qsl-1641-b2a-slots, base main, B1 merged as e5ac77201); qsl-package/src/{bare_slot,equality_file,reader_protocol_projection,bare,lib}.rs, qsl-package/src/emit/equality_reader.rs, src/{checked_handoff,emit_checked_v2}.rs, tests/it/bare_equality.rs, qsl-eval/tests/it/protocol_slots/acceptance.rs; FR-533-AC-11..17, FR-531-AC-14..17, FR-517-AC-21..22, FR-390-AC-65..67; ticket QSL-1641"
review_set: subset
---

## Summary

Ticket: QSL-1641. PR: quire-spec-language#2343, head 2067681e3 (an earlier review pass saw db894cc82; the delta is one assertion in acceptance.rs). The merge of main lost nothing: `git diff e5ac77201 b2b949ce6` (B1 squash vs B1 head) is empty and the per-file diff of the PR against the squash equals the earlier diff against the B1 branch plus the six-line FND-6(a) assertion.

Gates on the head (already containing main): fmt; check --locked --workspace --all-targets --all-features; clippy -D warnings default, --no-default-features and --all-features; arch-lint qualified-core; qsl-package lib; qsl-eval protocol_slots (72); root bare_package/checked_handoff/bare_equality with --features checked-handoff (670): all green.

Mutations (all caught): candidate order by name bytes (qsl-package lib test and bare_equality renamed-origins test fail); tied-slot ranking reversed (bare_equality and protocol_slots fail); walk match dropped (all three fail); check_equality removed (bare_equality refusals test fails).

Examined, clean: FR-531-AC-17 vector test reads the vector out of the merged FR-531 text and compares bytes (passes); the hand-written encoders round-trip every value-type, fact and owner tag and reject `+1`, `01`, `-0`, denominator `0` and `-2` and a compound unit in a source unit row; the writer joins every node member to the wire id and a node the package does not name is `Malformed` and so `Error::Unnameable`; the bytes route never reads the source graph (ProjectionSource::Bytes refuses record presence with MissingTypeRepresentation); the equality rows come from `Emission::equality_witness` (in-process unit's rows); no unwrap/expect/panic/#[allow]/unsafe on a production path; public surface growth (ReadEqualityRows, two refusal enums, EQUALITY_FORMAT, BareSupply.equality, Emission::equality_witness) is what FR-531/FR-533 name.

Split: not required. equality_file.rs is about 1300 production lines of one cohesive encoder/decoder and about 370 of tests; the writer, reader, BareSupply.equality and slot walk need each other to be tested, so a split would be artificial. Accepted as an exception.

Spec judgements. FR-533-AC-13: the AC cannot be built as written. A `choice` over a boolean must prove exhaustiveness (sum_case.rs MissingArm/DuplicateArm), so two arms guarded by the same `got.ready` are refused at check time, and any exhaustive pair of guards differs in an application (`not`), so no compiled fixture has two case guards with one walk. The spec must either (a) vector AC-13 on a package copy, as AC-14 does for `related` (edit the rows so two case-guard slots share one application and assert the exchange), or (b) state that no compilable source produces the exchange and move it to Inspection with the QSpec follow-up. Value-type vocabulary: FR-531 lists 17 tags and the writer refuses CarriedQuantity, Map and BoundedQuantity as Malformed (Error::Unnameable), a typed refusal and not a silent loss; the spec should say whether equality facts can carry those types (if so add tags, if not state that the writer refuses them). FR-390-AC-67 Unknown ordering: Foreign and Unknown cannot both be present as separate decisions in one rows object (foreign rows make every application unjoined), so the stated order Missing, Unknown, Foreign is not distinguishable; the code reports Foreign for rows of another package, which FR-531-AC-16 requires.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-390-AC-65, AC-66 and AC-67 have no test and no trace tag. The Package-before-Node ordering of AC-67 (`check_equality` runs on the closure before node projection) is implemented but no test fails if it moves after node refusals or if Missing/Unknown/Foreign were reordered. | qsl-package/src/bare_slot.rs:630-660 |
| FND-002 | medium | The traces FR-531-AC-16 and FR-533-AC-15 on `the_equality_witness_refusals_are_missing_foreign_and_unknown` overclaim: the test asserts only the `EqualityWitnessRefusal` enum from selection; the CallFailure::Admission, unknown_required_feature, exit 21, Package locus and meter halves need B2b. Likewise FR-531-AC-14 and FR-533-AC-16 claim evaluation parity that the test does not run (it selects and re-encodes). | tests/it/bare_equality.rs:154-179, 107-150 |
| FND-003 | low | FR-533-AC-14 and the `related` and `repeat` guard ranks are covered only partly: `tied_order`'s related interleaving and repeat-guard rank have no end-to-end test (the `99 100`/`100 105` vector is exercised only on `ordered_candidates`), and the missing-candidate test asserts the reason but not the Node locus key of the `got.ready` application, no partial view or no charge. | qsl-package/src/bare_slot.rs:340-380, tests/it/bare_equality.rs:374-400 |
| FND-004 | low | `check_equality` walks only the dependency closure, which stops at functions, while the span walk projects into called functions: an equality application inside a called function is checked at selection only for span-named slots and a Node refusal found earlier in such a walk can precede a Package equality refusal (FR-390-AC-67). Exact-site slots surface it only at evaluation (B2b). Also each application calls IR `admitted_equality` over all rows (quadratic) and `equality_file` re-emits the package a second time in the writer. | qsl-package/src/bare_slot.rs:630-660, src/checked_handoff.rs:339-355 |
| FND-005 | low | The writer's `rational()` encodes numerator and denominator without checking canonical form, so the canonical-integer rule is enforced on read only; a non-canonical row built in process writes a file the reader refuses. | qsl-package/src/equality_file.rs:393-398 |
| FND-006 | low | FR-517-AC-21 (non-empty source_units for fixture E) is not asserted and FR-517-AC-22 is tested at encoder level only; both disclosed by the coder, recorded here so the matrix does not read them as met. | tests/it/bare_equality.rs:111-134, qsl-package/src/equality_file.rs:1646 |

## Verdict

Mergeable as a B2a increment, with FND-001 and FND-002 fixed in this PR or carried by B2b with tickets (the tests are cheap: a package copy with the equality rows removed plus a Node-locus refusal gives AC-67; the tag overclaims are a one-line edit). No high finding. FR-533-AC-13 and the value-type vocabulary are spec changes for the spec lane, not code defects. FR-533-AC-15/16 evaluation halves and the Package locus remain B2b as disclosed.
