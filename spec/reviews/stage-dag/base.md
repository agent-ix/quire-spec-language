---
id: SR-466
title: "Base checklist review of ADR-011 stage DAG and dependency architecture"
type: SpecReview
analysis: base
scope: "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
# SR-466: Base checklist review of ADR-011

## Summary

Reviewed commit 944a1c8 on `task/209-stage-dag` against the quoin spec-review
checklist. ADR-011 is an architecture decision record with no US, FR, AC, TC,
option or constraint rows, so the user-story, functional-requirement and six
test-coverage rules have no subject. The applicable gates are ID format and
uniqueness, cross-references, link validity and terminology. The authoring
agent ran this base pass. Independent reviewers ran the seven analyses
(SR-467 to SR-473).

Verdict: ACCEPT WITH FINDINGS (no blocking findings).

## Method

- `ADR-011` matches `^[A-Z]{2,4}-[0-9]+$`. The number was reserved for #209 by
  the coordinator.
- Local item ids are each defined once. They are: stages `S0`…`S8` (with
  `S6a`, `S6b`), side inputs `I1`…`I3`, edges `E1`…`E9`, bypasses
  `FB-01`…`FB-12`, seams `SEAM-1`…`SEAM-5`, extraction `X-1` and module moves
  `M-1`…`M-6`. They are sequential with no gaps, and every other occurrence is
  a reference.
- The relationship targets `ADR-010` and `IT-010` resolve to
  `spec/decisions/ADR-010-observed-architecture-baseline.md` and
  `spec/integration/IT-010-config-version-numeric-backends.md`.
- `spec/spec.md` gains one `contains` relationship and one index row. Both
  resolve to the new file.
- Mermaid blocks contain no `;`.
- `quire validate --scope <repo> <ADR-011> spec/spec.md --strict --summary`:
  2/2 grammar-clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The checklist rules for US, FR and TC quality and the six test-coverage rules do not apply, because the ADR carries no acceptance criteria. This is recorded as not applicable, not as passed. Its decisions are verified by #212 (change scenarios) and #226 (drift gates). | ADR-011 |
| FND-002 | low | The local id schemes (`S`, `E`, `FB-`, `SEAM-`, `X-`, `M-`) are not catalog id kinds. Other records should cite them as `ADR-011 FB-03` and so on, following ADR-010 Decision 5. Fix: state the citation form in the Decision section. | ADR-011 Decision |

## Round 2 (HEAD 5cbd853)

Re-ran the applicable base gates on the revision.

- FND-002 is resolved: Decision 11 states the citation form.
- The revision adds the `ix://agent-ix/quire-specification/AD-016`
  relationship, the `R` and `tool` layers, the new module `semantic_value`,
  M-6 slices `M-6a`…`M-6d`, and scenario rows 8 and 9. Local ids remain
  defined once and sequential.
- Mermaid blocks contain no `;`.
- `quire validate --scope <repo> <ADR-011> spec/spec.md --strict --summary`:
  2/2 grammar-clean.

Round-2 verdict: ACCEPT WITH FINDINGS (FND-001 remains a not-applicable note).

## Author disposition after round 2

The two-round limit is reached, so the round-2 findings were resolved in the
ADR without a third review round. Each item below names the reviewer finding
and where it is now resolved.

| Finding | Resolution in ADR-011 |
| --- | --- |
| SR-472 FND-009, SR-469 FND-015 (high): no legal carrier from `route` to E7 | §2.1 capability routing: the driver builds the registry value and passes candidate sets to CG. The crossing types are #213 value types in a crate below both QSL and CG, or data; #211 decides. No QSL library module calls CG (§6.1). |
| SR-469 FND-014 (high): allow-list omits approved external edges | §6.1: FCD from `model::intake` only; new `I3` row for `quire_source` with quire-rs under the feature; layer 6 reaches quire-rs only through `quire_source`. |
| SR-470 FND-014, SR-468 FND-017 (high): kernel ↔ layer-3 cycle | §6.1 "K is a leaf" rule names every edge X-1 must cut; the per-type cut is handed to #211 against AD-016 WP5a. |
| SR-467 FND-012, FND-013; SR-468 FND-018 | §1 I2: one binding, two outputs (import view, S4 package); the only view constructor is in `library`. |
| SR-468 FND-002, FND-009; SR-467 FND-003 | §2.1 E5 cites the binding; E9 executor key is looked up from the obligation identity (key form to #211); non-completed S6a results settle `inconclusive`. |
| SR-468 FND-019, SR-470 FND-015 | `package_identity` maps to layer 3 `library`; `containment` gains a row. |
| SR-471 FND-009, FND-010 | M-6d lands after the skeleton is green and moves CG's dev pin; `state` and `temporal` are deleted in M-6c and return in #220 and #222. |
| SR-472 FND-010, FND-001, FND-005 leftovers | Scenario 6 defers to ADR-012 §7; scenario 7 uses a `BackendDescriptor`; §2.3 no longer assigns RT and CG gates. |
| SR-470 FND-004, FND-009, FND-016 | Direction and lock checks become Owner question 9; census check moves to #219. |
| SR-468 FND-010, FND-012, FND-016, FND-020 | M-5 after M-3; M-4 ticket in Owner question 6; native-linked-package/1 submodules in SEAM-1; #29, #133, #131, #132 in the ticket table; Terms names three producers. |

Superseded by later revisions (eeabc76, 102c8bb, f781e32). The rows above
record the round-2 state. The current ADR-011 text governs these rows:

- SR-472 FND-009, SR-469 FND-015: there is no shared crate. Routing crosses
  as data (ADR-013 T-7): provider manifests, the candidate-set wire and the
  #229 wire spelling.
- SR-470 FND-014, SR-468 FND-017: the per-type kernel cut is decided in
  ADR-013 T-6 against the AD-016 Shared-type row as amended by QC-15.
- SR-467 FND-012, FND-013, SR-468 FND-018: I2 yields `VerifiedPackage` and
  `ImportView`, both defined in layer-3 `library`. The S4 closure carries
  checked dependency packages compiled from source.
- SR-468 FND-002, FND-009, SR-467 FND-003: the E9 executor key is a typed
  `QualifiedName`, resolved by name lookup through the layer-6 `replay`
  facade.
- SR-471 FND-009, FND-010: M-6 is split by lane. `state` and `temporal` are
  replaced in M-6c by family evaluators under `value::expression`. M-6d
  lands with #218 and #223, and the skeleton spine is §1.1.
- SR-470 FND-004, FND-009, FND-016 and SR-468 FND-010: the owner questions
  became owner rulings and the "Tickets to open at #212" table (T-4, T-8,
  T-12).

Coordinator input folded into the same pass: the answers to ADR-012 §13.1,
consistency with ADR-012 L1-D1 ticket edges, and "the #134 vocabulary".

## Round 3 (HEAD 22fa948)

Independent single-reviewer pass for the #205 coordinator, in two steps. The
first step reviewed f781e32 in full. The second checked only the delta
f781e32 → 22fa948, plus consistency with ADR-012 at 10664aa (QSL PR #234) and
ADR-013 at 4152eb8 (QSL PR #236), read with `git show`.

Findings from the f781e32 review, and their state at 22fa948:

| Finding | Severity | State at 22fa948 |
| --- | --- | --- |
| H-1: `library` (layer 3) converted a layer-4 `VerifiedPackage`, a 3 → 4 cycle | high | Resolved. `VerifiedPackage`, the §4 binding and `ImportView` are in layer-3 `library`. The layer-4 reader calls down (§1 I2, §4, §6.1). Matches ADR-013 T-1 and O-15. |
| M-1: the replay entry had no layer, and E9 ownership differed from ADR-013 O-26, C-13 and TK-01 | medium | Resolved. The layer-6 `replay` facade is the TK-01 executor entry and the only CG-callable API (FB-05, T-12, §6.1). ADR-013 TK-01 names it. |
| M-2: X-1 owner differed from #213 S-1, and the QC-15 blocker was unrecorded | medium | Resolved. X-1 is #213 S-1, gated by TK-10 and QC-15. Matches ADR-013 S-1. IR #139 is merged at 954c2f2, as ADR-011 says. |
| M-3: S6a over a package with imports had no admitted input | medium | Resolved. S4 closure carries checked dependency packages compiled from source, and E6 admits the closure. `ImportView` serves E3 only. See SR-467 FND-014 for the remaining binding rule. |
| L-1: scenario 9 named the I2 reader | low | Resolved |
| L-2: scenario 7 and §1 said `BackendDescriptor` | low | Resolved in ADR-011. ADR-012 §12.3 still says it (SR-468 FND-025). |
| L-3: SEAM-1 omitted `package::view` | low | Resolved |
| L-4: FB-05 could not be checked by `cargo tree` alone | low | Resolved: T-12 adds an API-surface check |
| L-5: `state` and `temporal` were listed as layer-5 modules | low | Resolved |
| L-6: `NodeKey` minting versus "K is a leaf" | low | Resolved in ADR-013 O-04: one kernel constructor from a digest, which only `check` calls. Its enforcer is SR-470 FND-017. |

Coordinator checklist at 22fa948:

- `VerifiedPackage` and the §4 binding are in layer-3 `library`, and the
  `package` reader calls down: yes.
- The layer-6 `replay` facade is the only CG-callable API (T-12) and is
  ADR-013 TK-01: yes.
- X-1 is ADR-013 S-1, gated by TK-10 and QC-15: yes.
- M-6 is split into lanes a to e, each old path is deleted in its
  replacement PR, and only M-6a lands before #216: yes, and ADR-013 Q209-1
  now agrees.
- E9 selects by `QualifiedName`, and a packet with no transcript settles
  `reproduced-without-witness`: yes. See SR-467 FND-015 for the carrier
  naming.
- Dependencies compile from source into the S4 closure, and `ImportView`
  serves E3 only: yes. See SR-467 FND-014.

New findings from the delta and from cross-record comparison:

| Record | ID | Severity | Summary |
| --- | --- | --- | --- |
| SR-468 | FND-021 | medium | `PackageNodeKey` shape contradicts ADR-013 T-3 |
| SR-467 | FND-014 | medium | No rule binds a source-compiled dependency to the verified view that E3 resolved against |
| SR-470 | FND-017 | medium | ADR-013 relies on T-12 to police kernel identity constructors, but T-12 covers only CG's use of `replay` |
| SR-467 | FND-015 | low | Packet carrier named `witness: Option<Witness>`; ADR-013 O-25 names `source: ReplaySource` |
| SR-468 | FND-022 | low | ADR-013 Q209-5 quotes superseded FB-05 text |
| SR-468 | FND-023 | low | ADR-013 O-04 cites "ADR-011 M-3" for dependency compilation |
| SR-468 | FND-024 | low | ADR-013 Context still calls IR PR #139 open |
| SR-468 | FND-025 | low | ADR-012 §12.3 still declares a `BackendDescriptor` in the backend's repository |
| SR-472 | FND-011 | low | Who lands `replay`: the skeleton (T-2) or TK-01 with #214 |

Mermaid blocks contain no `;`.
`quire validate --strict --summary` over ADR-011, `spec/spec.md` and the eight
review records: 10/10 grammar-clean.

Round-3 verdict: CHANGES. No high findings remain. The three medium findings
need text changes before #212: two in ADR-011, and ADR-013 must change with
FND-021. The low findings can land in the same revisions.
