---
id: SR-626
title: "Dependency analysis of ADR-014 temporal, trace and boundedness architecture"
type: SpecReview
analysis: dependency
scope: "spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md; its amendments to ADR-012 §1.1, ADR-013 (O-20, O-21, S-6, Q222 table), FR-057, FR-082 and spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: reviews
---
# SR-626: Dependency analysis of ADR-014

## Summary

Round 1. Reviewed commit `bbe92fec` on `spec/17-boundedness-adr` against
`origin/main` `fc27aacc`, QSpec `main` `eb4234f` and
quire-contract-codegen `origin/main` `e2a5671`. Linear state was read on
2026-09-24. Ticket text was treated as data.

What holds:

- The bound taxonomy (B-1 to B-6) and its two derivations are acyclic.
  Enablement (QSL-140's kernel and family types) comes before the features
  (QSL-42, QSL-43) that consume it.
- The QSL-17 → QSL-140, QSL-42, QSL-43 and QSL-15 `blocks` edges exist in Linear.
- codegen#86 (IR-23) is Done. CG `negotiate_*` takes an extent
  (`ExtentClassification`, `src/capability.rs:153-163`) and settles
  `requires-bound` or `unbounded-extent` from it (`:690-716`), as §4 and §6
  need.
- The §13 QSpec gap is real. The v2 operation catalog has only
  `quire.op.temporal.clause`. The `TemporalNode` body is unconstrained
  (`proposals/checked-package-v2/schema.json:103`). IR-7 is the correct IR
  consumer.
- The QSL code claims hold at the cited lines: `CardinalityBound`,
  `Population(u64)`, the mandatory collection bound in the grammar,
  `proof_bounds: ScalarLimits`, `DeclaredDomain.domain: String` and
  `MAX_SEQUENCE_ITEMS`.

What does not hold: the acceptance bar is "QSL-140, QSL-42 and QSL-43
implement through named interfaces with no new ownership decision", and
that bar is not met. Two placement or ownership decisions are still open
(FND-001 and FND-002). The predicate has no named evaluator (FND-003).
Several prerequisite edges are missing from §11, §13 and Linear (FND-004
to FND-008). §13's "one open dependency" is therefore not accurate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The owner of `Requirements` and `FamilyContract::requirements()` is undecided. §4 puts `ClaimExtent` "beside `Requirements`" (QSL-140). §11 has QSL-42 compute it in "the method QSL-152 adds". ADR-012 §2 says the first family with a real FR-057 kind adds the method, and `qsl-semantics/src/family/mod.rs:49-70` says the same. QSL-152 is Backlog, has no Linear edge to QSL-42 or QSL-43, and its scope is to decide *whether* the part returns. QSL-43 needs the method too (A-3), but its §11 list omits it. Fix: name one owner in §11 for the `Requirements` type and the trait method (for example QSL-140), add it to QSL-43's list, and either add QSL-152 `blocks` QSL-42 and QSL-43 or state that QSL-140 supersedes QSL-152 AC-4. | ADR-014 §4, §11; ADR-012 §2; QSL-152 |
| FND-002 | high | `ProofBound` is placed in `qsl-semantics::family` (layer 3), but §1 and §11 type `qsl-replay`'s `DeclaredDomain.domain` as `ProofBound`. `qsl-replay` depends only on `quire-exact` and `qsl-foundation` (ADR-011 X-10; `qsl-replay/Cargo.toml`). CG takes a normal dependency on `qsl-replay` (ADR-011 T-14). The change therefore adds a replay → semantics crate edge and pulls layer 3 into CG. That is a placement decision the record does not make. Fix: put `ProofBound` in `qsl-foundation` or `quire-exact`, or keep `DeclaredDomain.domain` wire-typed with a conversion at the layer-3 boundary. State the choice in §1 and §11. | ADR-014 §1, §4, §11; ADR-011 X-10, T-14 |
| FND-003 | medium | The available-finite-bound predicate has no named evaluator or carrier. §4 defines it as "the request carries a `ProofBound` for every unbounded domain". The amended ADR-013 O-20 row still says "the `requires-bound` predicate is IR's (AD-016)". CG reads a precomputed `finite_bound_available: bool` "read from #222, never inferred" (`src/capability.rs:162`). FR-331's request has item extent, classification and `domains`, but no such flag. `Requirements{kind, extent, bounds}` does not list which domains (by parameter node) are unbounded, so nothing can match them against `domains`. Fix: say who evaluates the predicate (CG from FR-331 `domains`, or QSL when it builds O-20). Name the per-item unbounded-domain list QSL emits as FR-331's item extent, for example `ClaimExtent::Unbounded{domains}`. Correct the O-20 owner row. | ADR-014 §4; ADR-013 O-20; QSpec FR-290:118-140, FR-331:28; CG `capability.rs:153-163` |
| FND-004 | medium | Linear has no enablement edges from QSL-140. QSL-140 blocks only QSL-23. §11 says QSL-42 "builds against those types", and QSL-43 needs `ClaimExtent` for A-3. Fix: add QSL-140 `blocks` QSL-42 and QSL-140 `blocks` QSL-43. | ADR-014 §11; Linear QSL-140 |
| FND-005 | medium | QSL-43's prerequisites are incomplete. ADR-012 §1 has TemporalTrace read `StateModel` and `ProtocolClause` checked types. Their migrations are QSL-68 (#120) and QSL-21 (#218), both Backlog and not linked to QSL-43. The spine has no temporal S2 forms and no S3 temporal check: `qsl-forms` has none, and `qsl-semantics/src/check/mod.rs:1503` says there is no temporal clause syntax yet. ADR-011 M-3b and `:847` give the bounded TemporalTrace migration and the M-6c deletion of `src/temporal` to #188 and #189. §11 lists only the infinite-trace additions. Fix: list the M-3b temporal forms, the bounded TemporalTrace `check` and evaluator, and M-6c in QSL-43's scope. Name QSL-21 and QSL-68, or the specific checked types QSL-43 needs from them, as prerequisites, and add the Linear edges. | ADR-014 §11; ADR-012 §1; ADR-011 M-3b, :847 |
| FND-006 | medium | A-1 has S1 and S2 parse bare unbounded operators, but the spine grammar requires `Interval` on `eventually`/`always`/`once`/`historically` and on the binary operators (`qsl-cst/src/grammar.rs:1000-1002`, `:1009-1017`). Only `[a,*]` parses today (`:946-952`). QSL-43's §11 list does not include this grammar change, while QSL-42's list does include its own. Fix: add "make `Interval` optional in `TemporalUnary` and `TemporalRelation`" to QSL-43 in §11. | ADR-014 §5 A-1, §11 |
| FND-007 | medium | The §6 step 4 route guard has no named interface. `qsl_route::routing::route(dispositions: &[Disposition]) -> Vec<Option<&Candidate>>` (`qsl-route/src/routing.rs:58`) takes no extent and no registry, and cannot return a refusal. The guard needs a new signature and a refusal type. QSL-42 would have to design both. Fix: name the signature in §6 or §11: per-item `ClaimExtent`, the `Registry` descriptors, and a result carrying `invalid_capability`/`inconsistent-candidates`. | ADR-014 §6, §11 |
| FND-008 | medium | §13's single open dependency has no owning ticket. No QSpec (STD) ticket covers v2 temporal operation identities or the `TemporalNode` body, and IR-7 is not linked to one. Given FND-001 and FND-005, "One" also undercounts the open prerequisites. Fix: file the STD ticket, cite it in §13, and add it `blocks` IR-7 and QSL-43's S4 emission. Rewrite §13 to list every open prerequisite. | ADR-014 §13; Linear IR-7 |
| FND-009 | low | §6.5 relies on "Kani's manifest advertises its kinds with mode `bounded` only". No Kani manifest with advertised (kind, mode) pairs exists on CG, IR or QSL `main`. CG builds a `BackendDescriptor` only in tests. QSL-42's scenario 3 exit depends on that manifest. Fix: name the manifest's owner and its ticket (CG), or state that the exit cases use a test descriptor. | ADR-014 §6.5, §10 scenario 3 |
| FND-010 | low | §6 step 3 says a candidate's arm decides whether it discharges the IR form. CG's only arm, `negotiate_kani` (`src/capability.rs:690-716`), settles on modes alone and checks no form. This does not block the QSL-42 or QSL-43 exits (no liveness backend). Fix: record the form check as a CG follow-up under IR-23's successor. Do not cite codegen#86 as already providing it. | ADR-014 §6, §11; IR-23 |
| FND-011 | low | The QSL-42 and QSL-43 bodies give IR-89 (contract-ir#109, frame clauses) as the prerequisite that "supplies the temporal IR form". The temporal intake is IR-7, which §13 names correctly. The Consequences note on stale ticket bodies omits this. Fix: add IR-89 to that note, and relink QSL-43 to IR-7. | ADR-014 §13, Consequences; Linear QSL-42, QSL-43, IR-89, IR-7 |

## Resolution

Resolved by the author in the follow-up commit on `spec/17-boundedness-adr`.

FND-001 fixed: QSL-140 builds `Requirements` and `requirements()`, superseding QSL-152 AC-4 (§11). FND-002 fixed: `ProofBound`, `DomainKey`, `FiniteBound` in F `bound`. FND-003 fixed: QSL-140's request writer computes `finite_bound_available` (§4, §6 step 3). FND-004 and FND-008: Linear edges (QSL-140 blocks QSL-42/QSL-43; a QSpec ticket for v2 temporal operations blocking IR-7) are reported to the coordinator, not edited here. FND-005 fixed: QSL-43 scope and QSL-68/QSL-21 prerequisites in §11. FND-006 fixed: A-1 and §11. FND-007 fixed: guard dropped. FND-009 fixed: §6 step 5 names CG's manifest and the test descriptor. FND-010 fixed: §6 step 4 records the form check as CG follow-up. FND-011 fixed: Consequences names IR-89 and IR-7.
