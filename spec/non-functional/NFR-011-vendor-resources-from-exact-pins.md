---
id: NFR-011
title: "Vendor resources from exact pins"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quire-spec-language/FR-036"
    type: constrains
  - target: "ix://agent-ix/quire-spec-language/FR-055"
    type: constrains
  - target: "ix://agent-ix/quire-spec-language/IT-011"
    type: references
---
# NFR-011: Vendor resources from exact pins

## Statement

When `resources/native-v1`, `resources/complete-value`,
`tests/fixtures/architecture` or `tests/fixtures/modules` is vendored or
re-vendored, the `quire-spec-language` repository shall read every path from
the exact commit recorded in that tree's `VENDOR.json` manifest (or verify an
externally hosted file by its recorded digest alone), write exactly those
bytes, replace the tree wholesale at a new pin, and detect drift or an
untracked file offline with no clone and no network access.

## Scope

This applies to all four vendored trees -- `resources/native-v1`,
`resources/complete-value`, `tests/fixtures/architecture` and
`tests/fixtures/modules` -- their `VENDOR.json` manifests, and the `xtask`
`revendor`/`revendor-check` commands that read and check them. Every
source is named by a full commit sha read from a local clone
(`agent-ix/quire-specification` for a `Qspec` source, this repository's
own history for a `SelfRepo` source, or `agent-ix/filament-core-data` for
an `Fcd` source -- `--qspec-clone`/`--fcd-clone`) or by a recorded digest,
and the closed registry (`src/linking/composed/definition_source.rs`)
interprets `resources/native-v1`'s resulting bytes under its own rules;
`tests/fixtures/architecture`/`tests/fixtures/modules` are read directly
by this crate's own tests. `resources/native-v1` is a historical
selection pinned at its recorded commits. An `Fcd` source's own `commit`
is additionally checked against this workspace's
`Cargo.toml`/`Cargo.lock` `agent-ix-semantic-ir` pin (`xtask::cargo_pin`),
a separate consistency check this NFR does not itself define an
acceptance criterion for.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Manifest-recorded commits accepted without a full 40-character sha | 0 | 0 | manifest validation inspection |
| Vendored bytes not matching their pinned git blob or recorded digest | 0 | 0 | `revendor-check` digest comparison |
| Files present under a vendored tree that neither `VENDOR.json` nor the exclusion list (`VENDOR.json`, `README.md`) accounts for | 0 | 0 | `revendor-check` tree walk |
| Bytes written by a second `revendor` run at an unchanged pin | 0 | 0 | idempotency test |

## Verification

Run `cargo test --workspace`, which exercises `xtask/tests/revendor.rs` and
the `xtask` unit tests as part of the default suite. Run `cargo xtask
revendor-check` (or `make revendor-check`) for all four trees --
`native-v1`, `complete-value`, `test-fixtures-architecture` and
`test-fixtures-modules` -- and confirm it reports zero drifted and zero
stray paths. Run `cargo xtask revendor` against a local
`quire-specification` clone (`--qspec-clone`) for the trees with a `Qspec`
source and against a local `filament-core-data` clone (`--fcd-clone`) for
the two `tests/fixtures/*` trees, and confirm each reports nothing written
when the manifests already match that clone's pinned commits.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-011-AC-1 | Every git-backed manifest source records a full 40-character commit and only tree-relative paths with no leading `/`, no `\` and no `.`/`..` component; a short sha, a ref, an absolute or escaping path, a missing local clone, or an unsupported command flag is refused before any byte is read or written. | Test (TC-149) |
| NFR-011-AC-2 | Vendored bytes equal the git blob at the pinned commit for every listed path, and an externally hosted file's bytes equal its recorded digest; a mismatch is refused rather than silently accepted or fetched. | Test (TC-150) |
| NFR-011-AC-3 | Running `revendor` a second time at an unchanged pin writes nothing, removes nothing, and leaves the manifest byte-identical to the first run's result. | Test (TC-151) |
| NFR-011-AC-4 | `revendor-check` fails, with no clone and no network access, when a vendored byte differs from its recorded digest, when a vendored file is missing, or when a file (including a symlink or other non-regular entry) exists under the tree outside the manifest and `README.md`; it runs as part of the default `cargo test`. | Test (TC-152) |
| NFR-011-AC-5 | Re-vendoring at a new pin replaces the tree wholesale: a file the manifest no longer lists is removed and reported, not left behind as an unexplained stray file. | Test (TC-152) |
