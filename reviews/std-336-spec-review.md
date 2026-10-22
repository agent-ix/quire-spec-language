---
id: SR-16188
title: "spec-review of the public HP-2 HyperProductIdentity handoff (quire-spec-language PR 1282)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@d9ec0be1; spec/functional/FR-315-encode-the-smt-lib-transition-relation.md; spec/functional/FR-182-settle-a-hyper-verdict.md; spec/test-cases/TC-1190-the-hp-2-product-identity-binds-each-smt-outcome-to-its-item.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1190
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-183
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-336
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: references
---

# Spec review of the public HP-2 HyperProductIdentity handoff (quire-spec-language PR 1282)

## Summary

Ticket: STD-336 (unblocks QSL-1010 `encode_hyper_query` and its CG consumer
IR-974). One spec-review pass over `git diff origin/main...d9ec0be1`.
Oracles: FR-172 "Identity", ADR-023 HC-1, HC-7 and HV-7, ADR-013 O-07 and
O-09, FR-183's `check_match_witness`, FR-336, FR-059 and FR-060 T12-A,
ADR-011's crate DAG and the code on main (`qsl-analyze`
`model_check::request::SubjectIdentity`, `qsl-replay`
`model_check::hyper_settle`, the crate manifests).

Gates: `make check-spec-validation` (QSPEC_DIR=~/dev/quire-specification)
exits 0 with no finding on the three touched files. origin/main is an
ancestor of the head.

Examined, clean:

- No new digest domain. The identity is the clause's `claim` occurrence key
  (O-07), FR-336's subject projection and the instance, compared member-wise.
  It carries no digest of its own, which follows the rule that only canonical
  identity digests stay.
- ADR-013 O-09. CG cannot mint the identity: there is no constructor from
  members and no deserializer, and FR-315-AC-14 has compile-fail cases for
  both. CG still computes the O-09 obligation digest itself. That is
  unchanged and does not conflict.
- FR-172 "Identity" and ADR-023 HV-7. The fairness-set case (FR-172-AC-4)
  gives byte-equal scripts and a different `clause()`, as FR-315-AC-12 says.
  Kind, depth, meter and limits are left out of the identity, which is
  consistent with FR-336's rule on run limits.
- The crate DAG (ADR-011). `qsl-analyze` already depends on `qsl-replay`, and
  `qsl-replay` already depends on `qsl-eval` (`ModelSubject`), `qsl-package`
  (`emit_checked`), `qsl-foundation` (`ProofBound`, `OccurrenceKey`, already
  re-exported) and `qsl-semantics` (`PackageId`, `DeclarationKey`). Defining
  `SubjectIdentity` in `qsl-replay`, with `qsl-analyze` importing it from
  there, adds no edge and closes no cycle. A re-export the other way would
  be impossible, so the type has to move (see FND-004).
- FR-182 V-3, V-6 and the `VacuousMatch` rule. The existing rule that the
  witness instance must equal the outcome instance is kept. The new rule
  that the identity must be equal only narrows which `Accepted` checks
  count. A mismatched outcome refuses before any row is chosen, so it never
  settles V-3 or V-6, and FR-182-AC-11 tests this for both `Inductive` and
  `InductionNotClosed`.
- Most ACs can fail. FR-315-AC-12 checks that each of the instance, the
  fairness set and the proof bound changes only its own member, and that the
  meter changes nothing. FR-315-AC-13 names each new refusal and the
  zero-charge rule. FR-315-AC-14 checks the dependency set and the two
  compile-fail cases.
- Ids. TC-1190, FR-315-AC-12 to AC-14 and FR-182-AC-11 do not appear on main
  or in any added line of the other 23 open PRs. No Rust test is tagged with
  them yet.
- No status, changelog or "previously" wording. No compat layer.
- Lead defaults 1 to 6 are labelled as lead defaults in the PR body. Default
  5 (the identity is an in-process handoff with no wire form) is still an
  open owner question. The spec says nothing about whether the type has a
  `Serialize`. If the driver or CG must persist an outcome with its
  identity, the type needs a wire form, and QSpec would have to own it.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | [major] Nothing produces the match-witness check's identity. FR-182 needs the `MatchWitnessCheck` to "come with" the `HyperProductIdentity` of the clause, subjects and witness instance that the check replayed. An `Accepted` check counts toward V-3 only when that identity equals the outcome's. But FR-183's `check_match_witness(wire, subjects, witness, limits) -> MatchWitnessCheck` is unchanged and returns a bare `Accepted`/`Refused`. Only `of` and `encode_hyper_query` can make an identity. So CG would pair the check with an identity of its own choosing, most likely the query's own `product`, and the comparison would always pass. QSL would never bind the identity to what the replay actually replayed. CG would also have to guess where the identity comes from. Fix: make `check_match_witness` return the identity of what it replayed (for example `Accepted { product }`), amending FR-183, or state exactly which value goes with the check and why QSL can trust it. Then rewrite FR-182-AC-11's last sentence to match. | spec/functional/FR-182-settle-a-hyper-verdict.md:64-69, :155-159, :177; spec/functional/FR-183-replay-a-hyper-counterexample.md:106-116 | correct-requirement-no-evidence |
| FND-002 | medium | [major] The outcome handoff has no type and no entry point. FR-315 says the HP-2 SMT outcome CG returns "SHALL carry" the `product`, and FR-182 takes "the outcome's `HyperProductIdentity`" as an input. Neither names the Rust shape that holds the pair, or the settlement function that takes it. FR-182 still maps SMT outcomes "into the same values" (`HyperOutcome` has no `Inductive` or `InductionNotClosed` variant, and `settle_hyper_check` takes no identity). IR-974 cannot write the CG side of the call without guessing. Fix: name the type CG returns (outcome, depth, `product`, and the optional check with its identity) and the `qsl-replay` settle entry, with its `HyperSettleRefusal` result, in FR-182 Outputs or FR-315. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:375-380; spec/functional/FR-182-settle-a-hyper-verdict.md:58-69 | missing-requirement |
| FND-003 | medium | [major] FR-315 cites FR-060 T12-A as allowing CG's SMT backend to name these types, but T12-A's only allowed caller is "CG's `replay` adapter module". A CG SMT backend module outside that module that names `qsl_replay::HyperProductIdentity` or calls `encode_hyper_query` fails T12-A. FR-059 (#1278) sets the crate-level dependency, not the module allow-list. Fix: either say that the SMT backend's QSL calls go through CG's replay adapter module, or add the SMT backend module to T12-A's allowed callers (an FR-060 change). | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:313-319; spec/functional/FR-060-check-qsl-api-surface-boundary.md:86 | wrong-requirement |
| FND-004 | medium | [major] The move of `SubjectIdentity` is not specified. FR-315 says "FR-336's `SubjectIdentity`, which the facade also holds". But FR-336 names no `SubjectIdentity` type: its identity projection is prose, and the type exists only in `qsl-analyze` code. The PR body's default 2 says the type moves to `qsl-replay` and `qsl-analyze` imports it from there, but no spec text says so. "Holds" could also be read as a re-export, which the DAG forbids. The moved `of` also returns `qsl-analyze`'s `RequestRefusal::Package`, and the spec does not say what it returns from `qsl-replay`. Fix: in FR-336 (or FR-315), name `qsl_replay::SubjectIdentity` as the identity projection's type, say that it is defined in `qsl-replay` and used by `qsl-analyze` from there, and name its refusal (or map it to `product-identity`). | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:316-318, :339-342; spec/functional/FR-336-bound-a-model-subject-by-its-universes.md:86-94 | missing-requirement |
| FND-005 | medium | [minor] FR-182-AC-11 never tests `ProductMember::Subjects` or the order in which a mismatch names its member. Behaviour says settlement names "the first member that differs in the order `clause`, `subjects`, `instance`". The AC's mismatches differ in exactly one member each (`instance`, then `clause`). A settlement that never returns `Subjects`, or that checks `instance` first, passes every AC. Fix: add a vector where the outcome's identity is for a subject that differs only in a proof bound (FR-315-AC-12's case), which names `subjects`, and one where it differs in both clause and instance, which names `clause`. Add matching steps to TC-1190 step 5. | spec/functional/FR-182-settle-a-hyper-verdict.md:137-143, :177 | correct-requirement-no-evidence |
| FND-006 | low | [minor] The refusal order of `of` and `encode_hyper_query` is unspecified. `of` refuses only `subject-binding`, `instance-binding` and `product-identity`, and must agree with `encode_hyper_query` "when that refusal is one of these three". Take a request with both a `hyper-body` defect and an `instance-binding` defect: the spec does not say which one `encode_hyper_query` reports. It also does not say whether `of` returns an identity for an HP-3 clause, which `encode_hyper_query` refuses `hyper-form`. Two implementations can differ. Fix: state a precedence (for example, the identity refusals come first, then `hyper-form`, `hyper-body`, `query-kind`, `match-conjunct`) and what `of` does for a non-HP-2 clause. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:362-374 | wrong-requirement |
| FND-007 | low | [nit] `HyperProductIdentity` is declared `Hash`, but it holds `[SubjectIdentity]`, and `SubjectIdentity` on main is not `Hash`: its `proof_bounds` is a `HashSet<ProofBound>`. Either `SubjectIdentity` gains an order-independent `Hash` when it moves, or the `Hash` derive goes. The spec states neither. Fix: say `SubjectIdentity` is `Eq + Hash`, or drop `Hash` from the declaration. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:322 | missing-requirement |
| FND-008 | low | [minor] The `product-identity` refusal for "two subjects carry different checked packages" depends on lead default 6, that a hyper clause's subjects share the declaring unit's checked package. No requirement states that. ADR-023 HS-2 lets different aliases compare two models, and FR-336 gives each subject its own checked package. If an alias's model can come from a dependency package, this refusal rejects a valid HP-2 comparison. Fix: cite or add the rule that every subject's checked package is the clause's declaring package, or key `clause()` to the declaring package and drop the refusal. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:368-371 | wrong-requirement |
| FND-009 | low | [nit] The `clause` member's text says its node id "binds every member that FR-172 'Identity' and ADR-023 HV-7 name". It then lists them without the instance, which both FR-172 and HV-7 name and which a clause node id cannot bind. The `instance` member covers it, but the sentence as written is wrong. Fix: "every clause-level member", or name the instance as bound by the third member. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:335-338 | wrong-requirement |

## Verdict

Not mergeable yet. The structured, no-digest, QSL-minted identity is sound:
it respects O-09 and the crate DAG, and it agrees with FR-172 and HV-7.
FND-001 is a soundness gap in the V-3 witness rule. FND-002 to FND-004
leave IR-974 (CG) and the `SubjectIdentity` move to guesswork. FND-005 leaves
a Behaviour branch with no test. All five need fixing in this PR. FND-006 to
FND-009 are small and can be fixed in the same round. Lead default 5 (no
wire form) is still an owner question.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | low | [minor] An HP-2 SMT result other than the two induction results has no settlement path. FR-182's Description says every HP-2 item settles as exactly one FR-331 terminal record. `settle_hyper_smt` takes only `Inductive` and `InductionNotClosed`, and FR-182 says "a satisfiable base, an unknown solver answer, an encoding refusal and a resource stop are no `HyperSmtResult`". No text says how those settle. The Inputs line "mapped by CG into the same values" would route them through `settle_hyper_check`, which has no identity check, or they would settle as nothing. Lead default 8 scopes this to IR-705, which has not set the SMT resource ceiling, but the spec does not say so. Fix, or defer with a ticket: state which record such a result settles (for example V-6 or V-7 through `settle_hyper_smt`, with the same identity check), or name the follow-up that owns it. | spec/functional/FR-182-settle-a-hyper-verdict.md:35-38, :111-116 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The caller now passes `Option<&MatchWitness>`, never a `MatchWitnessCheck`. `settle_hyper_smt` checks the witness itself by FR-183's rules, over the item's own clause, the `ModelSystem` of each of the item's subjects and `outcome.product.instance()`, bounded by `replay_limits`. It does this only after `of(item)` equals `outcome.product`, so the check is bound to the identity. FR-183 gains the matching sentence. FR-182-AC-11 drops the "check comes with identity" vector for matched and unmatched witnesses. V-3 and V-6 `VacuousMatch` stay sound: the witness-instance rule is kept, and an empty `μ_U` still needs no witness. |
| FND-002 | fixed | FR-182 Outputs now name `HyperSmtResult {Inductive, InductionNotClosed}`, `HyperSmtOutcome {result, product}` and `settle_hyper_smt(item, outcome, witness, replay_limits, limits, requested) -> Result<HyperRecord, HyperSettleRefusal>` as `qsl-replay` facade items, with `ItemIdentity(SmtEncodingRefusal)` added to the refusal. FR-315 "Outcome handoff" says CG returns a `HyperSmtOutcome`. FR-315-AC-14 names the new items. |
| FND-003 | fixed | FR-315 now says CG names these types and calls `encode_hyper_query`, `of` and `settle_hyper_smt` from its `replay` adapter module, the one module T12-A allows, and that the SMT backend reaches them through it. FR-315-AC-14's test crate calls them from its `replay` module. No FR-060 change is needed. |
| FND-004 | fixed | FR-336 Outputs declare `qsl_replay::SubjectIdentity` with `of -> Result<_, SubjectIdentityRefusal>` and `SubjectIdentityRefusal::Package(EmitRefusal)`. Behaviour says the type is defined in `qsl-replay`, and that `qsl-analyze` uses it with no definition or re-export of its own. It also maps the refusal to the EN-1 request refusal and to FR-315's `product-identity`. FR-336-AC-6 and TC-1190 step 5 test it. |
| FND-005 | fixed | FR-182-AC-11 now has a proof-bound-subject outcome that names `subjects`, and a fair-clause outcome at instance `w` that differs in clause and instance and names `clause`. TC-1190 step 6 covers both. |
| FND-006 | fixed | FR-315 "Refusals and their order" sets the order `hyper-form`, `subject-binding`, `instance-binding`, `product-identity` for `of`. `encode_hyper_query` applies those four first and then the rest, so the two calls agree whenever `of` refuses. `of` refuses `hyper-form` for a non-HP-2 clause. FR-315-AC-13 adds the HP-3 case and the liveness clause with an empty instance, which names `instance-binding` and not `hyper-body`. |
| FND-007 | fixed | FR-336 now declares `SubjectIdentity` as `Clone, Debug, Eq, Hash`, with a hash that agrees with its equality and does not depend on order. FR-315 says `HyperProductIdentity`'s `Hash` agrees with its equality. FR-336-AC-6 checks that the reversed listings give an equal identity with an equal hash. |
| FND-008 | fixed | FR-315 "One checked package" now states the rule: every subject is a subject of the checked package of the unit that declares the clause, and HS-2 compares models through two aliases of that unit (ADR-018 SM-2, FR-125). A subject over another package refuses `subject-binding` at its alias, and FR-315-AC-13 tests it. This is still lead default 6, labelled in the PR body. |
| FND-009 | fixed | The text now says "every clause-level member", and the `instance` member is named as "the one member FR-172 and HV-7 name that a clause node id cannot bind". |
| FND-010 | fixed | Round 2: every HP-2 SMT outcome, from EN-2 or EN-3, now goes through `settle_hyper_smt` after the identity check, and none through `settle_hyper_check`. `HyperSmtResult` gains `Violated`, `BoundReached`, `Unknown`, `Cancelled` and `EncodingRefused`, each with a stated row. `query-kind` refuses `QueryKind`, and the other refusals refuse earlier as `ItemIdentity`. A satisfiable base goes on to EN-2. FR-182-AC-12 and TC-1190 step 7 test each row and the instance-`w` mismatch. The new rows raise FND-011 to FND-013. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | high | [major] An SMT `Violated` counterexample is not bound to the item it refutes. `settle_hyper_smt` checks `outcome.product` against `of(item)`, but it settles `Violated{counterexample}` through a `replay` closure the caller supplies, and no rule says `counterexample.instance` must equal `outcome.product.instance()` or that the replay must run over the item's own clause and subjects. In `settle_hyper_check` the counterexample comes from QSL's own EN-1 run. Here CG builds it from an EN-2 model. A counterexample for instance `w`, carried by an outcome with `v`'s identity and replayed by a closure that reproduces it, settles the item at `v` `refuted`: a wrong verdict. This is FND-001's gap moved to the refutation path. Fix: have `settle_hyper_smt` replay the counterexample itself, as it now checks the witness, or require `counterexample.instance == outcome.product.instance()` with a replay request built from the item, and refuse otherwise. Add a vector to FR-182-AC-12 (counterexample for `w` inside the `v` outcome: no `refuted`) and to TC-1190 step 7. | spec/functional/FR-182-settle-a-hyper-verdict.md:99, :118, :199-202 |
| FND-012 | medium | [major] `Unknown{depth}` settles with the cause `InductionNotClosed{depth}`, which ADR-018 V-6 defines as "base case holds to `k`, the step case has a counterexample-to-induction". An `unknown` on the base query, an `unknown` in EN-2's search, or a solver resource stop establishes neither, so the record states a cause that is false. ADR-018 V-7 also calls a limit stop "the one stop cause of every engine", and a solver resource stop lands in V-6 instead. The verdict stays sound (inconclusive, never `proved`). The cause misleads a reader, who would raise `k` when the solver is what stopped. Lead default 8 names the alternative (`SolverUnknown`, which needs a QSpec change). This is an owner decision: take the new cause, or accept the reuse and amend ADR-018 V-6's definition of `InductionNotClosed` to cover an undecided query. | spec/functional/FR-182-settle-a-hyper-verdict.md:205-210; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:151-152 |
| FND-013 | low | [minor] The FR-182 map table, which is exhaustive with no `_` arm and which FR-182-AC-1 tests row for row, does not list the new SMT inputs. V-5's input is only `BoundReached{depth}` (not SMT `BoundReached`). V-8 lists HP-4 and the profile case, not SMT `EncodingRefused` `hyper-body`, field-sort or out-of-fragment. V-6 does not list `Unknown` or `EncodingRefused` `match-conjunct`. V-7 `Cancelled` is listed as `Stopped(Cancelled, None)` only. The behaviour bullets and the table disagree on which inputs reach each row. Fix: add the `HyperSmtResult` inputs to the V-4 to V-8 input cells. Also name the `Undefined` (HV-8) counterexample from a satisfiable base: which `HyperSmtResult` carries it, and its method (`k-induction` or `smt-unrolling`), since the `Violated` comment says "EN-2: a lasso". | spec/functional/FR-182-settle-a-hyper-verdict.md:99, :143-147, :193-195 |
