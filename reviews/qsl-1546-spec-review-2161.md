---
id: SR-2161001
title: "spec-review of FR-518 stage amendments for rows 8, 12 and 13 (quire-spec-language PR 2161)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-518-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md; spec/test-cases/TC-1313-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md; spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-518
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-517
    type: references
---

# Spec review of FR-518 stage amendments for rows 8, 12 and 13 (quire-spec-language PR 2161)

## Summary

Ticket: QSL-1546. PR: quire-spec-language#2161, head 63b634123a511536952bc3c7af54c6dba898eeea, `git diff origin/main...HEAD`.

Checked against Contract IR main (b3b0bad0), `crates/quire-contract-model`, and the QSL-1546 code PR #2160 at head 8cdba1f10, where I ran `cargo test --features checked-handoff --test it checked_handoff` (23 passed, 3 ignored, among them `every_case_gets_the_refusal_of_its_row`, which submits every corpus case to the real reader and `Selection::check`).

Reproduced and true:
- Row 13: the reader refuses with `unknown_profile`/`unsupported-selection` at `/semantic_graph/nodes/{N}/body/operation/laws/0/definition`. `TemporalProfile::from_wire` is a closed list of five identities, each ending `/v1`, so no admitted package carries another revision (`vocabulary.rs`, `temporal.rs`).
- Row 12: with the clauses, the lock `profile_selections` row and the `temporal_bindings` rows moved together to the fixed-sample profile the reader admits and `Selection::check` refuses with `definition.identity`. The two partial-move refusals named in the cell match the reader's join in `temporal_binding.rs` (lock-selection or clock-kind mismatch refuses at `clock/kind`); I read them and did not run them.
- Row 8: the reader refuses an added `declaration` member on a `temporal_clause` node at `/semantic_graph/nodes/{N}/declaration`. The code is `invalid_semantic_graph` with no cause (FND-003).
- Recount: reader rows 1-8 (8), 13, 14b, 15b, 16 (4), 22-27 (6), 28m, 30, 31 = 21; selection rows 9-12, 17-21, 29 = 10; mutation-naming rows 1-13, 14b, 15b, 16-27, 28m, 29-31 = 31. The 16 axis names equal the `SelectionAxis::name` list in #2160.
- FR-517-AC-10 now reads AC-1 to AC-14, matching the new last AC. TC-1313 scope, rows 2, 3, 11-14 and the trace tag agree with the FR. No new id except FR-518-AC-11..14; none exists on main or on the open PR heads. #2160 also edits FR-517 and FR-518 (rows 14b, 15b); the two branches merge cleanly. `quire validate` on FR-517, FR-518 and TC-1313: exit 0, only the module duplicate-archetype and inline-schema warnings.

Not true: "nothing dropped" for the clause name and the revision. The unit-level successors FR-518-AC-11 and AC-12 are written for an "in-memory package", and no such package can be built (FND-001, FND-002).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-518-AC-11 cannot be run as written, so the clause-name property is dropped while the text says no coverage is dropped. It asks `Selection::check` to refuse "a package value built in memory whose clause name differs from the sidecar's", not read through the reader. `Selection::check` takes `&CheckedPackageV2`; that type has only private fields and the reader is its only constructor (Contract IR `lib.rs` carries a `compile_fail` doctest that a package cannot be made from anything else). The reader refuses a `declaration` on a `temporal_clause` node, and the package holds no other clause name, so the sidecar name has nothing to compare with: #2160's `the_clause_name_has_no_package_side_value` shows an edited name passes. No test, reader-fed or in-memory, can make `clause.name` fire today. The text also gives the AC no way to say where the name comes from, so it is not circular, it is unconstructible. State the gap truthfully (the legacy `clause.name` property has no test until the IR ticket closes) and write the successor in the AC-9/AC-10 pattern: "with a reader that admits a declared name on a temporal clause node, a package whose declared name differs from the sidecar's is admitted by the reader and refused by `Selection::check` with `clause.name`", gated on the IR ticket, plus an AC for today's behaviour. Then reword the 'Moved cases' sentence and the 'IR gap' paragraph, which say the axis is "exercised only by the unit-level test". | spec/functional/FR-518-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md:166-170 | wrong-requirement |
| FND-002 | high | FR-518-AC-12 has the same unconstructible subject ("a package value built in memory ... not read through `read_checked_package`"), and its subject is also mis-modelled. `CheckedArtifactRef` is `{authority, identity}`; the revision is the identity's trailing `/vN`, which `Selection::check` splits off. A different revision in the package is an `unknown_profile` refusal by the reader, so no package value exists to check. The property is testable another way, and #2160 already does it (`each_sidecar_value_is_compared_on_its_own_axis`): an admitted package checked against a sidecar whose clause profile revision differs. Reword AC-12 and TC-1313 row 12 to that (a sidecar whose revision differs from the admitted package's law revision is refused with `definition.revision`), and say in the Description that the revision is the identity suffix. | spec/functional/FR-518-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md:171-172 | wrong-requirement |
| FND-003 | high | The row 8 expected cell reads `invalid_package`/`invalid_semantic_graph`, which is code `invalid_package` with cause `invalid_semantic_graph`. The reader's code for this refusal is `invalid_semantic_graph` with no cause (`CheckedPackageRefusalCode::InvalidSemanticGraph`; #2160's corpus expects `reader("invalid_semantic_graph", None, ".../declaration")` and passes against the real reader). FR-518-AC-2 requires the table's code, cause and pointer, so the table as written fails its own AC, and the table disagrees with FR-518-AC-14 and TC-1313 row 14, which name `invalid_semantic_graph` alone. Write `invalid_semantic_graph` with no cause at `/semantic_graph/nodes/{N}/declaration`. | spec/functional/FR-518-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md:84 | wrong-requirement |
| FND-004 | low | FR-518-AC-13 and AC-14 (and TC-1313 rows 13 and 14) restate FR-518-AC-2 for rows 13 and 8: AC-2 already requires every reader case, now including both, to be refused with its row's code, cause and pointer. Neither can fail unless AC-2 does. Fold them into AC-2, or keep them only as the named place the IR-gap rows are pinned, and say so. | spec/functional/FR-518-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md:173-174 | wrong-requirement |
| FND-005 | low | Two passages narrate how the table was found rather than state the system. 'Verification status' says rows 8, 12 and 13 "have been executed against the Contract IR reader", and the row 12 cell carries the history of two partial moves that refuse earlier. Keep the rule (the three profile pieces move together) and drop the narration; the verified/not-verified split of the rows is already the section's purpose. | spec/functional/FR-518-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md:64-69 | wrong-requirement |
| FND-006 | low | TC-1313 rows 2 and 3 copy the FR's case counts ("21 cases", "10 cases") beside the row lists. The count then lives in two files and FR-518-AC-8 forbids a count constant in the crate for the same reason; the row lists carry the information. | spec/test-cases/TC-1313-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md:21-22 | wrong-requirement |

## Verdict

Not mergeable. FND-001 to FND-003 are wrong or unusable as written: the two unit-level successor ACs cannot be run, so 'nothing dropped' is true for rows 8 and 13 only as far as the reader refusals go, and false for the clause-name and revision properties of the sidecar, and the row 8 code is wrong. The stage moves for rows 8, 12 and 13, the counts (21/10/31/16), AC-4, FR-517-AC-10 and the TC scope, rows and trace tag are correct.
