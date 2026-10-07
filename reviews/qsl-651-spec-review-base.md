---
id: SR-1373
title: "Spec review of quire-spec-language PR #659 (QSL-651): delete authored status from the test matrices and spec.md"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@d19d2a47bd3a33d73248fe249e959ccdd21bf3b3; PR #659 diff against merge base dac0d90f; spec/tests.md, spec/{model-linking,native-lowering,native-packages,native-readiness,native-runtime,native-temporal,native-workflow}/tests.md, spec/spec.md, spec/test-cases/TC-470-runtime-invariant-exits-as-tool-failure.md"
review_set: subset
---
# Spec review of quire-spec-language PR #659

## Summary

Ticket: QSL-651. This PR applies the #195 ruling to QSL's matrices:

- Coverage-table Status cells are emptied. Test Case Summary Status cells become bare markers.
- The narrative "coverage" sections of spec/tests.md and the per-area status prose are deleted.
- 35 TC rows that sat in a header-less table after the "Sum types" narrative move into the Test Case Summary.
- spec/spec.md's Requirements table loses its Status column.
- TC-470 gains an Expected Results section.

Examined:
- Every deleted prose paragraph in the 8 tests.md files (examined)
- The 35 moved rows (TC-742..744, TC-756..786, TC-891), old against new (examined)
- spec/spec.md Requirements and integration tables (examined)
- TC-470 Expected Results against its three tagged tests (examined)
- tools/check-index-completeness.sh lines 38-44 (examined)
- Owning TC and FR files for the deleted test-design sentences: TC-183, TC-184, TC-213, TC-238, TC-239, TC-421, TC-469, TC-578, TC-723..725, TC-817, TC-842, TC-845, TC-861..863, TC-868, TC-886, FR-051..053, FR-089, FR-356, ADR-009 (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

The deletion is right, and nothing besides status text was lost.

**Index rows.** Every TC, FR, NFR and IT row was compared old against new, with the Status cell removed:

- spec/tests.md has 558 rows on both sides, and they are identical as a multiset.
- The six area matrices other than model-linking are identical as well.
- model-linking keeps all its TC/FR rows. It drops only the hand-written "Criterion | Module carrying the tag" table (FR-042-AC-1..15 mapped to test files). That table was file tracking: it duplicated the `#[trace]` tags that `quire matrix` already computes.
- The 35 moved rows (TC-742..744, TC-756..786 and TC-891) match their old cells exactly. None was invented or altered. They now sit under `## Test Case Summary`, so quire mints them.
- Six TC ids disappear from spec/tests.md prose: TC-265, TC-268, TC-292, TC-297, TC-898, TC-899. All are QSpec, quire-exact or quire-walk ids. FR-089 and FR-356 still name those repositories.

**spec.md.** All 409 `| [ID](path) | Type |` rows are unchanged. The Status column held lifecycle and progress notes. I spot-checked the substantive ones: retirements (FR-051, FR-052, FR-053), ADR-009's open question and FR-068's partial supersession. Each is recorded in its own FR or ADR file. The quire master-requirements skeleton requires a Requirements section but does not type its table. check-index-completeness.sh:40 reads only the `[FR-NNN]` link, and :44 reads only the leading `| TC-NNN` cell, so both still work.

**Deleted narrative.** The deleted text is status ("pass locally", "stays Planned", "qualified by SR-…"), file tracking (test-symbol lists, module maps) or claim disclaimers. The test-design sentences in it are repeated in their TC files:

- the 512 KiB stack cases (TC-723..725)
- the literal expected classes in the refinement-gate tests (TC-861..863)
- the CG `negotiate_*` descriptors (TC-842, TC-845)
- the QSpec TC-262..265 runs (TC-817)
- `make conformance` (TC-421)
- the parity-step retirement (TC-469)
- the ample-set vector (TC-578)
- the differential run (TC-886)
- the TC-238/TC-239/TC-184 caveats

The TC-183 gap (4 of the O-25 members tested) is visible in its own Test Procedure. The TC-213 code gap (normalization omits operation members) is tracked as QSL-350.

**Status words.** None remain in table cells besides bare markers. The "(retired)" labels on AC ids name the AC's own lifecycle, which the FR files record too.

**TC-470.** Each of the three Expected Results matches its test:

- `only_runtime_invariant_exits_as_tool_failure` asserts 30, otherwise 20..=22.
- `private_corruption_cannot_turn_failed_checked_arithmetic_into_a_boolean` asserts Refused, Phase::Evaluate, RuntimeInvariant, exit 30, not incomplete.
- `a_report_combines_exit_codes_by_fr_301_severity` asserts the six pairs in both orders and an empty set to None.

**Measured.**

- `quire validate --scope . 'spec/**/*.md'` (quire 0.36.1, engine 0.50.1) goes from 2 failed documents to 0. The two were TC-470's missing Expected Results and TC-202's non-marker Status cell.
- `quire matrix` differs in one line: FR-072-AC-4's location moves from qsl-replay/src/result.rs:1179 to :1178, because a doc-comment line was removed.
- The `make ci` log (~/dev/worktrees/logs/qsl-651-ci-2.log) was written after head commit d19d2a47. It ends rc=0, check-index-completeness passes, and no test failed.

Clean. **Mergeable.**
