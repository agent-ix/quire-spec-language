---
id: SR-375
title: "Code review — composed admission stages, TC-115 static subject and request report"
type: SpecReview
analysis: code-review
scope: "src/linking/composed/subject.rs; src/linking/composed/requests.rs; src/linking/composed.rs; src/linking/composed/scopes.rs; src/syntax.rs; tests/composed_admission_stages.rs; spec/functional/FR-036-link-composed-native-packages.md; spec/model-linking/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-115
    type: references
---

## Summary

`/code-review` with the Rust lane (`agent-skills/rust-review/SKILL.md`) over the
TC-115 increment: two new public modules (`linking::composed::subject`,
`linking::composed::requests`), eleven traced integration controls in
`tests/composed_admission_stages.rs`, and two derive additions
(`syntax::Limits`, `scopes::BinderType`). No `AssuranceProfile` exists in
`spec/`, so no `## Assurance Context` section applies. Six findings were raised
and all six were fixed before this artifact was written; the table records them
with their fixes.

## Verdict

**CONDITIONAL** — no finding remains open. Two were `high` at the moment they
were found (a library panic surface and a comparison-defeating coercion) and are
recorded here with their fixes rather than erased; the remaining four are
`medium`/`low`. All five local gates pass after the fixes.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | `model_selection` indexed `inputs[input]`, so a resolved import index outside the supplied model slice panicked the library instead of reporting. Fixed: `inputs.get(input)` returning `Option`, surfaced as `Unavailable::UnmatchedModelInput` | src/linking/composed/subject.rs:392 | implementation-bug-despite-evidence |
| FND-002 | high | `ModelSelection.revision` was `u64` built by `revision.parse().unwrap_or_default()`, so two different unparseable producer revisions both became `0` and two distinct static selections compared EQUAL — defeating the exact comparison FR-036-AC-5 requires. Fixed: the revision is retained as its original spelling | src/linking/composed/subject.rs:92 | implementation-bug-despite-evidence |
| FND-003 | medium | A missing declaration syntax fell back to an empty package name, so two different missing declarations would compare equal and a profile or role component would be attributed to `""`. Fixed: names are resolved once, up front, and a gap returns `Unavailable::UnmatchedDeclaration` | src/linking/composed/subject.rs:231 | correct-requirement-no-evidence |
| FND-004 | medium | A missing authored model alias fell back to an empty alias by the same pattern. Fixed: `Unavailable::UnmatchedModelAlias` | src/linking/composed/subject.rs:258 | correct-requirement-no-evidence |
| FND-005 | medium | `cargo clippy --all-targets -- -D warnings` failed on `clippy::needless_lifetimes` in the new test helper; the failure had been masked by piping the gate through `tail`, which reports the pipe's status. Fixed: lifetime elided, and every later gate run records its own `$?` | tests/composed_admission_stages.rs:763 | correct-requirement-no-evidence |
| FND-006 | low | `requests::report`'s parameter `binding` shadowed the imported `binding` module used in the same function's match arms, and a new `#[allow(dead_code)]` carried no reason. Fixed: parameter renamed to `bound`; the allow carries a comment | src/linking/composed/requests.rs:281 | correct-requirement-no-evidence |

## Gates

Run locally, serially, one Cargo job, in this worktree's own `./target`
(`env -u CARGO_TARGET_DIR`, because `CARGO_TARGET_DIR` is set to a dir shared
across worktrees). Exit status captured per command, never through a pipe.

| Gate | Command | Result |
| --- | --- | --- |
| 1 | `cargo fmt --all -- --check` | exit 0 |
| 2 | `cargo clippy -j 1 --all-targets -- -D warnings` | exit 0 |
| 3 | `cargo clippy -j 1 --all-targets --features quire-extraction -- -D warnings` | exit 0 |
| 4 | `cargo test -j 1 -- --test-threads=1` | exit 0 — 616 passed / 0 failed / 4 ignored over 61 binaries |
| 5 | `cargo test -j 1 --features quire-extraction -- --test-threads=1` | exit 0 — 632 passed / 0 failed / 4 ignored over 61 binaries |

The four ignored tests are pre-existing and outside this scope.

## Seam and completeness notes

The controls drive the real public pipeline — `admit_namespace` →
`binding::bind` → `subject::StaticSubject::of` / `requests::report` — and the
real historical `NativePackage::read_verified` and `Snapshot::read_verified`.
No `#[cfg(test)]` branch, feature flag or double replaces the unit under test.
Each refusal is asserted on its own typed error rather than on a shared generic
code: `ImportRefusal::StaleSelection`; `Disposition::UnsupportedCapability`,
`UnsupportedFamily`, `InapplicableCapability`, `RefusedSubject`,
`UnknownSubject`; `InventoryGap::MissingResponse`, `ExtraResponse`,
`MismatchedResponse`; and `Code::UnknownEdition`, `UnknownProfile`,
`UnknownWire`, `StaleDependency` with their `PackageStage`/`InputReadStage` and
exact decode paths.

No canonical digest, structural hash or JSON fingerprint was introduced;
`StaticSubject` compares the package contract's six declared components
directly, which is what the test case's Expected Results require.
