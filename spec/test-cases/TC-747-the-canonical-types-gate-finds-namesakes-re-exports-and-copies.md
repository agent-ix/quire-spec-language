---
id: TC-747
title: "The canonical-types gate finds namesakes, re-exports and same-shaped copies"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-272
    type: verifies
---
# TC-747: The canonical-types gate finds namesakes, re-exports and same-shaped copies

## Description

Verify FR-272's identifier, re-export and copy rules, its exclusions, a
backend-workspace run and its reporting, over fixture workspaces and the QSL
workspace.

Scope: FR-272-AC-1 to FR-272-AC-4.

## Test Procedure

1. Fixture workspace: crate K tags `pub enum Value`; crate W holds, one per
   run, a private `struct Value`; a `pub(crate) enum Value` in a nested
   module; `impl<'de> Visitor<'de> for V { type Value = T; … }`; a `struct
   Value` in `#[cfg(test)] mod tests`; `pub use k::Value;`. Then K itself
   holds `pub use self::value::Value;`.
2. Fixture `cargo metadata` with K a workspace member tagging `pub enum
   ComparisonOperator` and E an ecosystem package holding, one per run, an
   enum with K's variant names in order; the same with docs, derives,
   visibility and member type paths changed; with `LessEqual` for
   `LessOrEqual`; with two variants swapped; a `struct` with those field
   names. Then the same copy in a package outside the ecosystem.
3. A fixture backend workspace depending on K as an ecosystem package and
   defining `pub enum Value` with K's variants. Then run the gate over the
   QSL workspace and read the `quire-contract-model` findings.
4. Plant two namesakes in W in one run; run over a fixture with none.

Tag the tests `#[trace("TC-747", "FR-272-AC-n")]`.

## Expected Results

- Step 1: `identifier` findings for the first two plants naming both
  locations; none for the associated type or the test-module struct; a
  `re-export` finding for W's `pub use`; none for K's.
- Step 2: `copy` findings for the first two; none for the other three; none
  outside the ecosystem.
- Step 3: one `copy` finding in the backend run; no `copy` finding for any
  of `CollectionType`, `ComparisonOperator`, `EnumDeclaration`,
  `IntegerDomain` or `ValueType`.
- Step 4: two findings, each with both locations and its rule, and a
  non-zero exit; then exit 0.
