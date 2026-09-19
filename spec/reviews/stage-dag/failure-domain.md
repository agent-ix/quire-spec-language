---
id: SR-467
title: "failure-domain review of ADR-011 stage DAG and dependency architecture"
type: SpecReview
analysis: failure-domain
scope: "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---

## Summary

Reviewed ADR-011 (ARCH-10, #209) at quire-spec-language commit 944a1c8, plus
the uncommitted Decision 11 citation line, and its `spec/spec.md` index row.
The inputs were #209, gate #212, ADR-010 and accepted QSpec AD-016
(`origin/main`). The failure-domain checklist was applied to an architecture
record. It covers trust boundaries at stage edges and side inputs, entity
identity carried across edges, purity of the reference executor that replay
relies on, and topology of the stage, module and package graphs.

The stage DAG, the per-edge partial-output rules and the forbidden-bypass list
are sound. The E7 per-item rule and the E8 no-placeholder rule match the AD-016
terminal-disposition rule. The index row is correct. Two high findings block
acceptance. First, the trust anchor that lets wire bytes stand in for a checked
package (I2, and the "verified binding" in FB-03) is undefined. As written, a
digest check is enough to turn bytes into a checked value. Second, the §6.1
layer table has an upward dependency. S3 `check` takes I2 linked packages,
whose type belongs to layer-4 `package`, so the "acyclic" claim #212 relies on
does not hold. Seven medium findings cover unstated failure modes at E9,
executor purity, node-id uniqueness, the proof-gate claim set, a contradiction
with the executor path that AD-016 names, resource limits and the package
import graph. Two low findings are also open.

Verdict: REJECT. The two high findings need decisions, not wording changes.
Each has a concrete fix below. None of them reopens AD-016 or decides a #210 or
#211 question.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Trust boundary: the trust anchor for wire bytes is undefined. The I2 row says an imported package "enters S3 as an S4 linked package. Its identity is verified when it is read. Its declarations are never re-checked from its source." §4 says an I2 reader "re-establishes the checked state only by verifying the package identity it reads". A package identity is a digest, and anyone can compute a digest over bytes they wrote. Verifying it shows the bytes are intact. It does not show that S3 produced them. FB-03 forbids exactly this: wire data treated as checked "without a verified binding to a checked producer". The ADR never defines that binding, so I2 either breaks FB-03 or relies on an undefined term. The same gap applies to E5, where IR admits v2 bytes, and to E9, which needs "the S4 package of the same identity". "Never re-checked" also settles part of the question the ADR hands to #211 ("the rule by which an I2 reader re-establishes the checked state"). Fix: define "verified binding to a checked producer" once, in §3 under FB-03. State the trust anchor: an I2 or E5 package is admitted only when its package identity equals an identity recorded in an input the caller trusts, for example the importing package's library lock, together with a supported v2 schema version. Say explicitly that the digest proves integrity, not checking. Name the refusal for a mismatch or an unlisted identity. Then ask #211 which trust anchor to use and whether I2 re-checks, and remove "never re-checked" until #211 decides. | ADR-011 §1 I2 row, §3 FB-03, §4 bullet 2, §2.1 E5 and E9, Questions to #211 |
| FND-002 | high | Topology: the §6.1 module DAG has an upward edge. E3 admits "dependency linked packages (I2)", and S3 `check` sits in layer 3, which depends on "2, F, K". The linked-package type and its reader belong to layer-4 `package` (side-input table: "QSL `package` (reader)"; layer 4 lists "S4, I2"). `package` depends on layer 3. For `check` to consume an I2 package it must import layer 4, which closes a `check` ⇄ `package` cycle. Mapping `complete::package` to "3 `library` / I2" does not fix this, because the owner of the I2 type is still `package`. #212 requires "The proposed crate/module DAG remains acyclic." Fix: pick one layout and state it in §6.1. Either (a) put a read-only imported-package view type (declarations, identities, source-map reference) in layer 3 `library`, and have layer-4 `package` produce it from an S4 package or from verified bytes; or (b) define the I2 port as a trait in layer 3 that layer 4 implements. Update the I2 row, the layer-3 and layer-4 "Depends on" cells, and the `complete::package` row to match. | ADR-011 §1 side inputs I2, §2.1 E3, §6.1 layers 3 and 4, §6.2 `complete::package` row |
| FND-003 | medium | Unstated failure modes at E9. (a) Source: E9 admits "the S4 package of the same identity", but the ADR does not say how the CG replay adapter obtains an in-process S4 package. It could recompile from source through S0–S4, or read it through I2 (see FND-001). (b) An identity mismatch between the replayed package and the proved package has no row in §2.3. (c) The S6a entry also takes an `ObjectEnvironment` and a `&mut Meter`. E9 does not say where the replay object environment comes from or what budget the meter has. (d) S6a can return `InputRefusal`, `Outcome::Refused`, `Undefined` or `Incomplete`. §2.3 covers only decode refusal and disagreement, so it does not say which of these outcomes give a parity verdict and which give `inconclusive`. (e) The packet's `witness: None` case is not covered. Fix: extend the E9 row in §2.1 with its package source and the replay object environment and meter budget. Add E9 rows to §2.3: an identity mismatch refuses with a typed cause and no verdict. An S6a `Incomplete` or `InputRefusal` is `inconclusive` with a typed cause and never counts as agreement. `witness: None` follows AD-016 arrow 7's WP9 carrier field and is never reported as reproduced with an evaluated witness. Leave the envelope types to #231. | ADR-011 §2.1 E9, §2.3 E9; AD-016 arrow 7 (entry signature, WP9 accounting cell) |
| FND-004 | medium | Evaluation purity: S6a is the reference semantics and the replay executor, so parity at S8 and the "identical verdict" in AD-016 arrow 7 assume S6a is deterministic. The ADR never says so. The family evaluators (`state`, `temporal`) and `simulation` join S6a in layer 5 with no constraint on effects or iteration order. Fix: add an S6a rule to §2 or §4. The result is a function of the package identity, arguments, object environment and meter budget only. S6a does no I/O, reads no clock, environment or randomness, and does not depend on hash-map iteration order. Every layer-5 module meets the same rule. Add an acceptance test that evaluates twice and gets equal `Evaluation` values. | ADR-011 §1 S6a, §6.1 layer 5, §2.2 E6 |
| FND-005 | medium | Entity identity: node-id uniqueness is assumed but not stated. The ADR keys the source map by checked node id (E3, E4, E9) and sets the obligation id to the clause node id (E7). Today `NodeKey` is a content digest of a canonical preimage (`QSL:src/value/node.rs:53,320-322`), and a graph that holds two nodes with the same key is refused (`:208`). If structurally identical subexpressions or clauses get the same id, one source-map key maps to two spans and two obligations share one identity. Across I2, two packages can also mint the same id. The ADR leaves the id scheme to #211, which is correct, but it does not state the uniqueness property its own edges depend on. Fix: in the §2.2 E3 Identity cell, state the requirement that a checked node id is unique per occurrence within a package, and that the pair (package identity, node id) is unique across I2 dependencies. Add this requirement to the #211 question list. | ADR-011 §2.2 E3, E4, E7, E9; `QSL:src/value/node.rs:53,208,320-322` |
| FND-006 | medium | Proof-gate acceptance has two different claim sets. Decision 8 and §2.3 rule 1 use "every module it claims". The next paragraph says a gate that "compiles a module under the prover but reaches no proposition in it" fails. If compiling a module counts as claiming it, every helper module the gate compiles, such as formatting or diagnostics in `quire-exact`, must be reached, which no gate can meet. If the gate declares its own claims, it can shrink the list to avoid the `unreached` failure. That is the RT #53 defect in a different form. Fix: in §2.3, say each gate declares its claimed-module list in a version-controlled file that #226's drift gate checks. A module the gate compiles but does not claim is reported as `not claimed` and never counted as evidence. A claimed module that is never reached fails the gate. The evidence report may not list a module outside the declared set. | ADR-011 Decision 8, §2.3 proof-stage acceptance, FB-10 |
| FND-007 | medium | Contradiction with AD-016: M-5 moves the S6a entry out of `value::expression` into a layer-5 `evaluate` module. AD-016 arrow 7 names the executor by full path, `quire_spec_language::value::expression::CheckedPackage::call`, and CG will take a normal dependency on that path in WP9. AD-016 does not mark the path `OPEN`, so the ADR's placement rule does not cover it. §9 OBS-030 and OBS-036 still give the old path. The decision itself (the QSL complete-V1 entry) does not change, only its path. Fix: choose one. (a) Keep `CheckedPackage::call` at the AD-016 path until the QSpec record changes, and say so in M-5. (b) Make M-5 carry a QSpec AD-016 arrow 7 path amendment in the same change set. Either way, update §9 OBS-030 and OBS-036 to name the entry by role, and add no re-export shim. | ADR-011 §6.2 `value::expression` rows, §7.3 M-5, §9 OBS-030 and OBS-036; AD-016 arrow 7 "Typed in → out" |
| FND-008 | medium | Worst-case structures: resource limits appear only at E1 ("Source bytes, `SourceIdentity`, limits"). The E1 on-error row covers only a CST with error nodes. It says nothing about a size or nesting limit being exceeded. S2 form building, S3 checking, E4 v2 emission, E5 reading and S6a evaluation all recurse over the tree, and no depth bound or limit refusal is stated for any of them. A deeply nested input can exhaust the stack in any stage after E1. Fix: say that the E1 limits include a nesting-depth bound, that a limit breach at E1 is a refusal with no CST, and that each later recursive stage refuses on its own limit with a typed cause and emits no partial output. #211 names the cause types. Add a limit-breach row to §2.3 for E1, E3 and E4. | ADR-011 §2.1 E1, §2.3 E1 to E5; AD-016 arrow 2 `CheckedPackageLimit` |
| FND-009 | medium | Topology of the package import graph: I2 lets a package import S4 packages, but the ADR states no failure mode for the import graph. The unstated cases are (a) a cycle, where A imports B and B imports A, which S3 cannot satisfy because each side needs the other's S4 output; (b) two different identities of the same package name reached through a diamond; and (c) an import that is missing from the library lock or cannot be resolved. The "one revision per crate" rule in §7.1 covers Rust crates, not QSL packages. Fix: add I2 rules to §1 or §2.3. An import cycle refuses at E3 with a typed cause that names the cycle. Two identities under one package name refuse, and no version is picked silently. A missing or unlocked import refuses. Hand the cause names to #211. | ADR-011 §1 I2, §2.1 E3, §7.1 rules |
| FND-010 | low | Internal faults have no outcome. §5 requires one total map from outcome kind to exit code, and every stage returns output or a refusal. A panic or broken invariant inside a stage is neither, and nothing says it must never be rendered as a refusal or a success. Fix: add to §5 that an internal fault is a defect, not an outcome kind. The CLI reports it with a reserved exit code that differs from every outcome kind's code. The library does not turn a fault into a typed refusal. | ADR-011 §2.3 intro, §5 bullets 2 and 3 |
| FND-011 | low | §6.1 has unclear edges. The layer-3 cell lists "`model` (with `model::intake`), `library`, then `check`" but gives no direction between `model` and `library`. The `tool` layer has no rank relative to layer 6, so it is unclear whether `command` ("every lower layer") may call `format`. Layer 5 omits F but will emit diagnostics. Fix: give the order inside layer 3 explicitly (for example, `library` depends on `model`), rank `tool` below layer 6 or state that `command` does not call it, and add F to layer 5's "Depends on" cell. | ADR-011 §6.1 layer table |

## Method

- Checklist, adapted to an architecture record:
  - Trust boundaries: the side inputs I1 to I3, the wire edges E5 and E9, the
    family hooks handed to #210, and the capability router (#185). For each,
    whether the ADR defines what admits the input and what happens when
    admission fails. The #210 hook failure question is already handed off
    ("how a missing hook fails"), so no finding was raised.
  - Entity identity: node id, declaration identity, package identity and
    obligation identity across E3 to E9.
  - Evaluation purity: S6a as reference semantics and replay executor, family
    evaluators, `simulation`.
  - Topology: the stage DAG, the §6.1 module layers, the §7.1 crate graph,
    and the QSL package import graph.
- AD-016 was read at `quire-specification` `origin/main`. Its arrows, crate
  graph, replay ownership, Shared-type row and owner decisions were checked
  against ADR-011 §§1, 2, 6, 7 and 9. The only contradiction found is
  FND-007. The §7.1 crate graph matches the AD-016 target after WP9 (CG → QSL
  becomes normal, and the QSL → CG and IR root → QSL edges are removed).
- Sibling ownership: no finding decides a #210 or #211 question. FND-001,
  FND-005, FND-008 and FND-009 ask ADR-011 to state a requirement and to hand
  the type or scheme choice to #211.
- Code check: `QSL:src/value/node.rs` at the worktree HEAD, for how `NodeKey`
  is minted and refused (FND-005).
- `spec/spec.md` index row for ADR-011 (line 388): the link, type and
  description are correct.

## Round 2 (HEAD 5cbd853)

Re-reviewed the revised ADR-011 at commit 5cbd853 against the round-1
findings. The `CheckedPackage` placement was checked against
`QSL:src/value/expression/mod.rs:71,452,650`. Today the type, its private
fields and `call` all live in `value::expression`.

### Resolution of round-1 findings

| ID | Status | Reason |
| --- | --- | --- |
| FND-001 | Resolved | §4 "Verified binding (I2 and E9)" defines the trust anchor: a supported v2 version, a recomputed digest equal to the declared identity, and an identity listed in the consumer's lock or pinned request. Otherwise the reader refuses with a named cause. "Never re-checked" is gone, and re-checking is routed to #211. FB-03 cites §4. |
| FND-002 | Resolved | §1 I2 row and §6.1 place the import view type in layer-3 `library` and the reader in layer-4 `package`. Orchestration passes the view into E3, and `check` never calls `package`. The `complete::package` row maps to `library` over import views. FND-012 records a typestate gap that this layout creates. |
| FND-003 | Partly | §2.1 E9 names the package source (the I2 reader), the identity-mismatch refusal and `witness: None`. The object environment and meter budget travel in the #231 envelope. Still open: nothing says an S6a `Incomplete`, `InputRefusal` or `Refused` result is never counted as agreement. The mapping is handed to the #231 envelope with no constraint. The §2.3 E6 rule only stops `Incomplete` from being read as `Completed`. |
| FND-004 | Resolved | §1 S6a row: the outcome is deterministic and has no side effects, over package, arguments, object environment and meter budget. Layer 5 is stage S6a, so `state`, `temporal` and `simulation` are bound too. No repeat-evaluation acceptance test is named. That is left to #213 and #219. |
| FND-005 | Resolved | §2.2 E3 Identity states one id per node occurrence and uniqueness of the (package identity, node id) pair across I2 views. #211 gets the cross-package key question. The current content-digest `NodeKey` becomes an input to #211, not an ADR defect. |
| FND-006 | Partly | §2.3 adds a checked-in claimed-module list, the `unreached` failure, a mutation control for each claimed module, and a census check by #226. Still open: no rule stops a gate from shrinking its list. Nothing says a compiled but unclaimed module is never evidence, and nothing sets a minimum claim for the CG generated-harness gate. The RT and `quire-exact` scopes are named under "Gate owners". Owner question 7 covers the CG gate. |
| FND-007 | Resolved | §4 and the §6.2 `value::expression` rows keep the AD-016 arrow 7 path `value::expression::CheckedPackage::call`. §9 OBS-030 and OBS-036 agree. That path names a layer-4 type from layer 5, which is a downward edge and allowed. |
| FND-008 | Resolved | The §2.3 "Limits" paragraph requires explicit limits at every stage entry, including nesting depth. A limit refusal names the limit, never truncates and is never success. Recursive stages bound depth by limit, not by the native stack. §5 gives limit refusal its own exit code. |
| FND-009 | Resolved | §1 "Package import graph rules (I2)" refuse a missing or unlisted import, two identities for one package, and any cycle, including a self-import. |
| FND-010 | Resolved | The §2.3 "Internal fault" paragraph and §5 bullet 3 make an internal fault a distinct kind with its own exit code. It is never reported as success or as refusal. |
| FND-011 | Resolved | §6.1 now lists modules in order (`semantic_value < model < library < check`), gives an exhaustive allow-list rule, ranks `tool` below layer 6, and includes F in layer 5. |

### New findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | medium | Typestate: the FND-002 layout conflicts with §4 constructor privacy. The Terms and §4 say the I2 reader is a checked producer, and that "the constructors of S3 and S4 output types are private to their stage modules", backed by `compile_fail` tests. The import view type is defined in layer-3 `library` but constructed by the layer-4 `package` reader. `pub(in path)` can only name an ancestor module, so for `package` to build a `library` type, the constructor must be at least `pub(crate)`. Then `check`, `command` and every other QSL module can also build an import view without the verified binding, which is the FB-03 bypass the binding exists to close. Fix: state where the verified binding runs. Either (a) the view's only constructor is in `library` and performs the §4 checks itself over F `digest` and `wire_format`, so `package` only supplies the bytes; or (b) `library` defines an I2 port trait that `package` implements, and the view is built only inside `library` from the port's verified output. Add the import view to the §4 `compile_fail` evidence. | ADR-011 Terms "Checked", §1 I2 row, §4 bullets 2 and 3, §6.1 layers 3 and 4, §7.3 M-4 |
| FND-013 | medium | Unstated type at E9: the I2 reader outputs two different things. The §1 I2 row and §7.3 M-4 say it yields a layer-3 "verified import view" ("v2 bytes → verified import view"). E6 and the S6a `CheckedPackage::call` entry admit an in-process S4 linked package. E9 and §10 row 9 say replay reads "the S4 package through the I2 reader" and runs S6a on it. The ADR never says the reader also yields an S4 in-process package, or that an import view can be evaluated. So the replay executor's input has no stated producer. That producer also needs the v2 wire to carry everything S6a evaluates, such as function bodies, measures and dispatch tables (today's private `CheckedPackage` fields, `QSL:src/value/expression/mod.rs:71-75`). Fix: in §1 and M-4, give the reader two outputs under the same §4 binding: an import view for E3 and an in-process S4 package for E9. Or name a separate layer-4 v2 → S4 reader for E9. Hand QSpec the question of whether v2 carries everything S6a needs to evaluate. | ADR-011 §1 I2 row, §2.1 E6 and E9, §7.3 M-4, §10 row 9; AD-016 arrow 7 |

### Round-2 verdict

ACCEPT WITH FINDINGS. Both round-1 high findings are resolved. FND-003 and
FND-006 are partly resolved, and each residual is a single sentence. FND-012
and FND-013 are new medium findings that the FND-002 layout introduced. Each
needs one placement sentence, not a new decision. Neither reopens AD-016 or
decides a #210 or #211 question. The owner questions are open by design and
were not re-litigated. The ADR designs no compatibility layer: every §7.3 row
has disposition "none", and the AD-016 path is kept as the contract, not as a
shim.
