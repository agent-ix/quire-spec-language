---
id: SR-15411
title: 'Gap analysis of PR #1207 against QSpec FR-322-AC-37'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@5d4be17f7; git diff origin/main...5d4be17f7 (PR #1207): qsl-package/src/checked_v2/tests.rs; reference QSpec agent-ix/quire-specification@8ecde34 spec/objects/interfaces/FR-322-checked-package-artifact.md FR-322-AC-37; ticket STD-237'
review_set: subset
date: '2026-10-22'
---

## Summary

Ticket: STD-237. This is a planless gap check of the PR #1207 diff against QSpec FR-322-AC-37. Plan completion: not assessed. The PR changes only the conformance harness, so that the published positive-dependency-call fixture is read with its dependency's admitted package supplied under the selection identity.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-322-AC-37 | quire-specification spec/objects/interfaces/FR-322-checked-package-artifact.md | examined |
| supply_qspec_dependency_packages | qsl-package/src/checked_v2/tests.rs:1307-1367 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The AC-37 refusal mutations published in `dependency-call-vectors.json` (no package supplied, package absent, node without declaration, ineligible signatures, target listed in dependencies) are not consumed by any QSL test. This PR covers only the admitted path. The ticket already records this as remaining work. | qsl-package/src/checked_v2/tests.rs |

## Verdict

The supply logic matches FR-322-AC-37's admitted path:

- Selections are walked in lock order.
- Each selection is matched to the fixture whose recomputed identity-preimage digest it names.
- The library is read through the production I2 reader, pinned at its own recomputed id under the selection identity, with its own dependencies supplied first.
- The admitted package is inserted under the selection identity.

Malformed or unmatched entries supply nothing, so the reader's own refusals still decide those cases. FND-001 is deferred to the follow-up already recorded on STD-237 and does not block this PR.
