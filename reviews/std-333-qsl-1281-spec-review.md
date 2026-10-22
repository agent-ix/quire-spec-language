---
id: SR-16170
title: "spec-review of the closure and component certificate package_id and StaleDependency settlement (quire-spec-language PR 1281)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@8ad0ef48; spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md; spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md; spec/functional/FR-338-check-an-en-1-closure-certificate.md; spec/functional/FR-339-check-an-en-1-component-certificate.md; spec/test-cases/TC-1141-a-stale-package-identity-settles-stale-dependency-naming-both.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-282
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-339
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1141
    type: references
---

# Spec review of the closure and component certificate package_id (quire-spec-language PR 1281)

## Summary

Ticket: STD-333 (QSL half; paired with quire-specification PR 387, FR-331
and BP-32). One spec-review pass over `git diff origin/main...8ad0ef48`.
Oracles: the merged state-graph precedent (QSL PR 1246, STD-266: FR-169
`StateGraphCertificate.package_id`, its V-6 row and "Before any rule"
sentence, FR-282-AC-9, FR-169-AC-12), QSL FR-121 and FR-290, and QSpec
FR-331 at the #387 head 39b92375.

Gates: `quire validate --scope .` exits 0 on all five touched files.
`quire matrix --scope . --format tsv` at the head lists FR-338-AC-1 to
FR-338-AC-10, but not FR-338-AC-11 (FND-001). origin/main (48517b7b9) is an
ancestor of the head.

Examined, clean:

- Mirror of the precedent. `ClosureCertificate.package_id`, the
  `PackageMismatch{certificate, recompiled}` alternative of
  `TemporalCertificateCheck`, the "compare first, evaluate nothing else"
  behaviour and the settlement to `Inconclusive(StaleDependency{requested,
  recompiled})` with `requested` the certificate's `package_id` all match
  FR-169's shape.
- Mirror of QSpec PR 387. FR-331 says each certificate names its package's
  `package_id`, its checker compares it before any other rule, a stale
  certificate settles `stale-dependency` and never `certificate-rejected`,
  and a component certificate names its `package_id` once. FR-338, FR-339,
  FR-127 and FR-282 say the same.
- FR-127 now lists four `TemporalCertificateCheck` alternatives, and they
  agree with FR-338's enum. The new V-6 row and the Proof-certificates
  sentence both settle `PackageMismatch` as `StaleDependency`, never
  `CertificateRejected`.
- FR-282's binding sentence now covers the state-graph, closure and
  component checkers. The sentence that left their settlement unstated is
  gone, which settles SR-15870 FND-003.
- Each new AC can fail. FR-338-AC-11 and FR-339-AC-4 each pair the stale
  certificate with a rule defect (`InitialMissing`, `WitnessFails`), so a
  checker that compares late fails them. The edited bound `1001` and target
  `c.versionNumber = 1` give concrete second packages. FR-282-AC-10 and
  FR-127-AC-13 are testable. TC-1141 steps 5 and 6 cover all four ACs, and
  their expected results match them.
- Ids. FR-127-AC-13, FR-282-AC-10, FR-338-AC-11 and FR-339-AC-4 do not
  appear on main or in the other 24 open PRs. Old #723 adds only
  FR-127-AC-11, which this PR does not use.
- FR-121 (replay-only `StaleDependency` exception) and FR-290 (plugin
  `stale-dependency` refused `QslOnlyCause`) need no change.
- No status, changelog or "previously" wording. No compat layer.
- Lead defaults 1 and 2 (same shape as the state-graph certificate;
  `package_id` once, through the closure) are coherent and match QSpec 387.
  Lead default 3 (compare before clause-root resolution) is sound, but no AC
  tests it (FND-002). All three are labelled as lead defaults in the PR
  body.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | [major] FR-338-AC-11 is outside the AC table. The row is added after the blank line that ends the table at FR-338-AC-10. Without a header and a delimiter row it is a paragraph, not an AC. `quire matrix` at the head has no FR-338-AC-11 criterion, so nothing traces it. FR-127-AC-13, FR-282-AC-10 and TC-1141 step 5 all point at it. `quire validate` does not catch this. Fix: delete the blank line between FR-338-AC-10 and FR-338-AC-11, so the row joins the table. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:742-745 | correct-requirement-no-evidence |
| FND-002 | medium | [major] Lead default 3 (compare package_ids before resolving the clause root) has no AC. FR-338 Behavior says the checker compares `package_id`s "before it resolves the request's clause root". But FR-338-AC-11 uses AC-1's request, whose root resolves in both packages, and its rule defect is `InitialMissing`, which comes after root resolution in either order. A checker that resolves the root first and settles `Failed` passes every AC. Fix: add a vector, to FR-338-AC-11 or a new AC, with a stale certificate whose request clause root does not resolve in the recompiled package. It returns `PackageMismatch` and settles `StaleDependency`, not `clause-root-unresolved`/`Failed`. Add a matching step to TC-1141 step 5. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:679-681, :744 | correct-requirement-no-evidence |
| FND-003 | low | [minor] The clause-root refusal rule does not say it applies only when the `package_id`s agree. The bullet says "If the request's own clause root does not resolve ... the checker SHALL refuse the request without running the check ... FR-127 settles it `Failed`". FR-338-AC-8's last sentence is just as unconditional. Under the new first bullet, a stale certificate with an unresolved root settles `StaleDependency`. Read alone, those two sentences say `Failed`. Fix: add "when the certificate's `package_id` equals the recompiled one" (or "after the package comparison") to the bullet and to FR-338-AC-8's last sentence. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:701-705, :740 | correct-requirement-no-evidence |

## Verdict

CHANGES REQUESTED. FND-001 blocks merge. It is a one-line fix: delete the
blank line. FND-002 should land in this PR, because the ordering is this
PR's own lead default and nothing tests it. FND-003 is a wording fix to make
in the same round. The precedent mirror, the QSpec 387 mirror, the
four-outcome result, the settlement rows, the ids, the lead-default
labelling and the TC-1141 steps are clean. Merge after QSpec #387.

Context, not a finding of this PR: on main, FR-127's AC table has the same
blank-line split after FR-127-AC-6. So FR-127-AC-7 to FR-127-AC-13 (AC-13 is
this PR's) are missing from `quire matrix`. That defect predates this PR and
needs its own ticket. Separately, QSpec #387's FR-331-AC-36 calls result 1 a
"liveness proof", but its vector gives that result `kind`
`temporal-satisfaction`. The QSpec review should check that.
