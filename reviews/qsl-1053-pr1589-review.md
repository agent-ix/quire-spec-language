---
id: SR-1589001
title: "Rust and gap review of PR #1589: TC-774 plugin proof trusted and Kani certified (QSL-1053)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@f8997d1fdab969b0dd098a54aadb8419b459379d; PR #1589 (branch qsl-1053-fr290-record-test); spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md (AC-1); spec/functional/FR-289-keep-results-unchanged-by-provider-installation.md (AC-10); spec/test-cases/TC-774-*.md; qsl-replay/src/proof_result/plugin_tests.rs"
review_set: subset
date: '2026-10-25'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-774
    type: reviews
---
## Summary

Ticket: QSL-1053. One test, no production change. Plan completion: not assessed.

Checks run: `cargo test -p qsl-replay --lib tc_774_plugin_proof_settles` passes,
`cargo clippy -p qsl-replay --all-targets -- -D warnings` clean, `xtask string-edge` clean.

Tag honesty: FR-290-AC-1's row names TC-774 and TC-774 step 1 lists all four
records, so `#[trace("TC-774")]` is the right id. Omitting `FR-290-AC-1` is
honest: records 2 (FR-281-AC-1 EN-5 certified, QSL-574/575) and 3 (FR-281-AC-4
uncertified engine with no checker) are not asserted. The doc comment says so.

Mutation results (production edited, test rerun, reverted):

- `from_plugin_proof` bounded-solver refusal dropped: test FAILS (discriminates).
- `TerminalRecord::proved` certification changed to Trusted: test FAILS.
- `read_plugin_certification` certified/uncertified arm disabled: test PASSES
  (surviving mutant, FND-001).

C-09 stand-in: `TerminalRecord::proved` (Checks basis, Certified) is the only
Kani-shaped constructor on main; no Kani adapter exists to call. It is a
faithful stand-in for the record shape, but it asserts the constructor, not a
C-09 map.

Duplication (the #1447 objection): the trusted settle with the plugin
`BackendId` and the bounded-solver refusal on the non-hybrid entry are already
asserted by `plugin_bases_read_and_settle_by_item_kind`
(hybrid_replay/tests.rs, tagged FR-290-AC-5) and
`plugin_proof_is_trusted_and_names_the_plugin` (terminal_record_facade.rs,
FR-289-AC-10). Kani `Certified` is asserted at terminal_record_facade.rs:197.
What this test adds: one place that reads the FR-331 entry through
certification then basis then settle, with an `inductive-solver` basis, and the
Kani record beside the plugin record as TC-774 step 1 words it.

## Findings

| id | severity | finding | location |
|----|----------|---------|----------|
| FND-001 | medium | The test calls `read_plugin_certification(entry.get("certification")).unwrap()` only on `"trusted"`. The `Trusted` in the asserted record is hard-coded by `trusted_proof`, not read from the entry. Making the reader accept `certified` (or everything) leaves the test green. The doc comment says the entry "reads through the FR-331 readers (`certification` first)", which the assertions do not prove. Fix: also assert `read_plugin_certification(Some(&json!("certified")))` is `Err(CertificationNotTrusted{received:"certified"})`, or drop the call and the claim. | plugin_tests.rs:1163 |
| FND-002 | low | Most assertions repeat existing tagged tests (see Summary). Not tag inflation, since the test carries no AC tag and the only added value is the combined step-1 shape. Accepting this is a code-lead call. | plugin_tests.rs:1146-1202 |
| FND-003 | low | FR-290-AC-1 stays untagged until QSL-574/575 land and the FR-281-AC-4 engine exists. Needs a named follow-up so the gap is not lost. | spec FR-290-AC-1 |

## Verdict

PASS with findings. No blocker. FND-001 should be fixed in this PR (one added
assert) so the doc comment is true.
