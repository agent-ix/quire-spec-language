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

Run every gate invocation with `TMPDIR` set to a new empty directory.

1. Run `make clean-clone-check` at the head of `main` and read its report.
2. In a scratch repository, make three commits: one adding a dependency to
   `Cargo.toml` without updating `Cargo.lock`; one adding a path dependency
   outside the repository; one removing `examples/config-version/requests.sh`.
   Run the gate with `REPO` the scratch repository at each commit. Run it once
   more with a `.cargo/config.toml` placed in `TMPDIR`.
3. After each run, list `TMPDIR`.

Tag the tests `#[trace("TC-1397", "<AC id>")]`.

## Expected Results

- Step 1: the gate passes; its report shows steps 1 to 5 passed, every
  source being the clone, crates.io or an anonymously readable
  `https://github.com/agent-ix/` repository.
- Step 2: the first fails at step 3 with Cargo's `--locked` refusal; the
  second fails naming the path dependency; the third fails at step 5; the
  last refuses at step 1. Each report names the failing step.
- Step 3: `TMPDIR` is empty.
