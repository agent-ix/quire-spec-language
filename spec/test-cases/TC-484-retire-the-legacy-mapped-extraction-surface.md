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
3. Before removing the root feature, run
   `cargo test -p quire-spec-language --features quire-extraction,qsl-replay/quire-extraction --test config_version_spine --test spine_run`
   to exercise the current root-owned cases. The code change moves those
   cases from `tests/it/config_version_spine.rs` and `tests/it/spine_run.rs`
   into `qsl-replay` or `qsl-source` tests before deleting their former
   locations. In the resulting test sources, locate passing tests whose
   trace tags include each of `FR-030-AC-1`, `FR-030-AC-2` and
   `FR-031-AC-2`. Confirm the existing `qsl-source` tests tagged
   `FR-030-AC-3`, `FR-030-AC-4` and `FR-030-AC-5` remain. Run
   `cargo test -p qsl-replay --features quire-extraction` and
   `cargo test -p qsl-source --features quire-extraction` for the
   post-retirement owning-crate suites.

## Expected Results

- Steps 1 and 2 show that the root Rust API, root feature and direct root
  dependency are absent, while the `qsl-source` crate and replay-owned feature
  remain present.
- Step 3 shows each named acceptance criterion has a tagged, passing test
  under `qsl-replay` or `qsl-source`, with the proper owning feature enabled.
  Extraction identity, source-map, limit, refusal and clause-run behavior
  remains as specified by those requirements.
