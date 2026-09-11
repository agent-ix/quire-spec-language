---
id: TC-121
title: "Preserve source-owned protocol artifacts through strict Rust consumption"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-042, type: verifies }
  - { target: ix://agent-ix/quire-protocol/IT-001, type: references }
---
# TC-121: Preserve source-owned protocol artifacts through strict Rust consumption

## Description

Planned public Rust controls for the full
[FR-042](../functional/FR-042-publish-compiled-protocol-artifacts.md) artifact
boundary, using the exact [wire contract](../../docs/compiled-protocol-v1.md).
The positive producer fixture starts with native Quire source and actual admitted
model/definition/producer contracts; a hand-built wire fixture exercises only
reader behavior. The ten groups below correspond to the ten acceptance criteria.

Use source-owned orders O1/O2 sharing payment provider P, split shipments S1/S2,
payment attempts A1/A2 and effect E1, refund registration and distinct refund
attempts/effects R1/R2. Include called state predicates, a bounded temporal
obligation, choice visibility, two channels, an await, bounded repeat, join,
commit and both full/partial recovery relations. Give every source, requirement,
clause, model, contract, clock and instance role an explicitly authored identity;
do not substitute model ownership for native clause ownership. Concrete
observations belong only to the later consumer fixture.

## Test Procedure

1. Admit the complete source inventory through the real edition parser,
   namespace/definition/model/scope binding, value types, actual definedness and
   family admission. Emit through the production Rust entry point. Check the
   exact selectors and external SHA-256 over the emitted bytes. Attempt emission
   with type-only/unfinished evidence and a decoded wire value; no accepted
   producer artifact can result.
2. Compare emitted compact bytes with independently authored expected records
   and escape vectors (ASCII controls, quote/backslash, supplementary Unicode,
   slash, no NFC conversion). Reorder dependency/source sets without changing
   authored sequences and require byte equality; reverse an authored sequence
   and require a different artifact. Exercise integer safe endpoints/adjacent
   values, signed-64 extrema, integer 1 versus rational 1/1, 1/3 and zero 0/1,
   every bound field's minima/maxima, normalized source rational(2,4), and
   malformed, unreduced, wrong-domain, bare-number and structural-number mutants.
3. Mutate each reference component independently, retaining the expected
   accepted selection. Include native versus formal source revisions, rule
   bytes, model exports, producer/native correspondence, manifest/lock closure,
   baseline, inner versus outer profile, feature omission/addition and unknown
   optional feature. Substitute source, model/config canonical, IR and result-JCS
   digests into the external package/model byte slots; require typed refusal.
4. Inspect original unit-local expression/control handles, source slices,
   nominal types/units, call order and callee owners, all eight query/graph
   representations and immutable capture origins. Individually break a reference
   kind, scope, initializer order, pre origin, actual definedness prerequisite or
   selected profile. Place prohibited forms in unused/unreachable source.
   No erased operand, symbolic proof witness or guessed model authority can
   repair the refused emission.
   For native reference populations, use two admitted object roles sharing a
   universe label and independently selected models. Check the exact original
   role locus and model/object/universe binding through current, pre-state and
   captured-reference uses without supplying observations. Substitute one
   reference, object, population export, model owner or source locus at a time;
   require the corresponding typed refusal. Population closure remains a
   declared input requirement, not an observed compilation result.
5. Compare the explicit causal graph with the authored structured controls.
   Exercise both branch interleavings and equal-time observations without added
   edges. Mutate owner/visibility, overlapping labels/guards, join targets,
   await branch/anchor, repeat maximum/progress, termination and channel-local
   FIFO premises one at a time. Check wrong-kind send/receive/attempt/effect
   references and independently bounded nested/shared control graphs. Maximum
   zero takes normal/exhausted control without the repeat body; checks alone
   cannot establish continuing progress. Test both event and compensation await
   anchors, inclusive deadline matches and separately supplied timeout closure.
   Remove the static progress/closure requirement, change its await subject or
   remove its clock-binding dependency; require `Invalid::Binding` from the reader.
6. Bind O1 and O2 to the same provider through distinct instance requirements.
   Preserve one message, two delivery observations and one effect. Inspect
   effect-before-registration, separate registration/activation captures,
   distinct retry attempts, commit and the actual full/partial recovery
   predicates. Substitute a transport identity for an effect, O2's capture for
   O1's, omit required static population/relationship authority, or replace
   recovery with operation success. Require refusal of the relevant boundary;
   an absent future trigger/observation is not fabricated during compilation.
7. Give exact emitted bytes and external reference to the bounded public reader
   without invoking native parsing. Mutate every closed object/tag/member,
   duplicate equal entries, introduce invalid indices/cycles/noncanonical bytes,
   then change graph/literal/profile claims and recompute a new seal. Under the
   original expected selection all resealed mutants refuse. Separately show
   that an attacker-supplied replacement seal is not accepted inventory or
   source-compilation proof. Compare typed causes/loci, never English messages.
8. Refuse one required family/export beside an independent valid declaration;
   retain both compilation dispositions but no full-package artifact. Keep
   unsupported reader/version/canonical-domain, known input refusal and resource
   incompleteness distinguishable. A valid global subject may retain a separate
   unsupported projection result. Re-run the historical fixed artifact/profile
   controls without changing their identities or admission surface.
9. Calculate small fixture bytes, entries, links, type/reference visits and
   canonical writes independently of reported usage. Test each effective limit
   at zero, exact and one-short, including source/dependency content, output,
   depth, repeated/shared edge visits and diagnostic retention. Test above-hard
   clamping, arithmetic overflow and fresh retries. No reader/emitter may
   allocate an uncharged expansion or expose an unfinished package.
10. Pass the actual production-emitted immutable artifact/reference to
    [quire-protocol IT-001](ix://agent-ix/quire-protocol/IT-001) through the public
    Rust interfaces, preserving exact selectors and finite graph/source links.
    Perform its one-axis adverse controls and numeric boundary handoff. A
    missing family, producer adapter or consumer implementation records the
    unmet positive integration prerequisite; a negative unsupported test cannot
    stand in for successful source-to-consumer emission.

## Expected Results

The complete emitted package preserves the accepted native subject and its
distinct source/model/profile/producer authorities in one canonical artifact.
Only its independently selected bytes pass strict consumption. Partial or
unsupported work remains explicit; numeric codec, type and wire-reader success
are narrower evidence than this full acceptance. No full producer or consumer
result is claimed until the actual pipeline and planned controls execute.
