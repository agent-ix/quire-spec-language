---
id: TC-1399
title: "release-check builds the newest tag and fails on a vanished locked revision"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-018
    type: verifies
---
# TC-1399: release-check builds the newest tag and fails on a vanished locked revision

## Description

Run `make release-check` at the newest release tag and at scratch tags that
cannot build.

Scope: NFR-018-AC-1 and NFR-018-AC-2.

## Test Procedure

1. Run `make release-check TAG=<newest v* tag>`.
2. In a scratch repository, tag a commit whose `Cargo.lock` records a git
   revision that its dependency repository does not contain, and run the
   gate at that tag.
3. Run the gate with a tag name that does not exist.

Tag the tests `#[trace("TC-1399", "<AC id>")]`.

## Expected Results

- Step 1: the gate passes: the locked build and that commit's README
  examples succeed.
- Step 2: the gate fails, naming the dependency and the revision.
- Step 3: the gate refuses, exit 2, and clones nothing.
