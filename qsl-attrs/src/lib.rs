// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL#214 (FR-064): the `#[string_edge]` marker attribute.
//!
//! ADR-012 §9 fixes the rule that a string may select semantics only at a
//! listed edge, and that each edge function carries this marker so
//! `xtask string-edge` can find every edge mechanically instead of by
//! convention. The attribute is a pure identity transform: it returns the
//! annotated item unchanged. It carries no runtime behaviour and changes
//! nothing about what the marked function compiles to -- its only reader is
//! `xtask string-edge`'s source scan (FR-064-AC-1).
//!
//! A plain unregistered attribute (`#[string_edge]`) does not parse on
//! stable Rust outside a recognized tool namespace, so this is implemented
//! as the smallest possible attribute-position proc macro: it does not
//! inspect or rewrite the item at all.

use proc_macro::TokenStream;

/// Marks a function as a string-dispatch edge (ADR-012 §9). See the module
/// docs: this expands to exactly the input, unchanged.
///
/// No attribute arguments are admitted: there is nothing to configure --
/// the marker's presence is the whole signal `xtask string-edge` reads.
/// `#[string_edge(anything)]` is a compile error (PR #262 review, finding
/// F18): an earlier version silently dropped `attribute` instead, so a
/// typo'd or misremembered argument compiled cleanly and did nothing,
/// contradicting this doc's own claim that none are admitted.
///
/// The `compile_error!` is emitted **alongside** the unchanged item, not in
/// place of it (PR #262 review, nit): an earlier version of this arm
/// returned only the `compile_error!` token stream, dropping the annotated
/// item from the output entirely. That reported the real error, but then
/// cascaded a second, unrelated wave of "cannot find function/type" errors
/// at every call site that referenced the now-missing item -- noise that
/// buries the one real diagnostic. Keeping the item means a misspelled
/// `#[string_edge(anything)]` reports exactly one error, not a stack trace
/// of names that no longer resolve.
#[proc_macro_attribute]
pub fn string_edge(attribute: TokenStream, item: TokenStream) -> TokenStream {
    if !attribute.is_empty() {
        let mut output: TokenStream = "compile_error!(\"#[string_edge] takes no arguments\");"
            .parse()
            .expect("static compile_error! literal always parses");
        output.extend(item);
        return output;
    }
    item
}
