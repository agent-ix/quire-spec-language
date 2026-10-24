---
id: SR-12892
title: "Failure-domain review of STD-175 zero-instance hyperproperty items (QSL PR #987)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@944699b543e42ba482bc3cd405281b27dcf10ba9; PR #987 diff against origin/main 49bcf9a: FR-125, FR-172, FR-182, TC-607; context ADR-023 HS-3"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-172
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: reviews
---
# SR-12892: Failure-domain review of STD-175 (QSL PR #987)

## Summary

Ticket: STD-175. This pass looks for edge cases in the new zero-instance
rule. The empty-universe case and the population-with-no-universe case are
well formed and can be built.

## Verdict

One medium finding. Its second case is borrowed from the temporal `over`
parameter, and it does not fit a hyper object parameter.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The "type no object of the universes conforms to" case comes from FR-125's `over` rule. There, instances are every object, in any population, whose type conforms to `T`. A hyper parameter is different: FR-172 S3 resolves `v: V::T` statically to the one population of `V`'s model whose element type is `T`, and its instances are that population's universe keys. So when such a population exists, "no conforming object" is just an empty universe, which is case 1. When none exists, S3 cannot resolve the parameter, and the clause never reaches FR-125. FR-172 does not say what S3 does then, and FR-182-AC-9 and TC-607 step 7 expect `NoInstance`. On the vault subject (one population, `vaults: Vault`), case 2 cannot be built as a runtime case. An implementer either duplicates case 1 or writes an S3 refusal that the AC calls wrong. Drop case 2 for hyper parameters, or define it as an S3 refusal at the parameter (as FR-172-AC-2 does for an unbound alias). Make the same fix in QSpec. | spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md:90-96; spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:113-115; spec/functional/FR-182-settle-a-hyper-verdict.md:131; spec/test-cases/TC-607-hyper-verdicts-settle.md:27 |

## Dispositions

Round 1, reviewed at 470eeeb35 (merge of origin/main 7915615f1).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
