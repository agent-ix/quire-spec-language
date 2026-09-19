// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-060 (ADR-011 §3 FB-05; ADR-013 O-04, O-05; #215 T-12): the reusable
//! API-surface check.
//!
//! Each rule names a symbol (a constructor or a facade module) and the
//! module prefixes allowed to call it. The `quire-exact` crate (#213 S-1)
//! does not exist yet, so today's rule data points at the symbols' current
//! locations on origin/main (`crate::value::node::NodeKey`,
//! `crate::model::key::EffectiveId`) and QSL's current module names, rather
//! than the post-#213 kernel crate. #213 updates the paths here in its own
//! PR; this module keeps the rule set as data for exactly that reason.
//!
//! The scan is textual: it looks for `Type::method(` call syntax in a `.rs`
//! file's own module and does not resolve `use ... as` renames or macro
//! expansion, nor does it exclude a match found inside a comment or string
//! literal. Both are stated limitations, not a silent gap: a caller that
//! imports a rule's symbol under another name, or a comment that happens to
//! quote a call pattern, is not distinguished from a real call site by this
//! version of the check. `main.rs`'s printed report states both limitations.
//!
//! Each rule scans one *role*'s source tree (see [`Role`]): T12-B and T12-C
//! are QSL-side rules (which QSL module calls the kernel constructor), scanned
//! against the QSL tree passed with `--qsl`; T12-A is a CG-side rule (does CG
//! call only QSL's `replay` facade), scanned against the CG tree passed with
//! `--cg` -- never against QSL's own tree, which the rule's call pattern
//! (`quire_spec_language::replay::`, a fully-qualified external-caller path)
//! could never match from inside QSL itself. Evaluating a CG-role rule
//! against the QSL tree would go straight from `Pending` to a vacuous `PASS`
//! the moment the QSL-side facade module exists, without ever having scanned
//! the tree the rule actually protects (#249 review, HIGH-2/MEDIUM-4).

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

/// Whether a rule's symbol exists yet in the scanned tree. A rule the tool
/// cannot evaluate is reported as `Pending`, never as a silent pass.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RuleStatus {
    Live,
    Pending(&'static str),
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
    /// Call-site substrings that identify a use of the rule's symbol, for
    /// example `"NodeKey::of("`.
    pub(crate) call_patterns: &'static [&'static str],
    /// Module path prefixes allowed to contain a call site (matched as
    /// `module == prefix` or `module.starts_with("{prefix}::")`).
    pub(crate) allowed_caller_prefixes: &'static [&'static str],
    /// A file path, relative to the QSL tree, whose presence the rule needs
    /// before it is live. `None` means the rule is always live once a root is
    /// given (the constructor rules below: the files that define `NodeKey`
    /// and `EffectiveId` already exist on origin/main).
    pub(crate) requires_path: Option<&'static str>,
    pub(crate) pending_reason: &'static str,
}

/// today's three T-12 rules (ADR-011 §3 FB-05; ADR-013 O-04, O-05).
pub(crate) const RULES: &[Rule] = &[
    Rule {
        id: "T12-A",
        description: "CG calls the QSL layer-6 `replay` facade only (ADR-011 §3 FB-05, §2.1 E9)",
        role: Role::Cg,
        call_patterns: &["quire_spec_language::replay::"],
        // The facade's own internal adapter module has no ticket-assigned
        // name yet (ADR-011 places it in CG, "with RT ops and IR outcome",
        // #217/#219 build it). Left as a placeholder for #213/#217 to set.
        allowed_caller_prefixes: &["replay"],
        requires_path: Some("src/replay.rs"),
        pending_reason:
            "the QSL layer-6 `replay` module `src/replay.rs` (ADR-011 §1 S8, §2.1 E9) does not \
             exist yet on origin/main; it is #217/#219's work. This rule cannot be evaluated \
             until `src/replay.rs` lands.",
    },
    Rule {
        id: "T12-B",
        description: "only `check` calls the kernel `NodeKey` constructor (ADR-013 O-04)",
        role: Role::Qsl,
        // `of` mints from a preimage digest; `from_bytes` bridges another
        // 32-byte digest into the same domain with no re-hash (value/node.rs
        // doc comment). `node_key_of` is the crate-internal helper both
        // `value::enumeration` and `value::unit` mint through, calling `of`
        // in turn -- scanning for it directly, rather than only its callee,
        // is what surfaces those two modules' own minting sites (R1, #249
        // review, review item 7). All three are today's constructors; #213
        // replaces them with the `quire-exact` crate's single public
        // constructor.
        call_patterns: &["NodeKey::of(", "NodeKey::from_bytes(", "node_key_of("],
        // ADR-011 §1 stage table, S3 row: "QSL check (today: value::expression
        // check, model::checked_dispatch, value::library)". `check` itself
        // does not exist as a module yet, so today's rule uses that table's
        // named equivalents. `value::node` is also allowed: it is `NodeKey`'s
        // own defining module, where `node_key_of` itself calls `NodeKey::of`
        // -- the constructor minting itself is not a "caller" this rule's
        // boundary is about (R1, #249 review). ADR-011 §1's S3 "today" list
        // names only the three `check`-equivalent modules; it does not name
        // `value::node`, `value::enumeration` or `value::unit`, so this
        // five-site finding is new information for whoever owns
        // ADR-013/#211/#213, reported here, not resolved by widening this
        // list further.
        allowed_caller_prefixes: &[
            "value::expression::check",
            "model::checked_dispatch",
            "value::library",
            "value::node",
        ],
        requires_path: Some("src/value/node.rs"),
        pending_reason: "unreachable: NodeKey is defined in src/value/node.rs on origin/main",
    },
    Rule {
        id: "T12-C",
        description: "only `model` calls the kernel `EffectiveId` constructor (ADR-013 O-05)",
        role: Role::Qsl,
        call_patterns: &["EffectiveId::from_digest_bytes("],
        allowed_caller_prefixes: &["model"],
        requires_path: Some("src/model/key.rs"),
        pending_reason: "unreachable: EffectiveId is defined in src/model/key.rs on origin/main",
    },
];

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

fn scan_file(path: &Path, module: &str, patterns: &[&str]) -> Result<Vec<CallSite>> {
    let text = fs::read_to_string(path).map_err(|error| Error::io(path, error))?;
    let mut sites = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if patterns.iter().any(|pattern| line.contains(pattern)) {
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
/// for that role, which is only valid while the rule is still `Pending`.
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
        return Err(Error::new(
            Code::Usage,
            format!(
                "{}'s target has landed; pass {} to give it a tree to scan",
                rule.id,
                rule.role.flag_name()
            ),
        ));
    };
    let src_root = scan_root.join("src");
    if !src_root.exists() {
        return Err(Error::new(
            Code::Usage,
            format!("source root does not exist: {}", src_root.display()),
        ));
    }
    let mut files = Vec::new();
    walk_rs_files(&src_root, &mut files)?;
    let mut violations = Vec::new();
    for file in files {
        let relative = file
            .strip_prefix(&src_root)
            .expect("walked file is under src_root")
            .to_path_buf();
        let module = module_path_of(&relative);
        let sites = scan_file(&file, &module, rule.call_patterns)?;
        violations.extend(
            sites
                .into_iter()
                .filter(|site| !module_allowed(&site.module, rule.allowed_caller_prefixes)),
        );
    }
    violations.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    Ok(RuleOutcome {
        rule_id: rule.id,
        status: RuleStatus::Live,
        violations,
    })
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
        let rule = &RULES[0]; // T12-A: requires src/replay.rs, absent here.
        let outcome = evaluate(rule, dir.path(), None).unwrap();
        assert_eq!(outcome.rule_id, rule.id);
        match &outcome.status {
            RuleStatus::Pending(reason) => assert!(
                reason.contains("src/replay.rs"),
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
        write(
            dir.path(),
            "src/value/node.rs",
            "impl NodeKey { fn of(x: &[u8]) -> Self { todo!() } }\n",
        );
        write(
            dir.path(),
            "src/value/model_query.rs",
            "fn f() {\n    let k = NodeKey::of(&bytes);\n}\n",
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
        write(dir.path(), "src/model/key.rs", "impl EffectiveId {}\n");
        write(
            dir.path(),
            "src/model/normalize.rs",
            "fn f() {\n    let id = EffectiveId::from_digest_bytes(bytes);\n}\n",
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
        write(dir.path(), "src/model/key.rs", "impl EffectiveId {}\n");
        write(
            dir.path(),
            "src/value/model_query.rs",
            "fn f() {\n    let id = EffectiveId::from_digest_bytes(bytes);\n}\n",
        );
        let rule = &RULES[2];
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.violations.len(), 1);
        assert_eq!(outcome.violations[0].module, "value::model_query");
    }

    /// tc_arch_lint_api_surface_006 (negative control, R1/#249 review): a
    /// call to the crate-internal `node_key_of` helper from a module outside
    /// T12-B's allowed list is a violation, the same as a direct
    /// `NodeKey::of` call -- this is what surfaces `value::enumeration` and
    /// `value::unit`'s real minting sites, which a scan for `NodeKey::of(`
    /// alone would miss (they call the helper, not the constructor,
    /// directly).
    #[trace("TC-157", "FR-060-AC-3")]
    #[test]
    fn tc_arch_lint_api_surface_006_node_key_of_helper_call_is_a_violation() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "src/value/node.rs",
            "pub(crate) fn node_key_of() { NodeKey::of(&[]); }\n",
        );
        write(
            dir.path(),
            "src/value/enumeration.rs",
            "fn f() {\n    node_key_of(&decl);\n}\n",
        );
        let rule = &RULES[1]; // T12-B
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert_eq!(outcome.violations.len(), 1);
        assert_eq!(outcome.violations[0].module, "value::enumeration");
    }

    /// tc_arch_lint_api_surface_007 (R1/#249 review): `NodeKey`'s own
    /// defining module (`value::node`) calling its own constructor through
    /// the `node_key_of` helper is exempted -- the constructor minting itself
    /// is not a "caller" T12-B's boundary is about.
    #[trace("TC-157", "FR-060-AC-2")]
    #[test]
    fn tc_arch_lint_api_surface_007_defining_module_is_exempt() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "src/value/node.rs",
            "pub(crate) fn node_key_of() { NodeKey::of(&[]); }\n",
        );
        let rule = &RULES[1]; // T12-B
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert!(outcome.violations.is_empty(), "{:?}", outcome.violations);
    }

    /// tc_arch_lint_api_surface_008 (negative control, #249 review HIGH-2): a
    /// CG-shaped consumer tree, given as T12-A's `--cg` scan root, that calls
    /// the `replay` facade from a disallowed module is a real, failing
    /// violation -- T12-A must scan the CG tree, not the QSL tree, once its
    /// `requires_path` gate (QSL's own `src/replay.rs`) is satisfied.
    #[trace("TC-157", "FR-060-AC-3")]
    #[test]
    fn tc_arch_lint_api_surface_008_consumer_tree_violation_is_caught() {
        let qsl_dir = tempfile::tempdir().unwrap();
        write(qsl_dir.path(), "src/replay.rs", "pub fn run() {}\n");
        let cg_dir = tempfile::tempdir().unwrap();
        write(
            cg_dir.path(),
            "src/oracle.rs",
            "fn f() {\n    quire_spec_language::replay::run();\n}\n",
        );
        let rule = &RULES[0]; // T12-A
        let outcome = evaluate(rule, qsl_dir.path(), Some(cg_dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert_eq!(outcome.violations.len(), 1);
        assert_eq!(outcome.violations[0].module, "oracle");
        assert!(!outcome.passed());
    }

    /// tc_arch_lint_api_surface_009: once a rule's target has landed, scanning
    /// it with no root for its role is a usage error, not a silent pass --
    /// `Pending` is reserved for a target that does not exist yet, never for
    /// "no one told this tool where to look."
    #[trace("TC-157", "FR-060-AC-1")]
    #[test]
    fn tc_arch_lint_api_surface_009_live_rule_with_no_scan_root_is_an_error() {
        let qsl_dir = tempfile::tempdir().unwrap();
        write(qsl_dir.path(), "src/replay.rs", "pub fn run() {}\n");
        let rule = &RULES[0]; // T12-A: now live (src/replay.rs exists).
        let error = evaluate(rule, qsl_dir.path(), None).unwrap_err();
        assert!(error.to_string().contains("--cg"), "{error}");
    }
}
