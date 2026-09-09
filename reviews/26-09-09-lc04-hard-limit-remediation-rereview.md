---
id: SR-115
title: "Independent re-review of the LC04 hard-limit remediation"
type: SpecReview
analysis: code-review
scope: "PR #13 at d58ca7a; SR-114 FND-001..FND-004; src/lowering.rs; tests/native_lowering.rs, tests/native_backend.rs; FR-009"
review_set: subset
---

## Summary

Both actioned findings are closed, and the hard-limit fix is stronger than the
one SR-114 asked for: the ceilings are now bound per axis, so a single
off-by-one on `nodes`, `depth` or `bytes` fails, not just the wholesale
`usize::MAX` mutant. The diagnostic assertion now reads the real downstream
message and keeps the version tripwire while tolerating a rewording — verified by
printing the message the test actually sees. FR-009 gained the sentence
explaining that lowering's ceilings sit above the checker's, which is what keeps
a later reader from deleting the depth bound as dead. The remaining verdict is
driven entirely by two lows the owner deferred, not by anything new.

## Verdict

**CONDITIONAL** — no high or medium findings. Both findings this change targets
are closed; the two open lows are the owner-deferred FND-002 and FND-003 from
SR-114, carried forward unchanged.

## Gates run at `d58ca7a`

Rust 1.98.1, `-j 1`, `--test-threads=1`, isolated target directory.

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --locked --all-targets --all-features -j 1 -- -D warnings` | pass |
| `cargo test --locked -j 1` | pass — 266 ordinary tests plus doctests, 0 failures, 1 ignored (the activation lane) |

## Mutation evidence

Each mutant applied to `d58ca7a` and reverted; the tree was clean before and
after.

| # | Mutant | at `813069f` | at `d58ca7a` |
| --- | --- | --- | --- |
| M2 | All three defaults raised to `usize::MAX` | all 266 pass | **FAIL** — `exact_limits_and_fresh_retries` at `tests/native_lowering.rs:225` |
| M2a | `nodes` 10,000 → 10,001 only | not probed | **FAIL**, same assertion |
| M2b | `depth` 64 → 65 only | not probed | **FAIL**, same assertion |
| M2c | `bytes` 16 MiB → 16 MiB + 1 only | not probed | **FAIL**, same assertion |

The three single-axis mutants matter. A tuple assertion can be written so that
one axis carries the whole check; here each of the three constants is
individually bound, which is what FR-009's three separate numbers require.

For FND-004 I checked the assertion is live rather than vacuously true, by
temporarily inverting it and reading the message the test actually receives:

```
expected cargo-llvm-cov 0.9.0 / llvm.coverage.json.export 3.0.1;
found 0.9.0 / llvm.coverage.json.export 3.1.0
```

Both substrings are genuinely present, so `contains("3.0.1") &&
contains("3.1.0")` keeps the version tripwire while surviving an upstream
rewording — which was the whole point of the finding.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | Deferred from SR-114: the CI job gains a from-source tool build while `timeout-minutes: 10` is unchanged | .github/workflows/ci.yml | correct-requirement-no-evidence |
| FND-002 | low | Deferred from SR-114: `docs/dependency-licenses.json` is referenced by no test, tool or CI step | docs/dependency-licenses.json | missing-requirement |

Both are the owner's deferral, recorded so they are not lost rather than
re-argued. Neither blocks this change, and neither is affected by it.

## Closure detail

### SR-114 FND-001 (medium) — the ceilings are bound

`tests/native_lowering.rs:222-227`:

```rust
let hard = LoweringLimits::default();
assert_eq!(
    (hard.nodes, hard.depth, hard.bytes),
    (10_000, 64, 16_777_216)
);
```

Two lines, placed at the top of the test that already governs limit behaviour,
and they close the gap per axis. Everything that made
`exact_limits_and_fresh_retries` a good test — zero, one-below and exact on all
three axes, a fresh successful retry after each exhaustion, and the `usize::MAX`
clamp — is untouched; what was missing was a statement of what the default *is*,
and now there is one.

### SR-114 FND-004 (low) — the prose pin is gone, the tripwire is not

```rust
let message = report["diagnostics"][0]["message"].as_str().unwrap();
assert!(message.contains("3.0.1") && message.contains("3.1.0"));
```

`assert_eq!(report["state"], "unsupported")` and
`assert_eq!(report["diagnostics"][0]["code"], "unsupported_profile")` are both
unchanged and still precede it, so the deliberate "update the qualification when
the backend changes" tripwire is intact: when codegen accepts export 3.1.0 the
`state` assertion fails first, exactly as before. What changed is that an
upstream sentence rewrite no longer fails this repository for a non-behavioural
reason.

### The FR-009 addition is the right kind of clarification

> The native checker also caps source nodes at 10,000 and depth at 64; lowering
> retains these ceilings as an additional guard over already checked packages.

This records what `src/checking.rs:68` and `src/linking.rs:44` already implement
and what SR-114 observed: a package deep enough to trip lowering's depth ceiling
is refused two stages earlier, so that bound is defence-in-depth rather than the
binding constraint. Stating it is what stops a later reader deleting it as dead
code, and it is now covered by the constant assertion either way.

## One observation, not a finding

The deferral is recorded only in the PR body: "Hosted-CI timing and
dependency-inventory automation remain deferred low findings." That is the right
disclosure and it is accurate — but a PR body is not part of the record after
merge, and the same is true of this change's other cross-reference to SR-114.
`plan/Plan-008-native-lowering/plan.md` already carries the LC04 accounting and
would keep both items where the next author looks.

## Unchanged and re-confirmed

- The backend qualification still does real work; nothing in this change touches
  the lowering itself, and the full suite reproduces green at `d58ca7a`.
- `TM-006` still marks FR-009-AC-5 and TC-094 `⛔ Activation pending`, Task-020
  is still `in_progress` with unchecked subtasks, and the required activation
  lane is still an explicit `#[ignore]` naming its cause. Nothing was quietly
  reclassified while the review findings were being closed — which is the thing
  most worth checking in a remediation pass, and it holds.
