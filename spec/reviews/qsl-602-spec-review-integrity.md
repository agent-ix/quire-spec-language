---
id: SR-20261109172720000001
title: "Integrity review of the L5 JIT forms amendment"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; QSL-602; branch spec/l5-jit-forms-amend @ 05460d49a vs origin/main; ADR-035; FR-091; FR-093; FR-294; FR-560; FR-561; FR-562; FR-563; FR-568; TC-415; TC-1380; TC-1381; TC-1383; TC-1384; TC-1386"
review_set: subset
---

## Summary

Ticket: QSL-602. Consistency and atomicity across ADR-035, FR-091, FR-093, FR-294, FR-560 to FR-563, FR-568 and their TCs, with each claim checked against the jit branch (origin/qsl-602-jit 55ec85cdd), origin/l5/integration and QSpec FR-331. Spec-only branch spec/l5-jit-forms-amend at 05460d49a, scoped to `git diff origin/main...HEAD`. Tools: quire 0.36.2 (engine 0.50.2), quoin 0.29.1. No cargo builds were run.

## Verdict

Not mergeable until the high finding and the medium findings are fixed; the low findings are wording. Math: the FR-562 mod/rem table is sound for every law. Euclidean (`mod`, Euclidean `rem`) lies in [0, |b|-1] within [0, M-1]; Floor `rem` takes the divisor's sign ([0, M-1] for b > 0, [-(M-1), 0] for b < 0; the Floor 'every other case' row is unreachable because a 0-free interval has one sign); Truncating `rem` takes the dividend's sign, and a dividend spanning 0 gives [-(M-1), M-1]. M = 2^63 still gives M-1 inside the signed range. The `div` corner rule is sound (the quotient is monotone in each operand for a fixed divisor sign). AC-8's numbers check: ends-based `a mod 5` gives [0, 0]; the sound [0, 4] times [0, 4e18] reaches 1.6e19 > 2^63-1; rn/rf/rj's declared result types hold under each law. 8-form count: consistent in FR-560 (statement, AC-1), FR-294 and TC-1380; no stale 'seven' remains. Id collisions: none (every new AC id is unique in its file); quire validate on the 15 files exits 0. FR-091 versus FR-230: the narrowing does not contradict FR-230 'Selection' (S2 builds, S3 admits), apart from FND-003/004 of the integrity review. CheckMode: the code owner is right about the code. On origin/qsl-602-jit and origin/l5/integration, PackageDeclarations::check(self, limits: CheckingLimits) (qsl-semantics/src/check/mod.rs:1779) takes no CheckMode and CheckingLimits holds none. CheckMode is reachable only on standalone expression checks (check_expression(..., mode, limits), qsl-semantics/src/check/mod.rs:8317). The spec should keep the target design: FR-495 Inputs (already on main) supplies CheckMode to PackageDeclarations::check, and JIT entries come only from a CheckedPackage, so FR-562-AC-6 can be built only through a package-level mode. That FR-495 input is unimplemented, so the code side needs a ticket before FR-562-AC-6 (and FR-562-AC-3, per FND-005) can pass. Ceremony: no pins, digests or SHAs were added.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-561-AC-3 (and FR-561 lines 32-41, 76: 'each by a direct call') forbids every indirect call, but Cranelift, which ADR-035 CG-1 item 3 says meets the call boundary, reaches an out-of-module symbol (Linkage::Import, colocated false; the jit branch sets is_pic=false in qsl-jit/src/exec.rs Machine::new) by loading its absolute address from an Abs8 relocation into a register and calling through it (x86-64 movabs+call reg, aarch64 ldr+blr); with is_pic=true it goes through a PLT stub that jumps via a GOT slot, a function pointer in memory. A shim in the Rust binary is not colocated with JIT memory, so no conforming JIT passes AC-3 as written, and the step-2 control 'an indirect call through a register' fails the real code too. Target design: classify by relocation record: allow a register-indirect call only when its register is loaded by an absolute relocation naming a KernelTable shim symbol (immediate in finalized read-only code, no pointer in writable memory); fail any other indirect call. Not reproduced here (no builds); from Cranelift's far-call lowering. | FR-561:92 |
| FND-002 | medium | FR-568-AC-5 states EntryFrame's fields are 'the argument handles and the meter alone', but the one entry signature returns only a u32 status (FR-568, ADR-035 CG-4 item 2), so the entry's result value handle and the stop record must live in the frame; the jit branch's EntryFrame (qsl-jit/src/exec.rs) holds failed, result, ops, types and halt besides arguments and meter. 'alone' is unsatisfiable by any working entry and untested (the test only checks the absence of function-pointer, KernelTable and Cancel types). State the prohibition (no function pointer, KernelTable or Cancel reference) and drop 'alone', or list the result/status slot. | FR-568:96 |
| FND-003 | medium | The new AC-7 sentence requires S2 to build FR-230-AC-21's resets self.ts[1].x, self.m[k].x and self.m[k] = Timer{..} with no UnrepresentedConstruct, but FR-091 line 380 (edited in this diff) still lists `self` among the unrepresented constructs, so the prose refuses the very unit the AC admits. The code builds `self` (qsl-forms/src/value.rs:1878 Shape::SelfRef), so the list is stale; remove `self` from the list (or scope the list to function bodies). | FR-091:1231 |
| FND-004 | medium | The edited sentence 'S2 returns no form for any declaration of that unit that holds an unrepresented construct' reads as if declarations without one still get forms, contradicting the paragraph's own 'refuses the whole unit' and AC-7's 'returning no form' (cf. AC-5: no form for the other declaration). Restore 'S2 returns no form for any declaration of that unit'. | FR-091:383-384 |
| FND-005 | medium | The new FR-562 prose says a divisor interval that 'spans 0' never reaches the JIT because the checker refuses it unproved-nonzero; FR-495-AC-3 refuses the identical d1 shape (b: Int[0, 9]) under CheckMode::Linked. FR-562-AC-3's d0(a: Int[0,9], b: Int[0,9]) a div b with b = 0 returning Undefined therefore only exists under CheckMode::Kernel, which AC-3 does not say. Fix: AC-3 checks d0 under CheckMode::Kernel (as AC-6 now does), and the prose says 'contains 0' and 'under CheckMode::Linked' ([0, 9] contains 0 without spanning it). | FR-562:62-65, 116 |
| FND-006 | medium | The new 'Parity cases' text and AC-9 require the report to count vectors 'not selected', but FR-563 Outputs still defines ParityReport as 'per backend, the compared count, the unsupported entries and the findings', with no not-selected member. Add the member to Outputs (and the ParityReport type). | FR-563:70, 118-119 |
| FND-007 | low | FR-560 and FR-294 call `name` an 'ADR-012 Value family parsing form', but ADR-012's Value row lists literals, operators, let, if, calls, records, collections and function declarations, no name read. Either add name reads to ADR-012's Value row or say `name` is the checked syntax's Name variant (FR-560's lower-casing rule) rather than an ADR-012 form name. | FR-560:45-48 |
| FND-008 | low | FR-561, FR-568 and ADR-035 CG-4 say the meter 'holds the &Cancel'; quire-exact's Meter holds an owned `cancel: Option<Cancel>` (quire-exact src/accounting.rs:647), a handle clone, not a reference. Say 'the meter, which holds the call's Cancel handle'. | FR-561:51-52 |
| FND-009 | low | FR-562-AC-6 says 'the JIT decides the addition from its operand interval [0, 18]'; [0, 18] is the result interval (the operands are [0, 9]). Say 'from its result interval [0, 18]'. | FR-562:119 |
| FND-010 | low | FR-563 now builds parity cases through FR-350's input adapters and FR-100's run vectors, but its relationships keep FR-350 as `references` and do not name FR-100. Make FR-350 depends_on and add FR-100. | FR-563:1-21 |

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
