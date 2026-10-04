---
id: SR-1272
title: "QSL-381 B6 gap analysis of PR #623 (FR-260-AC-1, TC-730 trace tags)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@1a257626bbd0b7111971eea2c7ee378193ea6154; PR #623 diff against origin/main: qsl-semantics/src/model/intake.rs tests; spec/functional/FR-260-admit-semantic-ir-documents-at-any-depth.md; spec/test-cases/TC-730-intake-judges-a-deep-document-on-its-content.md; spec/tests.md; spec/functional/FR-259-encode-identities-and-read-json-through-quire-canonical-at-any-depth.md; spec/functional/FR-356-walk-nested-structures-through-one-iterative-walker-toolkit.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-260
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-730
    type: reviews
---
## Summary

Ticket: QSL-381 (slice B6). PR: quire-spec-language#623. This is a manual
check of the PR's acceptance criteria against its tests. It covers the trace
tags of the three new tests and the spec text they cite.

**FR-260-AC-1 is an end-to-end intake criterion.** On a 512 KiB thread, a
package document with valid declarations holds an array nested 100,000 deep
at a member the semantic-IR schema does not admit. Intake must refuse it by
FR-056's reader-refusal rule. It must end with no declaration and keep the
semantic-IR reader's diagnostic (IR node, artifact id, span). It must have no
`resource_exhausted` cause, and no outcome may name a depth. The second half
of AC-1 is that every corpus package FR-056 admits is still admitted with
the same digest. TC-730 step 1 is the procedure for the first half and
step 2 for the second.

**None of the three new tests does any of that.** They call the private
builders `value_of`/`json_of` on a bare `quire_canonical::read` tree, or
`PackageDocument::parse` at depth 199. None runs admission, holds a package
document with declarations, reaches a semantic-IR diagnostic, or checks
refusal causes. At 1a257626b, intake still refuses any document 200 or more
deep as `intake-limit-exceeded` naming `NestingDepth`. That is the exact
outcome AC-1 forbids, so AC-1 is not met, and `too_deep` keeps it unmet
until AGE-2224 (out of scope by ruling). The tags tell the trace matrix that
an unmet AC is verified. Their doc comments call the tests "TC-730 step 1",
which they are not.

**What the tests do verify has no requirement.** No FR-260 behavior, and no
AC in FR-259, FR-260 or FR-356, says that intake's derived views build, or
that its `serde_json` view drops, without a native frame per level. FR-259
covers the reader and digest. FR-356-AC-2 covers the toolkit itself, not
this walk. So the honest fix is to add the requirement and retag, not to
pick a nearby AC.

Clean bindings: `tc_730_a_lone_low_surrogate_refuses_at_its_byte_offset`
binds FR-260-AC-2. It is not in the diff and was read for context only.

## Verdict

Not mergeable yet: there are three trace findings, all high. Each applies
the same fix:

1. Add FR-260 Behavior item 7, **Derived views**: "Intake SHALL build its
   `serde_json` view and its semantic-IR `Json` view from the reader tree,
   and SHALL drop its `serde_json` view, with native stack use that does not
   grow with the document's depth."
2. Add FR-260-AC-5 for that behavior: "On a thread with a 512 KiB stack,
   intake's builders turn a reader tree nested 100,000 deep (arrays, and
   objects) into both views, and a `PackageDocument` holding a 100,000-deep
   `serde_json` view drops. A view build whose leaf conversion fails after
   a 100,000-deep sibling has finished returns that failure. A shallow document builds the `serde_json` tree `serde_json`
   reads from the same text, and a `Json` view with the members in document
   order and each number by its lexeme." Verification: Test (TC-730).
3. Add TC-730 step 4 for AC-5. Add FR-260-AC-5 to TC-730's Scope line, its
   tag list, and its `spec/tests.md` row (line 527). Note that step 4 is
   implemented in the Status.
4. Retag the three tests `#[trace("TC-730", "FR-260-AC-5")]` and change the
   doc comments from "TC-730 step 1" to "TC-730 step 4". Then FR-260-AC-1
   has no binding until AGE-2224, which is true.

The FND-002 and FND-003 fixes in SR-1271 (a real 100,000-deep
`PackageDocument` drop, and the error-path test) are what make the AC-5
clauses above testable. Write those tests under the same tag.

The minimal alternative is to remove the three `#[trace]` attributes and keep
the tests untagged. 11 of the 86 tests in intake.rs are already untagged. It
is honest, but it leaves real behavior without a requirement, so step 1-4 is
the recommended fix.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `tc_730_a_document_nested_100000_deep_builds_and_drops_on_a_small_stack` is tagged `#[trace("TC-730", "FR-260-AC-1")]` and documented as "TC-730 step 1". It runs only the private builders on a bare reader tree. It never runs intake, has no declarations or semantic-IR diagnostic, and checks no refusal cause. FR-260-AC-1 is an end-to-end refusal criterion that intake still fails at 1a257626b (`too_deep` names `NestingDepth`). Fix: add FR-260 Behavior 7 and FR-260-AC-5 (derived views build and drop at any depth) and TC-730 step 4, as in the Verdict. Retag `#[trace("TC-730", "FR-260-AC-5")]` and call it step 4. | qsl-semantics/src/model/intake.rs:6432-6446 |
| FND-002 | high | `tc_730_a_parsed_package_document_drops_on_a_small_stack` is tagged `#[trace("TC-730", "FR-260-AC-1")]` and documented as "TC-730 step 1". It parses and drops a 199-deep document. It verifies no clause of FR-260-AC-1, and as written it verifies nothing (SR-1271 FND-002). Fix: rewrite it as SR-1271 FND-002 says, and tag it `#[trace("TC-730", "FR-260-AC-5")]` under the new step 4. | qsl-semantics/src/model/intake.rs:6447-6466 |
| FND-003 | high | `tc_730_a_shallow_document_builds_the_same_trees` is tagged `#[trace("TC-730", "FR-260-AC-1")]`. It checks that the builders' trees for a shallow document match `serde_json` and an expected `Json`. That is builder correctness, which no AC states. Neither half of FR-260-AC-1 is about it: the corpus-admission half needs FR-056's corpus admitted with unchanged digests. Fix: tag it `#[trace("TC-730", "FR-260-AC-5")]` under the new AC-5's shallow-equivalence clause. | qsl-semantics/src/model/intake.rs:6468-6507 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | TC-730's Status now reads "Step 4 is implemented", but its `spec/tests.md` row still reads `🚧 Planned`. TC-728 and TC-729 use `🚧 Planned (step N implemented)`, so the TC-730 row disagrees with its own file. Fix: change the row's status to `🚧 Planned (step 4 implemented)`. | spec/tests.md:527 |

## Dispositions

Round 1, reviewed at b38a14d6c21d3c4851fafe08e3b7a76ebaa79b1d (fix commit
b38a14d6c on 1a257626b). Per the ruling, FR-260 gains Behavior 7 and
FR-260-AC-5, and TC-730 gains step 4. Its Scope, tag list and `spec/tests.md`
row add FR-260-AC-5, and the four step-4 tests are tagged
`#[trace("TC-730", "FR-260-AC-5")]`. No test binds FR-260-AC-1 now, and
TC-730's Status says steps 1 and 2 wait on AGE-2224. `quire validate` on
FR-260 and TC-730 is clean (warnings only). FND-004 above is new this round.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | b38a14d6c |
| FND-002 | fixed | b38a14d6c |
| FND-003 | fixed | b38a14d6c |
