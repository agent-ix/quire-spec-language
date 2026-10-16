---
id: TC-1040
title: "The population manifest decides whether a missing object is dangling or undecidable"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-364
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: verifies
---
# TC-1040: The population manifest decides whether a missing object is dangling or undecidable

## Description

Verify FR-364's population manifest (its rows, and that only an exhausted
traversal closes it), the five rules by which `resolve` classifies a missing
object, how navigation orders those decisions, condition 3's type check of an
absent full-reference end, the bundle key, and FR-106's snapshot-built
manifest.

Scope: FR-364-AC-10, FR-364-AC-11, FR-364-AC-12, FR-364-AC-13, FR-364-AC-14 and
FR-106-AC-39.

## Test Procedure

Use TC-1033's fixture **orders** and TC-1034's navigation notation and
charges. Rows 1 to 17 build the observation through the model-layer entry
with the enumerator below; rows 18 to 20 admit snapshot documents as in
TC-1035. Tag the tests `#[trace("TC-1040", "FR-364-AC-n")]`.

The enumerator yields one row per population declaration of the package, in
ascending declaration-key order, with the declared member types as covered
types and `complete` `true` exactly when the model layer binds the
population. The enumerator's row limit is the number of population declarations of the package. `p9`, `t9` and `v1` are
objects the observation does not hold.

Variants of **orders**:

- **spare**: a second declared population `Spare`, member type `Pump`, that the
  observation does not bind;
- **spare-bound**: **spare** with `Spare` bound holding no object;
- **spare-tank**: **spare** with the member type `Tank`;
- **valve**: an object type `Valve` that no declared population names, and the
  relationship `Pump-feeds-Valve`: source end role `feeds` (`Pump`, `{1,1}`),
  target end role `fed` (`Valve`, `{0,3}`), `source-to-target`;
- **parts**: `Pump` and `Tank` specialize a concrete object type `Part`, and
  the relationship `Sys-holds-Part`: source end role `holds` (`Sys`, `{1,1}`),
  target end role `heldBy` (`Part`, `{0,3}`), `source-to-target`.

| # | Step | Expected |
| --- | --- | --- |
| 1 | **orders**; read the manifest | closed, one row: `Q`, covered types `[Sys, Pump, Tank]`, complete |
| 2 | **spare**; read the manifest | closed, rows in ascending declaration-key order: `Q` complete and `Spare` (covered types `[Pump]`) not complete; **spare-bound**: both rows complete |
| 3 | **spare-bound**; the enumerator's pull count | closed: the enumerator is exhausted exactly at its row limit of 2, the package's declaration count, so a limit stop cannot occur |
| 4 | **spare-bound** with an enumerator that fails at its second pull | partial with the rows read and the failure as stop reason; the failure is not converted into a closed manifest, the only way a manifest is partial |
| 5 | `resolve(p9)`, **orders** | `dangling`: closed, `Q` covers `Pump` and is complete |
| 6 | `resolve(p1)` under the partial manifest of row 4; `resolve(p9)` under the same | `present`; then `undecidable` (rule 1) |
| 7 | **spare**; `resolve(p9)`; `resolve(t9)` | `undecidable` (rule 4: `Spare` covers `Pump` and is not complete, although `Q` is complete); then `dangling` (`Q` is the only row covering `Tank`) |
| 8 | **spare-bound**; `resolve(p9)` | `dangling` |
| 9 | **valve**; `resolve(v1)` | `undecidable` (rule 3: no row covers `Valve`) |
| 10 | **parts**; `resolve` of a missing reference of type `Part`; then of a missing `Pump` | `undecidable` (rule 3: `Q` declares `Pump` and `Tank`, so a declared child does not cover `Part`); then `dangling` |
| 11 | **valve**; `resolve` of a reference whose type is no object type of the model | `undecidable` (rule 2) |
| 12 | **orders**; composition `s1`-`p9`; `navigate(parts, s1)` | `refused { code: dangling_reference, cause: absent-target-in-complete-population }` for `p9` after `model.navigate` alone: one work unit |
| 13 | **spare**; composition `s1`-`p9`; `navigate(parts, s1)` | `incomplete { code: incomplete_population, cause: incomplete-scope }` retaining `p9` and the relationship, after `model.navigate` alone: one work unit |
| 14 | **spare**; composition `s1`-`p1`, `s1`-`p2`, `s1`-`p3`, `s1`-`p9` through `{0,3}`; `navigate(parts, s1)` | `incomplete-scope` for `p9` after `model.navigate` and three `collection.visit` (`p1`, `p2`, `p3`): four work units; no `above-maximum` refusal although the count is 4 |
| 15 | **parts** and **spare**; extent `Sys-holds-Part` `s1`-`p9`, `s1`-`t9`; `navigate(holds, s1)`; then the same with **spare-tank** | the first of `p9`, `t9` in ascending reference-key order decides, after `model.navigate` alone: in **spare** `p9` is undecidable and `t9` dangling, so `incomplete-scope` when the `Pump` reference sorts first and `dangling_reference` when the `Tank` reference sorts first; in **spare-tank** the two are swapped. The test sorts the two references with the reference-key comparator and asserts the matching result; across the two variants both results occur |
| 16 | **valve**; extent `Pump-feeds-Valve` `p1`-`v1`; `navigate(feeds, p1)` | `incomplete-scope` for `v1` after `model.navigate` alone |
| 17 | **orders**, each on a fresh observation; composition `s1`-`t9`; then `s1`-`p9`; then composition `s1`-`t1` | `invalid_runtime_input`/`wrong-value-kind` naming composition, end `target`, `Tank` against `Pump`, although `t9` is not held; then admitted; then `wrong-value-kind` naming `Tank` against `Pump` for the held `t1`; an instance whose reference type is no object type of the model gives `wrong-value-kind` retaining that type |
| 18 | snapshot **B** of TC-1035 over **spare**; the admitted observation | manifest closed with `Q` complete and `Spare` not complete; `resolve` of every stored end of both extents is `present`; with `Spare` listed `"complete": false` and empty, the same rows; with `Spare` listed `"complete": true` and empty, both rows complete |
| 19 | snapshot **B** with composition target `{"population": "ix://test/orders/Q", "key": "t9"}` (complete `Q`); then with `Q` listed `"complete": false` | `dangling_reference`/`absent-target-in-complete-population` at check 8, not `wrong-value-kind`, because a document end whose object is not listed has no type to check; then `Incomplete` with `incomplete_population`/`incomplete-scope` naming `Q` at check 7 |
| 20 | a bundle for source S, observation `pre` (selected clause C), built from the manifest and closure of S and the type environment of S′, where S′ has the same authority and identity as S and a digest differing in one byte; then the manifest of S with the closure of S selected under another clause; then the `pre` manifest (**spare**, `Spare` unbound) with the `post` closure of the same source and clause; then (snapshot-built) the manifest of snapshot document D with the closure of D′, a document of the same identity whose digest differs; then three parts of one key | `invalid_runtime_input`/`conflicting-identity` retaining part `types`, S's key and S′'s; then retaining part `closure`, the selected key and the differing one; then, retaining part `manifest`, the `pre` key and the `post` key (the observation role differs), so `resolve(p9)` is never read from a mixed pair that would give `dangling`; then retaining part `closure`, D's key and D′'s (the document digest differs); then a bundle whose `resolve(p9)` is `dangling` (**orders**) |

## Expected Results

`resolve` returns `dangling` only when the manifest was closed by exhausting
the package's declarations, a row covers the missing object's type and every
covering row is complete. Every other missing object is `undecidable`:
an incomplete or unbound covering population, a type no row covers, an unknown
type, and a partial manifest. A held object is always `present`. Navigation
stops at the first target that is not present and returns its kind of outcome
before any cardinality refusal. A full-reference end of the wrong type is
refused whether or not its object is held; a snapshot document end with no
object is decided by the closure rule. A manifest, closure or type environment
of another source or selection is refused before it can classify anything.
