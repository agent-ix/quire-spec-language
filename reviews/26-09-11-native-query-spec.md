---
id: SR-345
title: "Base spec review of ordered query requirements in FR-040 and FR-042"
type: SpecReview
analysis: base
scope: "spec/functional/FR-040-check-composed-values.md; spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-119-check-composed-values.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md; docs/compiled-protocol-v1.md; README.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-040
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-119
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

QUOIN `/spec-review` base checklist over the requirement and test-case documents
the ordered-query increment changed, plus the two prose delivery claims
(`README.md`, `docs/compiled-protocol-v1.md`). No applicable `AssuranceProfile`
is installed in this spec scope — `spec/assurance/` does not exist and no document
declares `review_selection` — so the review set is the user-chosen base plus
failure-domain, matching the parent-scope finding. ID formats, link integrity and
the six coverage rules pass; the findings are wording precision, not structure.
Rechecked at `baf93f5`, which rewrites FR-040-AC-4/AC-5, TC-119 step 5 and the
wire contract's availability and scope-locus sentences.

## Verdict

**CONDITIONAL** (recheck) — both mediums and two of the three lows are resolved
at `baf93f5`; FND-005 (no direct non-flattening assertion for TC-119 step 5's
"preserved population roles") remains the only open item.

## Findings

| ID      | Severity | Summary                                                                             | Refs                                              | Escape Cause        |
| ------- | -------- | ------------------------------------------------------------------------------------ | ------------------------------------------------- | ------------------- |
| FND-001 | medium   | RESOLVED at `baf93f5` — FR-040-AC-5 says N=0 is "checked against authored maxima"; the adopted behaviour refuses it at the model boundary | spec/functional/FR-040-check-composed-values.md:143 | wrong-requirement   |
| FND-002 | medium   | RESOLVED at `baf93f5` — FR-040-AC-5 asserts N=10,000 for all eight forms without bounding proof cost; `sum` cannot reach it | spec/functional/FR-040-check-composed-values.md:143 | wrong-requirement   |
| FND-003 | low      | RESOLVED at `baf93f5` — the rational sum-transfer carve-out lives only in FR-040 narrative, not in AC-4 or AC-5 | spec/functional/FR-040-check-composed-values.md:163 | missing-requirement |
| FND-004 | low      | RESOLVED at `baf93f5` — the wire contract's binder-availability sentence does not name the value `scope` handle as the authority | docs/compiled-protocol-v1.md:533                  | wrong-requirement   |
| FND-005 | low      | TC-119 step 5 requires "preserved population roles" and no flattening; no assertion targets non-flattening directly | spec/test-cases/TC-119-check-composed-values.md:58 | correct-requirement-no-evidence |

### FND-001 — N=0 is refused, not checked

FR-040-AC-5 reads "N=0 and N=10,000 are checked against authored maxima; N=10,001
refuses." The delivered behaviour refuses a zero-maximum producer domain at model
admission with `DiagnosticCode::UnboundedCollection`, and the test asserts exactly
that. The distinct in-language case — an admitted max-positive collection that may
be empty, and an always-false filter proving an empty result — is covered by the
`Empty` predicate but is a different proposition from "N=0 is checked". The AC
should name the adopted boundary so a reader cannot take the current refusal for a
regression, and so the `Empty` case gets its own words.

### FND-002 — the maximum is stated without a cost bound

The AC treats N=10,000 as a uniform property of "all eight ordered query forms".
`size`, `contains`, `forall`, `exists`, `filter`, `map` and `count` are constant
in `N` in the delivered path; `sum` is linear, and its per-prefix goal count runs
into a ceiling that callers cannot raise. Either the AC scopes the 10,000 claim to
the forms whose proof cost is independent of capacity and states a separate,
smaller bound for `sum`, or it states that exceeding the bound is a resource
exhaustion outcome rather than a semantic verdict. As written, a reader checking
the AC alone would expect a discharged `sum` over a maximal collection.

### FND-003 — carve-out below AC level

FR-040's narrative now says broader rational sum-domain transfer "remains an
explicit unsupported prerequisite where the selected proof interface cannot
establish it. This does not remove rational sums from the requirement." That is
the right framing and it does not narrow the requirement. But neither AC-4
(rational arithmetic) nor AC-5 (query forms) carries the marker, so the acceptance
table alone reads as though denominator-2 sums are in scope and passing. AC-4
already has the pattern for this ("Missing producer/proof representation yields an
explicit unsupported prerequisite and leaves the corresponding full-feature
acceptance outstanding") — extend it to the sum domain.

### FND-004 — availability authority

`docs/compiled-protocol-v1.md` now states "a binder is available in its body, not
its collection". The emitted package enforces this through the per-value `scope`
handle and the scope parent chain; the binder's scope *locus* lexically spans the
collection, because the region starts at the binder token. Name the handle as the
authority in the same sentence, otherwise a consumer implementing availability
from locus containment builds a conforming reader that disagrees with the emitter.

### Recheck at `baf93f5`

The selected review set stayed base plus failure-domain; the optional semantic
gap extension remained declined. The four documents changed as follows.

- **FND-001 resolved.** AC-5 now says producer sequence maxima are 1..10,000 and
  that declared maxima 0 and 10,001 refuse *at model admission*, and separately
  that an admitted collection may be empty at runtime and that checking admits
  provably empty filter results. The refusal boundary and the `Empty` proposition
  are now distinct sentences, matching the delivered behaviour.
- **FND-002 resolved.** AC-5 now claims maximum 10,000 for each form "including
  sum" only "with sufficient effective proof limits", and states that
  caller-lowered exhaustion is distinct from semantic refusal. That matches the
  code twice over: `sum` proof cost is now constant in N, so default limits
  suffice, and `query_body_proof_exhaustion_keeps_original_types_and_a_fresh_retry`
  backs the exhaustion clause TC-119 step 5 now requires.
- **FND-003 resolved.** AC-4 carries the carve-out ("including unsupported
  rational sum-domain transfer"), so the acceptance table no longer reads as
  though denominator-2 sums are in scope and passing.
- **FND-004 resolved.** `docs/compiled-protocol-v1.md:246,539` names the admitted
  per-value `scope` handle and its parent chain as the availability authority and
  demotes the scope locus to source provenance, in both the reads paragraph and
  the query paragraph.
- **FND-005 open.** TC-119 step 5 still requires "preserved population roles" and
  no flattening. The new empty-filter emission test is indirect evidence — the
  operator histogram and the assertion that `size`/`count`/`sum` each read the
  `absent` binder rather than an inlined constant — but no assertion targets
  non-flattening as such.

The new AC-5 and TC-119 text adds no unmeasurable clause: each new sentence names
a scalar bound, a named boundary or a typed outcome. FR-040's narrative addition
(the k-prefix interval and monotone-endpoint argument) states the transfer
argument the implementation relies on, which previously existed only as a code
comment.

### Base checklist

- ID formats: FR-040-AC-1..10, FR-042-AC-1..10, TC-119, TC-121, TC-120 all
  conform to the declared patterns. No duplicates, no gaps in the AC sequences.
- Validation link integrity: every AC row names a verifying TC; TC-119 and TC-121
  both declare `verifies` relationships to their FRs.
- Six coverage rules: `quire coverage --scope <worktree>` shows 0 status lies and
  no unbacked row in FR-040/FR-042/TC-119/TC-121. The six unbacked rows are
  inherited FR-036/FR-017/NFR-005 items reported in SR-344.
- Measurability: the changed AC-5 text is measurable (named scalars, units,
  bounds, explicit N values), which is why FND-001 and FND-002 are findings at all
  — the numbers are precise enough to be checked against the code.
- Error modes: FR-040-AC-8 keeps unproved definedness, unsupported prerequisite
  and resource exhaustion as distinct typed causes, and the delivered
  `SumDomainTransfer` cause fits that vocabulary without a new failure category.
- `quire validate --scope <worktree> "spec/**/*.md"`: 398/398 grammar-clean, 0
  findings.

### Delivery claims

`README.md` and the wire contract now say ordered queries retain binder,
collection and body through definedness and native emission, that sum admission
checks every prefix in integer or denominator-one rational domains, and that
broader rational transfer is unsupported. Each claim is backed by a real test.
Both documents correctly continue to exclude query runtime execution, the
producer-to-B handoff, recovery admission and general decision proofs.
