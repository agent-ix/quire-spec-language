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
//! expansion. That is a stated limitation, not a silent gap: a caller that
//! imports a rule's symbol under another name is not found by this version
//! of the check.

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

/// One ADR-011 T-12 API-surface rule (data, per the module doc above).
pub(crate) struct Rule {
    pub(crate) id: &'static str,
    pub(crate) description: &'static str,
    /// Call-site substrings that identify a use of the rule's symbol, for
    /// example `"NodeKey::of("`.
    pub(crate) call_patterns: &'static [&'static str],
    /// Module path prefixes allowed to contain a call site (matched as
    /// `module == prefix` or `module.starts_with("{prefix}::")`).
    pub(crate) allowed_caller_prefixes: &'static [&'static str],
    /// A file path (relative to the scanned source root) whose presence the
    /// rule needs before it is live. `None` means the rule is always live
    /// once a root is given (the constructor rules below: the files that
    /// define `NodeKey` and `EffectiveId` already exist on origin/main).
    pub(crate) requires_path: Option<&'static str>,
    pub(crate) pending_reason: &'static str,
}

/// today's three T-12 rules (ADR-011 §3 FB-05; ADR-013 O-04, O-05).
pub(crate) const RULES: &[Rule] = &[
    Rule {
        id: "T12-A",
        description: "CG calls the QSL layer-6 `replay` facade only (ADR-011 §3 FB-05, §2.1 E9)",
        call_patterns: &["quire_spec_language::replay::"],
        // The facade's own internal adapter module has no ticket-assigned
        // name yet (ADR-011 places it in CG, "with RT ops and IR outcome",
        // #217/#219 build it). Left as a placeholder for #213/#217 to set.
        allowed_caller_prefixes: &["replay"],
        requires_path: Some("src/replay.rs"),
        pending_reason:
            "the QSL layer-6 `replay` module (ADR-011 §1 S8, §2.1 E9) does not exist yet on \
             origin/main; it is #217/#219's work. This rule cannot be evaluated until it lands.",
    },
    Rule {
        id: "T12-B",
        description: "only `check` calls the kernel `NodeKey` constructor (ADR-013 O-04)",
        // `of` mints from a preimage digest; `from_bytes` bridges another
        // 32-byte digest into the same domain with no re-hash (value/node.rs
        // doc comment). Both are today's constructors; #213 replaces both
        // with the `quire-exact` crate's single public constructor.
        call_patterns: &["NodeKey::of(", "NodeKey::from_bytes("],
        // ADR-011 §1 stage table, S3 row: "QSL check (today: value::expression
        // check, model::checked_dispatch, value::library)". `check` itself
        // does not exist as a module yet, so today's rule uses that table's
        // named equivalents.
        allowed_caller_prefixes: &[
            "value::expression::check",
            "model::checked_dispatch",
            "value::library",
        ],
        requires_path: Some("src/value/node.rs"),
        pending_reason: "unreachable: NodeKey is defined in src/value/node.rs on origin/main",
    },
    Rule {
        id: "T12-C",
        description: "only `model` calls the kernel `EffectiveId` constructor (ADR-013 O-05)",
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

/// Evaluate one rule over the `.rs` files under `src_root` (a crate's `src/`
/// directory). `crate_root` is that crate's package root, used only to test
/// `requires_path`.
pub(crate) fn evaluate(rule: &Rule, crate_root: &Path, src_root: &Path) -> Result<RuleOutcome> {
    if let Some(marker) = rule.requires_path {
        if !crate_root.join(marker).exists() {
            return Ok(RuleOutcome {
                rule_id: rule.id,
                status: RuleStatus::Pending(rule.pending_reason),
                violations: Vec::new(),
            });
        }
    }
    if !src_root.exists() {
        return Err(Error::new(
            Code::Usage,
            format!("source root does not exist: {}", src_root.display()),
        ));
    }
    let mut files = Vec::new();
    walk_rs_files(src_root, &mut files)?;
    let mut violations = Vec::new();
    for file in files {
        let relative = file
            .strip_prefix(src_root)
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
    /// absent is reported `Pending`, never a silent pass with zero
    /// violations indistinguishable from "checked and clean".
    #[trace("TC-157", "FR-060-AC-1")]
    #[test]
    fn tc_arch_lint_api_surface_002_missing_symbol_is_pending_not_vacuous_pass() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "src/lib.rs", "pub mod value;\n");
        let rule = &RULES[0]; // T12-A: requires src/replay.rs, absent here.
        let outcome = evaluate(rule, dir.path(), &dir.path().join("src")).unwrap();
        assert_eq!(outcome.status, RuleStatus::Pending(rule.pending_reason));
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
        let outcome = evaluate(rule, dir.path(), &dir.path().join("src")).unwrap();
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
        let outcome = evaluate(rule, dir.path(), &dir.path().join("src")).unwrap();
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
        let outcome = evaluate(rule, dir.path(), &dir.path().join("src")).unwrap();
        assert_eq!(outcome.violations.len(), 1);
        assert_eq!(outcome.violations[0].module, "value::model_query");
    }
}
