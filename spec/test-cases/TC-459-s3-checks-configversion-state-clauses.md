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

Scope: FR-104-AC-1, FR-104-AC-2, FR-104-AC-7, FR-104-AC-8,
FR-104-AC-9, FR-104-AC-10.

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
5. Against TC-466 step 3's `probe` package variant (`probe(target:
   ConfigVersion)`, no result, empty frame), check these clauses on
   `probe`, one unit each:
   a. the postcondition `deref(target).versionNumber = 1`;
   b. the precondition `deref(target).versionNumber = 1`;
   c. the postcondition `pre(deref(target).versionNumber) = 1`;
   d. the postcondition `pre(target) = target`.
6. Under the selected `quire.model.complete/v1` definition, extend the
   ConfigVersion fixture with `ArchivedConfigVersion` conforming to
   `ConfigVersion`. Let it inherit `parent: Option<Reference<ConfigVersion>>`
   and declare `probe(target: ConfigVersion)`. Check a precondition on
   `ArchivedConfigVersion::probe` with body
   `reaches(self, target, parent)`. Inspect the checked operand reference
   identities and resolved edge owner.
7. Extend the fixture package with a process `Flow` (identity field `x`
   typed `Integer`, field `n: Integer`, one step `begin` of kind `command`
   consuming and emitting nothing), a closed population of `Flow`, and its
   operation `go` with frame `modifies [Flow/n]`. Check, one unit each,
   `pre P using v on Config::Flow::go { self.n < 3 }`, `post Q using v on
   Config::Flow::go { self.n = pre(self.n) + 1 }` and `invariant I using v on
   Config::Flow at current { self.n >= 0 }`.

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
- Step 5: (a) checks, its read through `target` `post`; (b) checks, its read
  `pre`; (c) checks, its read `pre`; (d) `wrong_snapshot`/
  `forbidden-pre-read` at the `pre` (QSpec: "A pre selector cannot retag a
  post-qualified parameter reference").
- Step 6: the clause checks. `self` retains
  `Reference<ArchivedConfigVersion>`, `target` is
  `Reference<ConfigVersion>`, and `parent` resolves to
  the inherited field owned by `ConfigVersion`; no implicit change to either
  reference identity is recorded to make the edge type match. Evaluation
  retains each object's most-specific identity under QSpec FR-043/FR-010.
- Step 7: each checks. `self` is `Reference<Config::Flow>` and `self.n` an
  `Attribute` of type `Integer`; `P`'s read of `n` is `pre`, `Q`'s reads are
  `post` and `pre`, and `I`'s read is `current`. (Evaluation of such clauses
  per run, with `self` the run, is QSpec TC-235 M16's reference test.)
