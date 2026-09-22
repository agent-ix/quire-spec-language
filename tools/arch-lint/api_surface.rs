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
//! **Scanning method (the layer-rule ruling, 2026-09-22).** The scan is
//! textual: it looks for each rule's declared `call_patterns` substring in a
//! `.rs` file and does not resolve `use ... as` renames or macro expansion.
//! A `forbidden_patterns` entry is a violation from any module, including an
//! allowed caller: a spelling that no longer names the rule's symbol at all
//! (T12-A's `quire_spec_language::replay::`, dead since the facade became
//! its own crate, QSL-185). Most rules match `Type::method(` call syntax;
//! T12-B's pattern is the bare path `NodeKey::from_digest`, which also
//! catches the constructor passed as a value rather than called (see T12-B's
//! own comment, below). T12-A and T12-D keep the original, looser posture --
//! neither excludes a match found inside a comment or string literal, both
//! stated limitations, not silent gaps. T12-B and T12-C (`Rule::shipped_only`)
//! additionally parse each file with `syn` to exclude `#[cfg(test)]` items
//! entirely and to blank out comment text before matching (FR-060 Behavior,
//! "T12-B and T12-C: shipped code and debt lists"), and to resolve each
//! mint's enclosing function for the named debt list. A match inside a
//! string literal remains a known limitation of all four rules. `main.rs`'s
//! printed report states these limitations.
//!
//! Each rule scans one *role*'s source tree (see [`Role`]): T12-B, T12-C and
//! T12-D are QSL-side rules (which QSL module calls the kernel constructor),
//! scanned against the QSL tree passed with `--qsl`; T12-A is a CG-side rule
//! (does CG call only QSL's `qsl-replay` facade crate), scanned against the
//! CG tree passed with `--cg` -- never against QSL's own tree, which
//! contains no CG call sites for the rule to find (#249 review,
//! HIGH-2/MEDIUM-4). A rule whose role's tree was not supplied reports
//! [`RuleStatus::NeedsRoot`]; the other rules are still evaluated --
//! `main.rs`'s loop calls `evaluate` once per rule regardless, so one rule's
//! missing input never hides another rule's report.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use syn::{spanned::Spanned, visit::Visit};

use crate::error::{Code, Error, Result};

/// One call site of a rule's symbol, named by the module and (for a
/// [`Rule::shipped_only`] rule) the enclosing function that contains it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CallSite {
    pub(crate) file: PathBuf,
    pub(crate) line: usize,
    pub(crate) module: String,
    /// The enclosing function's name (`Type::method` inside an `impl`, or a
    /// bare function name), for FR-060's named debt-list keying. Empty for
    /// a rule that does not resolve it (`shipped_only: false`, whose debt
    /// list is always empty) or when no enclosing function is found.
    pub(crate) function: String,
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
    /// Whether this rule scans shipped code only (FR-060 Behavior, "T12-B
    /// and T12-C: shipped code and debt lists"): `#[cfg(test)]` items and
    /// comment text are excluded, and each site's enclosing function is
    /// resolved for `debt_list`. `false` for T12-A/T12-D, which keep the
    /// original unfiltered textual scan and always carry an empty
    /// `debt_list`.
    pub(crate) shipped_only: bool,
    /// FR-060's named, shrinking debt list: a `(module, function)` pair
    /// whose mint is reported as debt rather than failing the rule. Keyed
    /// by function, not by line, so the list only shrinks: an entry leaves
    /// in the change that removes its last mint, and no entry is added.
    /// Never consulted for a `forbidden_patterns` match, which fails
    /// regardless of caller or debt status. `&[]` for a rule with no debt
    /// (T12-A, T12-D).
    pub(crate) debt_list: &'static [(&'static str, &'static str)],
}

/// today's four T-12 rules (ADR-011 §3 FB-05; ADR-013 O-04, O-05, O-13/QC-21).
pub(crate) const RULES: &[Rule] = &[
    Rule {
        id: "T12-A",
        description:
            "CG calls the QSL layer-6 `qsl-replay` facade only (ADR-011 §3 FB-05, §2.1 E9)",
        role: Role::Cg,
        // The facade is the `qsl-replay` workspace crate (ADR-011 §6.1 layer 6, QSL-185).
        call_patterns: &["qsl_replay::"],
        // The root crate has no `replay` module any more (QSL-185 moved it
        // into its own crate), so a call spelled this way cannot reach the
        // facade; it is a finding, never a pass, from any module including
        // an allowed one.
        forbidden_patterns: &["quire_spec_language::replay::"],
        // The facade's own internal adapter module has no ticket-assigned
        // name yet (ADR-011 places it in CG, "with RT ops and IR outcome",
        // #217/#219 build it). Left as a placeholder for #213/#217 to set.
        allowed_caller_prefixes: &["replay"],
        requires_path: Some("qsl-replay/src/lib.rs"),
        pending_reason: "the layer-6 facade crate `qsl-replay/src/lib.rs` is absent from the \
                         --qsl tree",
        scope_note: None,
        shipped_only: false,
        debt_list: &[],
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
        // The layer-rule ruling (2026-09-22): T12-B's allowed callers are
        // `check` and every module under it (ADR-013 O-04), replacing the
        // former fixed three-module list (`value::expression::check`,
        // `check::checked_dispatch`, `check::identity`) -- each of those was
        // already a `check` descendant, so the prefix alone covers them,
        // plus `check::family`'s `mint_declaration_identity`/
        // `mint_call_identity`, with no allow-list edit needed for a future
        // `check` descendant. `check::identity`'s own `from_digest` call
        // today is test-only (`identity.rs`, inside `#[cfg(test)]`, so it is
        // excluded from the shipped-code scan regardless of the allow-list).
        //
        // `value::node` re-exports the kernel `NodeKey` and is not
        // allow-listed: `node_key_of`'s own mint and `NodeIdDocument::key`'s
        // wire-digest-string parse-then-wrap (both in `src/value/node.rs`)
        // are named debt here (FR-060's list, below) rather than this rule's
        // sanctioned path.
        allowed_caller_prefixes: &["check"],
        requires_path: Some("src/value/node.rs"),
        // Genuinely unreachable for the same reason as T12-C's, below:
        // `src/value/node.rs` already exists on origin/main as a `pub use
        // quire_exact::NodeKey` re-export rather than a definition, but the
        // marker path's presence is all this check tests.
        pending_reason: "unreachable: src/value/node.rs already exists on origin/main",
        scope_note: Some(
            "scoped to QSL's own tree only; does not scan quire-contract-runtime's \
             independently defined NodeKey type or quire-contract-codegen's generated call \
             sites -- ADR-013 does not name an allowed-caller mapping for either, not decided \
             here (#213)",
        ),
        shipped_only: true,
        // FR-060 Behavior, "T12-B and T12-C: shipped code and debt lists" --
        // every shipped mint outside `check` on origin/main, named by
        // enclosing module and function. This list only shrinks: an entry
        // leaves in the change that removes its last mint.
        debt_list: &[
            ("value::enumeration", "EnumDeclarationPreimage::node_key"),
            ("value::enumeration", "EnumMemberPreimage::node_key"),
            ("value::unit", "DimensionPreimage::node_key"),
            ("value::unit", "UnitPreimage::node_key"),
            ("value::node", "node_key_of"),
            ("value::node", "NodeIdDocument::key"),
            ("value::model_query", "to_object_reference"),
            ("value::expression::family", "decode_v2"),
        ],
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
        shipped_only: true,
        // FR-060 Behavior, "T12-B and T12-C: shipped code and debt lists"
        // (OBS-018).
        debt_list: &[
            ("value::model_query", "bridge_lookup_key"),
            ("value::model_query", "resolve_target"),
        ],
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
        shipped_only: false,
        debt_list: &[],
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
    /// A mint outside the allowed callers, in a function not on
    /// `Rule::debt_list`, or a `Rule::forbidden_patterns` match from any
    /// module -- fails the rule.
    pub(crate) violations: Vec<CallSite>,
    /// A mint outside the allowed callers, in a function that *is* on
    /// `Rule::debt_list` -- reported, but does not fail the rule.
    pub(crate) debt: Vec<CallSite>,
    /// A `Rule::debt_list` entry with no remaining mint found in this scan
    /// -- fails the rule, so a fixed site cannot later hide a new mint
    /// under a stale name.
    pub(crate) stale_debt_entries: Vec<(&'static str, &'static str)>,
}

impl RuleOutcome {
    pub(crate) fn passed(&self) -> bool {
        match self.status {
            RuleStatus::Pending(_) => true,
            // Not a silent pass: the rule's compliance genuinely was not
            // checked, so the overall run does not report clean.
            RuleStatus::NeedsRoot(_) => false,
            RuleStatus::Live => self.violations.is_empty() && self.stale_debt_entries.is_empty(),
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

/// Whether `attrs` includes `#[cfg(test)]` -- FR-060 Behavior, "T12-B and
/// T12-C: shipped code and debt lists".
fn has_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("cfg") {
            return false;
        }
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("test") {
                found = true;
            }
            Ok(())
        });
        found
    })
}

/// Blank out `//`-style line comments (covering `///`/`//!` doc comments)
/// and `/* ... */` block comments from `source`, one output line per input
/// line, so a pattern match inside either is never reported for T12-B/T12-C
/// (FR-060 Behavior). A conservative, line-oriented pass: it does not
/// distinguish `//`/`/*` text that appears inside a string literal from a
/// real comment -- a known limitation, the same shape T12-A/T12-D's own
/// stated textual-scan limitations already have, and not one QSL's own
/// source style triggers (a constructor name is never written as a string
/// literal next to a real comment marker).
fn strip_comments(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_block_comment = false;
    for line in source.lines() {
        let mut sanitized = String::with_capacity(line.len());
        let mut chars = line.char_indices();
        while let Some((index, ch)) = chars.next() {
            if in_block_comment {
                if ch == '*' && line[index..].starts_with("*/") {
                    in_block_comment = false;
                    chars.next();
                }
                continue;
            }
            if ch == '/' && line[index..].starts_with("//") {
                break;
            }
            if ch == '/' && line[index..].starts_with("/*") {
                in_block_comment = true;
                chars.next();
                continue;
            }
            sanitized.push(ch);
        }
        out.push(sanitized);
    }
    out
}

/// Every source line inside a `#[cfg(test)]`-gated item (`mod`, `fn`, `impl`,
/// an `impl` method, `struct`, `enum`, `trait`, `static` or `const`), 1-based
/// and inclusive of the item's own first and last line -- FR-060 Behavior,
/// "T12-B and T12-C: shipped code and debt lists".
fn cfg_test_lines(parsed: &syn::File) -> BTreeSet<usize> {
    struct CfgTestVisitor {
        lines: BTreeSet<usize>,
    }
    impl CfgTestVisitor {
        fn mark(&mut self, span: proc_macro2::Span) {
            for line in span.start().line..=span.end().line {
                self.lines.insert(line);
            }
        }
    }
    macro_rules! skip_if_cfg_test {
        ($visit_fn:ident, $node_ty:ty, $default:path) => {
            fn $visit_fn(&mut self, node: &'_ $node_ty) {
                if has_cfg_test(&node.attrs) {
                    self.mark(node.span());
                } else {
                    $default(self, node);
                }
            }
        };
    }
    impl<'ast> Visit<'ast> for CfgTestVisitor {
        skip_if_cfg_test!(visit_item_mod, syn::ItemMod, syn::visit::visit_item_mod);
        skip_if_cfg_test!(visit_item_fn, syn::ItemFn, syn::visit::visit_item_fn);
        skip_if_cfg_test!(visit_item_impl, syn::ItemImpl, syn::visit::visit_item_impl);
        skip_if_cfg_test!(
            visit_impl_item_fn,
            syn::ImplItemFn,
            syn::visit::visit_impl_item_fn
        );
        skip_if_cfg_test!(
            visit_item_struct,
            syn::ItemStruct,
            syn::visit::visit_item_struct
        );
        skip_if_cfg_test!(visit_item_enum, syn::ItemEnum, syn::visit::visit_item_enum);
        skip_if_cfg_test!(
            visit_item_trait,
            syn::ItemTrait,
            syn::visit::visit_item_trait
        );
        skip_if_cfg_test!(
            visit_item_static,
            syn::ItemStatic,
            syn::visit::visit_item_static
        );
        skip_if_cfg_test!(
            visit_item_const,
            syn::ItemConst,
            syn::visit::visit_item_const
        );
    }
    let mut visitor = CfgTestVisitor {
        lines: BTreeSet::new(),
    };
    visitor.visit_file(parsed);
    visitor.lines
}

/// The `Self` type name of an `impl` block, for qualifying a method's own
/// name (`Type::method`) -- `"<impl>"` for a `Self` type this cannot name
/// (a rare shape none of T12-B/T12-C's debt-list functions use).
fn impl_self_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_else(|| "<impl>".to_owned()),
        _ => "<impl>".to_owned(),
    }
}

/// A `syn` walk finding the innermost `fn` (a bare module-level function, or
/// an `impl` method qualified `Type::method`) whose span contains a target
/// line -- FR-060's debt-list keying, "by enclosing module and function,
/// not by line number."
struct EnclosingFunction {
    target_line: usize,
    current_impl_self: Option<String>,
    found: Option<(String, usize)>,
}

impl EnclosingFunction {
    fn consider(&mut self, span: proc_macro2::Span, name: String) {
        let start = span.start().line;
        let end = span.end().line;
        if start <= self.target_line && self.target_line <= end {
            let width = end - start;
            if self.found.as_ref().is_none_or(|(_, best)| width < *best) {
                self.found = Some((name, width));
            }
        }
    }
}

impl<'ast> Visit<'ast> for EnclosingFunction {
    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        let previous = self
            .current_impl_self
            .replace(impl_self_name(&node.self_ty));
        syn::visit::visit_item_impl(self, node);
        self.current_impl_self = previous;
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.consider(node.span(), node.sig.ident.to_string());
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let name = match &self.current_impl_self {
            Some(self_ty) => format!("{self_ty}::{}", node.sig.ident),
            None => node.sig.ident.to_string(),
        };
        self.consider(node.span(), name);
        syn::visit::visit_impl_item_fn(self, node);
    }
}

/// Resolve a 1-based `line` to its innermost enclosing function's name (see
/// [`EnclosingFunction`]). Empty when no function's span contains `line`.
fn enclosing_function(parsed: &syn::File, line: usize) -> String {
    let mut visitor = EnclosingFunction {
        target_line: line,
        current_impl_self: None,
        found: None,
    };
    visitor.visit_file(parsed);
    visitor.found.map(|(name, _)| name).unwrap_or_default()
}

/// The source line a bare (no `::`) call pattern's *own* function
/// definition sits on, if this file defines one -- FR-060 Behavior, "T12-B
/// also matches calls of `node_key_of`... the helper's own `fn
/// node_key_of(` definition line is not a mint." `bare_names` is each
/// pattern with `::` in it filtered out and any trailing `(` trimmed (a
/// pattern naming an associated function, like `NodeKey::from_digest`,
/// never collides with a plain `fn`'s own signature text this way).
fn bare_fn_definition_lines(parsed: &syn::File, bare_names: &[&str]) -> BTreeSet<usize> {
    struct DefFinder<'a> {
        names: &'a [&'a str],
        lines: BTreeSet<usize>,
    }
    impl DefFinder<'_> {
        fn consider(&mut self, ident: &syn::Ident, fn_token_span: proc_macro2::Span) {
            if self.names.contains(&ident.to_string().as_str()) {
                self.lines.insert(fn_token_span.start().line);
            }
        }
    }
    impl<'ast> Visit<'ast> for DefFinder<'_> {
        fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
            self.consider(&node.sig.ident, node.sig.fn_token.span());
            syn::visit::visit_item_fn(self, node);
        }
        fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
            self.consider(&node.sig.ident, node.sig.fn_token.span());
            syn::visit::visit_impl_item_fn(self, node);
        }
    }
    let mut finder = DefFinder {
        names: bare_names,
        lines: BTreeSet::new(),
    };
    finder.visit_file(parsed);
    finder.lines
}

/// Every line of `path` that violates `rule`: a `forbidden_patterns` match
/// from any module (tagged `true`), or a `call_patterns` match from a
/// module outside the allowed callers (tagged `false`, for `evaluate`'s
/// debt-list lookup).
fn scan_file(path: &Path, module: &str, rule: &Rule) -> Result<Vec<(CallSite, bool)>> {
    let text = fs::read_to_string(path).map_err(|error| Error::io(path, error))?;
    let contains_any = |line: &str, patterns: &[&str]| patterns.iter().any(|p| line.contains(p));
    let caller_allowed = module_allowed(module, rule.allowed_caller_prefixes);
    let mut sites = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let is_forbidden = contains_any(line, rule.forbidden_patterns);
        let is_call = !caller_allowed && contains_any(line, rule.call_patterns);
        if is_forbidden || is_call {
            sites.push((
                CallSite {
                    file: path.to_path_buf(),
                    line: index + 1,
                    module: module.to_owned(),
                    function: String::new(),
                },
                is_forbidden,
            ));
        }
    }
    Ok(sites)
}

/// T12-B/T12-C's scan (`Rule::shipped_only`): excludes `#[cfg(test)]` items
/// and comment text, and resolves each site's enclosing function. Tags each
/// site the same way [`scan_file`] does (`forbidden_patterns` vs.
/// `call_patterns`).
fn scan_shipped_file(path: &Path, module: &str, rule: &Rule) -> Result<Vec<(CallSite, bool)>> {
    let text = fs::read_to_string(path).map_err(|error| Error::io(path, error))?;
    let parsed = syn::parse_file(&text).map_err(|source| Error::source_parse(path, source))?;
    let excluded_lines = cfg_test_lines(&parsed);
    let caller_allowed = module_allowed(module, rule.allowed_caller_prefixes);
    let bare_patterns: Vec<&str> = rule
        .call_patterns
        .iter()
        .copied()
        .filter(|pattern| !pattern.contains("::"))
        .map(|pattern| pattern.trim_end_matches('('))
        .collect();
    let bare_definition_lines = bare_fn_definition_lines(&parsed, &bare_patterns);
    let mut sites = Vec::new();
    for (index, line) in strip_comments(&text).iter().enumerate() {
        let line_number = index + 1;
        if excluded_lines.contains(&line_number) {
            continue;
        }
        let is_forbidden = rule
            .forbidden_patterns
            .iter()
            .any(|pattern| line.contains(pattern));
        let is_call = !caller_allowed
            && rule.call_patterns.iter().any(|pattern| {
                if !line.contains(pattern) {
                    return false;
                }
                let bare = pattern.trim_end_matches('(');
                !(bare_patterns.contains(&bare) && bare_definition_lines.contains(&line_number))
            });
        if is_forbidden || is_call {
            sites.push((
                CallSite {
                    file: path.to_path_buf(),
                    line: line_number,
                    module: module.to_owned(),
                    function: enclosing_function(&parsed, line_number),
                },
                is_forbidden,
            ));
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
                debt: Vec::new(),
                stale_debt_entries: Vec::new(),
            });
        }
    }
    let Some(scan_root) = scan_root else {
        return Ok(RuleOutcome {
            rule_id: rule.id,
            status: RuleStatus::NeedsRoot(rule.role),
            violations: Vec::new(),
            debt: Vec::new(),
            stale_debt_entries: Vec::new(),
        });
    };
    let mut violations = Vec::new();
    let mut debt = Vec::new();
    let mut debt_seen: BTreeSet<(&'static str, &'static str)> = BTreeSet::new();
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
            let sites = if rule.shipped_only {
                scan_shipped_file(&file, &module, rule)?
            } else {
                scan_file(&file, &module, rule)?
            };
            for (site, is_forbidden) in sites {
                if is_forbidden {
                    violations.push(site);
                    continue;
                }
                if module_allowed(&site.module, rule.allowed_caller_prefixes) {
                    continue;
                }
                let debt_entry = rule.debt_list.iter().find(|(module, function)| {
                    *module == site.module && *function == site.function
                });
                match debt_entry {
                    Some(entry) => {
                        debt_seen.insert(*entry);
                        debt.push(site);
                    }
                    None => violations.push(site),
                }
            }
        }
    }
    violations.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    debt.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    let stale_debt_entries: Vec<(&'static str, &'static str)> = rule
        .debt_list
        .iter()
        .copied()
        .filter(|entry| !debt_seen.contains(entry))
        .collect();
    Ok(RuleOutcome {
        rule_id: rule.id,
        status: RuleStatus::Live,
        violations,
        debt,
        stale_debt_entries,
    })
}

/// Every crate's own `src/` this scan's `role` covers, under one workspace
/// checkout root. A `Role::Cg` rule scans only the CG checkout's own `src/`
/// (CG is a single crate as far as this tool is concerned). A `Role::Qsl`
/// rule scans every QSL workspace crate whose `[dependencies]` can name the
/// symbols these rules match: the root crate's own `src/`, plus each
/// extracted ADR-011 §6.1 layer crate's `src/`: `qsl-foundation`
/// (ADR-011 §7.3 X-2), `qsl-cst` (X-3) and `qsl-replay` (X-10). Each later
/// layer crate joins this list when it is extracted. `quire-exact` and
/// `qsl-attrs` are excluded: `quire-exact` is the kernel these rules'
/// constructors are defined *in*, never a caller of them (T12-B/T12-C/T12-D's
/// own scope notes already exclude checking a copy of the constructor
/// elsewhere; the crate that defines a constructor calling its own inherent
/// `impl` is not a "caller"), and `qsl-attrs` is a proc-macro crate with no
/// dependency on `quire-exact` at all.
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
    /// root (every one below except tc_arch_lint_api_surface_015/019, which
    /// test exactly that) call this first so the extracted-crate roots they
    /// don't care about are present, but empty.
    fn ensure_qsl_roots(root: &Path) {
        for relative in ["src", "qsl-foundation/src", "qsl-cst/src", "qsl-replay/src"] {
            fs::create_dir_all(root.join(relative)).unwrap();
        }
    }

    /// Writes one real mint (`mint_expr`, e.g. `"let _ = NodeKey::from_digest(x);"`)
    /// for every entry on `rule.debt_list`, grouped one file per module.
    /// FR-060 Behavior treats a debt-list entry with no remaining mint as
    /// stale (a failure) -- correct against the real, whole-tree scan, but
    /// a fixture test that plants only the one scenario it cares about would
    /// otherwise see every *other* entry the debt list names as spuriously
    /// stale, since its own narrow tree never touches them. Tests that are
    /// not themselves about staleness call this first so the debt list
    /// starts fully satisfied, then plant their own scenario on top.
    fn seed_debt_list_baseline(root: &Path, rule: &Rule, mint_expr: &str) {
        let mut by_module: std::collections::BTreeMap<&str, String> =
            std::collections::BTreeMap::new();
        for (module, function) in rule.debt_list {
            let block = match function.split_once("::") {
                Some((type_name, method)) => format!(
                    "pub struct {type_name};\nimpl {type_name} {{\n    pub fn {method}(&self) {{ {mint_expr} }}\n}}\n"
                ),
                None => format!("pub fn {function}() {{ {mint_expr} }}\n"),
            };
            by_module.entry(module).or_default().push_str(&block);
        }
        for (module, body) in by_module {
            let relative = format!("src/{}.rs", module.replace("::", "/"));
            write(root, &relative, &body);
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
            "src/library/mod.rs",
            "fn f() {\n    let k = NodeKey::from_digest(bytes);\n}\n",
        );
        let rule = &RULES[1]; // T12-B
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert_eq!(outcome.violations.len(), 1);
        assert_eq!(outcome.violations[0].module, "library");
        assert_eq!(outcome.violations[0].line, 2);
        assert_eq!(outcome.violations[0].function, "f");
        assert!(!outcome.passed());
    }

    /// tc_arch_lint_api_surface_004: a call from an allowed caller module is
    /// not reported.
    #[trace("TC-157", "FR-060-AC-2")]
    #[test]
    fn tc_arch_lint_api_surface_004_allowed_caller_is_not_a_violation() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        let rule = &RULES[2]; // T12-C: allowed prefix "model"
        seed_debt_list_baseline(dir.path(), rule, "let _ = EffectiveId::from_digest(x);");
        write(dir.path(), "src/model/key.rs", "impl EffectiveId {}\n");
        write(
            dir.path(),
            "src/model/normalize.rs",
            "fn f() {\n    let id = EffectiveId::from_digest(bytes);\n}\n",
        );
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert!(outcome.violations.is_empty());
        assert_eq!(
            outcome.debt.len(),
            rule.debt_list.len(),
            "{:?}",
            outcome.debt
        );
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
    /// `node_key_of` mint is separately named debt (FR-060's list, tested by
    /// tc_arch_lint_api_surface_007), so this fixture's own caller -- an
    /// ordinary function, not one of the named debt functions -- is the one
    /// real violation here.
    #[trace("TC-157", "FR-060-AC-3")]
    #[test]
    fn tc_arch_lint_api_surface_006_node_key_of_helper_call_is_a_violation() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        let rule = &RULES[1]; // T12-B
        seed_debt_list_baseline(dir.path(), rule, "let _ = NodeKey::from_digest(x);");
        write(
            dir.path(),
            "src/value/enumeration.rs",
            "fn an_unlisted_caller() {\n    node_key_of(&decl);\n}\n",
        );
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert_eq!(outcome.violations.len(), 1, "{:?}", outcome.violations);
        assert_eq!(outcome.violations[0].module, "value::enumeration");
        assert_eq!(outcome.violations[0].function, "an_unlisted_caller");
        assert!(!outcome.passed());
    }

    /// tc_arch_lint_api_surface_007: `value::node` re-exports the kernel
    /// `NodeKey` type and is not allow-listed, so its own `node_key_of` mint
    /// would be a violation like any other disallowed caller -- except that
    /// `node_key_of` is itself one of FR-060 T12-B's named debt-list
    /// functions, so it is reported as debt, not a failure.
    #[trace("TC-157", "FR-060-AC-4")]
    #[test]
    fn tc_arch_lint_api_surface_007_value_node_mint_is_named_debt() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        let rule = &RULES[1]; // T12-B
        seed_debt_list_baseline(dir.path(), rule, "let _ = NodeKey::from_digest(x);");
        // Overwrite the seeded `value::node` baseline with a
        // shaped-like-the-real-thing mint for `node_key_of` specifically,
        // keeping `NodeIdDocument::key`'s own seeded mint alongside it so
        // both of this module's debt-list entries stay satisfied.
        write(
            dir.path(),
            "src/value/node.rs",
            "pub(crate) fn node_key_of() { let _ = NodeKey::from_digest([0; 32]); }\n\
             pub struct NodeIdDocument;\n\
             impl NodeIdDocument {\n    pub fn key(&self) { let _ = NodeKey::from_digest([0; 32]); }\n}\n",
        );
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert!(outcome.violations.is_empty(), "{:?}", outcome.violations);
        assert!(
            outcome
                .debt
                .iter()
                .any(|site| site.module == "value::node" && site.function == "node_key_of"),
            "{:?}",
            outcome.debt
        );
        assert!(
            outcome.stale_debt_entries.is_empty(),
            "{:?}",
            outcome.stale_debt_entries
        );
        assert!(outcome.passed());
    }

    /// tc_arch_lint_api_surface_008 (negative control, #249 review HIGH-2): a
    /// CG-shaped consumer tree, given as T12-A's `--cg` scan root, that calls
    /// the `qsl-replay` facade from a disallowed module is a real, failing
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
    /// scanner's own root list has not caught up with fails loudly instead
    /// of silently scanning nothing there -- previously this case passed
    /// with zero violations and zero coverage, indistinguishable from a
    /// clean tree.
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
    /// `.map(NodeKey::from_digest)`, the shape at `src/value/node.rs`'s
    /// `NodeIdDocument::key` -- is caught the same as an ordinary call from
    /// a disallowed module. Both spellings are planted in the same
    /// disallowed module (`value::reference`, not on T12-B's debt list) to
    /// prove neither is missed.
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
        assert_eq!(outcome.violations.len(), 2, "{:?}", outcome.violations);
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

    // -------------------------------------------------------------------
    // FR-060 T12-B/T12-C (the layer-rule ruling, 2026-09-22): the named,
    // shrinking debt list, `#[cfg(test)]`/comment exclusion, and the
    // function-value-reference patterns (TC-157 step 6/7).
    // -------------------------------------------------------------------

    /// TC-157 step 6: a shipped `NodeKey::from_digest(` call in a `check`
    /// submodule is not reported at all.
    #[trace("TC-157", "FR-060-AC-4")]
    #[test]
    fn tc_157_check_submodule_mint_is_not_reported() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        let rule = &RULES[1]; // T12-B
        seed_debt_list_baseline(dir.path(), rule, "let _ = NodeKey::from_digest(x);");
        write(
            dir.path(),
            "src/check/family.rs",
            "fn mint_declaration_identity(bytes: &[u8]) -> NodeKey {\n    NodeKey::from_digest(bytes)\n}\n",
        );
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert!(outcome.violations.is_empty(), "{:?}", outcome.violations);
        // Only the seeded baseline's own debt, none of it from `check`.
        assert!(
            outcome
                .debt
                .iter()
                .all(|site| site.module.starts_with("value::")),
            "{:?}",
            outcome.debt
        );
        assert_eq!(
            outcome.debt.len(),
            rule.debt_list.len(),
            "{:?}",
            outcome.debt
        );
        assert!(outcome.passed());
    }

    /// TC-157 step 6: a shipped `.map(NodeKey::from_digest)` -- the
    /// constructor passed as a function value, no trailing `(` in the
    /// matched text -- in a module outside `check`, in a function not on
    /// the debt list, fails T12-B and is named.
    #[trace("TC-157", "FR-060-AC-4")]
    #[test]
    fn tc_157_function_value_mint_outside_check_fails() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(dir.path(), "src/value/node.rs", "pub struct NodeKey;\n");
        write(
            dir.path(),
            "src/library/mod.rs",
            "fn f(bytes: Option<[u8; 32]>) -> Option<NodeKey> {\n    bytes.map(NodeKey::from_digest)\n}\n",
        );
        let rule = &RULES[1]; // T12-B
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.violations.len(), 1, "{:?}", outcome.violations);
        assert_eq!(outcome.violations[0].module, "library");
        assert_eq!(outcome.violations[0].function, "f");
        assert!(!outcome.passed());
    }

    /// TC-157 step 6: a shipped mint in a function on the debt list is
    /// reported as debt and does not fail T12-B.
    #[trace("TC-157", "FR-060-AC-4")]
    #[test]
    fn tc_157_debt_list_mint_is_reported_as_debt() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        let rule = &RULES[1]; // T12-B: debt list has (value::model_query, to_object_reference)
        seed_debt_list_baseline(dir.path(), rule, "let _ = NodeKey::from_digest(x);");
        // Overwrite `value::model_query`'s seeded baseline with a
        // shaped-like-the-real-thing mint -- T12-B's debt list has exactly
        // one entry in this module, so nothing else to preserve.
        write(
            dir.path(),
            "src/value/model_query.rs",
            "fn to_object_reference(bytes: [u8; 32]) -> NodeKey {\n    NodeKey::from_digest(bytes)\n}\n",
        );
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert!(outcome.violations.is_empty(), "{:?}", outcome.violations);
        assert!(
            outcome
                .debt
                .iter()
                .any(|site| site.module == "value::model_query"
                    && site.function == "to_object_reference"),
            "{:?}",
            outcome.debt
        );
        assert!(
            outcome.stale_debt_entries.is_empty(),
            "{:?}",
            outcome.stale_debt_entries
        );
        assert!(outcome.passed());
    }

    /// TC-157 step 6: a debt-list entry whose function no longer mints
    /// fails T12-B and names the stale entry, so a fixed site cannot later
    /// hide a new mint under a name the list still carries.
    #[trace("TC-157", "FR-060-AC-4")]
    #[test]
    fn tc_157_stale_debt_entry_fails() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(dir.path(), "src/value/node.rs", "pub struct NodeKey;\n");
        write(
            dir.path(),
            "src/value/model_query.rs",
            "fn to_object_reference() -> u8 {\n    0\n}\n",
        );
        let rule = &RULES[1]; // T12-B
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert!(outcome.violations.is_empty());
        assert!(outcome.debt.is_empty());
        assert!(outcome
            .stale_debt_entries
            .contains(&("value::model_query", "to_object_reference")));
        assert!(!outcome.passed());
    }

    /// TC-157 step 6: a `NodeKey::from_digest(` call inside a `#[cfg(test)]`
    /// item outside `check` is not reported.
    #[trace("TC-157", "FR-060-AC-4")]
    #[test]
    fn tc_157_cfg_test_mint_is_not_reported() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        let rule = &RULES[1]; // T12-B
        seed_debt_list_baseline(dir.path(), rule, "let _ = NodeKey::from_digest(x);");
        write(
            dir.path(),
            "src/library/mod.rs",
            "#[cfg(test)]\nmod tests {\n    use super::*;\n    fn f(bytes: [u8; 32]) -> NodeKey {\n        NodeKey::from_digest(bytes)\n    }\n}\n",
        );
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert!(outcome.violations.is_empty(), "{:?}", outcome.violations);
        assert!(
            outcome
                .debt
                .iter()
                .all(|site| site.module.starts_with("value::")),
            "{:?}",
            outcome.debt
        );
        assert_eq!(
            outcome.debt.len(),
            rule.debt_list.len(),
            "{:?}",
            outcome.debt
        );
        assert!(outcome.passed());
    }

    /// TC-157 step 6: a doc comment naming `NodeKey::from_digest` in a
    /// module outside `check` is not reported.
    #[trace("TC-157", "FR-060-AC-4")]
    #[test]
    fn tc_157_doc_comment_mention_is_not_reported() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        let rule = &RULES[1]; // T12-B
        seed_debt_list_baseline(dir.path(), rule, "let _ = NodeKey::from_digest(x);");
        write(
            dir.path(),
            "src/library/mod.rs",
            "/// Mints via `NodeKey::from_digest` in the real implementation.\npub fn f() {}\n",
        );
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert!(outcome.violations.is_empty(), "{:?}", outcome.violations);
        assert!(
            outcome
                .debt
                .iter()
                .all(|site| site.module.starts_with("value::")),
            "{:?}",
            outcome.debt
        );
        assert_eq!(
            outcome.debt.len(),
            rule.debt_list.len(),
            "{:?}",
            outcome.debt
        );
        assert!(outcome.passed());
    }

    /// TC-157 step 7: a shipped `EffectiveId::from_digest(` call in
    /// `value::model_query`, in a function not on T12-C's debt list, fails
    /// and is named with file, line, module and function.
    #[trace("TC-157", "FR-060-AC-4")]
    #[test]
    fn tc_157_t12c_mint_outside_debt_list_fails() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(dir.path(), "src/model/key.rs", "pub struct EffectiveId;\n");
        write(
            dir.path(),
            "src/value/model_query.rs",
            "fn a_new_function(bytes: [u8; 32]) -> EffectiveId {\n    EffectiveId::from_digest(bytes)\n}\n",
        );
        let rule = &RULES[2]; // T12-C
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.violations.len(), 1, "{:?}", outcome.violations);
        assert_eq!(outcome.violations[0].module, "value::model_query");
        assert_eq!(outcome.violations[0].function, "a_new_function");
        assert!(!outcome.passed());
    }
}
