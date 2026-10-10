.PHONY: check-no-committed-binaries check-index-completeness seam-probe string-edge route-lint checked-input cargo-deny-bans fuzz-deep-input ci ci-default-features ci-all-features ci-clean-build ci-docs conformance

# Fail when a tracked file is executable/binary content or exceeds
# the size ceiling. See the script's own header for the detection method and
# the ceiling's derivation.
check-no-committed-binaries:
	tools/check-no-committed-binaries.sh

# Fail when an FR or TC artifact under spec/ has no row in the
# master index (spec/spec.md, spec/**/tests.md). Offline, no build required.
check-index-completeness:
	tools/check-index-completeness.sh

# QSL#214 (FR-063-AC-5): demonstrated on every full-gate run, not only when
# run by hand. Runs three normal builds (the root crate, qsl-route and
# qsl-eval) and five probe builds (qsl-semantics, the root crate, qsl-route,
# qsl-eval and qsl-replay under `RUSTFLAGS=--cfg seam_probe`) on their own, independent
# of whatever feature set the caller's own `cargo build`/`clippy` steps used.
seam-probe:
	cargo run --package xtask -- seam-probe

# QSL#214 (FR-064): scans the QSL crates' non-test source for a
# string comparison or string `match` outside a `#[string_edge]`-marked
# function. Part of `ci:` -- the lint gate FR-064 requires it to run in.
string-edge:
	cargo run --package xtask -- string-edge

# FR-080-AC-3: scans the #185 registry crate (every file under
# qsl-route/src) for a static, OnceLock or thread_local! item -- ADR-012
# §5.3's registry evidence requires the registry stay an ordinary value,
# never ambient state.
route-lint:
	cargo run --package xtask -- route-lint

# FR-270 (ADR-032 CK-1 to CK-5): fails when an execution, routing or replay
# stage entry takes a pre-check representation, or a stage crate calls a
# pre-check stage function.
checked-input:
	cargo run --package xtask -- checked-input

# FR-080-AC-2: denies the inventory/linkme/ctor crates outright
# (deny.toml), so a future contributor cannot repopulate the registry through
# a link-time/plugin-discovery mechanism instead of the ordinary value FR-075
# requires. ADR-013 §2: deny.toml's `quire-canonical` entry denies a second
# copy of QSL's one RFC 8785 implementation. The recipe runs with
# `--workspace`, so both checks cover every workspace crate, not only the
# root package's graph. Scoped to `check bans` -- deny.toml configures no
# license or advisory policy.
#
# Install: `cargo install cargo-deny --locked`. This target FAILS when
# `cargo-deny` is not on PATH, with that install command --
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
		cargo deny --workspace check bans --config deny.toml; \
	else \
		echo "cargo-deny-bans: cargo-deny is not installed; run \`cargo install cargo-deny --locked\` to install it" >&2; \
		exit 1; \
	fi

# =============================================================================
# Local development against sibling checkouts (one-copy-deps, agent-ix
# org-wide: IR #225, RT #88, CG #196).
#
# `use-local` writes a gitignored .cargo/config.toml that patches each
# first-party git dependency to its working tree at $(SIBLINGS)/<repo>,
# uncommitted edits included. It validates every entry before writing
# anything, so a bad entry leaves an existing config untouched. It first
# snapshots Cargo.lock to the gitignored .cargo/Cargo.lock.pre-local (only if
# no snapshot exists); `use-remote` deletes the config and restores the lock
# from that snapshot (and does nothing to the lock if there is none). So the
# lock returns to its state before the first `use-local`; lock changes made
# while a patch is active are discarded, and a `cargo update -p` made without
# a patch is kept.
# Format: <repo>:<crate>:<crate-dir>; entries are grouped by repo here, in
# any order, so each repo gets exactly one [patch] table.
# SIBLINGS is the directory holding the sibling clones: the parent of the main
# checkout, so it is also right from a linked worktree. Override to relocate.
#
# Note: this repo's own `ci:`/`ci-*` targets below still pass `--locked`
# unconditionally -- `make use-local` and `make ci` together will fail on the
# lockfile mismatch a local patch introduces. Run confined `cargo build`/
# `cargo test` (no `--locked`) under `use-local`.
# =============================================================================

SIBLINGS ?= $(abspath $(shell git rev-parse --path-format=absolute --git-common-dir)/../..)
LOCAL_PATCHES ?= quire-contract-ir:quire-contract-model:crates/quire-contract-model \
	quire-canonical:quire-canonical:. \
	ix-trace-rs:ix-trace-rs:.

.PHONY: use-local
use-local:
	@set -e; mkdir -p .cargo; \
	for spec in $(LOCAL_PATCHES); do \
	  if [ "$$(printf '%s' "$$spec" | tr -cd ':' | wc -c)" != 2 ] || printf '%s' "$$spec" | grep -q '::\|^:\|:$$'; then \
	    echo "use-local: malformed LOCAL_PATCHES entry '$$spec' (want repo:crate:dir)" >&2; exit 1; \
	  fi; \
	  repo=$${spec%%:*}; rest=$${spec#*:}; dir=$${rest#*:}; \
	  if [ ! -f "$(SIBLINGS)/$$repo/$$dir/Cargo.toml" ]; then \
	    echo "use-local: $(SIBLINGS)/$$repo is not cloned (no Cargo.toml at $(SIBLINGS)/$$repo/$$dir); clone agent-ix/$$repo next to this repo" >&2; exit 1; \
	  fi; \
	done; \
	[ -f .cargo/Cargo.lock.pre-local ] || cp Cargo.lock .cargo/Cargo.lock.pre-local; \
	: > .cargo/config.toml; \
	repos=$$(for spec in $(LOCAL_PATCHES); do printf '%s\n' "$${spec%%:*}"; done | awk '!seen[$$0]++'); \
	first=1; \
	for repo in $$repos; do \
	  [ "$$first" = 1 ] || printf '\n' >> .cargo/config.toml; first=0; \
	  printf '[patch."https://github.com/agent-ix/%s"]\n' "$$repo" >> .cargo/config.toml; \
	  for spec in $(LOCAL_PATCHES); do \
	    [ "$${spec%%:*}" = "$$repo" ] || continue; \
	    rest=$${spec#*:}; crate=$${rest%%:*}; dir=$${rest#*:}; \
	    printf '%s = { path = "%s/%s/%s" }\n' "$$crate" "$(SIBLINGS)" "$$repo" "$$dir" >> .cargo/config.toml; \
	  done; \
	done; echo "wrote .cargo/config.toml"; \
	meta=$$(mktemp); \
	if ! cargo metadata --format-version 1 >/dev/null 2>"$$meta"; then \
	  cat "$$meta" >&2; rm -f "$$meta" .cargo/config.toml; \
	  [ ! -f .cargo/Cargo.lock.pre-local ] || { cp .cargo/Cargo.lock.pre-local Cargo.lock; rm -f .cargo/Cargo.lock.pre-local; }; \
	  echo "use-local: cargo metadata failed under the patch" >&2; exit 1; \
	fi; \
	if grep -q 'patch .* was not used' "$$meta"; then \
	  cat "$$meta" >&2; rm -f "$$meta" .cargo/config.toml; \
	  [ ! -f .cargo/Cargo.lock.pre-local ] || { cp .cargo/Cargo.lock.pre-local Cargo.lock; rm -f .cargo/Cargo.lock.pre-local; }; \
	  echo "use-local: a patch was not used; the sibling's version does not satisfy the requirement" >&2; exit 1; \
	fi; \
	rm -f "$$meta"

.PHONY: use-remote
use-remote:
	rm -f .cargo/config.toml
	@if [ -f .cargo/Cargo.lock.pre-local ]; then mv .cargo/Cargo.lock.pre-local Cargo.lock; echo "restored Cargo.lock from the pre-local snapshot"; fi

# QSL #154: default-feature build of `--all-targets` (including `tests/`) is
# its own gate, separate from the `--all-features` one below.
#
# A workspace-wide build unifies features across every package, dev-
# dependencies included. `quire-spec-language`'s dev-dependency turns on
# `qsl-semantics/test-support`, `qsl-forms`'s turns on
# `qsl-cst/test-support`, and several crates' turn on
# `quire-exact/test-support` (the charge log, from the `quire-exact`
# repository), so `--workspace` builds those crates with `test-support` on
# even here. The `-p` runs below build each crate alone, with the feature
# off: they lint the `not(feature = "test-support")` code paths under
# `-D warnings`, and check that the modules of each crate's own `tests/it`
# not gated on the feature compile and pass without it. These are the only
# two workspace crates with a `test-support` feature; `quire-exact`'s own
# default-feature gate runs in its repository.
ci-default-features:
	cargo fmt --all -- --check
	cargo clippy --locked --workspace --all-targets -- -D warnings
	cargo test --locked --workspace
	cargo clippy --locked -p qsl-semantics --all-targets -- -D warnings
	cargo test --locked -p qsl-semantics
	cargo clippy --locked -p qsl-cst --all-targets -- -D warnings
	cargo test --locked -p qsl-cst

ci-all-features:
	cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
	cargo test --locked --workspace --all-features

# The no-default-features checks: a from-clean build (its own target-dir, so
# it never reuses this build's cached artifacts) and the parse example.
#
# FR-042-AC-15/FR-050-AC-8: a `--lib`, `--no-default-features`
# check with only `handoff-writer` turned on demonstrates the acceptance
# itself -- the public writer builds with no dev-dependency in its closure --
# rather than resting on a claim in prose. `cargo check` (not `build`) is
# enough: the property under test is which dependencies the lib target
# resolves, not that its object code is produced.
# Export the literal caller value without evaluating embedded Make expressions.
# The shell expands this environment value as data, not as recipe source.
override export CARGO_TARGET_DIR := $(value CARGO_TARGET_DIR)

ci-clean-build:
	cargo build --locked --workspace --no-default-features --target-dir "$${CARGO_TARGET_DIR:-target}/clean"
	cargo check --locked -p quire-spec-language --lib --no-default-features --features handoff-writer --target-dir "$${CARGO_TARGET_DIR:-target}/clean"
	cargo run --locked --no-default-features -- parse agent-ix test:parent fixture fixture:1 tests/fixtures/parent.native

# PLAT-856: `rustdoc::broken_intra_doc_links` and `missing_docs` are only
# enforced fully under a real `cargo doc` build -- clippy never runs
# rustdoc, so a doc build is its own gate, not a byproduct of ci-all-features.
ci-docs:
	RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps --all-features

# The deep-input fuzz target (FR-356-AC-7, TC-903) over the S1 parser and
# the S3 checker: 10,000 generated sources nested 1 to 100,000 levels deep.
# Depths are spread over orders of magnitude (`qsl_bench::deep_input`).
# It resolves against the root `Cargo.lock`, copied in (fuzz/Cargo.lock is
# gitignored), and builds on the pinned stable toolchain with no sanitizer:
# QSL's crates forbid `unsafe`, so the run checks panics, aborts
# and stack overflows. Needs `cargo install cargo-fuzz`. Not part of `ci:`.
FUZZ_RUNS ?= 10000
fuzz-deep-input:
	cp Cargo.lock fuzz/Cargo.lock
	cd fuzz && cargo fuzz run -s none deep_input -- -runs=$(FUZZ_RUNS) -max_len=64

ci: check-no-committed-binaries check-index-completeness ci-default-features ci-all-features ci-clean-build seam-probe string-edge route-lint checked-input cargo-deny-bans ci-docs arch-lint-canonical-encoder arch-lint-api-surface-qsl arch-lint-qualified-core

# The FR-322 application-node key checked against QSpec's
# published `operation_vectors`, read at run time from the
# quire-specification checkout named by QSPEC_DIR. Opt-in while QSpec is not
# public; nothing of QSpec is copied into this repository. Without QSPEC_DIR
# the test itself skips, so this target refuses to run instead, and it fails
# when the test did not actually check the vectors (a renamed test filters to
# zero tests and would otherwise pass). Both tests are `qsl-semantics`'
# (the key lives in `check::node_key`).
CONFORMANCE_TEST := check::node_key::tests::conformance_fr322_application_keys_match_qspec_operation_vectors
# FR-092-AC-8 (TC-413 step 6): the nominal enum declaration and member keys
# against QSpec's `enum-status` and `enum-status-ready` vectors.
CONFORMANCE_ENUM_TEST := check::node_key::tests::conformance_fr092_nominal_enum_keys_match_qspec_vectors
# TC-411 step 3 (FR-088-AC-12): the compound-unit `UnitId`s against QSpec's
# `value-compound-unit-vectors.json`, guarded the same way.
CONFORMANCE_UNIT_TEST := quantities::tc_411_compound_unit_ids_match_qspec_vectors
# ADR-013 C-14 (TC-421): every source-map entry of QSpec's positive
# v2 fixtures looks up to its wire regions, guarded the same way.
CONFORMANCE_SOURCE_MAP_TEST := checked_v2::tests::conformance_c14_source_map_lookup_over_qspec_positive_fixtures
# QSL's full I2 read over QSpec's checked-package-v2 fixtures --
# every positive fixture admitted, every adverse mutation refused by cause.
CONFORMANCE_I2_TEST := checked_v2::tests::conformance_i2_read_over_qspec_checked_package_v2_fixtures
# FR-340: QSL's full I2 read over QSpec's
# `node-identity-vectors.json` `frame_mutations` -- the frame-body member
# eligibility table and its missing_declaration/invalid_model_binding
# refusal split and precedence.
CONFORMANCE_FRAME_TEST := checked_v2::tests::conformance_fr340_frame_mutations_match_qspec_vectors
# QSL's I2 read over QSpec's `dependency-selection-vectors.json`.
CONFORMANCE_DEPENDENCY_TEST := checked_v2::tests::conformance_dependency_selection_vectors
# FR-093-AC-13, TC-416 step 7: the emitted application nodes against
# QSpec's `positive-operation-identities.json` and
# `positive-control-operations.json`.
CONFORMANCE_GOLDEN_TEST := emit::tests::golden::conformance_emitted_application_nodes_match_qspec_positive_fixtures
# FR-110 and FR-111-AC-7: every header-profile and bundle refusal's
# (code, cause) pair against QSpec's `native-diagnostics.md`.
CONFORMANCE_PROFILE_CAUSES_TEST := check::profile::tests::conformance_fr110_profile_causes_are_listed_by_qspec_native_diagnostics
CONFORMANCE_BUNDLE_CAUSES_TEST := library::bundle_tests::conformance_fr111_resolution_causes_are_listed_by_qspec_native_diagnostics
# The `Value` family's definition catalog and its package-selection
# admission against QSpec's `complete-value-lock.json` and
# `complete-value-selection-vectors.json`.
CONFORMANCE_CATALOG_TEST := complete_value_lock::conformance_catalog_matches_qspec_complete_value_lock
CONFORMANCE_SELECTION_TEST := complete_value_lock::conformance_admit_selection_matches_qspec_selection_vectors
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
	@out=$$(QSPEC_DIR="$(QSPEC_DIR)" cargo test --locked -p qsl-package --lib -- --exact $(CONFORMANCE_FRAME_TEST) --nocapture 2>&1); \
	status=$$?; \
	echo "$$out"; \
	if [ $$status -ne 0 ]; then exit $$status; fi; \
	echo "$$out" | grep -q '^conformance: [1-9][0-9]* frame-body mutation vectors matched$$' || { echo "conformance: the frame-body mutation check did not run" >&2; exit 1; }
	@out=$$(QSPEC_DIR="$(QSPEC_DIR)" cargo test --locked -p qsl-package --lib -- --exact $(CONFORMANCE_DEPENDENCY_TEST) --nocapture 2>&1); \
	status=$$?; \
	echo "$$out"; \
	if [ $$status -ne 0 ]; then exit $$status; fi; \
	echo "$$out" | grep -q '^conformance: [1-9][0-9]* dependency-selection entry mutations and [1-9][0-9]* order vectors$$' || { echo "conformance: the dependency-selection vector check did not run" >&2; exit 1; }
	@out=$$(QSPEC_DIR="$(QSPEC_DIR)" cargo test --locked -p qsl-package --lib -- --exact $(CONFORMANCE_GOLDEN_TEST) --nocapture 2>&1); \
	status=$$?; \
	echo "$$out"; \
	if [ $$status -ne 0 ]; then exit $$status; fi; \
	echo "$$out" | grep -q '^conformance: [1-9][0-9]* emitted application nodes match QSpec.s positive fixtures$$' || { echo "conformance: the emitter golden check did not run" >&2; exit 1; }
	@out=$$(QSPEC_DIR="$(QSPEC_DIR)" cargo test --locked -p qsl-semantics --lib -- --exact $(CONFORMANCE_PROFILE_CAUSES_TEST) --nocapture 2>&1); \
	status=$$?; \
	echo "$$out"; \
	if [ $$status -ne 0 ]; then exit $$status; fi; \
	echo "$$out" | grep -q '^conformance: [1-9][0-9]* profile (code, cause) pairs listed by QSpec$$' || { echo "conformance: the profile cause check did not run" >&2; exit 1; }
	@out=$$(QSPEC_DIR="$(QSPEC_DIR)" cargo test --locked -p qsl-semantics --lib -- --exact $(CONFORMANCE_BUNDLE_CAUSES_TEST) --nocapture 2>&1); \
	status=$$?; \
	echo "$$out"; \
	if [ $$status -ne 0 ]; then exit $$status; fi; \
	echo "$$out" | grep -q '^conformance: [1-9][0-9]* bundle (code, cause) pairs listed by QSpec$$' || { echo "conformance: the bundle cause check did not run" >&2; exit 1; }
	@out=$$(QSPEC_DIR="$(QSPEC_DIR)" cargo test --locked -p qsl-semantics --test it -- --exact $(CONFORMANCE_CATALOG_TEST) --nocapture 2>&1); \
	status=$$?; \
	echo "$$out"; \
	if [ $$status -ne 0 ]; then exit $$status; fi; \
	echo "$$out" | grep -q '^conformance: [1-9][0-9]* catalog rows match QSpec.s lock$$' || { echo "conformance: the definition catalog check did not run" >&2; exit 1; }
	@out=$$(QSPEC_DIR="$(QSPEC_DIR)" cargo test --locked -p qsl-semantics --test it -- --exact $(CONFORMANCE_SELECTION_TEST) --nocapture 2>&1); \
	status=$$?; \
	echo "$$out"; \
	if [ $$status -ne 0 ]; then exit $$status; fi; \
	echo "$$out" | grep -q '^conformance: [1-9][0-9]* accepted and [1-9][0-9]* refused selection vectors$$' || { echo "conformance: the package-selection vector check did not run" >&2; exit 1; }

# FR-059/FR-060 (ADR-011 §7.1 T-12, #215): architecture-conformance
# checks over the QSL/IR/RT/CG ecosystem.
#
# `arch-lint-direction` (FR-059, FB-05/FB-11) needs real local checkouts of
# the three backend repositories; point IR_CLONE/RT_CLONE/CG_CLONE at them.
# It is not part of `ci:` for that reason.
#
# FR-060 T12-B/T12-C: `NodeKey`'s and `EffectiveId`'s mints outside
# `check`/`model` (ADR-013 OBS-018 and FB-13) are a named, shrinking debt
# list, reported as debt, not failures.
# `arch-lint-api-surface`'s T12-A rule is CG-side (#249 review, HIGH-2/
# MEDIUM-4): it scans CG_CLONE for calls into the `qsl-replay` facade crate.
# Without CG_CLONE, T12-A reports NOT EVALUATED, T12-B, T12-C and T12-D still
# run and report, and the target exits 2 (usage) for the missing input.
# `arch-lint-api-surface-qsl` passes `--qsl-only`, which skips T12-A, so the
# QSL-tree rules gate `ci:` without CG_CLONE.
#
# `arch-lint` runs the checks that need only this repository:
# api-surface-qsl, canonical-encoder and qualified-core.
IR_CLONE ?=
RT_CLONE ?=
CG_CLONE ?=

.PHONY: arch-lint-direction arch-lint-api-surface arch-lint-api-surface-qsl arch-lint arch-lint-canonical-encoder arch-lint-qualified-core

arch-lint-direction:
	cargo run --locked -p arch-lint -- direction \
		--qsl . --ir $(IR_CLONE) --rt $(RT_CLONE) --cg $(CG_CLONE)

arch-lint-api-surface:
	cargo run --locked -p arch-lint -- api-surface --qsl . $(if $(CG_CLONE),--cg $(CG_CLONE))

# The API-surface rules over QSL's own tree only (T12-B to T12-E and
# FR-100-AC-8), with T12-A, which scans CG_CLONE, skipped. Needs only this
# repository, so it is part of `ci:`.
arch-lint-api-surface-qsl:
	cargo run --locked -p arch-lint -- api-surface --qsl . --qsl-only

# ADR-013 §2 (ADR-013:113): no second canonical encoder beside
# `quire-canonical` -- a shipped file pairing a `serde_json` serializer with
# a hash fails, named files excepted with their reason. Needs only this
# repository and passes on it, so it is part of `ci:`.
arch-lint-canonical-encoder:
	cargo run --locked -p arch-lint -- canonical-encoder --qsl .

# FR-284 (ADR-029 CB-2, CB-3): no QSL core crate depends on a crate above
# the core, a CG or RT crate, or a frontend crate, and no core crate's
# shipped source reads ambient input or writes to the process. Needs only
# this repository and passes on it, so it is part of `ci:`.
arch-lint-qualified-core:
	cargo run --locked -p arch-lint -- qualified-core --qsl .

# Runs the three checks that need only this repository.
# `arch-lint-direction` needs IR_CLONE/RT_CLONE/CG_CLONE, and
# `arch-lint-api-surface` needs CG_CLONE (see above); each runs separately.
arch-lint: arch-lint-api-surface-qsl arch-lint-canonical-encoder arch-lint-qualified-core

# The whole 30,000-input parser differential against
# tests/fixtures/parser-differential/baseline.txt. `make ci` runs the first
# 1,000 inputs of each family; this runs all of them.
.PHONY: test-differential
test-differential:
	cargo test --locked --test it parser_differential -- --include-ignored

# The committed performance benchmarks (`qsl-bench/`), one
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
	for n in 2000 8000 50000 200000; do $(BENCH_PROBE) parse-volume $$n || exit 1; done
	for n in 250 1000 2000 4000 8000; do $(BENCH_PROBE) check chain $$n || exit 1; done
	for n in 1000 5000; do $(BENCH_PROBE) check independent $$n || exit 1; done
	for n in 3 4 5 6 7 8 9 10 11 12; do $(BENCH_PROBE) check text-cluster $$n || exit 1; done
	for b in 1 64 256; do $(BENCH_PROBE) check deep-wide $$b || exit 1; done
	for n in 1 1000; do $(BENCH_PROBE) eval $$n || exit 1; done
	$(BENCH_PROBE) model 4000 4 100
