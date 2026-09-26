---
id: TC-456
title: "S2 builds state clause forms and the self, result and reaches expressions"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-102
    type: verifies
---
# TC-456: S2 builds state clause forms and the self, result and reaches expressions

## Description

Verify that `qsl-forms` builds a `StateClauseForm` for each of `invariant`,
`pre` and `post`, and builds `self`, `result` and `reaches` as expressions.

Scope: FR-102-AC-1, FR-102-AC-2, FR-102-AC-3.

## Test Procedure

Every unit starts with the header
`language "ix:native" edition "1-draft";`, one `profile v = ...;` and
`model Config = "example/config-version" version "1" digest "sha256-jcs:<64 hex>";`.
Parse each through S1 and build through `qsl_forms::build_unit`.

1. Declare `invariant ParentOrder using v on Config::ConfigVersion at current
   { present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber }`.
2. Declare `pre P using v on Config::ConfigVersion::attemptUpdate { true }`,
   then the same with `post`.
3. Declare `invariant NoCycle using v on Config::ConfigVersion at current
   { not reaches(self, self, parent) }`, then
   `post VersionUnchanged using v on Config::ConfigVersion::attemptUpdate
   { self.versionNumber = pre(self.versionNumber) }`, then
   `post R using v on Config::ConfigVersion::attemptUpdate { result }`.

Tag the tests `#[trace("TC-456", "FR-102-AC-n")]`.

## Expected Results

- Step 1: one `DeclarationForm::StateClause` with kind `Invariant`, name
  `ParentOrder`, profile `v`, context `Config::ConfigVersion`, no operation.
  Each sub-expression's span slices the source to its own text
  (`self.parent`, `value(self.parent)`, and so on).
- Step 2: kind `Precondition` with operation `attemptUpdate`; then kind
  `Postcondition` with the same operation.
- Step 3: `Not(Reaches { SelfRef, SelfRef, edge: "parent" })`; an equality
  whose left is `Field(SelfRef, versionNumber)` and whose right is
  `Pre(Field(SelfRef, versionNumber))`; `Expression::Result`. No step
  refuses.

## Status

Planned (QSL-273). The code waits for the other lane's work in `qsl-forms`.
