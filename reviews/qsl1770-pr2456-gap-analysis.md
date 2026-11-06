---
id: SR-20261105101067
title: "QSL-1770 PR2456 gap-analysis"
type: SpecReview
analysis: gap-analysis
review_set: subset
scope: "agent-ix/quire-spec-language; QSL-1770 PR2456 at 826c26f159e8e454385a14c4590e8b6faa99f8a5; qsl-foundation/src/diagnostic.rs; qsl-foundation/tests/kernel_refusal_record.rs"
relationships:
  - type: references
    target: ix://agent-ix/quire-spec-language/FR-096
---

## Summary

Ticket: QSL-1770. Urgent exhaustive-match compile fix only; full bounded conversion and strict URI diagnostic integration remain incomplete. Examined both changed files and upstream quire-exact origin/main API. No applicable AssuranceProfile found.

## Verdict

CONDITIONAL. No new scoped trace or stub defect found; whole-repository assurance is not established because the bounded matrix command timed out. API compatibility: QuantityResultInterval::new(UnitId, Rational, Rational), unit/min/max accessors and boxed refusal target agree with fresh upstream source. The real test checks exact output, reduced bounds, two package identities, declared/compound units, code/cause/category, exact field set, caller locus and absence of actual magnitude.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Coverage

Both changed files reviewed against repository instructions and Rust API, implementation, testing and wire rules. No CI, manifest or lock changes. No source stubs, mocks, suppressions or gate narrowing introduced. Existing unit spelling search covered qsl-* Rust sources and found qsl-replay/src/spine.rs:2523. cargo fmt --check and git diff --check passed. Compilation and tests intentionally not run against known old lock lacking the new variant; focused execution requires Draco's lock bump.

Matrix command: timeout 20s quire matrix --scope . --format json exited 124. Scoped manual fallback inspected the new #[trace("TC-428", "FR-096-AC-8")] binding, owning FR-096 and TC-428, production arm and real assertions. Existing FR-096-AC-8 concerns its thirteen enumerated older causes; new bounded test extends the shared refusal behavior rather than claiming full QSL-1770 acceptance. No global coverage percentage is claimed. Semantic gap-analysis expansion skipped (not requested).

Plan completion: not assessed

Scope records:

```json
[
  {
    "id": "kernel_refusal_record",
    "path": "qsl-foundation/src/diagnostic.rs",
    "role": "examined",
    "excerpt": "Refusal::BoundedQuantityOutOfDomain { target }"
  },
  {
    "id": "unit_spelling",
    "path": "qsl-foundation/src/diagnostic.rs",
    "role": "examined",
    "excerpt": "fn unit_spelling(unit: &quire_exact::UnitId) -> String"
  },
  {
    "id": "a_bounded_quantity_refusal_renders_its_structural_target",
    "path": "qsl-foundation/tests/kernel_refusal_record.rs",
    "role": "examined",
    "excerpt": "expected(&format!(\"Quantity<{spelling}>[1/1, 2/1]\"))"
  },
  {
    "id": "FR-096-AC-8",
    "path": "spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md",
    "role": "context_only",
    "excerpt": "For each of the thirteen kernel causes in the key table, a `RefusalRecord` built from an S6a `Evaluation` whose outcome is that kernel `Refused` carries the table's code and cause, category refusal, exactly the table's field keys and the evaluation's resolved locus, and `Refusal::code()` and `Refusal::cause()` return the same code and cause."
  }
]
```

## Round 1 scope verification

Reviewed acc0cf0eae8be6de2c0f29d17fee3ae67985f55d: foundation owns one documented public structural formatter and replay consumes it without changing behavior. No new scoped traceability, reverse-ownership or stub gap. No matrix recomputation for confined consolidation. Verdict remains CONDITIONAL for previously recorded matrix timeout and pending paired-lock execution; full QSL-1770 integration remains open.

## Integration review round 2

Reviewed frozen edbd95d0b99033358a10cd3d11d6c839f04856ba. The owner-authorized one lock refresh supersedes the previous no-lock rule for this bump only: Cargo.lock changes only quire-exact 7e06eba7910e31146fd1b95ef56065cffb818363 to b58ac8f322611a7f68b7abb814df72ea2b9132f6, matching fresh origin/main. IR remains 54d46605a131c69dc1bc8664163315ad4f3f3f2f, already current; no artificial lock churn.

Examined qsl-replay/src/scalar.rs:618-638: BoundedQuantityOutOfDomain joins the existing domain-refusal arm as ScalarOutcome::OutOfRange. This is the typed semantic classification of an inclusive result-domain violation; it neither turns the refusal into a completed value nor catches checked invariants. Existing invariant/fault handling remains separate. No new findings, including lows; FND-001 remains fixed.

Observed actual logs: /tmp/qsl1770-foundation-focused.log reports the real focused bounded-refusal test passed (1 passed); /tmp/qsl1770-replay-check.log reports qsl-replay lib check finished successfully; /tmp/qsl1770-workspace-check.log now reports workspace dev check finished successfully in 22.97s. Reviewer did not rerun tests or any aggregate gate.

Current scoped verdict: PASS for this partial compile fix, subject to lead's final gate. This asserts only the reviewed changed source, lock compatibility and scoped behavior; it is not whole-repository assurance or acceptance of full QSL-1770. Earlier bounded matrix timeout is retained as an unavailable global measurement; no recomputation or global coverage claim. Strict URI expected rendering and bounded conversion integration remain open in full QSL-1737/QSL-1770.
