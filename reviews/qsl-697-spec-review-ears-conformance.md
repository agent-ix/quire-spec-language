---
id: "SR-4952"
title: "ears-conformance review of QSL-697 reusable string-edge scanner"
type: "SpecReview"
analysis: "ears-conformance"
scope: "agent-ix/quire-spec-language@fa9e7e20bc179ff96dbe1474aba3822990599636; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md; spec/test-cases/TC-162-string-edge-scan-coverage.md; QSL-697"
review_set: "subset"
---

## Summary

Ticket: QSL-697. The scoped deterministic grammar check reports 2/2 documents grammar-clean and zero grammar findings. Semantic examination finds concrete actors and observable responses for request validation, scanning, ordering, rejection, installation, and adapter delegation. The broader full-spec check has 17 grammar advisories in unchanged documents; none applies to FR-064 or TC-162.

## Verdict

PASS for specification readiness. Implementation, installation, consumer adoption, and runtime acceptance remain pending. GitHub reports MERGEABLE with mergeStateStatus BLOCKED at the reviewed head. This establishes conflict-free mergeability only; repository merge gates remain for the lead to resolve.

## Validation

Installed tools: quoin 0.28.3; quire 0.36.2 (engine 0.50.2). Scoped validation of both changed artifacts passed; full `quire validate --scope . 'spec/**/*.md' --summary` passed for 1544 documents with 17 grammar advisories outside the reviewed files. `git diff --check` passed. Installed-module duplicate-archetype, inverse-edge and inline-data-schema advisories were observed. No applicable AssuranceProfile was found. No build, lock job, install or hosted workflow was run.

The whole-repository `quire matrix --scope . --format tsv` produced no matrix after several minutes and was interrupted (exit 130); this report uses direct inspection of the FR-064 trace tags and assertions, not a completed computed matrix. The skill example `quoin write --types SpecReview` failed because the installed CLI requires a repository argument; `quoin write . --types SpecReview --json` succeeded and supplied the installed schema and skeleton.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined Scope

```yaml
{
  "clean": true,
  "scope": [
    {
      "id": "FR-064-AC-1",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "Given a QSL source tree with one string comparison inside a function marked `#[string_edge]` and one string `match` inside an unmarked function, `xtask string-edge` reports exactly the unmarked occurrence, naming its file and line, and does not report the marked occurrence."
    },
    {
      "id": "FR-064-AC-2",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "Given an unmarked occurrence whose location matches an allow-list entry, `xtask string-edge` does not report it; removing that allow-list entry causes the same occurrence to be reported on the next scan, with no change to the source."
    },
    {
      "id": "FR-064-AC-3",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "`xtask string-edge` scans no file under a `tests/` directory and no `#[cfg(test)]` module; a string `match` placed only inside such a module is not reported even when unmarked and not allow-listed."
    },
    {
      "id": "FR-064-AC-4",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "`xtask string-edge` exits non-zero when its report is non-empty and exits zero when its report is empty; a test constructs one fixture tree of each shape and asserts both exit codes."
    },
    {
      "id": "FR-064-AC-5",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "Given one allow-list entry whose comparison result feeds an `if`/`match` condition that selects between two different code paths, and one entry whose comparison result feeds only a logged message, `xtask string-edge` rejects the first (naming its file and line) and accepts the second. A test attempts to add each of the five ADR-010 \u00a74.3 production dispatch sites (named in this requirement's Behavior) as an allow-list entry and asserts the tool rejects all five; a wrong implementation that allow-lists all five to force a clean scan does not satisfy this criterion."
    },
    {
      "id": "FR-064-AC-6",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "The lint gate's own target list (the `Makefile`) names `string-edge` as a prerequisite of the gate it runs in, whose own recipe runs `cargo run --package xtask -- string-edge`, and `xtask string-edge`'s non-zero exit propagates to that gate's own exit through `make`'s ordinary prerequisite-failure semantics -- a `.PHONY` aggregate target depending on a target whose recipe can fail. Both halves are checked directly: a grep-shaped check over the real `Makefile` confirms the prerequisite naming and the recipe, and a minimal fixture `Makefile` of the same aggregate/prerequisite shape demonstrates "
    },
    {
      "id": "FR-064-AC-7",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "Given two non-QSL crate source roots and a third unselected crate containing a violation, the library and binary report only occurrences in the selected roots with the same file, one-based line, item, branch classification and ordering. Repeating with equivalent absolute paths from another working directory produces identical occurrences. A named string constant in one selected crate does not classify a same-named non-string constant in the other as string dispatch."
    },
    {
      "id": "FR-064-AC-8",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "Given consumer rules and fixtures for marked intake, test exclusion, an allowed display-only comparison and a branch-gating comparison, the library and binary apply the same marker/exclusion/allow-list results as the QSL adapter. Removing the display entry restores its occurrence. Adding an entry at the branch-gating item causes a rule error, including when that item is marked."
    },
    {
      "id": "FR-064-AC-9",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "Empty, missing, non-directory, duplicate, overlapping or out-of-base roots; a selection with no production Rust files; unreadable or malformed source; malformed rules; and a source link escaping the base or forming a cycle cannot produce a clean result. Each returns a typed library error identifying its input and command exit 2. Rule failures include unknown or missing fields, wrong types, empty reasons and out-of-base entry paths."
    },
    {
      "id": "FR-064-AC-10",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "Installing `qsl-string-edge` using the documented authoritative Git route and running it outside QSL against a non-QSL crate succeeds without QSL-named fixture directories. The documented clean fixture exits 0. Replacing its production body with an unmarked literal-string dispatch exits 1 and names the expected consumer file and line. A separate Rust consumer imports the public scanner API through the documented Git dependency without copied scanner sources."
    },
    {
      "id": "FR-064-AC-11",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "QSL's adapter passes its roots and rules to the standalone library, with no second detector implementation. On clean, violating and invalid-input fixtures, its report content and success/failure agree with the library; its existing lint prerequisite still propagates failure. Extraction adds only the scanner crate's own tests to QSL verification."
    },
    {
      "id": "FR-064:Description:1",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "ADR-012 \u00a79 fixes one rule: a string may select semantics only at a listed\nedge (source lexing, a typed wire reader, a protocol-artifact or state-input\nintake reader, or CLI argument parsing), and at that edge one total\nconversion turns the string into a closed enum or a typed identity, or\nrefuses it with a typed cause. After the edge, no code compares a string to\nchoose behaviour. QSL SHALL provide a tool attribute that marks an edge\nfunction and a scan that reports every string comparison or string `match`\noutside a marked function in the selected crates' non-test code, so this rule is\nenforc"
    },
    {
      "id": "FR-064:Description:2",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "The authoritative scanner SHALL be a standalone Rust library and command\nthat consumers can obtain from this repository and run against their own\ncrate roots. QSL's `xtask string-edge` SHALL delegate to that library with\nQSL's roots and edge rules."
    },
    {
      "id": "FR-064:Inputs:1",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "- Explicit consumer crate source roots and a base directory for resolving\n  source paths, rule entries and report locations. QSL supplies its own roots\n  through its adapter.\n- The selected crates' source (non-test code; `#[cfg(test)]` items, files declared\n  by `#[cfg(test)] mod x;` and files under a `tests/` directory are out of\n  scope for the scan; a file is never out of scope by its name alone).\n- The `#[string_edge]` tool attribute applied to an edge function.\n- A checked-in allow-list of string comparisons that compare a user-supplied\n  value and select no behaviour on its content."
    },
    {
      "id": "FR-064:Outputs:1",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "- A library report and a command report listing every string comparison or string\n  `match` found outside a `#[string_edge]`-marked function and not covered by\n  the allow-list, with the file and line of each.\n- A command exit of zero only after a completed scan with no violations;\n  non-zero for violations or a scan that could not complete."
    },
    {
      "id": "FR-064:The marker attribute:1",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "`#[string_edge]` SHALL be a tool attribute applicable to a function. It\nSHALL carry no runtime behaviour: it exists only for `xtask string-edge` to\nread, and its presence or absence SHALL NOT change what the marked function\ncompiles to."
    },
    {
      "id": "FR-064:What the scan reports:1",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "`xtask string-edge` SHALL scan every QSL crate's non-test source for:"
    },
    {
      "id": "FR-064:What the scan reports:2",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "- an equality or ordering comparison between a `&str`/`String` value and a\n  string literal or another `&str`/`String` value, and\n- a `match` expression whose scrutinee is a `&str`/`String` value,"
    },
    {
      "id": "FR-064:What the scan reports:3",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "and SHALL report each occurrence found outside a function carrying\n`#[string_edge]`, unless that occurrence is named in the allow-list."
    },
    {
      "id": "FR-064:The allow-list:1",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "The allow-list SHALL be a checked-in file naming each allowed occurrence by\nfile and line (or by a stable anchor if the file is expected to move), with a\none-line reason. An allow-list entry SHALL name only a comparison of a\nuser-supplied value that selects no program behaviour based on its content\n-- for example, a `debug_assert_eq!` in production code that compares a\nrendered user-supplied value against its expected display text purely as an\ninternal consistency check, compiled out of release builds and reachable by\nno code path in a release binary. An entry that gates a branch, a lookup or\n"
    },
    {
      "id": "FR-064:The allow-list:2",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "`xtask string-edge` SHALL reject an allow-list entry whose comparison result\nfeeds an `if` condition or a `match` scrutinee that selects between\ndifferent code paths, reporting the entry's file and line and refusing to\ntreat it as allowed; it SHALL accept an entry only when the comparison's\nresult feeds a non-branching sink (a logged or displayed message, a\ndiagnostic payload, a test assertion, or code compiled out of the checked\nbuild). None of the five production dispatch sites ADR-010 \u00a74.3 names (the\n`\"allocation\"` relationship-category string, the\n`\"quire.protocol.finite-global/v1\"` profil"
    },
    {
      "id": "FR-064:Gate placement:1",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "`xtask string-edge` SHALL run in the lint gate."
    },
    {
      "id": "FR-064:Reusable consumer interface:1",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "The standalone package and binary SHALL be named `qsl-string-edge`. Its\nlibrary SHALL accept a request containing a base directory, one or more\ncrate source directories, and edge rules. Each source directory defines one\ncrate's constant-resolution scope; supplying multiple roots SHALL NOT make\na constant declared in one crate visible to the detector in another crate.\nThe scanner SHALL use only the caller's supplied roots and rules, without\nrequiring QSL crate names, a QSL checkout as the current directory, or a\nQSL compiler/runtime dependency. Each root denotes the source directory\nitself, suc"
    },
    {
      "id": "FR-064:Reusable consumer interface:2",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "The command SHALL expose the same request as:"
    },
    {
      "id": "FR-064:Reusable consumer interface:3",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "```text\nqsl-string-edge --base-dir <directory> --crate-root <source-directory> [--crate-root <source-directory> ...] --rules <rules.json>\n```"
    },
    {
      "id": "FR-064:Reusable consumer interface:4",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "`--base-dir` SHALL resolve relative to the invocation's current directory;\nrelative crate-root and rules-file paths SHALL resolve relative to that base.\nAbsolute inputs SHALL be accepted. A source root SHALL be contained within\nthe base directory after path resolution. The scanner SHALL reject a source\nfile or directory link that escapes the base, and SHALL NOT follow a\ndirectory-link cycle. Reports SHALL identify files relative to the base\nwith `/` separators, using one-based source lines and the enclosing item\nanchor. Equivalent absolute and relative inputs SHALL produce the same\nordered occ"
    },
    {
      "id": "FR-064:Reusable consumer interface:5",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "The library SHALL return a structured report containing the occurrences\n(file, line, item and branch-gating classification). It SHALL return a typed\nerror identifying the affected input when a request, rule, source read or\nRust parse fails. The command SHALL exit `0` for a completed clean report,\n`1` for a completed report containing violations, and `2` for a request,\nrule, read or parse failure. An empty root list, missing or non-directory\nroot, or selection containing no production Rust files SHALL be a request\nfailure. A failed or partial scan SHALL NOT emit a clean result. Both\nentry point"
    },
    {
      "id": "FR-064:Consumer edge rules:1",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "The command SHALL read edge rules from a strict JSON object with an\n`allow_list` array; an empty array is an explicit empty allow-list:"
    },
    {
      "id": "FR-064:Consumer edge rules:2",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "```json\n{\"allow_list\": [{\"file\": \"src/display.rs\", \"item\": \"display\", \"reason\": \"Display-only comparison of supplied text\"}]}\n```"
    },
    {
      "id": "FR-064:Consumer edge rules:3",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "Each entry SHALL contain a base-relative file path, an enclosing item anchor\nand a non-empty reason. These are the stable-anchor alternative already\nallowed above. Unknown fields, wrong field types, missing fields, malformed\nJSON and paths outside the base SHALL be rule errors. The library SHALL\naccept the equivalent typed rule value without requiring a rules file.\nThe marker remains `#[string_edge]`, including its qualified form\n`#[qsl_attrs::string_edge]`; consumers that compile marked functions SHALL\nobtain the existing `qsl-attrs` crate by reference to the same authoritative\nrepository. A "
    },
    {
      "id": "FR-064:Consumer edge rules:4",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "Consumer rules SHALL obey the allow-list restrictions above. An entry\ncovering any detected branch-gating occurrence in its item SHALL be refused\nas a rule error, even if that item is marked. The scanner SHALL check that\ncondition before applying marker or allow-list suppression. QSL-specific\nroots and rules SHALL reside in QSL's adapter; the reusable scanner SHALL\nNOT import them as defaults. Consumers own their edge inventories and\nchoice of CI invocation."
    },
    {
      "id": "FR-064:Installation and acceptance boundary:1",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "The consumer documentation SHALL give this installation route from the\nauthoritative private source, using existing repository access:"
    },
    {
      "id": "FR-064:Installation and acceptance boundary:2",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "```text\ncargo install --git https://github.com/agent-ix/quire-spec-language --branch main qsl-string-edge\n```"
    },
    {
      "id": "FR-064:Installation and acceptance boundary:3",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "A library consumer SHALL be able to use the same package through a Cargo\ngit dependency with `branch = \"main\"`. Neither route SHALL require copying\nscanner sources, maintaining a scanner fork, or building QSL's `xtask` as\nthe consumer command. The documented consumer example SHALL run the\ninstalled binary from outside QSL against a non-QSL Rust crate, with its\nown rule file, and show both a clean run and an unmarked dispatch failure.\nThe implementation acceptance run SHALL execute that install and both\ninvocations; prose examples alone do not establish the route works."
    },
    {
      "id": "FR-064:Installation and acceptance boundary:4",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "QSL's adapter SHALL retain the existing `string-edge` lint prerequisite\nand propagate scanner failures. Extraction SHALL add no QSL gate beyond\nthe scanner crate's own tests. Consumer adoption in IR, CG and driver is\nseparate work. Reuse preserves the existing structural detector's coverage\nand documented limitations; it does not claim to resolve variable-to-variable\ncomparisons, external constants or indirect branch dataflow."
    },
    {
      "id": "FR-064:Dependencies:1",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) \u00a79\n  (string dispatch rule and the edge table, which cites ADR-010 \u00a74.3's five\n  production dispatch sites this requirement's allow-list rejection names).\n- [FR-062](FR-062-implement-checked-family-contract.md).\n- [US-006](../usecase/US-006-extend-a-family-without-breaking-seams.md).\n- agent-ix/quire-contract-ir#141 and agent-ix/quire-contract-codegen#86 run\n  the same scan in their own repositories, over the same rule; this\n  requirement builds the attribute and the scan tool that #141 and #86\n  invoke, not their own edg"
    },
    {
      "id": "FR-064:Status:1",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "The consumer interface and acceptance criteria FR-064-AC-7 through\nFR-064-AC-11 are specified for implementation. The existing QSL-only\nevidence below does not establish consumer installation or adoption."
    },
    {
      "id": "FR-064:Status:2",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "**Implemented, with a stated residual.** `#[string_edge]` and `xtask\nstring-edge` are built, `string-edge` is a prerequisite of `make ci` (the\nlint gate), and `cargo xtask string-edge` reports no unmarked, unlisted\noccurrence over the whole workspace. The detector resolves both a string\nliteral and a comparison, method call or `match` arm against a named\nsame-crate `const NAME: &str` (the string-value clause): `const\nNAME: &str` at module, `impl` and trait scope, matched by its last path\nsegment. Every occurrence the scan finds is either a `#[string_edge]`-marked\nedge (an intake reader, a type"
    },
    {
      "id": "FR-064:Status:3",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "Residual: the Behavior clause asks for a report on any comparison between a\n`&str`/`String` value and *any other* such value, and any `match` whose\nscrutinee is a string. The detector covers a literal and a same-crate named\nconstant on one side; it does not resolve a comparison between two bindings\n(for example `adapter.observation_contract_revision != offered\n.observation_contract_revision`, neither side a literal or a named\nconstant) or a constant defined in another crate. Both shapes are\nundetected today, not merely unmarked -- the scan cannot see them to report\nthem. Extending the detector"
    },
    {
      "id": "FR-064:Status:4",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "A re-measurement against a clean scan found 53 named-constant\noccurrences across roughly 35 functions the literal-only scan could not see.\nWire, contract-version and format checks (the large majority) are marked\n`#[string_edge]`, each a genuine one-shot admission edge. Two sites got a\nreal typed conversion rather than a mark, because a wire enum already\nexisted to decode into instead of comparing forms directly:\n`qsl-semantics/src/check/lowering.rs`'s `name == FUNCTION_PARAMETERS` reads\nthe parameters binding positionally (it is always the function node's first\nmember, by construction, not mer"
    },
    {
      "id": "FR-064:Status:5",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "`qsl-semantics/src/check/claims.rs`'s `identity == NARROW`/`SCALAR_FAMILIES`\ncompares (SR-758 FND-005) are marked `#[string_edge]`\nthrough one `operation_role` classifier, corrected here from an earlier\nclaim that this was a typed conversion: `operation_role` re-derives the\nclassification from the identity string on every call; nothing resolves it\nonce and threads a stored value. Mutation-testing the two checks that read\nit (`claims.rs`'s narrow lookup and `emit.rs`'s `forced_absent` `EnumValue`\narm, SR-758 FND-006) found neither is distinguished by any fixture in the\ntree today; both are kept"
    },
    {
      "id": "FR-064:Status:6",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "Detector scope: a comparison is branch-gating when it feeds an\n`if`/`while` condition or `match` scrutinee/guard, is a term of a `&&`/`||`\nchain, is a match arm's own value, or is a `strip_prefix` call."
    },
    {
      "id": "FR-064:Status:7",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "**By Acceptance Criterion, with the tracking tags in\n`xtask/src/string_edge.rs` (all `TC-162`):**\n- FR-064-AC-1: backed.\n- FR-064-AC-2: backed\n  (`allow_listed_occurrence_is_silent_removing_the_entry_reports_it_again`).\n- FR-064-AC-3: backed (`cfg_test_module_is_not_scanned`,\n  `file_level_test_modules_are_skipped_only_when_declared_cfg_test`).\n- FR-064-AC-4: backed\n  (`a_non_empty_report_exits_non_zero_a_clean_scan_exits_zero`).\n- FR-064-AC-5: backed for the sites in the tree. The two-entry and\n  five-site-shape halves are synthetic fixtures tagged `FR-064-AC-5`, and\n  `named_string_consts_ar"
    },
    {
      "id": "FR-064:Status:8",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "The original six acceptance criteria have the evidence listed above;\nconsumer acceptance remains separate, as do the detector residuals above."
    },
    {
      "id": "TC-162:Description:1",
      "path": "spec/test-cases/TC-162-string-edge-scan-coverage.md",
      "role": "examined",
      "excerpt": "Verify [FR-064](../functional/FR-064-restrict-string-dispatch-to-marked-edges.md)\nthrough both QSL's adapter and the reusable scanner's library and command.\nThe scan reports unmarked string dispatch, respects valid consumer rules,\nexcludes test code, rejects branch-gating allow-list entries and preserves\nsource locations. Scope: FR-064-AC-1 through FR-064-AC-11."
    },
    {
      "id": "TC-162:Description:2",
      "path": "spec/test-cases/TC-162-string-edge-scan-coverage.md",
      "role": "examined",
      "excerpt": "The consumer procedures below are acceptance obligations for the extraction;\nexisting QSL-only tests do not establish consumer installation or adoption.\nThe structural detector covers literals and same-crate named string constants.\nVariable-to-variable comparisons, external constants and indirect branch\ndataflow retain the limitations stated by FR-064."
    },
    {
      "id": "TC-162:Test Procedure:1",
      "path": "spec/test-cases/TC-162-string-edge-scan-coverage.md",
      "role": "examined",
      "excerpt": "1. Build a fixture QSL-shaped source tree with one string comparison inside\n   a `#[string_edge]`-marked function and one string `match` inside an\n   unmarked function; run `xtask string-edge` against it.\n2. Add a non-empty allow-list with the unmarked occurrence's file and item\n   anchor and a display-only reason; re-run the scan. Remove the entry and\n   re-run again. Use a separate comparison whose result only feeds a\n   displayed message for this step, so it meets the allow-list rule.\n3. Move the unmarked occurrence from step 1 into a `#[cfg(test)]` module,\n   into a file under a `tests/` d"
    },
    {
      "id": "TC-162:Worked external consumer acceptance:1",
      "path": "spec/test-cases/TC-162-string-edge-scan-coverage.md",
      "role": "examined",
      "excerpt": "The implementation's acceptance test creates a non-QSL crate outside the QSL\ncheckout, for example at `/tmp/string-edge-consumer`, with this manifest:"
    },
    {
      "id": "TC-162:Worked external consumer acceptance:2",
      "path": "spec/test-cases/TC-162-string-edge-scan-coverage.md",
      "role": "examined",
      "excerpt": "```toml\n[package]\nname = \"consumer-example\"\nversion = \"0.1.0\"\nedition = \"2021\"\n```"
    },
    {
      "id": "TC-162:Worked external consumer acceptance:3",
      "path": "spec/test-cases/TC-162-string-edge-scan-coverage.md",
      "role": "examined",
      "excerpt": "Its `edge-rules.json` is `{\"allow_list\": []}`. Its clean `src/lib.rs` is:"
    },
    {
      "id": "TC-162:Worked external consumer acceptance:4",
      "path": "spec/test-cases/TC-162-string-edge-scan-coverage.md",
      "role": "examined",
      "excerpt": "```rust\npub enum Kind { Accepted, Other }\npub fn accepted(kind: Kind) -> bool { matches!(kind, Kind::Accepted) }\n```"
    },
    {
      "id": "TC-162:Worked external consumer acceptance:5",
      "path": "spec/test-cases/TC-162-string-edge-scan-coverage.md",
      "role": "examined",
      "excerpt": "The acceptance run installs and invokes the actual binary using existing\naccess to the private authoritative repository:"
    },
    {
      "id": "TC-162:Worked external consumer acceptance:6",
      "path": "spec/test-cases/TC-162-string-edge-scan-coverage.md",
      "role": "examined",
      "excerpt": "```sh\ncargo install --git https://github.com/agent-ix/quire-spec-language --branch main qsl-string-edge\ncd /tmp/string-edge-consumer\nqsl-string-edge --base-dir . --crate-root src --rules edge-rules.json\n```"
    },
    {
      "id": "TC-162:Worked external consumer acceptance:7",
      "path": "spec/test-cases/TC-162-string-edge-scan-coverage.md",
      "role": "examined",
      "excerpt": "It then replaces `src/lib.rs` with this one-line production violation and\nrepeats the same scan command:"
    },
    {
      "id": "TC-162:Worked external consumer acceptance:8",
      "path": "spec/test-cases/TC-162-string-edge-scan-coverage.md",
      "role": "examined",
      "excerpt": "```rust\npub fn accepted(kind: &str) -> bool { match kind { \"accepted\" => true, _ => false } }\n```"
    },
    {
      "id": "TC-162:Worked external consumer acceptance:9",
      "path": "spec/test-cases/TC-162-string-edge-scan-coverage.md",
      "role": "examined",
      "excerpt": "The separate Rust library client's manifest references the authoritative\npackage using:"
    },
    {
      "id": "TC-162:Worked external consumer acceptance:10",
      "path": "spec/test-cases/TC-162-string-edge-scan-coverage.md",
      "role": "examined",
      "excerpt": "```toml\n[dependencies]\nqsl-string-edge = { git = \"https://github.com/agent-ix/quire-spec-language\", branch = \"main\" }\n```"
    },
    {
      "id": "TC-162:Worked external consumer acceptance:11",
      "path": "spec/test-cases/TC-162-string-edge-scan-coverage.md",
      "role": "examined",
      "excerpt": "A consumer compiling marked intake functions uses the existing attribute\ncrate by reference as well:"
    },
    {
      "id": "TC-162:Worked external consumer acceptance:12",
      "path": "spec/test-cases/TC-162-string-edge-scan-coverage.md",
      "role": "examined",
      "excerpt": "```toml\n[dependencies]\nqsl-attrs = { git = \"https://github.com/agent-ix/quire-spec-language\", branch = \"main\" }\n```"
    },
    {
      "id": "TC-162:Worked external consumer acceptance:13",
      "path": "spec/test-cases/TC-162-string-edge-scan-coverage.md",
      "role": "examined",
      "excerpt": "These commands are the acceptance recipe, not a claim that the standalone\npackage has already been implemented or the installation run has passed."
    },
    {
      "id": "TC-162:Expected Results:1",
      "path": "spec/test-cases/TC-162-string-edge-scan-coverage.md",
      "role": "examined",
      "excerpt": "- Steps 1 and 3: only the unmarked production occurrence is reported, with\n  its file and one-based line. Production files named `tests.rs` remain in\n  scope; declared test modules and test directories do not.\n- Step 2: adding the valid display-only entry suppresses its occurrence;\n  removing the entry restores it without changing source.\n- Step 4: the clean report exits zero and the violating report exits non-zero.\n- Steps 5 and 6: branch-gating entries are rejected with source locations;\n  display-only entries are accepted. Every named production dispatch shape\n  is rejected as an allow-list"
    },
    {
      "id": "US-006",
      "path": "spec/usecase/US-006-extend-a-family-without-breaking-seams.md",
      "role": "context_only",
      "excerpt": "**As a** QSL contributor adding a new semantic variant to a family\n**I want** the build to fail immediately, at every place that must handle my\nnew variant, if I forget to add a required arm\n**So that** I cannot ship a family extension that silently falls through an\nunhandled case, and I cannot reintroduce a string comparison where a closed\ntype or a marked edge belongs.\n\n## Context\n\nBefore this ticket, whether every dispatch site over a family's enums has an\narm for a given variant depends on the reviewer noticing a missing arm by\nreading the diff; nothing forces a build failure. Likewise, wh"
    },
    {
      "id": "ADR-012 \u00a79",
      "path": "spec/decisions/ADR-012-semantic-family-extension-contracts.md",
      "role": "context_only",
      "excerpt": "Rule: a string may select semantics only at a listed edge. At an edge, one\ntotal conversion turns it into a closed enum or a typed identity, or refuses\nit with a typed cause. A typed identity over an open set (a `BackendId`) is\nresolved by a lookup in the registry value, and an unknown name is refused\nwith a cause naming it. After the edge, no code compares a string to choose\nbehaviour.\n\nThe edges are:\n\n- source lexing, where keywords become closed leading-token kinds;\n- the typed v2 reader and the other wire readers named in the table;\n- the protocol-artifact and state-input intake readers;\n-"
    },
    {
      "id": "FR-062",
      "path": "spec/functional/FR-062-implement-checked-family-contract.md",
      "role": "context_only",
      "excerpt": "QSL semantic families (`Value`, `StateModel`, `SumCase`, `TemporalTrace`,\n`ProtocolClause`, `Relation`) each own their own grammar, checked nodes and\ndiagnostics, but check, requirement-derivation and evaluation are reached\nthrough the same shape for every family, and every family's checked nodes\nreach the checked package through the same S4 v2 emitter. QSL SHALL\nimplement one shared contract, with the parts below, that every family\nimplements once and that no family bypasses.\n\nThe contract's six parts (ADR-012 \u00a72):\n\n1. **Identity.** Every checked node the contract's `check` produces SHALL\n   "
    }
  ],
  "findings": [],
  "bindings": []
}
```
