---
id: SR-1947002
title: "gap analysis of PR #1947 (QSL-1358): linear Debug for the checked package source map"
type: SpecReview
analysis: gap-analysis
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1947 git diff origin/main...516b5f9b9 (FR-264-AC-1; qsl-foundation/src/source/provenance.rs tests, qsl-semantics/src/library/mod.rs tests, qsl-package/src/emit/tests/deep_bodies.rs as context)"
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-264
  type: reviews
---
# SR-1947002: gap analysis of PR #1947 (QSL-1358)

## Summary

Ticket: QSL-1358. Plan completion: not assessed. The PR changes no spec. Three new tests are tagged `FR-264-AC-1`. I checked each tag against the AC text and checked whether each test can fail on the old implementation. With the derived `Debug` restored locally, `the_map_debug_output_grows_linearly_with_depth` fails (500 -> 1k: 2,922,057 -> 11,589,308 bytes). The ImportView test's `lib:[` count would be 3 on the old Display-based output, not 2. So both tests can fail. The binding to FR-264-AC-1 is wrong, though. The AC's own test is `a_100000_term_sum_reads_back_verifies_clones_compares_and_drops_on_a_small_stack` (TC-738). These unit tests cover a QSL-381 scaling property of one component, not the AC.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-264-AC-1 | spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md:57 | examined |
| the_map_debug_output_grows_linearly_with_depth | qsl-foundation/src/source/provenance.rs:575-585 | examined |
| the_map_debug_names_each_node_once_and_each_entry | qsl-foundation/src/source/provenance.rs:588-611 | examined |
| import_view_debug_writes_each_node_once | qsl-semantics/src/library/mod.rs:495-518 | examined |
| a_100000_term_sum_reads_back_verifies_clones_compares_and_drops_on_a_small_stack | qsl-package/src/emit/tests/deep_bodies.rs:179-222 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Three unit tests are tagged FR-264-AC-1, but none of them exercises the AC (a 100,000-term sum emitted, read through I2 and formatted as a V2Read on a 512 KiB stack). They test a 2,000-deep in-memory map and a 2-node ImportView, which inflates AC-1's binding count. | qsl-foundation/src/source/provenance.rs:575, qsl-foundation/src/source/provenance.rs:588, qsl-semantics/src/library/mod.rs:495 |
| FND-002 | medium | The linear-growth test counts bytes only, and only for an Arc-shared chain. It cannot detect the quadratic lookup time that wire-read (unshared) keys cause (SR-1947001 FND-002), so "linear" is asserted for output size alone. | qsl-foundation/src/source/provenance.rs:552-585 |

## Verdict

**Changes requested (test binding and coverage).**

1. Both new exact-output tests are real checks. `the_map_debug_names_each_node_once_and_each_entry` compares the whole string against a literal, and it pins position order, entry order and region formatting.
2. The growth test passes at head and fails on the derived Debug, so it is not vacuous for output size. It builds every key by `NodeKey::new(.., PathSegment::Node(node))` over the previous key, so every nested node is the same `Arc`. Production keys from `read_checked_package_v2` are rebuilt per entry by `emit::admitted_key`. The test therefore never reaches the `NodeKey == NodeKey` deep-comparison path that dominates time there.
3. FR-264-AC-1's real test is the deep_bodies test, and the ticket requires it to finish unchanged. It is the right binding for this ticket's acceptance. See SR-1947001 FND-001 for its result at this head.
