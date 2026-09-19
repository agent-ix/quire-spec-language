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

.PHONY: revendor revendor-check ci ci-default-features ci-all-features ci-clean-build

revendor:
	cargo xtask revendor --tree $(TREE) $(if $(QSPEC_CLONE),--qspec-clone $(QSPEC_CLONE))

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

# FR-059/FR-060/FR-061 (ADR-011 §7.1 T-12, #215): architecture-conformance
# checks over the QSL/IR/RT/CG ecosystem. Not part of `ci:` -- FR-059 and
# FR-060 report real, already-tracked findings against QSL's own current
# head (ADR-011 OBS-029, ADR-013 OBS-018), owned by #213/#211, not by this
# target's caller. `arch-lint-direction` needs real local checkouts of the
# three backend repositories; point IR_CLONE/RT_CLONE/CG_CLONE at them.
IR_CLONE ?=
RT_CLONE ?=
CG_CLONE ?=

.PHONY: arch-lint-direction arch-lint-api-surface arch-lint-duplicate-revisions arch-lint

arch-lint-direction:
	cargo run --locked --bin arch-lint -- direction \
		--qsl . --ir $(IR_CLONE) --rt $(RT_CLONE) --cg $(CG_CLONE)

arch-lint-api-surface:
	cargo run --locked --bin arch-lint -- api-surface --qsl .

arch-lint-duplicate-revisions:
	cargo run --locked --bin arch-lint -- duplicate-revisions --lockfile Cargo.lock

# Runs the two checks that need only this repository. `arch-lint-direction`
# needs IR_CLONE/RT_CLONE/CG_CLONE (see above) and is run separately.
arch-lint: arch-lint-api-surface arch-lint-duplicate-revisions

# FR-058 (ADR-011 §7.1 T-12, #215): the current-head integration lane. Not
# part of `ci:` -- it needs network access to fetch each repository's
# default branch head, and it is a separate lane from the exact-pin build
# `ci:` verifies. See integration/current-head/README.md.
.PHONY: integration-current-head-prepare integration-current-head integration-current-head-revision-log integration-current-head-incompatible-fixture

# Refreshes the local clones the lane's [patch] entries need. Run this first,
# and again any time quire-contract-ir's head should be picked up again.
integration-current-head-prepare:
	cargo run --manifest-path integration/current-head/tool/Cargo.toml -- \
		prepare --vendor-root integration/current-head/.vendor

integration-current-head:
	cargo test --manifest-path integration/current-head/Cargo.toml

integration-current-head-revision-log:
	cargo run --manifest-path integration/current-head/tool/Cargo.toml -- \
		revision-log --qsl . --manifest integration/current-head/Cargo.toml \
		--vendor-root integration/current-head/.vendor

integration-current-head-incompatible-fixture:
	cargo run --manifest-path integration/current-head/tool/Cargo.toml -- \
		check-incompatible-fixture --manifest integration/current-head/fixtures/incompatible/Cargo.toml
