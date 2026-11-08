---
id: TC-1397
title: "A clean clone builds locked from public sources, and the gate fails on a broken lock and cleans up"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-017
    type: verifies
---
# TC-1397: A clean clone builds locked from public sources, and the gate fails on a broken lock and cleans up

## Description

Run `make clean-clone-check` at `main` and at scratch commits that break it,
and check that it leaves nothing behind.

Scope: NFR-017-AC-1, NFR-017-AC-3 and NFR-017-AC-4.

## Test Procedure

1. Run `make clean-clone-check` at the head of `main`. In its clone, list
   every package source `cargo metadata --locked` reports, and run
   `git ls-remote` on each git source with `GIT_TERMINAL_PROMPT=0` and no
   credential helper.
2. In a scratch repository, make three commits: one adding a dependency to
   `Cargo.toml` without updating `Cargo.lock`; one whose `Cargo.lock` names a
   `path` source outside the repository; one removing
   `examples/config-version/requests.sh`. Run the gate at each.
3. After each run, list the temporary directories the gate reported making.

Tag the tests `#[trace("TC-1397", "<AC id>")]`.

## Expected Results

- Step 1: the gate passes; every source is the clone, crates.io or an
  anonymously readable `https://github.com/agent-ix/` repository.
- Step 2: the first fails at the build with Cargo's `--locked` refusal; the
  second fails naming the path source; the third fails at the examples step.
- Step 3: none of the directories remains.
