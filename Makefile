.PHONY: check-no-committed-binaries check-index-completeness seam-probe string-edge ci ci-default-features ci-all-features ci-clean-build ci-docs

# QSL-169: fail when a tracked file is executable/binary content or exceeds
# the size ceiling. See the script's own header for the detection method and
# the ceiling's derivation.
check-no-committed-binaries:
	tools/check-no-committed-binaries.sh

# QSL-168: fail when an FR or TC artifact under spec/ has no row in the
# master index (spec/spec.md, spec/**/tests.md). Offline, no build required.
check-index-completeness:
	tools/check-index-completeness.sh

# QSL#214 (FR-063-AC-5): demonstrated on every full-gate run, not only when
# run by hand. Builds the QSL crate twice on its own (once under
# `RUSTFLAGS=--cfg seam_probe`, once without), independent of whatever
# feature set the caller's own `cargo build`/`clippy` steps used.
seam-probe:
	cargo xtask seam-probe

# QSL#214 (FR-064): scans the QSL crates' non-test source for a string
# comparison or string `match` outside a `#[string_edge]`-marked function.
# FR-064's own text requires this tool to run "in the lint gate", but the
# real, unmarked crate today has 67 pre-existing occurrences outside every
# file #214 touches (src/cli.rs, src/complete/, src/model/,
# src/protocol_artifact/, src/state/evaluation.rs, src/value/definition.rs,
# src/linking.rs, src/mapped.rs). PR #262 review, finding F5: that "none of
# them are reachable from src/family/*/src/value/expression/*" claim rested
# on the scanner's own known limits (literal-operand `ExprBinary` comparisons
# only -- no method-call forms like `starts_with`/`contains`, and no
# const-named operand), not on those two directories actually being clean;
# the same PR's own new code had two unmarked comparisons the scanner missed
# for exactly that reason, now fixed and (where genuine) marked
# `#[string_edge]`. Round 2 of the same review extended the method-call
# detection to `ends_with`/`strip_prefix`/`trim_start_matches` (the count
# moved 60 -> 67, all seven new findings outside src/family/*/src/value/
# expression/* -- one of the seven was inside xtask/* itself,
# xtask/src/cargo_pin.rs's `locked_rev`, now marked `#[string_edge]` for the
# same reason `seam_probe`'s `offline_registry_unavailable` already was: a
# real branch gate over hand-parsed `Cargo.lock` text, not a family-dispatch
# smell to launder). FR-064's allow-list mechanism cannot paper over the
# remaining 67 in src/* (it refuses any branch-gating entry, which is what
# almost all of them are), so making them clean is real conversion work
# belonging to the family this string selects, not to #214. `xtask
# string-edge` itself is complete and its own footprint (xtask/*) is clean
# under it (verified: 0 findings), so this target runs it standalone -- like
# `arch-lint` below, it is not part of `ci:` until the crate-wide marking
# sweep QSL-145 tracks lands. Remaining work: QSL-145.
string-edge:
	cargo xtask string-edge

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

# PLAT-856: `rustdoc::broken_intra_doc_links` and `missing_docs` are only
# enforced fully under a real `cargo doc` build -- clippy never runs
# rustdoc, so a doc build is its own gate, not a byproduct of ci-all-features.
ci-docs:
	RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps --all-features

ci: check-no-committed-binaries check-index-completeness ci-default-features ci-all-features ci-clean-build seam-probe ci-docs

# FR-059/FR-060/FR-061 (ADR-011 §7.1 T-12, #215): architecture-conformance
# checks over the QSL/IR/RT/CG ecosystem. Not part of `ci:` -- FR-059 and
# FR-060 report real, already-tracked findings against QSL's own current
# head (ADR-011 OBS-029, ADR-013 OBS-018), owned by #213/#211, not by this
# target's caller. `arch-lint-direction` needs real local checkouts of the
# three backend repositories; point IR_CLONE/RT_CLONE/CG_CLONE at them.
# `arch-lint-api-surface`'s T12-A rule is CG-side (#249 review, HIGH-2/
# MEDIUM-4) and needs CG_CLONE too, once `src/replay.rs` lands; until then it
# stays PENDING with no root given.
#
# `arch-lint` (this repo's own checks: api-surface, duplicate-revisions on
# QSL's own root lock, and duplicate-revisions on the current-head lane's own
# lock) exits 1 by design today: T12-B and T12-C's real, already-tracked
# findings above make `arch-lint-api-surface` fail, and QSL's own root
# Cargo.lock's deliberate double pin of the IR repository
# (`quire-contract-ir` vs. `quire-contract-model`, #249 review R2) makes
# `arch-lint-duplicate-revisions` fail. `arch-lint-duplicate-revisions-lane`
# passes (R3: the lane's own lock converges via its own [patch] table) and is
# included here so that convergence is routinely enforced, not merely
# checkable on request. Neither of the two failing checks is remediated by
# this target's caller. `arch-lint` joins `ci:` once #211/#213 remediate both.
# Remaining work: #211.
IR_CLONE ?=
RT_CLONE ?=
CG_CLONE ?=

.PHONY: arch-lint-direction arch-lint-api-surface arch-lint-duplicate-revisions arch-lint arch-lint-duplicate-revisions-lane

arch-lint-direction:
	cargo run --locked --bin arch-lint -- direction \
		--qsl . --ir $(IR_CLONE) --rt $(RT_CLONE) --cg $(CG_CLONE)

arch-lint-api-surface:
	cargo run --locked --bin arch-lint -- api-surface --qsl . $(if $(CG_CLONE),--cg $(CG_CLONE))

arch-lint-duplicate-revisions:
	cargo run --locked --bin arch-lint -- duplicate-revisions --lockfile Cargo.lock

# FR-061 (#249 review R3): the current-head lane's own Cargo.lock is in scope
# too -- it converges on one revision per ecosystem repository via the lane's
# own [patch] table (integration/current-head/Cargo.toml), independent of the
# root workspace's lock this target above checks.
arch-lint-duplicate-revisions-lane:
	cargo run --locked --bin arch-lint -- duplicate-revisions \
		--lockfile integration/current-head/Cargo.lock

# Runs the three checks that need only this repository (#249 review round 2
# L-2: `arch-lint-duplicate-revisions-lane` was previously checkable only on
# request, with no target routinely enforcing R3's convergence).
# `arch-lint-direction` needs IR_CLONE/RT_CLONE/CG_CLONE (see above) and is
# run separately.
arch-lint: arch-lint-api-surface arch-lint-duplicate-revisions arch-lint-duplicate-revisions-lane

# FR-058 (ADR-011 §7.1 T-12, #215): the current-head integration lane. Not
# part of `ci:` -- it needs network access to fetch each repository's
# default branch head, and it is a separate lane from the exact-pin build
# `ci:` verifies. See integration/current-head/README.md.
.PHONY: integration-current-head-prepare integration-current-head integration-current-head-revision-log integration-current-head-incompatible-fixture

# Refreshes the local clones the lane's [patch] entries need, then runs
# `cargo update` against the lane's own manifest so its committed Cargo.lock
# picks up each dependency's current head (#249 review, HIGH-1) -- this never
# touches the root workspace's Cargo.lock. Run this first, and again any time
# a dependency's head should be picked up again.
integration-current-head-prepare:
	cargo run --manifest-path integration/current-head/tool/Cargo.toml -- \
		prepare --deps-root integration/current-head/.deps \
		--manifest integration/current-head/Cargo.toml

# #249 review round 2 L-2: a test run must not silently execute against a
# local checkout that fell behind head -- previously only the separate
# `integration-current-head-revision-log` target caught that (HIGH-1's
# `require_current_head` guard), so a plain `make integration-current-head`
# could test a stale snapshot with no warning. `revision-log`'s freshness
# check now gates every test run too, and fails loudly (non-zero exit) before
# `cargo test` runs at all if any local clone or CG's resolved head is
# stale.
integration-current-head: integration-current-head-revision-log
	cargo test --manifest-path integration/current-head/Cargo.toml

integration-current-head-revision-log:
	cargo run --manifest-path integration/current-head/tool/Cargo.toml -- \
		revision-log --qsl . --manifest integration/current-head/Cargo.toml \
		--deps-root integration/current-head/.deps

integration-current-head-incompatible-fixture:
	cargo run --manifest-path integration/current-head/tool/Cargo.toml -- \
		check-incompatible-fixture --manifest integration/current-head/fixtures/incompatible/Cargo.toml
