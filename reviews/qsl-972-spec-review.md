---
id: SR-12327
title: "Spec review of quire-spec-language PR #1215: FR-121 Fault sentence (QSL-972)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@a59c5848f; spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md"
review_set: subset
---
# Spec review of quire-spec-language PR #1215

## Summary

The PR adds one sentence to FR-121's `frame_operation` bullet: a declaring
object that is no declaration of the selected model alias's domain package
refuses `CallSiteRefusal::Fault`, never a partial site.

- The sentence matches the code (call_site.rs `operation_site`) and the
  `Fault` variant's rustdoc, which the PR also extends.
- `Fault` is the right refusal. The case is a broken checker invariant, not
  caller input, because supertypes are intra-package.
- `quire validate` (quire 0.36.2, engine 0.50.2) on FR-121 reports three
  warnings on this branch: line 140 `ears:unclassifiable` and
  `ears:missing-subject`, and line 189 `quality:ambiguous-term`
  (`effective`). Running the same command on the origin/main copy gives the
  same three warnings, with the ambiguous term at line 187. The PR's two
  added lines moved it. All three were there before this PR.

## Verdict

No major findings. Mergeable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The added sentence is joined to the prior line, which makes line 172 136 characters long. The rest of the bullet wraps near 76 characters. This is cosmetic only. | FR-121 line 172 |
| FND-002 | low | No acceptance criterion covers the Fault sentence. That is acceptable, because valid input cannot reach the case, but the sentence could say it is an invariant break, not a caller refusal. | FR-121 lines 172-174 |
