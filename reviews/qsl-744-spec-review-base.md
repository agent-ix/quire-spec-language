---
id: SR-8722
title: "Spec review (base) of quire-spec-language PR #808 (QSL-744): FR-056 origin retention, FR-056-AC-33 and TC-1005"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@88cdadd279a4247945340f3bd8ae3b982c85e1cf; spec/functional/FR-056-admit-domain-package-model-declarations.md, spec/test-cases/TC-1005-malformed-declaration-retains-the-nodes-origin.md, spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
---
# Spec review (base) of quire-spec-language PR #808

## Summary

Ticket: QSL-744. Method: quoin spec-review, base checklist, over the spec hunks of PR #808. The PR adds an FR-056 paragraph on origin retention and node-less refusals, FR-056-AC-33, TC-1005 and its spec/tests.md row. It also widens the population-member sentence. The companion QSpec change (agent-ix/quire-specification#245, merged) amends TC-235 M10 run 10's path to `PopulationDefinition.members[0]`.

Examined:
- the FR-056 paragraph "Every `malformed-declaration` refusal on a node, at intake and at normalization alike, retains that node's FCD `origin` as FR-154 requires ... Two refusals carry no node by design ..." (examined)
- the widened population-member sentence in FR-056 (examined)
- FR-056-AC-33 (examined)
- TC-1005 and its spec/tests.md row (examined)
- FR-056-AC-32 (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The paragraph says origins are absent only for a `generated` origin or a package built without the reader, and that "two refusals carry no node by design". Intake has a third origin-less `malformed-declaration` family: refusals of the package document itself (malformed JSON, a number with no finite double, an RFC 8785 encoding failure, a view with no root). These carry node `$` and no artifact or span (intake.rs:422-450, 466, 959). `$` is not an IR node, so "on a node" arguably excludes them. But a reader of "two refusals carry no node" would take the list as complete. Fix: add one clause naming refusals of the package document itself (node `$`), which have no declaration origin. | spec/functional/FR-056-admit-domain-package-model-declarations.md:555-566 |

## Verdict

- **Uniqueness.** `FR-056-AC-33` and `TC-1005` appear on no other remote branch of the repo (every `refs/remotes/origin/*` was grepped), including the heads of all open PRs (#807, #802, #792, #789, #770 and the older ones).
- **FR-056-AC-33.** The AC is testable and typed. It lists each refusal site, names the operation-located case and the population path, and states the generated negative. It matches TC-1005's procedure and the `tc235_origin_retained.rs` vectors one to one. It is a list of vectors under one rule, not a compound of independent behaviours.
- **TC-1005.** Its frontmatter `verifies` FR-056. Its scope names AC-33. Its steps 2, 3 and 4 match the test functions. Its expected results require typed comparison, never detail text. The tests.md row is consistent with this.
- **Population sentence.** It now says any meaning other than an object type is `malformed-declaration` and an undeclared name stays `missing_declaration`. That agrees with FR-056-AC-32, the code, and the amended QSpec TC-235 M10 run 10.
- **Node-less statement.** The two node-less refusals (empty selection identity or version, and a key with an empty component) match the only two unit `MalformedDeclaration` constructors left in production code (normalize.rs:1089, 1992). The paragraph gives a reason for each.
- No hash, digest, pin or tracking record is added.

**PASS with one low finding.**

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | TC-1005 gains an expected result ("So do the refusals that name no declaration: an empty model selection, an empty declaration key and the package document at node `$`"), and FR-056-AC-33 gains the same clause. But TC-1005's procedure has no step that produces those refusals, and no test traced to TC-1005 or FR-056-AC-33 asserts them. The expected result cannot be observed by running the TC as written. The behaviour is true, and is shown elsewhere: the unit `MalformedDeclaration` has no origin fields, and intake's own tests assert `node: "$"` with `artifact: None`. Fix: add a TC-1005 step (or a traced vector) that raises an empty selection and a malformed document and checks that no artifact or span is present. Or keep the clause in the FR-056 paragraph as a statement and drop it from the AC and TC expected results. | spec/test-cases/TC-1005-malformed-declaration-retains-the-nodes-origin.md:26; spec/functional/FR-056-admit-domain-package-model-declarations.md:806 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 44f6c4298 |

Disposition pass 1 (reviewed 44f6c4298). The FR-056 paragraph now says the two node-less refusals carry no declaration origin. It adds that the refusals about the package document itself, at node `$`, likewise have no declaration origin to retain. FR-056-AC-33 and TC-1005 list the same three exceptions. FND-002 above is new and comes from that addition.
