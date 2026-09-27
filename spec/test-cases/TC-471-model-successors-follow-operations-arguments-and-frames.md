---
id: TC-471
title: "Model successors follow operations, arguments, frames and contracts"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: verifies
---
# TC-471: Model successors follow operations, arguments, frames and contracts

## Description

Verify the model-level successor relation: enabled applications, receiver
and argument identities, frame post-states with creations and deletions,
closure, postcondition filtering and the state key's `semantic` form.

Scope: FR-120-AC-1, FR-120-AC-2, FR-120-AC-3, FR-120-AC-4.

## Test Procedure

Integration tests in `qsl-eval/tests/it/`. Build FR-120's `test/counters`
domain package (package identity `test/counters`, nodes
`test/counters/Counter`, `test/counters/Counter/value`, `…/label`, `…/next`,
`test/counters/counters` and `test/counters/Counter/<operation>`) as a
hand-authored domain package in TC-458's fixture shape, with the operations
each step names, selected as model alias `Counters`. The bound scalar types
need QSL-289 (FR-056's `value-type/v1` scalar reader). Compile each unit through S1 to S4 in memory
(`PackageDeclarations::check`, then `CheckedPackage::link`), with no S5
emission. Initial snapshots are in-memory provisions, anchor
`{initialization, start}`. Read successors through
`ModelSystem::successors` and keys through `ModelSystem::key`.

1. `increment()` (frame `modifies value`), no clause: expand `s0`. Then add
   `post Inc` and expand `s0`. Then, without `Inc`, add `pre CanInc` and
   expand `v0`, `v1` and `v2`.
2. `setTo(n: Int[0, 2])` (frame `modifies value`, post `self.value = n`):
   expand `s0`; then `s0` plus `c2` (`value` 0, `label` 0, `next` absent).
   Then add `pre A { self.value >= 1 }` and `pre B { self.value <= 1 }` and
   expand `v0`, `v1` and `v2`.
3. `spawn()` (frame `creates Counter`), no clause: expand `s0`.
   `remove()` (frame `deletes Counter`), no clause: expand the state
   holding `c1` (`next` naming `c2`) and `c2` (`next` absent).
4. Read the key of `s0` and of `s0` with `c1.label` 1.

Tag the tests `#[trace("TC-471", "FR-120-AC-n")]`.

## Expected Results

- Step 1: three successors `v0`, `v1`, `v2`, each with transition identity
  `{"type":"transition","operation":"{\"digest_domain\":\"sha256-jcs\",\"node\":\"test/counters/Counter/increment\",\"package\":\"test/counters\"}","arguments":[<c1>]}`,
  `label` 0 and `next` absent. With `Inc`: one successor, `v1`. With
  `CanInc` alone: three successors from `v0` and from `v1`, none from `v2`.
- Step 2: three successors from `s0`, with arguments `[<c1>,
  {"type":"integer","value":"0"}]`, `…"1"}]` and `…"2"}]`, reaching `v0`,
  `v1`, `v2`. With `c2`, each of `c1` and `c2` has its own three
  identities. With `A` and `B`: `setTo` successors at `v1`
  only, none at `v0` or `v2` (each has one `Completed(false)` clause), and
  no finding.
- Step 3: `spawn` gives 19 successors: `s0`, and 18 states adding `c2` with
  every (`value`, `label`, `next`) in {0, 1, 2} × {0, 1} × {absent, `c1`,
  `c2`}. `remove` gives, per receiver, three successors: deletion sets {},
  {`c1`} and {`c1`, `c2`}; none deletes `c2` alone. Each successor's
  created and deleted sets equal the delta `StateModel::check_frame`
  returns.
- Step 4: `semantic` is exactly FR-120-AC-4's JCS value, with `<U>` the
  `UniverseId` of `counters` and `<E>` the `EffectiveId` of `Counter`, each
  as 64 lowercase hex digits; its population key is
  `{"type":"text","value":"{\"digest_domain\":\"sha256-jcs\",\"node\":\"test/counters/counters\",\"package\":\"test/counters\"}"}`
  and `c1`'s `identity` is `6331`. `control`, `queues`, `roles`,
  `observations` and `bounds` are `{"type":"map","entries":[]}`. The two
  keys differ.

## Status

Planned (QSL-274); runnable once QSL-289 lands.
