---
id: TC-484
title: "The legacy mapped and root extraction surface is absent while replay extraction remains"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-490
    type: verifies
---
# TC-484: The legacy mapped and root extraction surface is absent while replay extraction remains

## Description

Rust compile-fail probes, Cargo metadata checks, and the retained `qsl-replay`
extraction integration suite. This case verifies the exact public retirement
boundary in FR-490 without duplicating FR-030/FR-031's extraction behavior
cases.

## Test Procedure

1. Compile a downstream crate that attempts to import the root package's
   `mapped` module and `command::extraction` module and output types; confirm
   each import fails to compile because that item is absent.
2. Read the root package record from `cargo metadata --no-deps --format-version 1`.
   Confirm it declares no `quire-extraction` feature and no direct `qsl-source`
   dependency. Confirm the workspace still contains `qsl-source` and
   `qsl-replay` declares its own `quire-extraction` feature.
3. Run `cargo test -p qsl-replay --features quire-extraction` and the
   FR-030/FR-031 integration tests. Confirm the extraction test modules
   previously gated by the root feature in `tests/it/config_version_spine.rs`
   and `tests/it/spine_run.rs` are exercised without that feature and direct
   root dependency.

## Expected Results

- Steps 1 and 2 show that the root Rust API, root feature and direct root
  dependency are absent, while the `qsl-source` crate and replay-owned feature
  remain present.
- Step 3 passes with extraction coverage owned by the crate that owns the
  FR-030/FR-031 path. Extraction identity, source-map, limit, refusal and
  clause-run behavior remains as specified by those requirements.
