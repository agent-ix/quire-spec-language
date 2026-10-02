---
id: SR-114
title: "Independent code review of the LC04 Boolean lowering"
type: SpecReview
analysis: code-review
scope: "PR #13 at 813069f; src/lowering.rs, src/lowering/wire.rs; tests/native_lowering.rs, tests/native_backend.rs; FR-009, IT-008, TC-092..TC-094, TM-006, Plan-008"
review_set: subset
---

## Summary

The lowering is careful and the qualification is real. `cargo test --locked`
reproduces 266 passing tests with one ignored lane, and the backend test is not
a paper exercise: mutating `BinaryOp::And` to lower as `ShortCircuitOr` makes the
*compiled generated program's* output disagree with the independent truth
equation and fails at `tests/native_backend.rs:282`. The blocked activation gate
is narrower than the PR body's framing suggests — truth parity across all eight
assignments and native implication activation both pass today; only the
*generated* activation counts are blocked, and the matrix says so. One medium:
the three hard ceilings FR-009 states normatively are bound by no test.

## Verdict

**CONDITIONAL** — no high findings. One medium, three lows.

## Gates run at `813069f`

Rust 1.98.1 (the pinned toolchain), `-j 1`, `--test-threads=1`, isolated target
directory.

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --locked --all-targets --all-features -j 1 -- -D warnings` | pass |
| `cargo test --locked -j 1` | **266 passed, 0 failed**, 1 ignored (the activation lane) |
| `tests/native_backend.rs` alone | 1 passed, 1 ignored, 1.28 s |
| `cargo llvm-cov --version` | `cargo-llvm-cov 0.9.0`, as IT-008 requires |
| `docs/dependency-licenses.json` vs `Cargo.lock` | 140 entries against 140 `[[package]]` blocks — the "140 packages" claim is accurate |

## Mutation experiments

Each applied to `813069f` and reverted; the tree was clean before and after.

| # | Mutant | Result |
| --- | --- | --- |
| M1 | `BinaryOp::And` lowered as `ShortCircuitOr` | **FAIL** — `generated_truth_matches_all_assignments…` at `tests/native_backend.rs:282` |
| M2 | `LoweringLimits::default()` raised to `usize::MAX` on all three axes | **all 266 pass** |

M1 settles a question the timing raises. The backend test finishes in 1.28 s,
which looks far too fast for "compile generated Rust, run it eight times, run
`cargo llvm-cov report` eight times". It is not too fast — the generated crate is
three files with no dependencies, and the mutation proves the loop genuinely
compiles, executes and compares. Worth recording so the next reader does not
re-derive the same doubt.

M2 is FND-001.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The three hard lowering ceilings FR-009 states normatively are bound by no test | src/lowering.rs:32, tests/native_lowering.rs:222 | correct-requirement-no-evidence |
| FND-002 | low | The CI job gains a from-source tool build and the generated-crate qualification while `timeout-minutes: 10` is unchanged | .github/workflows/ci.yml | correct-requirement-no-evidence |
| FND-003 | low | `docs/dependency-licenses.json` is referenced by no test, tool or CI step | docs/dependency-licenses.json | missing-requirement |
| FND-004 | low | A downstream crate's verbatim prose diagnostic is pinned beside the durable code assertion | tests/native_backend.rs:313 | correct-requirement-no-evidence |

## Finding detail

### FND-001 — the ceilings are documented, clamped, and unbound

FR-009 states them normatively: "The compiler shall bound lowering to 10,000
visited native nodes, depth 64 and 16 MiB serialized output, with caller limits
able only to lower these ceilings." `LoweringLimits` repeats them in doc
comments ("at most 10,000", "at most 64").

Measured: replacing

```rust
nodes: 10_000,
depth: 64,
bytes: 16_777_216,
```

with `usize::MAX` on all three axes leaves **every one of the 266 tests green**.

`exact_limits_and_fresh_retries` is a good test and it is not the problem — it
proves zero, one-below and exact behaviour on all three axes, proves a fresh
retry succeeds after each exhaustion, and proves `bounded()` clamps a
`usize::MAX` request. But every one of those assertions is *relative* to
`LoweringLimits::default()`: the `above` case asserts
`lower(&package, above).usage() == projection.usage()`, which holds for any
default value. Nothing states what the default is.

Two lines close it:

```rust
let hard = LoweringLimits::default();
assert_eq!((hard.nodes, hard.depth, hard.bytes), (10_000, 64, 16_777_216));
```

A fixture that actually exceeds a ceiling under `usize::MAX` caller limits would
be stronger still, though only for `nodes` and `bytes`: `depth: 64` mirrors
`src/checking.rs:68` and `src/linking.rs:44`, so a package deep enough to trip
lowering's depth ceiling is refused two stages earlier. That makes lowering's
depth bound defence-in-depth rather than the binding constraint — which is the
right design, and worth one sentence in FR-009 so a later reader does not delete
it as dead.

### FND-002 — the CI budget did not move with the work

This PR adds two steps to the job:

```yaml
- run: rustup toolchain install 1.98.1 … --component llvm-tools-preview
- run: cargo install cargo-llvm-cov --version 0.9.0 --locked
```

`cargo install … --locked` builds the tool from source, and the existing
`cargo test --locked --no-default-features` step now additionally compiles the
generated crate under `-C instrument-coverage` and runs eight `cargo llvm-cov
report` invocations. `timeout-minutes: 10` is unchanged in the diff.

The job is `workflow_dispatch`-only, per repository policy, so nothing fails
silently and no PR is gated on it — which is why this is low rather than higher.
But the first person to dispatch it is likely to hit a timeout that has nothing
to do with their change. Either raise the budget or cache the installed tool.

### FND-003 — a hand-maintained inventory with no gate

`docs/dependency-licenses.json` carries 140 entries and `Cargo.lock` carries 140
`[[package]]` blocks, so the inventory and the "The LC04 lock selects 140
packages" sentence are both accurate today — I checked. Nothing keeps them that
way: `grep` across `tests/`, `src/`, `tools/` and `.github/` finds no reference
to the file. `fixture-audit` covers producer fixtures, not the dependency
inventory.

Pre-existing, and correctly updated by this PR — including the two
revision-qualified git sources, which are exactly the entries a regeneration
would be most likely to get wrong. Recorded because this is the first change to
make the inventory materially harder to maintain by hand, and because a
comparison against `cargo metadata` is a small test in a repository that already
runs `fixture-audit` as a CI step.

### FND-004 — a prose string pinned across a repository boundary

```rust
assert_eq!(report["diagnostics"][0]["code"], "unsupported_profile");
assert_eq!(report["diagnostics"][0]["message"], "expected cargo-llvm-cov 0.9.0 / llvm.coverage.json.export 3.0.1; found 0.9.0 / llvm.coverage.json.export 3.1.0");
```

The `code` assertion is the durable one and is already present. The `message`
assertion pins a human-readable sentence owned by `quire-contract-codegen`, so a
pure rewording upstream fails this repository's suite for no behavioural reason.

Pinning the refusal *at all* is right — the comment "update the qualification
when the backend changes" makes this a deliberate tripwire, and it is the honest
alternative to waiving the gate. The version numbers inside the message are the
part worth keeping; asserting that the message contains `3.0.1` and `3.1.0`
rather than equalling the whole sentence keeps the tripwire and drops the
brittleness.

## Checked and found sound

These are the things I expected to be defects and were not. Recorded so the next
reviewer does not spend the same time.

- **Read deduplication cannot lose an observation.** `read()` uses
  `self.used.entry(name).or_insert_with(…)`, so a second read of one declaration
  keeps the first `ProjectedRead`. That would under-report correspondence if two
  reads of one name in one clause could carry *different* observations. They
  cannot: the only construct that would produce one is `pre`, which is an
  `ExprKind` builtin, and `expression()` admits only `Group`, `Boolean`, `Name`,
  `Unary{Not}` and `Binary{And,Or,Implies}` — every builtin falls to the `_` arm
  and refuses the whole projection. The dedup is safe by construction, and the
  test's `assert_eq!(projection.reads().len(), 1, "repeated occurrences share
  one projected declaration")` says so deliberately.
- **The hand-written wire serializer is checked.** `src/lowering/wire.rs` encodes
  the IR wire shape by hand rather than reusing the IR's serializer, which is
  where drift would normally hide. `lower()` immediately re-parses its own output
  with `ir::BoundPackage::from_json_bytes`, the format string comes from
  `ir::EXECUTABLE_PROJECTION_FORMAT` rather than a literal, and M1 shows operand
  semantics are caught downstream. The `_` arm returns a serde error rather than
  emitting a partial node.
- **Byte-limit classification is correct.** `Output::write` raises
  `io::Error::other`, serde wraps it as an Io error, and `lower()` maps
  `error.is_io()` to `ResourceExhausted` — asserted by the `bytes: exact.bytes - 1`
  case. A non-io serde error maps to `InvalidCorrespondence`, which is the right
  split.
- **The refusal set is broad and asserts non-mutation.** Five unsupported
  expression forms, a later-clause refusal, and two owner-population violations
  each assert the code, the authored clause identity, a source span, and that
  `package.bytes()` is unchanged.
- **AC-4 is tested from both sides**: state-versus-input changes the bound
  digest, while an unrelated native model change (`scalars[0].maximum = 1001`)
  changes `canonical_identity()` and leaves the bound digest equal.
- **The dependency split is honest.** `quire-contract-codegen`,
  `quire-contract-ir-backend` and `syn` are all in `[dev-dependencies]`;
  production depends only on IR `690bde7`. The two-IR arrangement is verified,
  not asserted — `consumer.digest() == projection.bound().digest()` proves both
  revisions bind the same bytes to the same identity.
- **The blocked gate is scoped correctly.** `TM-006` marks FR-009-AC-5 and
  TC-094 `⛔ Activation pending` and everything else `🚧 Tested; PR gate
  pending`; Task-020 is `in_progress` with unchecked subtasks; the README gives
  the exact failing command and says it "is required before claiming complete
  backend parity". The ignored lane is `#[ignore]`d with a reason naming the
  cause. Nothing is counted as passing or waived.

## One clarification worth making in the PR body

"Draft: generated activation acceptance is blocked" reads as though the backend
qualification is unverified. It is narrower than that, and the narrower statement
is stronger. In the lane that runs today:

- all eight assignments of the compiled generated program are compared against
  native evaluation *and* against independently written truth equations;
- native implication activation counts are compared against independently
  written `expected_entries` for all eight assignments;
- only the *generated* activation counts — the ones that need the LLVM coverage
  export — are unreachable.

Saying that explicitly separates "we cannot yet measure generated activation"
from "we do not know whether the generated code is correct", and the evidence
supports the first only.
