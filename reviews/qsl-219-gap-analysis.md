---
id: SR-1268
title: "QSL-219 gap analysis of PR #617 (FR-056-AC-13, FR-056-AC-14, FR-071-AC-11, FR-106-AC-11, TC-145, TC-186, TC-465)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@91a1607feb9a65dfa70e01dd66b53c6baa24ea26; PR #617 diff 9e035e7fa...91a1607f; spec/functional/FR-056-admit-domain-package-model-declarations.md; spec/functional/FR-071-implement-typed-replay-request.md; spec/functional/FR-106-admit-snapshots-and-invocations.md; spec/test-cases/TC-145-admit-ir-2-domain-package.md; spec/test-cases/TC-186-replay-request-digest-only-byte-provision.md; spec/test-cases/TC-465-admission-refuses-each-input-defect.md; spec/model-linking/tests.md; spec/tests.md; context: spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md (I2 reader section)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
---
## Summary

Ticket: QSL-219. PR: quire-spec-language#617, head 91a1607f. This is the quoin
gap-analysis method with no plan bundle (QSL-219 has none), scoped to the
brief's criteria and test cases, plus a check of each test's oracle. Code
findings are in SR-1267.

Trace, per unit:

- **FR-056-AC-13 / TC-145.** `refuses_a_whole_number_beyond_2_53_at_its_pointer`
  covers each listed spelling at `/package/count`, the escaped pointer
  `/a~1b/0/c~0d`, and first-in-document-order (`/b` over `/a/0`, which also
  proves members are walked in document order, not RFC 8785 order).
  `admits_2_53_its_negation_and_every_exact_number` covers ±2^53.
  `documents_differing_only_in_an_inexact_number_share_no_digest` hard-codes
  the canonical text that `18446744073709551615` and `18446744073709551616`
  used to share and asserts the exact refusal under it.
  `a_u64_bound_above_2_53_refuses_at_its_pointer_before_check_3` covers the
  2^60 + 1 bound under the old digest and the raw digest. Every clause is
  covered, and every assertion checks the code, the cause and the pointer
  against literals.
- **FR-056-AC-14 / TC-145.** `refuses_a_number_that_is_not_its_doubles_shortest_text`
  covers each listed value, precedence (`9007199254740993` and `1e20` refuse
  `inexact-integer`) and `/a/1`. The admit list and the `0.1` digest clause
  are covered by the two tests above. The binding is correct, but the AC has
  no case where a double has two shortest texts (FND-001).
- **FR-056-AC-2.** Removing the big-integer clause matches the new rule, and
  the clause now lives, reversed, in AC-13.
- **FR-071-AC-11 / TC-186 step 9.** `a_package_document_refusal_keeps_its_cause`
  decodes both entries and asserts the variant, cause, pointer, code and
  message prefix. Correct.
- **FR-106-AC-11 / TC-465 row 45.** Four `state_clauses` tests, one per
  member. Each asserts code, cause and `document_pointer`, then shows the exact
  replacement admits. The tests fail if check 1.3's number rule is removed,
  because a precondition does not otherwise read those members. FR-106-AC-9's
  amendment ("Beyond check 1.3's number rule") agrees with AC-11.
- **FR-056 statement.** It matches the code: the text-only integer decision,
  precedence, `document_pointer` escaping, refusal before the digest and
  before check 3, and every number at any depth. One term is ambiguous
  (FND-001). The QSpec FR-271/FR-272 citations match the contract.

The reverse trace (code with no owning statement) finds two gaps: the
tree-holds-the-digest's-double fix (FND-002) and the I2 reader's new code
(FND-003). No stub, placeholder return or hollow implementation is in the
diff.

## Verdict

**CONDITIONAL**: one medium finding and two low findings. Every scoped
criterion and test case is backed by a tagged test that passes. The medium
finding is the spec gap behind SR-1267 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-056 calls a number exact when its value equals "the shortest round-trip text of its nearest double". A double can have two shortest round-trip texts, and the statement does not say which one RFC 8785 writes (ECMAScript `Number::toString`: the closest, then the even digit). FR-056-AC-14 names no such number, so TC-145 cannot catch an implementation that picks the other one, which is what SR-1267 FND-001 found. Name the rule in the statement, and add a case to AC-14 and TC-145, for example `1500000000000000.2` admitted and `1500000000000000.3` refused `inexact-number`. | FR-056-AC-14, spec/functional/FR-056-admit-domain-package-model-declarations.md:117-121, spec/functional/FR-056-admit-domain-package-model-declarations.md:376 |
| FND-002 | low | `a_non_whole_number_in_the_tree_is_the_digests_double` is tagged FR-056-AC-2, but AC-2 says nothing about which double the tree holds. The behaviour it tests, declarations reading the double the digest encodes, is in FR-056's statement ("the digest spells the value the declarations read"), and no AC states it. Add it to an AC (for example AC-14: a document holding `1.5e-300` admits and its tree holds the double `1.5e-300` parses to) and retag the test. | qsl-semantics/src/model/intake.rs:5845-5847 |
| FND-003 | low | The I2 reader now maps IR's `noncanonical_wire` to `Code::NoncanonicalWire` instead of `Code::InvalidPackage`, so callers see a different code. The FR-096 section "The I2 reader locates its refusals in the artifact" names only `unknown_wire`, so no QSL statement or AC owns the new code. `maps_noncanonical_bytes_to_the_native_noncanonical_wire_code` has no trace tag. State the mapping in FR-096, add an AC for it, and tag the test. | qsl-package/src/checked_v2.rs:489, qsl-package/src/checked_v2/tests.rs:608 |

## Coverage

- Reconciliation: quire coverage (quire 0.33.0, engine 0.47.1, module
  spec-artifacts-process v0.26.0).
- Tasks done: not applicable. QSL-219 has no plan bundle under `plan/`.
- Scoped rows backed by a tagged test: 7 / 7 (FR-056-AC-13, FR-056-AC-14,
  FR-071-AC-11, FR-106-AC-11, TC-145, TC-186, TC-465). FR-056 as a whole is
  12 / 14; FR-056-AC-7 and FR-056-AC-11 were unbacked before this PR and are
  outside its scope. Repository total: 1208 / 2522.
- Untraced behaviours: 2 (FND-002, FND-003). Stubs: 0.
- Semantic review: ran over the four scoped criteria and their seven tests
  (oracle strength included). The weak oracle in
  `big_integers_admit_under_neither_their_double_nor_their_digits` is SR-1267
  FND-003. The float-fix test's oracle was confirmed to fail without the fix
  on `1.5e-300` (SR-1267).
- Not run: `/spec-review` sub-analyses. The brief limited this review to
  code-review, rust-review and gap-analysis. The statements were read here for
  agreement with the code, not for EARS form.

## Dispositions

Round 1, reviewed at 7d2d36551295eb197932458c24407c464afbc320 (fix commit
7d2d36551 on 7fe95cce9, which is 91a1607f rebased unchanged onto main
a7017bea2). This round adds no gap-analysis finding of its own. The
FR-106-AC-12 change the fix round added is SR-1267 FND-004.

- **FND-001.** The FR-056 statement now names the text RFC 8785 writes:
  ECMAScript `Number::toString`'s shortest round-trip text, with the even
  last digit on a tie, taken from quire-canonical and no other formatter.
  AC-14 and TC-145 carry the three even-digit and three odd-digit cases, and
  `a_tie_between_two_shortest_texts_admits_only_the_even_digit` backs them.
- **FND-002.** New FR-056-AC-15 states the derived-view double.
  `a_non_whole_number_in_the_tree_is_the_digests_double` is retagged
  `("TC-145", "FR-056-AC-15")`. TC-145's scope and procedure and the
  model-linking matrix carry AC-15.
- **FND-003.** FR-096's I2 section now says the reader refuses with
  `noncanonical_wire`, not `invalid_package`. New FR-096-AC-18 and TC-429
  step 5 cover it, and the test is tagged `("TC-429", "FR-096-AC-18")`.
- **Coverage.** quire coverage at this head shows FR-056-AC-13, AC-14 and
  AC-15, FR-071-AC-11, FR-096-AC-18, FR-106-AC-11 and AC-12, TC-145, TC-186,
  TC-429 and TC-465 all backed, with no untracked symbol among them
  (repository total 1213 / 2527).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 7d2d36551 |
| FND-002 | fixed | 7d2d36551 |
| FND-003 | fixed | 7d2d36551 |
