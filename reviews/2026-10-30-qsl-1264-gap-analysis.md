---
id: SR-1878002
title: "gap analysis of PR #1878 (QSL-1264): Semantic IR fixture digest purge"
type: SpecReview
analysis: gap-analysis
date: 2026-10-30
scope: "agent-ix/quire-spec-language; PR #1878 git diff origin/main...f8d4247f4; QSL-1264 items 1-3"
review_set: subset
---
# SR-1878002: gap analysis of PR #1878 (QSL-1264)

## Summary

Ticket: QSL-1264. Plan completion: not assessed. This was a manual scoped check of the ticket items this PR claims (items 1-3: the Semantic IR fixture digests, the generated layering corpus, and `switch.semantic-ir.json`) against the PR head. The PR touches no `spec/` file and no tagged acceptance criterion, so `quoin matrix` was not run.

- Item 1 is done. All `PLACEHOLDER`/`DIGEST`/`PLACEHOLDER_DIGEST` consts are gone (zero hits), and so are the members in every fixture and in `examples/config-version/model.semantic-ir.json`.
- Item 2 is done. `xtask/src/refinement/layering_corpus.rs` no longer writes the members, and its `BLOCKED-BY PLAT-1205` const is gone.
- Item 3 is done. `examples/switch-transition/switch.semantic-ir.json` no longer carries `sha256:878fa019…`.
- Tests: every fixture builder still feeds the same intake paths. No test lost a behaviour assertion. Only digest-equality lines and the helper's own self-test were removed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | This PR lands PLAT-1205, but three sites still cite `BLOCKED-BY PLAT-1205` as the only blocker of a digest: the native-linked package `Source.digest` record field, its schema member, and `Source::read_verified_typed`. Their stated blocker is now lifted, so each is either a leftover digest to delete with this bump, or a false blocker marker. QSL #1865 (merged) touched all three files and left them. | src/package/wire.rs:39; schemas/native-linked-package-1.schema.json:171; qsl-foundation/src/source.rs:422 |

## Verdict

Findings: FND-001. Ticket items 1-3 are complete in this diff. Items 4-7 belong to other PRs (#1856, #1857) per the ticket's status comment and are not assessed here.
