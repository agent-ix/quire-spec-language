---
id: SR-243
title: "Code and Rust review of bounded integer IR lowering"
type: SpecReview
analysis: code-review
scope: "src/lowering.rs; src/lowering/wire.rs; src/command*; src/main.rs; integer tests and examples"
review_set: subset
---
## Summary

Author PR-readiness review of 7b5b663 using actual code-review, rust-review and
rust-style skills. Numeric implementation is unchanged from 5a7e5db; this
revision corrects generator I/O handling. No applicable AssuranceProfile or
deny.toml exists.

## Verdict

**PASS** — no remaining scoped code/Rust findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Checks

One explicit target enum extends the existing lowerer and CLI. Translation walks
the checked native AST and borrows actual scalar/declaration representations;
it does not reuse proof-abstraction inputs or evaluate predicates into constants.
The borrowed wire views encode the existing IR input contract. Both pinned
strict readers reconstruct the actual bytes and agree on bound identity.

The exhaustive binary-operator mapping preserves operand order. Original
source/observation correspondence and complete-package limits stay in the shared
path. Unsupported fields/locals/calls/objects refuse; numeric literal conversion
is fallible. No dependency, unsafe block, production panic, shared mutable state
or permissive request decoder was added. Target selection validates before I/O.
The example helper now returns io::Result for every filesystem operation.
The executable reports selected-path failures with exit 2; only known static
fixture constructor defects retain assertions. Both native and Markdown paths
handle early and later write failure without panic.

Seven new tests carry real trace attributes. Assertions independently name numeric
operators/bounds, compare original source fragments, check model/observation
identity and exact limits, exercise actual command output and current backend
unsupported-expression diagnostics. Healthy/violating runtime results have the
same source-only projection, ruling out snapshot-specific constant evaluation.

Local gates passed: fmt; strict all-targets/all-features Clippy; 314 ordinary
tests plus three compile-fail doctests; nine relevant minimal-feature tests;
minimal build; rustdoc with warnings denied. Four existing assurance tests remain
ignored. Fresh example files also produced true/false and identical integer IR
outside the harness. Boolean lowering and existing generated tests passed.

Those full-suite results are the unchanged numeric implementation baseline at
5a7e5db. For 7b5b663, all 19 affected command tests, six minimal-feature tests,
fmt, strict all-target Clippy and the actual example build pass. Invalid output
directories and a late Markdown write return exit 2; fresh generation and native
execution succeed. The core compiler/runtime source did not change.
