---
id: SR-11414
title: "Code review — QSL-794 relationship extents admission, manifest and check 11.5 (PR 880)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@c8b1583be44d3bc1ad495842e6af63dd65fc9a65; qsl-semantics/src/model/observation/{relationships,extents,document,frame}.rs, qsl-semantics/src/model/observation.rs, qsl-semantics/src/model/object_environment.rs, Cargo.lock, qsl-semantics/tests/it/{relationship_extents,relationship_snapshots,main}.rs"
review_set: subset
---

## Summary

Ticket: QSL-794 (slice b). Code review with the rust-review lane over this
PR's own commit only (`git diff HEAD~1 HEAD`; HEAD~1 is #878's head before its
format-only commit `038f251ea`, which touches one qsl-replay test file). The
implementation is checked against merged FR-364 (conditions 1 to 4, the
condition-3 full-reference amendment, the read contract, the manifest, the
bundle key and constructor order), FR-106 checks 6.6 to 6.9, 7, 8 and 11.5,
and FR-115's `Link` witness. No build was run: read-only review.

The QSV dependency is the `Cargo.lock` entry only (98091ee); no QSV code is
copied into this repo.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Check 11.5 faults every state-machine-context postcondition and Frame run. `first_link_change` always calls `side_extents`, which builds an `ObjectReference` for every object of every population through `object_reference` -> `population_universe` -> `object_universe_of`. A state-machine context object (its population is the context type, a `StateMachine` record) has no object universe: normalize.rs builds universes only over `index.object_types()`, and index.rs never sets `object_type` for a `StateMachine`. That is why `admit_state_machine_context_values` mints the context reference from the observation digest instead. So `side_extents` returns `fault("object-universe-unresolved")` before any relationship is read, even when neither snapshot lists `relationships`. Before this PR, check 11 passed these invocations (steps 1 to 4 call `object_reference` only for created or deleted keys). A user's state-machine postcondition now gets an internal fault where it admitted. The existing qsl-eval test `tc235_m15_lifecycle_invariant_keeps_receiver_context` ("run 1 postcondition admits", `OrderLifecycle` of kind `lifecycle`) takes this path and should now fail. Fix: resolve only the objects that instance ends name, or skip step 5 when neither snapshot lists an entry, and add a state-machine postcondition with no `relationships` to the focused tests. | qsl-semantics/src/model/observation/relationships.rs:756-777, qsl-semantics/src/model/observation/relationships.rs:823-825, qsl-semantics/src/model/observation.rs:2656-2668, qsl-semantics/src/model/normalize.rs:3293-3327 |

## Verdict

**FAIL: 1 blocking finding (FND-001).**

Checked and correct (no finding):
- Read and checks 6.6 to 6.9: missing members before unknown members, then
  kinds, in `relationship`, `complete`, `instances` order. The walk is entry
  by entry and instance by instance after the last object. 6.8 checks the
  instance and end shape, then `source` before `target` populations, then the
  type of listed ends only. The 6.9 repeat key is (population, key) on both
  ends and is skipped only when both ends declare `unique: false`. An end with
  no multiplicity counts as unique.
- Check 6.9 is tested as its own check. With 6.9 off, the store would refuse
  the same triple, but only in `finish_populations`, after checks 7 to 10.
  TC-1035 row 32 (a repeat at instance 1, a wrong end type at instance 2)
  would then refuse `wrong-value-kind` at instance 2, so that test kills the
  mutant. `check_6_9` alone does not: the store backstop gives the same
  code, cause and instance.
- Check 7: required populations extend through instance ends to a fixed
  point. End-only populations come after the walk order. The unlisted-end
  rule runs after the required-population rule. Check 8 runs after the
  `reference` values, `source` before `target`. The call order is pre 7,
  post 7, pre 8, post 8.
- Condition 3 at the model layer compares the full reference's type (QSV),
  whether the object is held or not. Admission passes no multiplicity bound
  to the store, so a count check at admission is structurally impossible.
- Check 11.5 uses a `BTreeMap<DeclarationKey, _>` (ascending key order). It
  counts instances keyed by (source, target) `ObjectReference`, whose
  derived `Ord` is (universe, type, object), so the order is the reference
  key's. Only relationships listed complete in both snapshots are compared,
  `modifies` grants, ends at the invocation's `deleted` are left out, and
  created ends count as added. Step 5 runs once, after every population's
  steps 1 to 4. The `Link` witness takes the source end's population
  (FR-115), and `modifies` holds the checked `modifies` plus
  `receiver_modifies`.
- `ResolveBundle::new` checks manifest, closure and types, in that order,
  against the whole key. `RowLimit` is unreachable because the row limit is
  the declaration count and the enumerator yields exactly those rows. A
  partial manifest arises only from `PopulationEntry::try_new` failing (an
  empty covered-type set). The coder's dead-code claim holds.
- The two `FrameStop` matches are updated for `Link`. No new `unwrap`,
  `expect`, `panic!` or unchecked indexing in `src/`.
- PR body: honest about what is in and out (charges, the
  `modifies [relationship]` intake grant, TC-1037 rows 13 to 15, the `pipe`
  Connection sub-cases).

Recorded (medium/low, not blocking):
- Medium: `AdmittedObservations::bundle` keys `documents` by the
  current/pre/post digests and leaves out the invocation document's digest.
  FR-106 lists the invocation among a postcondition's selected documents.
  It cannot change a `resolve`: two invocations that share pre/post snapshots
  build identical manifests and closures. It is still a departure from
  FR-364's key text.
- Low: `BundleRefusal::record()` names only `part`. FR-364 says the refusal
  retains the selected key and the differing key. The struct keeps both, but
  the admission record drops them.
- Low: `listed.contains_key` / `insert` / `get_mut` in
  `admit_population_values` should use `entry(...).or_default()`.
- Low: `first_link_change` recomputes `model_facts` and re-derives every
  reference on each frame run. That cost is O(objects) per run, with no
  wrong result.
