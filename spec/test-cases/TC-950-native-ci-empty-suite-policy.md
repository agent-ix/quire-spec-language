---
id: TC-950
org: agent-ix
title: "Native CI accepts only declared genuine empty suites"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-014
    type: verifies
---
# TC-950: Native CI accepts only declared genuine empty suites

## Description

Check the local guard's acceptance decision against NFR-014. Rust fixtures
exercise the actual producer/parser/evaluator and subprocess boundary. Expected
results below are independent of the implementation's allowlist and decoder.
All criteria NFR-014-AC-1 through NFR-014-AC-10 belong to this test case; each
actual test binds only the criteria it asserts. This document specifies checks,
not an executed test receipt or a current empty-target inventory.

## Test Procedure

1. Execute the four actual Makefile test recipes through a Rust process fixture
   that records complete child argv and target-directory environment. Compare
   with the four Scope vectors, including order, package/workspace selection,
   `--all-features`, literal paths with shell metacharacters, and inherited
   versus explicitly selected target roots. Observe both guard discovery and
   execution transport. Unknown lanes and inconsistent selections must refuse.
   This recorder establishes transport only, not Cargo semantics. (AC-1)
2. Separately use genuine Cargo in an isolated workspace with multiple packages,
   repeated test-target names, feature-dependent tests and required-feature
   targets. Run default and all-feature workspace selections and each package
   selection. Obtain expected target identities from the fixture's authored
   manifests and independently retained Cargo output, never the guard's own
   parsed list. Exercise relative and absolute executable paths and a genuine
   external dependency artifact. (AC-1, AC-2)
3. Exercise `--lib` and `--tests` with genuine Cargo: excluded doctests must not
   cause missing-suite findings. For `--doc`, verify either genuine compatible
   doc-only discovery/execution or a specific pre-spawn usage refusal. Restore
   an ordinary selection and show doctest expectations return. (AC-1, AC-8)
4. Feed the actual decoder independently constructed invalid JSON and producer
   records with missing fields, wrong types, duplicate/conflicting target or
   executable identity, and colliding normalized doctest names. Keep distinct
   same-named targets in different packages as a positive control. Excluding
   dependency records must not exclude a selected workspace target. (AC-2)
5. Use accurately labelled adapted transcript fixtures for unknown/malformed
   executable and doc headers, duplicated outcomes, missing running/terminal
   summary lines, invalid numeric fields including filtered-out, arithmetic
   overflow and mismatched counts. Include ordinary test output resembling an
   earlier zero summary followed by the real terminal summary. Compare exact
   affected identities and failure categories. (AC-3, AC-4)
6. Run genuine Cargo with one passing integration target beside a library with
   no unit tests or executable doc examples. Declare each genuinely empty
   selected suite with its source-specific reason and obtain acceptance. Remove
   declarations and require guard refusal while Cargo remains successful.
   Duplicate or blank declarations and a reason borrowed from another target
   do not qualify legitimate emptiness. Reports distinguish empty identities
   and reasons from passing suites. (AC-5, AC-6)
7. From a genuine populated fixture, remove or feature-disable all tests; keep
   the formerly populated suite undeclared and require refusal. Restore source
   and require the prior passing outcome. Separately add a passing test and an
   ignored test to a declared-empty fixture, and filter a registered test away
   through real libtest arguments. Compare registered/running, ignored and
   filtered counts against retained producer output. Each stale declaration
   must refuse. Independent measured-count vectors also refuse emptiness;
   label those vectors as adapted if the selected stable Cargo producer cannot
   emit a measured case. Nonallowlisted all-ignored/all-filtered suites refuse;
   ignored tests beside a passing test need not refuse. (AC-4, AC-6, AC-7)
8. Omit an expected selected executable section and separately a nonallowlisted
   selected doctest section from retained genuine producer output. Keep other
   suites nonzero. Require missing-suite refusal with the exact identity.
   Restore each omitted section and require acceptance. Include an absent
   declared doctest and a wholly empty invocation. Compare intentional
   target exclusions separately from disappearance. (AC-8)
9. Run a compile-failure fixture and a failing-test fixture with genuine Cargo
   directly and through the guard. Compare the actual original nonzero codes,
   rather than merely checking nonzero. Guard findings cannot mask either
   child failure. A successful Cargo child with an undeclared empty suite
   instead yields guard refusal. (AC-9)
10. Exercise post-spawn read failure and a controlled failing output sink with
    observable owned-child lifetime and completion/reaping. Coordinate using
    process events, not sleeps as evidence. Assert the original I/O cause and
    child cleanup, including a cleanup-failure control preserving that cause.
    The Rust fixture's injection boundary must not replace the guard's
    lifecycle logic or substitute a fabricated Cargo acceptance result. (AC-10)
11. In isolated fixture copies, independently disable undeclared-zero refusal,
    ignored/filtered drift detection, missing selected doctest reconciliation,
    and one Makefile argv/target-env transport rule. Each mutation must produce
    the independently expected wrong acceptance or transport and be caught by
    the corresponding assertion. Restore exact source and rerun the control.
    Retain negative and restored results separately from ordinary passes.
12. For final integrated acceptance, enumerate named suite
    identities and all count categories in each of the four final local lanes.
    For each actual empty suite, inspect its selected source/feature scope and
    retain its legitimate reason or unresolved discrepancy. Historical logs
    and compilation alone do not establish this inventory. Existing native
    validation, feature isolation, conformance and aggregate acceptance gates
    remain separate prerequisites owned by their existing requirements.

## Expected Results

Acceptance requires a complete, unambiguous selected producer population,
consistent suite observations and the NFR-014 declared-emptiness policy.
Genuine all-zero declared suites pass beside passing suites; undeclared loss
of coverage, stale declarations, missing expected suites and malformed or
ambiguous observations refuse. Supported selections retain their real expected
population; specifically refused selections do not run substitute work. Cargo
failure codes and original I/O causes survive, and owned children are reaped.

Canonical bindings use imported `ix_trace_rs::trace`, `TC-950`, and explicit
`NFR-014-AC-N` IDs matching each actual assertion. Existing narrow tests may
cover part of a criterion; they must not claim the broader genuine-producer,
transport, mutation or lifecycle checks before those assertions exist and run.
No test-case ID is used as a Verification method, no computed matrix is authored,
and no historical suite total is reported as final-lane runtime evidence.
