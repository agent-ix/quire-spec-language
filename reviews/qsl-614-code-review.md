---
id: SR-1209
title: "Code review of quire-spec-language PR #595: obligation_identity, CallSiteRefusal codes, version members deleted"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@31fa710fe1399a1203bbe03d0b09e204049300bc; PR #595 diff against origin/main: qsl-replay/src/{call_site,execute,identity,lib,proof_result,request,result,witness}.rs, qsl-replay/src/execute/tests.rs, qsl-replay/src/spine/clause/tests/{call_site,frame_replay}.rs, qsl-semantics/src/check/identity.rs, spec FR-069/070/071/121, spec/spec.md, spec/tests.md"
review_set: subset
---
# Code review of quire-spec-language PR #595

## Summary

Ticket: QSL-614. The PR does four things:
- It renames `originating_counterexample_identity` to `obligation_identity` on
  `ReplayRequest` and `ReplayRequestWire`.
- It deletes the caller-supplied `contract_version`, `capability_vocabulary`
  and `package_contract_version` members, their checks and their refusals.
  This covers `ReplayRequest`, `WitnessEnvelope`/`WitnessPacket` and
  `BackendProviderSource`.
- It gives `CallSiteRefusal` a `code()`: the struct variants now carry the
  `Code` of the refusal they were built from.
- It rewrites the O-09 doc comments.

What the coordinator asked to check:
- **No compatibility alias is left.** There is no matches for
  `originating_counterexample`, `package_contract_version`,
  `capability_vocabulary` or the three deleted refusal variants anywhere in
  the Rust tree. There is no `#[deprecated]` accessor and no re-export. The
  only remaining `UnknownContractVersion` uses are IR's own
  `CheckedPackageRefusalCode` in qsl-package, which this PR does not cover.
- **The `CallSiteRefusal` tests compare against `replay`'s real codes.**
  `call_site_refusal_codes_are_the_replay_refusal_codes` and
  `call_site_refusal_codes_for_operations_clauses_and_intake` run
  `crate::replay` on the same unit and inputs, and assert `refusal.code() ==
  replayed.code()` for `UnknownFunction`, `Compile`, `Import`,
  `DependencyInput`, `Dependency` and `ModelIntake`. They also assert the
  `replay` variant (`Recompile`, `DependencyInput`), so the match is not a
  coincidence. `UnknownOperation` and `UnknownClause` are checked against the
  `missing_declaration` code that AC-14 names. `Fault` maps to
  `RuntimeInvariant`, the same code as `ReplayRefusal::Fault`.
- The O-09 doc comment in `identity.rs` matches ADR-013 O-09: clause node id,
  occurrence key, kind, arguments, and the span excluded.
- Rust lane (rust-review): no new panic on a production path.
  `CallSiteRefusal::code` is an exhaustive match with no `_` arm. The bound
  tests that used to inflate a deleted member now inflate real members
  (`state_environment`, the backend identity), so they still exercise the
  measured bound.
- `KNOWN_SEMANTIC_PROFILES` is kept, per the team-leader ruling. Its missing
  trace is recorded in the gap analysis (SR-1210).
- Gates run at this head with a worktree-local target dir: `cargo test -p
  qsl-replay --all-features` passed (232 lib tests, 5 integration tests, 1
  doctest group), and `cargo clippy -p qsl-replay -p qsl-semantics
  --all-targets --all-features -D warnings` was clean.

## Verdict

Changes requested: one low finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `ReplayRequestRefusal::UnknownSemanticProfile` doc still says "catalog `unknown_profile`/`unsupported-selection` (revision `1-draft.3`)". A catalog revision label in a doc is version tracking, and this PR edited that same doc line. Delete "(revision `1-draft.3`)". | qsl-replay/src/request.rs:268-271 |
