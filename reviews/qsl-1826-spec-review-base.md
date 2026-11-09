---
id: SR-20261109172912000007
title: QSL-1826 checked-subject witness replay base review
type: SpecReview
analysis: base
scope: agent-ix/quire-spec-language; spec/functional/FR-202-replay-a-probabilistic-witness.md;
  spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md
review_set: subset
---

## Summary

Ticket: QSL-1826. Frozen PR: quire-spec-language#2610, head b4f7d3870208910499a6d8b3c4344c25a72449e4. Diff: origin/main...HEAD only. FAIL: one medium structural defect. Acceptance obligations are otherwise concrete and falsifiable.

No active `type: AssuranceProfile` document was found in spec/. Direct subset review: base, integrity, failure-domain, scope-boundary and ears-conformance. No required enforced run applies.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A blank line separates AC-9 through AC-11 from the headered acceptance table, so Quire recognizes only the eight existing criteria; join the rows to the table (or add a second header) so all eleven criteria are extracted. | spec/functional/FR-202-replay-a-probabilistic-witness.md:248 |

## Examined scope

```yaml
scope:
- id: FR-202-checked-entry-1
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'In addition to the wire facade below, layer 6 SHALL expose a checked-subject

    path-set replay entry for untimed acyclic reachability claims, with no fairness,

    one initial state, at most one bound instance, and a deterministic memoryless

    witness scheduler (or the bound workload). It SHALL take the `CheckedPackage`,

    its admitted `ModelSystem`, the claim occurrence and binding, typed path-set

    evidence, caller-configured expansion/work/storage limits and `&Cancel`.

    This entry decodes no wire and recompiles nothing, as the checked-subject

    certificate entry in FR-201 does. It SHALL bind the claim fro'
- id: FR-202-checked-entry-2
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'Typed evidence SHALL retain actual semantic action identities, resolved

    `ModelTransition` content and draws, initial and post-state content, and complete

    canonical product keys. The entry SHALL reuse the qualified semantic types

    (`ScheduledIdentity` and `ModelTransition`) without importing `qsl-analyze`

    product/action types or treating local state/action numbers as identities.

    The core SHALL independently re-enumerate enabled transitions, evaluate target

    predicates, check the scheduler, and recompute exact path mass and prefix freedom

    under the Replay rules below. An asserted transition probab'
- id: FR-202-checked-entry-3
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'For an `AtMost` bound `Pr(E) <= theta`, accepted paths SHALL end at their
    first

    target hit and their mass SHALL exceed `theta`. For an `AtLeast` bound

    `Pr(E) >= theta`, accepted paths SHALL avoid the target and end at leaves where

    the core independently confirms no enabled continuation; their mass SHALL

    exceed `1 - theta`. An initial target is a first hit with an empty path; a live

    non-target prefix is not a terminal nonreach leaf. These are sufficient

    refutations under the bound workload or a consistent deterministic witness

    policy; an engine''s assertion that a policy is optimal is not replay'
- id: FR-202-checked-entry-4
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'The entry SHALL produce an opaque reproduced-witness token only after
    all

    checks and a final cancellation poll. The token SHALL bind the package, claim

    occurrence, subject binding and accepted evidence. Layer-6 EN-5 terminal

    construction SHALL require the matching token to settle `refuted` and retain

    the probabilistic counterexample evidence in the analyze record. A caller

    SHALL NOT construct the token or turn a bare engine verdict into `refuted`.

    Replay disagreement/refusal SHALL retain OP-2''s `replay_parity`/`replay_refused`

    settlement; exhaustion and cancellation SHALL remain `incomplete` w'
- id: FR-202-checked-entry-5
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'This entry SHALL report unsupported for cyclic subjects, subsystem evidence,

    randomized schedulers, fairness, multiple initial states/instances, timed or

    other claim forms outside this slice. It SHALL NOT project them onto this

    slice. This additional entry does not change the wire facade obligation, the

    broader replay rules, or any public JSON key or witness codec.'
- id: FR-202-AC-9
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: A checked-subject acyclic reachability fixture replays a target-first-hit
    path set of mass `2/3` against `AtMost 1/2` and a terminal-nonreach path set of
    mass `2/3` against `AtLeast 1/2`; each produces a token permitting only its matching
    EN-5 refuted record, with the accepted evidence retained. Repeating replay gives
    equal semantic outcomes. Neither replay needs an emitted package or analyzer graph
    types.
- id: FR-202-AC-10
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: In AC-9, altering the claim occurrence, package or binding refuses; altering
    an action, draw, post-state or full key cannot produce a token. Duplicate/prefix-overlapping
    paths, inconsistent scheduler choices, an earlier target on a first-hit path,
    and a live non-target leaf cannot reproduce a refutation. Mass equal to the relevant
    threshold yields replay parity.
- id: FR-202-AC-11
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: A pre-cancelled call, cancellation at the final acceptance poll, or exhaustion
    of each configured expansion/work/storage limit returns incomplete without a token
    or a refuted record. Cyclic, subsystem, randomized-policy, fairness, multiple-initial/instance
    and timed inputs report unsupported without a token. A caller cannot construct
    a reproduced token or settle EN-5 refuted from a bare engine result.
- id: ADR-029-OP-2-checked-replay
  path: spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md
  role: examined
  excerpt: "- **EN-5 checked-subject path replay (proposed amendment, QSL-1826).**\n\
    \  FR-202 adds a qualified-core entry over the checked package and admitted\n\
    \  subject for its narrow untimed acyclic reachability path-set slice, alongside\n\
    \  the wire/recompile facade. As with FR-201's checked-subject certificate\n \
    \ route, this avoids requiring an emitted package that FR-185 refuses. The\n \
    \ core rebinds the claim and replays actual semantic steps independently;\n  it\
    \ imports no engine product graph or optimizer. Only its opaque,\n  item-bound\
    \ reproduced-witness token permits an EN-5 `refuted` terminal\n  record, "
- id: FR-202
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: context_only
  excerpt: EN-5 SHALL refute a claim with a `ProbabilisticCounterexample` (ADR-028
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: The checker SHALL refuse a certificate whose identity differs from the
- id: FR-282
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: context_only
  excerpt: 'The EN-5

    checker has two forms with one set of conditions (FR-201 "Checking").'
```

## Evidence and limits

Quire validation of the two edited artifacts exits 0, with pre-existing module-registry advisories. It reports eight criteria. An external scratch copy with only the blank line before AC-9 removed reports eleven criteria. No source file was changed. `git diff --check` passes. No executable embedding dependency was found. New ACs are prospective implementation obligations; no whole-repository matrix, runtime verification or assurance completion is claimed. Existing FR-202 wire/recompile duties and FR-201/282 checked-subject certificate context were examined for compatibility.

## Verdict

FAIL: one medium structural defect. Acceptance obligations are otherwise concrete and falsifiable. Not mergeable until the table structure is fixed and revalidated. Broader replay implementation and QSL-1826 follow-ups remain open.
