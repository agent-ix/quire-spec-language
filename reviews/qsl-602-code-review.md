---
id: SR-20261109103206
title: "Code review of QSL-602 qsl-jit (branch qsl-602-jit fb06033b)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; git diff origin/l5/integration...fb06033b: qsl-jit/**, qsl-eval/src/execution.rs, qsl-eval/src/execution/scan.rs, qsl-conformance/**, tools/arch-lint/unsafe_confinement.rs, tools/arch-lint/main.rs, xtask/src/parity.rs, Makefile"
review_set: subset
---

## Summary

Ticket: QSL-602. Code review with the rust-review lane (rust-style loaded) of the Cranelift JIT backend, the BackendCoverage seam, the FR-563 parity runner, the FR-568 unsafe-confinement check and the parity xtask. `cargo test --locked -p qsl-jit` passed (14 + 3 integration tests, doctests) and `make arch-lint-unsafe` passed on the branch. Accepted draft gaps (missing quire-exact entries, NodeKey accessor, EnvironmentRefused, unbounded-Integer kernel-handle path, the f6 coverage gap) were not re-reported.

## Verdict

CHANGES REQUESTED. The confined unsafe module is sound as currently wired (frame pointer provenance, entry transmute after the arity check, free_memory ordered after every entry via field order, CallableEntry borrowing PreparedJit), and charge identity with the interpreter holds on the tested functions including denied-charge sweeps. One unsound FR-562 interval rule (mod/rem) makes the JIT fault where the interpreter completes; a parameter-kind mismatch does the same for wide unread parameters; the FR-568 recorded-signature check is vacuous and its test hooks ship in the public API; the exec safety contract depends on unchecked layout coupling in translate.rs; the TC-1386-tagged JIT parity test is an allow-list; and the unsafe-confinement check fails open in several ways.

## Scope

- `qsl-jit/src/exec.rs` (examined): EntryFrame, shim, Machine, Entry::call, Drop for Machine.
- `qsl-jit/src/prepared.rs` (examined): Jit::prepare, PreparedJit, CallableEntry, call_metered, registers.
- `qsl-jit/src/translate.rs` (examined): Compiler, Emitter, division_interval, entry trampoline.
- `qsl-jit/src/kernel.rs` (examined): kernel::run.
- `qsl-jit/src/coverage.rs` (examined): coverage().
- `qsl-eval/src/execution/scan.rs` (examined): first_outside, untranslated.
- `qsl-conformance/src/lib.rs`, `qsl-conformance/src/compare.rs` (examined): run_parity, difference.
- `tools/arch-lint/unsafe_confinement.rs` (examined): check.
- `qsl-jit/tests/it/*.rs` (examined): trace tags vs TC-1380/1381/1382/1385/1386/1400.
- `FR-562`, `FR-563`, `FR-568`, `TC-1385`, `TC-1386`, `TC-1400` (context_only, origin/main 234cc379e).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | mod/rem result interval taken from the four operand-interval ends is unsound (remainder is not monotone), so a downstream op is wrongly judged to fit 64 bits and the kernel result faults at run time where the interpreter completes. | qsl-jit/src/translate.rs:734-772; qsl-jit/src/translate.rs:485-489; qsl-jit/src/kernel.rs:35-37 | wrong-requirement |
| FND-002 | medium | registers() treats every Int parameter as a register Integer, but type_of drops Int types whose bounds exceed i64; an unread wide parameter given a large in-range argument returns Fault jit-argument-kind instead of the interpreter's value. | qsl-jit/src/prepared.rs:203-210; qsl-jit/src/prepared.rs:457-469; qsl-jit/src/translate.rs:105-117 | implementation-bug-despite-evidence |
| FND-003 | medium | The FR-568 recorded-signature check is vacuous: Machine::entry stamps the constant ENTRY_SIGNATURE, the translator records none, so exec.rs:291 compares a constant with itself; the FR-568-AC-3 tests only exercise a hook. | qsl-jit/src/exec.rs:191-198; qsl-jit/src/exec.rs:291-293; qsl-jit/src/translate.rs:77-81; qsl-jit/tests/it/lifecycle.rs:120-132 | correct-requirement-no-evidence |
| FND-004 | medium | The FR-568 test hooks are doc(hidden) pub on the shipped PreparedJit; return_out_of_range_status_for_test makes it enter an address not recorded at generation, contradicting FR-568's last AC in production builds. | qsl-jit/src/prepared.rs:339-363; qsl-jit/src/exec.rs:269-282 | implementation-bug-despite-evidence |
| FND-005 | medium | exec's safety rests on unchecked layout coupling outside exec: translate.rs hard-codes EntryFrame offsets 0/8/16 with no offset_of assertion, and shim is a safe extern "C" fn that dereferences a raw pointer. | qsl-jit/src/translate.rs:218-223; qsl-jit/src/translate.rs:708-712; qsl-jit/src/exec.rs:53-66; qsl-jit/src/exec.rs:98-103 | implementation-bug-despite-evidence |
| FND-006 | medium | The JIT parity test tagged TC-1386 filters out every CoverageGap finding (an allow-list FR-563 forbids) and passes while `cargo xtask parity --count 300` exits 1; TC-1386 Step 1 expects no finding. | qsl-jit/tests/it/parity.rs:10-34 | correct-requirement-no-evidence |
| FND-007 | medium | unsafe-confinement fails open: unparsable files are skipped, lowering attributes are only checked on fn/mod/use items (inner #![allow(unsafe_code)] or one on an impl passes), and `[lints] workspace = true` counts as forbidden though the root has no [workspace.lints]. | tools/arch-lint/unsafe_confinement.rs:149-211; tools/arch-lint/unsafe_confinement.rs:242-244; tools/arch-lint/unsafe_confinement.rs:256-258; tools/arch-lint/unsafe_confinement.rs:33-51 | implementation-bug-despite-evidence |
| FND-008 | medium | call_metered returns InternalFault jit-entry-not-prepared for an existing function that was not selected, where the interpreter's prepared value runs it; a caller condition is reported as a backend defect. | qsl-jit/src/prepared.rs:390-395; qsl-eval/src/execution.rs:556-581 | implementation-bug-despite-evidence |
| FND-009 | low | Silent fallbacks in code generation (stack slot size unwrap_or(8), offset unwrap_or(0), op index unwrap_or(i64::MAX)) would emit wrong code instead of refusing. | qsl-jit/src/translate.rs:568-575; qsl-jit/src/translate.rs:689 | implementation-bug-despite-evidence |
| FND-010 | low | Generated entry symbol is qsl_entry_{name with :: replaced by _}, so distinct names such as a::b_c and a_b::c collide and the second define_function faults the whole prepare. | qsl-jit/src/prepared.rs:179; qsl-jit/src/translate.rs:199-206 | implementation-bug-despite-evidence |
| FND-011 | low | Stale or dead text and code: lifecycle test says make arch-lint-unsafe does not exist; exec doc names Ctx::failed; EntryFrame.kernel is never read; rust_files has a dead `top` param; EmptyComparison is pushed past max_reported; `units.len() as u64`; the parity target splits the arch-lint comment block in the Makefile. | qsl-jit/tests/it/lifecycle.rs:93-95; qsl-jit/src/exec.rs:13; qsl-jit/src/exec.rs:61; tools/arch-lint/unsafe_confinement.rs:111-112; qsl-conformance/src/lib.rs:444-458; Makefile:383-390 | implementation-bug-despite-evidence |
