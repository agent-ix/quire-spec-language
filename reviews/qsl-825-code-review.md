---
id: SR-12081
title: "Code review of quire-spec-language PR #937: all-features test lane package scope"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@7956f9efcab154e35a7d63fad864ff7e7cd73fda; PR #937: xtask/src/feature_scope.rs, xtask/src/test_suites.rs, xtask/src/ci_feature_lane_tests.rs, xtask/src/lib.rs"
review_set: subset
---
# Code review of quire-spec-language PR #937

## Summary

Ticket: QSL-825. Code review with the rust-review lane folded in. Read-only
review of `git diff origin/main...HEAD`. The resolves were measured with
`cargo metadata` and `cargo tree --locked --offline` (resolution only, no
build).

The decision is computed at every run from two `cargo metadata` resolves.
There is no hand-maintained package list, ledger or pin. The 16-package table
this review reproduced from `cargo metadata` matches the author's table: 9
skipped, 7 run. The default lane still runs `cargo test --workspace` over
every package. All-features clippy and build still cover every package.
`validate_lane_selection` still refuses a caller-supplied `--exclude`, and it
runs before the computed excludes are added. Malformed metadata returns
`ArtifactError`, and there is no unwrap or panic on the new production paths.
The fixed point adds one member per iteration and stops when none is added, so
it always terminates. Reading the code found no compile or type error: the
borrows, coercions, `is_none_or` (toolchain 1.98.1), `Copy` on `EmptySuite`,
the glob import of `skipped` in `real_cargo`, and the test helpers all check
out.

`cargo tree` (resolver 2, per unit) gives the same result for all 9 skipped
packages under the default and all-features selections. That holds for target
units and for host units. So today no skipped package goes untested in the
all-features lane.

## Verdict

Changes requested. One high finding: excluding the skipped packages changes
the feature resolve of the packages that remain, and this happens on the
current workspace. One medium finding: the comparison is not fail-safe for
host/target and platform splits. Both findings break the planner's fail-safe
condition. Where the rule cannot prove that a skip is safe, it skips anyway
instead of running the package.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The sole-requester fixed point only models members' direct manifest `features = [...]` requests. It misses everything else that `--exclude` removes: transitive requests through the skipped package's own dependency subgraph, `default-features = true` versus `false`, `[features]`-table `dep/feat` activations, and platform-conditional requests (a kept package's `cfg(windows)` request counts as cover). This was measured on the current main with the 9 computed excludes. qsl-attrs is a proc-macro, and its dev-dependency chain `trybuild -> toml -> serde_spanned/toml_datetime` is the only thing that turns on `serde_core/alloc` on the host. Excluding qsl-attrs turns it off. So `cargo tree --workspace --all-features --exclude ...` has a host `serde_core` with features `result,std`. That feature set exists in neither the full all-features resolve nor the default one. The host units built against it include `semver`, `serde` and `serde_json`, then the build scripts of `tree-sitter` and `quire-verification-contracts`, and the proc-macros `typify-macro` and `schemars_derive`. The all-features lane compiles those, runs the build scripts, and feeds their generated code into the kept packages' test binaries, under a feature set that no proof covers. Target-side features of the kept packages are identical today, but the same mechanism removes target features when the lost request sits on a normal or dev edge. Fix, fail-safe: after choosing the excludes, check that the per-unit resolve (`cargo tree -e features`, host and target) of the remaining selection equals the full `--all-features` resolve for every remaining unit. On any difference, keep the responsible package, or else run the whole workspace. | xtask/src/feature_scope.rs:19-26; xtask/src/feature_scope.rs:84-98; xtask/src/feature_scope.rs:270-300 |
| FND-002 | medium | `cargo metadata` has one resolve node per package id. Its `features` are one set: the union over host and target units, every platform's `cfg` edges, and all dependency kinds. Resolver 2 builds those separately. The `serde` node shows this today: `cargo tree` has two variants, `alloc,default,derive,serde_derive,std` and `default,derive,serde_derive,std`, and metadata reports one. So equal node features do not prove an equal compile. Suppose a dependency D in a skipped package's closure gets feature f on the target only under `--all-features`, through a member outside that closure, and gets f on the host (or on another platform) in the default build. Both metadata unions are then `D{f}`, so the package is skipped, but its target test binary differs between the lanes and is never tested under all features. No such case was measured on the current main: `cargo tree` per root agrees for all 9 skips. But the module documents this as a proof, and this path falls to SKIP where it should fall to RUN. Fix: compare per-unit resolves (`cargo tree -e features` with host and target kept apart, or `--filter-platform` on the host triple plus a host-unit check), or treat any package with a host/target or platform-split dependency in its closure as RUN. | xtask/src/feature_scope.rs:9-17; xtask/src/feature_scope.rs:231-253 |

The finding table holds defects only. The checks that were verified and found
clean are listed under Summary.

## Dispositions

| ID | Disposition |
| --- | --- |
| FND-001 | fixed: after `cargo metadata` nominates candidates, `settle` checks them against `cargo tree` per-unit feature sets of the full `--all-features` workspace and puts back, one at a time, the candidate whose return removes the most differences; with none left the whole workspace runs. On the real workspace qsl-attrs is put back. |
| FND-002 | fixed: the verification runs `cargo tree --target all` with host and target units kept apart, so a platform or host/target split that metadata merges shows as a difference and the package runs. |
