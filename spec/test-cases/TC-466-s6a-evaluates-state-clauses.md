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
reads under `pre`, and `reaches` semantics and its exact charge log.

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
3. Use a variant of TC-458's fixture package that adds operation
   `probe(target: ConfigVersion)` on `ConfigVersion`, with no result and an
   empty frame, and the clause
   `pre ReachesTarget using v on Config::ConfigVersion::probe { reaches(self, target, parent) }`.
   Snapshot `chain` holds `a`, `b`, `c` with `a.parent = b`, `b.parent = c`,
   `c.parent` absent; snapshot `loop` holds `a` with `a.parent = a`. Each
   invocation uses the same snapshot as pre and post. Evaluate with a
   charge-logging meter:
   a. `self` `a`, `target` `c`, over `chain`;
   b. `self` `a`, `target` `a`, over `chain`;
   c. `self` `c`, `target` `a`, over `chain`;
   d. `self` `a`, `target` `a`, over `loop`;
   e. case (a) five times more, each with a meter that denies the 1st, 2nd,
      3rd, 4th or 5th charge of the `reaches` node.

Tag the tests `#[trace("TC-466", "FR-107-AC-n")]`.

## Expected Results

- Step 1: `Completed(true)`, `Completed(true)`, `Completed(false)`;
  `Completed(true)`, `Completed(false)`, `Completed(false)`;
  `Completed(true)`, `Completed(false)`.
- Step 2: all three `Completed(true)`.
- Step 3: (a) `Completed(true)`, and the `reaches` node's charge log is
  exactly `graph.expand`, `graph.edge`, `graph.expand`, `graph.edge`,
  `graph.result-retain` (2 expansions); (b) `Completed(false)`; (c)
  `Completed(false)`; (d) `Completed(true)`; (e) `Incomplete` with
  `resource_exhausted` each time.

## Status

Planned (QSL-273). Step 1's `ParentOrder` and `NoCycle` sub-cases and step 3's
`reaches` charge-log sub-case (a in the acyclic-chain form) are built and
passing (`qsl-replay/src/spine/clause/tests.rs`). Steps 2 and 3 (the
`VersionUnchanged` sub-case of step 1 included) are **Pending QSL-279**:
`evaluate_clause` is sealed to `qsl_package::checked::CheckedPackage`
(`qsl-eval/src/value/expression/mod.rs:314`), reachable only by compiling a
unit all the way through S4 emission, and any clause that references a
domain-package operation (`VersionUnchanged`'s `attemptUpdate`, or step 3's
own `ReachesTarget`/`probe` when evaluated rather than merely admitted)
refuses emission today: FR-105's own text states "Today QSL emits no `state`
node" (`spec/functional/FR-105-emit-state-nodes.md`), and a hand-authored
domain-package operation's `origin.source` is never a real byte source S1/S2
parsed, so `qsl_package::emit`'s occurrence-placement rule refuses
`EmitRefusal::UnlocatedOccurrence` (`qsl-package/src/emit.rs:35-42,142`) the
moment any clause referencing the operation is compiled through to S4.
QSL-279 ("A05-8: S4 state node emission and the ConfigVersion spine corpus,
FR-105, FR-108") is the ticket that implements the missing emission this
depends on.
