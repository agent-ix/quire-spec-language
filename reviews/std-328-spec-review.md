---
id: SR-16203
title: "Spec review of quire-spec-language PR #1297: STD-328, ADR-034 Timestamp payload, collection spelling and one evaluator"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@b31171a2f3d478d9ebe04b413db9820439264cca; PR #1297 (git diff origin/main...b31171a2f): spec/decisions/ADR-034-timestamp-payload-collection-spelling-and-one-evaluator.md; read for context: QSL ADR-031, ADR-032, ADR-033 (house style), FR-056, FR-070, FR-106 (as merged by #1255), FR-402, FR-405, AD-001; agent-ix/quire-specification main b3f3c803b FR-144, FR-149, FR-181"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-034
    type: reviews
---
# SR-16203: Spec review of quire-spec-language PR #1297: STD-328

## Summary

Ticket: STD-328. Sub-analyses folded in: integrity, scope-boundary, cross-repo fidelity. `make check-spec-validation` exits 0 at the head; main 5ba013c75 is an ancestor of the head. ADR-034 is the next free ADR id (main ends at ADR-033) and is unique across all 21 open PR heads.

Style: the frontmatter and the Context / Decision / Consequences / Alternatives Considered / References sections match ADR-031 and ADR-032. The text is a decision record. It has no tracker, status, owner-answer or changelog wording. The note that the decisions are lead defaults and not owner answers is in the PR body only, which is the right place. It does not belong in the ADR text, and it is not there.

Fidelity:

- **TS-1 and TS-2.** QSL FR-056 (via FR-070's citation) and FR-106 spell `{"kind":"timestamp","nanoseconds":...}` with the grammar "`0`, or an optional `-` then a nonzero digit and more digits". QSpec FR-181 spells `{"type":"timestamp","nanoseconds":"<decimal>"}` with no zone or calendar text. QSpec FR-149 compares the K2 payload. All match.
- **CS-1 to CS-3.** These match FR-106 as merged by #1255: the native single-tag spelling, FR-144's supplied-collection rules, the bound checked before any element is read, and an element with no FR-181 form refused. They also match QSpec FR-144, where FR-181's typed form is the runtime-input wire with "no second grammar".
- **EV-1 and EV-2.** FR-405 already says the std reference evaluator is deleted and "not kept behind a feature, as a fallback or as a test oracle", and its items 3 and 4 rule out a `cfg`, a setting, a dev-dependency and a test-support crate. The corpus regression harness takes over after the switch. FR-402's "After the switch" section limits differential testing to before it.
- **Other merged PRs.** No conflict with #1289 (FR-315). The ADR also agrees with open #1296's Decimal object: a payload object that carries FR-181's decimal members.

## Verdict

**NOT MERGEABLE as is.** One medium wording defect needs a fix: CS-2 states the duplicate rule for every collection kind, but bags group duplicates (FND-001). FND-002 is low. Everything else matches both mains.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | CS-2 says admission applies FR-144's rules so that "duplicates are refused and never removed", with no restriction by collection kind. For a bag, FR-144 and FR-106 group equal elements into one member whose multiplicity is their count, and refuse nothing. The decision text is wrong for bags. Say "in a set or ordered set, duplicates are refused and never removed; a bag groups equal elements into one member with their multiplicity". | spec/decisions/ADR-034-timestamp-payload-collection-spelling-and-one-evaluator.md:61 |
| FND-002 | low | CS-1 says "A FR-106 document never contains a QSpec FR-181 typed form `{"type": ..., ...}`". FR-106's record value is `{"record": {"type": "<declaration identity>", "fields": ...}}`, which contains an object with a `type` member, so the sentence can be read as excluding records. Name the excluded form by its FR-181 tag: an object whose `type` is an FR-181 tag, such as `{"type": "set", "elements": [...]}`. Also "A FR-106" should read "An FR-106". | spec/decisions/ADR-034-timestamp-payload-collection-spelling-and-one-evaluator.md:60 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 299278250: CS-2 reads "in a set or ordered set a duplicate is refused and never removed, while a bag groups equal elements into one member whose multiplicity is their count", matching FR-106 and QSpec FR-144. |
| FND-002 | fixed | 299278250: CS-1 reads "An FR-106 document never contains a QSpec FR-181 typed form, an object whose `type` member is an FR-181 tag such as `{"type": "set", "elements": [...]}`", and adds that FR-106's record payload `type` names a declaration, not an FR-181 tag. The "An FR-106" article is fixed. |
