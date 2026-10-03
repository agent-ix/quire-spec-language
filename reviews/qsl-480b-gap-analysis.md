---
id: SR-1257
title: "QSL-480 gap analysis of quire-spec-language PR #613 against FR-259, FR-260, FR-261, FR-056 and FR-106"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@f407facfd2235286412b7c3a6ea7a1706e7fe750; PR #613 diff against merge base 652ae3d51; contract: spec/functional/FR-259-encode-identities-and-read-json-through-quire-canonical-at-any-depth.md, spec/functional/FR-260-admit-semantic-ir-documents-at-any-depth.md, spec/functional/FR-261-read-other-untrusted-json-at-any-depth.md, spec/functional/FR-056-admit-domain-package-model-declarations.md, spec/functional/FR-106-admit-snapshots-and-invocations.md, spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, spec/decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md (D-4.4, D-4.5, D-4.6, D-4.10, slices), spec/test-cases/TC-729, TC-730, TC-733"
review_set: subset
---
# QSL-480 gap analysis of quire-spec-language PR #613

## Summary

Ticket: QSL-480 (QC1-adopt). The PR changes no spec file. Two team-leader
rulings apply. First, observation keeps FR-106 check 1.3's raw digest, and the
spec must be fixed to match in this PR. Second, every refusal uses catalogued
codes and causes. The code side of the second ruling is SR-1256 FND-001 to
FND-003.

Requirement to code and test trace:

- **FR-259 B1 (fixed-depth serde path).** Covered for every QSL-owned
  preimage in the diff. The node-key body uses the event API until slice 1
  stratifies `SemanticTerm`. The PR declares this (question 3), and ADR-030
  slice 1 owns it.
- **FR-259 B2 (input-depth digests through the event API or tree encoder).**
  - Intake: the digest encodes the reader's tree (intake.rs:327).
  - Observation: the same (observation.rs:550).
  - Simulation: `Key` and `TransitionId` are bound on `Encode`.
  - The 100,000-deep cases (FR-259-AC-1, AC-3, TC-728/729) are declared
    not covered.
- **FR-259 B3 (identity byte limit, default 16777216).** Not implemented and
  not declared (FND-004).
- **FR-259 B4 (byte error is the stage's input-bytes outcome).** Intake maps
  `ReadError::Limit` and `Error::Limit` to `input_bytes`. Observation does
  not (SR-1256 FND-004, latent).
- **FR-259 B5 / FR-260 B4 / FR-261 B2 (malformed input with offset).**
  - Intake's `PackageDocument::parse` returns the offset refusal.
    `tc_730_a_lone_low_surrogate_refuses_at_its_byte_offset` covers
    FR-260-AC-2 at that function, with an independent oracle. FR-154
    admission discards that refusal (FND-002).
  - Package identity returns `PreimageDefect::Malformed { offset }`. l08
    checks it: `{` gives `Malformed { offset: 1 }`. FR-261-AC-1 calls this
    "the malformed-input defect", and it reports through the existing
    `invalid_package`/invalid-value, so the name is catalogued. FR-261-AC-1's
    100,000-deep half is declared not covered, so no TC-733 trace is added.
  - Observation digests malformed bytes raw, as the ruling says. The spec
    still asks for a refusal (FND-001).
- **FR-260 B2 (arena tree as `PackageDocument::tree`, handed to
  semantic-IR).** Not done: semantic-IR still takes its own `Json`, which is
  derived from the reader's tree. It waits on O-4 (ADR-030 slice 3), and
  FR-260-AC-1 is declared not covered.
- **FR-260 B5 / AC-4.** Declared as waiting on FR-255.
- **Behaviour changes.** The intake duplicate-name refusal is declared. The
  matching observation change is not, and FR-056/FR-106 do not describe
  either one (FND-003).

What the PR leaves clean: the three IR-wait sites are marked plainly, with no
shim committed. No depth cap was added. The intake `too_deep` pre-scan is
semantic-IR's own `MAX_DEPTH` and still matters until O-4: it stops
semantic-IR's recursive checks and the two derived views from overflowing.
No compat layer, no ceremony, no literal `DEPTH`, and no
`into`/`with`/`remote` to get around the derive.

## Verdict

Changes requested: three medium and one low finding. All four are spec edits
in this PR. FND-002 also needs a team-leader decision on which way FR-056 and
FR-260 should agree. Apart from these and the IR wait, the PR's code traces to
the requirements it claims.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Under the team-leader ruling, observation digests evidence bytes as given (FR-106 check 1.3, raw digest), and the code does that (observation.rs:554). The spec still demands the opposite in four places. FR-259-AC-3's last sentence and TC-729 step 3 want observation digest admission to refuse `"\ud800"` and `1e400` with a malformed-input refusal carrying the offset. FR-259 B5 tells every calling site to report the reader's malformed refusal. FR-261 lists observation digest admission as a site, and its B2 says each site returns its malformed-input refusal. Fix in this PR: delete the observation sentence from FR-259-AC-3. Retarget TC-729 step 3 at the sites that parse, or delete it, since FR-260-AC-2 (TC-730) and FR-261-AC-1 (TC-733) already cover intake's parse and package identity. In FR-259 B5 and FR-261 B2, exempt observation digest admission: it digests bytes the reader refuses raw, per FR-106 check 1.3. Otherwise the conflict just moves to FR-261. | spec/functional/FR-259-encode-identities-and-read-json-through-quire-canonical-at-any-depth.md:73; spec/functional/FR-259-encode-identities-and-read-json-through-quire-canonical-at-any-depth.md:62-65; spec/functional/FR-261-read-other-untrusted-json-at-any-depth.md:30-45; spec/test-cases/TC-729-qsl-reads-and-encodes-deep-json-through-quire-canonical.md:26-36 |
| FND-002 | medium | FR-260 B4 and AC-2 say intake refuses a document the reader rejects as `invalid_model_binding`/`malformed-declaration` at `$` with the offset. FR-056 (lines 92-97) says that at admission such bytes are digested raw and refuse `content-mismatch`, or `wrong-model-selection` under their raw digest. The code follows FR-056 at every production seam. `admit` (intake.rs:560-568) and `package_input` (intake/unit.rs:87-88) discard `PackageDocument::parse`'s malformed refusal and digest raw. Replay's `digest_of` (qsl-replay/src/request.rs:332-334) maps every parse refusal, the new `IntakeLimit::Memory` included, to `NotAPackageDocument` with no offset. So FR-260-AC-2's outcome exists only at `PackageDocument::parse`, which is what tc_730 tests. The ruling's premise that intake parses is true of that function, not of intake admission. Fix (team-leader call): either scope FR-260 B4/AC-2 and TC-730 step 3 to `PackageDocument::parse`, intake's one read, and say FR-154 admission still digests unparseable bytes raw per FR-056; or change FR-056's raw-digest rule and FR-056-AC-2's outcomes so admission surfaces the offset refusal. | spec/functional/FR-260-admit-semantic-ir-documents-at-any-depth.md:34-37; spec/functional/FR-260-admit-semantic-ir-documents-at-any-depth.md:53; spec/functional/FR-056-admit-domain-package-model-declarations.md:92-97; qsl-semantics/src/model/intake.rs:560-568; qsl-semantics/src/model/intake/unit.rs:87-88 |
| FND-003 | medium | FR-056's list of documents that "do not parse" (lines 86-90) gives non-UTF-8, non-JSON, a lone surrogate and a number with no finite double. It leaves out a repeated member name, and the byte order mark that intake.rs:250-251 names. The shared reader now refuses both. So a duplicate-name document, which FR-056 gives a `sha256-jcs` digest, is digested raw at admission and refuses `content-mismatch` under every `sha256-jcs` digest. FR-106 check 1.3 says only "bytes that do not parse as JSON", and duplicate-name text is valid RFC 8259 JSON, so observation now digests it raw too. The PR's behaviour-change list does not mention observation. The intake duplicate-name assertions (intake.rs:5577-5601) are traced to FR-056-AC-2, which does not name duplicates. Fix in this PR: add "a repeated member name in one object (RFC 8785 requires I-JSON)" and the BOM to FR-056's list and to FR-056-AC-2 or FR-260-AC-2. Reword FR-106 check 1.3 to "bytes the shared reader refuses". | spec/functional/FR-056-admit-domain-package-model-declarations.md:86-90; spec/functional/FR-106-admit-snapshots-and-invocations.md:197-198; qsl-semantics/src/model/intake.rs:5577-5601 |
| FND-004 | low | FR-259 B3 (and ADR-030 D-4.4, FR-255's `identity.input_bytes` row) makes `IDENTITY_LIMITS` the published 16777216-byte default of `identity.input_bytes`. The PR keeps `u64::MAX` and rewrites the constant's doc to say the ceiling is "none of its own". That contradicts B3, and the PR's "Not covered here" list does not mention it. Keeping `u64::MAX` for now is right: `preimage_digest` (qsl-semantics/src/value/semantic_node.rs:205-209) turns every encoder error into `NonCanonicalPreimage`, which would break B4 at a finite limit. Fix: list FR-259 B3 and its FR-255 setting under "Not covered here" with the owning slice or ticket. | quire-semantic-value/src/semantic_node.rs:106-112; spec/functional/FR-259-encode-identities-and-read-json-through-quire-canonical-at-any-depth.md:52-56 |

## New findings (disposition pass 1)

Round 1, reviewed at 7f5ded6050a87f07920fb65c18eda2afee730b2f.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The new FR-259 B6 says "QSL SHALL NOT report [an allocation failure] as a limit outcome, a malformed value or malformed input". Replay's request reader breaks this. `digest_of` calls `PackageDocument::parse` and maps every refusal, `ModelRefusalCause::AllocationFailed` and `IntakeLimitExceeded` included, to `ReplayRequestRefusal::NotAPackageDocument`, which reports `invalid_model_binding`/`malformed-declaration`. So an allocation failure while keying a `sha256-jcs` byte-provision entry is reported as malformed input. Fix: map the parse refusal by cause in `digest_of` (allocation-failed to `resource_exhausted`/`allocation-failed`, the intake limit to its limit outcome, everything else to `NotAPackageDocument`). Or, if replay is out of scope, narrow B6's "SHALL NOT" to the three sites it names and record replay's reader as remaining work. | qsl-replay/src/request.rs:331-334; spec/functional/FR-259-encode-identities-and-read-json-through-quire-canonical-at-any-depth.md:68-75 |
| FND-006 | low | Under the ruling, `byte-digest-mismatch` is the name for the domain-package digest mismatch. FR-108:48-50 still says the spine corpus' domain-package digest check refuses `stale_dependency`/`content-mismatch`. That is the same FR-056 check 3 refusal, and it was missed by 0284fa39b. The other remaining `content-mismatch` uses (FR-116, FR-128, TC-517: envelope and identity mismatches) mean something else, so they correctly keep the name. Fix: change FR-108's to `byte-digest-mismatch`. | spec/functional/FR-108-run-the-configversion-spine-corpus.md:48-50 |

## New findings (disposition pass 2)

Round 2, reviewed at 7420d36d86d4901cca21d0a5a213687a175e1348.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | New FR-071-AC-11 requires an entry "nested 1,000 deep" to refuse `resource_exhausted`/`intake-limit-exceeded` naming intake's nesting-depth limit. FR-260-AC-1 is the target state: "no intake outcome names a depth", and ADR-030 RU-1 says no depth setting of any kind. So once O-4 retires semantic-IR's `MAX_DEPTH`, FR-071-AC-11 and FR-260-AC-1 cannot both pass. FR-056-AC-2 already has the same interim depth clause, so this adds a second copy. Fix: word the clause by cause, not by depth, for example "an entry intake refuses at one of its limits refuses `resource_exhausted`/`intake-limit-exceeded` naming that limit and its bound". The test can keep using today's depth case. | spec/functional/FR-071-implement-typed-replay-request.md:139; spec/functional/FR-260-admit-semantic-ir-documents-at-any-depth.md:59 |

## Dispositions

Round 1, reviewed at 7f5ded6050a87f07920fb65c18eda2afee730b2f. I checked the fix rounds fedc3708f..7f5ded605 (f407facfd rebased onto 8d1deba43, patch-identical by `git range-diff`) by reading. `quire validate` passes (exit 0) on the 10 changed spec files.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | a5ef1516b: FR-259-AC-3 drops the observation sentence. FR-259 B5 exempts observation digest admission. FR-261's site list and B2 say it digests the bytes the reader refuses raw (FR-106 check 1.3). TC-729 step 3 now tests the allocation outcome instead, and its description points the malformed-input refusals to TC-730 and TC-733. |
| FND-002 | fixed | a5ef1516b (docs d5cd29192), option 1 by team-leader ruling: FR-260 B4 and AC-2 are scoped to `PackageDocument::parse` and say FR-154 admission digests reader-refused bytes raw per FR-056 (`byte-digest-mismatch` or `wrong-model-selection`). TC-730 step 3 reads through `PackageDocument::parse`. The intake.rs doc says the refusal belongs to the parse and that `admit` and `package_input` digest raw. |
| FND-003 | fixed | a5ef1516b and d5cd29192: FR-056's list now has the byte order mark and the repeated member name. New FR-056-AC-12 (TC-145) is backed by `a_repeated_member_name_or_byte_order_mark_does_not_parse`, which covers the offsets, a BOM at byte 0, and `admit` refusing `ByteDigestMismatch` with the raw digest as actual. FR-106 check 1.3 says "bytes the shared reader refuses". `bytes_the_shared_reader_refuses_are_digested_raw` covers observation, and the PR's behaviour-change list now names observation. |
| FND-004 | fixed | 6dabe354d: the `IDENTITY_LIMITS` doc states B3's 16777216 default and why it stays u64::MAX until the setting lands. The PR body's "Not covered here" list names FR-259 B3 and the `identity.input_bytes` setting. |


Round 2, reviewed at 7420d36d86d4901cca21d0a5a213687a175e1348. I checked 7f5ded605..7420d36d8 by reading.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-005 | fixed | 13f35f7ad: `digest_of` now maps by cause through `package_document_refusal`. `IntakeLimitExceeded` becomes `ReplayRequestRefusal::IntakeLimitExceeded { entry, limit, bound }` and `AllocationFailed` becomes `ReplayRequestRefusal::AllocationFailed { entry, requested }`, both `Code::ResourceExhausted` with Display `resource_exhausted/intake-limit-exceeded` and `resource_exhausted/allocation-failed`. Only the rest stays `NotAPackageDocument`. FR-259 B6 names the replay reader, FR-071 states the mapping, and FR-071-AC-11 with TC-186 step 9 is backed by `a_package_document_refusal_keeps_its_cause`, which passes (ir-sites log). |
| FND-006 | fixed | 96ca76178: FR-108:50 now says `stale_dependency`/`byte-digest-mismatch`. |

Round 3, reviewed at 9d935de7db1c0894d5fec7a152b547b1fbde5f94. I checked 7420d36d8..9d935de7d (one spec-only commit) by reading.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-007 | fixed | 9d935de7d: FR-071-AC-11 (line 139) and FR-056-AC-2 (line 323) now word the clause by cause: an entry, or bytes, that intake refuses at one of its limits refuses `resource_exhausted`/`intake-limit-exceeded`, naming that limit and its bound. Neither AC names a depth, so neither conflicts with FR-260-AC-1. TC-186 step 9's expected result uses the same wording. Its procedure keeps the 1,000-deep entry, which is today's reachable limit, and the test asserts `IntakeLimit::NestingDepth`, so it will fail loudly when O-4 retires that limit. That is a concrete test case following behaviour, not a spec conflict, so it is not a finding. |
