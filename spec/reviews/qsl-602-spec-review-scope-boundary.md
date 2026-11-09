---
id: SR-20261109172720000004
title: "Scope-boundary review of the L5 JIT forms amendment"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language; QSL-602; branch spec/l5-jit-forms-amend @ 05460d49a vs origin/main; ADR-035; FR-091; FR-093; FR-294; FR-560; FR-561; FR-562; FR-563; FR-568; TC-415; TC-1380; TC-1381; TC-1383; TC-1384; TC-1386"
review_set: subset
---

## Summary

Ticket: QSL-602. Ownership between QSL seam types, QSpec FR-331's provider envelope, FR-285 exit categories, FR-495 CheckMode, FR-230 clock place expressions and FR-350 corpus adapters. Spec-only branch spec/l5-jit-forms-amend at 05460d49a, scoped to `git diff origin/main...HEAD`. Tools: quire 0.36.2 (engine 0.50.2), quoin 0.29.1. No cargo builds were run.

## Verdict

Not mergeable until the high finding and the medium findings are fixed; the low findings are wording. Math: the FR-562 mod/rem table is sound for every law. Euclidean (`mod`, Euclidean `rem`) lies in [0, |b|-1] within [0, M-1]; Floor `rem` takes the divisor's sign ([0, M-1] for b > 0, [-(M-1), 0] for b < 0; the Floor 'every other case' row is unreachable because a 0-free interval has one sign); Truncating `rem` takes the dividend's sign, and a dividend spanning 0 gives [-(M-1), M-1]. M = 2^63 still gives M-1 inside the signed range. The `div` corner rule is sound (the quotient is monotone in each operand for a fixed divisor sign). AC-8's numbers check: ends-based `a mod 5` gives [0, 0]; the sound [0, 4] times [0, 4e18] reaches 1.6e19 > 2^63-1; rn/rf/rj's declared result types hold under each law. 8-form count: consistent in FR-560 (statement, AC-1), FR-294 and TC-1380; no stale 'seven' remains. Id collisions: none (every new AC id is unique in its file); quire validate on the 15 files exits 0. FR-091 versus FR-230: the narrowing does not contradict FR-230 'Selection' (S2 builds, S3 admits), apart from FND-003/004 of the integrity review. CheckMode: the code owner is right about the code. On origin/qsl-602-jit and origin/l5/integration, PackageDeclarations::check(self, limits: CheckingLimits) (qsl-semantics/src/check/mod.rs:1779) takes no CheckMode and CheckingLimits holds none. CheckMode is reachable only on standalone expression checks (check_expression(..., mode, limits), qsl-semantics/src/check/mod.rs:8317). The spec should keep the target design: FR-495 Inputs (already on main) supplies CheckMode to PackageDeclarations::check, and JIT entries come only from a CheckedPackage, so FR-562-AC-6 can be built only through a package-level mode. That FR-495 input is unimplemented, so the code side needs a ticket before FR-562-AC-6 (and FR-562-AC-3, per FND-005) can pass. Ceremony: no pins, digests or SHAs were added.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new sentence says a report that serializes EnvironmentRefused(ExecutableMemoryRefused) for an FR-331 consumer 'carries the diagnostic of the exit-21 unsupported category (FR-285) naming the backend jit', but FR-285 defines exit categories, not a diagnostic code/cause, so no code is named; no AC tests the serialization; and QSpec FR-331 is the analysis-provider envelope, so it is unstated which execution path ever writes an FR-331 result for an execution backend. Name the diagnostic code and cause (or the owning QSL requirement), and add an AC, or drop the serialization sentence and keep only 'not an FR-331 cause'. | FR-560:127-133 |

## Coverage

- FR-091-AC-7 (examined)
- FR-091 (examined)
- FR-093-AC-62 (examined)
- FR-093 (examined)
- FR-294 (examined)
- FR-560-AC-1 (examined)
- FR-560-AC-7 (examined)
- FR-560-AC-8 (examined)
- FR-560 (examined)
- FR-561-AC-1 (examined)
- FR-561-AC-3 (examined)
- FR-561-AC-4 (examined)
- FR-561 (examined)
- FR-562-AC-3 (context_only)
- FR-562-AC-6 (examined)
- FR-562-AC-7 (examined)
- FR-562-AC-8 (examined)
- FR-562 (examined)
- FR-563-AC-9 (examined)
- FR-563 (examined)
- FR-568-AC-5 (examined)
- ADR-035 (examined)
- TC-415 (examined)
- TC-1380 (examined)
- TC-1381 (examined)
- TC-1383 (examined)
- TC-1384 (examined)
- TC-1386 (examined)

## Dispositions

Round 1 verified against eadabac24. The FR-561-AC-3 fix can be satisfied by Cranelift output and can still fail: non-PIC Cranelift calls an imported shim with an Abs8 relocation naming the shim, then a register call, which the rule allows. A call through a pointer loaded from the frame (the negative generator) has no such relocation and fails. So would a PLT/GOT call or a libcall that is not a shim. Constraint for the coder: the rule allows a register call only for a shim, so the translator must declare the prepared value's own functions with final linkage (colocated), so that calls between blocks are direct PC-relative calls. quire validate on all changed files exits 0. The committed spec/reviews/ copies at eadabac24 match the review-pass files and do not yet carry this section; the lead or coder copies these files over.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
