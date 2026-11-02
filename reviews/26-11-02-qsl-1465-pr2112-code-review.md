---
id: SR-202611021402
title: "Code, Rust and gap review of PR #2112: QSL-1465 explicit-threading Debug"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-foundation/src/source/node_table.rs; qsl-package/src/checked.rs; qsl-semantics/src/check; reviews"
review_set: subset
date: '2026-11-02'
---

## Summary

Ticket: QSL-1465. Reviewed origin/main...origin/peter/qsl-1465-linear-debug (head f94696b2c, base 1a4b6f3de). DebugTable plus DebugWith::fmt_with replaces the thread-local ACTIVE table. No thread_local, static cell, lock, once or unsafe in the changed core source; no allow attribute; no change under tools/arch-lint; the five target tests and their assertions are untouched; admission_corpus.rs is untouched. Key-hash collisions fall back to full NodeKey equality. An un-threaded path prints the full key, never a #n reference. Every debug_with_struct! field list matches its struct; hand-written impls destructure exhaustively or keep all fields. Cargo not run.

## Verdict

**NOT mergeable until FND-001 is fixed.** The rest of the design is sound.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | qsl-package no longer builds with default features off: the node_table import is std-gated but the impls using it are not | qsl-package/src/checked.rs:48-51,321-370 |
| FND-002 | low | The Drop doc comment is still glued onto the Debug doc comment above impl DebugWith for CheckedPackage | qsl-package/src/checked.rs:319-324 |
| FND-003 | low | reviews/ gains SR-1465001 and SR-1465002 for PRs #2101 and #2102, which are not this change; #2111's review files describe the removed thread-local approach | reviews/26-11-02-qsl-1465-pr2101-code-review.md |
| FND-004 | low | No test pins that a value inside a scoped call but not threaded prints a full key (not #n), and nothing builds qsl-package no-std | qsl-foundation/src/source/node_table.rs:493 |
