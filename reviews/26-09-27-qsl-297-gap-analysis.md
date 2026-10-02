---
id: SR-754
title: "QSL-297 gap analysis of PR 500 (FR-112; TC-510)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-112-build-protocol-scoped-anchor-forms.md; spec/functional/FR-113-resolve-scoped-anchors-in-nested-control-scopes.md; spec/test-cases/TC-510-s2-builds-protocol-scoped-anchor-forms.md; qsl-forms/src/protocol_clause.rs; qsl-forms/src/syntax.rs; qsl-forms/tests/it/protocol_clause_forms.rs; qsl-semantics/src/check/assemble.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-112
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-510
    type: reviews
---
## Summary

Ticket: QSL-297. PR: quire-spec-language#500. There is no plan
bundle for QSL-297, so this analysis checks FR-112's acceptance criteria and
TC-510 against the tests and the code.

Trace. The three tests carry `#[trace("TC-510", "FR-112-AC-1|2|3")]`, one
per AC. Every new production item traces to FR-112: the six sites, the
segments with their spans, the scope list and the source order. The
`ProtocolDeclarationForm` host is not named in FR-112. It is covered by
FR-112's "attached to the form of the construct that holds the reference"
and by the team-leader ruling on QSL-297.

Coverage by AC:

- FR-112-AC-1 is covered, with exact sites, segments, scopes and segment
  spans.
- FR-112-AC-2 is covered, and the scope assertion is a real oracle (it
  failed under a mutation).
- FR-112-AC-3 is covered as far as its spec text can be met (FND-002).

The Behavior bullets beyond the ACs are only partly covered: `receive-of`,
choice, case and repeat scopes are untested (SR-753 FND-004).

FR-113 did not leak into this PR. There is no resolution, no identity and
no S3 assembly of anchors.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Code and spec disagree on scope membership for `await`. FR-112 Outputs defines scope as the named controls that enclose the reference. FR-113 Inputs and its `await-after` row treat `await` as a structural control that declares its child nodes. The code leaves the await's name out of the scope of everything inside it. This is the same defect as SR-753 FND-001. It is recorded here because it breaks the input contract that FR-113 (QSL-298) reads. | qsl-forms/src/protocol_clause.rs:285-305; spec/functional/FR-113-resolve-scoped-anchors-in-nested-control-scopes.md:34-38 |
| FND-002 | low | Spec finding for the TC-510/FR-112 owner (QSL-296, merged). Do not fix it in this PR. TC-510 step 3 says to remove `effect Applied` and expects "the scoped anchors equal step 1's". FR-112-AC-3 says the two protocols "build equal `ScopedAnchorForm`s". Neither can hold as written: removing the effect removes its own `effect-of [Tried]` anchor and shifts the spans of later anchors. The test reads it sensibly: the two earlier anchors are byte-equal, and the later anchor keeps its site, segments and scope. The spec text should say that. Also, TC-510's fixture pointer is to `tests/it/composed_scopes.rs` `COMPENSATION` in the legacy composed lane, which M-6d deletes (QSL-303), so the pointer will go stale. The pointer is valid today. The PR's claim that it names a nonexistent file is wrong. | spec/test-cases/TC-510-s2-builds-protocol-scoped-anchor-forms.md:20-45; spec/functional/FR-112-build-protocol-scoped-anchor-forms.md:77 |
| FND-003 | low | FR-112 says `ScopedAnchorForm` "lives in `forms::protocol_clause`". It is defined in `qsl-forms/src/syntax.rs` and re-exported from the crate root. That matches the existing `StateClauseForm` precedent (FR-102), so this is accepted as the repo convention. The FR wording could be changed to match. | qsl-forms/src/syntax.rs:1128-1212 |

## Verdict

Gaps remain. FND-001 blocks: it breaks FR-112's scope output, which S3
depends on. FND-002 goes to the QSL-296 spec owner as a note and does not
block this PR. FND-003 needs no change.
