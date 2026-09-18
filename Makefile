# QSL #138: re-vendor resources/native-v1 and resources/complete-value from
# an explicit pinned commit recorded in resources/<tree>/VENDOR.json.
#
# `revendor` writes bytes; it needs QSPEC_CLONE only when the target tree's
# manifest actually contains a `qspec`-kind source (both do today). It never
# resolves "latest" and never fetches over the network -- QSPEC_CLONE must
# already contain the pinned commit.
#
# `revendor-check` is offline: it verifies the vendored trees against the
# manifests' own recorded digests and flags any file the manifest does not
# mention. No clone is required; this is also what `cargo test --workspace`
# runs on every build.

TREE ?= all
QSPEC_CLONE ?=

.PHONY: revendor revendor-check ci ci-default-features ci-all-features

revendor:
	cargo xtask revendor --tree $(TREE) $(if $(QSPEC_CLONE),--qspec-clone $(QSPEC_CLONE))

revendor-check:
	cargo xtask revendor-check --tree $(TREE)

# QSL #154: default-feature build of `--all-targets` (including `tests/`) is
# its own gate, separate from the `--all-features` one below. `test-support`
# fixture constructors are reachable only under `--all-features`
# (Cargo.toml `[[test]] required-features`), so a default-feature build must
# also be checked or a fixture-only test target can silently stop compiling
# under the feature set every non-Quire caller actually builds with.
ci-default-features:
	cargo fmt --all -- --check
	cargo clippy --locked --workspace --all-targets -- -D warnings
	cargo test --locked --workspace

ci-all-features:
	cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
	cargo test --locked --workspace --all-features

ci: ci-default-features ci-all-features
