---
id: SR-12323
title: "Gap analysis of QSL-655 process-provider terminal arms against FR-289-AC-4 to AC-6, ADR-029 PL-7/RU-4/PV-2 and QSpec FR-305/FR-462 (PR #1208)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@1ef4a3cbc6f981f2209d912e382c243893b00faa; spec/functional/FR-289-*.md, spec/test-cases/TC-773-*.md, spec/decisions/ADR-029-*.md (PL-7, RU-4, PV-2 context only); qsl-replay/src/proof_result/plugin.rs, qsl-replay/tests/terminal_record_facade.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-289
    type: reviews
---
# SR-12323: Gap analysis of QSL-655 process-provider terminal arms (PR #1208)

## Summary

Ticket: QSL-655, items 1 to 4. The ticket text is treated as a claim and was measured against the code. PR quire-spec-language#1208 at head 1ef4a3cbc. Plan completion: not assessed. QSpec FR-305 and FR-462 were read from a local read-only checkout and are cited by id only.

`quoin matrix` timed out after 300 s and was not used. The trace tags were checked by hand. `plugin_counterexample_settles_by_its_replay` and `plugin_counterexample_refusal_matches_from_replay_refusal` are tagged FR-289-AC-4. `plugin_proof_is_trusted_and_names_the_plugin` is tagged FR-289-AC-5. `plugin_result_claiming_a_qsl_only_cause_fails` is tagged FR-289-AC-6. All four use TC-773, and each assertion matches its AC.

Ticket items:

1. **Plugin counterexample → `refuted`.** Done: `from_plugin_counterexample`. It matches ADR-029 PL-7 "Counterexamples", PV-2 item 4, and QSpec FR-305-AC-4. `TerminalRecord::new` still refuses Input arms, so the Kani path is unchanged.
2. **Plugin `proved` → `trusted`.** Done: `from_plugin_proof` gives `Proved{basis, Trusted}` with the plugin's `BackendId`, and refuses `Checks`. This matches RU-4, PL-7 "Trusted proofs" and FR-305-AC-2 (record half). The renderer half of FR-305-AC-2 ("shows `trusted` beside the verdict") is the driver's and outside this diff.
3. **QSL-only causes.** Done in code: `read_plugin_cause_kind` and `PluginResultRefusal::record`. ADR-029 PL-7 on main does not yet state the rule. That prose is draco's per the planner routing, and FR-289 now states the behaviour before the ADR does.
4. **Plugin-refused request body (FR-462-AC-7).** Verified: no QSL change is needed. `TerminalRecord::failed(index, backend)` is public, and `OutcomeDiagnostic::new` is public for the diagnostic.

Driver fitness (IR-677 / IR-672). The driver needs these, and all are public:

- `ReplayRequest` with `ReplaySource::Input(Vec<CanonicalAssignment>)`;
- `from_plugin_counterexample`, `from_plugin_proof` and `read_plugin_cause_kind`;
- `TerminalRecord::{failed, inconclusive, declined, unsupported, incomplete, tested}`;
- `read_backend_provider_envelope`.

No constructor the driver needs is missing. The changelog line is in the PR body.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-289-AC-4's guarantee "settles `refuted` ... [only when the replay] reproduces the violation" leans on the `category() == Violation` guard, and no TC-773 case exercises that guard. An Input arm that reproduced a non-violation is untested, and a mutation that drops the guard survives. The same defect is SR-12322 FND-001, recorded here against the AC. | qsl-replay/tests/terminal_record_facade.rs:291-333 |
| FND-002 | low | `from_plugin_proof` refuses only `Checks`. It accepts `ProofBasis::ClosedForm`, whose doc says "evidence the core recomputed" (ADR-026 RT-7), and `ProofBasis::Witness`, which is "a replayed witness" (FR-169). A plugin cannot have either property. Paired with `Trusted`, these record a claim QSL never checked. FR-305 says "Its proof basis is the one the plugin reports", so this is the spec's call. Decide whether these two bases are QSL-only, like `Checks`, and refuse them if so. | qsl-replay/src/proof_result/plugin.rs:117-123 |
| FND-003 | low | FR-289-AC-6 says "every other kind is not refused by it", but only `bound-reached` is checked. Iterating over the other `InconclusiveCause::as_str` spellings would make the "every other" claim falsifiable. | qsl-replay/tests/terminal_record_facade.rs:412 |
| FND-004 | low | A `failed` record from `PluginResultRefusal::record`, or from a fault or Witness-arm counterexample, carries no `diagnostic`. `TerminalRecord::with_diagnostic` is `pub(crate)`, so the driver cannot attach one. QSpec FR-305 says failures settle `failed` "with a typed tool-failure cause". Item 4 shows the established pattern: the diagnostic goes in the outcome document. The refusal's cause is therefore the driver's to carry. Say so in PL-7 when draco writes it. | qsl-replay/src/proof_result/plugin.rs:51-55 |

## Verdict

The ticket's items 1, 2 and 4 are met, and item 3 is met in code. Its ADR prose is pending in draco's lane. There are no high gaps. The one medium is a missing test for a soundness guard, and it is cheap to add.
