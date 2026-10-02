---
id: SR-338
title: "Base requirements review of native population and reference exports"
type: SpecReview
analysis: base
scope: "spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md; spec/model-linking/tests.md; docs/compiled-protocol-v1.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Base checklist recheck at source commit `84aec59`. The specification half of the
correction adds six lines to FR-042's Behavior section, six lines to the
`compiled-protocol-v1` wire contract, and rewrites the TC-121 evidence paragraph
in the linking matrix. Two of the three initial findings are resolved; the third
is narrowed and retained, and two new low findings are recorded. The selected
review set is unchanged — base plus failure-domain (SR-339), with the optional
semantic gap extension declined. Re-confirmed at this commit: no
`AssuranceProfile` with a `review_selection` is installed anywhere in this spec
scope, so no profile-required set applies and this direct path remains correct.

## Verdict

**CONDITIONAL** — no high finding. IDs, cross-references and the six coverage
rules still hold, and the new normative text is bound to existing acceptance
criteria with its typed causes named. What remains is granularity: FR-042 still
has no error-conditions section of its own, TC-121's procedure was not extended
to name the now-normative surplus refusal, and one clause reuses "original" in a
sense the wire contract states more precisely.

## Disposition of the initial findings

- **FND-001 (low, new SHALLs not bound to an acceptance-criterion id) —
  resolved.** The paragraph now closes with "FR-042-AC-4 and FR-042-AC-7 cover
  these identity and reader-refusal obligations under the wire contract's typed
  cause catalog" (`:129-130`). A reader of the AC table can now follow the
  mapping without inferring it, and no id was added or renumbered, which matches
  the owner's directive against a new artifact campaign.
- **FND-003 (low, stale matrix narrative) — resolved.** Recorded in full as
  SR-337 FND-003. The paragraph no longer quotes a control count, names all three
  backing suites, states the reader-scope boundary, and holds the
  producer-to-B-consumer handoff and remaining full-family obligations open with
  every row still `🚧 Planned`.
- **FND-002 (low, refusals specified without typed causes) — narrowed;
  retained below.**

## Checklist results

- **ID format and uniqueness.** `FR-042`, `TC-121` and `FR-042-AC-1..AC-10`
  conform; the correction added, renumbered and duplicated no id. Correct: the
  new normative detail is an obligation of criteria that already exist.
- **Functional requirement quality.** Behaviour is more specific than before and
  still bounded by Inputs, which states that the reader "consumes no concrete
  workflow instances, observations, clocks, assessment requests or backend
  results". The added text keeps the two actors apart — derivation is the
  compiler's, refusal is the reader's — and the wire contract makes the reader's
  input explicit as the *decoded* binders and anchored values checked against
  admitted models. Neither document claims the reader validates source
  equivalence without parsing. "Error conditions documented (codes)" is still
  unmet as a section, though two causes are now named inline (FND-002).
- **Test coverage, six rules.** Coverage: every AC still names TC-121. Error
  path: two of the newly normative refusals — a surplus pair and a broken closure
  pairing — are not named in TC-121's procedure, which still enumerates five
  substitution axes (FND-004); the implementation now runs nine. Constraint
  boundary: no new numeric constraint; the traversal reuses the `Entries`/`Depth`
  dimensions already covered by FR-042-AC-9, and the wire contract now states the
  visit-once property those dimensions bound. Edge case: two roles sharing a
  universe label remains named explicitly in both FR-042 and TC-121.
- **Cross-referencing.** FR-042 → `docs/compiled-protocol-v1.md` and TC-121 →
  FR-042 links are intact; the added requirement text and the added wire-contract
  text agree, and terminology ("object role", "universe", "population export",
  "closure requirement", "requirement key") is consistent across all three
  documents. Cosmetic only, not a finding: the inserted sentences left
  `FR-042:122` and `docs/compiled-protocol-v1.md:117,120` unreflowed against the
  surrounding wrap width.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-002 | low | Narrowed, retained. The added paragraph now names `Invalid::Type` for a crossed triple and `Invalid::Binding` for a missing or surplus pair, so the two refusals this change introduced carry their codes. The inherited shape of the document is unchanged: FR-042 has Description, Inputs, Outputs, Behavior, Acceptance Criteria and Dependencies only, and the remaining causes the reviewed path can raise — `Invalid::Owner` for a substituted declaration subject, `Invalid::ForeignLocus` and `Invalid::Selection` for a tampered role locus, `Invalid::Duplicate` for a repeated pair — exist only in the `Invalid` enum, the wire contract's refusal list and the tests. The wire contract's own enumeration remains the catalog to point at | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:118; docs/compiled-protocol-v1.md:461; src/protocol_artifact/mod.rs:148 | missing-requirement |
| FND-004 | low | New, error-path rule. FR-042 now makes the surplus refusal normative, but TC-121's step 4 still reads "Substitute one reference, object, population export, model owner or source locus at a time; require the corresponding typed refusal" — a substitution-only matrix with no addition axis and no closure-pairing axis. The implementation and its suite already exercise both (nine axes against the five named), so this is a documentation lag rather than an untested behaviour, and closing it is a sentence in the existing control block, not a new id or a renumbering | spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md:58; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md:62; tests/native_population_emission.rs:328 | correct-requirement-no-evidence |
| FND-005 | low | New, wording precision. "The population/closure set SHALL equal the set required by the declaration's original input binders and anchored values" uses "original" in the non-derived sense — the very next sentence contrasts it with derived binders — but it sits four lines below "original evaluation anchor" and describes an obligation the reader enforces. Read quickly it can be taken as source equivalence, which the reader cannot check because it does not parse. The wire contract already has the unambiguous phrasing ("the parser-free reader checks the exact set justified by the decoded input binders and anchored values against admitted models"); mirroring "non-derived" or "decoded" into FR-042 removes the reading | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:125; docs/compiled-protocol-v1.md:114 | wrong-requirement |
