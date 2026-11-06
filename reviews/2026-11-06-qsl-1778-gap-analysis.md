---
id: SR-20261105101067
title: "Scoped gap analysis of QSL-1778 native checked temporal reports"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1778; PR #2448 diff beyond qsl-1777-fr350-runtime-b; FR-052-AC-12, AC-15..AC-18; src/temporal/owner.rs; tests/it/native_owner_report.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-052
    type: reviews
---

## Summary

Ticket: QSL-1778. Planless, diff-scoped gap analysis. Plan completion: not assessed. `quoin matrix --json` did not finish within 500 s on this repository, so the acceptance-criteria-to-test mapping below is a manual check of the `#[trace]` tags in tests/it/native_owner_report.rs against FR-052.

## Verdict

**GAPS.** Every FR-052 criterion the PR claims (AC-12, AC-15, AC-16, AC-17, AC-18) has at least one tagged test that exercises production code with real assertions; no stubs or coverage inflation were found. Two gaps: the "two builds" half of AC-17 is not exercised by any gate, and the refusal branches of `CheckedSubject::new` and `evaluate_v2`'s own binding checks have no test.

## Coverage

- FR-052-AC-12: correction_keeps_direct_predecessor_immutable_and_rejects_stale_or_foreign_identity, reader_rejects_a_correction_self_edge_and_original_with_invented_predecessor, another_clause_and_another_occurrence_cannot_borrow_predecessor_authority.
- FR-052-AC-15: native_report_owns_truth_and_preserves_duplicate_support_and_roundtrip, opposite_and_missing_valuations_are_computed_from_admitted_rows, another_clause_and_another_occurrence_cannot_borrow_predecessor_authority, native_report_two_build_bytes.
- FR-052-AC-16: native_report_owns_truth_and_preserves_duplicate_support_and_roundtrip (rows at positions 0,0,0,1), opposite_and_missing_valuations_are_computed_from_admitted_rows.
- FR-052-AC-17: native_report_owns_truth_and_preserves_duplicate_support_and_roundtrip (round trip and two calls); native_report_two_build_bytes (two builds only when env vars are set).
- FR-052-AC-18: every_truncated_prefix_and_extra_byte_is_refused, opaque_observation_and_revision_binding_are_independent, binding_outcome_member_order_and_support_mutations_are_typed_refusals.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-17 two-build determinism runs only when QSL_OWNER_REPORT_PREVIOUS/OUTPUT are set; nothing in the repo sets them, so every gate run is a single-build round trip | tests/it/native_owner_report.rs:794-805 |
| FND-002 | medium | No test drives the refusal branches of `CheckedSubject::new` (foreign clause/occurrence, request selecting another occurrence, row position outside the request, missing source-map entry) or `evaluate_v2`'s revision/anchor checks | src/temporal/owner.rs:51-83 |

## Evidence and limits

Manual AC-to-test check; Quoin matrix unavailable (timed out). `grep -rn QSL_OWNER_REPORT` finds only the test itself. Tests in the file use `CheckedSubject::new(...).unwrap()` throughout and no test expects `BindingMismatch(Revision)` or `SupportNotInTrace` from `evaluate_v2`.

## Dispositions

Round 1, current head `5ed40a131`. Original review rows unchanged.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The environment-dependent comparison was removed, but replacement runs with_inputs twice in the same test executable. That constructs independent package/evidence values; it does not compare independently compiled binaries as TC-144 row11 explicitly requires. Preserve or wire actual two-build byte comparison in the one final gate, without another worktree. |
| FND-002 | still-open | New test covers foreign occurrence, request occurrence mismatch and evaluate_v2 revision/anchor refusal. It still omits the separately listed foreign clause and admitted row position beyond the shorter request; source-map refusal feasibility also has no disposition. Complete reachable controls or justify unreachable branches explicitly. |

### Round 2

Reviewed head `1ad7a1fc0`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Actual independent Rust recompilation lane is explicit, executable and has a passing receipt. |
| FND-002 | fixed | Genuine foreign-clause and short-request refusal controls added; missing source-map arm is unreachable after admitted-package exact-key validation, traced below. |

## Disposition evidence

Static review in the sole existing ticket worktree, no build/clone/worktree created. Existing coder receipts considered but not represented as reviewer reruns. Final gate remains the lead's next stage after every open finding is resolved.

## Round 2 evidence and readiness

Disposition round 2, final reviewed head 1ad7a1fc0 (owned fix 402547d2a..1ad7a1fc0). Same existing QSL-1778 worktree only; no source edits, build, clone or extra worktree. Relevant merged-main delta leaves native owner production behavior and canonical identity validation unchanged; owned production delta is formatting only. Both new test attributes now pair TC-144 with their criterion ids. The explicitly ignored local lane cargo-compiles the integration executable twice with different Rust metadata in the sole existing target, asserts fresh=false for each compilation, executes the exact fixture in each binary, and compares nonempty byte outputs. Existing pre-rebase coder receipts show eleven normal tests passing and the two-build lane passing with 7209 bytes from each binary; these were inspected, not rerun. Foreign-clause control uses a genuinely admitted other package, and the shortened request rejects its still-admitted position-1 valuation row.

Source-map reachability independently traced: quire-contract-model/src/checked_package/common.rs validate_source_map_entries constructs expected (node, role, ordinal) from all graph occurrences and compares actual source-map keys exactly, returning InvalidSourceMap when actual != expected. checked_package/v2/mod.rs calls that validator before constructing the admitted CheckedPackageV2, whose fields are private; TemporalSubjectView::arena_of requires a declared occurrence. Thus a missing map entry for an accepted occurrence cannot reach CheckedSubject::new through the public admitted API. TC-144 now documents this defensive-arm proof rather than fabricating an invalid checked package.

All findings across the six reviewed artifacts have a latest resolved outcome. No new fix-diff findings. Review mergeable, subject to the lead's one final gate and normal integration checks; no claim that the unavailable repository-wide matrix or final gate passed.
