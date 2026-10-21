---
id: SR-12251
title: "Code review (Rust lane) of QSL-559 E15 PR 3b ProtocolSystem<Sc>: replicated roles and activation on each (PR #1139)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@d9bb4ef97d03028400d44824d439c531ae556f5e; qsl-eval/src/simulation/protocol{.rs,/roles.rs,/identity.rs,/footprint.rs,/program.rs,/steps.rs,/compensation.rs}, qsl-eval/src/simulation/model/footprints.rs, qsl-semantics/src/check/{footprint.rs,mod.rs,protocol_clause/mod.rs,protocol_clause/types.rs,protocol_slots.rs,protocol_slots/tests.rs}, qsl-replay/src/model_check/counterexample.rs, qsl-analyze/src/model_check/check.rs (context), qsl-analyze/tests/it/{protocol_roles.rs,protocol_footprints.rs,weak_memory.rs,fill.rs,main.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-208
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-209
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-211
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: reviews
---
# SR-12251: Code review (Rust lane) of QSL-559 E15 PR 3b (PR #1139)

## Summary

Ticket: QSL-559. PR quire-spec-language#1139, head d9bb4ef97, scoped to `git diff origin/main...HEAD`. The `rust-review` lane is folded into this file.

The review checked the new steps and their semantics against ADR-027: ST-13 to ST-15, AE-1 to AE-4, RR-1 to RR-4, PS-2, PS-5, TS-2, FT-1 to FT-5, PD-2 and PD-4. It also checked the footprints of `spawn`, `retire`, `activate` and steps `by` a role, the visibility of a role anchor, the instance bound, refusal precedence and the panic surface.

The protocol-system semantics are sound. The defect is at the model-check boundary. This PR stops refusing `activation on each`, so `check_protocol` and the closure certificate now run over such protocols. Neither knows about the instance bound, so a run that hit it returns `Holds` with a certificate (FND-001).

No test was run. The one focused run (`protocol_roles`, `every_independent_pair` and a probe of `check_protocol` over an `on each` unit) was refused by the session's permission layer. FND-001 was traced statically, from `check.rs:1187-1212` through `ProtocolRun::decide`.

## Verdict

**FAIL**: FND-001 is a wrong result. It blocks the merge.

What holds:

- **PS-2 and AE-2 ordinals.** `activate` takes the smallest ordinal no live instance holds, and `finish` frees it.
- **Instance bound.** `activate` is not expanded at the bound. `instance_limited` reports the state, and `terminal_class` returns `NotTerminal` for it before the PD-2 check. A state with live instances whose only steps are `activate` falls through to the `terminal` member, as PD-2 states.
- **Footprints.** `spawn` and `retire` read and write `role(r)` keyed by protocol instance. A step `by` r reads it. `activate` reads and writes `pinst` and writes the new root's `ctl`. Under `on each`, `finish` writes `pinst` and every `role(r)` of its instance. `retire` and the activation guard read their predicates' model locations as `AnyField` plus membership. Settling the new root reads only binders of its own new `ctl`.
- **POR pairs.** Every pair checked is dependent where its two orders differ: `activate`/`finish` (ordinal), `spawn`/step-by-r (actor set), `retire`/attempt (until-read field). `scope_writes` is conservative.
- **Identity.** The coder added an `instance` member to `spawn` and `retire`. It is sound: `role(r)` is per protocol instance, so the footprint function needs the ordinal from the step. Without it, `spawn(r, o)` in instances 0 and 1 share one identity. TS-2 would tell them apart only by the post-state digest. The `actor` member is in the encoding and in `PartialEq`.
- **RR-2.** The successor is shared across actors. No node body can read the role binder: `role_binders` are typed only into `until`, so the post-states really are equal.
- **Refusals and panics.** Refusal precedence is unchanged apart from the narrowed `ActivationOnEachTemplate`. No new `unwrap`, indexing or unchecked arithmetic was found. Internal faults are `ProtocolRefusal::fault`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `activation on each` is no longer refused, but nothing above `ProtocolSystem` counts instance-limited states. `ProtocolRun::decide` (`check.rs:1187-1212`) knows only the memory bound and `horizon_cut`. An `on each` run that reaches `max_live_instances` with no counterexample therefore returns `ModelCheckOutcome::Holds` with a closure certificate, which settles V-1. AE-4 and FR-209 require V-6 `Undecided(InstanceBoundReached{bound, states})`. Example: deadlock-freedom over `roles::unit(EACH, "", "", touch("T"))` explores up to three instances, never expands the fourth `activate`, and reports Holds as if every instance count were covered. This is the FR-209 use case's "a pass mistaken for a proof over every instance count". `qsl-replay` closure checking builds the same system with the same gap. Fix in this PR: either thread `max_live_instances` and the instance-limited count through `ModelCheckRequest`/`ProtocolCheckRequest` and return `InstanceBoundReached` with FR-209's precedence (that is FR-209-AC-1/2/4, see SR-12252 FND-001), or refuse `on each` in `check_protocol` and in closure checking until that lands. | qsl-analyze/src/model_check/check.rs:537-546, qsl-analyze/src/model_check/check.rs:1187-1212, qsl-eval/src/simulation/protocol/program.rs:355-363, qsl-replay/src/certificate/closure.rs:272 |
| FND-002 | medium | A `scope` lifetime ignores compensation threads. `reaches_any` returns `false` for `Rest::Compensate`/`Rest::Retry`, and `Replicated::owns` comes from `CheckedProtocol::owners`, which holds only run-tree declarations. A template's compensation attempt is owned by `template.role` but is never in `owns`. Under `on origin`, a template whose role is a `scope` replicated role therefore has its instances retired once the run tree passes its last `by r` node. A compensation thread activated later rests at a `cattempt` the role owns, and gets `NoRoleInstance{r}`: a spurious deadlock. RR-4's "no live thread can reach any node by r" includes that thread. Fix: count a running compensation thread of a template owned by r as reaching r, and add a fixture. If S3's run-tree-only region is deliberate, record it as a spec gap instead. | qsl-eval/src/simulation/protocol/program.rs:1051-1056, qsl-eval/src/simulation/protocol/roles.rs:279-297, qsl-eval/src/simulation/protocol/program.rs:420-425 |
| FND-003 | low | Under `on each`, `terminal_class` calls `instance_limited` (one guard evaluation per trigger record) and then `only_activations`. That second call re-runs `expand_bounded` over the whole state whenever there is a successor and a live instance, so nearly every state is expanded twice. The successors the explorer already has could answer the PD-2 question. The same branch also returns a terminal class other than `NotTerminal` for a state that has successors (only `activate`s). That is correct for PD-2's deadlock, but it is undocumented for any other reader of `TerminalClass`. | qsl-eval/src/simulation/protocol.rs:1287-1316 |
| FND-004 | low | The AE-2/PS-2 rule "the smallest ordinal no live instance holds" is implemented twice, as `(0u64..).find(...)` in `activate_steps` and again in `activate_footprint`. If one changes, the footprint and the step disagree on which `ctl` is written. Use one helper. The live-instance count of a role (`values().filter(Live)`) is also repeated in `acting`, `spawn_possible`, `no_role_instance` and `scope_writes`. | qsl-eval/src/simulation/protocol/roles.rs:312-314, qsl-eval/src/simulation/protocol/footprint.rs:933-935 |
| FND-005 | low | `from pop` is never resolved (spec gap 2 in comment 0b94faf0). `role_universes` takes every object of the role's object type, and an unknown or misspelled population name is accepted silently: the fixtures write `from cells` and `from cores`, and neither name is checked. While the state model has one population per object type, the result is the same. But S3 should resolve the name, or refuse one that names no population of `T`, rather than ignore it. | qsl-eval/src/simulation/protocol.rs:807-829, qsl-semantics/src/check/protocol_slots.rs:1133-1138 |

## Readings judged

- **Spec gap 1** (`instance` member on `spawn`/`retire`): real. It is sound and needed by this implementation's per-instance `role(r)`. TS-2 and QSpec FR-426 need the amendment.
- **Spec gap 3** (send/receive ownership): real. FR-218 states no owner output. Reading the owner from the channel's `from`/`to` is the only reading consistent with PD-4.
- **Spec gap 4** (locations of the `retire` and guard reads): partly stated already. FT-1 gives `retire` "the model and binder locations p reads", and FT-3 lists the guard. The derivation rule (field reads on any object) is unstated and is a sound over-approximation.
- **The `ActivationOnEachTemplate` refusal** names a real missing slot in the template frames. It is an honest `not-yet-implemented` (SR-12252 FND-004).
