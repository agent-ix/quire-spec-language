---
id: SR-202611210022442313144108507
title: "QSL-2158 PR 3285 finite fraction mean-refutation base review"
type: SpecReview
analysis: base
scope: "spec/functional/FR-202-replay-a-probabilistic-witness.md and spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md, diff 3498af270b^..3498af270b only"
review_set: base
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-202", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-1043", type: references}
---

## Summary

This review covers only the mean/fraction refutation amendment in pinned
pure-spec commit `3498af270b4efc3bb1f07f453676af379939ad5d` (parent
`4fefc2c427`). The working copies of both documents are byte-identical to the
pinned blobs (`git diff 3498af270b --` on the two paths is empty). The
amendment follows the architecture ruling on semantics. The accepted
certificate uses the same finite-horizon-measure base and the deciding
extremum. Refutation needs a strict wrong-side value. Equality does not
refute. No probability-mass semantics are used, no wire field, profile or
request flag is added, and FSpec FR-413 subsystem evidence is reused. The AC
arithmetic is correct. Five defects block acceptance: the new criteria are not
registered as criteria, the amendment contradicts FR-202's existing
subsystem-replay rule and checked-subject entry scope, typed outcomes are
missing for non-refuting evidence, and partial-product admission is ambiguous.

## Verdict

**FAIL**. FND-001 and FND-002 are high. FND-003, FND-004, FND-005 and FND-007
are medium, and FND-006 is low. This verdict covers only the normative spec
scope of the two FR edits. It makes no claim about the implementation, the
tests, or whether the PR can merge.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The new AC rows come after a blank line, which ends each AC table. FR-1043-AC-8/9 (FR-1043 lines 210-212) and FR-202-AC-27/28/29 (FR-202 lines 509-512) are therefore not table rows. `quire properties --json` on both documents extracts FR-1043-AC-1..7 and FR-202 up to AC-26 only. `pandoc -f gfm` renders a split row as a `<p>`. As a result the matrix cannot see these five criteria and trace tags to them cannot bind. Fix: delete the blank lines at FR-1043 line 210 and FR-202 line 509. | FR-1043-AC-8, FR-1043-AC-9, FR-202-AC-27, FR-202-AC-28, FR-202-AC-29 | wrong-requirement |
| FND-002 | high | The new mean rule (FR-202 lines 118-147, FR-1043 lines 174-197) checks an `exact` minimum/maximum or an FR-1042 bound against the complete minimum/maximum base a proof would use. Lines 146-147 say "existing replay ... rules continue to apply", and FR-1043 line 193 reuses "FR-202's ... replay acceptance rules". The existing FR-202 rule at lines 450-452 is unamended: "For subsystem evidence it SHALL rebuild the induced chain ... by applying the witness scheduler, and run FR-201's checker on it." Adverse case: FR-1043-AC-8's exact minimum `11/48` document (optimization minimum) rechecked on the `slow`-induced chain has no choices left to minimise. It either refuses objective mismatch or is checked under a different base, so AC-8/FR-202-AC-28 cannot pass while line 450 holds. Fix: amend line 450 to exempt mean certificates, or state that they are checked over their base product. | FR-202, FR-1043, FR-202-AC-28, FR-1043-AC-8 | wrong-requirement |
| FND-003 | medium | The amendment does not say which replay entry accepts mean subsystem evidence. The checked-subject entries "SHALL report unsupported for subsystem evidence" (FR-202 line 222), and FR-202-AC-11, AC-16 and AC-21 assert subsystem evidence is unsupported or gives no token on the native/bounded entries. FR-202-AC-27/28/29 and FR-1043-AC-8 name no entry. Adverse case: a native implementation that satisfies AC-27 violates line 222 and AC-16, and a wire-facade one hits FND-002. Fix: name the entry in the mean bullet and AC-27, and scope line 222 accordingly. | FR-202-AC-27, FR-202, FR-202-AC-11, FR-202-AC-16, FR-202-AC-21 | wrong-requirement |
| FND-004 | medium | The amendment gives no typed outcome for non-refuting mean evidence. It only says "does not refute" or "yields no accepted refutation" for four cases: a value or bound equal to `c`; a wrong-kind document for the role (a `lower` on the minimum for `>= c`, an `upper` on the maximum for `<= c`); failed certificate conditions; and a witness policy that disagrees with the document's policy. The surrounding text types analogous cases differently: FR-202 line ~471 gives a sum at the bound `inconclusive`/`ReplayParity`; FR-1042 line 164 gives wrong kind `Malformed` at `kind`; FR-1043 line 154 gives `ThresholdMissed`; FR-1042 line 188 gives `CertificateRejected{rule, at}`. Adverse case: AC-9's exact `5/12` at `>= 5/12` may settle `ReplayParity` in one implementation and `CertificateRejected{ThresholdMissed}` in another. AC-28's `lower` on the minimum may settle `Malformed` at `kind` or parity. Fix: state each outcome, and say which of the checker or replay issues it. | FR-1043-AC-9, FR-202-AC-28, FR-202-AC-29, FR-1043 | missing-requirement |
| FND-005 | medium | The amendment does not say whether a partially explored product can carry a mean refutation. FR-202 line 125 requires the checker to check the "complete product", but the unamended line 172 says evidence on a partial product settles the item when it is "a `Lower` certificate refuting a `<= θ` bound". FR-1042 lines 92-96 also admit a `lower` on a partial product. Adverse case: `max_states` stops EN-5 on FR-1043-AC-8's maximum with an explored-part `lower` `2/5 > 1/3`. Under line 172 it refutes `<= 1/3`; under line 125 it must be refused. Fix: state whether a mean `lower` found on a partial product refutes, and which least value counts for unnamed successors of a `fraction` base. FR-1042 line 93 lists none for `fraction`. | FR-202, FR-1043, FR-1043-AC-8 | wrong-requirement |
| FND-006 | low | The new text names the mean threshold `c` (FR-1043 line 174, FR-202 line 118), but FR-1043 elsewhere writes `mean of F op θ` (lines 34, 74). In FR-1043, `c` is also `measure.threshold`, which is null for payoff `fraction` (lines 100-104). Adverse case: an implementer who takes "the same complete objective" and `c` literally writes the mean threshold into `measure.threshold`, and that refuses objective mismatch. Fix: use `θ` for the mean threshold in both new sections. | FR-1043, FR-202 | wrong-requirement |
| FND-007 | medium | FR-1043-AC-9 and FR-202-AC-28 give no concrete document for the wrong-optimization adverse case ("a lower bound on the minimum or an upper bound on the maximum ... even when strictly past the threshold"). Adverse case: a test can offer a `lower` on the minimum of `1/2`, which is invalid because the minimum is `11/48`. It is rejected for its conditions and passes the AC without exercising the role check. Fix: name valid documents, e.g. a `lower` on the minimum of `1/5` (`<= 11/48`) at `>= 1/2` and an `upper` on the maximum of `1/2` (`>= 5/12`) at `<= 1/3`, both valid and on the wrong side, and neither refutes. | FR-1043-AC-9, FR-202-AC-28 | wrong-requirement |

## Coverage

Examined units: FR-202 Description (lines 27-35), Emission mean bullets
(lines 118-147), FR-202-AC-27/28/29; FR-1043 relationships, Forms preamble
(lines 66-67), Checking/results (lines 148-170), Mean-refutation role
(lines 172-197), FR-1043-AC-5/8/9. Context only: FR-202 lines 172-174, 218-227
and 436-470; FR-1042 lines 84-100, 112-123 and 155-195; FR-202-AC-11/16/21.

- **Arithmetic.** `Retry` over `[0, 2]` has 3 positions with delivery
  probabilities 0, 1/2 and 3/4, so the mean is `(5/4)/3 = 5/12`. With `fast`
  `1/2` and `slow` `1/4`, the sum `2p1 + p2 − p1·p2` is minimised by
  `slow, slow` at `11/16`, giving `11/48`, and maximised by `fast, fast` at
  `5/4`, giving `5/12`. Mixed policies fall in between. `upper` `1/4` is at
  least `11/48` and below `1/2`, so it refutes. `lower` `2/5` is at most
  `5/12` and above `1/3`, so it refutes. The AC-9 equality cases (`5/12`,
  `upper` `1/2`, `lower` `2/5`) are valid bounds at equality. The FR-1042
  policy table (lines 120-121) requires a policy for an `upper` on a minimum
  and a `lower` on a maximum, which matches AC-8's `slow`/`fast`.
- **Architecture ruling.** Same base and profile: yes. Deciding extremum:
  minimum for `>= c`, maximum for `<= c`. Strict sign: yes. Equality does not
  refute: yes. The proof `ThresholdMissed` does not reject the refutation
  role (FR-1043 lines 189-190, FR-202 lines 138-140). A path set or
  probability-mass comparison is excluded. No new wire field, profile or
  request flag. Cancellation and caller limits are inherited (FR-1043 line
  197, FR-202 lines 146-147, FR-202-AC-29).
- **Contradiction check.** The FR-1043 Forms table is scoped to proofs by
  the new preamble, and line 154 is narrowed to the proof threshold, so
  neither contradicts the new role. The FR-1042 claim-side rule (line 157)
  is overridden explicitly, and the overriding text does not conflict with
  it. The remaining conflicts are FND-002, FND-003 and FND-005.
- **FSpec carrier.** The author's citations were verified read-only at
  filament-specification `903947958abe7ff32865ace12d5f29ed0ec24074`:
  - schema.json `ProbabilisticCounterexample.evidence` `subsystem` carries
    `certificates[1..2]` of `ProbabilityCertificate`, and `scheduler` is
    optional;
  - FR-413's subsystem paragraph allows expected-reward refutations "with
    their own certificate kinds";
  - FR-412's finite-horizon-measure bullet says "the completed fraction for
    a mean";
  - FR-412's side rule says "`upper` for ... a refutation of `>= θ`;
    `exact` ... for either".

  These support the carrier claim. FSpec does not need to change.
- **Hash/pin antipattern.** None. "identity" means the structural item
  identity, not a digest or a pin.
- **Structure.** `quire validate --scope .` on the two paths exited 0 with no
  output. `git diff --check 3498af270b^ 3498af270b` on the two paths exited
  0. The validator does not detect FND-001.

## Assurance Context

No AssuranceProfile document and no `review_policy` exist in this repository.
A search for `type: AssuranceProfile` and `review_policy` found only prose in
older reviews under `reviews/`. No enforced spec-review run applies and no
acceptance gate is claimed. Tools: quoin 0.36.8, quire 0.52.0
(engine 0.62.2), module `spec-artifacts-process@v0.36.0`. No cargo, tests,
matrix or full validation sweep ran, by dispatch.

## Dispositions

### Round 1

Rechecked against pure-spec commit `beb797a543184e85a1f724296b76dc00fd930951`
(diff `3498af270b..beb797a543` on the two paths). The working documents are
byte-identical to that commit. `quire properties --json` on both documents now
extracts FR-1043-AC-1..9 and FR-202-AC-27..29, each exactly once.
`quire validate --scope .` on the two paths exits 0 with no output, and
`git diff --check` exits 0.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1: The blank lines are gone, so both AC tables are contiguous. quire extracts FR-1043-AC-8/9 and FR-202-AC-27/28/29. |
| FND-002 | fixed | Round 1: FR-202 Replay rules: "For subsystem evidence other than a mean certificate it SHALL rebuild the induced chain ... For a mean certificate it SHALL check the original finite-horizon-measure base product, preserving its workload, minimum or maximum optimization ... applying the witness scheduler SHALL NOT replace that base with an induced-chain objective." |
| FND-003 | fixed | Round 1: FR-202 adds the "Checked-subject mean-certificate replay entry". Native replay routes mean subsystem evidence to it, and the wire facade dispatches to the same checks. Line 222 is narrowed to "The checked-subject path-set subsets", and AC-11, AC-16 and AC-21 are narrowed to the path-set entry and subset. AC-27 and FR-1043-AC-8 name the entry. |
| FND-004 | fixed | Round 1: Outcomes are now typed, and each case has exactly one outcome. A valid document failing the strict comparison, including equality, gives `ThresholdMissed` at the initial key. A wrong kind for the role gives `Malformed` at `kind`, "even if its number lies on the wrong side". Failed conditions keep FR-201's rule and locus. Each of these settles `inconclusive`, `CertificateRejected{rule, at}`. Policy disagreement or a witness that does not attain the exact extremum gives `ReplayParity`, and identity or base mismatch keeps its refusal. The text is the same in FR-1043 and FR-202, and AC-9, AC-28 and AC-29 match it. |
| FND-005 | fixed | Round 1: The text drops "complete product" for "product coverage". `exact` and `upper` use the full stopped product. A `lower` may name a subset, with each unnamed `fraction` successor counted as 0, and an accepted partial `lower > θ` refutes `<= θ` without a counterpart. This is consistent with FR-202 line 172 and FR-1042 lines 90-97. |
| FND-006 | fixed | Round 1: The new mean text uses `θ` throughout, and FR-1043 adds that `θ` "SHALL NOT be written into `measure.threshold`, which remains null for payoff `fraction`". The remaining `c` occurrences are quantile `M op c`, which is correct. |
| FND-007 | fixed | Round 1: AC-28 and FR-1043-AC-9 name a valid `lower` on the minimum of `1/5` (at most `11/48`) at `>= 1/2`, and a valid `upper` on the maximum of `1/2` (at least `5/12`) at `<= 1/3`. Each rejects `Malformed` at `kind`. |

Round-1 checks: the arithmetic is unchanged and correct. The partial `lower`
`2/5 > 1/3` on the maximum `5/12` can be met by a subset that leaves out only
zero-payoff decided states, so AC-9 can be satisfied. The new entry's contract
covers the package, model, claim binding, typed certificate, limits, `&Cancel`,
a final poll and an opaque outcome. It adds no wire field, profile, request flag
or hash or pin. FSpec is unchanged and is not required. No new findings.

**Round-1 verdict: PASS.** The normative spec scope of the FR-202 and FR-1043
mean/fraction refutation amendment is accepted on BASE. This verdict makes no
claim about the implementation, the tests, or whether the PR can merge.

## New findings

### Round 2 scope extension

This round extends BASE scope to two more pure-spec commits:

- `d80cc467ebc3f7904e1babf3654ac9767f0fd02f` (FR-203: certified conditional
  vacuity);
- `75a71e3f3c0b4a7233b22b88bb40c3bb81951fd7` (FR-1042, FR-202, TC-3151:
  checked bound policy hints).

The working docs are byte-identical to these commits, and no later commit
touches `spec/`. `quire validate --scope .` on the four docs exits 0. Its one
warning, `ac:vague-response` at FR-1042 line 227, is on the unchanged
FR-1042-AC-1. `quire properties` extracts FR-203-AC-10/11 and
FR-1042-AC-12/13/14, each once. `git diff --check` is clean on both commits.

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-008 | high | FR-201's failure table is unamended, and FR-1042 reuses FR-201's rules and loci. It still makes "a lower bound on a maximum without `policy`" `Malformed` at `policy` (FR-201 line 105), and "a policy entry is missing" `InequalityFails` (line 100). This contradicts FR-1042 lines 84-86 and 129-150, which make `policy` an optional hint and synthesize the choice when it is absent. Adverse case: FR-1042-AC-3's `lower` `s` `7/16`, rank 1, offered with no `policy` (and AC-12's and AC-13's no-row `lower` maximum documents) is accepted under FR-1042 but rejected `Malformed` at `policy` under FR-201. Fix: amend FR-201 lines 100 and 105 so they do not apply to FR-1042 hint documents, or scope them to the profiles that still require `policy`. | FR-1042, FR-201, FR-1042-AC-3, FR-1042-AC-12, FR-1042-AC-13 | wrong-requirement |
| FND-009 | medium | The witness can disagree with the checker's choice on a valid refutation. FR-202 lines 133-137 require the counterexample's scheduler to equal the checker's selected policy, which is the first satisfying action when a hint is absent. EN-5, however, forms its witness from "the optimizer's attaining policy" with ties broken by canonical transition order (lines 97-100), and nothing requires it to emit hints matching that witness. Adverse case: a refuting `upper` on the minimum has slack, and the canonical-first action satisfies `Q ≤ y` but is not the attaining action. The checker then selects the first action, the witness takes the attaining one, and replay settles `ReplayParity` instead of `refuted`. The orderings are also named inconsistently: "canonical action order" in FR-202 versus "canonical full-product-action order" in FR-1042. Fix: require EN-5 to emit a hint equal to its witness choice at each existential state, or to form the witness from the selected policy, and use one order term. | FR-202, FR-1042 | missing-requirement |
| FND-010 | medium | FR-203 lines 111-119 apply the vacuous `Exhaustive` settlement to every "conditional claim" with no reachable activation. FR-1043 line 73 still proves an every-scheduler quantile through a `shifted` document "0 with no activated state reachable", and line 163 settles it `ExactValue`. FR-203-AC-10 names no scheduler. Adverse case: every-scheduler `quantile 0.5 of (steps from holds(false) until … within 4) <= 2` settles `proved` `Exhaustive` with a `Vacuous` certificate under FR-203. Under FR-1043 it settles `proved` `ExactValue` from a `shifted` document, and that value is undefined, which the vacuity ruling forbids fabricating. Fix: state which conditional forms take the vacuity route, align the FR-1043 every-scheduler row and settlement, and name the scheduler in AC-10. | FR-203, FR-1043, FR-203-AC-10 | wrong-requirement |
| FND-011 | low | FR-203 lines 120-133 and AC-10/11 require FR-331's contract to admit a `conditional` `vacuous` certificate on an `exhaustive` basis. FSpec origin/main `247a3941` schema.json line 1465 still sets `certificate: false` for `exhaustive`. FR-203's Dependencies and References name only FSpec FR-411, not the FR-331/FR-412 companion exception. Adverse case: a reader who validates AC-10 against current FSpec gets a schema refusal and cannot tell the AC depends on an unlanded companion. Fix: name the FSpec FR-331/FR-412 companion in FR-203's References. | FR-203, FR-203-AC-10, FR-203-AC-11 | missing-requirement |

Round-2 checks that passed:

- **Vacuity ruling.** FR-203 meets it. The vacuous case is value-free
  `Exhaustive` and `Certified`, and settles only after independent acceptance
  of `Conditional` `Vacuous`, with absence of activation rederived for every
  initial state and binding, an opaque acceptance bound to package, subject,
  claim and binding, and complete exploration. No fabricated 0 or `0/0`, no
  new basis, and no numeric entries, bounds or `evidence`. Other `exhaustive`
  results keep their no-certificate restriction. `exact-value` still requires
  a `Rational`; the sketch change from `ExtRational` to `Rational` matches
  that. Infinity stays unsupported per FR-201 lines 832-833. Rejection with
  reachable activation is `InequalityFails`, which agrees with FR-201 line 106.
- **Policy-hint ruling.** FR-1042 meets it. The checker rebuilds every action
  for an `upper` on a minimum or a `lower` on a maximum. A present hint must
  name an enabled action that satisfies all per-state conditions, or it
  rejects without fallback; a nonexistent or disabled hint gives
  `InequalityFails`, which agrees with FR-201 line 100. An absent hint is
  existential: the first satisfying action is selected, with `InequalityFails`
  versus `RankBroken` stated. Terminal-stutter and post-state choices use
  existing actions, with no new wire form. Universal rows remain universal.
- **Arithmetic on `W`.** `Q_go = 1/2`, and `wait` loops back to `s`. AC-3's
  `7/16` is met by `go`, while `wait` satisfies the inequality but fails
  progress. AC-14's `1/4` fails with `go` (`1/2`) and passes with `wait`;
  `lower` `1` fails progress with `wait` and fails the inequality with `go`,
  so it gives `RankBroken`. AC-13's `1/2`, `1/8` and `7/8` are correct for
  `1/4` and `3/4`. AC-12's all-zero fraction is correct.
- **Unchanged rules.** The existing `ReplayParity` mismatch rule, the
  accepted-refutation rule and the equality rules are unchanged. TC-3151's
  scope and steps match AC-12 to AC-14.
- **No ceremony.** No hash, pin or tracking ceremony is introduced.

### Round 2 dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-008 | still-open | Round 2: new finding; no fix yet. |
| FND-009 | still-open | Round 2: new finding; no fix yet. |
| FND-010 | still-open | Round 2: new finding; no fix yet. |
| FND-011 | still-open | Round 2: new finding; no fix yet. |

FND-001 to FND-007 keep their latest outcome, `fixed` in round 1, and need no
new row.

**Round-2 verdict: FAIL** for the extended BASE scope (FR-203, FR-1042,
FR-202, TC-3151) because of FND-008 to FND-011. The mean-refutation
amendment's own BASE acceptance from round 1 is unchanged. This verdict makes
no claim about the implementation or about whether the PR can merge.

### Round 3 new findings

This round rechecks FND-008 to FND-011 against pure-spec fix commit
`89e3c26ee2a2d2dba4c1917d445b9337cc9fe9a1`, which changes only FR-201,
FR-202, FR-203 and FR-1043. FR-1042 and TC-3151 are unchanged from
`75a71e3f3c`. The working docs are byte-identical to the commit, and no later
commit touches `spec/`.

- `quire validate --scope .` on the four docs exits 0 with no output.
- `quire properties` extracts FR-1043-AC-10, FR-202-AC-30 and
  FR-203-AC-10/11, each once.
- `git diff --check` is clean.

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-012 | medium | The new precedence preamble in FR-1043 (lines 68-75) says the numeric quantile rows "apply only when activation is reachable". No rule says how the checker rejects a numeric document offered where its own search finds activation unreachable. The `<= c` every-scheduler row still accepts a `shifted` initial value "at least 0" with no activation qualifier. FR-203-AC-11 covers only certificates offered on an `exhaustive` basis. Adverse case: every-scheduler `quantile 0.5 of (steps from holds(false) until holds(true) within 4) <= 2` is offered as an `exact` `shifted` document with every value 0, or as a workload `ratio` with denominator 0. One checker reads the row and accepts a fabricated numeric `ExactValue`; another rejects under an unnamed rule. Fix: name the rule and locus (for example `InequalityFails` at the initial key), require settlement `inconclusive`, `CertificateRejected`, and add the case to FR-1043-AC-10. | FR-1043, FR-1043-AC-10, FR-203 | missing-requirement |

### Round 3 dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-008 | fixed | Round 3: FR-201's `InequalityFails` row now reads "a policy entry required by its profile is missing". The `Malformed` row adds "where its profile requires that member (excluding FR-1042 optional-hint documents)". A new paragraph applies the `value-bounds/v1` optional-hint rules, whose profile name matches FR-1042 line 30: first-satisfying canonical selection, no fallback for a present hint, and `InequalityFails`/`RankBroken`. It contains no contradiction with FR-1042-AC-3, AC-12, AC-13 or AC-14. Other profiles, including `Optimum` `reset`, keep their rules. |
| FND-009 | fixed | Round 3: FR-202 exempts mean-bound certificates from the optimizer-witness rule and requires EN-5 to build the witness from the policy its independent FR-1042 check returns. Matching hints are optional where a representable form exists; implicit stutter and post-state choices get no invented operation. The order term is unified as "canonical full-product-action order". New FR-202-AC-30 covers the slack case and old-witness `ReplayParity`. A fixture exists: with an initial `y` of `1/2` at `>= 3/5` on AC-8's model, both `fast` and `slow` satisfy the bound. |
| FND-010 | fixed | Round 3: FR-1043's preamble gives vacuity precedence for every conditional reading, under a workload or every scheduler, with or without fairness. The every-scheduler tight-0 branch now requires activation to remain reachable, and settlement separates vacuous `Exhaustive` from numeric `ExactValue`. FR-203's scope covers every FR-201 conditional reading, and FR-203-AC-10 names the claim under a workload and under every scheduler, with and without fairness. FR-201 already accepts `Vacuous` under both scheduler clauses (lines 685-690, FR-201-AC-53). The one remaining gap, typed rejection of a numeric document for a vacuous claim, is FND-012. |
| FND-011 | fixed | Round 3: FR-203 adds `depends_on` links to FSpec FR-331 and FR-412, and a References entry stating that FR-331 owns the value-free `exhaustive` carrier with its sole `vacuous` certificate exception and forbidden `evidence`, and FR-412 owns the `vacuous` certificate and its independent check. The dependency is stated in normative terms; the FSpec companion (#787) has not merged, and the actual pin comes later. |
| FND-012 | still-open | Round 3: new finding; no fix yet. |

FND-001 to FND-007 stay `fixed` from round 1.

**Round-3 verdict: FAIL** on FND-012 alone. FND-008 to FND-011 are fixed.
This verdict covers normative spec acceptance only. It makes no claim about
the implementation or about whether the PR can merge.

### Round 4 new findings

This round rechecks FND-012 against pure-spec fix commit
`373bfe30686284837cb172b4b0b63974132faaa7`, which changes only FR-1043. All
other docs are unchanged from `89e3c26ee2` and `75a71e3f3c`. The working docs
are byte-identical to the commit, and no later commit touches `spec/`.
`quire validate --scope .` on FR-1043 exits 0, and `git diff --check` is clean.

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-013 | medium | The new FR-1043 rejection (lines 75-81) and FR-1043-AC-10 reject a workload `ratio` with denominator 0 when activation is unreachable, with `InequalityFails` at the initial complete key. FR-1043's workload row uses "FR-201's `conditional` `ratio`", and the unamended FR-201 lines 643-645 accept it: "At an initial key where the denominator is 0 it SHALL confirm by its own reachability search that no first activating state is reachable, and that instance holds vacuously." That rule does not require any initial key to activate. Adverse case: FR-203-AC-10's claim under a resolved workload, offered as a `ratio` whose denominator is 0 at every initial key. Under FR-201 every instance holds vacuously and the `Ratio` is accepted on the proof side; under FR-1043 it is rejected `InequalityFails`. Fix: state the precedence. In FR-1043, say the all-unreachable rejection overrides FR-201's per-key vacuity for this profile, or qualify FR-201 lines 643-645 so they apply only when some initial key activates. | FR-1043, FR-1043-AC-10, FR-201 | wrong-requirement |

### Round 4 dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-012 | fixed | Round 3's gap is closed. FR-1043 now rejects any offered numeric quantile certificate (an all-zero `exact` or bound `shifted` document, or a workload `ratio` with denominator 0) with `InequalityFails` at the initial complete product key, settling `inconclusive`, `CertificateRejected{rule, at}` with no `ExactValue`. The every-scheduler `shifted` row is now qualified by "with activation reachable in the complete product" for both `<= c` and `>= c`. AC-10 adds the negative offerings, and the rule and locus match FR-201 line 106. The one remaining conflict with FR-201's `Ratio` prose is FND-013. |
| FND-013 | still-open | Round 4: new finding; no fix yet. |

FND-001 to FND-011 stay `fixed`.

**Round-4 verdict: FAIL** on FND-013 alone. It is a single precedence
sentence, fixable in FR-1043 or FR-201. Every other branch is consistent: the
comparison-event and mean rows are not conditional and are unaffected, the
workload row routes unreachable activation to vacuity, and the shifted row
and the tight-0 branch both require reachable activation. This verdict covers
normative spec acceptance only. It makes no claim about the implementation or
about whether the PR can merge.

### Round 5 dispositions (targeted FND-013 verification)

This is a targeted check of FND-013 only, made under an explicit architecture
exception and not a full review round. It covers pure-spec commit
`1d8d38109c78083a994d1f7737182c207971002a`, which changes only FR-201 and
FR-1043. The working docs are byte-identical to the commit, and no later
commit touches `spec/`. `quire validate --scope .` on both docs exits 0 with
no output, and `git diff --check` is clean.

| FND | outcome | reason |
| --- | --- | --- |
| FND-013 | fixed | Round 5: FR-201 lines 643-652 now treat a denominator-0 initial key as vacuous "only when at least one other initial key has reachable activation". When no initial key has reachable activation, the numeric `Ratio` is rejected `InequalityFails` at the first initial complete product key in canonical order, settling `inconclusive`, `CertificateRejected{rule, at}`, and the claim is certified only through an accepted `Vacuous` certificate with the value-free `Exhaustive` basis under FR-203. FR-1043 lines 82-85 point mixed-key workload ratios to FR-201's per-key rule. The FR-201 and FR-1043 outcomes now agree, and the locus matches FR-201 line 106 and FR-1043's "initial complete product key", which FR-201 refines with canonical order. |

FND-001 to FND-012 stay `fixed`.

**Final verdict: PASS.** All findings FND-001 to FND-013 are closed. The
normative BASE spec scope is accepted for FR-202 and FR-1043 (mean/fraction
refutation, vacuity precedence), FR-203 (certified conditional vacuity), and
FR-1042, FR-201 and TC-3151 (policy hints, profile exceptions, ratio vacuity
precedence). This verdict makes no claim about the implementation or the
tests, or about whether the PR can merge.

### Round 6 scope addition (FR-1043 FractionValue undefined observation)

This round is a narrow normative addition authorized by architecture; it is
not a reopened review loop. It covers pure-spec commit
`faa136f99d063852b939c04fb16772e356913c32`, which changes only FR-1043: two
Behavior bullets under "Product and payoff" (lines 150-172) and
FR-1043-AC-11. The working doc is byte-identical to the commit, and no later
commit touches `spec/`. `quire validate --scope .` on FR-1043 exits 0 with no
output, `git diff --check` is clean, and `quire properties` extracts
FR-1043-AC-11 once.

No new findings.

Round-6 checks:

- **Architecture ruling.** The final departing step's contribution is
  tentative, and the quotient is evaluated before any outside-window
  predicate. A zero total gives the existing `division-by-zero` cause at the
  fraction expression. The location is position `h + 1` with the actual
  resolved post-state, while the control and accumulator stay at their
  undecided pre-step values and the contribution is not committed. Nothing
  is fabricated: no predicate read, no decided control, no broken-invariant
  failure, and no wire extension.
- **Consistency with FR-202.**
  - The checked-subject undefined entry (lines 434-472) requires the control
    to be undecided before the read; the pre-step control here is undecided
    with `remaining = 1`.
  - That entry requires the first undefined evaluation to be at `where`
    (locus, position, complete pre-read key) with an equal cause, or settles
    `ReplayParity`; FR-1043's replay bullet uses the same comparison and the
    same `ReplayParity` outcome.
  - Limits and cancellation stay `incomplete`.
  - The pattern of position 1 with a pre-read key matches FR-202-AC-24 and
    AC-26 on `Coin`.
- **Consistency with FSpec FR-413** (origin/main `247a3941`, lines 158-164
  and 190-197). `where` carries the `locus` (the occurrence of the claim
  expression with no value), the `position`, and a `product_state.key` that
  is "the complete … product key of the state before the read (the pre-read
  monitor and accumulator, which an undefined value does not advance)". This
  matches the post-state model content combined with the pre-step control
  and accumulator. `division-by-zero` is an existing catalogued cause (FSpec
  native-diagnostics line 261). No new locus or evidence kind is needed.
- **Arithmetic in AC-11.** The window `[0,0]` gives `h = 0`, and the control
  starts at `remaining = 1`. Position 0 has `c.v = 0`, so the predicate
  holds. The departing `drop(0)` has weight `R = 0`, so the tentative sums
  are `(0,0)` and the total of 0 is undefined. The path has probability
  `1/2` and its post-state lacks `c`, so reading `c.v` outside the window
  would give `absent-key`; the ordering therefore decides the cause, and
  that is the genuine adverse case. `drop(1)` is also undefined with weight
  0, which is consistent and does not affect the evidence. Under FR-202-AC-24
  an undefined read refutes regardless of the threshold. The forged
  variants (position 0, a decided control, a changed accumulator, the
  `absent-key` cause) each disagree on the location or cause, so they give
  `ReplayParity`. The `weighted by R` syntax matches FR-190 and FR-392. The
  ordinary positive-weight reward case is not required, as directed.

| FND | outcome | reason |
| --- | --- | --- |
| (none) | - | Round 6: no new findings; FND-001 to FND-013 remain closed and need no new row. |

**Round-6 verdict: PASS.** The FR-1043 FractionValue undefined-observation
addition is accepted on BASE, and FND-001 to FND-013 remain closed. This
verdict covers normative spec acceptance only. It makes no claim about the
implementation, which is still in progress, or about whether the PR can
merge.

### Round 7 scope addition (CR-003, FR-203 mixed-initial activation coverage)

This round is a narrow normative addition authorized by architecture, and it
is owned by FSL. It covers pure-spec commit
`51d98fca473ac03d87e4788340fb82bfe8c0fa61`, which changes only FR-203: the
entries and coverage bullet at lines 208-232 and FR-203-AC-12. The working
doc is byte-identical to the commit, and no later commit touches `spec/`.
FR-201, FR-202 and FR-1043 are unchanged since `faa136f99d`; other spec docs
that changed on the branch in between are outside this scope.
`quire validate --scope .` on FR-203 exits 0 with no output,
`git diff --check` is clean, and `quire properties` extracts FR-203-AC-12
once.

No new findings.

Round-7 checks:

- **Architecture ruling.** FR-203 meets it.
  - Every admitted (initial, binding) pair is accounted for in the
    independently checked acceptance, either by its checked value or by
    unreachable activation under FR-201's per-key rule.
  - That coverage is derived from the existing certificate values and
    bindings. There is no external coverage member, no wire change, and no
    FSpec change or pin.
  - There is exactly one numeric entry per activating pair, with its actual
    initial index and binding, so the builder cannot hardcode index 0.
    Nonactivating pairs are omitted, with no `0/0` or scalar placeholder.
  - A missing, duplicate, nonactivating or mismatched entry settles
    `failed`, `runtime_invariant`/`established-invariant-broken`.
  - When nothing activates, the `Vacuous` certificate and `Exhaustive`
    basis are required.
- **Consistency with FR-201.** FR-201 lines 643-652 treat a
  denominator-0 key as vacuous only when another initial key activates.
  AC-12 is exactly that mixed case: index 1 activates and index 0 does not,
  so the `Ratio` is accepted with index 0 per-key vacuous. The all-unreachable
  case stays routed to `Vacuous`, as the bullet's final sentence states.
- **Consistency with the existing `ExactEntry`.** `ExactEntry`
  (`initial: u32`, `binding`, `value`, `extremum`) carries the actual index
  and binding unchanged, and FR-203 line 88 keeps entries numeric-only. The
  least favourable pair is chosen among activating pairs, because only they
  have values. For AC-12 that is index 1 with `3/4`, so `ExactValue{3/4}`
  matches FR-1043's "ratio for a workload quantile". No average or
  first-value summary is introduced.
- **Arithmetic in AC-12.** Index 1 starts undelivered, so `not m.delivered`
  holds at position 0 and activation is immediate. The only positive-weight
  step, `fast`, delivers with probability `1/2`, so `Pr(M <= 2)` is
  `1/2 + 1/4 = 3/4` with a denominator of 1, and `3/4 >= 0.5` proves the
  claim. This matches FR-1043-AC-1. Index 0 is delivered and terminal, so it
  stutters forever, never activates, and has numerator and denominator 0.
  `slow` at weight 0 is admitted (FR-185 lines 177-181, FR-185-AC-10). The
  four negative entry sets (an extra index 0, a missing index 1, a duplicate,
  and index 0 substituted for index 1) each break the
  exact-activating-coverage rule, so they settle `runtime_invariant`. The
  typed outcomes are deterministic.

| FND | outcome | reason |
| --- | --- | --- |
| (none) | - | Round 7: no new findings; FND-001 to FND-013 remain closed and need no new row. |

**Round-7 verdict: PASS.** The FR-203 mixed-initial activation coverage
addition is accepted on BASE. FND-001 to FND-013 remain closed, and the
round-6 FractionValue AC-11 remains accepted. This verdict covers normative
spec acceptance only. It makes no claim about the implementation, which is
still in progress, or about whether the PR can merge.
