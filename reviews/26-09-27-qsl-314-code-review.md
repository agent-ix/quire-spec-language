---
id: SR-768
title: "QSL-314 code review (with rust-review lane) of PR 509"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@01356698adcabce3ed91138221b7424b12097bed; tests/it/config_version_spine.rs; examples/config-version/spine.rs; examples/config-version/model.semantic-ir.json; examples/config-version/model.semantic-ir.json.license; examples/config-version/fixtures.rs; tests/it/main.rs; examples/config-version/cases.rs (unchanged); tests/it/config_version.rs (unchanged); qsl-replay/src/spine/clause/tests.rs config_version_domain_document (unchanged); qsl-semantics/src/model/intake.rs read_records/validate_with_semantic_ir (unchanged); qsl-semantics/src/check/node_key/mod.rs FrameField (unchanged); qsl-semantics/src/check/lowering/state.rs frame_node/frame_field (unchanged); quire-contract-model@48ab5dc checked_package/v2/mod.rs validate_frame_body/visit_node_refs/frame_eligibility (pinned dependency)"
review_set: subset
---
## Summary

Ticket: QSL-314. PR: quire-spec-language#509 at 01356698, base f17c2d4f.
It is two commits behind main at 760ef144, and merges cleanly. Methods:
code-review, with the rust-review lane folded in.

Gate: I ran my own `make ci` on a local, unpushed merge of 01356698 with
origin/main 760ef144, using a fresh `CARGO_TARGET_DIR`. It exited 0, with 93
`test result: ok` lines and 0 FAILED. All five TC-469 tests that run passed:
steps 1, 2+3, 4 and 6-pin in both the default and the all-features pass, and
step 5 in the all-features pass only. The I04 test was reported ignored.

What I checked, and what I found:

1. **The domain package validates, and matches the in-crate fixture.**
   `model.semantic-ir.json` matches `qsl-replay/src/spine/clause/tests.rs`'s
   `config_version_domain_document()` member for member. The only difference
   is the package identity (`example/` instead of `test/`), and
   `PLACEHOLDER_DIGEST` is the same all-zero digest. Schema validation is live,
   not assumed: `intake::read_records` always calls `validate_with_semantic_ir`
   (the pinned `agent-ix-semantic-ir` input-bundle schema plus its rules)
   before reading nodes. So every case that reaches `evaluate` in step 1
   (16 of 17) proves the committed document admits.
2. **The three claimed bug fixes are correct in the code.** All three are
   inside files this PR creates, so there is no merged "before" state. They
   are fixes to the author's own draft. The current logic is right:
   - `snapshot_bytes` takes `role` and writes `"observation": role`. The
     pre/post/current roles flow from `write_update` and `write_current`.
     `unchanged-version` passing (success) proves admission accepts it.
   - `request()` uses one fixed `SourceIdentity`. The PackageId
     equality in step 6 would fail if this were per case.
   - The `extracted()` markdown has the `## Invariants` / `### <clause_id>`
     structure, and step 5 passes under `--all-features`.
3. **The step 6 deferral is genuine and precisely scoped.** Run with
   `--ignored`, the test fails with
   `Refused(CheckedPackageRefusal { code: InvalidSemanticGraph, path:
   "/semantic_graph/nodes/2/body/modifies/0" })`, which is exactly QSL-315.
   The pinned reader's `visit_node_refs` checks every `modifies` entry as a
   bare node reference, and `FrameField` serializes as 3 members.
   Side note for QSL-315, outside this PR: `frame_field` returns the *owning
   object type* node, and the reader's `frame_eligibility` allows only a
   `field_declaration` or `relationship` node in `modifies`. So QSL-315's
   proposed "plain `NodeRef(object)`" fix would pass the shape check and then
   be refused on eligibility. The fix needs to reference the field's own
   declaration node.
4. **All 17 cases are covered.** `cases!` generates both `Case` and `CASES`
   from one list, so every variant is iterated. `expected()` is a wildcard-free
   `match` over all 17 variants, and its arms agree with the FR-108 Corpus
   table row for row, including `missing-model`
   (compile/`missing_import`), `forbidden-parent-change`
   (admit/`frame_violation`) and `exhausted-work`
   (evaluate/incomplete/`work_units`).
5. **The parity test uses a real subprocess, but compares too little.**
   `run_native` really invokes `CARGO_BIN_EXE_quire-spec run` and reads the
   real exit code and JSON. It compares exit code only, though (FND-001), and
   step 3's native locus check is vacuous (FND-002).
6. **Step 5 is reachable.** Root `Cargo.toml` defines `quire-extraction`, and
   `make ci`'s `ci-all-features` runs `cargo test --workspace --all-features`.
   The CI log shows `extraction::tc_469_step_5_...` ran and passed. It does
   not run in the default-features pass, by design.

Rust idioms: there is no production code in this diff. Everything is test,
example or fixture code, so panics via `expect`/`unwrap` are acceptable.
There is no `unsafe`, no lock or async code, and no new integer conversions
beyond `exit_code() as i32` (a u8-range exit code, lossless).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The step 2 parity test compares only exit codes. FR-108-AC-2 and TC-469 step 2 require stage (under the map), category, truth, code and exit code. Exit 20 is shared by `dangling_reference`, `frame_violation`, `invalid_runtime_input` and `missing_import`, and exit 0 or 10 does not check the stage. So a spine refusal with the wrong code at the wrong stage would still pass. Native output already carries `status`, `stage`, `truth`, `code` and `diagnostics[].code`: map and compare them. | tests/it/config_version_spine.rs:218-233 |
| FND-002 | medium | The native half of step 3's object check is vacuous. `native_text.contains("root") \|\| native_text.contains("child")` runs over the whole output JSON, and every native ConfigVersion run contains `"child"` in `selection.self_object.key`, so it can never fail. The spine side also accepts either object for either case. TC-469 step 3 requires `root` for below-range and `child` for above-range. Native already reports that in `diagnostics[0].runtime.path` (verified: `{"object":"root"}` for below-range, `{"object":"child"}` for above-range). Assert that exact object, and the exact spine `record.fields["object"]`, per case. | tests/it/config_version_spine.rs:235-276 |
| FND-003 | medium | Step 4 is narrower than FR-108-AC-4 and TC-469 step 4. It re-runs only 3 cases, not every case, and compares 4 report fields, not the report. Its comment says provenance names "the selection and the limits", but neither is asserted. The file comparison is also one-directional: an extra file only in `second/` would pass. | tests/it/config_version_spine.rs:293-302; tests/it/config_version_spine.rs:305-361 |
| FND-004 | low | The step 6 doc comment (FR-108-AC-6, "genuinely runnable now, not still blocked on STD-111") sits above `record_locked_artifacts`, so rustdoc merges it into that helper's doc. It also contradicts the `#[ignore]` on the I04 test. Move it onto `tc_469_step_6_package_id_is_pinned_across_every_case`, and reword the I04 part to cite QSL-315. | tests/it/config_version_spine.rs:498-514 |
| FND-005 | low | The fixed unit `SourceIdentity` literal (`"agent-ix", "ix://example/config-version/spine/unit", "example", "1"`) is spelled out three times. Step 6's PackageId equality depends on the three copies staying identical. Expose one `spine::unit_identity()` and use it in all three places. | examples/config-version/spine.rs:410-416; tests/it/config_version_spine.rs:583-589; tests/it/config_version_spine.rs:623-629 |
| FND-006 | low | Nit: `extracted()` matches `ClauseRunSource` with an `#[allow(unreachable_patterns)] _ => unreachable!` arm. A `let ... else { panic!(...) }` says the same thing without the lint allow. | tests/it/config_version_spine.rs:397-403 |

## Verdict

CHANGES REQUESTED, with three medium test-strength findings. The corpus, the
generator and the domain package are sound, and `make ci` on the merge with
main exits 0. The QSL-315 deferral is honest, and the ignored test fails on
exactly that bug. But the AC-2 parity test and the AC-3 native locus check
assert much less than the spec requires, and step 3's native object check
cannot fail. FND-001 to FND-003 should be fixed in this PR. FND-004 to
FND-006 are small cleanups for the same fix round.
