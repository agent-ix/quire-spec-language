.PHONY: check-no-committed-binaries check-index-completeness seam-probe string-edge route-lint cargo-deny-bans ci ci-default-features ci-all-features ci-clean-build ci-docs conformance

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
# run by hand. Runs three normal builds (the root crate, qsl-route and
# qsl-eval) and five probe builds (qsl-semantics, the root crate, qsl-route,
# qsl-eval and qsl-replay under `RUSTFLAGS=--cfg seam_probe`) on their own, independent
# of whatever feature set the caller's own `cargo build`/`clippy` steps used.
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

# QSL-46 (FR-080-AC-3): scans the #185 registry crate (every file under
# qsl-route/src) for a static, OnceLock or thread_local! item -- ADR-012
# §5.3's registry evidence requires the registry stay an ordinary value,
# never ambient state.
route-lint:
	cargo xtask route-lint

# QSL-46 (FR-080-AC-2): denies the inventory/linkme/ctor crates outright
# (deny.toml), so a future contributor cannot repopulate the registry through
# a link-time/plugin-discovery mechanism instead of the ordinary value FR-075
# requires. Scoped to `check bans` -- deny.toml configures no license or
# advisory policy.
#
# Install: `cargo install cargo-deny --locked --version 0.19.8` (the same
# pinned version `.github/workflows/ci.yml` installs). This target FAILS
# when `cargo-deny` is not on PATH, with that exact install command --
# `.github/workflows/ci.yml` is `workflow_dispatch`-only, so `make ci` is the
# gate that actually runs in practice, and a machine without cargo-deny must
# not be able to report a green `make ci` over a tree that depends on one of
# the three banned registry-discovery crates (PR #305 review round 2,
# finding 4). There is no `SKIP_CARGO_DENY`-style opt-out.
#
# The `cargo-deny`-backed trigger test in `qsl-route/tests/it/route_registry.rs`
# (`cargo_deny_bans_the_three_registry_crates`) still skips, rather than
# fails, when the binary is absent: it is a network/tool test exercising the
# same binary this target already requires, so this target failing first is
# what actually enforces the check locally.
cargo-deny-bans:
	@if command -v cargo-deny >/dev/null 2>&1; then \
		cargo deny check bans --config deny.toml; \
	else \
		echo "cargo-deny-bans: cargo-deny is not installed; run \`cargo install cargo-deny --locked --version 0.19.8\` to install it" >&2; \
		exit 1; \
	fi

# QSL #154: default-feature build of `--all-targets` (including `tests/`) is
# its own gate, separate from the `--all-features` one below.
#
# A workspace-wide build unifies features across every package, dev-
# dependencies included. `quire-spec-language`'s dev-dependency turns on
# `qsl-semantics/test-support`, `qsl-forms`'s turns on
# `qsl-cst/test-support`, and several crates' turn on
# `quire-exact/test-support` (QSL-206's charge log), so `--workspace` builds
# those three crates with `test-support` on even here. The `-p` runs below
# build each crate alone, with the feature off: they lint the
# `not(feature = "test-support")` code paths under `-D warnings`, and check
# that the modules of each crate's own `tests/it` not gated on the feature
# compile and pass without it. `-p quire-exact` is where the production
# meter's no-allocation test runs. These are the only three workspace crates
# with a `test-support` feature.
ci-default-features:
	cargo fmt --all -- --check
	cargo clippy --locked --workspace --all-targets -- -D warnings
	cargo test --locked --workspace
	cargo clippy --locked -p qsl-semantics --all-targets -- -D warnings
	cargo test --locked -p qsl-semantics
	cargo clippy --locked -p qsl-cst --all-targets -- -D warnings
	cargo test --locked -p qsl-cst
	cargo clippy --locked -p quire-exact --all-targets -- -D warnings
	cargo test --locked -p quire-exact

ci-all-features:
	cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
	cargo test --locked --workspace --all-features

# The same three no-default-features checks `.github/workflows/ci.yml` runs
# after its own clippy/test steps: a from-clean build (its own target-dir, so
# it never reuses this build's cached artifacts), the fixture-audit negative
# controls and the parse example.
#
# QSL-251 (FR-042-AC-15/FR-050-AC-8): a `--lib`, `--no-default-features`
# check with only `handoff-writer` turned on demonstrates the acceptance
# itself -- the public writer builds with no dev-dependency in its closure --
# rather than resting on a claim in prose. `cargo check` (not `build`) is
# enough: the property under test is which dependencies the lib target
# resolves, not that its object code is produced.
ci-clean-build:
	cargo build --locked --workspace --no-default-features --target-dir target/clean
	cargo check --locked -p quire-spec-language --lib --no-default-features --features handoff-writer --target-dir target/clean
	cargo run --locked --no-default-features --bin fixture-audit -- self-test
	cargo run --locked --no-default-features -- parse agent-ix test:parent fixture fixture:1 tests/fixtures/parent.native

# PLAT-856: `rustdoc::broken_intra_doc_links` and `missing_docs` are only
# enforced fully under a real `cargo doc` build -- clippy never runs
# rustdoc, so a doc build is its own gate, not a byproduct of ci-all-features.
ci-docs:
	RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps --all-features

ci: check-no-committed-binaries check-index-completeness ci-default-features ci-all-features ci-clean-build seam-probe route-lint cargo-deny-bans ci-docs arch-lint-canonical-encoder

# QSL-156 A4a: the FR-322 application-node key checked against QSpec's
# published `operation_vectors`, read at run time from the
# quire-specification checkout named by QSPEC_DIR. Opt-in while QSpec is not
# public; nothing of QSpec is copied into this repository. Without QSPEC_DIR
# the test itself skips, so this target refuses to run instead, and it fails
# when the test did not actually check the vectors (a renamed test filters to
# zero tests and would otherwise pass). Both tests are `qsl-semantics`'
# (QSL-181 moved them there; QSL-156 A4b moved the key into `check::node_key`).
CONFORMANCE_TEST := check::node_key::tests::conformance_fr322_application_keys_match_qspec_operation_vectors
# FR-092-AC-8 (TC-413 step 6): the nominal enum declaration and member keys
# against QSpec's `enum-status` and `enum-status-ready` vectors.
CONFORMANCE_ENUM_TEST := check::node_key::tests::conformance_fr092_nominal_enum_keys_match_qspec_vectors
# TC-411 step 3 (FR-088-AC-12): the compound-unit `UnitId`s against QSpec's
# `value-compound-unit-vectors.json`, guarded the same way.
CONFORMANCE_UNIT_TEST := quantities::tc_411_compound_unit_ids_match_qspec_vectors
# ADR-013 C-14 (TC-421, QSL-159): every source-map entry of QSpec's positive
# v2 fixtures looks up to its wire regions, guarded the same way.
CONFORMANCE_SOURCE_MAP_TEST := checked_v2::tests::conformance_c14_source_map_lookup_over_qspec_positive_fixtures
# QSL-232: QSL's full I2 read over QSpec's checked-package-v2 fixtures --
# every positive fixture admitted, every adverse mutation refused by cause.
CONFORMANCE_I2_TEST := checked_v2::tests::conformance_i2_read_over_qspec_checked_package_v2_fixtures
# QSL-255: QSL's I2 read over QSpec's `dependency-selection-vectors.json`.
CONFORMANCE_DEPENDENCY_TEST := checked_v2::tests::conformance_dependency_selection_vectors
conformance:
	@if [ -z "$(QSPEC_DIR)" ]; then \
		echo "conformance: set QSPEC_DIR to a quire-specification checkout" >&2; \
		exit 1; \
	fi
	@out=$$(QSPEC_DIR="$(QSPEC_DIR)" cargo test --locked -p qsl-semantics --lib -- --exact $(CONFORMANCE_TEST) --nocapture 2>&1); \
	status=$$?; \
	echo "$$out"; \
	if [ $$status -ne 0 ]; then exit $$status; fi; \
	echo "$$out" | grep -q '^conformance: ' || { echo "conformance: the vector check did not run" >&2; exit 1; }
	@out=$$(QSPEC_DIR="$(QSPEC_DIR)" cargo test --locked -p qsl-semantics --lib -- --exact $(CONFORMANCE_ENUM_TEST) --nocapture 2>&1); \
	status=$$?; \
	echo "$$out"; \
	if [ $$status -ne 0 ]; then exit $$status; fi; \
	echo "$$out" | grep -q '^conformance: 2 nominal enum vectors' || { echo "conformance: the nominal enum vector check did not run" >&2; exit 1; }
	@out=$$(QSPEC_DIR="$(QSPEC_DIR)" cargo test --locked -p qsl-semantics --test it -- --exact $(CONFORMANCE_UNIT_TEST) --nocapture 2>&1); \
	status=$$?; \
	echo "$$out"; \
	if [ $$status -ne 0 ]; then exit $$status; fi; \
	echo "$$out" | grep -q '^conformance: [1-9][0-9]* compound-unit vectors$$' || { echo "conformance: the compound-unit vector check did not run" >&2; exit 1; }
	@out=$$(QSPEC_DIR="$(QSPEC_DIR)" cargo test --locked -p qsl-package --lib -- --exact $(CONFORMANCE_SOURCE_MAP_TEST) --nocapture 2>&1); \
	status=$$?; \
	echo "$$out"; \
	if [ $$status -ne 0 ]; then exit $$status; fi; \
	echo "$$out" | grep -q '^conformance: [1-9][0-9]* source-map entries over [1-9][0-9]* positive fixtures$$' || { echo "conformance: the source-map lookup check did not run" >&2; exit 1; }
	@out=$$(QSPEC_DIR="$(QSPEC_DIR)" cargo test --locked -p qsl-package --lib -- --exact $(CONFORMANCE_I2_TEST) --nocapture 2>&1); \
	status=$$?; \
	echo "$$out"; \
	if [ $$status -ne 0 ]; then exit $$status; fi; \
	echo "$$out" | grep -q '^conformance: [1-9][0-9]* positive fixtures through QSL.s full I2 read' || { echo "conformance: the I2 positive-fixture check did not run" >&2; exit 1; }; \
	echo "$$out" | grep -q '^conformance: [1-9][0-9]* adverse mutations refused' || { echo "conformance: the I2 adverse-mutation check did not run" >&2; exit 1; }
	@out=$$(QSPEC_DIR="$(QSPEC_DIR)" cargo test --locked -p qsl-package --lib -- --exact $(CONFORMANCE_DEPENDENCY_TEST) --nocapture 2>&1); \
	status=$$?; \
	echo "$$out"; \
	if [ $$status -ne 0 ]; then exit $$status; fi; \
	echo "$$out" | grep -q '^conformance: [1-9][0-9]* dependency-selection entry mutations and [1-9][0-9]* order vectors$$' || { echo "conformance: the dependency-selection vector check did not run" >&2; exit 1; }

# FR-059/FR-060/FR-061 (ADR-011 §7.1 T-12, #215): architecture-conformance
# checks over the QSL/IR/RT/CG ecosystem. Not part of `ci:` -- FR-059
# reports real, already-tracked findings against QSL's own current head
# (ADR-011 OBS-029), owned by #213, not by this target's caller.
# `arch-lint-direction` needs real local checkouts of the three backend
# repositories; point IR_CLONE/RT_CLONE/CG_CLONE at them.
#
# FR-060 T12-B/T12-C: `NodeKey`'s and `EffectiveId`'s mints outside
# `check`/`model` (ADR-013 OBS-018 and FB-13) are a named, shrinking debt
# list, reported as debt, not failures.
# `arch-lint-api-surface`'s T12-A rule is CG-side (#249 review, HIGH-2/
# MEDIUM-4): it scans CG_CLONE for calls into the `qsl-replay` facade crate.
# Without CG_CLONE, T12-A reports NOT EVALUATED, T12-B, T12-C and T12-D still
# run and report, and the target exits 2 (usage) for the missing input.
#
# `arch-lint` (this repo's own checks: api-surface, duplicate-revisions on
# QSL's own root lock, and duplicate-revisions on the current-head lane's own
# lock) exits non-zero today: QSL's own root Cargo.lock's deliberate double
# pin of the IR repository (`quire-contract-ir` vs. `quire-contract-model`,
# #249 review R2) makes `arch-lint-duplicate-revisions` fail, and
# `arch-lint-api-surface` exits 2 without a `CG_CLONE` checkout, per T12-A
# above. `arch-lint-duplicate-revisions-lane`
# passes (R3: the lane's own lock converges via its own [patch] table) and is
# included here so that convergence is routinely enforced, not merely
# checkable on request. `arch-lint` joins `ci:` once #211 remediates the
# lockfile pin and a `CG_CLONE` checkout is routinely available.
IR_CLONE ?=
RT_CLONE ?=
CG_CLONE ?=

.PHONY: arch-lint-direction arch-lint-api-surface arch-lint-duplicate-revisions arch-lint arch-lint-duplicate-revisions-lane arch-lint-canonical-encoder

arch-lint-direction:
	cargo run --locked -p arch-lint -- direction \
		--qsl . --ir $(IR_CLONE) --rt $(RT_CLONE) --cg $(CG_CLONE)

arch-lint-api-surface:
	cargo run --locked -p arch-lint -- api-surface --qsl . $(if $(CG_CLONE),--cg $(CG_CLONE))

arch-lint-duplicate-revisions:
	cargo run --locked -p arch-lint -- duplicate-revisions --lockfile Cargo.lock

# FR-061 (#249 review R3): the current-head lane's own Cargo.lock is in scope
# too -- it converges on one revision per ecosystem repository via the lane's
# own [patch] table (integration/current-head/Cargo.toml), independent of the
# root workspace's lock this target above checks.
arch-lint-duplicate-revisions-lane:
	cargo run --locked -p arch-lint -- duplicate-revisions \
		--lockfile integration/current-head/Cargo.lock

# ADR-013 §2 (ADR-013:113, QSL-194): no second canonical encoder beside
# `quire-canonical` -- a shipped file pairing a `serde_json` serializer with
# a hash fails, named files excepted with their reason. Needs only this
# repository and passes on it, so unlike the checks above it is part of `ci:`.
arch-lint-canonical-encoder:
	cargo run --locked -p arch-lint -- canonical-encoder --qsl .

# Runs the four checks that need only this repository (#249 review round 2
# L-2: `arch-lint-duplicate-revisions-lane` was previously checkable only on
# request, with no target routinely enforcing R3's convergence).
# `arch-lint-direction` needs IR_CLONE/RT_CLONE/CG_CLONE (see above) and is
# run separately.
arch-lint: arch-lint-api-surface arch-lint-duplicate-revisions arch-lint-duplicate-revisions-lane arch-lint-canonical-encoder

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

# QSL-197: the whole 30,000-input parser differential against
# tests/fixtures/parser-differential/baseline.txt. `make ci` runs the first
# 1,000 inputs of each family; this runs all of them.
.PHONY: test-differential
test-differential:
	cargo test --locked --test it parser_differential -- --include-ignored

# QSL-196: the committed performance benchmarks (`qsl-bench/`), one
# criterion bench per axis, each runnable by name. `make bench` runs all
# six; `make bench-probe` prints the counts, refusal boundaries and one-shot
# large-input timings (with peak RSS) the criterion benches do not record.
# The recorded baseline, with its variance, is `qsl-bench/BASELINE.md`.
# Pass criterion options through BENCH_ARGS, e.g.
# `make bench-model BENCH_ARGS='--save-baseline before'`. Not part of `ci:`
# -- timing is not a pass/fail gate.
BENCH_ARGS ?=
BENCH_AXES := parser checker cst model evaluator text_cluster

.PHONY: bench bench-probe $(addprefix bench-,$(BENCH_AXES))

bench: $(addprefix bench-,$(BENCH_AXES))

$(addprefix bench-,$(BENCH_AXES)): bench-%:
	cargo bench --locked -p qsl-bench --bench $* -- $(BENCH_ARGS)

# One process per `check`/`eval` size: peak RSS is process-wide.
BENCH_PROBE := cargo run --locked --release -q -p qsl-bench --bin qsl-bench-probe --
bench-probe:
	$(BENCH_PROBE) parse
	$(BENCH_PROBE) cst
	for n in 250 1000 2000 4000 8000; do $(BENCH_PROBE) check chain $$n || exit 1; done
	for n in 1000 5000; do $(BENCH_PROBE) check independent $$n || exit 1; done
	for n in 3 4 5 6 7 8 9 10 11 12; do $(BENCH_PROBE) check text-cluster $$n || exit 1; done
	for b in 1 64 256; do $(BENCH_PROBE) check deep-wide $$b || exit 1; done
	for n in 1 1000; do $(BENCH_PROBE) eval $$n || exit 1; done
	$(BENCH_PROBE) model 4000 4 100
