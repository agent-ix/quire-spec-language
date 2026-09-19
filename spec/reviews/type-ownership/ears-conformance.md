---
id: SR-489
title: "EARS conformance review of ADR-013 canonical type, package and conversion ownership"
type: SpecReview
analysis: ears-conformance
scope: "spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
# SR-489: EARS conformance review of ADR-013

## Summary

Reviewed commit `660aa25` (branch `task/211-type-ownership`):
`spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md` and its
index row in `spec/spec.md` (line 398, plus the `contains` edge at line 167).
The index row matches the ADR Status ("Proposed", #211).

ADR-013 is a design decision record, not an FR, NFR or StR. It has no `shall`
statement, so the EARS pattern checks (event, state, unwanted-behavior
triggers, `non-canonical-trigger`, `unclassifiable`, `missing-subject` on a
`shall`) are **not applicable**. They are not counted as passed. What does
apply is the requirement-quality part of the lens: the ten ownership rules
R-01..R-10 (§1) and the per-object rule and invariant rows in §3 to §5. #213
and #231 will derive FRs from these rules, so each rule must be unambiguous,
singular, testable and free of weak words.

`quire validate --strict --summary` on the ADR reports 1/1 docs grammar-clean
and 0 grammar findings. Of the ten rules, R-02, R-08 and R-10 are clean. R-01,
R-04 and R-07 are not testable as written against the ADR's own §3 and §4 (all
medium). R-03, R-05, R-06 and R-09 carry low-severity wording defects. The
O-16 outcome category map has an unassigned target set that makes
"category-preserving" untestable (medium). Three low findings concern the
program's current-state rule: the §5 "compatibility" heading, the O-13
"fallback" paragraph and the transitional "until" in §6. No finding changes an
owner decision, so none is high.

Verdict: ACCEPT WITH FINDINGS

## Method

1. Ran the deterministic engine check:
   `quire validate --scope /home/peter/dev/worktrees/qsl-arch12 spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md --strict --summary`
   (quire 0.32.0, engine 0.46.0). Result: grammar-clean.
2. Decided the scope. EARS pattern rules apply only to `shall` requirement
   statements; ADR-013 has none, so those rules are recorded as not applicable.
   The applicable checks are: singular statement, named subject, concrete and
   verifiable response, no weak words, no internal contradiction with the rest
   of the record, and current-state wording.
3. Scanned the body for modal verbs (`shall`, `must`, `should`, `may`), the
   vague-response lexicon (`support`, `handle`, `manage`, `process`, `provide`,
   `enable`), and history, migration and compatibility wording (`until`,
   `fallback`, `compatibility`, `adapter`, `bridge`, `legacy`, `was`).
4. Checked each rule R-01..R-10 against the §2 kinds, the §3 object tables and
   the §4 conversion table, to see whether a test derived from the rule would
   pass or fail on the record's own decisions.

Applicability per rule:

| Rule | Singular | Subject named | Testable | Weak words | Result |
| --- | --- | --- | --- | --- | --- |
| R-01 | yes | yes | no, §3 contradicts it | none | FND-001, FND-002 |
| R-02 | yes | yes | yes | none | clean |
| R-03 | yes | yes | partly | "test" unnamed | FND-008 |
| R-04 | yes | yes | no, §3 contradicts it | none | FND-003 |
| R-05 | yes | yes | partly | "incidental" | FND-006 |
| R-06 | yes | yes | partly | "name" undefined | FND-007 |
| R-07 | yes | yes | no domain stated | "admitted" | FND-004 |
| R-08 | yes | yes | yes | none | clean |
| R-09 | yes | yes | partly | "new" | FND-009 |
| R-10 | yes | yes | yes | none | clean |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | R-01 says every §3 object has exactly one owner: "one repository, one stage and one public type". About ten §3 objects state two stages, two repositories or two types. O-07 names the source and check stages. O-09 names QSL for the clause and CG for the obligation. O-11 names parsed-forms and check. O-12 names source and check. O-14 names QSL check for package types and `quire-exact` for the evaluation shape. O-16 has three families. O-17 gives "typed causes: each layer and stage". O-21 has four kinds. O-25 has two witnesses. O-26 names CG for the request and QSL for the executor-side type. An FR that derives "one owner per object" from R-01 fails on the record itself. Fix: either split each multi-owner row into one object per concept (for example O-09a clause, O-09b obligation), or restate R-01 as "Every concept in §3 has exactly one canonical owner per layer, and each owner names its repository, stage and public type." | ADR-013 R-01, §3 O-07, O-09, O-11, O-12, O-14, O-16, O-17, O-21, O-25, O-26 |
| FND-002 | low | R-01 says a second representation "exists only as a layer-owned type … reached by a named, total, tested conversion". The §6 lane-private types are second representations with no conversion (R-09). As written, R-01 forbids them and R-09 permits them. Fix: add to R-01 "except a lane-private representation under R-09". | ADR-013 R-01, R-09, §6 |
| FND-003 | medium | R-04 says identity equality is one of the four §2 kinds and each §3 object names its kind. Several rows do not. O-25 (separating witness) and O-27 use "value, componentwise", which is not a §2 kind. O-10 uses "enum identity inside each layer". O-16, O-19, O-21 and O-23 have no equality row. O-15 says "Not an identity" and names no kind. A derived FR that checks every object against the closed kind set fails. Fix: map each of these rows to one §2 kind (for example "declared, componentwise over the FR-351 key" for O-25 and O-27), or add an explicit "no identity; R-04 does not apply" row for O-15, O-16, O-19, O-21 and O-23. | ADR-013 R-04, §2, O-10, O-15, O-16, O-19, O-21, O-23, O-25, O-27 |
| FND-004 | medium | R-07 requires every conversion to be "total over its admitted input" and to refuse "everything else". §4 states a loss and provenance rule for each conversion, but it does not state the admitted input domain for most of them (C-06, C-08, C-12, C-14 and C-15 have none). Without that domain, "total" cannot be tested and "everything else" is empty, so any conversion passes. Fix: add an "Admitted input" column to §4 that names the domain of each C-NN (for example C-04 "bytes whose `contract` is `quire.checked-package/v2`"). Leave the domain as the full source type only where the conversion is a total `From`. | ADR-013 R-07, §4 C-01..C-16 |
| FND-005 | medium | O-16 and O-24 require the `KaniOutcomeKind` → FR-331 map to be "total and category-preserving". The category table assigns an FR-331 category only to `proved`, `refuted` and `incomplete`. The refusal and internal-failure rows say "per the IR map", which is `OPEN — decided in WP9`. `tested`, `inconclusive` and `failed` have no category, and `Inconclusive` sits under internal failure while FR-331 has its own `inconclusive` result. A #231 test cannot check category preservation for 5 of the 10 kinds. Fix: assign each of the six FR-331 result values to exactly one of the six categories in the O-16 table. Then state that the WP9 map chooses only among the kinds that share a category. | ADR-013 O-16, O-24, C-09 |
| FND-006 | low | R-05 forbids identity from "the index of an item in an incidental collection". "Incidental" is not defined. The rule's second sentence fixes the exception ("only where a declaration defines it"), so the first sentence adds nothing and adds a weak word. Fix: drop "incidental" and state it positively: "A position is identity only where a declaration defines it (a tuple position, a parameter position); every other collection index is not identity." O-25's "position in an incidental collection" gets the same change. | ADR-013 R-05, O-25 |
| FND-007 | low | R-06 says "After the check stage no consumer resolves a name". "Name" is not defined. Later stages look up v2 wire strings such as operation identities (`quire.op.claim.clause`), catalog codes and `node_tag` values. A reader cannot tell whether those lookups are name resolution. O-11 limits the term to qualified names. Fix: "After the check stage no consumer resolves a qualified name (O-11). Catalog identifiers and v2 wire strings are lexical keys (§2), not names." | ADR-013 R-06, O-11, O-10 |
| FND-008 | low | R-01 and R-03 require a "tested" conversion whose test §4 names. Some §4 Test cells name a kind of test, not a test: C-06 "RT mapping test", C-08 "Adverse category tests (#213)", C-12 "#231 round trip". A derived FR cannot trace to them. Fix: name the owning ticket and the planned test id or vector file for each cell, or say in the §4 lead-in that the Test column names the evidence class and the implementing ticket supplies the id. | ADR-013 R-01, R-03, §4 C-06, C-08, C-12 |
| FND-009 | low | R-09 says "no new family, stage or boundary consumes" a lane-private type. "New" has no baseline, so a test cannot tell a new consumer from an existing one. Fix: anchor it to the recorded baseline, for example "No consumer outside the native-v1 and composed lanes recorded in ADR-010 §4 consumes it." | ADR-013 R-09, §6 |
| FND-010 | low | Current-state and no-compatibility program rule. The §5 heading is "Version and compatibility policy". §5 bullet 2 reads "Where an older artifact must be read, it is regenerated", which conflicts with the preceding "No component reads an older artifact version" and is the only body `must` outside Consequences. Fix: retitle §5 "Version policy". Rephrase bullet 2 as "An artifact at another version is regenerated from source at the current version." | ADR-013 §5 |
| FND-011 | low | Current-state and no-fallback program rule. The O-13 paragraph that starts "The AD-016 fallback rule … applies only if the owner withdraws Owner decision 2. It is inactive while that decision stands" records a conditional fallback. Fix: state only the decision: "OBS-005 is decided: `quire-exact` is the canonical owner (AD-016 Owner decision 2). Its crate creation and dependency direction are decided in #209; its types are built in #213." Change the §9 OBS-005 cell "fallback inactive" to match. | ADR-013 O-13, §9 OBS-005 |
| FND-012 | low | Transitional wording. §6 Digests row: "`state::input::CanonicalDigest` (until #213 folds it)". Other §6 rows state the lane-private status without a time clause, and O-18 already assigns the fold to #213. Fix: drop "(until #213 folds it)". | ADR-013 §6, O-18 |

## Round 2 (commit 0042691)

I re-checked ADR-013 at `0042691`. The EARS pattern checks still do not
apply (the record has no `shall` statement). R-01, R-04, R-05, R-06, R-07,
R-09 and §5 were reworded. `quire validate --strict --summary` on the ADR is
still grammar-clean.

| Finding | Status | How |
| --- | --- | --- |
| FND-001 | resolved differently, low residual | R-01 now reads "one canonical owner per layer" and joins two-layer objects by a named §4 conversion. O-07 names only the check stage, and O-09 splits the clause and obligation owners. Residual: R-01 still requires "one stage" per layer, and O-11, O-12 and O-14 each name two QSL stages or types (FND-013). |
| FND-002 | resolved | R-01 puts lane-private types outside the rule and points to R-09. |
| FND-003 | resolved | R-04 allows "states that it is not an identity". Every O-01..O-27 now names a §2 kind or says it is not an identity: O-10 lexical, O-15, O-16 and O-21 not an identity, O-19 and O-23 lexical, O-25 declared with semantic comparison of the deciding value, O-27 normalized. |
| FND-004 | resolved differently, low residual | R-07 defines admitted input as "the values that pass the source contract's reader and schema", so no §4 column was added. Residual: conversions whose source is an in-memory type have no reader, and C-22 gives an ambiguous "or" (FND-014). |
| FND-005 | resolved | O-16 has eight categories, and "every source value has exactly one row". All ten `KaniOutcomeKind`s and all six FR-331 result values are placed: `tested` under success, `inconclusive` in its own row, `failed` under internal failure. OQ-3 (d) sends the proof column to QSpec. The success and violation rows overlap on the evaluation side (FND-015). |
| FND-006 | resolved | R-05 defines the forbidden index as one "in a collection whose order no declaration defines", with examples. O-25 now says "never by collection position". |
| FND-007 | resolved differently, low residual | R-06 defines a name as "an identifier or qualified name in source or on a wire", and handles a library export resolved in an importing package's check stage. Residual: "on a wire" can cover catalog and operation identifiers (FND-016). |
| FND-008 | resolved differently | Each §4 Test cell now names the evidence and who supplies it: C-06 is an RT test over all six kinds (no ticket, §7), C-08 is one adverse test per O-16 row in #213 S-1, and C-12 is a CG #50 contract test plus the #231 round trip. The lead-in says what the column holds. It gives no test ids, which is acceptable for a record that comes before the tickets. |
| FND-009 | resolved differently | R-09 uses the acceptance of this record as the baseline ("no consumer added after this record is accepted uses it"), and the #226 drift gate enforces it. |
| FND-010 | resolved | §5 is titled "Version policy". Bullet 2 reads "An older artifact is regenerated from source at the current version." |
| FND-011 | resolved | The O-13 fallback paragraph is gone. O-13 and the §9 OBS-005 row state only that `quire-exact` is the canonical owner, with crate direction in Q209-4. |
| FND-012 | resolved | The §6 Digests row is removed. §6 states that `CanonicalDigest` and `ByteDigest` are duplicates that #213 S-2 folds into O-18. |

New findings introduced by, or left exposed by, the revision:

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-013 | low | R-01 requires one owner per layer, meaning "one repository, one stage and one public type". Three QSL-layer objects still name two stages or two types in one layer. O-11: parsed forms produce, check resolves. O-12: source and check stages; `LocatedSpan` and the node-keyed source map. O-14: package type node in the check stage and kernel `ValueType` in `quire-exact`, with no §4 conversion joining them. Fix: name the owning stage and type for each object (for example O-12 is owned by the check stage through the node-keyed source map, with `LocatedSpan` a source-stage input joined by C-21). For O-14, split the kernel evaluation shape into its own row or add the checked type node → kernel `ValueType` conversion to §4. | ADR-013 R-01, O-11, O-12, O-14 |
| FND-014 | low | R-07 defines admitted input only for sources that have a contract reader and schema. C-02, C-08, C-14, C-18 to C-22 and C-25 convert in-memory types that have no reader. C-22 and O-13 say an out-of-domain value gives "`requires-bound` or refusal", and `requires-bound` is a disposition, not an R-07 refusal. Fix: add to R-07 "for an in-memory source, the admitted input is every value of the source type". Then state which out-of-domain case in C-22 yields `requires-bound` (an unbounded declared domain) and which one refuses (a value outside a finite declared domain). | ADR-013 R-07, C-22, O-13 |
| FND-015 | low | O-16 says "Every source value has exactly one row", but `Completed(false)` of a claim matches both the success row (`Completed(value)`) and the violation row. Fix: success row "`Completed(value)` except `Completed(false)` of a claim". | ADR-013 O-16 |
| FND-016 | low | R-06 defines a name as "an identifier or qualified name in source or on a wire". Operation identities (`quire.op.claim.clause`), `node_tag` strings and catalog codes are identifiers on a wire, and IR and CG look them up after the check stage. Read literally, R-06 forbids those lookups and the #226 static check would flag them. Fix: add "Catalog identifiers, operation identities, node tags and codes are lexical keys (§2), not names." | ADR-013 R-06, O-10, O-17 |

Round-2 verdict: ACCEPT WITH FINDINGS. All twelve round-1 findings are
resolved, seven as proposed and five by a different fix; the residuals of FND-001, FND-004 and FND-007 are carried as FND-013, FND-014 and FND-016. The four new findings
are low, and none blocks.
