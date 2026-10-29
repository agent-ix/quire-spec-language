---
id: FR-490
title: "Retire the legacy mapped and root extraction surface"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-030
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-031
    type: references
---
# FR-490: Retire the legacy mapped and root extraction surface

## Description

Following ADR-012 §15.8 step 2, QSL SHALL remove the legacy mapped native
compiler and its root-command extraction adapter. The retirement SHALL remove
the exact surfaces listed in AC-1 and AC-2. It SHALL NOT remove or weaken the
`qsl-source` consumer and `qsl-replay` extraction path that FR-030 and FR-031
define.

## Behavior

- The root QSL package SHALL have no `mapped` module, mapped compilation API,
  `command::extraction` adapter, or extraction output type.
- QSL SHALL delete the corresponding source files and mapped integration
  test listed in FR-490-AC-1.
- The root package SHALL have no `quire-extraction` Cargo feature or direct
  optional `qsl-source` dependency.
- The workspace SHALL retain the `qsl-source` crate and
  `qsl-replay/quire-extraction` feature.
- The root package SHALL contain no compatibility alias, re-export, fallback,
  feature-gated duplicate, or replacement adapter for the retired API.
- When a caller builds `qsl-replay` with `quire-extraction`, QSL SHALL retain
  FR-030's verified Quire extraction and FR-031's spine clause-run behavior
  through `qsl-source` and `qsl-replay`'s
  `quire-extraction` feature. Their existing results, refusals, identities,
  source maps, and limits remain governed by those requirements.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-490-AC-1 | A downstream Rust crate cannot import the root package's `mapped` module or `command::extraction` module or its output types. The implementation removes `src/mapped.rs`, `src/command/extraction.rs`, `src/command/output/extraction.rs`, and `tests/it/mapped.rs`. | Test |
| FR-490-AC-2 | Root `Cargo.toml` declares neither a `quire-extraction` feature nor a direct optional `qsl-source` dependency; it has no compatibility feature, alias, or fallback for either. The `qsl-source` workspace crate and `qsl-replay/quire-extraction` feature remain declared. | Test |
| FR-490-AC-3 | After the root surfaces are removed, FR-030-AC-1, FR-030-AC-2 and FR-031-AC-2 each remain backed by a passing test owned by `qsl-replay` or `qsl-source`, with that acceptance-criterion ID on the test's trace tags. The tests now at `tests/it/config_version_spine.rs` and `tests/it/spine_run.rs` move to that owning crate as needed; the existing `qsl-source` tests tagged for FR-030-AC-3/4/5 remain. The replay feature and the root integration cases build and pass with the owning feature enabled. | Test (TC-484) |

## Dependencies

- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md):
  §15.8 step 2 authorizes deletion of the mapped path as part of M-6c.
- [FR-030](FR-030-consume-quire-extraction.md) and
  [FR-031](FR-031-run-extracted-native-source.md): retained extraction and
  clause-run path.
