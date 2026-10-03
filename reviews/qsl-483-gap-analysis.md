---
id: SR-1261
title: "QSL-483 gap analysis of PR #615 (FR-258-AC-1 to AC-5, FR-269-AC-3; TC-725, TC-726, TC-727, TC-744)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@220d8fa2b2028aad8e7379ed6550707900fbda24; PR #615 diff against origin/main (merge base 8d1deba43); spec/functional/FR-258-check-and-lower-expressions-at-any-depth.md; spec/test-cases/TC-725-deep-expressions-check-lower-and-emit-on-a-small-stack.md; spec/test-cases/TC-726-deep-types-lower-their-text-leaves-on-a-small-stack.md; spec/test-cases/TC-727-checker-defaults-and-limit-outcomes-name-their-setting.md; spec/functional/FR-269-settle-a-witness-disagreement-as-a-typed-cause.md; spec/test-cases/TC-744-settle-a-witness-disagreement-as-a-typed-cause.md; spec/decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md (D-1, D-2, D-4.3, D-7); spec/functional/FR-356-walk-nested-structures-through-one-iterative-walker-toolkit.md; spec/functional/FR-062-implement-checked-family-contract.md (AC-5, AC-7, AC-11, AC-12); spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md (AC-14); spec/functional/FR-092 (AC-7); spec/test-cases/TC-378, TC-381, TC-413, TC-415"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-258
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-269
    type: reviews
---
## Summary

Ticket: QSL-483 (B2). PR: quire-spec-language#615, head 220d8fa2b. This is a
manual check of each AC against its tests (the quoin gap-analysis method; no
plan bundle for this slice is in the repo, and Linear is unreachable). Code
findings are in SR-1260.

Trace, per unit:

- **FR-258-AC-1 / TC-725 step 1. Not met (FND-001).** The end-to-end test
  `a_100000_deep_body_checks_lowers_and_emits_on_a_small_stack` is ignored.
  The test that runs, `a_100000_deep_checked_body_clones_compares_formats_and_drops_on_a_small_stack`,
  builds a body by hand and never runs the checker. The ignore reason
  (`Location` paths) is accurate but incomplete (SR-1260 FND-002).
- **FR-258-AC-2 / TC-726. Met.** There are three tests:
  - `a_100000_deep_option_lowers_its_text_leaf_on_a_small_stack` checks a
    path of 100,000 `inner` segments.
  - `a_100000_record_chain_lowers_its_text_leaf_on_a_small_stack` checks a
    path of 2×100,000+1 segments ending `field:label`.
  - `a_100000_deep_option_type_keys_on_a_small_stack` compares every node key
    on a 512 KiB stack with the keys on 8 MiB, and counts 100,000 `option`
    nodes.

  The oracles are strong. One wording mismatch (FND-006).
- **FR-258-AC-3 / TC-727 step 1. Met.**
  - `the_longest_and_chain_s1_admits_checks_and_emits_at_the_defaults`
    bisects for the longest chain S1's defaults admit, then checks and emits
    it on a 512 KiB stack at `CheckingLimits::default()`.
  - `the_checking_defaults_hold_no_depth` checks
    (100,000; 16,777,216; 16,777,216) and node counts 0, 1 and `u64::MAX`.
- **FR-258-AC-4 / TC-727 steps 2-3. Partly met (FND-003).** The node stop
  asserts code, kind, bound 1,500, count 1,501 and the `1` region, through the
  contract path. The work stop asserts code and bound only.
- **FR-258-AC-5 / TC-725 step 2. Partly met (FND-002).** The TC-415 half is
  covered at 2 and 1,000 levels. The AC-1 half is not.
- **FR-269-AC-3 / TC-744. Wrong binding (FND-004).** The deep encode test is
  tagged to AC-3, but AC-3 is about round-trip, refusal and inequality. The
  existing round-trip tests still back AC-3.
- **FR-062-AC-7 / TC-378.** `real_checker_node_limit_is_the_proximate_cause`
  (newly tagged TC-378) checks kind, bound N-1 and count N, and that N
  admits. The AC also names the setting `s3.nodes`, which no outcome carries
  yet (the same gap as FND-003).
- **FR-093-AC-14 / TC-415.**
  `every_nested_form_checks_at_1000_levels_as_at_2_on_a_small_stack` compares
  the outcome at 1,000 levels with the outcome at 2, and the `pre` test refuses
  as a forbidden pre-read. The bindings are correct. The longest-`and`-chain
  sentence is covered by the TC-727 step 1 test, which carries only the
  FR-258-AC-3 tag.
- **FR-092-AC-7 / TC-413.** The deep option keying test is correct for the
  512 KiB half. The 30-record and 1,000-record chain tests now assert that the
  chains check.

Underspecified code:

- `encode_element`'s depth property has no owning AC (FND-004).
- The text-leaf walk's charged-growth bound is stated in FR-258 Behavior 3/4,
  but no AC names it (SR-1260 FND-007).

Spec drift: stale TC text (FND-005). The PR body lists docs among the touched
files, but the diff touches no spec or doc file.

## Verdict

Changes requested. AC-2 and AC-3 are fully and strongly covered. AC-1 is
unmet, and AC-4 and AC-5 are partly met; one of these gaps is the ruled
pre-check removal in this PR. One test is bound to the wrong AC, and the TC
text is stale. Not mergeable at this head.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-258-AC-1 is not met. Its only end-to-end test is `#[ignore]`d. The test that runs builds a `CheckedBody` by hand with `BodyBuilder` and never runs the checker, lowering or emitter, so it covers Behavior 2 only. The ignored test also falls short of the AC in three ways: (1) it builds forms with `ExpressionBuilder` rather than through S1 ("under S1 and S3 limits raised to fit them"); (2) it clones, compares and formats only the function's `CheckedBody`, not "the checked package"; (3) it drops the package without cloning it. Fix: keep QSL-483 open on AC-1 until the `Location` ruling and SR-1260 FND-002 land. Then un-ignore the test, run it from source through S1, and clone, compare and format the checked package. | qsl-package/src/emit/tests/deep_bodies.rs:59-108; qsl-semantics/src/check/ir.rs:913-936; spec/functional/FR-258-check-and-lower-expressions-at-any-depth.md:78 |
| FND-002 | medium | FR-258-AC-5's AC-1 half is not covered. The AC requires the stratum walk "across the checked packages of TC-415 and of AC-1". `every_lowered_body_is_in_the_stratified_grammar` walks only the TC-415 forms, at 2 and 1,000 levels. The module doc says the walk over the 100,000-deep packages "runs with step 1, where the package is emitted (`qsl-package`'s `emit::tests::deep_bodies`)", but that test runs no stratum walk at all, ignored or not. Fix: correct the doc, and add the stratum check to the AC-1 emit test (or a sibling) so it runs once that test is un-ignored. | qsl-semantics/src/check/lowering/tests/deep_bodies.rs:1-5; qsl-semantics/src/check/lowering/tests/deep_bodies.rs:79-100; qsl-package/src/emit/tests/deep_bodies.rs:59-108; spec/functional/FR-258-check-and-lower-expressions-at-any-depth.md:82 |
| FND-003 | medium | FR-258-AC-4 is partly met, through the library builder only. (a) No outcome names the setting `s3.nodes` or `s3.work_units`: `LimitExceeded` has no setting field, and nothing in the tree spells `s3.nodes` (ADR-030 D-7 slice 2). (b) The replay request's `stage_limits` entry and FR-255's settings operation are not exercised. (c) On the production path, `PackageDeclarations::check`'s per-declaration pre-check fires first. It refuses at the declaration span with count 1,999. The AC-4 test reaches the 1,501-at-the-node stop only because `check_sum` calls `ValueFunctionFamily::check` with `StageLimits::node_count = u64::MAX`. The pre-check is check/mod.rs:932-935 → family.rs:1486 → family/contract.rs:182-190. (d) The work-budget test asserts only the code and bound `w - 1`, not the count or locus. The work charge is one declaration-level meter charge, so its locus is the declaration span, not a node, and "the same holds" cannot hold for the locus. Ruled: the pre-check goes in this PR. When it goes, FR-258 Behavior 1 ("per-declaration stage limits SHALL be input bytes and node count"), FR-062-AC-5, FR-062-AC-12/TC-432 and the tests that pin the declaration-span locus (`package_checking_keeps_the_family_limit_region`, region.rs `a_checking_limit_stop_is_located_at_its_node`) change with it. Fix: per the ruling. Then run AC-4 through `PackageDeclarations::check`. Either assert the work stop's count and locus, or narrow AC-4's "same holds". Track (a) and (b) on the slice-2 ticket. | qsl-semantics/src/check/family.rs:3465-3556; qsl-semantics/src/check/mod.rs:932-935; qsl-semantics/src/check/family.rs:1486; qsl-semantics/src/family/contract.rs:182-190; qsl-semantics/src/check/family.rs:3669-3694; spec/functional/FR-258-check-and-lower-expressions-at-any-depth.md:39-44 |
| FND-004 | medium | `a_100000_deep_deciding_element_encodes_on_a_small_stack` is tagged `#[trace("TC-744", "FR-269-AC-3")]`. FR-269-AC-3 is a round-trip to an equal result, two refusals and an inequality on `derived`, and the test only encodes. No AC in FR-269 or FR-263, and no TC-744 step, says that a deep deciding element encodes. So `encode_element`'s depth property is underspecified code. Reading the same document back would also refuse at serde_json's recursion limit of 128 (ruled: the reader moves to `quire_canonical::read` after #613). Fix: add an FR-269 or FR-263 AC and a TC step for a deep deciding element that round-trips, once the reader moves, and retag the test to it. | qsl-replay/src/spine/clause/tests/witness.rs:1279-1341; spec/functional/FR-269-settle-a-witness-disagreement-as-a-typed-cause.md:67; spec/test-cases/TC-744-settle-a-witness-disagreement-as-a-typed-cause.md:21-27 |
| FND-005 | low | Spec text is stale after this PR. TC-381 still uses `CheckingLimits::new(nodes, depth)` and `new(4, 128)`, which no longer compile. The Status sections of TC-378, TC-413 and TC-415 say their tests "drive the depth limit ADR-030 deletes"; this PR deleted that limit and replaced those tests. TC-725, TC-726 and TC-727 still say "Planned". Fix: update them in this PR (spec-only edits). | spec/test-cases/TC-381-package-wide-expression-node-limit.md:14-24; spec/test-cases/TC-378-on-a-nested-fixture-the-node-limit-is-the-proximate-cause.md:44-48; spec/test-cases/TC-413-type-and-declared-node-keys-match-golden-vectors.md:121-123; spec/test-cases/TC-415-checked-expressions-lower-to-catalogued-fr-322-nodes.md:157-159 |
| FND-006 | low | FR-258-AC-2 says the record chain's text leaf path has "one segment per level". The fixture `chain(LEVELS)` gives each record an optional `next` field, so the path is `field:next`, `inner` per record, then `field:label`: 2×100,000+1 segments. The test asserts that count. Fix: reword AC-2 and TC-726 step 2 to that path shape, or use a chain of required fields. | qsl-semantics/src/check/lowering/tests/depth.rs:314-326; spec/functional/FR-258-check-and-lower-expressions-at-any-depth.md:79; spec/test-cases/TC-726-deep-types-lower-their-text-leaves-on-a-small-stack.md:24-26 |

## Dispositions

Round 1, reviewed at d3fe0a5a629876c960646cc7e1047be62c1ece09: the fix round
rebased on main fbf69cb9c, so this round reviews `git diff
origin/main...d3fe0a5`. I ran no build. The coder's `make ci` log ends
`exit=0`, and the AC-1 test passes in it in 446.84 s (debug, once per test
run).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 302494b2e. `a_100000_deep_body_checks_lowers_and_emits_on_a_small_stack` is no longer ignored. For a 100,000-term sum, an `else if` chain and nested `let`s, it parses source through S1 with raised limits, then S2 and the assembler. It checks with every S3 limit at `u64::MAX`, links and emits. It recomputes each written node's key from its body and compares it with `node_id`. It clones and compares the function's `CheckedBody` and the lowered graph, and formats and drops the package, all on a 512 KiB stack (qsl-package/src/emit/tests/deep_bodies.rs:65-129). The oracle is strong: the key check fails on any wrong body, and the stratum check fails on any nested composite. The AC's "the checked package clones" cannot hold as written (FND-009). |
| FND-002 | fixed | 302494b2e. The AC-1 test asserts `is_stratified_body` on every written node (deep_bodies.rs:90-94). The qsl-semantics module doc now says that walk runs there (lowering/tests/deep_bodies.rs:1-5). |
| FND-003 | fixed | b03a80050 and be774366e. The per-declaration node pre-check is gone: `check_node_count` is deleted, and FR-258 Behavior 1, FR-062-AC-11, FR-062-AC-12, TC-432 and FR-096's table are updated. AC-4 now runs through `PackageDeclarations::check`. The node stop asserts bound 1,500, count 1,501 and the `1` region. The work stop asserts bound w - 1, count w and the declaration span, and at w it finds a later lowering charge (family.rs:3394-3446). Parts (a) and (b) are deferred to B5 by owner ruling, and FR-258's Status says so. |
| FND-004 | fixed | 08de27836. FR-269-AC-4 and TC-744 step 4 now state that a deciding element 100,000 levels deep round-trips. `a_100000_deep_deciding_element_round_trips_on_a_small_stack` is tagged `#[trace("TC-744", "FR-269-AC-4")]`. It encodes an `Option` type 100,000 deep and a record chain 100,000 deep, reads each back through `quire_canonical::read`, and re-encodes it to the same text (witness.rs:1302-1372). |
| FND-005 | fixed | be774366e. TC-381 uses `CheckingLimits::new(nodes)`. TC-378, TC-413 and TC-415 no longer cite the deleted depth limit. TC-725, TC-726 and TC-727 name their backing tests. |
| FND-006 | fixed | be774366e. FR-258-AC-2 and TC-726 step 2 now say two segments per level (`field:next`, `inner`), ending `field:label`. |
| FND-007 | fixed | Round 2, aed7b2df0: 8eb0b9ff9. G18 now holds `Option<A>`: an `option` over `group_reference` ordinal 3 (record A) at recursion ordinal 0 of the size-4 group, with key `f9c72623…` (FR-093:480, 530-536). `recursive_text_leaf_vectors_check_and_key` finds G16 to G21 by what they declare and hold, never by their own key (`named_group_node`, leaves.rs:156-183). It compares each one's key and preimage with its vector. `assert_published_tables_agree` asserts that no two vectors share a key (tests.rs:141-148), so a copied vector now fails. |
| FND-008 | fixed | Round 2, aed7b2df0: 8eb0b9ff9. Headings, key-table descriptions and group tables now give the preimages' ordinals (FR-092 G2 ordinal 1 and G3 ordinal 0; FR-093 G16 1, G17 0, G18 0, G19 2, G20 3, G21 1). The signature columns are deleted except G1's, which a unit test asserts, and FR-092's prose says why. `fr_092_published_tables_agree_with_its_vectors` and `fr_093_published_tables_agree_with_its_vectors` check each key-table row against its heading and key, each heading's and each group table's ordinal against the preimage, each table's digest against the members' `recursion.group`, and that every grouped vector is in exactly one table. |
| FND-009 | fixed | Round 2, aed7b2df0: 732a93a8e. The fixture builds exactly 100,000 sum terms, `if`s and `let`s (deep_bodies.rs:25-38). FR-258-AC-1 now says every checked function body and the lowered semantic graph clone and compare equal, the package formats, and all of them drop. TC-725 step 1 already said that. |

## New findings (disposition pass 1)

These are in the golden vectors the five-strata change regenerated
(5bc131ebc, 5b7699280, e691247c5) and in the AC-1 wording.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | high | FR-093's G18 vector (`Option<A>`) is a copy of G21 (`Option<B>`). Both have the same preimage, `{"members":[{"ordinal":2,"term":"group_reference"}]}` at recursion ordinal 1, and the same key `bbe19be8…`, in the key table (FR-093:480, 483) and in the vectors (FR-093:530-536, 554-560). The group has size 4, record B's `a` names ordinal 0 (FR-093:541), and no published vector holds ordinal 0. So the real `Option<A>` node (ordinal 0, naming ordinal 3, record A) is neither published nor asserted, and FR-093 claims that two different types share one key. `recursive_text_leaf_vectors_check_and_key` still passes, because `assert_fr093` finds each vector's node by the vector's own key and compares that node's preimage (leaves.rs:21-29). Nothing checks that the 21 keys are distinct. Fix: regenerate G18 from the graph. Make the test fail on such a copy: assert that the vectors' keys are pairwise distinct, or find each node by its declared type and compare it with the vector, as FR-092's `assert_vector` does. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:480; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:530-536; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:554-560; qsl-semantics/src/check/lowering/tests/leaves.rs:21-29; qsl-semantics/src/check/lowering/tests/leaves.rs:154-177 |
| FND-008 | medium | The regenerated recursion groups are internally inconsistent. In all three, the five-strata shape flipped the ordinals inside the preimages: FR-092 G2 `List` is now ordinal 1 and G3 is 0; FR-093 G16 is 1 and G17 is 0; G19 is 2, G20 is 3 and G21 is 1. The vector headings (`**G2**: … ordinal 0`), the key-table descriptions and each group table's Ordinal column still give the old ordinals. The Anonymous signature and Full signature columns were not regenerated either, although each member's group-local preimage changed shape, and the flipped order shows that their signatures changed. e691247c5 updated only the group digest lines. No test reads these tables. Only G1's signatures are asserted, and G1 is unchanged (node_key/tests.rs:510-545). Fix: regenerate the headings, ordinals and signatures from `group_keys`, and assert them as the G1 test does. Or delete the signature tables, if nobody needs the intermediate values. | spec/functional/FR-092-key-type-parameter-and-declared-nodes.md:429-430; spec/functional/FR-092-key-type-parameter-and-declared-nodes.md:761; spec/functional/FR-092-key-type-parameter-and-declared-nodes.md:769; spec/functional/FR-092-key-type-parameter-and-declared-nodes.md:880-885; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:478-483; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:514-560; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:666-680 |
| FND-009 | low | FR-258-AC-1 says "The checked package clones, compares equal to its clone". `CheckedPackage` and `CheckedGraph` derive neither `Clone` nor `PartialEq` (qsl-package/src/checked.rs:132-133, qsl-semantics/src/check/mod.rs:304-305). The test clones and compares the function's `CheckedBody` and the lowered `SemanticGraph`, then formats and drops the package. That is the right check, but the AC promises more. The fixture also builds 99,999 `let`s and 99,999 `if`s (`for level in 1..DEEP`), not the AC's "100,000 nested `let`s". Fix: reword AC-1 and TC-725 step 1 to what is checked (every checked body and the lowered graph clone and compare equal to their clones, and the package formats and drops). Make the counts agree. | spec/functional/FR-258-check-and-lower-expressions-at-any-depth.md:80; qsl-package/src/emit/tests/deep_bodies.rs:25-37; qsl-package/src/emit/tests/deep_bodies.rs:101-123 |
