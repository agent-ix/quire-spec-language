---
id: SR-20261104025137
title: QSL-1533 PR #2250 code review
type: SpecReview
analysis: code-review
date: '2026-11-04'
scope: 'agent-ix/quire-spec-language; QSL-1533 PR #2250 at 4676b710d60b0469654fb14aeab3a24bca63bab8; aggregate quantity refusal ordering, cause and source attribution'
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-514
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-480
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: references
---

## Summary

Ticket: QSL-1533. PR: agent-ix/quire-spec-language#2250. Reviewed the frozen head `4676b710d60b0469654fb14aeab3a24bca63bab8` against `810535be0cc74507133de4b2fb7bc13277b69bd4` and inspected the complete changed-path diff.

The assembler retains canonical unit order for successful package assembly. For a noncanonical request whose canonical pass first finds a quantity topology defect, it rechecks in caller order and returns that refusal. The spine keeps input order as indices, preserves a canonical primary source for unrelated failure regions, maps named quantity errors to the owning input source, and exposes the assembler's catalog cause through `CompileRefusal`. Tests assert both FR-480-AC-6 list orders, exact declaration source spans, topology causes and cycle edges. Existing aggregate assembly tests exercise successful results in reversed input order and assert canonical source ownership and stable identities.

No source findings remain on the reviewed head. `cargo fmt --all -- --check` and `git diff --check` pass. Clippy and focused Rust tests were unavailable: clean main at `810535be` fails `cargo check --locked --workspace --all-targets` in `qsl-foundation/src/diagnostic.rs` because `Category::Pending` and `Category::Indeterminate` are unhandled (QSL-1621, PR #2248). No build or test was run by this reviewer. No Cargo lockfiles, manifests or CI workflows changed. No applicable AssuranceProfile was found under `spec/`.

## Scope

```yaml
scope:
  - id: CompileRefusal::cause
    path: qsl-replay/src/spine.rs
    role: examined
    excerpt: "Assembly refusals return the catalog cause of their first AssemblyError."
  - id: assembly_error_source and finish_unit
    path: qsl-replay/src/spine/lifecycle.rs
    role: examined
    excerpt: "QuantityCycle and UnitGraphTopology declaration names select the input source for the refusal region; unmatched causes retain the canonical primary source."
  - id: AdmittedUnitList input order and Frame::new_units
    path: qsl-replay/src/spine/lifecycle.rs
    role: examined
    excerpt: "The caller's order is stored as indices into canonical units; combined unit forms and the primary fallback source remain canonical."
  - id: PackageDeclarations::assemble_units
    path: qsl-semantics/src/check/assemble.rs
    role: examined
    excerpt: "Canonical assembly returns successful packages; a noncanonical topology refusal is retried in caller order to select the first listed defect."
  - id: aggregate_quantity_cycles_and_zero_scale_name_the_declaration_source
    path: qsl-replay/src/spine/lifecycle/tests.rs
    role: examined
    excerpt: "Checks both list orders, invalid_package causes, exact name spans, source references and unit/dimension cycle edges."
  - id: u11q_the_first_document_takes_the_first_turn_whatever_its_offsets
    path: qsl-semantics/src/check/assemble/tests.rs
    role: examined
    excerpt: "Exercises [1, 2] and [2, 1], traces FR-480-AC-6, and asserts the first listed topology defect."
  - id: FR-514-AC-5
    path: spec/functional/FR-514-specify-the-spine-type-report-the-inventory-on-exhaustion-and-the-multi-unit-cycle-refusals.md
    role: examined
    excerpt: "The source-owning documents produce the specified zero-scale, self-cycle, cross-unit cycle and dimension-cycle refusals with the listed causes and edges."
  - id: FR-480-AC-6
    path: spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md
    role: examined
    excerpt: "The [1, 2] input reports the zero scale in document 1; [2, 1] reports u_self's cycle in document 2; the first list defect is reported at its source declaration span."
  - id: FR-091-AC-46
    path: spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md
    role: examined
    excerpt: "The first listed source document takes its turn before declaration byte offset."
  - id: FR-514-AC-1..4
    path: spec/functional/FR-514-specify-the-spine-type-report-the-inventory-on-exhaustion-and-the-multi-unit-cycle-refusals.md
    role: context_only
    excerpt: "Existing type-report, inventory-on-limit and formal-identity behavior provides the surrounding FR-514 context; this PR does not change those paths."
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

CONDITIONAL: static source review found no defects on the frozen head. Merge verification remains conditional on the main compile repair and then the focused tests and Rust gates.

## Coverage

Code-review ran with the Rust review lane and rust-style baseline folded into this artifact. Checked behavior, source ownership, error code/cause mapping, canonical successful output, test assertions and trace tags; inspected duplication, integrity, safety and caller-controlled failure paths. The changed paths are Rust only, so React/Python lanes do not apply. Spec-review is inapplicable because no spec or plan path changed. Quoin gap-analysis is recorded separately. Formatter and diff checks passed; no Clippy, test or aggregate gate ran because the base does not compile. No AssuranceProfile applies.
