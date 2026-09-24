---
id: SR-611
title: "Failure-domain review of the QSL-211 recursion-group order"
type: SpecReview
analysis: failure-domain
scope: "Commit 7be2d02a: FR-092 Recursion groups (the names graph, dependency order, the refinement passes, groups that collide, FR-092-OQ-1), the FR-093 IR-242 dependency, ADR-013 QC-24, and TC-413 steps 5 and 9"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-413
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---

## Summary

The checklist was run over the recursion-group rules.

- **Topological robustness.** The group is a strongly connected component of
  a finite graph, and the components form a DAG, so dependency order exists.
  A refinement pass takes at most `n` rounds. The self-edge case (G1) is
  covered.
- **Entity identity.** A structural in-group node's key is unique per group
  within a package, because every reachable group holds a declared record or
  function whose `declaration` and `owner` enter the group digest. An
  in-group application node's key is not unique: FR-092-OQ-1 records this,
  and the collision refusal guards it.
- **Evaluation purity.** Not affected: the order reads only preimages.
- **Extension points.** None added.

A scratch model of the rules found two failure modes that the prose does not
state. It also found one resource bound that is not stated.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-092 says in-group application collisions come from "groups whose members differ only in declared names or owners". They also come from groups that differ in structure, and whether a given pair collides depends on how content hashes sort. The scratch model keyed `f`'s group against a copy renamed `g` whose `decreases` binding names another node. In 146 of 600 random measure digests, `g`'s conditional keyed to G5, because the hash order happened to give the members the same ordinals. So a package with two recursive functions that differ in a non-application member is refused about one time in four. The rule does not say which, and an unrelated edit to the measure can move a package into or out of refusal. The refusal itself is correct, because the two nodes mean different things. What is missing is the stated scope. Fix: in "Groups that collide" and FR-092-OQ-1, state that the refusal fires whenever two groups' in-group application nodes get equal preimages, whatever differs in their structural members. Add a TC-413 step 5 case with a concrete colliding structural variant, found by search as above. | FR-092:223-227, :229-244, :975-983; TC-413 step 5 |
| FND-002 | medium | With QSL's content order and digest-ordered emission, every QSL package that holds a recursion group is refused by IR-242 as `invalid_package`/`stale-node-key`. For `f`, digest order is G4 (`23cd…`), G5 (`3d8a…`), G6 (`8cd0…`). IR-242 therefore recomputes G5 with ordinal 1, but FR-092 gives it ordinal 0. FR-092's Dependencies say that IR recomputes QSL's keys only after QSpec and IR adopt the order. No requirement says what `check` or emission does before then, and no TC shows the outcome. Fix: take the emission-order fix in SR-610 FND-001, and add a TC-416 step that runs IR-242's reader over an emitted `f` package and expects it to admit. If that fix is rejected, state in FR-093 that emission of a group is refused until QC-24 is decided, so that QSL never writes a package a conforming reader refuses. | FR-092:941-946; FR-093:241-243; TC-416; `quire-contract-ir` 210d47a |
| FND-003 | low | The two refinement passes cost up to `n` rounds of `n` SHA-256 hashes, plus the targets, for a group of `n` members. They run outside the depth limit, and no budget covers them. Every call site and conditional in a recursive body joins the group, so one large recursive function makes a large group. FR-092 bounds the type-node and lowering walks by `CheckingLimits`, but not this. Fix: charge each round's hashes to the check stage's work budget, and state the refusal (`resource_exhausted`/`insufficient-next-charge`) as for the depth limit. Alternatively, state the bound `n` × (members + targets) hashes and the size limit that keeps it small. | FR-092:115-119, :179-187 |

## Resolution

All findings are fixed. FND-001: "Groups that collide" states that the refusal covers any two in-group application nodes of different groups with equal preimages, which always happens for groups that differ only in declared names and can happen for groups that differ in other content; the names bullet claims determinism only for the name-only case. No concrete structural fixture is added: finding one needs a hash search, and the rule under test is the same equal-key refusal AC-7 exercises. FND-002: fixed with integrity FND-001; emission writes each group in ordinal order, so IR-242 recomputes QSL's keys, and TC-416 step 5 checks it. FND-003: FR-092 states the bound, at most `n²` signatures for a group of `n` members, with `n` bounded by the check stage's node limit.
