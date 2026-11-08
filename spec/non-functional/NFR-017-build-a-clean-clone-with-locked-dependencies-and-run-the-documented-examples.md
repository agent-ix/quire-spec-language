---
id: NFR-017
org: agent-ix
title: "Build a clean clone with locked dependencies and run the documented examples"
type: NFR
quality_attribute: portability
relationships:
  - target: ix://agent-ix/quire-spec-language/US-037
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/NFR-013
    type: references
---
# NFR-017: Build a clean clone with locked dependencies and run the documented examples

## Statement

When the `make clean-clone-check` gate runs at a commit, the gate shall
clone that commit into a new directory, build it with
`cargo build --locked --workspace` under an empty `CARGO_HOME`, run the
examples the README documents, and fail if any step fails or needs a
sibling checkout, a local patch or a repository that cannot be cloned
anonymously.

## Scope

The README's Usage section promises an outside user that a clone builds with
only a Rust toolchain at the `rust-version` in `Cargo.toml`. This gate keeps
that promise at every commit it runs on.

### The gate

`make clean-clone-check` (default `REV=HEAD`):

1. clones the repository with `git clone --no-local` into a new temporary
   directory and checks out `REV`;
2. sets `CARGO_HOME` and `CARGO_TARGET_DIR` to new, empty temporary
   directories, so no registry cache, git checkout or build artifact of the
   host is reused (NFR-013);
3. runs `cargo build --locked --workspace` in the clone;
4. runs the README's example commands in the clone:
   `cargo run --locked --example config_version_fixtures -- <tmp>/config-version`;
   `examples/config-version/requests.sh <tmp>/config-version/healthy-parent ParentOrder`;
   `cargo run --locked --example emit_checked_v2 -- --package-name switch --model examples/switch-transition/switch.semantic-ir.json examples/switch-transition/switch.native`,
   writing the package to `<tmp>/package.json`;
   and `cargo test --locked --test it walkthrough_requests_reproduce_the_documented_exit_codes`;
5. removes every temporary directory it made, whether it passes or fails.

The dependency graph `cargo metadata --locked` reports in the clone may name
only packages in the clone, crates.io, and git repositories under
`https://github.com/agent-ix/` that `git ls-remote` reads with no
credentials.

### Out of scope

`make ci`'s conformance and qualification targets, which read a sibling
QSpec checkout, are maintainer gates, not part of an outside user's build.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Failed steps of `make clean-clone-check` at the head of `main` | 0 | 0 | Test |
| Package sources outside the clone, crates.io and anonymously readable `agent-ix` repositories | 0 | 0 | Test |
| Temporary directories left after a gate run | 0 | 0 | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-017-AC-1 | `make clean-clone-check` passes at the head of `main`: the locked build exits 0, and every package source `cargo metadata --locked` reports is the clone, crates.io, or an `https://github.com/agent-ix/` git repository that `git ls-remote` reads with `GIT_TERMINAL_PROMPT=0` and no credential helper. | Test (TC-1397) |
| NFR-017-AC-2 | In the gate's clone, `config_version_fixtures` writes the case directories `healthy-parent`, `violating-parent`, `dangling-parent`, `incomplete-population` and `cycle`; `requests.sh` writes `run.json` and `compile.json` in `healthy-parent`; `emit_checked_v2` exits 0 and writes package bytes that QSL's I2 reader reads back with the switch model; and the walkthrough test passes with the README's exit codes 0, 10, 20, 22 and 10. | Test (TC-1398) |
| NFR-017-AC-3 | The gate run on a scratch commit whose `Cargo.lock` lacks a dependency `Cargo.toml` declares fails at the build step with Cargo's `--locked` refusal; on a scratch commit whose `Cargo.lock` names a `path` source outside the repository, it fails naming that source; on a scratch commit that removes `examples/config-version/requests.sh`, it fails at step 4. | Test (TC-1397) |
| NFR-017-AC-4 | After a passing run and after each AC-3 failing run, none of the gate's temporary directories remains. | Test (TC-1397) |

## Verification

TC-1397 runs the gate and its failure cases; TC-1398 checks the examples'
outputs in the gate's clone. Rust tests carry `#[trace("TC-...",
"NFR-017-AC-...")]` attributes (NFR-005).

## Dependencies

- README Usage: the documented commands.
- [NFR-013](NFR-013-isolate-native-ci-feature-artifacts.md): a gate's own artifact directory.

## References

- QSL-12: the Layer 5 outside-user path.
