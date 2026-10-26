---
id: SR-1568001
title: "code review of the contract IR pin for state graph and terminal support (QSL PR 1568)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ad1a59dc07bed1fc1ff8bfdd16406a85044f5497; git diff origin/main...origin/qsl-ir-562-pin; Cargo.lock, qsl-package/src/emit/tests/admission_corpus.rs"
review_set: subset
date: '2026-10-26'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: references
---
# SR-1568001: code review of QSL PR 1568 (IR pin)

## Summary

Ticket: QSL-983 (landing dependency). Branch `qsl-ir-562-pin`, head ad1a59dc0, based on
e08cceeb8, which includes QSL-987 (#1579). It has three commits:

- 605b90092 moves the locked `quire-contract-model` revision from e31c506a to 6f6b5621.
- 578d453c5 adds two admission-corpus classifications.
- ad1a59dc0 removes those two classifications.

The net diff against origin/main is the one `Cargo.lock` line. `Cargo.toml` keeps
`branch = "main"`, so there is still one copy of the dependency.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. The lock bump is the only change at merge: the two admission-corpus commits cancel out.
On origin/main, `temporal`/`terminal` is already classified as not emitted, and QSL-987 already
classifies `claim`/`state_graph`. So #1568 alone leaves the FR-093-AC-19 corpus consistent.

The branch is three commits behind origin/main (#1584, #1589, #1580). The #1595 stack, which
contains it, merges onto origin/main 350524f18 with no textual conflict. This review did not
measure whether 6f6b5621 compiles against those three commits; the planned single gate run on
the merged stack covers that. Mergeable, landing with #1595 after that gate.
