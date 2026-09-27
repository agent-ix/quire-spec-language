---
id: SR-758
title: "QSL-287/QSL-288 code review (with rust-review lane) of PR 501"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@8b5606b9d2f5b92c14837142f6c0ac99e82b8dd2; src/protocol_artifact/v2/wire.rs; src/protocol_artifact/v2/intake.rs; src/protocol_artifact/v2/refusal.rs; src/protocol_artifact/mod.rs; src/protocol_artifact/native/temporal_v2.rs; src/protocol_artifact/handoff/writer.rs; tests/it/handoff_writer.rs; tests/it/compiled_protocol_v2.rs; examples/native_protocol_v2_handoff.rs; docs/compiled-protocol-v2.md; artifacts/compiled-protocol-v2; xtask/src/string_edge.rs; xtask/src/import_graph.rs; qsl-semantics/src/check/claims.rs; qsl-semantics/src/check/lowering.rs; qsl-package/src/emit.rs; qsl-eval/src/simulation/sample.rs; src/state/evaluation.rs; src/command.rs; src/linking/composed/inventory.rs; tests/it/family_outcome_layering.rs; the remaining #[string_edge] mark sites in the diff"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-050
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-064
    type: reviews
---
## Summary

Tickets: QSL-288 (typed clock_name in the v2 wire), QSL-287 (const-resolving
string-edge detector). PR: quire-spec-language#501 at 8b5606b9, base main.
Method: code-review with the rust-review lane folded in. `make ci` and `make
conformance` exit 0 at this head were measured by the dispatching session and
not re-run here.

Mutation checks were run by this reviewer, each in a scratch detached worktree
at 8b5606b9 with `cargo test --workspace` (plus `--all-features` for the v2
one):

- `lowering.rs` `members.first()` changed to `members.last()`: killed (6
  `claims::tests::tc_160_*`).
- `operation_role`'s `identity == NARROW` branch disabled: killed (many
  lowering/assemble/claims tests).
- `claims.rs:692` `operation_role(identity) == OperationRole::Narrow`
  replaced by `true`: **survives the whole workspace suite**.
- `emit.rs` EnumValue branch of `forced_absent` replaced by `false`:
  **survives the whole workspace suite**.
- `v2/intake.rs` clock-name consistency check disabled: killed (the new
  `a_disagreeing_typed_clock_name_is_refused_not_silently_admitted`, 4 other
  handoff_writer tests and the v2 example test).

The two "marked rather than converted" sites were judged legitimate:
`is_current_observation_contract_revision` is a one-value refuse-only
admission check on an offered wire revision, extracted to a minimal helper,
the same shape as the many contract-version marks already accepted; the
sampler identity/version check in `sample_request` is likewise refuse-only
(FND-007 covers only its mark granularity). The `lowering.rs` positional read
is backed by FR-092 "Function nodes", which fixes `parameters` first, and
`SemanticNode` is built only by the lowering emitter. The `emit.rs` decode
through IR's `CheckedNodeKind::decode` is a reasonable reuse of the closed IR
vocabulary.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The v2 wire contract document was not updated. `docs/compiled-protocol-v2.md` still gives the closed grammar `TemporalBinding = {declaration:U,definition:U,clock:ClockConfiguration}` and its refusal list omits clock-name disagreement. The document is embedded in every written v2 handoff (`CONTRACT_V2 = include_bytes!`), so each handoff now ships a contract its own offer bytes violate, and an implementer following it gets `Error::Json` from the strict reader. QSL-288 itself says "needs spec first if it changes the wire schema". | docs/compiled-protocol-v2.md:44; docs/compiled-protocol-v2.md:76-80; src/protocol_artifact/handoff/writer.rs:63; src/protocol_artifact/v2/wire.rs:81-86 |
| FND-002 | medium | The committed v2 handoff snapshot was not regenerated. `artifacts/compiled-protocol-v2/compiled-protocol-v2.json` has no `clock_name` (the strict reader now refuses its shape) and `mutations/manifest.json` has 28 cases, not 29. Earlier wire-shape PRs (#409 QSL-63, #425 QSL-64) refreshed it. The committed-snapshot tests only verify checksums, so nothing catches this. | artifacts/compiled-protocol-v2/compiled-protocol-v2.json; artifacts/compiled-protocol-v2/mutations/manifest.json; src/protocol_artifact/handoff.rs:50-60 |
| FND-003 | medium | The claim "populated at emission from the same source as the legacy binding name (no new parsing site)" is false. The `clock:` prefix parser `clock_binding_name` is made `pub(crate)` and gains two new callers: v2 emission and v2 admission. The typed field is derived by re-parsing the v1 string, not carried from the lexed temporal form. ADR-012 §9's target column still reads "the prefix is parsed once, at lexing" beside "Done", and QSL-288 asked to remove the prefix convention, which remains in both versions. The v1 freeze justifies keeping it in v1. The ADR row should say the prefix is still parsed at v2 emission and at the v2 consistency check, or emission should take the name from the typed temporal form. | src/protocol_artifact/native/temporal_v2.rs:86-95; src/protocol_artifact/v2/intake.rs:358-376; src/protocol_artifact/mod.rs:111-120; spec/decisions/ADR-012-semantic-family-extension-contracts.md:858 |
| FND-004 | low | `Refusal::code` adds `v2.binding.expected-clock-name` and `v2.binding.producer-clock-name`, but only the Offer side is ever constructed. These are published stable codes with no producer. Restrict the cause to the offer side, or document why the other two exist. | src/protocol_artifact/v2/refusal.rs:217-228 |
| FND-005 | medium | `OperationRole` is a mark, not a conversion. `operation_role` is a `#[string_edge]`-marked wrapper around the same `identity == NARROW` and prefix compares, and it re-runs on every call: for each application via `is_scalar_identity`, and again at `key_claims`'s narrow lookup. Its doc comment ("resolved once ... never re-derived by comparing it a second time") is false. FR-064 Status and the PR list it among the "real typed conversions". QSL-287's own classification asked lowering to record the narrow/scalar role and thread it to claims, and its Done-when says "no mark launders dispatch". Either record the role on the lowered application, or relabel this as a mark in FR-064 Status, the doc comment and the ticket. | qsl-semantics/src/check/claims.rs:76-111; qsl-semantics/src/check/claims.rs:692; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:137-151 |
| FND-006 | medium | Two of the claimed mutation proofs do not hold. Replacing `operation_role(identity) == OperationRole::Narrow` at `key_claims`'s narrowed-bound lookup with `true` passes the full workspace suite. So does replacing the `emit.rs` `ValueForm::EnumValue` branch of `forced_absent` with `false`. The PR body says both were mutation-proved. Add a narrowed-bound claim test where another unary application at the same location references the same key. For `emit.rs`, add a test with an `enum_value` node carrying a declaration occurrence, or record that branch as defensive and unreachable from well-formed lowering. | qsl-semantics/src/check/claims.rs:688-697; qsl-package/src/emit.rs:386-396 |
| FND-007 | low | `sample_request` is marked `#[string_edge]` as a whole public entry, which exempts its entire body from the scan. The commit's own rule extracts a small helper when the function does more than the check, as `is_current_observation_contract_revision`, `is_domain_package` and `is_supported_selection` do. Extract the sampler identity/version check the same way. | qsl-eval/src/simulation/sample.rs:262-277 |

## Verdict

Changes requested. QSL-288's refusal path is sound and mutation-proof: the
new test and the writer's own corpus self-check kill a disabled consistency
check. But the published v2 contract document and the committed snapshot now
contradict the wire (FND-001, FND-002). In QSL-287, the detector and the ~48
marks are sound, and both "marked rather than converted" sites are legitimate
judgment calls, not corner cuts. However, the `OperationRole` "conversion" is
a mark under another name (FND-005). Two claimed mutation proofs do not hold
(FND-006).
