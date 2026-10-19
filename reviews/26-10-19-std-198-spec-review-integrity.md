---
id: SR-13059
title: "Integrity review of PR #1016: internal-list source, class spellings, identity and the #970 refusal"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@6f2f80ba5; spec/functional/FR-147 (The profile's internal list, Internal abstract steps, AC-5, AC-6); spec/functional/FR-135 (Refinement features the build does not check, AC-5); spec/decisions/ADR-020 RM-9; against QSpec FR-432, FR-375, FR-322, FR-271, FR-272 (quire-specification@ad2b37e) and #970 (qsl-543 @d22cb4034)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: reviews
---

## Summary

Ticket: STD-198. Checks the PR against QSpec FR-432 (step classes and their
spellings), FR-375 (refinement grammar and checked-package node), FR-322
(`dependency_selections`), FR-271/FR-272 (codes and causes), QSL FR-286 and
#970's `RefinementCause::Unsupported`. The causes the member uses are all
in FR-272's closed lists, as claimed. Six defects remain. The member has a
name but no host document. Its identity claim needs a node member that
QSpec does not define. The class spellings cover only part of FR-432. The
ordering key is undefined. The FR-271 rationale is misattributed. The spec
and #970 disagree on span, feature names and how many refusals are emitted.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | STD-198 asks where the list is declared. FR-147 answers with a name, "the unit's own admission input, as the `refinement_internals` member beside `dependency_selections`", but QSL has no unit admission input document. `dependency_selections` is not an authored input either. It is QSpec FR-322's closed member of the checked package's `lock` and `identity_preimage`, computed from `import` declarations. The spec does not say whether the JSON is written in the source unit, the native-compile/1 request (beside `libraries`), the lock or another file, so an implementer cannot place or read it. Name the host document and the member's place in it. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:99-108,110-117; quire-specification spec/objects/interfaces/FR-322-checked-package-artifact.md:80-83,96-123 |
| FND-002 | high | The Reading bullet says the resolved classes "enter the refinement node and so the `package_id`". QSpec FR-375's checked-package node body is a closed list, `{"term": "refinement", "abstract", "profile", "populations", "objects", "steps", "histories", "assume", "ensure"}`, with no member for internal classes, and ADR-020 QS-4 gives the v2 node to QSpec. A conformant producer cannot carry `abstract_internal` in the node. FR-147-AC-5's identity half therefore needs a wire member that neither spec defines, the member is not marked QSL-proposed, and STD-199 does not ask QSpec for it. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:118-125,190; quire-specification spec/functional/analysis/FR-375-declare-model-refinement.md:168-180; spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md:553 |
| FND-003 | medium | Shape says each entry is "in the form of FR-432's left sides" and then lists only node path, `duplicate`/`lose` channel and `memory <kind>`. FR-432 also has `fork`/`join` of a parallel node, `cend` of a template, `spawn`/`retire` of a role and `activate` of the trigger. A bare `<A>::<P>::<par>` cannot say whether fork, join or both are meant, and `"spawn Spec::P::r"` has no stated reading. An abstract class that cannot be spelled can never be internal. STD-199 item 2 lists spawn/retire spellings, but FR-147 Remaining work does not record that these classes are left out. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:110-117,142-145,193-199; quire-specification spec/functional/protocol/FR-432-map-every-protocol-step-in-refinements.md:48-58 |
| FND-004 | medium | `abstract_internal` enters identity "ordered by their canonical class keys", but no QSL or QSpec artifact defines a canonical class key. Two producers can order `[finish, lose ch]` differently and get different `package_id`s. `dependency_selections`, the precedent the PR cites, states its order exactly: strictly ascending UTF-8 bytes of `identity`. Define the key and its byte order. "Without repeats" in the same bullet is also dead text, because Defects already refuses a repeated class. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:118-125,133-136; quire-specification spec/objects/interfaces/FR-322-checked-package-artifact.md:96-99 |
| FND-005 | medium | FR-135 says `unimplemented_capability` is the code "which FR-271 gives the driver's unbuilt engine". FR-271 says otherwise: "Valid complete-V1 meaning is not implemented by the selected producer; distinct from prohibited source and unsupported backend projection". That definition describes an unchecked history row exactly and explicitly excludes the backend case. The unbuilt-engine use is QSL FR-286's. Keeping `unknown_required_feature` matches #970 and QSL precedent (FR-092 recursion, state clauses), and FR-272 gives `unimplemented_capability` no cause. The rationale should say that, and cite FR-286, rather than misquote FR-271. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:150-157; quire-specification spec/objects/foundation/FR-271-diagnostic-code-vocabulary.md:43,79; spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:140 |
| FND-006 | medium | FR-135-AC-5 and #970 disagree in three ways. (1) AC-5 refuses the protocol side "at the `abstract` alias", the body says "abstract-side token", and #970 refuses at `clause.span`, the protocol name. (2) AC-5 names the features `history-row` and `protocol-abstract-side` plus the profile identity; #970's `Unsupported { construct }` carries free text ("a history row (FR-138)") and no profile identity. (3) The body refuses "at the span of the first row", which is one refusal, while #970 refuses every history row. Choose one span and one count, and record that #970 must carry the feature name and scope. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:149-153,178; qsl-543 qsl-semantics/src/check/refinement.rs:116-122,484-495,580-592 |

## Verdict

Not mergeable. FND-001 leaves STD-198's central question (where the list
lives) unanswered in a way an implementer can act on. FND-002 makes AC-5's
identity half unbuildable without a QSpec wire member that STD-199 does not
request. FND-003 to FND-006 are consistency fixes. Clean: every cause the
member uses (`malformed-declaration`, `missing-name`, `conflicting-binding`)
is in FR-272's closed lists. The conflicting-binding addition keeps its
QSL-addition label in AC-2 and AC-4. ADR-020 RM-9 and FR-147 Remaining
work cite STD-199.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | high | AC-2 and AC-7 give the same input two different refusals. A non-empty list for `CasRefinesCounter`, which has a model abstract side, refuses `invalid_model_binding`/`malformed-declaration` under "Internal abstract steps" and AC-2's last vector. AC-7 and TC-552 step 7 refuse `unsupported_construct`/`not-yet-implemented` for it "at the span of the `<protocol>` name", but `CasRefinesCounter` (`abstract Spec`) has no protocol name. Resolution also runs "against its abstract protocol clause", which a model side lacks. State which rule decides a model-side refinement and its span, and make AC-2, AC-7 and TC-552 step 7 agree. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:125-126,150-156,174-175,209,214; spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md:36-37,56-59 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | STD-199 item 1. FR-147 no longer claims a host: Inputs reads "No document of the build hosts one yet, so it is absent", and Remaining work lists the host as undecided. |
| FND-002 | fixed | Round 1: the claim that `abstract_internal` enters the node and `package_id` is removed. "Until QSpec decides the host" leaves the node member and identity undecided, and STD-199 item 2 asks QSpec for them. |
| FND-003 | fixed | Round 1: the Shape table closes the spellings to four forms. fork, join, cend, spawn, retire and activate "cannot be named internal, so each stays visible", recorded in Remaining work and STD-199 item 5. |
| FND-004 | fixed | Round 1: the order is ascending UTF-8 bytes of the canonical spelling, which Shape defines, and "without repeats" is removed. |
| FND-005 | fixed | Round 1: the rationale now quotes FR-271 correctly and gives FR-272's missing cause as the reason; the code moved to `unsupported_construct`/`not-yet-implemented` per FR-341-AC-5 and ADR-017 Q-6. |
| FND-006 | fixed | Round 1: the spec picks the `<protocol>` name span and one refusal per row, both matching #970 at b68219026, which now returns the same code and cause. #970 still differs from FR-135 in three ways: its `Unsupported { construct }` carries free text, not `history-row`/`protocol-abstract-side`, and no profile identity; it returns early after the protocol-side refusal although FR-135 checks the rest of the declaration; and it refuses at `row.span`, not the `history` keyword token. Those are #970 changes. |
