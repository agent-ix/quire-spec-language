---
id: SR-2611083
title: "Evidence review for the L5 phase-0 batch"
type: SpecReview
analysis: evidence
scope: "Verification method and test-procedure strength for FR-560 to FR-567, NFR-017 and NFR-018 and TC-1380 to TC-1399"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-560
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-561
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-563
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-564
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-565
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-566
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-567
    type: reviews
  - target: ix://agent-ix/quire-spec-language/NFR-017
    type: reviews
  - target: ix://agent-ix/quire-spec-language/NFR-018
    type: reviews
---

## Summary

`quoin advise --json` (quoin 0.29.1) was run over the worktree. It runs
repository-wide and was stopped by a 240-second timeout with no output, so the
recommendations below are judgement, not catalog verdicts. Every authored
method is `Test`, except FR-563-AC-8 (`Test` and `Inspection`), which fits
since that AC checks that a member is absent. The concerns are not the choice
of method. They are criteria and procedures that cannot fail, or that cannot be
run through any interface the specification defines.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-566-AC-1 cannot fail. FR-565 cases carry their request in full, including each `analyze` item's disposition and each `prove` item's routed backend, and the `check`, `package`, `execute` and `monitor` cases ignore the registry and `Engines`. Byte equality across the three configurations therefore holds whatever layer R does. Fix: have `run_case_with` negotiate each `analyze` and `prove` item's disposition from the registry it is given (FR-075, `negotiate_*`), and add a negative control: a plugin that advertises a pair a case uses changes that item's candidate set and settles it `ambiguous-backend`. | FR-566-AC-1, FR-565, TC-1395 |
| FND-002 | medium | The `prove` (QSL half) timeout, cancellation and tool-failure cells (TC-1393 step 2) build FR-291 records and serialize them. No QSL operation runs, so the cells repeat FR-291-AC-1 and AC-2, although FR-565 says `run_case` "runs the case through QSL's library operation". Fix: mark these three cells as produced by the driver half (n/a for the QSL half, with that reason), or have them cite FR-291's ACs and not claim a run. | FR-564-AC-6, FR-565, TC-1393, FR-291 |
| FND-003 | medium | FR-561-AC-3 checks only relocation records. Calls through a `KernelTable` are naturally indirect (the address is loaded from the table) and leave no call relocation, so a stray indirect call goes unseen, and the check passes even when it finds no call relocations at all. Fix: require every indirect call target to be loaded from the `KernelTable` base or a block of the same prepared value, add an indirect stray-call negative control, and assert that at least one kernel-table call is observed over the generated units. | FR-561-AC-3, TC-1383 |
| FND-004 | medium | FR-567-AC-2 and AC-3 replace the matrix case set and the linked backend set, but FR-565's `lifecycle_matrix()` is a static function and FR-567 gives `qualify` no input that takes a substitute case set or backend. As specified, these ACs cannot be run. Fix: specify the `qualify` library entry that takes the cases and backends (the xtask binary passes the defaults), and test through it. | FR-567-AC-2, FR-567-AC-3, TC-1396, FR-565 |
| FND-005 | low | FR-563-AC-8's "`make parity` with an unknown option refuses, exit 2" passes without testing anything: `make` itself exits 2 on an unknown flag and silently accepts an unknown `VAR=value`. Fix: define the options as flags of the xtask command `make parity` runs, and test that command's own refusal. | FR-563-AC-8, TC-1387 |
| FND-006 | low | TC-1397 step 3 checks only the directories "the gate reported making", so a directory the gate left behind without reporting it passes. Fix: run the gate with `TMPDIR` set to a fresh empty directory and assert that it is empty afterwards. | NFR-017-AC-4, TC-1397 |
| FND-007 | low | NFR-018's second clause (the repository creates a release tag only at a commit where the gate passed) has no AC. Fix: add a release target that runs the gate before `git tag` and a Test AC that it refuses to tag on failure, or an Inspection AC on the release procedure. | NFR-018 |
| FND-008 | low | FR-560-AC-3 and AC-4 need a form that `coverage()` does not list. If the coverage lists every ADR-012 form, no such form exists and the ACs pass without testing anything. Fix: take the form from another family, which the `Value`-family JIT cannot list, or test against a test coverage value. | FR-560-AC-3, FR-560-AC-4, TC-1381 |
| FND-009 | low | FR-565-AC-3 and TC-1394 step 3 say "no case opens a file" but name no way of observing file opens. "No QSL checkout on disk except the built crate" cannot be arranged inside a cargo test run from the checkout. Fix: name the mechanism (for example, run the cases in a child process with no readable filesystem), or state the property by type: `run_case` takes no path (CB-3 item 2). | FR-565-AC-3, TC-1394 |

## Dispositions

Fix round 1, in the same PR:

- FND-001: fixed. FR-566-AC-1 adds a live-configuration control: FR-281-AC-14's item runs on the plugin provider in configuration 3 and reads `trusted`.
- FND-002: fixed by citation. The prove timeout, cancellation and tool-failure cells cite FR-291-AC-1 and FR-291-AC-2's records, and FR-564 states the plugin run is the driver's.
- FND-003: fixed. FR-561-AC-3 constrains indirect calls to table slots, requires at least one observed table call, and adds an indirect negative control.
- FND-004: fixed. FR-567 specifies `xtask::qualify::run(&QualifyInputs)` with test-support replacements.
- FND-005: fixed. FR-563-AC-8 tests `cargo xtask parity --no-such-option`.
- FND-006: fixed. TC-1397 runs with a fresh empty `TMPDIR` and asserts it is empty.
- FND-007: fixed. NFR-018-AC-3 tests `make tag-release`.
- FND-008: accepted as stated. If the JIT's coverage lists every ADR-012 form and K2 type, no construct is untranslatable and FR-560-AC-3 has nothing to name; FR-563's coverage-gap check still holds the JIT to that coverage.
- FND-009: fixed. FR-565-AC-3 states the property by type.
