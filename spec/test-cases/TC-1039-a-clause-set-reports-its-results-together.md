---
id: TC-1039
title: "A clause set reports its results together, evaluates postconditions only after true preconditions and refuses a bad set whole"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: verifies
---
# TC-1039: A clause set reports its results together, evaluates postconditions only after true preconditions and refuses a bad set whole

## Description

Verify FR-107's `evaluate_clause_set`: the entries it returns and their order,
that a postcondition is evaluated only after every precondition is true, the
combined outcome, the stop at an incomplete clause, and the refusals.

Scope: FR-107-AC-13, FR-107-AC-14, FR-107-AC-15, FR-107-AC-16, FR-107-AC-18, FR-107-AC-19, FR-107-AC-20.

## Test Procedure

Use FR-108's `Config` unit and domain package with these added clauses on
`ConfigVersion::attemptUpdate`, an operation with no parameter, the result
`Boolean` and the frame `modifies versionNumber`:

```text
pre UpdateAllowed using v on Config::ConfigVersion::attemptUpdate { self.versionNumber < 3 }
pre UpdateNotMax using v on Config::ConfigVersion::attemptUpdate { self.versionNumber < 1000 }
post UpdateRaises using v on Config::ConfigVersion::attemptUpdate { self.versionNumber - pre(self.versionNumber) = 1 }
```

Invocations of `attemptUpdate` with `self` `child` and result `true`; every
snapshot holds `root` (version 0, no parent) and `child`, and the table gives
`child`'s version:

| Invocation | Pre snapshot | Post snapshot |
| --- | --- | --- |
| `up12` | `child` 1 | `child` 2 |
| `up34` | `child` 3 | `child` 4 |

A current snapshot `cur` holds `root` 0 and `child` 1 with no parent, for
`NoCycle` (FR-108) with `self` `child`. Admit each clause's observations by
FR-106 (a precondition from the invocation's pre snapshot, a postcondition
from both) and call `evaluate_clause_set` with unlimited `limits` unless a
step sets one. Tag the tests `#[trace("TC-1039", "FR-107-AC-n")]`.

1. The set {`UpdateAllowed`, `UpdateRaises`} over `up12`, then over `up34`.
2. The set {`UpdateRaises`, `UpdateNotMax`, `UpdateAllowed`}, listed in that
   order, over `up12`, then over `up34`.
3. The set {`UpdateRaises`, `UpdateAllowed`, `NoCycle`} over `up34`.
4. The set of step 1 over `up12` with `limits` whose `work_units` is 0; the
   empty set; the set of step 1 over `up12` evaluated twice.
5. The set {`UpdateAllowed`, `UpdateAllowed`}; the set {`UpdateAllowed`,
   `sameIdentity`}; the set {`UpdateAllowed` with the observations admitted
   for `UpdateRaises`}.
6. The combination of verdicts, called directly as a unit test of the crate
   (no `Linked` clause evaluates to a kernel `Undefined` or `Refused`, FR-107
   Behavior): the sets {}, {true, true}, {true, false}, {false, X},
   {true, X}, {X}, {X, X}, {true, incomplete}, {false, incomplete} and
   {X, incomplete}, X being in turn a kernel `Undefined(DivisionByZero)` and
   a kernel `Refused`.
7. A self-contained fixture, not the `Config` unit: TC-792's, a model `M`
   with an object type `A` and a query operation `ready(): Boolean` whose
   effective precondition is `false` for the receiver `r1`, with the clauses
   `pre F using v on M::A::ready { self.ready() }`,
   `pre G using v on M::A::ready { true }` and
   `post H using v on M::A::ready { true }`, and an invocation of `ready`
   with `self` `r1` and result `true` whose snapshots hold `r1`. Call
   `evaluate_clause_set` with the sets {`F`}, {`G`, `F`} and {`H`, `F`}; then
   with the effective precondition `true`, the set {`F`}.

## Expected Results

- Step 1: over `up12` two entries, `UpdateAllowed` `Completed(true)` then
  `UpdateRaises` `Completed(true)`, outcome `True`; over `up34` exactly one
  entry, `UpdateAllowed` `Completed(false)`, outcome `False`, and no entry for
  `UpdateRaises`.
- Step 2: over `up12` three entries in the order `UpdateAllowed`,
  `UpdateNotMax`, `UpdateRaises`, each `Completed(true)`, outcome `True`; over
  `up34` two entries, `UpdateAllowed` `Completed(false)` then `UpdateNotMax`
  `Completed(true)`, outcome `False`, and no entry for `UpdateRaises`.
- Step 3: two entries, `NoCycle` `Completed(true)` then `UpdateAllowed`
  `Completed(false)`; outcome `False`; no entry for `UpdateRaises`.
- Step 4: one entry, `UpdateAllowed` `Incomplete` with `resource_exhausted`/
  `insufficient-next-charge`, outcome `Incomplete`, and no entry for
  `UpdateRaises`; the empty set gives no entry and outcome `True`; the two
  evaluations of the set are equal, entry for entry.
- Step 5: `CallFailure::Input(InputRefusal::DuplicateClause)` naming
  `UpdateAllowed`; `CallFailure::Input(InputRefusal::UnknownClause)` naming
  `sameIdentity`; `CallFailure::Input(InputRefusal::ObservationsMismatch)`.
  No call returns an entry.
- Step 6: `True` for {} and {true, true}; `False` for {true, false} and
  {false, X}; `Undecided` for {true, X}, {X} and {X, X}; `Incomplete` for
  {true, incomplete}, {false, incomplete} and {X, incomplete}; each X gives
  the same outcome.
- Step 7: {`F`}: one entry, `F` `FamilyResult::Undefined` with reason
  `precondition-false` (operation `ready`), outcome `Undecided`; {`G`, `F`}:
  two entries in the order `F`, `G`, `F` that result and `G`
  `Completed(true)`, outcome `Undecided`; {`H`, `F`}: one entry, `F` with that
  result, no entry for `H`, outcome `Undecided`; with the precondition
  `true`, {`F`}: one entry, `F` `Completed(true)`, outcome `True`.
