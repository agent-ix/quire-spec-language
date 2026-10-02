---
id: TC-592
title: "EN-1 explores a state-graph subject once, labels it and tracks open nodes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-167
    type: verifies
---
# TC-592: EN-1 explores a state-graph subject once, labels it and tracks open nodes

## Description

Verify the explored graph of ADR-022 §7's subjects, the open-node causes
under each limit and an undecided expansion, the deadlock-freedom item on
the same exploration, the reduction pre-check, the manifests, and
determinism.

Scope: FR-167-AC-1 to FR-167-AC-5.

## Test Procedure

Fixtures: ADR-022 §7.1, §7.2 and §7.3's units; a subject with an undecided
contract conjunction at one reachable node; a subject with no initial
state; a subject with an unbounded population root.

1. Explore each §7 subject; request `ReachesTwo`, `ReachesThree` and the
   deadlock-freedom item over §7.1 together.
2. §7.1 with `max_depth` 1, and with `max_states` 3; the undecided subject.
3. The deadlock-freedom item over §7.2 with no `terminal` member; the
   subject with no initial state; the unbounded subject.
4. `possible`, `always possible` and `unique path` items over one subject
   with partial-order reduction selected; a `unique path` item with
   symmetry selected. Read the EN-1 and trace-monitor manifests.
5. Step 1's §7.2 request twice.

Tag the tests `#[trace("TC-592", "FR-167-AC-n")]`.

## Expected Results

- Step 1: 9 nodes and 18 edges, all closed; 4 nodes and 3 edges in order
  `Start`, `Mid`, `Lost`, `Won`; 5 nodes and 5 edges; one exploration
  counting 9 nodes.
- Step 2: `(1, 0)` and `(0, 1)` open `MaxDepth`, `(0, 0)` closed, end
  `Completed`; end `Stopped(ResourceExhausted, MaxStates)` with discovered
  unexpanded nodes open `NotExpanded`; the undecided node open
  `UndecidedSuccessor` and every other reachable node expanded.
- Step 3: violation at `Lost` with path `play, lose`; `NoInitialState`;
  `RequiresBound` with no node explored.
- Step 4: the `always possible` and `unique path` items settle
  `ReductionNotPreserving` before expansion and the `possible` item is
  explored; `ReductionNotPreserving`; the EN-1 manifest lists the three
  forms and the monitor's lists none.
- Step 5: byte-equal graphs, labels and open sets.
