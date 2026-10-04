---
id: SR-1296
title: "Gap analysis of quire-spec-language PR #630 against FR-264"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@e28bc06358a790a8e27a74b6208956ede2ddfb19; FR-264-AC-1..AC-5 against qsl-package/src/checked_v2.rs, qsl-package/src/checked_v2/tests.rs, qsl-package/src/emit/tests.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-264
    type: reviews
---
# Gap analysis of quire-spec-language PR #630

## Summary

Ticket: QSL-488. Its acceptance says "The tests in this PR back FR-264 and pass".
QSL-488 is the only ticket that implements FR-264. This was a manual check of each
AC against the tests, using `#[trace]` tags and a read of the code.

- FR-264-AC-3: `an_inline_nested_application_argument_is_a_malformed_wire`,
  binding correct.
- FR-264-AC-5: `a_wire_nested_100000_deep_reads_without_a_depth_outcome_on_a_small_stack`,
  binding correct.
- FR-264-AC-1, AC-2, AC-4: no test.

## Verdict

Request changes: three medium findings. B7's own Behavior 4 (the decoded nodes of a
`V2Read` have fixed depth, so clone, eq, Debug and drop do not recurse) has no
evidence at all.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-264-AC-1 has no test. The read-back half is this PR's own code (Behavior 4): a `V2Read` of a 100,000-node flat package must clone, compare, format for Debug and drop on a 512 KiB stack. It can be built now from a synthesized flat wire, as `a_caller_raised_ir_node_ceiling_admits_past_the_ir_default` already does for 10,001 nodes with raised limits. The emit half (compiling a 100,000-term sum on 512 KiB) depends on the deep-compile work (TC-902, FR-356). | qsl-package/src/checked_v2/tests.rs:874-918; spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md:57 |
| FND-002 | medium | FR-264-AC-2 has no test. A walk of the emitted JSON for the TC-415 corpus that classifies each body position as Leaf, Group, Tuple, Member or Body can be written now: admission_corpus.rs already emits that corpus. | qsl-package/src/emit/tests/admission_corpus.rs; spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md:58 |
| FND-003 | medium | FR-264-AC-4 has no tagged test. `a_caller_raised_ir_node_ceiling_admits_past_the_ir_default` (FR-087-AC-3) covers the bound and the raise. It does not cover setting `i2.nodes`, `Locus::Artifact`, FR-255's settings operation or the CLI `--limit`, and the string `i2.nodes` appears nowhere in the code. | qsl-package/src/checked_v2/tests.rs:872-918; spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md:60 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | still-open | The read-back half now has a test (a_100000_node_v2_read_clones_compares_formats_and_drops_on_a_small_stack). The emit half was marked partial and pointed at TC-902, but this tree already compiles, lowers and emits a 100,000-deep sum on a 512 KiB stack: a_100000_deep_body_checks_lowers_and_emits_on_a_small_stack (qsl-package/src/emit/tests/deep_bodies.rs:73, TC-725), which passes at 4c0f5cc5. To close AC-1, read that emission back through read_checked_package_v2 on the same thread, verify it at its package_id, clone, compare, format and drop the V2Read, and remove the partial note. |
| FND-002 | still-open | every_emitted_body_of_the_corpus_is_in_the_stratified_grammar walks the TC-416 step 11 admission corpus (FR-093-AC-19), not the TC-415 corpus AC-2 names. The two overlap only in TC-416's functions; the TC-415 nested forms (expression_depth::FORMS) are walked only as lowered graphs in qsl-semantics (TC-725), never as emitted JSON. Emit each FORMS entry, at the 2 and 1,000 levels deep_bodies already uses, and walk the written bodies, or restate AC-2 to name the corpus the test actually covers. |
| FND-003 | deferred | Lead ruling: the setting name i2.nodes, FR-255's settings operation and the CLI --limit do not exist in QSL yet. They are FR-255's planned work (TC-721, Planned). The bound, raise, consumed count and Locus::Artifact are now tested, and FR-264-AC-4 and TC-739 are marked partial and point to TC-721. |
| FND-001 | fixed | ba79c5ad142f2490b8f5be19a452a99bf5d720bb |
| FND-002 | fixed | ba79c5ad142f2490b8f5be19a452a99bf5d720bb |
