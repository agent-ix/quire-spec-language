---
id: SR-13338
title: "Code review (Rust lane) of QSL-907 guard facts for deref(r).f in a function body (PR #1051)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6cce08195; qsl-semantics/src/check/facts.rs, qsl-semantics/tests/it/state_clauses.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1084
    type: reviews
---
# SR-13338: Code review (Rust lane) of QSL-907 guard facts for deref(r).f in a function body (PR #1051)

## Summary

Ticket: QSL-907. PR quire-spec-language#1051, head 6cce08195, branch qsl-907-guard. The review covers `git diff origin/main...HEAD` (2 files, +52/-1). origin/main is a2d68af2e, which is an ancestor of the head. The rust-review lane is folded into this file. The worktree was not edited. Probes ran in a scratch clone of 6cce08195 with its own target directory. The probe patch is saved at /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1051/probe-907.patch. Each probe ran twice: once on the PR head, and once with origin/main's `facts.rs`.

Gates run on the clone:

- `cargo fmt --all -- --check`: clean.
- `cargo clippy -p qsl-semantics --all-targets -- -D warnings`: clean.
- `cargo test -p qsl-semantics`: all pass (lib 674, `it` 457, 23). Every existing state-clause test is unchanged and green.
- `cargo test -p qsl-replay -p qsl-package --lib`: all pass.
- `make ci`: not run, per the brief.
- `arch-lint api-surface --qsl . --qsl-only`: fails FR-100-AC-8 on `qsl-replay/src/spine/clause.rs` (`enum ClauseDisposition`). An origin/main a2d68af2e checkout gives the same result, so the failure is pre-existing and not this PR's.

## Verdict

**FAIL (not mergeable as is).** FND-001 is a reproduced soundness regression. The PR makes `deref(r).f` a stable path in every function body, and the termination checker reads stable paths too. It now admits a structural-recursion measure that decreases through a model read, and such a recursion can loop forever.

The range-fact change itself is sound. These points were probed on the PR head, with origin/main as the control:

- A function body is one fixed model state. `pre(...)` in a function body is refused `wrong_snapshot`/`forbidden-pre-read`, so keying every read on `Observation::Current` cannot mix two states.
- Guard facts follow the connectives correctly:
  - A guard holds in the right operand of `and`, the right operand of `implies`, the `then` branch of `if`, and the negated side of `or` and `not`.
  - A guard gives no fact in `(g implies true) and ...`, in `(g or n = 0) and ...`, in the `else` branch, or on the wrong side of `or`.
- Facts do not leak out of quantifiers. A guard inside `forall(r in xs: ...)` or `exists(r in xs: ...)` gives no fact after it. A guard on the binder's own `deref(r).f` inside the body does discharge that body's sum.
- Facts do not cross a call. `g(earlier) and ... deref(earlier).versionNumber + 1` still refuses `unproved-range` when `g` is the guard.
- Aliasing:
  - A guard on `deref(later).f` gives no fact for `deref(earlier).f`. Dropping the fact when `a` and `b` happen to alias is sound, because it is only conservative.
  - Keeping a fact across references would need two roots to share a path. Only `let b = a` does that, and there `b` really is `a`.
  - `let e = deref(r).f` aliases the stable path, and a guard on `e` (or on the path, then a use of `e`) discharges correctly.
- Shadowing cannot reach the fact table. `let earlier = later in ...` and `forall(earlier in xs: ...)` are refused `ambiguous_declaration`/`ambiguous-name` at resolution.

A let/shadowing test is therefore not required for soundness. Shadowing is refused before definedness runs, and a let alias uses the same per-slot alias table that state clauses already use.

The new test fails on origin/main's `facts.rs`, which the probe confirmed by running it. It asserts real refusal codes, not a tautology. No `unwrap`, panic or `unsafe` is added.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Termination regression. `Definedness::shape` classifies an argument as `ArgumentShape::Projection(p)` whenever `stable_path` returns a non-empty path rooted at parameter `p`, and `termination.rs` counts `Projection` as a structural decrease. Before this PR, `stable_path` returned `None` for an `Attribute` node in a function body. Now it returns a path through `Step::Attribute`, so a model read counts as a structural sub-term. Probe: record `Holder { owner: ConfigVersion }` plus `ConfigVersion.holder: Holder`, and `function walk using v(h: Config::Holder): Boolean pure decreases(h) { walk(deref(h.owner).holder) }`. The PR head returns Ok. origin/main refuses `undefined_expression`/`unproved-decrease`. In a model where object X has `holder = Holder{owner: X}`, `walk` calls itself on an equal argument forever. Fix: `shape` must return `Other` for a path that contains a `Step::Attribute`, since a dereferenced value is not a sub-term of the parameter. Add a regression test for this probe. | qsl-semantics/src/check/facts.rs:474-490, 725-728; qsl-semantics/src/check/termination.rs:553-556 |
| FND-002 | low | Three doc comments still say the attribute step exists only in a state clause, which this PR makes false: `Step::Attribute` ("A model attribute read in a state clause (FR-104)"), the `observations` field ("Only a state clause extends stable paths through model attribute reads and `pre(e)`"), and the `stable_path` comment ("FR-104: in a state clause a stable path also takes the steps..."). | qsl-semantics/src/check/facts.rs:44-47, 400-404, 470-473 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | In 0fa5ac4a6 (head 2ec367d9f), `stable_path` is back to main's form (`let observation = self.observations?.of_read(current.location())?;`). A new `range_path` adds the `deref(r).f` step under `Observation::Current` in a function body, and it feeds only `subject()`'s integer `Value` arm and the `IntervalWalk` leaf. `shape`, `Present`, `let` aliasing and the nonzero-map lookup keep `stable_path`. The Holder probe is refused `unproved-decrease` again. `recursion_through_a_deref_read_is_not_a_structural_decrease` fails with the 6cce08195 `facts.rs` and passes with main's, so it is a real regression test. |
| FND-002 | fixed | 0fa5ac4a6: `stable_path` no longer takes the attribute step outside a state clause, so the `observations` field doc and the `stable_path` comment are accurate again. `Step::Attribute` now reads "A model attribute read in a state clause (FR-104), or in a function body an integer range fact only (`range_path`)". |
