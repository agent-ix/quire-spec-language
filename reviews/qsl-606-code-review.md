---
id: SR-1219
title: "Code review of quire-spec-language PR #598: cargo xtask checked-input (FR-270)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@a49ee006adcdd0e5014dbf97f5b196df93d10aa2; PR #598 diff against origin/main: Makefile, xtask/src/checked_input.rs, xtask/src/error.rs, xtask/src/lib.rs, xtask/src/main.rs, xtask/src/typestate_scan.rs"
review_set: subset
---
# Code review of quire-spec-language PR #598

## Summary

Ticket: QSL-606 (slice D1 of QSL-13). The PR adds `cargo xtask checked-input`
(ADR-032 CK-1 to CK-5), wires it into `make ci`, and shares
`typestate_scan::shipped_files` and `S3_CONSTRUCTOR` with the new module.

Examined:
- The CK-2 derivation (`PreCheck::derive`): reach closure from `pub` returns
  over the three pre-check crates, fielded-type filter, configuration filter.
  It holds no name list. The configuration rule is the accepted team-leader
  ruling and is not raised here. Measured: today it also excludes
  `qsl_source::Selection` and `qsl_cst::SyntaxLimit`, which fits the rule.
- CK-3 (`StageScanner::signature`) over receivers, parameters and generic
  bounds, through `use` renames, globs and `type` aliases.
- CK-4 (`StageScanner::reconstruction`) over plain paths in bodies, signatures
  and comma-expression macros.
- The `typestate_scan` refactor: behaviour is unchanged, and the TC-244 table
  now reads `S3_CONSTRUCTOR`.
- Rust lane (rust-review): no `unwrap`/`expect` on a production path, no
  `unsafe`. Error code and exit code are added in both matches with no `_`
  arm. The `#[qsl_attrs::string_edge]` markers sit on the string-matching
  helpers, as the repo convention requires.

Probes were run with a scratch binary over a copy of the scanned trees:
- `<PackageDeclarations>::check(d, l)` gives no finding.
- `checked_dispatch_operation(..)` followed by `declarations.check(limits)`
  gives no finding.
- `qsl_semantics::check::PackageDeclarations::check(d, l)`,
  `qsl_forms::build_unit(..)`, `impl Fn(&qsl_forms::Expression)` in a
  parameter, and `&qsl_cst::ParsedSource` are all found.

Gates at this head (worktree-local target dir): `cargo test -p xtask` passed
(87 tests), and `cargo clippy -p xtask --all-targets -D warnings` and
`cargo fmt --check` were clean.

## Verdict

Changes requested. FND-001 is high and FND-002 is medium.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | CK-4 misses an S3 re-run that the public API allows today. The module doc says a method-syntax call is safe because "a value of the receiver type has to be named". That is false. `qsl_semantics::check::checked_dispatch_operation` is `pub` and returns `PackageDeclarations`, so a private `qsl-eval` function can write `let d = checked_dispatch_operation(view, root, clauses, src, meter)?; d.check(limits)`. That names no pre-check path, and the probe gives no finding. A qualified-self call `<PackageDeclarations>::check(d, l)` is missed too, because `path_segments` drops `qself` and leaves one segment. Fix: resolve `qself` as the owner segment. Also report a method call named `S3_CONSTRUCTOR.name` in shipped stage-crate code: today every `.check(` call in `qsl-eval`, `qsl-route` and non-spine `qsl-replay` is in `#[cfg(test)]`, so it causes no false finding. Add both as TC-745 plants and delete the false rationale from the module doc. | xtask/src/checked_input.rs:45-51; xtask/src/checked_input.rs:377-392; xtask/src/checked_input.rs:627-645; qsl-semantics/src/check/checked_dispatch.rs:1303-1309 |
| FND-002 | medium | ADR-032 §1 says "The gate checks names, so a pre-check type that `qsl-semantics` re-exports ... is still caught". The code matches a fielded pre-check type only when the path is rooted at `qsl_source`/`qsl_cst`/`qsl_forms` (`is_representation`), and the module doc lists re-exports as a known limit. So the code does not do what the ADR says. Following re-exports is small: build a `UseTable` of `pub use` items over the workspace crates the stage crates depend on (`qsl-semantics`, `qsl-package`, `qsl-foundation`, `qsl-eval`), and add each entry that resolves into a pre-check crate as an alias in `Resolver`. Do that here, with a plant, rather than narrowing the ADR. | xtask/src/checked_input.rs:361-372; xtask/src/checked_input.rs:49-51; spec/decisions/ADR-032-checked-input-and-duplicate-canonical-type-gates.md (Decision §1, paragraph after "What the gate rejects") |
