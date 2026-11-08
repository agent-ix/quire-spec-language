---
id: TC-1399
title: "release-check builds the current release tag and fails on a vanished locked revision"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-018
    type: verifies
---
# TC-1399: release-check builds the current release tag and fails on a vanished locked revision

## Description

Run `make release-check` at the current release tag and at scratch tags that
cannot build.

Scope: NFR-018-AC-1 to NFR-018-AC-3.

## Test Procedure

1. Run `make release-check TAG=<the current release tag>`.
2. In a scratch repository, tag a commit whose `Cargo.lock` records a git
   revision that its dependency repository does not contain, and run the
   gate at that tag.
3. Run the gate with a tag name that does not exist.
4. In a scratch repository, run `make tag-release TAG=v9.9.9` at a commit
   whose `Cargo.lock` lacks a declared dependency, then at a commit where
   the clean-clone gate passes; list `git tag -l v9.9.9` after each.

Tag the tests `#[trace("TC-1399", "<AC id>")]`.

## Expected Results

- Step 1: the gate passes: the locked build and the source check succeed.
- Step 2: the gate fails, naming the dependency and the revision.
- Step 3: the gate refuses, exit 2, and clones nothing.
- Step 4: the first exits non-zero with no tag; the second creates the
  annotated tag at `HEAD`.
