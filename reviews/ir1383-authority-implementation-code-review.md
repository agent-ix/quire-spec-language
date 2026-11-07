---
id: SR-202611070458156
title: "IR-1383 authority code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; IR-1383 PR2489 same-emission order/progress authority; FR-251-AC-45..48"
review_set: subset
---

## Summary

Reviewed the implementation at 818ff2bcc9c20208c530d2cbb43a2ee994414248 against the merged FR-251/TC-1081 authority contract and actual observation PR49 API. Original native errors, structural NodeIdentity/ClaimOccurrence and owner View values are retained; no mirror authority or document-byte substitute was found.

## Verdict

**CONDITIONAL** — Fix each recorded finding and qualify genuine positive admission before merge. The code lead owns the one final aggregate gate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | A correct existing claim with a different independently selected observation source is filtered out before check_time_source_fields and returns Selection(Claim). The public Source code documents this actual mismatch but is unreachable through admit_authority. Distinguish no matching claim from a matching claim whose observation source differs, and assert exact Selection(Source). | qsl-analyze/src/monitor_root/progress.rs:390-403 |

## Coverage

Plan completion: not assessed

Rust review covers API ownership, read-only PlanRoot/AdmittedPlanAuthority, typed errors, finite reader/canonical limits, structural node/occurrence equality, original versus final-emission joins, public getters and complete owner value retention. QSL SourceIdentity and observation Identity are deliberately distinct domains and are independently selected. The final package is strictly read from this immutable root's own final emission/evidence; its AdmissionBinding cannot be replaced through a public setter. Source and test changes contain no unsafe or CI weakening. First-party dependency manifests remain branch=main. No applicable AssuranceProfile was found.

The positive fixture genuinely constructs a StaticProducerBundle, uses owner admission, canonical record/population identities, derives and strict-reads progress before supplying View. Runtime success still requires the positive tests to pass; no owner capability is inferred from a name, digest, arbitrary JSON or component-only comparison.

Computed matrix was attempted with a 30-second bound; result is recorded separately. Manual criterion/test/code correspondence checks direct paired trace tags. No whole-repository completion or computed coverage credit is asserted. No spec/plan file is changed relative to current main, so spec-review methods are not applicable; merged FR-251/TC-1081 are the context.

## Validation evidence

Coder compile/check passed in /tmp/IR-1383-authority-check.log. At the original runtime checkpoint /tmp/IR-1383-authority-tests.log reports three passing negatives and two genuine positive failures at strict IR admission. No reviewer Cargo build or aggregate gate was started in the shared ticket worktree. Later fixes require this reviewer's explicit disposition.

## Examined scope

- `FR-251-AC-45` — `spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md`: From a genuine authored tick-monitor unit, the public plan-root handoff returns the complete QSL-admitted model-step order and the observation owner's original canonical association VALUE, bound to the same final emission as its plan and two timing obligations. A public consumer compares the complete values and original source, structural clause node, claim role and ordinal, selected subject and checked model-time facts; the final emission temporal binding join equals its plan-root clause and the strict reader declaration. Both values and owner capabilities remain usable after temporary compil
- `FR-251-AC-46` — `spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md`: Independently omit the checked order premise and progress association; substitute a foreign admitted emission; change the structural clause node, claim role, claim ordinal or original source; change the selected subject, observation source, clock identity/revision, model, structural model/unit/dimension/digital-operation node, unit, source kind, digital operation or exact period; and substitute order or progress values. Each candidate returns its typed missing-premise, owner refusal or mismatched-selection condition and no partial admitted context. Exact owner canonical-association byte bounds
- `FR-251-AC-47` — `spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md`: After original source, model and request inputs are dropped, a public consumer reads the actual combined checked unit retained by the plan root: its exact source includes the authored unit and generated entries, its selected clause equals the plan subject, and its checked time source retains the selected model node, source kind and unit. These are explicitly original checker facts, not final renamed identities or admitted order/progress.
- `FR-251-AC-48` — `spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md`: A genuine dense model-time monitor root yields the complete specified canonical model-step value, with the actual checked model/unit/dimension identities. A digital source retains its exact period and checked tick-operation anchor. Changed source kind, unit or period changes the value. Exact-byte-ceiling encoding succeeds and one byte less returns Encoding with the original native canonical limit and no partial order value.
- `Cargo.lock` — `Cargo.lock`: Reviewed the complete changed file and relevant callers/owner contracts.
- `qsl-analyze/Cargo.toml` — `qsl-analyze/Cargo.toml`: Reviewed the complete changed file and relevant callers/owner contracts.
- `qsl-analyze/src/monitor_root.rs` — `qsl-analyze/src/monitor_root.rs`: Reviewed the complete changed file and relevant callers/owner contracts.
- `qsl-analyze/src/monitor_root/order.rs` — `qsl-analyze/src/monitor_root/order.rs`: Reviewed the complete changed file and relevant callers/owner contracts.
- `qsl-analyze/src/monitor_root/progress.rs` — `qsl-analyze/src/monitor_root/progress.rs`: Reviewed the complete changed file and relevant callers/owner contracts.
- `qsl-analyze/src/monitor_root/progress/tests.rs` — `qsl-analyze/src/monitor_root/progress/tests.rs`: Reviewed the complete changed file and relevant callers/owner contracts.
- `qsl-analyze/tests/it/main.rs` — `qsl-analyze/tests/it/main.rs`: Reviewed the complete changed file and relevant callers/owner contracts.
- `qsl-analyze/tests/it/monitor_authority.rs` — `qsl-analyze/tests/it/monitor_authority.rs`: Reviewed the complete changed file and relevant callers/owner contracts.
- `qsl-analyze/tests/it/monitor_authority/observation.rs` — `qsl-analyze/tests/it/monitor_authority/observation.rs`: Reviewed the complete changed file and relevant callers/owner contracts.
- `qsl-analyze/tests/it/monitor_root.rs` — `qsl-analyze/tests/it/monitor_root.rs`: Reviewed the complete changed file and relevant callers/owner contracts.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The source mismatch is distinguished from absent claim before canonical authority issuance. The genuine independent selector control asserts exact Selection(Source) and passes. |

## Current outcome

**PASS** on c7368900224daca67789bc6c03f8b25e4b9df868 — All original findings are fixed. Re-reviewed the full implementation and the new writer evidence delta. The final context consumes one immutable original PlanRoot and retains that exact final emission, original checked unit, actual strict-reader package, QSL model-step value, original observation View and its complete bounded canonical association value. QSL source and observation source are distinct domains; no equality or invented field conflates them. Full structural node and actual claim occurrence fields are compared, then the retained temporal row joins to the strict reader's final clause. No mirror authority, synthetic evidence, source re-emission or arbitrary document-byte substitution is introduced.

The producer evidence fix clones the actual CheckedPackageEvidence supplied to the original strict base read, preserving admitted domain/dependency capabilities. The private writer's final package serializes the same base lock, so supporting base.lock.required_features at this boundary names precisely that final lock; no additional fabricated record or authority is synthesized. New strict reads use only returned emission evidence.

## Final focused evidence

/tmp/IR-1383-authority-tests.log: seven genuine authority tests pass (0 failures, 8.62s), including real tick positive/full owner values, dense/tick retained evidence without reinjection, isolated independent claim controls, exact source diagnostic, foreign/missing selections and canonical exact/one-short bounds. /tmp/IR-1383-authority-clippy.log: qsl-analyze library/test-target Clippy passes with warnings denied (16.72s). Read-only compile-fail documentation covers root and admitted-order immutability; the lead's final gate includes qsl-analyze documentation and the existing monitor-root suite. No duplicate reviewer Cargo or aggregate gate was run. Computed matrix timed out at 30s with no JSON (exit124); manual scoped trace review is recorded, without computed coverage credit or whole-repository completion claim.

## Additional examined scope

- `qsl-package/src/emit/monitor_plan.rs`: actual evidence retention at the publication boundary and unchanged final lock.

Additional focused results verified on the same clean c73689002 source: nineteen existing monitor-root tests, one private checked-source component test, and both immutable-context compile-fail doctests pass in the corresponding /tmp/IR-1383-authority-{existing-roots,components,docs}.log files. No further source changes or reviewer builds.

## Final combined-head verification

**PASS** on 199530b4a62b008d1e3101da0690181d5742c508 — Rechecked the composed current-main diff. The only intervening upstream delta is five unrelated specification Markdown files under FR-290/FR-529/FR-530 and TC-1329/TC-1330. All Rust sources, manifests and Cargo.lock are identical to the genuinely tested c73689002 head. No new findings; all round-one dispositions remain fixed. No duplicate Cargo test or review comment is needed for this source-equivalent composition.
