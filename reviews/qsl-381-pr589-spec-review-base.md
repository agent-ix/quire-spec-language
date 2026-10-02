---
id: SR-1197
title: "Spec review of quire-spec-language PR #589: tracking strip and STD-150 source identity"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language PR #589 (spec/strip-tracking-rebuild); git diff origin/main...5fdfb175; main 7959be70"
review_set: subset
---
# Spec review of quire-spec-language PR #589

## Summary

Ticket: QSL-381. PR #589 (spec/strip-tracking-rebuild) is one branch on main 7959be70, which includes #577 and #588. Reviewed the PR's own diff (`git diff origin/main...5fdfb175`). STD-150: FR-001's `RawSourceRef` is now the authority, the identity and the `quire.source.bytes/v1` digest, "and no other member". That is QSpec #178's `RawSourceRef` `{authority, identity, digest_domain, digest}` on QSpec main 5e85e890. Labels are checked authority then identity, and the node key ignores the digest (FR-001-AC-7). Every implemented-for-four-labels status line names the two-label remainder. Merged content: the ADR-010, ADR-011, ADR-013 and ADR-014 regions lose only revision text, and no #577 or #588 text is lost. TC-430 keeps #577's step 5 and FR-031 is untouched; ADR-011 §4's QSL-347 text and E7's O-09 content are intact. `quire validate` over the 680 touched spec and review files reports one structural failure (reviews/26-09-17-model-graph-binding-code-rust.md severity cells), which is identical on main. As the brief directed, the SCHEMA_SHA256 removal, BASELINE.md's measured revisions and `formal_revision` are not raised.

Examined:
- FR-001 (examined)
- FR-001-AC-5 (examined)
- ADR-013 O-07 Public type (examined)
- ADR-010 §2 to §8 SHA removals (examined)
- ADR-011, ADR-012, ADR-014, ADR-015 SHA removals (examined)
- TC-430 (examined)
- QSpec RawSourceRef (quire-specification#178, main 5e85e890) (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-045 pins the QSpec baseline `782c1ce39a197cd52b8b35b50adf2e5e3ecedd0f` twice (lines 93 and 164). It also requires the code to carry it: `classify` keeps a constant naming FR-095 "at the same baseline" on every supported classification, "so a disposition always reports the exact source revision it enforces". TC-125 step 5 (line 52) asserts that revision. That is version tracking built into behaviour. Nothing breaks without it, and only the pinned literal passes the check. Delete the baseline from FR-045's table sentence and Status, the retained-revision behaviour, and TC-125 step 5's revision clause, and cite FR-095 by id. | spec/functional/FR-045-classify-temporal-mapping-support.md:90-94, 162-166 |
| FND-002 | low | FR-049:48 cites F's observation contract "at the accepted native-v1 baseline `782c1ce39a197cd52b8b35b50adf2e5e3ecedd0f`", and TC-136:28 builds its inputs from "the F observation contract revision `782c1ce…`" and has the tester "substitute ... the observation revision". These are SHA pins on a cited contract. Cite the contract by id and drop the revision and the substitution step. | spec/functional/FR-049-admit-composed-evaluation-inputs.md:48 |
| FND-003 | low | Three files this PR touches still pin the adopted standard to commit `e897f81`: FR-008:38 ("It uses the adopted standard at e897f81"), spec/model-linking/tests.md:10 ("adoption of specification PR8 at e897f81") and plan/Plan-007-native-packages/log.md:16 ("private audits at adopted standard e897f81"). Drop the SHA from each. | spec/functional/FR-008-evaluate-state-reference.md:38 |
| FND-004 | low | The touched scope-boundary review keeps the baseline SHA in its mermaid node label: `Standard["Immutable native-v1 baseline\n782c1ce"]`. Drop `\n782c1ce` from the label. | spec/reviews/core-language/scope-boundary.md:70 |
| FND-005 | medium | The PR deletes ADR-010's revision table and §6.3 "Pin staleness", but the Evidence convention still reads `git grep -n -F '<pattern>' <sha> -- <path>` "at the revision above", and still says '"Behind" counts are `git rev-list --count <pin>..<Context sha>` against the Context table'. Neither the revision nor the Context table exists any more, and no "behind" counts remain. Write the absence check against each repository's `origin/main` on 2026-09-19, as the Repositories inspected paragraph now says, and delete the "Behind" sentence. | spec/decisions/ADR-010-observed-architecture-baseline.md:59-65 |
| FND-006 | medium | The PR changes only the first clause of the S-4b slice row to "FR-001's two-label `SourceIdentity`". The same row still says the wires and schemas are "each carrying the four labels" and that FR-021's hash vectors change "because the manifest's source identity gains two labels". S-4b is the slice that landed the four-label identity: every status line this PR writes says "implemented for four labels (ADR-013 §7 S-4b), two-label form remaining (QSpec STD-150)". So the row now contradicts itself and misstates what S-4b landed. Restore "four-label" in S-4b, and if the slice table should mention it, add the two-label change as the QSL-381 remainder. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:1105 |

## Verdict

Not mergeable: three medium findings (FND-001 FR-045's behavioural QSpec baseline pin, FND-005 ADR-010's dangling `<sha>`/Context-table convention, FND-006 the self-contradicting ADR-013 S-4b row) and three low findings (FND-002 FR-049/TC-136 baseline pins, FND-003 `e897f81` in three touched files, FND-004 the scope-boundary mermaid label).
