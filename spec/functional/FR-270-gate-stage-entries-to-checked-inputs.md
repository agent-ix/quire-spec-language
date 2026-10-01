---
id: FR-270
title: "Gate execution, routing and replay entries to checked inputs"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-009
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-032
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-064
    type: traces_to
---
# FR-270: Gate execution, routing and replay entries to checked inputs

## Description

QSL SHALL provide `cargo xtask checked-input`, a gate that fails when an
execution, routing or replay stage entry names a pre-check representation
in its signature, or when shipped code in a stage-entry crate calls a
pre-check stage, as ADR-032 CK-1 to CK-5 decide. With FR-087's typestate
scans (TC-243, TC-244), which keep stage-output constructors private to
their stage, it closes the path: a stage entry receives a checked value,
and only a stage constructor produces one. The gate is a prerequisite of
`make ci`.

## Inputs

- The shipped source (test code excluded, as `typestate_scan` excludes it)
  of the workspace crates, read by `syn`.

## Outputs

- One finding per violation, each naming the file and line, the rule
  (`signature` or `reconstruction`), the stage entry or function, and the
  pre-check type or function it names.
- Exit status 0 when there is no finding, non-zero otherwise.

## Behavior

- The gate SHALL take as stage entries every `pub` function and `pub`
  method in the shipped code of `qsl-eval` and `qsl-route`, and every `pub`
  function of the `replay` facade crate `qsl-replay` outside its `spine`
  module. Membership comes from crate and module alone.
- The gate SHALL take as pre-check representations:
  - a type with fields, defined in `qsl-source`, `qsl-cst` or `qsl-forms`,
    that the return type of one of those crates' `pub` functions reaches;
  - `PackageDeclarations`, the receiver of the S3 stage constructor in
    TC-244's constructor table;
  - every type defined in `quire-contract-model`.
- The gate SHALL treat a fieldless enum from `qsl-source`, `qsl-cst` or
  `qsl-forms` (such as `StateClauseKind`) as closed vocabulary that S3
  carries forward, and a configuration type no pre-check stage returns
  (such as `qsl_cst::Limits`) as configuration; both are outside the set.
- If a stage entry's receiver or parameter type mentions a pre-check
  representation at any depth, seen through `use … as` renames and `type`
  aliases as `typestate_scan` sees them, then the gate SHALL report a
  `signature` finding.
- If a shipped function in `qsl-eval`, `qsl-route`, or `qsl-replay` outside
  `spine` names a `pub` function of `qsl-source`, `qsl-cst` or `qsl-forms`,
  or the S3 stage constructor, then the gate SHALL report a
  `reconstruction` finding, whatever the function's own signature.
- The gate SHALL report every finding it meets before it exits.
- The gate SHALL derive both sets from the rules above applied to the
  scanned code.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-270-AC-1 | Planted signatures fail the gate: over a copy of the workspace's scanned trees with one planted item each, a `pub fn` in `qsl-eval` taking `&qsl_forms::Expression`; the same through `use qsl_forms::Expression as Expr`; the same through a `type` alias of a CST node; a `pub` method in `qsl-route` taking `Option<Vec<&PackageDeclarations>>`; and a `pub fn` in `qsl-replay` outside `spine` taking a `quire-contract-model` package, each make the gate exit non-zero with one `signature` finding naming the planted file and line, the entry and the type. | Test (TC-745) |
| FR-270-AC-2 | Planted reconstruction fails the gate: a private function in `qsl-eval` with a checked-only signature that calls a `qsl-cst` parse function, and one in `qsl-route` that calls `PackageDeclarations::check`, each give one `reconstruction` finding naming the file, line, function and the called stage function. The same parse call planted in `qsl-replay`'s `spine` module gives no finding. | Test (TC-745) |
| FR-270-AC-3 | No false findings: a `pub fn` in `qsl-eval` taking `StateClauseKind`, a function in `qsl-replay` outside `spine` that constructs `qsl_cst::Limits`, and a `#[cfg(test)]` module in `qsl-eval` that takes a `qsl_forms::Expression`, give no finding. Two planted signatures in one run give two findings. | Test (TC-745) |
| FR-270-AC-4 | Over the QSL workspace as it is, `cargo xtask checked-input` reports nothing and exits 0. With one FR-270-AC-1 signature planted in `qsl-eval`'s shipped code, the `checked-input` make target that `make ci` runs exits non-zero. | Test (TC-745) |

## Dependencies

- [ADR-032](../decisions/ADR-032-checked-input-and-duplicate-canonical-type-gates.md)
  CK-1 to CK-5.
- [FR-087](FR-087-typestate-and-cross-package-node-key.md) (the typestate
  scans and the S3 stage constructor table, TC-243, TC-244).
- ADR-011 §4 (checking precedes lowering) and §6.1 (the layers).

## References

- Owning ticket: Linear QSL-391. Implementation: Linear QSL-13.
