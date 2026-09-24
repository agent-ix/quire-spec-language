// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-060 (ADR-011 §3 FB-05; ADR-013 O-04, O-05; #215 T-12): the reusable
//! API-surface check.
//!
//! Each rule names a symbol (a constructor or a facade module) and the
//! module prefixes allowed to call it. The `quire-exact` crate defines the
//! kernel `EffectiveId`/`NodeKey` types; T12-B, T12-C and T12-D each match
//! that type's `from_digest` constructor; QSL defines no `NodeKey` of its
//! own.
//!
//! **Scanning method (FR-060 Behavior, "Scanning method and its stated
//! limitations").** A `forbidden_patterns` entry is a violation from any
//! module, including an allowed caller: a spelling that no longer names the
//! rule's symbol at all (T12-A's `quire_spec_language::replay::`, dead since
//! the facade became its own crate, QSL-185).
//!
//! T12-B, T12-C and T12-D (`Rule::shipped_only`) match each pattern against
//! the file's `proc_macro2` tokens, not its text: `NodeKey::from_digest` is
//! the token run `NodeKey` `:` `:` `from_digest`. A comment is not a token and
//! a string is one literal token, so neither can match, while a call split
//! across lines, a constructor passed as a function value, and a call inside
//! a macro's arguments all do. They also parse each file with `syn` to
//! exclude `#[cfg(test)]` items and to resolve each mint's enclosing function
//! for the named debt list.
//!
//! T12-A scans CG's text for its substrings, so a match inside a comment or
//! string literal is reported (a possible false positive).
//!
//! No rule resolves a `use ... as` rename of the constructor's type.
//! `main.rs`'s printed report states these limitations.
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

/// One allowed caller of a rule's symbol: the crate, named by its source
/// root relative to the scanned checkout (`"src"` for the root crate,
/// `"qsl-semantics/src"`), and a module path prefix inside it, matched as
/// `module == prefix` or `module.starts_with("{prefix}::")`. Module paths are
/// relative to their own crate, and two crates can have modules of the same
/// name (the root crate and `qsl-semantics` both have `value` and
/// `complete`), so a module prefix alone would also allow a same-named
/// module in any other crate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AllowedCaller {
    pub(crate) crate_src: &'static str,
    pub(crate) module_prefix: &'static str,
}

/// One entry of a rule's named, shrinking debt list: a function, named by its
/// crate (source root relative to the scanned checkout, as in
/// [`AllowedCaller`]), its module path inside that crate and its name
/// (`Type::method` inside an `impl`). Keyed by crate as well as module, for
/// the reason [`AllowedCaller`] gives: a same-named module in another crate
/// is not the listed debt.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct DebtEntry {
    pub(crate) crate_src: &'static str,
    pub(crate) module: &'static str,
    pub(crate) function: &'static str,
}

/// One ADR-011 T-12 API-surface rule (data, per the module doc above).
pub(crate) struct Rule {
    pub(crate) id: &'static str,
    pub(crate) description: &'static str,
    pub(crate) role: Role,
    /// Patterns that identify a use of the rule's symbol: a path such as
    /// `"NodeKey::from_digest"` (called or passed as a value), or a trailing
    /// `(` to require a call, as in `"node_key_of("`.
    pub(crate) call_patterns: &'static [&'static str],
    /// Call-site substrings that are a violation from any module, including
    /// an allowed caller: a path that no longer names the rule's symbol.
    pub(crate) forbidden_patterns: &'static [&'static str],
    /// The callers allowed to contain a call site: each names its crate and
    /// a module path prefix within it (see [`AllowedCaller`]).
    pub(crate) allowed_callers: &'static [AllowedCaller],
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
    /// Whether this rule scans shipped code by token (FR-060 Behavior,
    /// "T12-B and T12-C: shipped code and debt lists"): `#[cfg(test)]` items,
    /// comments and string literals are excluded, and each site's enclosing
    /// function is resolved for `debt_list`. `false` for T12-A, a textual
    /// scan of CG's tree whose `debt_list` is always empty.
    pub(crate) shipped_only: bool,
    /// FR-060's named, shrinking debt list: a function (see [`DebtEntry`])
    /// whose mint is reported as debt rather than failing the rule. Keyed
    /// by function, not by line, so the list only shrinks: an entry leaves
    /// in the change that removes its last mint, and no entry is added.
    /// Never consulted for a `forbidden_patterns` match, which fails
    /// regardless of caller or debt status. `&[]` for a rule with no debt
    /// (T12-A, T12-D).
    pub(crate) debt_list: &'static [DebtEntry],
}

/// today's five T-12 rules (ADR-011 §3 FB-05; ADR-013 O-04, O-05, O-13/QC-21,
/// T-1).
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
        allowed_callers: &[AllowedCaller {
            crate_src: "src",
            module_prefix: "replay",
        }],
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
        // (`.map(NodeKey::from_digest)`), which a call-form-only pattern
        // misses. `node_key_of` was the crate-internal minting helper
        // `value::enumeration` and `value::unit` called (R1, #249 review,
        // review item 7); QSL-131 K4 deleted it, and the pattern stays so a
        // reintroduced helper of that name is caught.
        call_patterns: &["NodeKey::from_digest", "node_key_of("],
        forbidden_patterns: &[],
        // T12-B's allowed callers are `check` and every module under it
        // (ADR-013 O-04), including `check::node_key`, which mints every
        // checked node key.
        allowed_callers: &[AllowedCaller {
            crate_src: "qsl-semantics/src",
            module_prefix: "check",
        }],
        // No marker: the kernel `NodeKey` constructor exists, so T12-B is
        // always live once a root is given.
        requires_path: None,
        pending_reason: "",
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
        debt_list: &[DebtEntry {
            crate_src: "qsl-eval/src",
            module: "value::expression::family",
            function: "decode_v2",
        }],
    },
    Rule {
        id: "T12-C",
        description: "only `model` calls the kernel `EffectiveId` constructor (ADR-013 O-05)",
        role: Role::Qsl,
        // The kernel's constructor (`quire-exact`'s `EffectiveId::
        // from_digest`, #213 S-1/S-2). The bare path matches it called or
        // passed as a function value (`.map(EffectiveId::from_digest)`).
        call_patterns: &["EffectiveId::from_digest"],
        forbidden_patterns: &[],
        allowed_callers: &[AllowedCaller {
            crate_src: "qsl-semantics/src",
            module_prefix: "model",
        }],
        requires_path: Some("qsl-semantics/src/model/key.rs"),
        // Genuinely unreachable for the same reason as T12-B's, above:
        // `qsl-semantics/src/model/key.rs` already exists on origin/main (it now
        // re-exports the kernel `EffectiveId` rather than defining it, but
        // the marker path's presence is all this check tests).
        pending_reason: "unreachable: qsl-semantics/src/model/key.rs already exists on origin/main",
        scope_note: Some(
            "scoped to QSL's own tree only; does not scan quire-contract-runtime's or \
             quire-contract-codegen's own copies of this identity's shape -- ADR-013 does not \
             name an allowed-caller mapping for either, not decided here (#213)",
        ),
        shipped_only: true,
        // FR-060 Behavior, "T12-B and T12-C: shipped code and debt lists".
        // Empty since QSL-131 V1 retyped reference types to `EffectiveId`
        // (ADR-013 O-05, OBS-018), so any shipped mint outside `model` fails.
        debt_list: &[],
    },
    Rule {
        id: "T12-D",
        description: "only `model` calls the kernel `PopulationId` constructor (ADR-013 QC-21)",
        role: Role::Qsl,
        // The kernel's constructor (`quire-exact`'s `PopulationId::
        // from_digest`, QSL-131 Slice B), called or passed as a function
        // value. Shipped code only: a test fixture that builds a
        // `PopulationId` from literal bytes mints nothing. Its debt list is
        // empty, so any shipped mint outside `model` fails.
        call_patterns: &["PopulationId::from_digest"],
        forbidden_patterns: &[],
        allowed_callers: &[AllowedCaller {
            crate_src: "qsl-semantics/src",
            module_prefix: "model",
        }],
        requires_path: Some("qsl-semantics/src/model/population.rs"),
        // Genuinely unreachable for the same reason as T12-C's, above:
        // `qsl-semantics/src/model/population.rs` already exists on origin/main (FR-084's
        // `admit_binding`/`admit_invocation`), so the marker path's presence
        // is all this check tests.
        pending_reason:
            "unreachable: qsl-semantics/src/model/population.rs already exists on origin/main",
        // Unlike T12-B/T12-C's `NodeKey`/`EffectiveId`, no cross-repo
        // `PopulationId` shape is known to exist in quire-contract-runtime or
        // quire-contract-codegen today (#295 review finding 8): this rule
        // makes no claim about either, rather than asserting a copy this
        // scan has not found.
        scope_note: Some("scoped to QSL's own tree only"),
        shipped_only: true,
        debt_list: &[],
    },
    Rule {
        id: "T12-E",
        description: "only layer-4 `qsl-package`'s `checked_v2` mints the ADR-011 §4 \
                      condition-1 witness `SupportedV2Wire` (ADR-013 T-1, FR-087-AC-1)",
        role: Role::Qsl,
        // `library::SupportedV2Wire::attest_ir_admitted_v2` is `pub` only so
        // the layer-4 v2 reader can call it across the QSL-181 crate
        // boundary; the witness attests that IR's v2 reader admitted the
        // bytes, which only that reader knows. The bare name matches every
        // spelling: `SupportedV2Wire::attest_ir_admitted_v2(..)`,
        // `Self::attest_ir_admitted_v2(..)` and the minter passed as a value.
        // Its own `fn attest_ir_admitted_v2(` definition is not a call.
        call_patterns: &["attest_ir_admitted_v2"],
        forbidden_patterns: &[],
        allowed_callers: &[AllowedCaller {
            crate_src: "qsl-package/src",
            module_prefix: "checked_v2",
        }],
        requires_path: Some("qsl-semantics/src/library/witness.rs"),
        pending_reason: "the witness module `qsl-semantics/src/library/witness.rs` is absent from \
                         the --qsl tree",
        scope_note: Some(
            "scoped to QSL's own tree; confines references by module only, so a wrapper \
             defined inside `checked_v2` (a `#[macro_export]` macro, a trait impl or a helper) \
             and called elsewhere passes this rule -- the companion gate \
             `tests/it/verified_binding_witness.rs` refuses any reference in `checked_v2` but \
             the one direct call in `read_checked_package_v2`'s `AdmittedV2` arm, across every \
             workspace crate's `src/`; the allowed caller is `qsl-package`'s `checked_v2` \
             since X-7",
        ),
        shipped_only: true,
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
    pub(crate) stale_debt_entries: Vec<DebtEntry>,
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

/// Whether `module` of the crate whose source root is `crate_src` is one of
/// `allowed`.
fn module_allowed(crate_src: &str, module: &str, allowed: &[AllowedCaller]) -> bool {
    allowed.iter().any(|caller| {
        caller.crate_src == crate_src
            && (module == caller.module_prefix
                || module.starts_with(&format!("{}::", caller.module_prefix)))
    })
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

/// One source token, reduced to what [`CallPattern`] matching needs. Comments
/// are not tokens at all, a doc comment lexes as a `#[doc = "..."]` string
/// literal, and every literal is [`Token::Other`], so a pattern named inside a
/// comment or a string never matches.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Token {
    Ident(String),
    Punct(char),
    /// The opening delimiter of a `( ... )` group.
    OpenParen,
    /// A literal, or any other group delimiter.
    Other,
}

/// A [`Token`] and the 1-based source line it starts on.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LocatedToken {
    pub(crate) token: Token,
    pub(crate) line: usize,
}

/// Flatten `stream` depth-first into `out`, descending into every group --
/// macro arguments included -- so a call split across lines or written
/// inside `vec![...]` is one contiguous token run.
pub(crate) fn flatten_tokens(stream: proc_macro2::TokenStream, out: &mut Vec<LocatedToken>) {
    for tree in stream {
        let line = tree.span().start().line;
        match tree {
            proc_macro2::TokenTree::Ident(ident) => out.push(LocatedToken {
                token: Token::Ident(ident.to_string()),
                line,
            }),
            proc_macro2::TokenTree::Punct(punct) => out.push(LocatedToken {
                token: Token::Punct(punct.as_char()),
                line,
            }),
            proc_macro2::TokenTree::Literal(_) => out.push(LocatedToken {
                token: Token::Other,
                line,
            }),
            proc_macro2::TokenTree::Group(group) => {
                let open = match group.delimiter() {
                    proc_macro2::Delimiter::Parenthesis => Token::OpenParen,
                    _ => Token::Other,
                };
                out.push(LocatedToken { token: open, line });
                flatten_tokens(group.stream(), out);
                out.push(LocatedToken {
                    token: Token::Other,
                    line: group.span_close().start().line,
                });
            }
        }
    }
}

/// A rule's call pattern compiled to a token sequence: `"NodeKey::from_digest"`
/// is `NodeKey`, `:`, `:`, `from_digest`; a trailing `(`, as in
/// `"node_key_of("`, requires an opening parenthesis next.
pub(crate) struct CallPattern {
    tokens: Vec<Token>,
}

impl CallPattern {
    pub(crate) fn compile(pattern: &str) -> Self {
        let (path, call) = match pattern.strip_suffix('(') {
            Some(path) => (path, true),
            None => (pattern, false),
        };
        let mut tokens = Vec::new();
        for (index, segment) in path.split("::").enumerate() {
            if index > 0 {
                tokens.extend([Token::Punct(':'), Token::Punct(':')]);
            }
            tokens.push(Token::Ident(segment.to_owned()));
        }
        if call {
            tokens.push(Token::OpenParen);
        }
        Self { tokens }
    }

    /// Whether the pattern names a plain function (no `::`), whose own
    /// `fn name(` definition is not a call of it.
    fn is_bare_function(&self) -> bool {
        !self.tokens.contains(&Token::Punct(':'))
    }

    /// Whether this pattern matches `source` starting at `start`. A `>`
    /// between a type segment and its `::` is skipped, so the qualified-path
    /// spelling `<NodeKey>::from_digest` matches too.
    fn matches_at(&self, source: &[LocatedToken], start: usize) -> bool {
        let mut position = start;
        for (index, expected) in self.tokens.iter().enumerate() {
            let follows_ident = index > 0 && matches!(self.tokens[index - 1], Token::Ident(_));
            if *expected == Token::Punct(':')
                && follows_ident
                && source.get(position).map(|located| &located.token) == Some(&Token::Punct('>'))
            {
                position += 1;
            }
            match source.get(position) {
                Some(located) if located.token == *expected => position += 1,
                _ => return false,
            }
        }
        true
    }
}

/// The 1-based line of every match of any of `patterns` in `tokens`. A
/// bare-function pattern preceded by `fn` is that function's own definition,
/// not a call, and is skipped (FR-060 Behavior: "the helper's own `fn
/// node_key_of(` definition line is not a mint").
pub(crate) fn pattern_match_lines(
    tokens: &[LocatedToken],
    patterns: &[CallPattern],
) -> BTreeSet<usize> {
    let mut lines = BTreeSet::new();
    for (start, located) in tokens.iter().enumerate() {
        let is_definition = start.checked_sub(1).is_some_and(
            |previous| matches!(&tokens[previous].token, Token::Ident(name) if name == "fn"),
        );
        let matched = patterns.iter().any(|pattern| {
            pattern.matches_at(tokens, start) && !(pattern.is_bare_function() && is_definition)
        });
        if matched {
            lines.insert(located.line);
        }
    }
    lines
}

/// Every source line inside a `#[cfg(test)]`-gated item (`mod`, `fn`, `impl`,
/// an `impl` method, `struct`, `enum`, `trait`, `static` or `const`), 1-based
/// and inclusive of the item's own first and last line -- FR-060 Behavior,
/// "T12-B and T12-C: shipped code and debt lists".
pub(crate) fn cfg_test_lines(parsed: &syn::File) -> BTreeSet<usize> {
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

/// Every line of `path` that violates `rule`: a `forbidden_patterns` match
/// from any module (tagged `true`), or a `call_patterns` match from a
/// module outside the allowed callers (tagged `false`, for `evaluate`'s
/// debt-list lookup).
fn scan_file(
    path: &Path,
    crate_src: &str,
    module: &str,
    rule: &Rule,
) -> Result<Vec<(CallSite, bool)>> {
    let text = fs::read_to_string(path).map_err(|error| Error::io(path, error))?;
    let contains_any = |line: &str, patterns: &[&str]| patterns.iter().any(|p| line.contains(p));
    let caller_allowed = module_allowed(crate_src, module, rule.allowed_callers);
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

/// The scan for a [`Rule::shipped_only`] rule (T12-B, T12-C, T12-D): matches
/// each pattern against the file's tokens, not its text, so comments and
/// string literals never match and a call split across lines still does;
/// excludes `#[cfg(test)]` items; and resolves each site's enclosing function.
/// Tags each site the same way [`scan_file`] does (`forbidden_patterns` vs.
/// `call_patterns`).
fn scan_shipped_file(
    path: &Path,
    crate_src: &str,
    module: &str,
    rule: &Rule,
) -> Result<Vec<(CallSite, bool)>> {
    let text = fs::read_to_string(path).map_err(|error| Error::io(path, error))?;
    let parsed = syn::parse_file(&text).map_err(|source| Error::source_parse(path, source))?;
    let stream: proc_macro2::TokenStream = text
        .parse()
        .map_err(|source| Error::source_parse(path, source))?;
    let mut tokens = Vec::new();
    flatten_tokens(stream, &mut tokens);
    let excluded_lines = cfg_test_lines(&parsed);
    let compile = |patterns: &[&str]| -> Vec<CallPattern> {
        patterns.iter().map(|p| CallPattern::compile(p)).collect()
    };
    let forbidden_lines = pattern_match_lines(&tokens, &compile(rule.forbidden_patterns));
    let call_lines = if module_allowed(crate_src, module, rule.allowed_callers) {
        BTreeSet::new()
    } else {
        pattern_match_lines(&tokens, &compile(rule.call_patterns))
    };
    let sites = forbidden_lines
        .union(&call_lines)
        .filter(|line| !excluded_lines.contains(line))
        .map(|&line| {
            (
                CallSite {
                    file: path.to_path_buf(),
                    line,
                    module: module.to_owned(),
                    function: enclosing_function(&parsed, line),
                },
                forbidden_lines.contains(&line),
            )
        })
        .collect();
    Ok(sites)
}

/// Every module declared out of line as `#[cfg(test)] mod name;`, as the
/// module path of its file (`parent::name`) -- FR-060 Behavior, "T12-B and
/// T12-C: shipped code and debt lists". Such a file, and every module under
/// it, is test-only even though the file itself carries no `#[cfg(test)]`.
/// A declaration with a `#[path]` attribute is not resolved, so its file is
/// still scanned.
pub(crate) fn cfg_test_module_declarations(
    modules: &[(PathBuf, String)],
) -> Result<BTreeSet<String>> {
    fn collect(items: &[syn::Item], prefix: &str, in_test: bool, out: &mut BTreeSet<String>) {
        for item in items {
            let syn::Item::Mod(module) = item else {
                continue;
            };
            let path = if prefix.is_empty() {
                module.ident.to_string()
            } else {
                format!("{prefix}::{}", module.ident)
            };
            let is_test = in_test || has_cfg_test(&module.attrs);
            match &module.content {
                Some((_, inner)) => collect(inner, &path, is_test, out),
                None if is_test && !module.attrs.iter().any(|a| a.path().is_ident("path")) => {
                    out.insert(path);
                }
                None => {}
            }
        }
    }
    let mut declared = BTreeSet::new();
    for (file, module) in modules {
        let text = fs::read_to_string(file).map_err(|error| Error::io(file, error))?;
        let parsed = syn::parse_file(&text).map_err(|source| Error::source_parse(file, source))?;
        collect(&parsed.items, module, false, &mut declared);
    }
    Ok(declared)
}

pub(crate) fn walk_rs_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
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
    let mut debt_seen: BTreeSet<DebtEntry> = BTreeSet::new();
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
        let crate_src = src_root
            .strip_prefix(scan_root)
            .expect("a source root is under its scan root")
            .to_string_lossy()
            .replace('\\', "/");
        let mut files = Vec::new();
        walk_rs_files(&src_root, &mut files)?;
        let modules: Vec<(PathBuf, String)> = files
            .into_iter()
            .map(|file| {
                let relative = file
                    .strip_prefix(&src_root)
                    .expect("walked file is under src_root")
                    .to_path_buf();
                let module = module_path_of(&relative);
                (file, module)
            })
            .collect();
        let test_modules = if rule.shipped_only {
            cfg_test_module_declarations(&modules)?
        } else {
            BTreeSet::new()
        };
        for (file, module) in modules {
            let under = |ancestor: &String| {
                module == *ancestor || module.starts_with(&format!("{ancestor}::"))
            };
            if test_modules.iter().any(under) {
                continue;
            }
            let sites = if rule.shipped_only {
                scan_shipped_file(&file, &crate_src, &module, rule)?
            } else {
                scan_file(&file, &crate_src, &module, rule)?
            };
            for (site, is_forbidden) in sites {
                if is_forbidden {
                    violations.push(site);
                    continue;
                }
                if module_allowed(&crate_src, &site.module, rule.allowed_callers) {
                    continue;
                }
                let debt_entry = rule.debt_list.iter().find(|entry| {
                    entry.crate_src == crate_src
                        && entry.module == site.module
                        && entry.function == site.function
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
    let stale_debt_entries: Vec<DebtEntry> = rule
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
/// (ADR-011 §7.3 X-2), `qsl-cst` (X-3), `qsl-source` (X-4), `qsl-forms`
/// (X-5), `qsl-semantics` (X-6), `qsl-package` (X-7), `qsl-eval` (X-8), `qsl-route` (X-9)
/// and `qsl-replay` (X-10). A module path is relative to its own crate's
/// `src/`, so `check` (T12-B), `model` (T12-C, T12-D) and `library` name
/// `qsl-semantics`' modules, and `checked_v2` (T12-E) `qsl-package`'s. Each later layer crate joins this list when it is
/// extracted. `quire-exact` and `qsl-attrs` are excluded: `quire-exact` is the kernel these rules'
/// constructors are defined *in*, never a caller of them (T12-B/T12-C/T12-D's
/// own scope notes already exclude checking a copy of the constructor
/// elsewhere; the crate that defines a constructor calling its own inherent
/// `impl` is not a "caller"), and `qsl-attrs` is a proc-macro crate with no
/// dependency on `quire-exact` at all.
pub(crate) fn qsl_scan_src_roots(role: Role, scan_root: &Path) -> Vec<PathBuf> {
    match role {
        Role::Cg => vec![scan_root.join("src")],
        Role::Qsl => [
            "src",
            "qsl-foundation/src",
            "qsl-cst/src",
            "qsl-source/src",
            "qsl-forms/src",
            "qsl-semantics/src",
            "qsl-package/src",
            "qsl-eval/src",
            "qsl-route/src",
            "qsl-replay/src",
        ]
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
        for relative in [
            "src",
            "qsl-foundation/src",
            "qsl-cst/src",
            "qsl-source/src",
            "qsl-forms/src",
            "qsl-semantics/src",
            "qsl-package/src",
            "qsl-eval/src",
            "qsl-route/src",
            "qsl-replay/src",
        ] {
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
        let mut by_module: std::collections::BTreeMap<(&str, &str), String> =
            std::collections::BTreeMap::new();
        for DebtEntry {
            crate_src,
            module,
            function,
        } in rule.debt_list
        {
            let block = match function.split_once("::") {
                Some((type_name, method)) => format!(
                    "pub struct {type_name};\nimpl {type_name} {{\n    pub fn {method}(&self) {{ {mint_expr} }}\n}}\n"
                ),
                None => format!("pub fn {function}() {{ {mint_expr} }}\n"),
            };
            by_module
                .entry((crate_src, module))
                .or_default()
                .push_str(&block);
        }
        for ((crate_src, module), body) in by_module {
            let relative = format!("{crate_src}/{}.rs", module.replace("::", "/"));
            write(root, &relative, &body);
        }
        if let Some(marker) = rule.requires_path.filter(|path| !root.join(path).exists()) {
            write(root, marker, "");
        }
    }

    /// tc_arch_lint_api_surface_001: module-path mapping matches Rust's own
    /// `mod.rs`/`foo.rs` convention and the crate-root special case.
    #[trace("TC-157")]
    #[test]
    fn tc_arch_lint_api_surface_001_module_path_mapping() {
        assert_eq!(
            module_path_of(Path::new("value/semantic_node.rs")),
            "value::semantic_node"
        );
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
            "qsl-semantics/src/check/mod.rs",
            "pub use quire_exact::NodeKey;\n",
        );
        write(
            dir.path(),
            "qsl-semantics/src/library/mod.rs",
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
        write(
            dir.path(),
            "qsl-semantics/src/model/key.rs",
            "impl EffectiveId {}\n",
        );
        write(
            dir.path(),
            "qsl-semantics/src/model/normalize.rs",
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
        write(
            dir.path(),
            "qsl-semantics/src/model/key.rs",
            "impl EffectiveId {}\n",
        );
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
    /// call to a `node_key_of` helper from a module outside T12-B's allowed
    /// list is a violation, the same as a direct `NodeKey::from_digest`
    /// call, so a minting helper that wraps the constructor cannot hide a
    /// mint outside `check`.
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

    /// tc_arch_lint_api_surface_022: a file declared out of line as
    /// `#[cfg(test)] mod tests;`, and a module under it, is test-only, so
    /// its mints are excluded like any other `#[cfg(test)]` item. The same
    /// declaration without `#[cfg(test)]` is shipped code, and its mint is a
    /// violation (negative control).
    #[trace("TC-157", "FR-060-AC-3")]
    #[test]
    fn tc_arch_lint_api_surface_022_cfg_test_mod_declaration_excludes_its_file() {
        let mint = "pub fn f() { let _ = NodeKey::from_digest([0; 32]); }\n";
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        let rule = &RULES[1]; // T12-B
        seed_debt_list_baseline(dir.path(), rule, "let _ = NodeKey::from_digest(x);");
        write(
            dir.path(),
            "src/value/widget.rs",
            "#[cfg(test)]\nmod tests;\nmod shipped;\n",
        );
        write(dir.path(), "src/value/widget/tests.rs", "mod nested;\n");
        write(dir.path(), "src/value/widget/tests/nested.rs", mint);
        write(dir.path(), "src/value/widget/shipped.rs", mint);
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.violations.len(), 1, "{:?}", outcome.violations);
        assert_eq!(outcome.violations[0].module, "value::widget::shipped");
        assert!(!outcome.passed());
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
            "qsl-semantics/src/model/population.rs",
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
            "qsl-semantics/src/model/population.rs",
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
            "qsl-semantics/src/model/population.rs",
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
        fs::create_dir_all(dir.path().join("src")).unwrap();
        write(
            dir.path(),
            "qsl-semantics/src/model/population.rs",
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
            "qsl-semantics/src/model/population.rs",
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
    /// `.map(NodeKey::from_digest)`, the shape the deleted
    /// `value::node::NodeIdDocument::key` had -- is caught the same as an
    /// ordinary call from a disallowed module. Both spellings are planted in the same
    /// disallowed module (`value::reference`, not on T12-B's debt list) to
    /// prove neither is missed.
    #[trace("TC-157", "FR-060-AC-3")]
    #[test]
    fn tc_arch_lint_api_surface_017_path_form_call_is_a_violation() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(
            dir.path(),
            "qsl-semantics/src/check/mod.rs",
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

    /// tc_arch_lint_api_surface_018 (ADR-011 §7.3 X-10, QSL-185; X-4,
    /// QSL-179; X-5, QSL-180; X-6, QSL-181; X-9, QSL-184): a `Role::Qsl` rule
    /// scans `qsl-replay/src/`, `qsl-source/src/`, `qsl-forms/src/`,
    /// `qsl-semantics/src/` and `qsl-route/src/` too, the same way tc_arch_lint_api_surface_014/016
    /// cover `qsl-foundation/src/` and `qsl-cst/src/` -- each extracted layer
    /// crate is as much "QSL's own tree" as the root crate. Each crate gets
    /// its own checkout so one crate's violation cannot stand in for the
    /// other's.
    #[trace("TC-157", "FR-060-AC-3")]
    #[test]
    fn tc_arch_lint_api_surface_018_qsl_replay_and_qsl_source_crates_are_scanned() {
        for (file, module) in [
            ("qsl-replay/src/identity.rs", "identity"),
            ("qsl-source/src/preflight.rs", "preflight"),
            ("qsl-forms/src/dispatch.rs", "dispatch"),
            ("qsl-semantics/src/check/identity.rs", "check::identity"),
            ("qsl-route/src/lib.rs", ""),
        ] {
            let dir = tempfile::tempdir().unwrap();
            ensure_qsl_roots(dir.path());
            write(
                dir.path(),
                "qsl-semantics/src/model/population.rs",
                "impl PopulationId {}\n",
            );
            write(
                dir.path(),
                file,
                "fn f() {\n    let id = PopulationId::from_digest(bytes);\n}\n",
            );
            let rule = &RULES[3]; // T12-D
            let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
            assert_eq!(outcome.status, RuleStatus::Live, "{file}");
            assert_eq!(outcome.violations.len(), 1, "{file}");
            assert_eq!(outcome.violations[0].module, module, "{file}");
            // The module path alone does not name the crate (`qsl-route`'s
            // `lib.rs` and the root crate's are both `""`), so the file is
            // checked too.
            assert!(
                outcome.violations[0].file.ends_with(file),
                "{file}: {:?}",
                outcome.violations[0].file
            );
            assert!(!outcome.passed(), "{file}");
        }
    }

    /// tc_arch_lint_api_surface_019 (QSL-178 review F4, QSL-185, QSL-179,
    /// QSL-180, QSL-181, QSL-182, QSL-184): a checkout with no `qsl-replay/`,
    /// no `qsl-source/`, no `qsl-forms/`, no `qsl-package/`, no `qsl-route/`
    /// or no `qsl-semantics/` directory is an error, the same as tc_arch_lint_api_surface_015 for `qsl-foundation/`.
    /// Every listed root is required precisely so a crate rename or move this
    /// scanner's root list has not caught up with fails loudly instead of
    /// silently scanning nothing there.
    #[trace("TC-157", "FR-060-AC-2")]
    #[test]
    fn tc_arch_lint_api_surface_019_missing_qsl_replay_or_qsl_source_crate_is_an_error() {
        for missing in [
            "qsl-replay/src",
            "qsl-source/src",
            "qsl-forms/src",
            "qsl-package/src",
            "qsl-eval/src",
            "qsl-route/src",
        ] {
            let dir = tempfile::tempdir().unwrap();
            ensure_qsl_roots(dir.path());
            fs::remove_dir_all(dir.path().join(missing)).unwrap();
            write(
                dir.path(),
                "qsl-semantics/src/model/population.rs",
                "fn f() {\n    let id = PopulationId::from_digest(bytes);\n}\n",
            );
            let rule = &RULES[3]; // T12-D
            let error = evaluate(rule, dir.path(), Some(dir.path())).unwrap_err();
            assert!(error.to_string().contains(missing), "{missing}: {error}");
        }
        // `qsl-semantics/` holds T12-D's own `requires_path` marker, so its
        // absence is checked through T12-B, which has none (QSL-181).
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        fs::remove_dir_all(dir.path().join("qsl-semantics")).unwrap();
        write(
            dir.path(),
            "src/value/enumeration.rs",
            "fn f() {\n    let key = NodeKey::from_digest(bytes);\n}\n",
        );
        let rule = &RULES[1]; // T12-B
        let error = evaluate(rule, dir.path(), Some(dir.path())).unwrap_err();
        assert!(error.to_string().contains("qsl-semantics/src"), "{error}");
    }

    // -------------------------------------------------------------------
    // FR-060 T12-B/T12-C/T12-D: the named, shrinking debt list,
    // `#[cfg(test)]`/comment/string exclusion, and the token-level patterns
    // (TC-157 steps 5-7).
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
            "qsl-semantics/src/check/family.rs",
            "fn mint_node_key(bytes: &[u8]) -> NodeKey {\n    NodeKey::from_digest(bytes)\n}\n",
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
        write(
            dir.path(),
            "qsl-semantics/src/check/mod.rs",
            "pub struct NodeKey;\n",
        );
        write(
            dir.path(),
            "qsl-semantics/src/library/mod.rs",
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
    /// reported as debt and does not fail T12-B. The entry names its crate
    /// (QSL-183): the same module and function in another crate is a
    /// violation, not debt.
    #[trace("TC-157", "FR-060-AC-4")]
    #[test]
    fn tc_157_debt_list_mint_is_reported_as_debt() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        // T12-B: debt list has qsl-eval's (value::expression::family, decode_v2)
        let rule = &RULES[1];
        seed_debt_list_baseline(dir.path(), rule, "let _ = NodeKey::from_digest(x);");
        // Overwrite `value::expression::family`'s seeded baseline with a
        // shaped-like-the-real-thing mint -- T12-B's debt list has exactly
        // one entry in this module, so nothing else to preserve.
        let mint =
            "fn decode_v2(bytes: [u8; 32]) -> NodeKey {\n    NodeKey::from_digest(bytes)\n}\n";
        write(dir.path(), "qsl-eval/src/value/expression/family.rs", mint);
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert!(outcome.violations.is_empty(), "{:?}", outcome.violations);
        assert!(
            outcome
                .debt
                .iter()
                .any(|site| site.module == "value::expression::family"
                    && site.function == "decode_v2"),
            "{:?}",
            outcome.debt
        );
        assert!(
            outcome.stale_debt_entries.is_empty(),
            "{:?}",
            outcome.stale_debt_entries
        );
        assert!(outcome.passed());

        // The same module path and function in the root crate is not the
        // listed debt.
        write(dir.path(), "src/value/expression/family.rs", mint);
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(
            outcome
                .violations
                .iter()
                .map(|site| (site.module.as_str(), site.function.as_str()))
                .collect::<Vec<_>>(),
            [("value::expression::family", "decode_v2")],
            "{:?}",
            outcome.violations
        );
        assert!(!outcome.passed());
    }

    /// TC-157 step 6: a debt-list entry whose function no longer mints
    /// fails T12-B and names the stale entry, so a fixed site cannot later
    /// hide a new mint under a name the list still carries.
    #[trace("TC-157", "FR-060-AC-4")]
    #[test]
    fn tc_157_stale_debt_entry_fails() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(
            dir.path(),
            "qsl-semantics/src/check/mod.rs",
            "pub struct NodeKey;\n",
        );
        write(
            dir.path(),
            "qsl-eval/src/value/expression/family.rs",
            "fn decode_v2() -> u8 {\n    0\n}\n",
        );
        let rule = &RULES[1]; // T12-B
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert!(outcome.violations.is_empty());
        assert!(outcome.debt.is_empty());
        assert!(outcome.stale_debt_entries.contains(&DebtEntry {
            crate_src: "qsl-eval/src",
            module: "value::expression::family",
            function: "decode_v2",
        }));
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
            "qsl-semantics/src/library/mod.rs",
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
            "qsl-semantics/src/library/mod.rs",
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
        write(
            dir.path(),
            "qsl-semantics/src/model/key.rs",
            "pub struct EffectiveId;\n",
        );
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

    /// TC-157 step 7: a `"/*"` inside a string literal does not open a block
    /// comment -- the scan matches tokens, so the real mint after it is still
    /// reported.
    #[trace("TC-157", "FR-060-AC-4")]
    #[test]
    fn tc_157_block_comment_opener_in_a_string_does_not_hide_a_later_mint() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(
            dir.path(),
            "qsl-semantics/src/model/key.rs",
            "pub struct EffectiveId;\n",
        );
        write(
            dir.path(),
            "qsl-semantics/src/library/mod.rs",
            "fn glob() -> &'static str {\n    \"src/*.rs\"\n}\n\
             fn a_new_function(bytes: [u8; 32]) -> EffectiveId {\n    EffectiveId::from_digest(bytes)\n}\n",
        );
        let rule = &RULES[2]; // T12-C
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.violations.len(), 1, "{:?}", outcome.violations);
        assert_eq!(outcome.violations[0].line, 5);
        assert_eq!(outcome.violations[0].function, "a_new_function");
        assert!(!outcome.passed());
    }

    /// TC-157 step 7: `.map(EffectiveId::from_digest)` -- the constructor
    /// passed as a function value -- is a T12-C mint.
    #[trace("TC-157", "FR-060-AC-4")]
    #[test]
    fn tc_157_t12c_function_value_mint_fails() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(
            dir.path(),
            "qsl-semantics/src/model/key.rs",
            "pub struct EffectiveId;\n",
        );
        write(
            dir.path(),
            "qsl-semantics/src/library/mod.rs",
            "fn f(bytes: Option<[u8; 32]>) -> Option<EffectiveId> {\n    bytes.map(EffectiveId::from_digest)\n}\n",
        );
        let rule = &RULES[2]; // T12-C
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.violations.len(), 1, "{:?}", outcome.violations);
        assert_eq!(outcome.violations[0].line, 2);
        assert_eq!(outcome.violations[0].function, "f");
        assert!(!outcome.passed());
    }

    /// TC-157 step 7: a call split across lines (`EffectiveId::from_digest`
    /// on one line, its arguments on the next) is a mint, reported at the
    /// line the path starts on.
    #[trace("TC-157", "FR-060-AC-4")]
    #[test]
    fn tc_157_t12c_call_split_across_lines_fails() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(
            dir.path(),
            "qsl-semantics/src/model/key.rs",
            "pub struct EffectiveId;\n",
        );
        write(
            dir.path(),
            "qsl-semantics/src/library/mod.rs",
            "fn f(bytes: [u8; 32]) -> EffectiveId {\n    EffectiveId\n        ::\n        from_digest\n        (bytes)\n}\n",
        );
        let rule = &RULES[2]; // T12-C
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.violations.len(), 1, "{:?}", outcome.violations);
        assert_eq!(outcome.violations[0].line, 2);
        assert!(!outcome.passed());
    }

    /// TC-157 step 6: the qualified-path spelling `<NodeKey>::from_digest`
    /// and a call inside a macro's arguments are both T12-B mints; the
    /// constructor named only inside a string literal is not.
    #[trace("TC-157", "FR-060-AC-4")]
    #[test]
    fn tc_157_qualified_path_and_macro_argument_mints_fail() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        let rule = &RULES[1]; // T12-B
        seed_debt_list_baseline(dir.path(), rule, "let _ = NodeKey::from_digest(x);");
        write(
            dir.path(),
            "qsl-semantics/src/library/mod.rs",
            "fn a(bytes: [u8; 32]) -> NodeKey {\n    <NodeKey>::from_digest(bytes)\n}\n\
             fn b(bytes: [u8; 32]) -> Vec<NodeKey> {\n    vec![NodeKey::from_digest(bytes)]\n}\n\
             fn c() -> &'static str {\n    \"NodeKey::from_digest\"\n}\n",
        );
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        let lines: Vec<usize> = outcome.violations.iter().map(|site| site.line).collect();
        assert_eq!(lines, vec![2, 5], "{:?}", outcome.violations);
        assert!(!outcome.passed());
    }

    /// TC-157 step 6: `node_key_of`'s own definition is not a mint, but a
    /// call of it split across lines is.
    #[trace("TC-157", "FR-060-AC-4")]
    #[test]
    fn tc_157_helper_definition_is_not_a_mint_but_a_split_call_is() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        let rule = &RULES[1]; // T12-B
        seed_debt_list_baseline(dir.path(), rule, "let _ = NodeKey::from_digest(x);");
        write(
            dir.path(),
            "qsl-semantics/src/library/mod.rs",
            "fn node_key_of(x: u8) -> u8 {\n    x\n}\n\
             fn caller() -> u8 {\n    node_key_of\n        (1)\n}\n",
        );
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.violations.len(), 1, "{:?}", outcome.violations);
        assert_eq!(outcome.violations[0].line, 5);
        assert_eq!(outcome.violations[0].function, "caller");
    }

    /// TC-157 step 5: T12-D scans shipped code only -- a `PopulationId`
    /// built from literal bytes inside `#[cfg(test)] mod tests` outside
    /// `model` (the shape at `qsl-eval/src/value/expression/evaluate.rs`) is not a
    /// mint, and T12-D passes with zero call sites.
    #[trace("TC-157", "FR-060-AC-4")]
    #[test]
    fn tc_157_t12d_cfg_test_mint_is_not_reported() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(
            dir.path(),
            "qsl-semantics/src/model/population.rs",
            "pub struct PopulationId;\n",
        );
        write(
            dir.path(),
            "qsl-eval/src/value/expression/evaluate.rs",
            "pub fn run() {}\n#[cfg(test)]\nmod tests {\n    fn f() {\n        let _ = PopulationId::from_digest([7; 32]);\n    }\n}\n",
        );
        let rule = &RULES[3]; // T12-D
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert!(outcome.violations.is_empty(), "{:?}", outcome.violations);
        assert!(outcome.debt.is_empty(), "{:?}", outcome.debt);
        assert!(outcome.passed());
    }
    /// tc_arch_lint_api_surface_023 (T12-E, TC-157, FR-060-AC-3, FR-087-AC-1): a call to
    /// the condition-1 witness minter from any module outside
    /// `qsl-package`'s `checked_v2` -- including `library`, which defines
    /// it in `qsl-semantics`, and another `qsl-package` module -- is a
    /// violation, whether spelled through the type, through `Self`, as a
    /// function value or across the crate boundary
    /// (`qsl_semantics::library::SupportedV2Wire::..`, QSL-181), and so is a
    /// `checked_v2` module in another crate; the one call in `qsl-package`'s
    /// `checked_v2` and the minter's own definition are not.
    #[trace("TC-157", "FR-060-AC-3", "FR-087-AC-1")]
    #[test]
    fn tc_arch_lint_api_surface_023_witness_minter_outside_checked_v2_is_a_violation() {
        let dir = tempfile::tempdir().unwrap();
        ensure_qsl_roots(dir.path());
        write(
            dir.path(),
            "qsl-semantics/src/library/witness.rs",
            "pub struct SupportedV2Wire(());\n\
             impl SupportedV2Wire {\n\
             \x20   pub fn attest_ir_admitted_v2() -> Self {\n\
             \x20       Self(())\n\
             \x20   }\n\
             \x20   fn again() -> Self {\n\
             \x20       Self::attest_ir_admitted_v2()\n\
             \x20   }\n\
             }\n",
        );
        write(
            dir.path(),
            "qsl-package/src/checked_v2.rs",
            "fn read() {\n    let _ = SupportedV2Wire::attest_ir_admitted_v2();\n}\n",
        );
        write(
            dir.path(),
            "qsl-package/src/emit.rs",
            "fn emit() {\n    let mint = SupportedV2Wire::attest_ir_admitted_v2;\n}\n",
        );
        write(
            dir.path(),
            "src/route.rs",
            "fn route() {\n    let _ = qsl_semantics::library::SupportedV2Wire::attest_ir_admitted_v2();\n}\n",
        );
        // A module of the same path in another crate is not the allowed
        // caller: the allow-list names `qsl-package`'s `checked_v2`.
        write(
            dir.path(),
            "src/checked_v2.rs",
            "fn read() {\n    let _ = SupportedV2Wire::attest_ir_admitted_v2();\n}\n",
        );
        let rule = RULES.iter().find(|rule| rule.id == "T12-E").unwrap();
        let outcome = evaluate(rule, dir.path(), Some(dir.path())).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        let found: Vec<(&str, usize, &str)> = outcome
            .violations
            .iter()
            .map(|site| (site.module.as_str(), site.line, site.function.as_str()))
            .collect();
        assert_eq!(
            found,
            vec![
                ("emit", 2, "emit"),
                ("library::witness", 7, "SupportedV2Wire::again"),
                ("checked_v2", 2, "read"),
                ("route", 2, "route"),
            ],
            "{:?}",
            outcome.violations
        );
        assert!(!outcome.passed());
    }

    /// tc_arch_lint_api_surface_024 (T12-E on this repository's own tree):
    /// the witness minter's only shipped caller is
    /// `qsl-package`'s `checked_v2`. This is the CI gate for the rule: it
    /// runs in `cargo test --workspace` (`make ci`), where the `arch-lint`
    /// binary itself does not yet run.
    #[trace("TC-157", "FR-060-AC-2", "FR-087-AC-1")]
    #[test]
    fn tc_arch_lint_api_surface_024_witness_minter_live_tree_has_no_other_caller() {
        let qsl_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        assert_is_qsl_root(&qsl_root).unwrap();
        let rule = RULES.iter().find(|rule| rule.id == "T12-E").unwrap();
        let outcome = evaluate(rule, &qsl_root, Some(&qsl_root)).unwrap();
        assert_eq!(outcome.status, RuleStatus::Live);
        assert!(outcome.violations.is_empty(), "{:?}", outcome.violations);
        assert!(outcome.passed());
    }
}
