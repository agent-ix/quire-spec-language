---
id: SR-2441
title: "Gap analysis of quire-spec-language PR #652: FR-260 and FR-056 intake at any depth (QSL-487)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@0552dd1f2fc86ae544feb58daf9f2d5670ef3422; PR #652 diff origin/main...HEAD; FR-260-AC-1..AC-5, FR-056-AC-2, FR-056-AC-16, FR-255 intake.input_bytes row; TC-730, TC-731, TC-732, TC-145, TC-186, TC-911; quoin matrix at the reviewed head"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-260
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
---
# Gap analysis of quire-spec-language PR #652

## Summary

Ticket: QSL-487 (B6). Planless gap analysis. Plan completion: not assessed.
The ticket's acceptance asks that the tests in this PR back FR-260 and FR-056.

`quoin matrix --repo .` at 0552dd1f2: FR-260-AC-2, AC-4 and AC-5 are tagged.
**FR-260-AC-1 and FR-260-AC-3 are untagged.** FR-056-AC-2 and AC-16 are tagged.
FR-255-AC-1..AC-6 are tagged.

Examined:
- FR-260-AC-1 (examined): "On a thread with a 512 KiB stack, a package document whose declarations are valid and which holds an array nested 100,000 deep at a member the semantic-IR schema does not admit, with `intake.input_bytes` raised to fit, is refused by FR-056's reader-refusal rule: intake ends with no declaration and retains the `agent-ix-semantic-ir` reader's diagnostic for that member ... and no intake outcome names a depth. Every corpus package FR-056's tests admit is admitted, with the same `sha256-jcs` digest it had."
- FR-260-AC-3 (examined): "A package whose composite types form a cycle of 300 types, and one whose composite types form a cycle of 100,000 types with `intake.input_bytes` raised to fit, are each refused with semantic-IR's composite-cycle refusal naming every type on the cycle, on a thread with a 512 KiB stack."
- FR-260-AC-4 (examined): backed by `refuses_an_oversize_document_with_its_correct_digest_as_a_size_limit`, which covers bound B, B + 1, the setting, and raising the bound through the builder and through `parse_operands`. The `--limit` clause belongs to the driver CLI.
- FR-260-AC-5 (examined): backed by the existing TC-730 step 4 tests and, in effect, by the new 100,000-deep parse-and-drop test.
- FR-056-AC-2 (examined): its limit clause ("bytes intake refuses at one of its limits refuse `resource_exhausted`/`intake-limit-exceeded`, naming that limit and its bound") is asserted by the oversize test, which is now tagged only to FR-260-AC-4.
- FR-056-AC-16 (examined): `a_wide_bound_is_a_decimal_string_and_a_wide_json_number_is_inexact` now runs `read_records` over the string-bound `Wide`. It is correct and tagged TC-911.
- FR-255 `intake.input_bytes` row (examined): "(pending QSL-487)" is removed and the table tests read the row as a real setting.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-260-AC-1 has no test. This is the AC that proves the deleted depth cap is safe end to end. The new 100,000-deep test only parses and drops (`PackageDocument::parse`). It never runs `admit`, `read_records` or semantic-IR's `decide`, which is the recursion the cap protected (ticket comment, PR body). The cap is gone, and nothing at this head shows that a 100,000-deep, schema-refused member reaches FR-056's reader-refusal rule on a 512 KiB stack instead of overflowing. The only `read_records` depth test is 150 deep. TC-730's Status still says steps 1 and 2 wait on AGE-2224, and this PR lands AGE-2224 through the FCD bump. Fix: add the TC-730 step 1 test in this PR. On a 512 KiB thread, with `intake.input_bytes` raised to fit, a valid package with a 100,000-deep array at a member the schema does not admit goes through `admit` and `read_records`. Assert no declaration, the reader's diagnostic for that member, and no `resource_exhausted`. Tag it `#[trace("TC-730", "FR-260-AC-1")]`. Add step 2 (corpus digests unchanged) under the same tag. | spec/functional/FR-260-admit-semantic-ir-documents-at-any-depth.md (AC-1); qsl-semantics/src/model/intake.rs:6288-6308; spec/test-cases/TC-730-intake-judges-a-deep-document-on-its-content.md:53-55 |
| FND-002 | medium | FR-260-AC-3 has no test, and TC-731 is "Planned". It needs a 300-type and a 100,000-type composite cycle, each refused with semantic-IR's cycle refusal on a 512 KiB stack. The FCD bump this PR lands is what makes the 100,000 case possible, and the ticket asks that FR-260 be backed. Fix: add the TC-731 test (`#[trace("TC-731", "FR-260-AC-3")]`) through `read_records`. If the lead rules it is out of scope for B6, defer it to a named ticket. | spec/functional/FR-260-admit-semantic-ir-documents-at-any-depth.md (AC-3); spec/test-cases/TC-731-intake-reports-composite-cycles-of-any-length.md |
| FND-003 | low | Trace. `a_package_document_nested_100000_deep_is_read_not_refused_as_a_depth` is tagged `#[trace("TC-145", "FR-056-AC-2")]`, but FR-056-AC-2 has no depth clause. What the test asserts, a 100,000-deep document reading and dropping on 512 KiB, is FR-260-AC-5 (TC-730 step 4), and its own doc comment says so. FR-056-AC-2's limit clause meanwhile lost its tag when the oversize test was retagged to TC-732 only. Fix: retag the deep test `#[trace("TC-730", "FR-260-AC-5")]`, and add `#[trace("TC-145", "FR-056-AC-2")]` back to the oversize test next to its TC-732 tag. | qsl-semantics/src/model/intake.rs:6291; qsl-semantics/src/model/intake.rs:6314 |
| FND-004 | low | Stale test-case text. TC-730 Status (lines 53-55) still says intake refuses a document nested 200 or more deep until AGE-2224, which is false at this head. TC-732 Status (line 35) says "Planned", but its test now exists and passes. TC-186 step 9 (line 67) still prescribes "arrays nested 1,000 deep" for the intake-limit case, which no longer refuses at all, while `a_package_document_refusal_keeps_its_cause` now uses a byte-limit case. Fix: update the three files in this PR. | spec/test-cases/TC-730-intake-judges-a-deep-document-on-its-content.md:53-55; spec/test-cases/TC-732-intake-byte-limit-names-its-setting.md:35; spec/test-cases/TC-186-replay-request-digest-only-byte-provision.md:67 |

## Verdict

One high, one medium, two lows. The code changes match the ticket. FR-260-AC-2,
AC-4 and AC-5 and FR-056-AC-16 are backed. The slice's headline criterion,
FR-260-AC-1 (deep, schema-refused intake judged on content through the decide
path), is untested, although this PR deletes the cap that guarded that path. Not
mergeable until FND-001 is fixed in this PR. FND-002 needs a test here or a named
deferral.

## New findings (disposition pass 1)

Reviewed at be49d0e576454f5620fc611e037316bf255aedd0.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | FR-260-AC-3's text was changed in this PR (55b92bde5) from "naming every type on the cycle" to "naming the relationship that closes the cycle", to match semantic-IR's refusal. The new TC-731 test asserts only that some refusal's detail contains `COMPOSITE_CYCLE`. It checks neither naming clause, old or new, so the AC's naming clause has no oracle. The AC edit is also a spec change, which no spec review has checked. Fix: assert that the refusal names the closing relationship, `ix://acme/orders/relationship/C{count-1}-has-C0` (or the identity semantic-IR names), for both 300 and 100,000. Run spec review on the FR-260-AC-3 edit, or have the spec owner confirm it. | spec/functional/FR-260-admit-semantic-ir-documents-at-any-depth.md:65; qsl-semantics/tests/it/model_intake.rs:1048-1090 |
| FND-006 | low | TC-730's Status now says "Steps 1 to 4 are implemented". Step 2 ("Admit every corpus package FR-056's tests admit, and read each digest") has no test tagged TC-730/FR-260-AC-1. The step 1 test's oracle also checks only that some refusal's detail contains "agent-ix-semantic-ir refused". The AC also requires the retained diagnostic to name the member's IR node, artifact id and span, and nothing asserts that. Fix: assert the diagnostic's node, artifact id and span for the `bogus` member in `a_deep_member_the_schema_refuses_is_judged_by_the_readers_rule`. Either tag an existing corpus-digest test for step 2 or reword the Status. | spec/test-cases/TC-730-intake-judges-a-deep-document-on-its-content.md:25,53; qsl-semantics/tests/it/model_intake.rs:985-1040 |

## Dispositions

Round 1, reviewed at be49d0e576454f5620fc611e037316bf255aedd0 (fix commits 9f771b93..be49d0e57).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 4b1e37f63. `a_deep_member_the_schema_refuses_is_judged_by_the_readers_rule` (model_intake.rs:985), tagged TC-730/FR-260-AC-1, runs on a 512 KiB thread. A valid package holds a 100,000-deep array at a schema-refused member. `PackageDocument::parse` reads it, and `read_records` runs `validate_with_semantic_ir`, which calls `agent_ix_semantic_ir::decide` on the deep bundle (intake.rs:1389). The test asserts refusals only, every one `IntakeMalformedDeclaration`, none `resource_exhausted`, then drops the document on the same thread. Oracle gaps are recorded as FND-006. |
| FND-002 | fixed | 4b1e37f63 (fixture 688e28a4f, be49d0e57). `a_composite_cycle_of_any_length_is_refused_by_the_readers_rule` (model_intake.rs:1048), tagged TC-731/FR-260-AC-3, builds 300-type and 100,000-type composite cycles on a 512 KiB thread and asserts a `COMPOSITE_CYCLE` refusal through `read_records`. The naming-clause gap and the AC edit are recorded as FND-005. |
| FND-003 | fixed | 4b1e37f63. The deep test is retagged `#[trace("TC-730", "FR-260-AC-5")]` (intake.rs:6300). The oversize test carries `#[trace("TC-145", "FR-056-AC-2")]` again next to TC-732 (intake.rs:6323-6324). |
| FND-004 | fixed | 55b92bde5. TC-730 Status, TC-732 Status ("Implemented") and TC-186 step 9 (an entry longer than the request's `intake.input_bytes` (4), refusing with bound 4 and its size) are updated. TC-730's new Status overclaims step 2, recorded as FND-006. |

Round 2, reviewed at 3ebac1553ddf7ef1a6b0d1a1ba3c48dfde439208 (fix commits be49d0e57..3ebac1553).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-005 | fixed | acb417817. The TC-731 test now requires a `COMPOSITE_CYCLE` refusal whose `IntakeMalformedDeclaration.node` equals `ix://acme/orders/relationship/C{count-1}-has-C0`, the back edge, for both 300 and 100,000 types. The plan lead signed off the FR-260-AC-3 rewording ("naming the relationship that closes the cycle"). |
| FND-006 | still-open | acb417817 fixes the step 1 oracle: the AC-1 test now requires `IntakeMalformedDeclaration { node: Widget, artifact: Some("ix://acme/orders/spec"), span: Some(3:5) }` alongside "agent-ix-semantic-ir refused". The TC-730/731/732 Status sections are deleted. Step 2 is still not backed. The newly tagged `domain_package_digest_matches_its_golden_vector` admits one minimal golden document (`test/golden`, `"types": []`) under its pinned digest, and does not run on a 512 KiB thread. TC-730 step 2 asks to admit every corpus package FR-056's tests admit and to read each digest, expecting each to keep the `sha256-jcs` digest it had. One empty-model document shows that the digest rule is unchanged. It does not show that the corpus packages (spine model, architecture bundle, QSpec-conformant document, config-version) still admit under their digests. Remaining fix, either one: (a) add a TC-730/FR-260-AC-1 test that runs `package_input` + `admit` over the corpus documents those tests use and asserts their pinned `sha256-jcs` digests; or (b) have the spec owner narrow TC-730 step 2 and FR-260-AC-1's last sentence to a golden-vector check. |

Round 3, reviewed at f02c56f14c9267c11aee77a5d9e17f2e87d95cda (fix commit 3ebac1553..f02c56f14).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-006 | still-open | f02c56f14 adds `every_corpus_package_admits_under_its_recorded_digest` (identity_golden_vectors.rs:380), tagged TC-730/FR-260-AC-1, and drops the TC-730 tag from the golden-vector test. What it does is sound. It admits `tests/fixtures/spine-model.semantic-ir.json` (acme/orders) and `systems-interface.semantic-ir.json` (acme/systems) through `admit` under hard-coded `sha256-jcs` digests. `admit` refuses a recomputed digest that differs (intake.rs:1048-1080), so `is_ok()` checks the digest. The digests are independent: RFC 8785 text computed in Python gives 5fc327ab…5bfd and 47b316ec…cf72. Raw-byte SHA-256 differs (8c447b1f…, 07ef1029…), so the JCS rule is what passes. 5fc327ab… is also the digest `tests/fixtures/spine-model.native:7` already selects, so it is the digest the package had. It does not meet TC-730 step 2 as written. Step 2 says "Admit every corpus package FR-056's tests admit". That set is defined by FR-056's tests, not by the files under `tests/fixtures`. FR-056-tagged tests also admit FCD's architecture document `agent-ix/architecture`, which `lifts_the_architecture_bundle_and_admits_it` (model_intake.rs:111) and TC-148's composed_domain_models.rs:405 and :474 lift from the pinned extraction-frontend checkout and admit. The test at :111 asserts that the lifted document equals the committed `architecture/expected/semantic-ir.json`. Its digest, computed outside the crate at FCD rev 68ace48, is aed36197…8ee0. The new test omits it. `acme/systems` is not admitted by any FR-056 test; FR-093's admission corpus admits it. Including it does no harm. In-test documents (`test/plant`, the TC-147 `acme/orders` envelope, `test/golden`) are built in test code, so they are not corpus packages. The round-2 list wrongly included config-version: no FR-056 test admits it. The step also still does not run on a 512 KiB thread, which TC-730's procedure requires for every step that does not opt out. Remaining fix: add the architecture expected document, with its pinned digest, to the corpus array; run the loop on a 512 KiB-stack thread; and either drop the unused name element of each tuple or use it in the assertion message. |
