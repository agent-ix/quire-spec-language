// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-060 (ADR-011 §3 FB-05; ADR-013 O-04, O-05; #215 T-12): the reusable
//! API-surface check.
//!
//! Each rule names a symbol (a constructor or a facade module) and the
//! module prefixes allowed to call it. The `quire-exact` crate defines the
//! kernel `EffectiveId`/`NodeKey` types; T12-B, T12-C and T12-D each match
//! that type's `from_digest` constructor, and `src/value/node.rs` re-exports
//! the kernel `NodeKey` type rather than defining its own.
//!
//! The scan is textual: it looks for each rule's declared `call_patterns`
//! substring in a `.rs` file's own module and does not resolve `use ... as`
//! renames or macro expansion, nor does it exclude a match found inside a
//! comment or string literal. Most rules match `Type::method(` call syntax;
//! T12-B's pattern is the bare path `NodeKey::from_digest`, which also
//! catches the constructor passed as a value rather than called (see T12-B's
//! own comment, below). These are stated limitations, not a silent gap: a
//! caller that imports a rule's symbol under another name, or a comment that
//! happens to quote a call pattern, is not distinguished from a real call
//! site by this version of the check. `main.rs`'s printed report states both
//! limitations.
//!
//! Each rule scans one *role*'s source tree (see [`Role`]): T12-B, T12-C and
//! T12-D are QSL-side rules (which QSL module calls the kernel constructor),
//! scanned against the QSL tree passed with `--qsl`; T12-A is a CG-side rule (does CG
//! call only QSL's `qsl-replay` facade crate), scanned against the CG tree
//! passed with `--cg` -- never against QSL's own tree, which contains no CG
//! call sites for the rule to find (#249 review, HIGH-2/MEDIUM-4). A rule
//! whose role's tree was not supplied reports [`RuleStatus::NeedsRoot`]; the
//! other rules are still evaluated.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::error::{Code, Error, Result};

/// One call site of a rule's symbol, named by the module that contains it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CallSite {
    pub(crate) file: PathBuf,
    pub(crate) line: usize,
    pub(crate) module: String,
}

/// Whether a rule could be evaluated. A rule the tool cannot evaluate is
/// reported as `Pending` or `NeedsRoot`, never as a silent pass.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RuleStatus {
    Live,
    /// The rule's `requires_path` marker is absent from the QSL tree.
    Pending(&'static str),
    /// The rule's target exists, but no tree was supplied for its role, so
    /// nothing was scanned. Not a pass.
    NeedsRoot(Role),
}

/// Which repository's tree a rule scans for call sites. `requires_path` is
/// always checked against the QSL tree (every T-12 target lands in QSL
/// first), but the scan itself runs over whichever role names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Role {
    /// Scan QSL's own tree: which QSL module calls a QSL-defined constructor.
    Qsl,
    /// Scan the CG tree: does CG call QSL only through the stated facade.
    Cg,
}

impl Role {
    /// The CLI flag that supplies this role's scan root (`main.rs`).
    pub(crate) fn flag_name(self) -> &'static str {
        match self {
            Role::Qsl => "--qsl",
            Role::Cg => "--cg",
        }
    }
}

/// One ADR-011 T-12 API-surface rule (data, per the module doc above).
pub(crate) struct Rule {
    pub(crate) id: &'static str,
    pub(crate) description: &'static str,
    pub(crate) role: Role,
    /// Call-site substrings that identify a use of the rule's symbol. Most
    /// rules match call syntax, for example `"EffectiveId::from_digest("`;
    /// T12-B matches the bare path `"NodeKey::from_digest"` instead (see its
    /// own comment, below).
    pub(crate) call_patterns: &'static [&'static str],
    /// Call-site substrings that are a violation from any module, including
    /// an allowed caller: a path that no longer names the rule's symbol.
    pub(crate) forbidden_patterns: &'static [&'static str],
    /// Module path prefixes allowed to contain a call site (matched as
    /// `module == prefix` or `module.starts_with("{prefix}::")`).
    pub(crate) allowed_caller_prefixes: &'static [&'static str],
    /// A file path, relative to the QSL tree, whose presence the rule needs
    /// before it is live. `None` means the rule is always live once a root is
    /// given (the constructor rules below: the files that define `NodeKey`,
    /// `EffectiveId` and `PopulationId`'s minting site already exist on
    /// origin/main).
    pub(crate) requires_path: Option<&'static str>,
    pub(crate) pending_reason: &'static str,
    /// A fixed note on scope this rule does not evaluate, printed alongside
    /// every status regardless of whether the scan it does run passes,
    /// fails, or is pending -- not folded into `pending_reason`, because it
    /// applies unconditionally, not only while the rule is pending (#249
    /// review round 2 H-1). `None` for a rule with no such gap.
    pub(crate) scope_note: Option<&'static str>,
}

/// today's four T-12 rules (ADR-011 §3 FB-05; ADR-013 O-04, O-05, O-13/QC-21).
pub(crate) const RULES: &[Rule] = &[
    Rule {
        id: "T12-A",
        description: "CG calls the QSL layer-6 `qsl-replay` facade only (ADR-011 §3 FB-05, §2.1 E9)",
        role: Role::Cg,
        // The facade is the `qsl-replay` workspace crate (ADR-011 §6.1 layer 6).
        call_patterns: &["qsl_replay::"],
        // The root crate has no `replay` module, so a call spelled this way
        // cannot reach the facade; it is a finding, never a pass.
        forbidden_patterns: &["quire_spec_language::replay::"],
        // The facade's own internal adapter module has no ticket-assigned
        // name yet (ADR-011 places it in CG, "with RT ops and IR outcome",
        // #217/#219 build it). Left as a placeholder for #213/#217 to set.
        allowed_caller_prefixes: &["replay"],
        requires_path: Some("qsl-replay/src/lib.rs"),
        pending_reason: "the layer-6 facade crate `qsl-replay/src/lib.rs` is absent from the \
                         --qsl tree",
        scope_note: None,
    },
    Rule {
        id: "T12-B",
        description: "only `check` calls the kernel `NodeKey` constructor (ADR-013 O-04)",
        role: Role::Qsl,
        // `quire_exact::NodeKey::from_digest` is the kernel `NodeKey`'s one
        // public constructor. The pattern is the bare path
        // `NodeKey::from_digest`, not the call form `NodeKey::from_digest(`:
        // a bare-path match also catches the constructor passed as a value
        // (`.map(NodeKey::from_digest)`, `value::node::NodeIdDocument::key`),
        // which a call-form-only pattern misses. `node_key_of` is the
        // crate-internal helper both `value::enumeration` and `value::unit`
        // mint through, calling `from_digest` in turn -- scanning for it
        // directly, rather than only its callee, is what surfaces those two
        // modules' own minting sites (R1, #249 review, review item 7).
        call_patterns: &["NodeKey::from_digest", "node_key_of("],
        forbidden_patterns: &[],
        // ADR-011 §1 stage table, S3 row: "QSL check (today: value::expression
        // check, model::checked_dispatch, value::library)". `model::
        // checked_dispatch` -> `check::checked_dispatch` (FR-074, ADR-011
        // §7.3 M-2, QSL-7, 2026-09-21) realised ADR-011:694-697's end state
        // ("only `check` calls the kernel `NodeKey` constructor") for that
        // one caller. `check::identity` (QSL-158 S-3b) is also an
        // allow-listed `check` submodule; its own `from_digest` call today
        // is test-only (identity.rs:630, inside `#[cfg(test)]`).
        //
        // `value::node` re-exports the kernel `NodeKey` and is not
        // allow-listed: `node_key_of`'s own `from_digest` mint and
        // `NodeIdDocument::key`'s wire-digest-string parse-then-wrap (both
        // in `src/value/node.rs`) are real production call sites outside
        // this rule's allow-list (FR-060 T12-B's named-debt list, entry
        // `value::node::NodeIdDocument::key`), not this rule's sanctioned
        // path.
        allowed_caller_prefixes: &[
            "value::expression::check",
            "check::checked_dispatch",
            "check::identity",
        ],
        requires_path: Some("src/value/node.rs"),
        // Genuinely unreachable for the same reason as T12-C's, below:
        // `src/value/node.rs` already exists on origin/main as a `pub use
        // quire_exact::NodeKey` re-export rather than a definition, but the
        // marker path's presence is all this check tests.
        pending_reason: "unreachable: src/value/node.rs already exists on origin/main",
        // ADR-013 O-04/O-05, DA-02 name `NodeKey`/`EffectiveId` minting as a
        // system-wide, single-constructor invariant, but today (pre-#213
        // kernel extraction) quire-contract-runtime defines its own,
        // independent `NodeKey` type (`rt/src/exact/node.rs`), called from
        // RT's own tests and generated by quire-contract-codegen's own
        // templates (e.g. `rt::NodeKey::from_bytes(...)` in
        // `composite_equality.rs`). This rule scans QSL's own tree only
        // (Role::Qsl); ADR-013 does not name an allowed-caller-module
        // mapping for RT's or CG's own copies, so scanning them is not
        // decided here -- escalated to whoever owns #213.
        scope_note: Some(
            "scoped to QSL's own tree only; does not scan quire-contract-runtime's \
             independently defined NodeKey type or quire-contract-codegen's generated call \
             sites -- ADR-013 does not name an allowed-caller mapping for either, not decided \
             here (#213)",
        ),
    },
    Rule {
        id: "T12-C",
        description: "only `model` calls the kernel `EffectiveId` constructor (ADR-013 O-05)",
        role: Role::Qsl,
        // The kernel's real constructor (`quire-exact`'s `EffectiveId::
        // from_digest`, #213 S-1/S-2), not the pre-migration
        // `from_digest_bytes` name this rule matched before S-2 retired
        // `model::key`'s own `EffectiveId` struct in favor of re-exporting
        // the kernel type.
        call_patterns: &["EffectiveId::from_digest("],
        forbidden_patterns: &[],
        allowed_caller_prefixes: &["model"],
        requires_path: Some("src/model/key.rs"),
        // Genuinely unreachable for the same reason as T12-B's, above:
        // `src/model/key.rs` already exists on origin/main (it now
        // re-exports the kernel `EffectiveId` rather than defining it, but
        // the marker path's presence is all this check tests).
        pending_reason: "unreachable: src/model/key.rs already exists on origin/main",
        scope_note: Some(
            "scoped to QSL's own tree only; does not scan quire-contract-runtime's or \
             quire-contract-codegen's own copies of this identity's shape -- ADR-013 does not \
             name an allowed-caller mapping for either, not decided here (#213)",
        ),
    },
    Rule {
        id: "T12-D",
        description: "only `model` calls the kernel `PopulationId` constructor (ADR-013 QC-21)",
        role: Role::Qsl,
        // The kernel's real constructor (`quire-exact`'s `PopulationId::
        // from_digest`, QSL-131 Slice B). No mint call site exists yet on
        // origin/main (QSL `model` minting a `PopulationId` at admission
        // time is QSL-131's other half), so this rule is expected to report
        // zero violations until that lands, the same as any newly added
        // rule with no live callers yet.
        call_patterns: &["PopulationId::from_digest("],
        forbidden_patterns: &[],
        allowed_caller_prefixes: &["model"],
        requires_path: Some("src/model/population.rs"),
        // Genuinely unreachable for the same reason as T12-C's, above:
        // `src/model/population.rs` already exists on origin/main (FR-084's
        // `admit_binding`/`admit_invocation`), so the marker path's presence
        // is all this check tests.
        pending_reason: "unreachable: src/model/population.rs already exists on origin/main",
        // Unlike T12-B/T12-C's `NodeKey`/`EffectiveId`, no cross-repo
        // `PopulationId` shape is known to exist in quire-contract-runtime or
        // quire-contract-codegen today (#295 review finding 8): this rule
        // makes no claim about either, rather than asserting a copy this
        // scan has not found.
        scope_note: Some("scoped to QSL's own tree only"),
    },
];

/// Validates that `qsl_root` is actually a quire-spec-language checkout, by
/// its own manifest's declared package name. Every rule's `requires_path` is
/// checked against this root regardless of which role it scans (`Role`), so
/// without this guard a `--qsl` argument that points at some other tree
/// (for example quire-contract-runtime's or quire-contract-codegen's own
/// checkout) can silently reach the `Pending` branch and report a
/// QSL-specific reason about a tree the rule never actually scanned, with
/// exit 0 -- a green line describing a different repository (#249 review
/// round 2 H-1).
pub(crate) fn assert_is_qsl_root(qsl_root: &Path) -> Result<()> {
    let manifest_path = qsl_root.join("Cargo.toml");
    let manifest =
        fs::read_to_string(&manifest_path).map_err(|error| Error::io(&manifest_path, error))?;
    let is_qsl = manifest
        .lines()
        .any(|line| line.trim() == "name = \"quire-spec-language\"");
    if !is_qsl {
        return Err(Error::new(
            Code::Usage,
            format!(
                "{} is not a quire-spec-language checkout (its Cargo.toml does not declare \
                 name = \"quire-spec-language\"); --qsl must point at quire-spec-language \
                 itself. T12-B, T12-C and T12-D scan QSL's own tree only; see each rule's scope \
                 note for what they do not evaluate",
                qsl_root.display()
            ),
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuleOutcome {
    pub(crate) rule_id: &'static str,
    pub(crate) status: RuleStatus,
    pub(crate) violations: Vec<CallSite>,
}

impl RuleOutcome {
    pub(crate) fn passed(&self) -> bool {
        match self.status {
            RuleStatus::Pending(_) => true,
            RuleStatus::NeedsRoot(_) => false,
            RuleStatus::Live => self.violations.is_empty(),
        }
    }
}

/// Map a `.rs` file's path, relative to `src_root`, to its module path.
/// `foo/bar.rs` -> `foo::bar`; `foo/bar/mod.rs` -> `foo::bar`; a top-level
/// `lib.rs`/`main.rs` is the crate root, `""`.
pub(crate) fn module_path_of(relative: &Path) -> String {
    let mut segments: Vec<String> = relative
        .with_extension("")
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();
    if segments.last().map(String::as_str) == Some("mod") {
        segments.pop();
    }
    if segments.len() == 1 && matches!(segments[0].as_str(), "lib" | "main") {
        segments.clear();
    }
    segments.join("::")
}

fn module_allowed(module: &str, allowed_prefixes: &[&str]) -> bool {
    allowed_prefixes
        .iter()
        .any(|prefix| module == *prefix || module.starts_with(&format!("{prefix}::")))
}

/// Every line of `path` that violates `rule`: a forbidden pattern from any
/// module, or a call pattern from a module outside the allowed callers.
fn scan_file(path: &Path, module: &str, rule: &Rule) -> Result<Vec<CallSite>> {
    let text = fs::read_to_string(path).map_err(|error| Error::io(path, error))?;
    let contains_any = |line: &str, patterns: &[&str]| patterns.iter().any(|p| line.contains(p));
    let caller_allowed = module_allowed(module, rule.allowed_caller_prefixes);
    let mut sites = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if contains_any(line, rule.forbidden_patterns)
            || (!caller_allowed && contains_any(line, rule.call_patterns))
        {
            sites.push(CallSite {
                file: path.to_path_buf(),
                line: index + 1,
                module: module.to_owned(),
            });
        }
    }
    Ok(sites)
}

fn walk_rs_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(root).map_err(|error| Error::io(root, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| Error::io(root, error))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| Error::io(&path, error))?;
        if file_type.is_dir() {
            walk_rs_files(&path, out)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    Ok(())
}

/// Evaluate one rule. `qsl_root` is QSL's own package root, used to test
/// `requires_path` regardless of which role the rule scans (every T-12
/// target lands in QSL first). `scan_root` is the package root of the tree
/// the rule's `role` actually scans (QSL itself for a `Role::Qsl` rule, the
/// CG checkout for a `Role::Cg` rule) -- `None` when the caller has no root
/// for that role, in which case a rule whose target exists reports
/// [`RuleStatus::NeedsRoot`].
pub(crate) fn evaluate(
    rule: &Rule,
    qsl_root: &Path,
    scan_root: Option<&Path>,
) -> Result<RuleOutcome> {
    if let Some(marker) = rule.requires_path {
        if !qsl_root.join(marker).exists() {
            return Ok(RuleOutcome {
                rule_id: rule.id,
                status: RuleStatus::Pending(rule.pending_reason),
                violations: Vec::new(),
            });
        }
    }
    let Some(scan_root) = scan_root else {
        return Ok(RuleOutcome {
            rule_id: rule.id,
            status: RuleStatus::NeedsRoot(rule.role),
            violations: Vec::new(),
        });
    };
    let mut violations = Vec::new();
    for src_root in qsl_scan_src_roots(rule.role, scan_root) {
        if !src_root.exists() {
            // Every listed root is required, the primary root
            // (`<scan_root>/src`) and every extracted-layer-crate root
            // (`qsl-foundation/src`, `qsl-cst/src`, ...) alike: a missing
            // configured root must fail loudly rather than silently drop
            // that tree's coverage (QSL-178 review F4, carried over from
            // review-325's same finding). A crate rename or move that this
            // scanner's own root list has not caught up with is exactly the
            // failure this guards -- it must not read as a clean, coverage-free
            // pass.
            return Err(Error::new(
                Code::Usage,
                format!("source root does not exist: {}", src_root.display()),
            ));
        }
        let mut files = Vec::new();
        walk_rs_files(&src_root, &mut files)?;
        for file in files {
            let relative = file
                .strip_prefix(&src_root)
                .expect("walked file is under src_root")
                .to_path_buf();
            let module = module_path_of(&relative);
            violations.extend(scan_file(&file, &module, rule)?);
        }
    }
    violations.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    Ok(RuleOutcome {
        rule_id: rule.id,
        status: RuleStatus::Live,
        violations,
    })
}

/// Every crate's own `src/` this scan's `role` covers, under one workspace
/// checkout root. A `Role::Cg` rule scans only the CG checkout's own `src/`
/// (CG is a single crate as far as this tool is concerned). A `Role::Qsl`
/// rule scans every QSL workspace crate whose `[dependencies]` can name the
/// symbols these rules match: the root crate's own `src/`, plus each
/// extracted ADR-011 §6.1 layer crate's `src/`: `qsl-foundation`
/// (ADR-011 §7.3 X-2), `qsl-cst` (X-3) and `qsl-replay` (X-10). Each later
/// layer crate joins this list when it is extracted. `quire-exact` and `qsl-attrs` are
/// excluded: `quire-exact` is the kernel these rules' constructors are
/// defined *in*, never a caller of them (T12-B/T12-C/T12-D's own scope notes
/// already exclude checking a copy of the constructor elsewhere; the crate
/// that defines a constructor calling its own inherent `impl` is not a
/// "caller"), and `qsl-attrs` is a proc-macro crate with no dependency on
/// `quire-exact` at all.
fn qsl_scan_src_roots(role: Role, scan_root: &Path) -> Vec<PathBuf> {
    match role {
        Role::Cg => vec![scan_root.join("src")],
        Role::Qsl => ["src", "qsl-foundation/src", "qsl-cst/src", "qsl-replay/src"]
            .into_iter()
            .map(|relative| scan_root.join(relative))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use std::fs;

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    /// Every `Role::Qsl` scan root now has to exist (QSL-178 review F4): a
    /// missing one is an error, not a silently skipped tree. Tests that
    /// exercise `Role::Qsl` rules but are not themselves about a missing
    /// root (every one below except tc_arch_lint_api_surface_015, which
    /// tests exactly that) call this first so the extracted-crate roots
    /// they don't care about are present, but empty.
    fn ensure_qsl_roots(root: &Path) {
        for relative in ["src", "qsl-foundation/src", "qsl-cst/src", "qsl-replay/src"] {
            fs::create_dir_all(root.join(relative)).unwrap();
        }
    }

    /// tc_arch_lint_api_surface_001: module-path mapping matches Rust's own
    /// `mod.rs`/`foo.rs` convention and the crate-root special case.
    #[trace("TC-157")]
    #[test]
    fn tc_arch_lint_api_surface_001_module_path_mapping() {
        assert_eq!(module_path_of(Path::new("value/node.rs")), "value::node");
        assert_eq!(
            module_path_of(Path::new("value/expression/mod.rs")),
            "value::expression"
        );
        assert_eq!(module_path_of(Path::new("lib.rs")), "");
        assert_eq!(module_path_of(Path::new("main.rs")), "");
    }

    /// tc_arch_lint_api_surface_002: a rule whose `requires_path` marker is
    /// absent is reported `Pending`, naming the missing path in its reason --
    /// not a silent pass with zero violations indistinguishable from
    /// "checked and clean", and not an assertion that only compares the
    /// reason against itself (#249 review, MEDIUM-6: the old version of this
    /// test could not fail no matter what `pending_reason` said).
    #[trace("TC-157", "FR-060-AC-1")]
    #[test]
    fn tc_arch_lint_api_surface_002_missing_symbol_is_pending_not_vacuous_pass() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "src/lib.rs", "pub mod value;\n");
        let rule = &RULES[0]; // T12-A: requires qsl-replay/src/lib.rs, absent here.
        let outcome = evaluate(rule, dir.path(), None).unwrap();
        assert_eq!(outcome.rule_id, rule.id);
        match &outcome.status {
            RuleStatus::Pending(reason) => assert!(
                reason.contains("qsl-replay/src/lib.rs"),
                "pending reason must name the missing path, got: {reason}"
            ),
            other => panic!("expected Pending, got {other:?}"),
        }
        assert!(outcome.passed());
        assert!(outcome.violations.is_empty());
    }

    /// tc_arch_lint_api_surface_003 (negative control): a call to the
    /// `NodeKey` constructor from a module outside the allowed list is
    /// reported as a violation, naming the file and line.
    #[trace("TC-157", "FR-060-AC-3")]
    #[test]
    fn tc_arch_lint_api_surface_003_disallowed_caller_is_a_violation() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(
            dir.path(),
            "src/value/node.rs",
            "pub use quire_exact::NodeKey;\n",
        );
        write(
            dir.path(),
            "src/value/model_query.rs",
            "fn f() {\n    let k = NodeKey::from_digest(bytes);\n}\n",
        );
        let rule = &RULES[1]; // T12-B
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert_eq!(outcome.violations.len(), 1);
        assert_eq!(outcome.violations[0].module, "value::model_query");
        assert_eq!(outcome.violations[0].line, 2);
        assert!(!outcome.passed());
    }

    /// tc_arch_lint_api_surface_004: a call from an allowed caller module is
    /// not reported.
    #[trace("TC-157", "FR-060-AC-2")]
    #[test]
    fn tc_arch_lint_api_surface_004_allowed_caller_is_not_a_violation() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(dir.path(), "src/model/key.rs", "impl EffectiveId {}\n");
        write(
            dir.path(),
            "src/model/normalize.rs",
            "fn f() {\n    let id = EffectiveId::from_digest(bytes);\n}\n",
        );
        let rule = &RULES[2]; // T12-C: allowed prefix "model"
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert!(outcome.violations.is_empty());
        assert!(outcome.passed());
    }

    /// tc_arch_lint_api_surface_005 (negative control): a call from a sibling
    /// module that merely starts with the same prefix text (`model_query`,
    /// not `model::...`) is still a violation -- prefix matching is by path
    /// segment, not by string prefix.
    #[trace("TC-157", "FR-060-AC-3")]
    #[test]
    fn tc_arch_lint_api_surface_005_segment_boundary_not_string_prefix() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(dir.path(), "src/model/key.rs", "impl EffectiveId {}\n");
        write(
            dir.path(),
            "src/value/model_query.rs",
            "fn f() {\n    let id = EffectiveId::from_digest(bytes);\n}\n",
        );
        let rule = &RULES[2];
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.violations.len(), 1);
        assert_eq!(outcome.violations[0].module, "value::model_query");
    }

    /// tc_arch_lint_api_surface_006 (negative control, R1/#249 review): a
    /// call to the crate-internal `node_key_of` helper from a module outside
    /// T12-B's allowed list is a violation, the same as a direct
    /// `NodeKey::from_digest` call -- this is what surfaces
    /// `value::enumeration` and `value::unit`'s real minting sites, which a
    /// scan for the constructor pattern alone would miss (they call the
    /// helper, not the constructor, directly). `value::node`'s own
    /// `node_key_of` definition line is a second violation
    /// (tc_arch_lint_api_surface_007): `value::node` is not allow-listed, so
    /// its own mint is a reported site like any other caller.
    #[trace("TC-157", "FR-060-AC-3")]
    #[test]
    fn tc_arch_lint_api_surface_006_node_key_of_helper_call_is_a_violation() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(
            dir.path(),
            "src/value/node.rs",
            "pub(crate) fn node_key_of() { NodeKey::from_digest([0; 32]); }\n",
        );
        write(
            dir.path(),
            "src/value/enumeration.rs",
            "fn f() {\n    node_key_of(&decl);\n}\n",
        );
        let rule = &RULES[1]; // T12-B
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert_eq!(outcome.violations.len(), 2);
        assert!(outcome
            .violations
            .iter()
            .any(|site| site.module == "value::node"));
        assert!(outcome
            .violations
            .iter()
            .any(|site| site.module == "value::enumeration"));
    }

    /// tc_arch_lint_api_surface_007: `value::node` re-exports the kernel
    /// `NodeKey` type and is not allow-listed, so its own `node_key_of` mint
    /// is a real, reported violation like any other disallowed caller
    /// (FR-060 T12-B's named-debt list).
    #[trace("TC-157", "FR-060-AC-3")]
    #[test]
    fn tc_arch_lint_api_surface_007_value_node_is_a_reported_caller() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(
            dir.path(),
            "src/value/node.rs",
            "pub(crate) fn node_key_of() { NodeKey::from_digest([0; 32]); }\n",
        );
        let rule = &RULES[1]; // T12-B
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert_eq!(outcome.violations.len(), 1);
        assert_eq!(outcome.violations[0].module, "value::node");
        assert!(!outcome.passed());
    }

    /// tc_arch_lint_api_surface_008 (negative control, #249 review HIGH-2): a
    /// CG-shaped consumer tree, given as T12-A's `--cg` scan root, that calls
    /// the `replay` facade from a disallowed module is a real, failing
    /// violation -- T12-A must scan the CG tree, not the QSL tree, once its
    /// `requires_path` gate (QSL's own `qsl-replay/src/lib.rs`, QSL-185) is
    /// satisfied.
    #[trace("TC-157", "FR-060-AC-3")]
    #[test]
    fn tc_arch_lint_api_surface_008_consumer_tree_violation_is_caught() {
        let qsl_dir = tempfile::tempdir().unwrap();
        write(qsl_dir.path(), "qsl-replay/src/lib.rs", "pub fn run() {}\n");
        let cg_dir = tempfile::tempdir().unwrap();
        write(
            cg_dir.path(),
            "src/oracle.rs",
            "fn f() {\n    qsl_replay::run();\n}\n",
        );
        let rule = &RULES[0]; // T12-A
        let outcome = evaluate(rule, qsl_dir.path(), Some(cg_dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert_eq!(outcome.violations.len(), 1);
        assert_eq!(outcome.violations[0].module, "oracle");
        assert!(!outcome.passed());
    }

    /// tc_arch_lint_api_surface_010 (negative control, #249 review round 2
    /// H-1): a `--qsl` root whose manifest is not quire-spec-language's own
    /// is rejected before any rule is evaluated -- previously this reached
    /// T12-B/T12-C's `Pending` branch and exited 0 with a QSL-specific
    /// reason describing a tree it never scanned.
    #[trace("TC-157", "FR-060-AC-1")]
    #[test]
    fn tc_arch_lint_api_surface_010_non_qsl_root_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "Cargo.toml",
            "[package]\nname = \"quire-contract-runtime\"\nversion = \"0.1.0\"\n",
        );
        write(
            dir.path(),
            "src/exact/node.rs",
            "pub struct NodeKey([u8; 32]);\n",
        );
        let error = assert_is_qsl_root(dir.path()).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("is not a quire-spec-language checkout"),
            "{error}"
        );
    }

    /// tc_arch_lint_api_surface_011: a `--qsl` root whose manifest really is
    /// quire-spec-language's own is accepted.
    #[trace("TC-157", "FR-060-AC-1")]
    #[test]
    fn tc_arch_lint_api_surface_011_qsl_root_is_accepted() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "Cargo.toml",
            "[package]\nname = \"quire-spec-language\"\nversion = \"0.2.0\"\n",
        );
        assert_is_qsl_root(dir.path()).unwrap();
    }

    /// tc_arch_lint_api_surface_009: a rule whose target exists but whose
    /// role's tree was not supplied reports `NeedsRoot` naming that role, and
    /// does not pass -- `Pending` is reserved for a target that does not
    /// exist yet, never for "no one told this tool where to look."
    #[trace("TC-157", "FR-060-AC-1")]
    #[test]
    fn tc_arch_lint_api_surface_009_live_rule_with_no_scan_root_needs_root() {
        let qsl_dir = tempfile::tempdir().unwrap();
        write(qsl_dir.path(), "qsl-replay/src/lib.rs", "pub fn run() {}\n");
        let rule = &RULES[0]; // T12-A: live (qsl-replay/src/lib.rs exists).
        let outcome = evaluate(rule, qsl_dir.path(), None).unwrap();
        assert_eq!(outcome.status, RuleStatus::NeedsRoot(Role::Cg));
        assert!(outcome.violations.is_empty());
        assert!(!outcome.passed());
    }

    /// tc_arch_lint_api_surface_020 (negative control): a CG call spelled
    /// `quire_spec_language::replay::` is a T12-A violation even from the
    /// allowed `replay` module -- the root crate has no `replay` module, so
    /// that spelling can never be a pass.
    #[trace("TC-157", "FR-060-AC-3")]
    #[test]
    fn tc_arch_lint_api_surface_020_root_crate_replay_path_is_a_violation() {
        let qsl_dir = tempfile::tempdir().unwrap();
        write(qsl_dir.path(), "qsl-replay/src/lib.rs", "pub fn run() {}\n");
        let cg_dir = tempfile::tempdir().unwrap();
        write(
            cg_dir.path(),
            "src/replay.rs",
            "fn f() {\n    quire_spec_language::replay::run();\n    qsl_replay::run();\n}\n",
        );
        let rule = &RULES[0]; // T12-A
        let outcome = evaluate(rule, qsl_dir.path(), Some(cg_dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert_eq!(outcome.violations.len(), 1, "{:?}", outcome.violations);
        assert_eq!(outcome.violations[0].module, "replay");
        assert_eq!(outcome.violations[0].line, 2);
        assert!(!outcome.passed());
    }

    /// tc_arch_lint_api_surface_012 (negative control, ADR-013 QC-21): a call
    /// to the kernel `PopulationId` constructor from a module outside T12-D's
    /// allowed list is a violation, the same shape as T12-C's `EffectiveId`
    /// check (tc_arch_lint_api_surface_003).
    #[trace("TC-157", "FR-060-AC-3")]
    #[test]
    fn tc_arch_lint_api_surface_012_population_id_disallowed_caller_is_a_violation() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(
            dir.path(),
            "src/model/population.rs",
            "impl PopulationId {}\n",
        );
        write(
            dir.path(),
            "src/value/composite.rs",
            "fn f() {\n    let id = PopulationId::from_digest(bytes);\n}\n",
        );
        let rule = &RULES[3]; // T12-D
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert_eq!(outcome.violations.len(), 1);
        assert_eq!(outcome.violations[0].module, "value::composite");
        assert!(!outcome.passed());
    }

    /// tc_arch_lint_api_surface_013 (ADR-013 QC-21): a call to the kernel
    /// `PopulationId` constructor from a `model` module is not a violation --
    /// the mirror of T12-C's allowed-caller check
    /// (tc_arch_lint_api_surface_004).
    #[trace("TC-157", "FR-060-AC-2")]
    #[test]
    fn tc_arch_lint_api_surface_013_population_id_allowed_caller_is_not_a_violation() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(
            dir.path(),
            "src/model/population.rs",
            "fn f() {\n    let id = PopulationId::from_digest(bytes);\n}\n",
        );
        let rule = &RULES[3]; // T12-D: allowed prefix "model"
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert!(outcome.violations.is_empty());
        assert!(outcome.passed());
    }

    /// tc_arch_lint_api_surface_014 (ADR-011 §7.3 X-2, QSL-177): a `Role::Qsl`
    /// rule scans `qsl-foundation/src/` too, not only the root crate's own
    /// `src/` -- the extracted layer-F crate is as much "QSL's own tree" as
    /// the root crate for a rule like T12-D that scans for a kernel
    /// constructor call. Negative control mirroring
    /// tc_arch_lint_api_surface_012, with the disallowed call site moved into
    /// the extracted crate instead of the root crate's own `src/`.
    #[trace("TC-157", "FR-060-AC-3")]
    #[test]
    fn tc_arch_lint_api_surface_014_qsl_foundation_crate_is_scanned() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(
            dir.path(),
            "src/model/population.rs",
            "impl PopulationId {}\n",
        );
        write(
            dir.path(),
            "qsl-foundation/src/digest.rs",
            "fn f() {\n    let id = PopulationId::from_digest(bytes);\n}\n",
        );
        let rule = &RULES[3]; // T12-D
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert_eq!(outcome.violations.len(), 1);
        assert_eq!(outcome.violations[0].module, "digest");
        assert!(!outcome.passed());
    }

    /// tc_arch_lint_api_surface_015 (QSL-178 review F4): a checkout with no
    /// `qsl-foundation/` directory at all is an error, the same as the root
    /// crate's own `src/` being absent (tc_arch_lint_api_surface_009). Every
    /// listed root is required precisely so a crate rename or move this
    /// scanner's root list has not caught up with fails loudly instead of
    /// silently scanning nothing there -- previously this case passed with
    /// zero violations and zero coverage, indistinguishable from a clean
    /// tree.
    #[trace("TC-157", "FR-060-AC-2")]
    #[test]
    fn tc_arch_lint_api_surface_015_missing_qsl_foundation_crate_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "src/model/population.rs",
            "fn f() {\n    let id = PopulationId::from_digest(bytes);\n}\n",
        );
        let rule = &RULES[3]; // T12-D
        let error = evaluate(rule, dir.path(), Some(dir.path())).unwrap_err();
        assert!(error.to_string().contains("qsl-foundation/src"), "{error}");
    }

    /// tc_arch_lint_api_surface_016 (ADR-011 §7.3 X-3, QSL-178): a
    /// `Role::Qsl` rule scans `qsl-cst/src/` too, the same way
    /// tc_arch_lint_api_surface_014 covers `qsl-foundation/src/` -- the
    /// extracted layer-1 crate is as much "QSL's own tree" as the root
    /// crate.
    #[trace("TC-157", "FR-060-AC-3")]
    #[test]
    fn tc_arch_lint_api_surface_016_qsl_cst_crate_is_scanned() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(
            dir.path(),
            "src/model/population.rs",
            "impl PopulationId {}\n",
        );
        write(
            dir.path(),
            "qsl-cst/src/token.rs",
            "fn f() {\n    let id = PopulationId::from_digest(bytes);\n}\n",
        );
        let rule = &RULES[3]; // T12-D
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert_eq!(outcome.violations.len(), 1);
        assert_eq!(outcome.violations[0].module, "token");
        assert!(!outcome.passed());
    }

    /// tc_arch_lint_api_surface_017 (R-1, review-335-v2): T12-B's pattern
    /// matches the bare path `NodeKey::from_digest`, not only the call form
    /// `NodeKey::from_digest(`, so the constructor passed as a value --
    /// `.map(NodeKey::from_digest)`, the shape at `src/value/node.rs:198`'s
    /// `NodeIdDocument::key` -- is caught the same as an ordinary call from
    /// a disallowed module. Both spellings are planted in the same disallowed
    /// module to prove neither is missed.
    #[trace("TC-157", "FR-060-AC-3")]
    #[test]
    fn tc_arch_lint_api_surface_017_path_form_call_is_a_violation() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(
            dir.path(),
            "src/value/node.rs",
            "pub use quire_exact::NodeKey;\n",
        );
        write(
            dir.path(),
            "src/value/reference.rs",
            "fn f() {\n    NodeKey::from_digest([0; 32]);\n}\n\
             fn g(digest: Option<[u8; 32]>) -> Option<NodeKey> {\n    digest.map(NodeKey::from_digest)\n}\n",
        );
        let rule = &RULES[1]; // T12-B
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert_eq!(outcome.violations.len(), 2);
        assert_eq!(outcome.violations[0].module, "value::reference");
        assert_eq!(outcome.violations[0].line, 2);
        assert_eq!(outcome.violations[1].module, "value::reference");
        assert_eq!(outcome.violations[1].line, 5);
        assert!(!outcome.passed());
    }

    /// tc_arch_lint_api_surface_018 (ADR-011 §7.3 X-10, QSL-185): a
    /// `Role::Qsl` rule scans `qsl-replay/src/` too, the same way
    /// tc_arch_lint_api_surface_014/016 cover `qsl-foundation/src/` and
    /// `qsl-cst/src/` -- the extracted layer-6 crate is as much "QSL's own
    /// tree" as the root crate.
    #[trace("TC-157", "FR-060-AC-3")]
    #[test]
    fn tc_arch_lint_api_surface_018_qsl_replay_crate_is_scanned() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(
            dir.path(),
            "src/model/population.rs",
            "impl PopulationId {}\n",
        );
        write(
            dir.path(),
            "qsl-replay/src/identity.rs",
            "fn f() {\n    let id = PopulationId::from_digest(bytes);\n}\n",
        );
        let rule = &RULES[3]; // T12-D
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert_eq!(outcome.violations.len(), 1);
        assert_eq!(outcome.violations[0].module, "identity");
        assert!(!outcome.passed());
    }

    /// tc_arch_lint_api_surface_019 (QSL-178 review F4, QSL-185): a checkout
    /// with no `qsl-replay/` directory at all is an error, the same as
    /// tc_arch_lint_api_surface_015 for `qsl-foundation/`. Every listed root
    /// is required precisely so a crate rename or move this scanner's root
    /// list has not caught up with fails loudly instead of silently scanning
    /// nothing there.
    #[trace("TC-157", "FR-060-AC-2")]
    #[test]
    fn tc_arch_lint_api_surface_019_missing_qsl_replay_crate_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "src/model/population.rs",
            "fn f() {\n    let id = PopulationId::from_digest(bytes);\n}\n",
        );
        fs::create_dir_all(dir.path().join("qsl-foundation/src")).unwrap();
        fs::create_dir_all(dir.path().join("qsl-cst/src")).unwrap();
        let rule = &RULES[3]; // T12-D
        let error = evaluate(rule, dir.path(), Some(dir.path())).unwrap_err();
        assert!(error.to_string().contains("qsl-replay/src"), "{error}");
    }
}
