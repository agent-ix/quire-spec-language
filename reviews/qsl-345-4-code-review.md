---
id: SR-1310
title: "Code review of quire-spec-language PR #637: explicit DomainKey subject, field and population call_site selections"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@d764bb72d0cb411f62b266d8455aa8e84b4602a9; PR #637 diff against origin/main c8f0c2818: qsl-foundation/src/bound.rs, qsl-replay/src/{call_site,identity,lib,proof_result,witness}.rs, qsl-replay/src/spine/clause/tests/call_site.rs, qsl-replay/tests/declared_domain_facade.rs, qsl-semantics/src/check/{mod,state_clause,claims}.rs, qsl-semantics/src/check/lowering/model.rs, qsl-semantics/src/family/requirements.rs, qsl-route/src/request.rs, qsl-package/src/emit/extent_agreement.rs, qsl-eval/tests/it/finite_simulation.rs, tests/it/request_builder.rs"
review_set: subset
---
# Code review of quire-spec-language PR #637

## Summary

Ticket: QSL-345 (item 4). PR: quire-spec-language#637, head d764bb72, three
commits rebased onto main c8f0c2818. Rust lane (rust-review) folded in.

Checked and clean:
- `DomainKey` is now `enum { Node{node, path}, Population{member_type, ordinal} }`
  (bound.rs:40). Derived `Ord` puts every `Node` before every `Population`.
  Nothing serializes a `DomainKey`, and QSpec FR-331 states no key order, so
  the reordering of a requirement record's map changes no wire bytes.
- `CheckedGraph::field_domain` (check/mod.rs:1934) resolves the field in the
  named type's effective attribute set, takes `attribute.owner()` as the
  declaring type, sorts that type's own `FieldDeclaration` names with `str`'s
  `Ord` (UTF-8 byte order), and binary-searches the field. That is exactly the
  §15.4 rule. The `u32` conversion is checked and faults rather than truncates.
- The node comes from `AdmittedModel::model_node_key`, which keys the same
  `model_node_content` that `Lowering::model_node` now inserts. `insert_owned`
  and the new `insert_node(model_node_content(..))` build an identical
  `NodeContent`, so the lowering refactor changes no node key. AC-18 pins the
  computed node to the lowering's own node, and AC-19 to the case where no
  clause mints it.
- `population_domain` (check/mod.rs:1986) reads `model_population_tables`, built
  from the same `AdmittedModel::population_domains` iterator that
  `populations_of` (requirement records) now filters. Both build the key through
  the single `state_clause::population_key`. A second implementation of the
  §15.7 ordinal therefore cannot drift from the first.
- `UnknownField` and `UnknownPopulation` map to `Code::MissingDeclaration` in
  `CallSiteRefusal::code` and to `Declined{InvalidInput, Qsl(code)}` in
  `TerminalValue::from_call_site_refusal`, the same arm as `UnknownClause`.
  Their error strings follow the existing `missing_declaration/missing-name`
  prefix.
- Rebase: origin/main equals the merge base (c8f0c2818), so the three-dot diff
  shows every removal against main. In lowering/model.rs, check/mod.rs and
  identity.rs the only removals are the intended ones: the `insert_owned` call
  (replaced by an equivalent call), the `covers` filter (moved into
  `populations_of`), `DeclaredDomain::parameter`, and the unused `WireNodeId`
  import. Nothing from main was dropped or reverted.
- Public API removals: `DomainKey::new`/`node`/`path` and
  `DeclaredDomain::parameter`. On CG origin/main the only uses are
  `declared_domains: Some(Vec::new())` (src/replay/frame.rs:184) and `None` in a
  test, so CG does not break. CG can build parameter keys
  (`Node{node, path: vec![]}`), field keys (`FieldSite.domain`) and population
  keys (`PopulationSite.domain`) through the facade alone.
- witness.rs byte accounting counts a population key's `ordinal`, as it counts a
  node key's `path`. Neither counts the node id, which is consistent.
- No `unwrap`, `expect`, `unsafe` or panic in production paths. The one `panic!`
  in extent_agreement.rs is test code.
- Tests: the coder's make ci on d764bb72 exited 0
  (~/dev/worktrees/logs/qsl-345-item4-make-ci.log). Its log shows all five
  AC-18..22 tests and `a_population_key_never_equals_a_node_key` passing. A
  focused rerun queued behind two other make ci runs on the build lock was
  cancelled as redundant, so no separate build was made.

## Verdict

Approve: no code defects. The ordinal computation is the ruled rule, the
population key has a single construction site, and the rebase is clean. The
spec-side findings are in SR-1312, and the nested-path and trace gaps are in
SR-1311.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
