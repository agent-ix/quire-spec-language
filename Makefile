# QSL #138: re-vendor resources/native-v1 and resources/complete-value from
# an explicit pinned commit recorded in resources/<tree>/VENDOR.json.
# QSL #131 PR 3 adds tests/fixtures/architecture and tests/fixtures/modules,
# vendored from agent-ix/filament-core-data at the same commit Cargo.toml
# pins its agent-ix-extraction-frontend/agent-ix-semantic-ir git deps to.
#
# `revendor` writes bytes; it needs QSPEC_CLONE/FCD_CLONE only when the
# target tree's manifest actually contains a `qspec`/`fcd`-kind source. It
# never resolves "latest" and never fetches over the network -- the clone
# must already contain the pinned commit.
#
# `revendor-check` is offline: it verifies the vendored trees against the
# manifests' own recorded digests and flags any file the manifest does not
# mention. No clone is required; this is also what `cargo test --workspace`
# runs on every build.

TREE ?= all
QSPEC_CLONE ?=
FCD_CLONE ?=

.PHONY: revendor revendor-check ci ci-default-features ci-all-features ci-clean-build

revendor:
	cargo xtask revendor --tree $(TREE) $(if $(QSPEC_CLONE),--qspec-clone $(QSPEC_CLONE)) $(if $(FCD_CLONE),--fcd-clone $(FCD_CLONE))

revendor-check:
	cargo xtask revendor-check --tree $(TREE)

# QSL #154: default-feature build of `--all-targets` (including `tests/`) is
# its own gate, separate from the `--all-features` one below. `test-support`
# fixture constructors are reachable with `--features test-support` (Cargo.toml
# `[[test]] required-features`), not only under `--all-features`, so a
# default-feature build must also be checked or a fixture-only test target
# can silently stop compiling under the feature set every non-Quire caller
# actually builds with.
ci-default-features:
	cargo fmt --all -- --check
	cargo clippy --locked --workspace --all-targets -- -D warnings
	cargo test --locked --workspace

ci-all-features:
	cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
	cargo test --locked --workspace --all-features

# The same three no-default-features checks `.github/workflows/ci.yml` runs
# after its own clippy/test steps: a from-clean build (its own target-dir, so
# it never reuses this build's cached artifacts), the fixture-audit negative
# controls and the parse example.
ci-clean-build:
	cargo build --locked --workspace --no-default-features --target-dir target/clean
	cargo run --locked --no-default-features --bin fixture-audit -- self-test
	cargo run --locked --no-default-features -- parse test:parent fixture:1 tests/fixtures/parent.native

ci: ci-default-features ci-all-features ci-clean-build
