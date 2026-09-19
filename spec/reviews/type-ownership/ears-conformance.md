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
