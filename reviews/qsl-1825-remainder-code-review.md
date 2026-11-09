---
id: SR-61009
title: "Code and Rust review of QSL-1825 remainder slice"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@e6929e1e5; git diff qsl-1825-native...qsl-1825-remainder: qsl-semantics/src/check/{mod,facts,family,region,protocol_controls}.rs and every mechanical PackageDeclarations::check caller (qsl-bench, qsl-conformance, qsl-eval, qsl-inspect, qsl-package, qsl-replay, qsl-semantics, src/population_fixtures.rs, tests/), qsl-jit/src/{exec,lib,prepared,relocations,translate}.rs, qsl-jit/tests/it/{call_targets,common,coverage,main,native,records,clause}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-495
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-560
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-561
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-562
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-568
    type: reviews
---
# Code and Rust review: QSL-1825 remainder slice

## Summary

Ticket: QSL-1825. Branch `qsl-1825-remainder` at e6929e1e5, two commits on
`qsl-1825-native`. Scope is that diff only. Methods: code-review with the
rust-review lane (rust-style loaded). gap-analysis was not run; the brief
scoped this pass to code-review and rust-review. The gates (`fmt`, `clippy`,
the crate tests, `make ci`) were not run by the reviewer, because a gate was
running in the same worktree and the brief forbade contending builds. Gate
evidence comes from that run, not from this review.

Examined: the `CheckMode` parameter on `PackageDeclarations::check` and its
four siblings, and how it reaches `ValueDeclarations` and `Definedness`;
`discharge_in_mode` and every `Unproved` obligation it can now suppress
(range, nonzero, presence, rational range, non-empty reduction); the
existing expression-level `CheckMode` handling at `mod.rs:8479`; that every
non-test caller passes `CheckMode::Linked` and that `Definedness::new` still
defaults to refusing, so Linked behavior is unchanged; the relocation
recorder (`RawBlock`, `classify`, `indirect_sources`, `check_call_targets`);
the stray-call test generator and its `test-support` gating; `unsafe` and
`extern "C"` placement against FR-568; the coverage, call-target, Kernel
result-bound and tuple tests against FR-560-AC-1/2, FR-561-AC-3,
FR-562-AC-6, TC-1380, TC-1383, TC-1384 and TC-1385.

## Verdict

**CONDITIONAL**: no high findings. Linked behavior is unchanged, and the
FR-561-AC-3 check and its stray generators do what the AC asks. The open
items are a second, divergent statement of the Kernel rule, untested Kernel
suppression beyond the result bound, one AC fixture that differs from the
spec, and inaccurate trace tags.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The Kernel rule now has two implementations that disagree. The expression-level check skips `Definedness` entirely under `CheckMode::Kernel`. The new package-level path runs it and suppresses only `CheckCause::Unproved`, so `InvalidDispatchDeclaration` still refuses in one path and not the other. A change to what Kernel means must be made in both places. Searched: `CheckMode` in qsl-semantics/src | qsl-semantics/src/check/mod.rs:8479, qsl-semantics/src/check/facts.rs:568-580 |
| FND-002 | medium | Under Kernel, the relaxation also covers the termination measure's definedness walk. The `check` doc and the `ValueDeclarations::mode` doc limit it to the function body, and no test covers an unproved measure obligation under Kernel | qsl-semantics/src/check/family.rs:1597-1603, qsl-semantics/src/check/mod.rs:1776-1781 |
| FND-003 | medium | Only one package-level Kernel behavior is tested: the `Coerce` result bound in TC-1384. Package-level suppression of the nonzero, presence, rational-range and non-empty-reduction obligations has no test. FR-495 makes zero-divisor `Undefined` under the package-level Kernel check an observable outcome | qsl-semantics/src/check/facts.rs:568-580, qsl-jit/tests/it/native.rs:538 |
| FND-004 | medium | The FR-560-AC-2 fixture differs from the AC. The AC names `half(a: Rational): Rational { a / 2 }`, but the test declares `a: Int[0, 9]`, so the Rational-parameter path is not exercised. `name` also uses a bounded `Text[0, 16; nfd]` rather than the AC's `Text` | qsl-jit/tests/it/coverage.rs:113 |
| FND-005 | low | The tuple-construction parity test is tagged `TC-1385`. TC-1385 is the FR-562-AC-4 fast-path charge-parity case, and tuple construction is not a fast-path operation. The bare tag binds no criterion, so the new test traces to nothing accurate | qsl-jit/tests/it/records.rs:155-157 |
| FND-006 | low | The tuple test's docstring promises a denied charge "at each point of the spend", but the loop is fixed at `0..16` and is not derived from the measured spend. TC-1385 step 2 uses one below the measured spend | qsl-jit/tests/it/records.rs:165 |
| FND-007 | low | `is_table_slot_load` accepts a load from any symbol's address as a `KernelTable` slot without checking that the symbol is the kernel table. FR-561-AC-3 asks for "a `KernelTable` slot of the same prepared value". No production block makes an indirect call, so the `TableSlot` arm is never exercised positively | qsl-jit/src/relocations.rs:207-222 |
| FND-008 | low | `stray_call` reports a declare failure through `emitter.refused`, but `refused` is read before `stray_call` runs. A `jit-declare-stray` failure is dropped silently, and the block is defined without the stray call | qsl-jit/src/translate.rs:649-655, qsl-jit/src/translate.rs:685-697 |
| FND-009 | low | Every production `prepare` walks relocations and IR for every block (`RawBlock::of`, `classify`) and stores `call_targets`, but only `test-support` code reads them. The field needs `cfg_attr(not(feature = "test-support"), allow(dead_code))` | qsl-jit/src/prepared.rs:90-92, qsl-jit/src/prepared.rs:214-220, qsl-jit/src/translate.rs:502-504 |
| FND-010 | low | Every relocation kind other than `Abs4`/`Abs8` is classified `Relative`, including GOT and PLT forms. The test then requires `KernelTable` targets to be `Absolute` and `Block` targets to be `Relative`, which ties the assertion to the current x64/aarch64 non-PIC lowering | qsl-jit/src/relocations.rs:130-133, qsl-jit/tests/it/call_targets.rs:43-48 |
| FND-011 | low | Under Kernel, `discharge_in_mode` also suppresses the `Rational` arm's operand-shape mismatch, which is a structural invariant reported as `Unproved(RationalRange)`. A malformed node would pass the Kernel check silently | qsl-semantics/src/check/facts.rs:1278-1283 |
| FND-012 | low | The Kernel test asserts equality with the interpreter and `Ok(None)`, but not that the cause is the result-bound range or the `Coerce` location. If both backends produced the same wrong cause, the test would still pass | qsl-jit/tests/it/native.rs:546-556 |
| FND-013 | low | Most of the 687 mechanical caller updates spell `quire_semantic_value::checking::CheckMode::Linked` inline instead of importing `CheckMode`, which adds about three lines per call site across 246 files | qsl-semantics/src/check/region.rs:821, qsl-eval/src/value/expression/mod.rs:2272 |

Clean, examined:

- **Linked unchanged:** `Definedness::new` initializes `kernel: false`, and
  `with_mode` is applied only in `check_declaration_body` (body and
  measure). Every state-clause, protocol, temporal, hyper and probabilistic
  `Definedness` is built without a mode. Every non-test caller in the diff
  passes `CheckMode::Linked`.
- **Kernel runtime soundness:** the interpreter re-checks rational domains
  (`evaluate_rational_arithmetic(.., Some(domain), ..)`). The JIT's `Coerce`
  matches the interpreter at 5 + 5, and `Int[0,9] + Int[0,9]` stays on the
  native path with no kernel call.
- **FR-561-AC-3:** relocation records are read from `CompiledCode` for every
  function and entry block. The direct stray (`qsl_jit_stray`, imported)
  classifies `Outside`, and the indirect stray (an `iconst` address)
  classifies `Register`. Both report the block and the target. The
  10,000-unit run shares one `OnceLock` generation with the TC-1380 test.
- **FR-568:** `stray` is a safe `extern "C"` function defined in `exec`, and
  it, `STRAY_SYMBOL` and `stray_address` are all `test-support`-gated.
  `relocations.rs` has no `unsafe`. The shim symbol is now one constant.
- **FR-560-AC-1:** the seven floor forms, the floor types and an explicit
  operator table per coverage class are asserted, with the FR-560-AC-5
  exclusions listed as negatives.
