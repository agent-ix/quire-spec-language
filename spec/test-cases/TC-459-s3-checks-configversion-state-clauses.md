---
id: TC-459
title: "S3 checks the ConfigVersion state clauses and types self, result and pre"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: verifies
---
# TC-459: S3 checks the ConfigVersion state clauses and types self, result and pre

## Description

Verify that the `ProtocolClause` check admits the ConfigVersion clauses and
types their context reads.

Scope: FR-104-AC-1, FR-104-AC-2, FR-104-AC-7.

## Test Procedure

Compile through S1 to S3 against TC-458's fixture package, with the header of
FR-108's unit.

1. Check `ParentOrder`, `NoCycle` and `VersionUnchanged` as FR-108 writes
   them.
2. Check `post R using v on Config::ConfigVersion::attemptUpdate { result }`.
3. Check `invariant I using v on Config::ConfigVersion at current { result }`
   and `pre Q using v on Config::ConfigVersion::attemptUpdate { result }`.
4. Against the fixture package with `attemptUpdate`'s frame changed to
   `modifies [ConfigVersion/parent]`, check these postconditions of
   `attemptUpdate`, one unit each:
   a. `present(pre(self.parent)) implies deref(value(self.parent)).versionNumber > 0`;
   b. `pre(present(self.parent) implies deref(value(self.parent)).versionNumber > 0)`;
   c. `let v = self.versionNumber in pre(v) = 1`;
   d. `let s = self in pre(s.versionNumber) = 1`;
   e. `let s = pre(self) in s.versionNumber = 1`.

Tag the tests `#[trace("TC-459", "FR-104-AC-n")]`.

## Expected Results

- Step 1: three checked clauses, kinds `Invariant`, `Invariant`,
  `Postcondition`. In the checked bodies `self` is
  `Reference<Config::ConfigVersion>`, `self.versionNumber` is an `Attribute`
  node of type `Int[0, 1000]`, `self.parent` is
  `Option<Reference<Config::ConfigVersion>>`, and `pre(self.versionNumber)`
  is a `Pre` node over that `Attribute`. `ParentOrder` checks with no
  definedness refusal, and each of its reads is `current`; in
  `VersionUnchanged` the left read is `post` and the right `pre`.
- Step 2: checks, with `result: Boolean`.
- Step 3: each refuses `wrong_snapshot`/`wrong-anchor` at `result`, naming
  the clause kind and `attemptUpdate` (the invariant names none).
- Step 4: (a) `undefined_expression`/`unproved-presence` at
  `value(self.parent)`; (b) checks, every read `pre`; (c) and (d)
  `wrong_snapshot`/`forbidden-pre-read` at the `pre`; (e) checks, its read
  `pre`.

## Status

Planned (QSL-273). The code waits for the other lane's work in `check/`.
