---
id: TC-466
title: "S6a evaluates state clauses over their observations, pre reads and reaches"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: verifies
---
# TC-466: S6a evaluates state clauses over their observations, pre reads and reaches

## Description

Verify the truth values of the ConfigVersion clauses, observation-qualified
reads under `pre`, and `reaches` semantics and charging.

Scope: FR-107-AC-1, FR-107-AC-2, FR-107-AC-3.

## Test Procedure

Admit FR-108's case documents (FR-106) and call `evaluate_clause` with the
default meter.

1. `ParentOrder` over healthy-parent, absent-parent and violating-parent;
   `NoCycle` over healthy-parent, cycle and self-loop; `VersionUnchanged` over
   unchanged-version and changed-version.
2. Over changed-version, evaluate the postconditions
   `pre(self.versionNumber) = 2`, `self.versionNumber = 3` and
   `pre(present(self.parent) implies deref(value(self.parent)).versionNumber = 1)`.
3. Snapshot `chain`: objects `a`, `b`, `c` with `a.parent = b`,
   `b.parent = c`, `c.parent` absent, and snapshot `loop`: `a.parent = a`.
   Evaluate `reaches` through test invariants
   `invariant R using v on Config::ConfigVersion at current { reaches(self, self, parent) }`
   and the function `function r using v(x: Config::ConfigVersion, y: Config::ConfigVersion): Boolean pure { reaches(x, y, parent) }`, called with each snapshot's object environment, for the pairs
   `(a, c)`, `(c, a)`, `(a, a)` over `chain` and `(a, a)` over `loop`. Then
   evaluate `(a, c)` over `chain` with meter budgets `0` up to the number of
   expansions it charges.

Tag the tests `#[trace("TC-466", "FR-107-AC-n")]`.

## Expected Results

- Step 1: `Completed(true)`, `Completed(true)`, `Completed(false)`;
  `Completed(true)`, `Completed(false)`, `Completed(false)`;
  `Completed(true)`, `Completed(false)`.
- Step 2: all three `Completed(true)`.
- Step 3: `true`, `false`, `false` over `chain`; `true` over `loop`. Every
  budget below the expansion count gives `Incomplete` with
  `resource_exhausted`; that count gives `Completed(true)`.

## Status

Planned (QSL-273).
