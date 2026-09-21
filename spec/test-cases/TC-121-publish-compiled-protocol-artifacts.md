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
The positive emitter fixture starts with native Quire source, an admitted domain
package and actual definition contracts; a hand-built wire fixture exercises only
reader behavior. The twelve groups below correspond to the twelve acceptance
criteria.

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
   artifact can result.
   The [native producer recipe](../../examples/protocol-handoff/README.md)
   exercises four original predicate/state/temporal/workflow units with cross-unit
   references, bounded `sum`/`size` queries, population/reference roles, actual model
   operations and distinct full/partial recovery requirements. Two receives on
   the original channel feed an owned Boolean choice after an all-branch join;
   check both receive anchors, the channel recipient, original field operands
   and both authored choice branches through the real stripped producer.
   It preserves
   source-owned registration/activation/retry anchors without supplying runtime
   recovery observations; it does not cover this case's full choreography fixture.
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
   bytes, domain package identity, version and declaration keys, manifest/lock closure,
   baseline, inner versus outer profile, feature omission/addition and unknown
   optional feature. Substitute source, domain package `sha256-jcs`, IR and
   result-JCS digests into the external package and source byte slots; require
   typed refusal.
4. Inspect original unit-local expression/control handles, source slices,
   nominal types/units, call order and callee owners, all eight query/graph
   representations and immutable capture origins. Individually break a reference
   kind, scope, initializer order, pre origin, actual definedness prerequisite or
   selected profile. Place prohibited forms in unused/unreachable source.
   No erased operand, symbolic proof witness or guessed model authority can
   repair the refused emission.
   In a two-unit package, place a temporal declaration first and a protocol declaration second.
   Bind the protocol role to a bare record with no object role, require `Invalid::Type`, and resolve
   the returned locus through the reported source index. The indexed source slice must equal the
   exact refused role declaration; the temporal source at the same byte range must not be selected.
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
   For received Boolean choices, admit a real record/object channel to the choice
   owner and complementary guards `received.ready` / `not received.ready` with
   that field in `visible(...)`. Exercise grouping, conjunction, disjunction,
   implication, Boolean equality/inequality, conditionals and source-owned let
   aliases in guards; visible entries retain supported Boolean formulas, direct
   atoms, transparent aliases or closed constants. Check exact original receive binder, field export and anchor
   through emission and independent reading. Repeated reads/aliases share an atom;
   separate receive binders or fields cannot be merged by spelling or equal type.
   Independently enumerate small Boolean assignments to confirm exactly one
   case, retaining original guards and every original operand in the artifact.
   Change the receiver role, omit a guard's visible atom, use an arbitrary input
   or disclose only `a and b` while guarding on `a` / `not a`, or replace a field with another
   receive's field. Missing visibility/provenance returns `Unsupported::FamilyProof`
   after prior stages; an already refused source/scope keeps its actual cause. A guaranteed receive
   after a parallel all-join is positive; the sibling before the join, a
   branch-local record after choice, and an await-success record on timeout are
   unavailable. Aliases/captures cannot bypass those original scope checks.
   Repeat the positive with an attempt owned by the choice role and real pre/post
   contracts from another source unit. Preserve its operation, contract handles,
   result binder and original anchor through emission and reading. Two owned
   attempts after an all-branch join retain distinct atoms even with equal record
   types; `left.ready` / `not right.ready` is not a valid partition. Change only
   the choice role to another role on the same model and require FamilyProof.
   An attempt in a parallel sibling remains unavailable before the join. None
   of these cases treats the attempt result as proof that an effect occurred.
   Repeat the positive with an authored domain event, including one qualified
   by an existing compensation. Check exact event/choice role identity,
   binder/field/anchor identity and retained registration prerequisite through
   emission and reading. Compare an admitted qualified-event baseline without
   the choice to the same source with the choice; the original compensation
   association remains independently required. Refuse another same-model role,
   omitted visibility and nonpartitions over distinct joined event/receive
   records. Send, effect and commit records remain ineligible atom sources.
   Check domain-event resource exhaustion and fresh retry without implying
   registration, activation or business-effect success.
   For composite visibility, accept `visible(a and b)` with guards `a and b`
   and its complement, preserving original expressions through emission/reading.
   Refuse a partition whose selected case differs between assignments with the
   same advertised results, including `visible(a or not a)` with `a` / `not a`.
   Exercise aliases, nested formulas, duplicate advertised roots, foreign-role
   operands and a guard atom absent from every advertised operand closure.
   Exhaust signature allocation/evaluation/comparison budgets; require typed
   incomplete at the original choice, exact/one-short limits and a fresh retry.
   Retain definedness success while giving symbolic guards an abstract overlap
   or hole, including correlated facts whose model constraints are not imported
   into this abstraction; require `Unsupported::FamilyProof`, not a claimed
   concrete counterexample. Closed literal overlap/hole remains `Invalid::Control`.
   Numeric comparisons, queries and callee expansion remain unsupported.
   Admit an observed repeat guard over the same fragment, and require
   `Unsupported::FamilyProof` for a foreign-role, unlisted or composite guard
   atom, and an out-of-scope type discharge refusal, before family admission,
   for an atom established only inside the repeat body. Put hidden inputs behind `false and`, `true or`,
   `false implies` and an unselected conditional branch; require FamilyProof
   rather than admission through the closed-decision path. Closed Boolean let
   aliases retain the evaluated overlap/hole `Invalid::Control` classification.
   Combine the choice with pre/post contracts in another original source unit,
   retaining the exact operation export and contract owners through reading.
   In a constant-guard continuing repeat, compare a symbolic
   choice whose every feasible branch progresses with one having a feasible
   check-only branch; one progressing branch cannot establish the choice's
   progress. The latter remains FamilyProof unless another guaranteed body
   event establishes whole-body progress. A closed check-only repeat retains
   its `Invalid::Control` refusal.
   Repeat the same comparison under an observed guard: the continuing body
   carries the identical progress obligation, a non-progressing body remains
   `Invalid::Control`, an unproved one remains FamilyProof, and guard
   falsification is a normal exit rather than evidence of progress.
   Nest that repeat inside an enclosing bounded repeat that has no other
   progress source: a feasible false valuation supplies the enclosing repeat
   no progress and refuses with `Invalid::Control`, while a guard no valuation
   falsifies supplies it and admits with the authored guard retained.
   Author a payment retry as a bounded repeat over a pre-loop observed charge
   outcome. Require one static attempt identity for the authored attempt,
   distinct from the carrying delivery and from the single business effect,
   with no further attempt record minted per iteration, and require that a
   retried attempt inherit no earlier effect. Exercise zero, exact and one-short budgets for the repeat's
   valuation work.
6. Bind O1 and O2 to the same provider through distinct instance requirements.
   Preserve one message, two delivery observations and one effect. Inspect
   effect-before-registration, separate registration/activation captures,
   distinct retry attempts, commit and the actual full/partial recovery
   predicates. Substitute a transport identity for an effect, O2's capture for
   O1's, omit required static population/relationship authority, or replace
   recovery with operation success. Require refusal of the relevant boundary;
   an absent future trigger/observation is not fabricated during compilation.
   For an identity-only compensation effect, check null value type with the
   exact operation export, owning compensation subject, retry anchor and sole
   attempt-binding prerequisite. Independently remove its model, select a wrong
   export, swap subject/anchor/attempt with another obligation, or sever the
   registration-to-forward, activation-to-registration or attempt-to-activation
   chain. Require `Invalid::Binding` for these semantic mismatches, retaining
   `Invalid::Owner` for foreign-declaration handles. Null an ordinary effect's
   value type and require refusal. Insert a payload type into the compensation
   effect while retaining its exact operation; require `Unsupported::Export`
   because the profile has no authoritative typed-effect correspondence. Insert
   that type alongside a foreign operation and require `Invalid::Binding`.
   Swap compensation clocks, recovery snapshots, progress or closure records;
   sever the snapshot's activation edge or each progress/closure dependency on
   clock, effect or snapshot independently. Require `Invalid::Binding` and retain
   a positive control with every original prerequisite present. Unrelated
   population pairs cannot enter a compensation's recovery inventory. Derive
   expected recovery anchors independently from the original recover-root
   operand, binder-initializer and selected-origin dependencies, including the
   recovery anchor itself. Compare native emission with reading the unchanged
   bytes for captured aliases and mixed origins. A self-selected marker adds no
   edge, and proof simplification cannot remove an original operand. Distinguish
   actual dependencies from merely enclosing source spans: use a Boolean capture
   with an unused let initializer and an unselected conditional operand, then
   compare recovery that reads the capture with recovery that does not. The
   unread capture and a neighboring obligation cannot add recovery populations.
   Omit a contributing population/closure pair or add an unrelated pair and
   require `Invalid::Binding`.
   Calls retain original argument dependencies without guessing callee captures.
   Retain the exact signed-64 maximum attempt bound through compilation and reading;
   refuse an overflowing bound without expanding attempts. No operation result
   or recovery snapshot supplies the missing effect identity or correspondence.
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
   Include private Boolean atom/formula records, repeated provenance/operand
   visits and case/valuation inspections in those independent vectors. Lowered
   proof-work limits return source-located incomplete, preserving prior results
   and distinguishing exhaustion from an unproved partition.
10. Pass the actual production-emitted immutable artifact/reference to
    [quire-protocol IT-001](ix://agent-ix/quire-protocol/IT-001) through the public
    Rust interfaces, preserving exact selectors and finite graph/source links.
    In B's Rust integration harness, independently select the supplied original
    sources, admitted domain packages, registry/contract and compiler source
    identity (a digest on `Producer.binary` over the producer's own source
    text, not a dependency's retained bytes) to construct the reader's
    accepted inventory. Compare the offered package against that inventory;
    neither the payload nor the fixture's `expected.json` authorizes its own
    selections.
    Retain all four original source files and the recipe's own source
    identity. Reconstruct its admitted model declarations through the same
    intake path; do not replace the model or the producer's identity with
    invented metadata.
    A's local emission/reader round trip alone leaves this B acceptance incomplete.
    Perform its one-axis adverse controls and numeric boundary handoff. A
    missing family, model intake or consumer implementation records the
    unmet positive integration prerequisite; a negative unsupported test cannot
    stand in for successful source-to-consumer emission.
11. Run the [Rust emitter recipe](../../examples/protocol-handoff/README.md)
    rewritten over the filament-core-data#173 architecture fixture bundle: admit
    its domain package through the real FR-056 intake seam (parts, interfaces,
    ports and a connection between two ports, as IT-012 exercises), link a
    native package that references it, and emit the compiled-protocol package.
    Decode the emitted `Model` and confirm its identity, version and digest,
    read directly from the `Model` record, equal the `DomainPackageRef` the
    intake seam admitted, with no `correspondence`, `producer` or `interface`
    member present. Separately, hand-construct a decoder input whose `Model`
    object adds a `correspondence` member (with any value, including explicit
    `null`) or a `producer`/`interface` member shaped like the deleted
    `ProducerObject`/`Correspondence` records, and require the reader to refuse
    each as an unrecognized field rather than accept or silently drop it.
    Inspect the compiler source tree and confirm no `ProducerObject` or
    `Correspondence` type or wire tag remains, and that the domain-package
    member decodes through a plain `record!` field rather than any internally
    tagged (`#[serde(tag = ...)]`) representation (FR-042-CON-2); the
    reader's refusal of a `correspondence`/`producer`/`interface` member
    above is the behavioral proof that mechanism actually fires, not an
    assumption about `deny_unknown_fields` in the abstract. Regenerate both
    existing golden directories, `artifacts/compiled-protocol-v1` and
    `artifacts/compiled-protocol-v2`, and confirm both directories' own tests
    still pass under the new `Model` shape, at their existing
    `quire.compiled-protocol/1` and `quire.compiled-protocol/2`
    ([FR-050](../functional/FR-050-publish-authenticated-temporal-artifacts.md))
    wire identities unchanged; `quire.compiled-protocol/3`
    ([FR-054](../functional/FR-054-publish-control-temporal-activation-map.md))
    keeps its own wire identity and activation-mapping delta, and only its
    shared `Model` population changes shape.
12. Compile a native package that admits no domain package at all (a
    directly admitted native model) and decode its emitted `Model`; confirm
    its domain-package member decodes as explicit `null`, distinguished by
    byte content from the member being omitted. Separately hand-construct a
    decoder input for such a model that omits the member entirely and require
    the reader to refuse it, exactly as an omitted `Nullable<T>` member
    refuses elsewhere in this contract, never reading the omission as an
    implicit `null`.

## Expected Results

The complete emitted package preserves the accepted native subject and its
distinct source/domain-package/profile/compiler authorities in one canonical artifact.
Only its independently selected bytes pass strict consumption. Partial or
unsupported work remains explicit; numeric codec, type and wire-reader success
are narrower evidence than this full acceptance. No full emitter or consumer
result is claimed until the actual pipeline and planned controls execute.
