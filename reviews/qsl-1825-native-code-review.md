---
id: SR-61008
title: "Code and Rust review of QSL-1825 native fast-path slice"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@c46a1603c; git diff qsl-1825-collection...qsl-1825-native: Cargo.lock, qsl-jit/src/{kernel,lib,prepared,translate}.rs, qsl-jit/tests/it/{main,native}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-561
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-562
    type: reviews
---
# Code and Rust review: QSL-1825 native fast-path slice

## Summary

Ticket: QSL-1825. Branch `qsl-1825-native` at c46a1603c, three commits on
`qsl-1825-collection` (the reviewed collection slice). Scope is that diff
only. Methods: code-review with the rust-review lane (rust-style loaded). No
PR exists for the branch. gap-analysis was not run; the brief scoped this pass
to code-review and rust-review. The AC-to-test check below was done by hand.

Examined: every machine operation the emitter now generates (`iadd`, `isub`,
`imul`, `ineg`, `sdiv`/`srem` with the floor and Euclidean fix-ups, `icmp`
for the six comparisons) against the prepare-time interval rule; the
`ChargeNative` shim and the `NativeIntegerSchedule` it builds for each
operator; the order of charge, stop and machine operation; the recording
kernel table (`kernel_name`, the recorder hook in `kernel::run`,
`call_recorded`/`call_inner`/`call_with`); the `test-support` feature wiring;
the `Cargo.lock` change; and the seven tests in `tests/it/native.rs` against
FR-561-AC-1/2 and FR-562-AC-1..5. `cargo test -p qsl-jit --test it -- native`
passed (7 tests).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `rem` and `mod` on the fast path compute `sdiv` unconditionally. Cranelift `sdiv` traps when the dividend is i64::MIN and the divisor is -1. `division_interval` gives `rem`/`mod` the interval (-(max abs divisor - 1), +same), which is [0, 0] for a divisor of [-1, -1], so `a rem b` with `a: Int[-9223372036854775808, 0], b: Int[-1, -1]` is translated natively. The call with (i64::MIN, -1) traps and kills the process, but the interpreter returns 0. No trap handler exists in qsl-jit | qsl-jit/src/translate.rs:1607, qsl-jit/src/translate.rs:1797-1803, qsl-jit/src/translate.rs:822-839 |
| FND-002 | medium | The FR-562-AC-1 test declares `f` with `b: Int[0, 4611686018427387903]`, while the AC uses 4611686018427387904. The largest sum tested is 2^63 - 2, so the AC's exact edge (a sum of 2^63 - 1 stays on the fast path) is never exercised | qsl-jit/tests/it/native.rs:20, qsl-jit/tests/it/native.rs:172 |
| FND-003 | medium | The fixture admits only the floor law, and the only fast-path `div` test (`d`) uses non-negative operands. The truncating and Euclidean branches of `machine_division` never run, and neither does the floor quotient step. A wrong sign fix-up for `div` (for example, Euclidean `up`/`down` swapped) would pass every test | qsl-jit/src/translate.rs:1600-1657, qsl-jit/tests/it/common.rs:66-70, qsl-jit/tests/it/native.rs:27 |
| FND-004 | medium | No test checks charge identity for most of the fast path. `agrees` runs only f, g, m, m2, n, d and d0, and only f, m and d of those are fast-path. The `ChargeNative` schedules for binary `-`, unary `-`, `rem` (Division schedule), `mod` (Modulus), the four orderings and `=`/`!=` are never compared with the interpreter's charges or `Incomplete` point. `values_agree` compares the result only. FR-562's charge rule is unverified for six of the nine operators | qsl-jit/tests/it/native.rs:405, qsl-jit/tests/it/native.rs:112-122, qsl-jit/src/kernel.rs:397-428 |
| FND-005 | low | `tuples_of` falls back to `digits(0, 9)` for `d`, so 10 of the 100 AC-4 tuples pass `b = 0` to `b: Int[1, 9]`. Those calls test argument refusal, not the fast path's charges | qsl-jit/tests/it/native.rs:394 |
| FND-006 | low | In the `*` arm, `NativeIntegerOperation::Multiply` and its two `native_interval` checks are built and then discarded, because `Multiply` uses `NativeCharge::Multiply` with no plan. If that conversion failed, it would fault prepare for a value nobody uses | qsl-jit/src/translate.rs:744-753 |
| FND-007 | low | The recorder plumbing has three layers: `call_metered`, then `call_inner`, then `call_with`. In production it carries an `Option<()>` slot and needs `cfg_attr(allow(unused_variables, unused_mut))`. `call_inner` exists only to hand the recorder back. Passing `Option<&mut KernelRecorder>` (or the slot by `&mut`) straight into `call_with` would remove one layer and the allow | qsl-jit/src/prepared.rs:442-466 |
| FND-008 | low | `KernelCall::operation` documents nine names, but `kernel_name` also yields `convert`. Unary `neg` records the Boolean placeholder `0` as a second operand, so the recorded operands of `neg` are not the kernel call's operands | qsl-jit/src/kernel.rs:62-63, qsl-jit/src/kernel.rs:109, qsl-jit/src/kernel.rs:376-385 |

## Verdict

FAIL on FND-001: a checked program crashes the process on the fast path where
the interpreter returns a value. Fix: emit `sdiv` only for
`DivisionMember::Quotient`. `srem` does not trap on MIN % -1, and the
quotient's result interval already excludes that case. Add `rem` and `mod`
tests at (i64::MIN, -1).

Clean, examined:

- **Add, subtract, multiply, negate:** the result-interval rule rules out
  wrap in `iadd`, `isub`, `imul` and `ineg`. `ineg` of i64::MIN is excluded
  because `fits(-high, -low)` fails. `imul` is i64-wide only when every
  corner product fits. Signed `icmp` is correct for mixed operand bounds.
- **Divide:** `div` at MIN / -1 goes to the kernel because the quotient
  interval does not fit. A zero divisor never reaches the machine path, so
  `Undefined` is always the kernel's, at the `div`'s location.
- **Floor and Euclidean fix-ups:** these cannot overflow when the divisor is
  nonzero and the case is not MIN / -1.
- **Charge order:** the charge shim runs after both operands and before the
  machine operation. An `Incomplete` or `Cancelled` from the charge leaves
  through `leave_on_failure` at the operation's location before any
  arithmetic, which is the interpreter's order.
- **Recording table:** it records only real kernel value operations.
  `ChargeNative`, Boolean operations, range checks and value handling are
  excluded, and a fault is injected before the kernel runs, so no trace is
  inflated by charge-only entries.
- **Cargo.lock:** only the quire-exact source line moved, to 7f1c7d7, the
  #140 merge and current quire-exact main.
- **Unsafe code:** no new `unsafe`, and `exec` is untouched.
- **`test-support`:** the feature is reached only through the self
  dev-dependency (resolver 2), and xtask depends on qsl-jit without it, so
  nothing leaks into production builds.
- **Accepted blocks:** FR-562-AC-6, FR-561-AC-3/TC-1383, the unbounded
  value-handle entry, the NodeKey accessor and EnvironmentRefused are outside
  this slice.

## Dispositions

Round 1, fix commits f7c8716ed and 305320d6b on c46a1603c. Checked against the
code at 305320d6b. `cargo test -p qsl-jit` and `cargo test -p qsl-jit
--features test-support` both pass (lib 4, it 39, plus 3). `cargo clippy -p
qsl-jit --all-targets` and `--lib` without the feature are clean.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `sdiv` is now emitted only for `DivisionMember::Quotient`. Cranelift `srem` does not trap on MIN % -1: x64 lowers it through `checked_srem_seq`, and aarch64 lowers it as `sdiv` (no overflow trap) plus `msub`, giving 0. The fast-path quotient is sound because the quotient is monotone in each operand over a one-signed divisor interval, and the MIN / -1 end of such an interval does not fit. The new test runs `rone`/`mone`/`rwide`/`mwide` natively at (MIN, -1) under all three laws and asserts which functions call no kernel operation |
| FND-002 | fixed | `f` now declares `b: Int[0, 4611686018427387904]`, and both ends are tested |
| FND-003 | fixed | `package_with_law` plus `division_under_every_law_agrees_with_the_interpreter` cover `div`/`rem`/`mod` under the floor, truncating and Euclidean laws, with both divisor signs, signed dividends, MIN, MAX and -1 |
| FND-004 | fixed | `agrees` now covers dr, dm, neg, sub, lt, le, gt, ge, same and differ, and the new division test compares charges |
| FND-005 | fixed | `tuples_of` gives `d` the tuples (0..=9, 1..=9) |
| FND-006 | fixed | The `*` arm no longer builds an unused operation |
| FND-007 | fixed | `call_inner` is removed, and the recorder parameter exists only under `test-support`. The unsupported-entry refusal that f7c8716ed dropped is restored in 305320d6b. The net diff keeps it at the top of `call_with` |
| FND-008 | fixed | The docs now list `convert`, and unary operations and conversions record one operand |
