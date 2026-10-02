---
id: SR-1226
title: "QSL-482 gap analysis of PR #605 (FR-256, FR-257, FR-091-AC-9, FR-102-AC-5, FR-096-AC-3, NFR-001)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@ec8166e82f696654d39a06e854aeb59baf9338c7; PR #605 diff against origin/main; spec/functional/FR-256-parse-source-at-any-nesting-depth.md; spec/functional/FR-257-build-forms-at-any-depth.md; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md; spec/functional/FR-102-build-state-clause-forms.md; spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md; spec/non-functional/NFR-001-bound-syntax-work.md; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md; spec/functional/FR-311-build-random-reward-and-workload-forms-at-s2.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-256
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-257
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-102
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
---
## Summary

Ticket: QSL-482. PR: quire-spec-language#605. Manual check of each AC against
its tests (quoin gap-analysis method, with no plan bundle for this slice).

Bindings examined:
- FR-256-AC-1 / TC-722: `deep_sources.rs` has five 100,000-deep chains on a
  512 KiB thread, and `limits.rs` checks 6,000-deep parens and `Option`
  with the limits recorded. Correct and strong.
- FR-256-AC-2 / TC-723: `brackets_nest_to_the_default_token_ceiling`
  (bisection, then one deeper refuses `Tokens { bound: 100_000 }`) and
  `longest_chains_parse_and_one_longer_names_a_ceiling`. They do not check
  the setting name, the count reached, or that raising the setting through
  FR-255's settings operation makes the input parse. FND-002.
- FR-256-AC-3 / TC-723: `ten_thousand_nested_brackets_parse_at_the_default_limits`.
  It asserts `Limits` is exactly `{source_bytes, tokens, nodes}`, but the AC
  requires a fourth field, parser work. FND-002.
- FR-257-AC-1 / TC-724: `deep_bodies_build_clone_compare_format_and_drop_on_a_small_stack`
  and the deep parameter-type test check the depth of the built tree, clone
  equality, Debug output and drop on a 512 KiB thread. Correct. The
  `format!("{unit:?}")` non-empty check is weak, but the type-form test
  counts `TypeForm {` occurrences, which is a real oracle.
- FR-257-AC-2 / TC-724: the await-chain and repeat-chain tests check each
  anchor's scope and parent hops, and the full path length of the deepest
  anchor, by induction. Checking at S2 is correct for this AC: FR-257 and
  FR-112 are S2 requirements. The S3 resolver builds `scope_names` per
  anchor (qsl-semantics/src/check/protocol_clause.rs:129, :1003), which is
  O(anchors × depth). That is S3 work, outside this slice. Plan v2 gives S3
  limits and walks to B2/B4.
- FR-257-AC-3 / TC-724, FR-102-AC-5 / TC-457:
  `deep_not_invariants_build_with_no_s2_limit` (128 and 100,000 nots). The
  "no limit set" part holds at compile time (`build_unit(&parsed)`).
  Correct.
- FR-091-AC-9 / TC-397: `deep_not_chains_build_with_no_s2_limit` (8, 20 and
  129 nots, depth = count + 1). Correct.
- FR-096-AC-3 / TC-427: `s1_bounds_the_body_and_s2_builds_it` checks the
  node bound and that a span is present. It does not check the count
  reached, the setting `s1.nodes`, or the region under `RawSourceRef`.
  FND-002.
- Ticket acceptance bullet 3 ("each new heap stack's growth is covered by an
  existing node or byte charge (test)") and plan v2's rule for every B
  slice: there is no such test. FND-001.

Deferrals the coder reported:
- The setting name and the "raise through the settings operation" step of
  FR-256-AC-2, and the setting and count in FR-096-AC-3: these belong to B5,
  which lists FR-255 and FR-096 and `LimitExceeded.setting`. Honest and
  owned.
- The parser-work field of FR-256-AC-3: no slice lists it by name. B5's
  FR-255-AC-3 requires each limits type to map every field to one setting,
  with the union equal to the setting table, and the table has
  `s1.work_units` mapped to "`qsl_cst::Limits` parser work". So B5 owns it
  implicitly. But FR-256 is B1's requirement, and the field plus its builder
  is small.
- FR-257-AC-2 checked at S2 only: correct for the AC, as above.

## Verdict

Changes requested: two medium findings and one low finding. All three are
coverage or spec drift, for this PR's fix round.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Ticket acceptance bullet 3 and plan v2's B-slice rule require a test showing that each new heap stack grows by at most a constant per charged token or node. No such test exists. The claim is only in doc comments, for the lexer delimiter stack, the parser frame stack, the S2 build stack and `ControlWalk`. Fix: add one test per new stack. Instrument the peak stack length (a `#[cfg(test)]` high-water counter, or `quire_walk`'s frame count) on a 100,000-deep input, and assert that peak ≤ k × (tokens or nodes charged). | qsl-forms/src/protocol_clause.rs:474-487; qsl-cst/src/lexer.rs:199-202; qsl-cst/src/parser.rs:1045-1060 |
| FND-002 | medium | Three tests are traced as backing ACs they only partly meet. `ten_thousand_nested_brackets_parse_at_the_default_limits` asserts a three-field `Limits` for FR-256-AC-3, which requires source bytes, tokens, nodes and parser work. Parser work is the fixed `WORK_PER_TOKEN = 256`, not a field. The FR-256-AC-2 tests do not check the setting, the count, or raising the setting. `s1_bounds_the_body_and_s2_builds_it` does not check FR-096-AC-3's count, setting or region. Fix: add `Limits.work_units` (or `work_per_token`) with a `with_*` builder and assert it, which completes FR-256-AC-3 here. For the setting and count parts, which B5 owns, record in QSL-482 that FR-256-AC-2 and FR-096-AC-3 are partial, with B5 as owner, and mark the trace as partial in the test doc comments. | qsl-cst/tests/it/nesting_levels.rs:190-213; qsl-cst/tests/it/nesting_levels.rs:215-243; qsl-cst/src/lexer.rs:21-46; qsl-cst/src/parser.rs:1035; qsl-forms/tests/it/value_forms.rs:550-576 |
| FND-003 | low | FR-311's Inputs list "`FormsLimits`", and its Behavior says S2 "SHALL charge each weighted value, weight and expression node against `FormsLimits`". This PR deletes `FormsLimits`, and FR-257 Behavior 4 says S2 takes no limit set. Fix: in this PR, delete the Inputs bullet and the charging sentence. S1's `s1.nodes` bounds S2. | spec/functional/FR-311-build-random-reward-and-workload-forms-at-s2.md:38; spec/functional/FR-311-build-random-reward-and-workload-forms-at-s2.md:71-72 |
