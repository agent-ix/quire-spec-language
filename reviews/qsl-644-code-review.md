---
id: SR-1350
title: "Code review of quire-spec-language PR #649: the qsl_replay::compile_package facade (QSL-644)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@3d53d1dd4c57afaac44a9112bc8ae1a270fa969f; PR #649 diff against origin/main: qsl-replay/src/compile.rs, qsl-replay/src/execute.rs, qsl-replay/src/lib.rs, qsl-replay/tests/compile_package_facade.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
---
# Code review of quire-spec-language PR #649

## Summary

Ticket: QSL-644. Code review with the rust-review lane folded in.

The PR moves the S1-to-E4 spine run out of `recompile` (qsl-replay/src/execute.rs) into `run_spine`, makes `spine_limits` crate-visible, and adds `compile_package` and `CompiledPackage` (qsl-replay/src/compile.rs), re-exported from the crate root.

- Same pipeline, no replay behaviour change. `run_spine`'s body is the old inline block, token for token: the same `Cancel::new()`, the same `refusal_or_fault` mapping to `ReplayRefusal::Recompile`/`Fault`, `parse` under `limits.source`, `select` under `limits.model`, `check` with `LockEvidence::default()` and the full `SpineLimits`, and `package` under `PackageLimits::default()`. `recompile` passes `&labels(source)`, `source.identity()`, the provided bytes, `domain_packages(request)`, the built `DependencyInput` and `spine_limits(request.stage_limits())`, which is what the old block used. Rules 1 to 3 and 5 to 7 are untouched, and their order relative to the spine run is unchanged.
- Inputs match. `packages` is `impl IntoIterator<Item = &'a [u8]>` in fourth position, as in `call_site`. It goes through the same `qsl_semantics::model::intake::package_input` that `call_site` and replay's `domain_packages` use, so packages are keyed the same way. Stage limits go through the same `spine_limits`, so `LimitAboveReader` and the S1/S3 mapping are shared. The unit-owner check is `DependencyInput::check_unit_owner`, mapped to `ReplayRefusal::DependencyInput`, as in `recompile`. The facade runs `spine_limits` before the owner check, which is the same order as `recompile`.
- Public API surface. Two new public items: a function and an opaque struct with private fields and two accessors (`bytes()`, `package_id() -> DigestRecord`). Every type in the signature (`SourceIdentity`, `DependencyInput`, `SuppliedLibrary`, `StageLimits`, `ScalarLimits`, `ReplayRefusal`, `DigestRecord`) is already exported from the crate root, so a CG caller never has to name `qsl_replay::spine`, which T12-A requires. Reusing `ReplayRefusal` means callers match on replay-only variants, but those variants already exist and the spec says it "refuses as that recompile does". That is a design choice, not a defect.
- Rust idioms. No `unwrap`, `expect` or panics in production code, no new `unsafe`, no integer casts. `run_spine` takes `&BTreeMap<[u8; 32], Vec<u8>>` by full path, matching `domain_packages`' return type. The doc comments are accurate. Two doc lines run past the wrap width (`compile.rs` line 37 and `lib.rs` line 6). rustfmt leaves comments alone, so this is only a nit and has no row.
- Gates run at 3d53d1dd: `cargo test -p qsl-replay` and `cargo clippy -p qsl-replay --all-targets` through `locked-build.sh` (results in Verdict).

Examined:
- qsl-replay/src/compile.rs (examined)
- qsl-replay/src/execute.rs `spine_limits`, `run_spine`, `recompile`, `domain_packages`, `labels` (examined)
- qsl-replay/src/lib.rs (examined)
- qsl-replay/tests/compile_package_facade.rs (examined)
- qsl-replay/src/call_site.rs `call_site` (context_only)
- qsl-semantics/src/model/intake/unit.rs `package_input` (context_only)
- qsl-semantics doc comments naming `next?:` (examined, per the brief)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Five qsl-semantics doc comments still spell the optional field as `next?: X`. The grammar no longer parses that form: `P::Field` is `ident ":" TypeReference "?"? ";"` (qsl-cst/src/grammar.rs:437-446). The PR fixes the same declarations in FR-092, FR-093 and TC-413 but not these comments, so the code docs now disagree with the spec they cite. Respell them `next: List?`, `next: Node?`, `next: N?` and `next: C{i+1}?`. | qsl-semantics/src/check/lowering/tests.rs:686, qsl-semantics/src/check/lowering/tests/leaves.rs:63, qsl-semantics/src/check/lowering/tests/differential.rs:104, qsl-semantics/src/check/depth_forms.rs:72, qsl-semantics/src/check/lowering/tests/depth.rs:19 |
| FND-002 | low | The test module doc names "TC-908 (FR-060-AC-5, FR-060-AC-6)", but the file also carries the FR-060-AC-7 test (`a_domain_package_is_the_i1_input_as_in_the_spine`). Add FR-060-AC-7 to it. | qsl-replay/tests/compile_package_facade.rs:2 |

## Verdict

Gates at 3d53d1dd: `cargo test -p qsl-replay` passes (343 unit tests, 5 in `compile_package_facade`, plus the other integration tests and doctests, 0 failed), so the replay tests show no behaviour change. `cargo clippy -p qsl-replay --all-targets -- -D warnings` is clean, and `cargo fmt --check` passes.

Two low findings. Neither is a behaviour defect. The extraction is a faithful single pipeline that `replay` and `compile_package` both run. `compile_package`'s inputs, limits, owner check and refusals match `recompile`, and its packages input matches `call_site`. The public surface is minimal and fits T12-A. Mergeable once the findings are fixed in this PR.
