---
id: TC-1040
title: "A clause evaluation reports its receiver path and the runtime value path of each field read"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: verifies
---
# TC-1040: A clause evaluation reports its receiver path and the runtime value path of each field read

## Description

Verify FR-107's `receiver_path()` and `reads()`: which selections record an
entry, in what order, and the QSpec FR-207 runtime value path of each: the
observation, the object, receiver, parameter or built-collection subject, and
the member, field, option-value, reference and index steps.

Scope: FR-107-AC-7, FR-107-AC-8, FR-107-AC-9, FR-107-AC-10, FR-107-AC-11,
FR-107-AC-12, FR-107-AC-17.

## Test Procedure

Admit each clause's observations by FR-106 and evaluate it by `evaluate_clause`
with an unlimited meter unless a step says otherwise. A path is written
`(observation, subject, steps)`. `object(K)` is the declared object identity of
the object with key `K`: the universe of its population, its type and `K`.
The observation `H` is the current snapshot `healthy-parent` of FR-108 (`root`
1, `child` 2 with parent `root`) and `A` its current snapshot `absent-parent`.
Tag the tests `#[trace("TC-1040", "FR-107-AC-n")]`.

**Object receiver (FR-108 `Config`).**

1. `ParentOrder` over `H`, `self` `child`: `receiver_path()`; `reads()`.
2. `ParentOrder` over `A`, `self` `root`: `reads()`; and over `H` with a
   meter whose work budget is 0: `reads()`.
3. `VersionUnchanged` over changed-version (pre snapshot `R`, `child` 2; post
   snapshot `P`, `child` 3), `self` `child`: `reads()`.
4. Add to the unit the operation `probe(target: ConfigVersion)` with no result
   and an empty frame (TC-466 step 3) and the clause `pre ProbeOrdered using v
   on Config::ConfigVersion::probe { self.versionNumber <
   target.versionNumber }`. The pre snapshot `W` holds `a` (version 1) and `b`
   (version 2), no parents; the invocation `i1` has `self` `a` and `target`
   `b`. Evaluate `ProbeOrdered` selected by `Invocation` (pre snapshot `W`),
   then selected by `PreCall` (snapshot `W`, `self` `a`, `target` `b`):
   `receiver_path()` and `reads()` of each.
5. FR-265's `witness` unit, snapshot `high` (`root` 0, `mid` 600 with parent
   `root`, `leaf` 700), `GuardedAll` with `self` `mid`: `reads()`.

**State machine (the `Orders` fixture, built as the TC-235 runtime tests'
business-model package with these declarations).** The frame of `place`
modifies both `placed` and `cancelled`, because admission refuses the post
state of `i1` otherwise. State machine `OrderLifecycle` with
the context fields `placed` and `cancelled`, both `Boolean`; the record value
type `Flags` with the field `placed: Boolean`; the operation `place(flags:
Flags)` with no result; and the clauses

```text
invariant Consistent using v on M::OrderLifecycle at current { not (self.placed and self.cancelled) }
pre PlaceRequires using v on M::OrderLifecycle::place { not self.placed }
post FlagsPlaced using v on M::OrderLifecycle::place { flags.placed }
```

The invocation `i1` of `place` binds `flags` to `{placed: true}`; its pre
snapshot `p1` holds the context `{placed: false, cancelled: false}` and its post
snapshot `q1` the context `{placed: true, cancelled: true}`.

6. `Consistent`, its observation constructed at the model layer (FR-107
   "The receiver path") from the invocation `i1` and its pre context `p1` and
   post context `q1`, not admitted from a snapshot document: `receiver_path()`; `reads()`.
7. `FlagsPlaced`, admitted by FR-106 over `i1`: `reads()`; then compare its path with the path of
   the `Consistent` read of `self.placed`, and serialize both by QSpec FR-207's
   wire form.
8. `PlaceRequires` over `i1` (pre snapshot `p1`), then over a pre-call
   observation (snapshot `p1`, no invocation): `receiver_path()`; `reads()`.

**Navigation (the TC-1033 `orders` fixture).** Tank gains the field `level:
Int[0, 100]`; `t1` holds 10; the extents are those of TC-1033 step 1, complete,
and `p1` is housed by `t1`.

9. `invariant TankLevel using v on Orders::Pump at current { self.home.level <
   50 }` over a current snapshot `N` with `self` `p1`: `reads()`.

## Expected Results

- Step 1: `receiver_path()` is `(H, object(child), [])`. `reads()` holds four
  entries, in order, for the selections `self.parent` (the `present` operand),
  `self.parent` (the `value` operand), `deref(value(self.parent)).versionNumber`
  and `self.versionNumber`, with the paths
  `(H, object(child), [member parent])`,
  `(H, object(child), [member parent])`,
  `(H, object(child), [member parent, option_value, reference object(root),
  member versionNumber])` and
  `(H, object(child), [member versionNumber])`. `value`, `deref` and `present`
  record no entry.
- Step 2: over `A` one entry, `self.parent`, `(A, object(root), [member
  parent])`, because `implies` skips its right operand; with the zero budget
  no entry, because the meter denies a charge before any read.
- Step 3: two entries, `self.versionNumber` `(P, object(child), [member
  versionNumber])` and then `pre(self.versionNumber)`'s selection `(R,
  object(child), [member versionNumber])`.
- Step 4: selected by `Invocation`, `receiver_path()` is `(W, object(a), [])`
  and `reads()` holds `self.versionNumber` `(W, object(a), [member
  versionNumber])` then `target.versionNumber` `(W, invocation i1, parameter
  target, [reference object(b), member versionNumber])`. Selected by `PreCall`,
  `receiver_path()` and the first entry are the same and the second entry has no
  path.
- Step 5: four entries: the two `self.parent` selections, each `(high,
  object(mid), [member parent])`; then `c.versionNumber` for `c` the first
  element of the built list `(high, built(L), [index 0, reference object(root),
  member versionNumber])`, and for the second element `(high, built(L), [index
  1, reference object(mid), member versionNumber])`, where `L` is the list's
  occurrence key (FR-265-AC-1); the `forall` stops at the second element.
- Step 6: `receiver_path()` is `(q1, invocation i1, receiver, [])`. `reads()`
  holds `self.placed` `(q1, invocation i1, receiver, [field placed])` and
  `self.cancelled` `(q1, invocation i1, receiver, [field cancelled])`; the
  clause is `Completed(false)`.
- Step 7: one entry, `flags.placed`, `(q1, invocation i1, parameter flags,
  [field placed])`. The two `placed` paths are unequal and the parameter path
  orders before the receiver path. The receiver path serializes as
  `{kind: runtime_value, root: {observation: q1, subject: {tag: invocation,
  invocation: i1, binding: receiver}}, steps: [{tag: field, name: placed}]}` and
  the parameter path with the subject `{tag: invocation, invocation: i1,
  binding: parameter, parameter: flags}`.
- Step 8: over `i1`, `receiver_path()` is `(p1, invocation i1, receiver, [])`
  and one entry, `self.placed`, `(p1, invocation i1, receiver, [field placed])`;
  over the pre-call observation, `receiver_path()` is none and the one entry
  has no path.
- Step 9: one entry, `self.home.level`, `(N, object(t1), [member level])`; the
  navigation `self.home` records none.
