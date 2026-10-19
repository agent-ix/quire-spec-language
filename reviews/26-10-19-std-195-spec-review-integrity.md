---
id: SR-12927
title: "Integrity review of STD-195 FR-125 instancing (PR #1013)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@9a8a05e9d; spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md, spec/test-cases/TC-520-model-behaviours-read-as-temporal-traces.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
---
# SR-12927: Integrity review of STD-195 FR-125 instancing (PR #1013)

## Summary

Ticket: STD-195. PR quire-spec-language#1013 at head 9a8a05e9d. Checked FR-125's internal consistency after the PR: whether the new instancing rules agree with the unchanged instancing, `RequiresBound` and `NoInstance` bullets, the relationship and Dependencies sections, and the Remaining-work citations.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new key rule gates instancing on a population covering `T` (FR-104: `T` or a supertype of `T` is a member type). The unchanged `RequiresBound` bullet still triggers on "a population whose member type conforms to `T`", which is the subtype direction, and `NoInstance` still triggers on "no object conforming to `T`". The first instancing bullet (lines 104-110) still states "one instance per object ... whose type conforms". Take a population of member `Dog` and a claim over `Animal`. With no universe it is `RequiresBound`. Given a universe it yields no instance, because it does not cover `Animal`. The three gates use two different relations, and the PR does not say which one governs. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:104-110,121-124,136-143 |
| FND-002 | low | The frontmatter adds a second `FR-120` `depends_on` edge (lines 20 and 26), which duplicates the existing one. The new `FR-104` dependency is not added to the `## Dependencies` section, which lists FR-120, FR-106, FR-107, FR-101, FR-123, FR-124 and FR-165. Remove the duplicate edge and list FR-104 under Dependencies. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:20,26,228-239 |
| FND-003 | low | Remaining work justifies the QSL-proposed marking with "FR-331 supplies only keys". QSpec FR-331 (backend provider envelope) lists `universes` inside the subject and does not give their shape. The key-list shape comes from QSL FR-120 (`keys: Vec<String>`) and QSpec's hyper fixtures (TC-337 "universe keys"). Cite the source that actually states it, so STD-196's premise can be checked. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:219-223; quire-specification/spec/objects/interfaces/FR-331-backend-provider-envelope.md:59 |
| FND-004 | low | "FR-169 (state-graph claims) and FR-182 (hyper clauses) instance under the same rules" conflicts with the hyper bullet. It instances each object parameter over "the one population each object parameter `v: V::T` resolves to", not over every covering population, and orders instances by the product. AC-10's multi-population order does not apply to it as written. Limit the sentence to FR-169, or state which of the new rules (key instancing, binding type, absent-key) the hyper rule takes. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:144-149,224-226 |

## Verdict

FND-001 must be fixed together with SR-12928 FND-001, because both come from the direction of 'covers'. Clean: the order bullet's new sentence (canonical order governs `ModelLetters::bindings`; ascending reference order does not apply) agrees with the existing order rule and with AC-10. `quire validate` passes on FR-125 and TC-520 (quire 0.36.2, engine 0.50.2).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The supertype-only exclusion is a deviation from QSpec FR-361, but the Behavior text presents it as a realization. The first instancing bullet quotes FR-361 ("one instance per object ... whose type conforms") and says "The next bullets state how QSL realizes this". The 'QSpec model-subject rules' bullet still says QSL reads the `over` binding "as QSpec FR-361 states them". Only the AC and Remaining work carry the QSL-proposed marking. Remaining work and the STD-196 comment frame "whether a population of proper supertypes of `T` holds objects conforming to `T`" as open, but QSpec FR-153 already answers yes. Mark the exclusion as a QSL-proposed deviation in the Behavior bullet itself. State in Remaining work and STD-196 that QSpec admits such objects and QSL knowingly does not instance them, pending a ruling. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:82-85,103-110,222-232; quire-specification/spec/functional/type-model/FR-153-query-closed-model-environments.md:89-91 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 01f5d081c: `RequiresBound` and `NoInstance` now gate on 'a population that holds `T`', the same relation instancing uses; the first bullet points to the following bullets. |
| FND-002 | fixed | 01f5d081c: duplicate FR-120 edge removed; FR-104 listed under Dependencies. |
| FND-003 | fixed | 01f5d081c: Remaining work cites QSL FR-120 for the key-list shape and says QSpec FR-331 names `universes` without a shape. |
| FND-004 | fixed | 01f5d081c: FR-169 instances under these rules; FR-182 instances by the hyper bullet and takes only the reference reading and the absent-key rule. |
