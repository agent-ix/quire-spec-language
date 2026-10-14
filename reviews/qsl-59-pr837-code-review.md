---
id: SR-9097
title: "Code review of quire-spec-language PR #837: test-support gating, string-edge marker, Lowering rename"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@32f665201524406e3ddde0e96bf5e234e3be75a6; qsl-semantics/tests/it/main.rs, qsl-semantics/tests/it/variant_type.rs, qsl-semantics/src/model/intake/unit.rs, qsl-semantics/src/check/lowering.rs"
review_set: subset
---
# Code review of quire-spec-language PR #837

## Summary

Ticket: QSL-59. Code review with the rust-review lane folded in, scoped to the PR diff against base `impl/business-model-meanings` (commits 6e573567e, 32f665201). Reviewer-only: no build; gate results taken from the coder's gate log.

Examined:
- `mod model_conformance_boundary;` and `mod model_intake;` gated on `feature = "test-support"` in tests/it/main.rs. The only other references to them in tests/it are from `model_conformance.rs` (already gated) and `variant_type.rs::lifted` with its two callers (gated in this PR), so the default-feature build has no dangling path (examined)
- the gated tests still run in the gates. Feature unification turns `qsl-semantics/test-support` on for `--workspace` (root dev-dependency), so they run in both the `default-workspace` and `all-features-workspace` lanes. The gate log shows 23 `model_conformance_boundary` and 19 `model_intake` tests passing in each lane (46 and 38 lines). Only the `-p qsl-semantics` default lane skips them, and that lane exists to test the not-test-support paths (examined)
- `use crate::model::domain_package::DomainPackage` moved under `cfg(any(test, feature = "test-support"))`. `-p qsl-semantics --all-targets` clippy with `-D warnings` passes, so there is no unused import and no missing import in either configuration (examined)
- `#[qsl_attrs::string_edge]` on `with_parsed_operation_clauses`: the compare is `clause.language == "quire"`, where `language: String` holds an open authored-language identifier from the domain package wire form. Clauses in other languages are kept verbatim, so no closed enum fits. This matches how the repo already uses the marker (for example `NativeValueType::from_str`). xtask string-edge passes (examined)
- `Lowering::composite` renamed to `composite_type_node`: the definition plus 4 call sites (`composite_node`, tuple value, record value, field member). No `self.composite(` call remains in lowering.rs. The remaining `.composite(` calls are `TypeTable::composite`, a different method. The new name is unique in the workspace (only a similarly named test function exists in check/mod.rs). The method is private, so there is no behaviour change (examined)
- gate evidence: /home/peter/dev/worktrees/logs/qslcifix-ci4.log ran 19:18:28 to 20:25:56 and ends `EXIT=0`. Commit 32f665201 is timestamped 20:26:13, but every changed file's mtime predates the run start (latest: lowering.rs at 19:16:41). The run therefore built the final tree, and the commit only recorded it. string-edge and arch-lint canonical-encoder both PASS in the log (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS.** The gating drops no test from any gate that matters, the string-edge marker covers a real wire-identifier compare, and the rename is complete with no behaviour change. The gate ran on the final tree and ended EXIT=0.

Recorded, not reported (below the reporting bar): low, the `string_edge` attribute exempts the whole function body, not just the one `language == "quire"` compare.
