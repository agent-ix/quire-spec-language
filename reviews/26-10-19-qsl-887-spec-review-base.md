---
id: SR-13075
title: "Base spec review of PR #1023: FR-147/FR-135/TC-552 against the merged QSpec internal-list rules"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@54b78f5f7; git diff origin/main...HEAD: ADR-020 RM-9/QS-11, FR-135, FR-147, FR-310, TC-552"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-552
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: reviews
---

## Summary

Ticket: QSL-887. PR quire-spec-language#1023 at head 54b78f5f7, one commit,
five files. Compared against merged QSpec (agent-ix/quire-specification
origin/main 2ec4f1d, STD-199 #285): FR-375 Internal list, Unbuilt features,
Checked-package node and AC-12 to AC-14; FR-432 internal-list section and
AC-7 to AC-10; TC-310 mutations 14 to 17; TC-386 IL-01 to IL-04. Review set:
subset (base, integrity, ears-conformance, failure-domain, scope-boundary,
dependency). `quire validate` exits 0 on all five changed files.

Agreeing with QSpec: the class spelling table, the alias-free node spelling
and `abstract_internal` order (FR-375's rule), absent-is-empty, the
one-class-one-decision conflict, the (input, pointer) locus and the
whole-compilation refusal for a library value, the resolve-first precedence,
the not-yet-built locus at the first token after `abstract`, and the
two-refusal case under `abstract Spec::Twice`. Existing FR-147-AC-1 to AC-7
keep their ids; AC-8 to AC-11 are appended, so PR #970's citations stay valid.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-147-AC-1 | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147-AC-5 | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147-AC-6 | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147-AC-7 | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147-AC-8 | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147-AC-9 | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147-AC-10 | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147-AC-11 | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-135-AC-5 | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| TC-552 | spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| QSpec FR-432-AC-7..10, TC-386 IL-01..04 | quire-specification spec/functional/protocol/FR-432-map-every-protocol-step-in-refinements.md | context_only |
| QSpec FR-375-AC-12..14, TC-310 mutations 14..17 | quire-specification spec/functional/analysis/FR-375-declare-model-refinement.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-147-AC-6 locates `"refinement": 7` at `/0`. QSpec locates the same defect at `/0/refinement` (TC-386 IL-03 `[{"refinement": 7, "internal": []}]` at `/0/refinement`; TC-310 mutation 17 "the number `malformed-declaration` at `/0/refinement`"), and FR-147's own rule is the pointer of the offending value, which is the `refinement` member. A build that follows AC-6 fails both QSpec vectors. Change it to `/0/refinement`. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:253 |
| FND-002 | medium | TC-552 Expected Results were not updated to the alias-free spelling: Step 1 still expects `abstract_internal = [finish]` and Step 5 expects `[finish]` "ordered by the bytes of the canonical spellings". FR-147-AC-1 and AC-5 now say `[Twice::finish]` and the byte order of the alias-free spellings; FR-147 calls the alias-including spelling the canonical one, so Step 5 states the order QSpec FR-375 rejects for the node. | spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md:51-52,59-61 |
| FND-003 | medium | No vector tells alias-free from alias-including ordering. Every fixture uses an upper-case alias (`Spec`, `S`) and protocol (`Twice`, `Full`), which sort before every lower-case keyword under both rules, so an implementation that orders by `Spec::Twice::finish`-style spellings passes AC-5, AC-8 and AC-9 yet emits different node bytes and `package_id` for, e.g., alias `spec` with `["spec::Twice::finish", "lose spec::Twice::ch"]` (alias-free: `Twice::finish` first; alias-including: `lose spec::Twice::ch` first). Add such a vector to AC-5 or AC-9. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:252,255-256 |
| FND-004 | low | FR-147-AC-10 and TC-552 step 10 test the library-value refusal only with the library present in the compilation; they do not cover a unit that does not import the library, which FR-147's Defects paragraph ("whether or not the unit imports that library") and QSpec TC-310 mutation 16 both state. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:257 |

## Verdict

Not mergeable as is: FND-001 contradicts the QSpec conformance vectors and
FND-002 leaves the TC expecting the old spelling and order. Both are one-line
fixes. FND-003 and FND-004 are test-strength gaps. The rest of the alignment is
correct: spellings, alias-free node member and order, locus pair,
resolve-first precedence and the not-yet-built locus all match merged QSpec.

QSpec note for the planner (not a QSL defect): QSpec FR-432 Resolution orders
"the resolved list" by the canonical, alias-including spellings, while FR-375
orders the node's `abstract_internal` by alias-free spellings and requires
alias-rename invariance. The two orders differ for a lower-case alias or
protocol name; FR-432-AC-7's vector does not show it. QSL follows FR-375,
which is the rule that fixes node bytes and identity, so QSL is right. QSpec
should restate FR-432's order as alias-free (or as a set, with FR-375 owning
the order).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | FR-147-AC-11's first vector (an unresolved entry `Spec::Twice::zz` "reports `missing_declaration`/`missing-name` and no `not-yet-implemented`") names no refinement, and the fix makes it unsatisfiable as written. A model-subject list is no longer name-resolved (the same AC says `Spec::Twice::zz` on `CasRefinesCounter` gives no `missing-name`), so the vector can only be on `CasTwice`. TC-552 runs step 11 against **unbuilt**, which omits `protocol-abstract-side` too, and there `CasTwice` also gets the `protocol-abstract-side` `not-yet-implemented` at `Twice` (FR-147 "Both features"). Name `CasTwice` and say "no `not-yet-implemented` naming `refinement-internal-list`" (the protocol-side refusal still beside it), or run the vector in a build that holds `protocol-abstract-side`. Step 11's Expected Results ("the unresolved entry gives `missing-name` and no `not-yet-implemented`") needs the same change. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:265; spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md:90-91 |

## Dispositions

Round 1, against head 40892cef1 (fix commit edced4cd7).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | edced4cd7: FR-147-AC-6 now locates `"refinement": 7` at `/0/refinement`, matching QSpec TC-386 IL-03 and TC-310 mutation 17; TC-552 step 6 names it. |
| FND-002 | fixed | edced4cd7: TC-552 Steps 1 and 5 expect `[Twice::finish]`, ordered by the bytes of the alias-free spellings. |
| FND-003 | fixed | edced4cd7: FR-147-AC-5 and TC-552 step 5 add the lower-case alias `spec` vector, `[Twice::finish, lose Twice::ch]`, which alias-including ordering would reverse (checked: `T` 0x54 < `l` 0x6C; `l` < `s` 0x73). |
| FND-004 | fixed | edced4cd7: FR-147-AC-10 and TC-552 step 10 cover the unit importing and not importing the library. |
