---
id: SR-12341
title: "Gap analysis of QSL#1232 and SV#60 (IR-585): FR-401 S1 criteria, traces and the specs the move invalidates"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@509a7919a0dddd42ac070b76be7f08c726cea39f with agent-ix/quire-semantic-value@1cf30d9c33006cf8d9cf017973dbf10550839d43; FR-401 (S1 rows), FR-400 X-13 paragraph, ADR-011 §7.2 and X-15, FR-068-AC-4, FR-068-CON-3, FR-063-AC-7, FR-090-AC-6, TC-161, TC-170, TC-173"
review_set: subset
---

## Summary

Ticket: IR-585. Planless gap analysis over the S1 move. Matrix from
`quire matrix --scope . --format json` (quire 0.36.2, engine 0.50.2) on
the QSL PR head.

| Criterion | Matrix status | What backs it after the move |
| --- | --- | --- |
| FR-401 S1 item set (`WrongSnapshotCause` only, from `check::refusal` to SV `call`) | n/a (Behavior) | Diff moves exactly that one type; nothing else |
| FR-401 Outputs / ADR-011 §7.2, X-15: no re-export at the old path | n/a | Inspected: no `pub use` of it anywhere in QSL |
| FR-401: receiving crate declares `seam_probe`; QSL matches stay non-exhaustive | n/a | SV `Cargo.toml` check-cfg; `xtask seam-probe` exit 0 with `ProtocolClauseSnapshot::catalog_code` reported |
| FR-400 X-13 | n/a | X-13 has not moved it (no `quire-checked` yet), so S1 is the slice that moves it |
| FR-401-AC-3 | untagged | Nothing. See FND-001 |
| FR-090-AC-6 | tagged (`causes.rs` `protocol_clause_snapshot_catalog_code`) | Unchanged; still passes against the SV type |
| FR-068-AC-4, FR-068-CON-3 | tagged (definition_scan tests) | Tests now assert the opposite of the AC text for this type. See FND-002 |
| FR-063-AC-7 / TC-161 step 10 | tagged | `WrongSnapshotCause` dropped from the check; see SR-12340 FND-001 |

Reverse gap: none. Every changed line is an import repoint, the deletion,
a scan-list edit, or the SV addition; no new behaviour without an owner.
Stubs: none. The SV test asserts the two real tags.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-401-AC-3 is untagged and its S1 half has no test: nothing checks that `quire_semantic_value::call::WrongSnapshotCause` resolves and `qsl_semantics::check::WrongSnapshotCause` fails to compile from another crate. A test is feasible now, and was run on the review worktree: a `compile_fail,E0432` doctest (`use qsl_semantics::check::WrongSnapshotCause;`) plus a positive control (`use quire_semantic_value::call::WrongSnapshotCause;`) both pass under `cargo test -p qsl-semantics --doc`. It is the pattern `src/linking/composed/requests.rs:395` already uses. Put it on one item or the module doc, not on a `pub use` group, where rustdoc copies it to every re-exported name; doctests do not bind in the matrix, so add a `#[trace("FR-401-AC-3")]` test beside it (for example an xtask scan asserting no QSL `pub use` names it, next to `assert_not_defined_in_qsl`) | xtask/src/definition_scan.rs:450-459; qsl-semantics/src/check/mod.rs:326-334 |
| FND-002 | medium | The move falsifies FR-068-AC-4 and FR-068-CON-3 ("`check` defines ... `WrongSnapshotCause`") and the TC-170 and TC-173 lists that repeat it, but no spec is amended. The tests traced to them were edited to assert the opposite (`con3_methods_and_symbols_each_have_exactly_one_defining_location`, traced FR-068-CON-3, now asserts it is defined nowhere in QSL), so the matrix shows the criteria as tagged while the tagged test contradicts the criterion text. Amend FR-068-AC-4/CON-3 to name SV `call` for it, as they already do for `InputRefusal` and `CheckMode`, and TC-170 steps, TC-173 and TC-161 step 10 to match | spec/functional/FR-068-split-expression-checking-into-check-stage.md:481,492; spec/test-cases/TC-170-check-stage-modules-move-with-no-duplication.md:28,56; spec/test-cases/TC-173-refusal-split-check-causes-versus-input-refusal.md:18; spec/test-cases/TC-161-seam-probe-exhaustiveness.md:125,161; xtask/src/definition_scan.rs:413,456 |

## Verdict

CONDITIONAL. The code meets FR-401's S1 item set, the no-re-export rule and
the seam-probe obligation. Two medium traceability gaps: FR-401-AC-3 has no
test though one is feasible, and the stale FR-068/TC criteria now contradict
the tests traced to them. Both are fixable inside #1232.

## Coverage

- Criteria examined: FR-401-AC-3, FR-068-AC-4, FR-068-CON-3, FR-090-AC-6,
  FR-063-AC-7 (via TC-161 step 10); FR-401 Behavior S1 bullets, Outputs,
  and the moved-type probe paragraph; ADR-011 §7.2 and X-15; FR-400 X-13
  paragraph.
- Other FR-401 ACs (AC-1, AC-2, AC-4 to AC-7) are S0/quire-eval scope and
  untagged in QSL; not attributed to this PR.
- Semantic review: done by hand for the S1 rows above (the move is a
  type relocation); the full intent-test-code pass was not run.
- Plan completion: not assessed
