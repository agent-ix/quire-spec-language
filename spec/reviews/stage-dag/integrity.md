---
id: SR-468
title: "Integrity review of ADR-011 stage DAG and dependency architecture"
type: SpecReview
analysis: integrity
scope: "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/spec.md (ADR-011 index row)"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
# SR-468: Integrity review of ADR-011 stage DAG and dependency architecture

## Summary

Reviewed ADR-011 on `task/209-stage-dag` at 944a1c8, plus the one uncommitted
working-tree edit present during review (Decision 11, local-id citation form),
and its index row and `contains` relationship in `spec/spec.md`. ADR-011 is a
design record, not a requirement set. So this pass checks four things:
completeness against the #209 Required design and Acceptance bullets,
internal consistency of the stage, edge, bypass, seam and move ids across
tables and diagrams, the mapping of every current `src/lib.rs` module, and
consistency with accepted QSpec AD-016 and the sibling boundaries (#210, #211).

What holds:
- The #209 flow maps one to one onto S0 to S8.
- Every edge E1 to E9 has admitted types, an owner, a preservation row and a
  partial-output rule.
- All 19 ADR-010 findings routed to #209, plus the three secondaries, have a
  §9 row.
- All seven #209 change scenarios are placed.
- Every §7.3 move has direction, API, order and disposition columns.
- The compatibility disposition is uniformly none.
- The index row, link and `contains` relationship are correct.
- The ADR hands type names, typestate encoding, the outcome vocabulary and the
  locus type to #211, and hands family hooks and capability content to #210.
  It decides none of them. No AD-016 decision is reopened.

The defects are in how the edges fit together and in the module map:
- **FB-05 and E9 contradict each other (FND-001).** E9 needs an in-process S4
  package inside CG. FB-05 lets CG use only the S6a entry. So the replay path
  E9 defines is forbidden as written.
- **Undefined terms.** "Verified binding" (FB-03, E5, I2) is not defined, and
  the definition of "Checked" contradicts the I2 reader.
- **Two readings of the layer rule.** §6.1 can be read so that S6a may depend
  on S2 `forms`, which §4 forbids.
- **The module map is not complete at submodule level.** It places `value`
  submodules in the `quire-exact` kernel beyond AD-016's "exactly these types"
  row. It leaves the unchecked `value::expression` tree unplaced. `model`
  keeps a dependency on `check` that M-2 says cannot exist. `command` and
  `package` are split into halves the ADR does not name.
- **Smaller issues.** §7.3 has a stage-id slip ("S5 `evaluate`"). The
  migration order is partial and one move has no owning change. §4 says there
  are two `CheckedPackage` types; AD-016 says four, and Owner decision 6
  defers renames.

#209 Acceptance status:

| #209 bullet | Status |
| --- | --- |
| Every current module/crate maps to one stage or temporary seam | Partial: FND-004, FND-005, FND-006, FND-007 |
| No downstream stage reparses text, CST, display strings or diagnostics | Met (FB-01, FB-02), subject to FND-009 (string name lookup at S6a) |
| Checking precedes lowering; no ambiguous checked/unchecked public type | Met in intent (§4); FND-002 and FND-003 leave two readings |
| CLI orchestrates stage APIs with structured outcomes | Met (§5, design delegated to #225) |
| Extractions state direction, API, migration order, compatibility | Partial: FND-010 (order and owner) |
| `/specify` and `/spec-review all` complete with findings resolved | Pending: this review is part of that set |

Verdict: **ACCEPT WITH FINDINGS.** The architecture is sound and fits AD-016.
FND-001 (high) must be fixed before ADR-011 is accepted or #212 runs, because
#212 scenarios 6 and 7 walk E9. FND-002 to FND-010 (medium) should be fixed in
the same revision.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | E9 contradicts FB-05. E9 admits "the S4 package of the same identity" and calls the S6a executor. That executor, `CheckedPackage::call(&self, …)` (AD-016 arrow 7), is a method on an in-process checked package. The CG replay adapter must obtain one, either by running QSL S0 to S4 on the source or by reading a package through I2. Both mean depending on QSL Rust types beyond "the S6a entry", which FB-05, §7.1 and scenario 7 ("depend on QSL types other than the S6a entry") forbid. The mermaid also draws no S4 → S8 input. Fix: name the QSL library surface the adapter uses to get the S4 package, either a pipeline entry (S0 to S4) or the I2 reader over v2 bytes with identity verification. Widen the FB-05 exception, the §7.1 CG → QSL edge label and scenario 7 to exactly that surface plus the S6a entry. Add E9's S4 input to the diagram. | ADR-011 §1, §2.1 E9, §3 FB-05, §7.1, §10 #7 · AD-016 arrow 7, Owner decision 5 |
| FND-002 | medium | "Verified binding to a checked producer" (FB-03) is undefined, so FB-03 cannot be tested. E5 is itself wire-admitted data reaching a backend (IR `CheckedPackageV2`), yet the E5 row names no binding. I2 is QSL reading packages whose identity "is verified when it is read", and identity verification proves integrity, not that a checker produced the package. As a result, E5 either violates FB-03 or FB-03 does not cover it, and I2 may or may not satisfy it. Fix: state what counts as the binding (for example the S4-minted package identity and digest, checked on read). State that E5 and I2 meet FB-03 by that binding, or scope FB-03 to exclude them explicitly. If #211 owns the rule, add FB-03 to the §4 question already handed to #211 so both cite one definition. | ADR-011 §1 I2, §2.1 E5, §3 FB-03, §4, Questions to #211 |
| FND-003 | medium | The term "Checked" contradicts the I2 reader. Terms says checked means "produced by the S3 check stage. Nothing else constructs a value of a checked type." §4 and I2 have the S4 `package` reader construct a checked package from bytes, and E5 has IR construct `CheckedPackageV2`. Fix: redefine "Checked" as "produced by S3, or re-established from an S4 identity by the I2 reader under the §4 rule". State that IR `CheckedPackageV2` is an S5 type named by AD-016 and is not a QSL checked type. | ADR-011 Terms, §1 I2, §4 · AD-016 arrow 2 |
| FND-004 | medium | The §6.1 layer rule can be read two ways. The prose says a module may depend on "modules in lower layers", which allows layer 5 `evaluate` to depend on layer 2 `forms`. That contradicts §4 ("no path from S2 to … S6a") and FB-04. The Depends-on column instead lists subsets: layer 5 is "4, 3, K" and omits F, although S6a emits `Evaluation.location` and diagnostics. Fix: make the Depends-on column the exhaustive, normative allow-list. Add F to layer 5. State that no layer at or above 3 other than `check` may depend on layer 2, and name the check (#226) that enforces the column. | ADR-011 §4, §6.1, FB-04 |
| FND-005 | medium | The `value` → K mapping contradicts AD-016's kernel row. AD-016 says the `quire-exact` kernel holds "exactly the types listed in this row plus `Undefined` and the scalar/collection operations" and is `no_std` + `alloc`. §6.2 sends `value` "and the other value operations" to K. That catch-all covers `value::definition` (the definition-lock catalog and profile admission, serde, `OnceLock`), `value::enumeration` (declaration node identities, which AD-016 makes layer-owned), `value::unit` (the admitted unit graph) and the declaration parts of `value::composite` (`TypeEnvironment`, `ObjectTypeDeclaration`). Deferring the list to #211 does not cure a row that already asserts K. Fix: map each `value` submodule by name. Only the contents of the AD-016 row go to K. Admission and declaration content (`definition`, the `enumeration` identity, `unit` admission, `composite` declarations) goes to S3 `check`, `library` or `model`. Mark any submodule that is genuinely undecided as "K or S3, #211". | ADR-011 §6.2 `value` row, §7.3 X-1 · AD-016 Shared-type strategy · `src/value/definition.rs:2-8`, `enumeration.rs:2-6`, `unit.rs:2-10` |
| FND-006 | medium | The inside of `value::expression` is not mapped. §6.2 splits it only into check (S3) and evaluate (S6a). It also holds `syntax.rs`: the unchecked value-expression tree over source names, which is AD-016's arrow 1 input (`Expression`) and the only unchecked form today. It also holds `ir.rs` (the checked tree shared by check and evaluate) and `facts.rs` and `termination.rs` (FR-146 checking). Where the unchecked tree lands decides §4, because S2 `forms` is described as absent. Fix: map `syntax.rs` to S2 `forms`, which M-3 replaces or absorbs. Map `ir.rs` to the S3 output type (constructors private to `check`, §4). Map `facts.rs`, `termination.rs` and `refusal.rs` to S3 `check`. Add the `syntax.rs` move to M-3 or M-5. | ADR-011 §1 S2, §4, §6.2, §7.3 M-3, M-5 · `src/value/expression/syntax.rs:2-6`, `ir.rs:2-3` |
| FND-007 | medium | The `model` mapping and M-2 contradict the code. §6.1 and M-2 say that after X-1, `model` "depends on K and never on `value` or `check`". But `model::checked_dispatch` imports `value::expression::{PackageDeclarations, DispatchTable}` and prepares input for `PackageDeclarations::check` (ADR-010 OBS-007 names it as the only non-test producer of checked input). `model::accounting` holds `Meter`, a kernel type under AD-016. Neither moves with X-1 as stated. Fix: map `model::checked_dispatch` to S3 `check`, as the dispatch-family checker behind the #210 hook, and `model::accounting` to K. Add both to M-2. Check the other `model` → `value` imports (`conformance`, `population`, `domain_package`, `key`, `normalize`) against the K list from FND-005. | ADR-011 §6.1 rules, §6.2 `model` row, §7.3 M-2 · `src/model/checked_dispatch.rs:5-16,109` · AD-016 Shared-type strategy |
| FND-008 | medium | §4 misstates the `CheckedPackage` situation. §4 says "Two public types named `CheckedPackage`" and has #211 decide "the canonical name". AD-016 records four distinct `CheckedPackage` types, including RT `exact/expression.rs`, which AD-016 keeps in RT. Owner decision 6 defers renames of `CheckedPackage`, `DeclarationKey` and `CanonicalDigest` "until the owner asks". Fix: say that one type per stage output applies to QSL S3 and S4 outputs only. Name the other types (RT's, and IR's `CheckedPackageV2` at S5) as distinct by AD-016. Change the #211 handoff from choosing a canonical name to naming the S3/S4 types, subject to AD-016 Owner decision 6, which has renames wait for the owner. | ADR-011 §4, Questions to #211 · AD-016 Shared-type strategy ("the four `CheckedPackage` types"), Owner decision 6 |
| FND-009 | medium | §1 says "No stage after S3 re-resolves a name". The S6a entry that AD-016 fixes, `CheckedPackage::call(&self, function: &str, …)`, selects a function by a string name at S6a. E9 names no rule for how the replay adapter derives that string from the obligation or node id. As written, S6a and E9 resolve a name after S3, which §1 forbids, and FB-02's ban on string channels between stages does not reach it. Fix (AD-016's signature stands): state that the `&str` is an exact lookup of a declaration name that S3 already resolved and recorded in the S4 package, never re-resolution. State that E9 takes it from the obligation's checked identity, not from display text. If the name-to-identity rule is #211's (ADR-010 DA-18 Names), hand it there explicitly. | ADR-011 §1, §2.1 E6 and E9, FB-02 · AD-016 arrow 7 |
| FND-010 | medium | The migration order is partial, and M-2 has no owning change. The "Order" column mixes ordinals ("1st", "2nd"), a relative order ("after X-1") and ticket numbers. M-2 names no ticket. M-4 (#216) must land before M-6 ("before #216 passes"), because SEAM-1 retires only once the backend output uses v2, but this is not stated. M-3 and M-5 both sit in #214 with no order between them. #209 requires a migration order. Fix: replace the column with an owning change and a "requires" list for each move: X-1 ← none; M-1 ← #211 locus; M-2 ← X-1; M-3 ← #210 forms; M-5 ← M-2, M-3; M-4 ← M-5; M-6 ← M-4, M-3. Give M-2 an owning ticket. | ADR-011 §7.3 · #209 Acceptance bullet 5 |
| FND-011 | low | M-5 says "S3 `check` and S5 `evaluate`". S5 is Contract IR. Evaluation is stage S6a (layer 5). Decision 11 makes these local ids citable elsewhere, so the slip would spread. Fix: change it to "S6a `evaluate`", and keep "layer" and "stage" separate in the Direction column. | ADR-011 §7.3 M-5, Decision 11 |
| FND-012 | low | Some modules are split into halves the ADR does not name. "The native half of `command`" (SEAM-1) is not defined. Today `command::wire` imports `checking` and `runtime`, `command::extraction` imports `formal_source` and `native_model`, and `compilation` runs native link then check, so almost all of `command` is native. Current `package` is entirely `NativePackage`: `view.rs` imports the IR model and `intake` and `reading` implement native-linked-package/1. No S4 content exists yet (OBS-001). Fix: list the `command` submodules for SEAM-1 and for layer 6. Map current `package` wholly to SEAM-1, and describe S4 `package` as new (M-4). | ADR-011 §6.2 SEAM-1, `package` and `command` rows · `src/command/wire.rs:3-5`, `src/command/extraction.rs:3-5`, `src/package/view.rs:5` |
| FND-013 | low | Decision 8 understates §2.3. Decision 8 requires "at least one proposition over every module it claims". §2.3 also requires a mutation control inside each claimed module that turns the gate red. Fix: restate Decision 8 as "passes only under both §2.3 conditions for every claimed module". | ADR-011 Decision 8, §2.3 |
| FND-014 | low | The placement of the negotiate predicates is vague. §6.2 sends `value::division::negotiate_*` and `value::ieee::negotiate_*` "outside QSL". AD-016 places the pure predicates in RT (arrow 3; Shared-type row: "the `negotiate_*` predicates stay in RT") and negotiation in CG (arrow 4). Fix: name RT as the home of the predicates and CG as the only caller that negotiates. Keep the predicate list with #210. | ADR-011 §6.2, FB-12 · AD-016 arrow 3, arrow 4, Shared-type strategy |
| FND-015 | low | The gate cross-reference is imprecise. §10 says "#212 re-walks all seven", but #212's seven scenarios are not ADR-011's. #212 scenario 4 (change a model-bound identity while keeping provenance) and scenario 6 (a nested counterexample replayed natively) have no placement row. In addition, ADR scenario 6 places unbounded admission in `check` and `package`, while AD-016 scenario 4 names QSL `NativeModelProfile`, which lives in `native_model` and `linking` and retires with SEAM-1. The ADR does not note the difference. Fix: add placement rows for #212 scenarios 4 and 6, or map #212's list onto §10. In row 6, note that AD-016's `NativeModelProfile` touch point retires with SEAM-1 and that the WP11 re-walk will see `check`. | ADR-011 §10 · #212 Gate scenarios · AD-016 Change scenario 4 |
| FND-016 | low | The traceability links are incomplete. The implementing-tickets table omits tickets the body assigns work to: #219 (§2.3 proof-spine gates), #220, #221 and #223 (SEAM-2, SEAM-3, `simulation`), #29 and #133 (§5), and the external RT #53, IR #109 and IR #140. The frontmatter has no relationship to AD-016, which is the record's binding input. Fix: add the missing tickets to the table, and add a relationship to `ix://agent-ix/quire-specification/AD-016` with the type the repository uses for cross-repository inputs. | ADR-011 frontmatter, Context "Implementing tickets", §2.3, §5, §6.2 |

## Method

- **#209 coverage.**
  - Each Required-design clause was checked against the section that answers
    it: flow (§1), admitted types and owner (§2.1), preservation of five
    properties (§2.2), refusals and partial output (§2.3), bypasses (§3), CLI
    (§5), crate/module DAG and extraction criteria (§6, §7).
  - Change scenarios: the seven #209 scenarios match §10 rows 1 to 7 in order.
  - Acceptance: see the Summary table.
- **Id consistency.**
  - S0–S8 (with S6a/S6b), E1–E9, I1–I3, FB-01–FB-12, SEAM-1–SEAM-5, X-1 and
    M-1–M-6 have no gaps or duplicates.
  - The stage table, the edge table and the mermaid agree, except the missing
    S4 → S8 input (FND-001).
  - Every FB row's "Observed today" cites an ADR-010 OBS id that exists.
  - The 19 routed findings in Context equal the ADR-010 §9.2 #209 row and the
    §9 rows of ADR-011.
- **Module map.** Compared the §6.2 rows with `src/lib.rs:13-44` and the
  submodule directories at 944a1c8. Every top-level module and `cli`, `main`,
  `xtask` and `tools/fixture-audit` has a row. The submodule gaps are FND-005,
  FND-006, FND-007 and FND-012. `model::intake` is absent on this revision
  (PR #200). The ADR treats it as incoming, and that is consistent.
- **AD-016.** Read `origin/main:spec/assurance/AD-016-semantic-family-extension-path.md`.
  Checked: arrow owners, the arrow 1 "negotiates nothing" rule, the arrow 2 v2
  wire as the only QSL → IR seam, the arrow 7 executor signature, replay
  ownership, the Shared-type row, the crate graph and Owner decisions 2, 5
  and 6.
  - Contradictions found: FND-005 (the kernel "exactly" row) and FND-008
    (the four types and Owner decision 6).
  - FND-001 and FND-009 are internal contradictions exposed by the arrow 7
    signature.
  - Removing the IR root → QSL edge and choosing S4 `package` for the v2
    emitter do not contradict AD-016: it calls that edge "outside this
    pipeline" and leaves the emitter's module path `OPEN — WP6`, which is a
    QSL-internal placement.
- **Sibling boundaries.** Every type-name, typestate, outcome-vocabulary,
  locus, wire-retention and kernel-list question is deferred to #211. Every
  family hook, `capability_report` content and v2 family form is deferred to
  #210. Neither is decided in the body.
- **Compatibility.** All §7.3 rows and all seams remove the old path in the
  landing change. None of the fixes recommended here adds a shim, fallback or
  migration window.
- **Index row.** `spec/spec.md:388` links the correct file with the Proposed
  status, and `spec/spec.md:39` carries the `contains` relationship.
- **Not applied.** US → FR → StR traceability and the EARS and
  hidden-assumption CLI probes do not apply to a design ADR with no
  requirement statements.
