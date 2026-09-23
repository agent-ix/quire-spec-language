// SPDX-License-Identifier: AGPL-3.0-or-later
//! The lossless complete-V1 concrete syntax tree: exact definition/model
//! selection digests and identities, the recovering node/token tree, and
//! incremental whitespace-only editing.
use std::collections::BTreeSet;

use qsl_foundation::digest::InvalidDigest;
use qsl_foundation::{ByteDigest, Source, Span};

/// Exact versioned definition digest in the profile/import domain.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DefinitionDigest(ByteDigest);

impl DefinitionDigest {
    /// Parse the canonical SHA-256 spelling selected by source.
    pub fn parse(value: &str) -> Result<Self, InvalidDigest> {
        value.parse().map(Self)
    }

    /// Wrap an already-computed raw-byte digest.
    // Widened to `pub`: the root crate's `complete::package` calls it
    // across the crate boundary (ADR-011 §7.3 X-3).
    pub fn from_digest(digest: ByteDigest) -> Self {
        Self(digest)
    }

    /// Canonical selected value.
    pub fn digest(self) -> ByteDigest {
        self.0
    }
}

/// Exact definition identity/version/digest triple.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DefinitionRef {
    /// Opaque definition identity.
    identity: String,
    /// Exact selected version.
    version: String,
    /// Exact content digest.
    digest: DefinitionDigest,
}

/// Why an identity/version pair failed [`DefinitionRef::new`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidDefinitionComponent {
    /// Identity was empty or exceeded 512 bytes.
    Identity,
    /// Version was empty or exceeded 256 bytes.
    Version,
}

impl DefinitionRef {
    /// Validate a non-empty exact definition selection.
    pub fn new(
        identity: impl Into<String>,
        version: impl Into<String>,
        digest: DefinitionDigest,
    ) -> Result<Self, InvalidDefinitionComponent> {
        let (identity, version) = (identity.into(), version.into());
        Self::validate_components(&identity, &version)?;
        Ok(Self::from_validated(identity, version, digest))
    }

    pub(crate) fn validate_components(
        identity: &str,
        version: &str,
    ) -> Result<(), InvalidDefinitionComponent> {
        if identity.is_empty() || identity.len() > 512 {
            return Err(InvalidDefinitionComponent::Identity);
        }
        if version.is_empty() || version.len() > 256 {
            return Err(InvalidDefinitionComponent::Version);
        }
        Ok(())
    }

    pub(crate) fn from_validated(
        identity: String,
        version: String,
        digest: DefinitionDigest,
    ) -> Self {
        Self {
            identity,
            version,
            digest,
        }
    }

    /// Opaque definition identity.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Exact selected version.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Exact raw-byte definition digest.
    pub fn digest(&self) -> DefinitionDigest {
        self.digest
    }
}

/// Raw-byte SHA-256 digest of one compiled-model document.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModelDigest(ByteDigest);

impl ModelDigest {
    /// Parse the canonical SHA-256 spelling selected by source.
    pub fn parse(value: &str) -> Result<Self, InvalidDigest> {
        value.parse().map(Self)
    }

    /// Wrap an already-computed raw-byte digest.
    // Widened to `pub`: the root crate's `complete::package` calls it
    // across the crate boundary (ADR-011 §7.3 X-3).
    pub fn from_digest(digest: ByteDigest) -> Self {
        Self(digest)
    }

    /// Canonical selected value.
    pub fn digest(self) -> ByteDigest {
        self.0
    }
}

/// Exact compiled-model identity/version/raw-byte-digest triple.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModelRef {
    identity: String,
    version: String,
    digest: ModelDigest,
}

impl ModelRef {
    /// Validate a non-empty exact compiled-model selection.
    pub fn new(
        identity: impl Into<String>,
        version: impl Into<String>,
        digest: ModelDigest,
    ) -> Result<Self, InvalidModelComponent> {
        let (identity, version) = (identity.into(), version.into());
        Self::validate_components(&identity, &version)?;
        Ok(Self::from_validated(identity, version, digest))
    }

    pub(crate) fn validate_components(
        identity: &str,
        version: &str,
    ) -> Result<(), InvalidModelComponent> {
        DefinitionRef::validate_components(identity, version).map_err(|component| match component {
            InvalidDefinitionComponent::Identity => InvalidModelComponent::Identity,
            InvalidDefinitionComponent::Version => InvalidModelComponent::Version,
        })
    }

    pub(crate) fn from_validated(identity: String, version: String, digest: ModelDigest) -> Self {
        Self {
            identity,
            version,
            digest,
        }
    }

    /// Opaque compiled-model identity.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Exact selected version.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Exact raw-byte compiled-model digest.
    pub fn digest(&self) -> ModelDigest {
        self.digest
    }
}

/// Why an identity/version pair failed [`ModelRef::new`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidModelComponent {
    /// Identity was empty or exceeded 512 bytes.
    Identity,
    /// Version was empty or exceeded 256 bytes.
    Version,
}

/// Source-located profile definition selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileSelection {
    /// Local alias used by declarations.
    pub alias: String,
    /// Exact definition triple.
    pub definition: DefinitionRef,
    /// Full profile declaration range.
    pub span: Span,
    /// Exact identity literal range used for located refusals.
    pub identity_span: Span,
}

/// Source-located import definition selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportSelection {
    /// Optional local alias.
    pub alias: Option<String>,
    /// Exact definition triple.
    pub definition: DefinitionRef,
    /// Full import declaration range.
    pub span: Span,
}

/// Source-located formal model selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelSelection {
    /// Local model alias.
    pub alias: String,
    /// Exact compiled-model document selection.
    pub model: ModelRef,
    /// Full model declaration range.
    pub span: Span,
}

/// Exact package-relevant selections recovered from the admitted syntax.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceSelections {
    /// Selected profile definitions in source order.
    pub profiles: Vec<ProfileSelection>,
    /// Selected package imports in source order.
    pub imports: Vec<ImportSelection>,
    /// Selected model exports in source order.
    pub models: Vec<ModelSelection>,
}

/// Public lossless leaf classification; token spellings remain exact bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenClass {
    /// Significant grammar token.
    Token,
    /// Exact whitespace bytes omitted by the generated lexer.
    Whitespace,
    /// Exact line-comment bytes.
    Comment,
    /// Invalid bytes retained in a recovering CST.
    Invalid,
}

/// Public lexical kind retained for every lossless leaf.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenKind {
    /// Reserved word, delimiter or operator.
    Grammar,
    /// User-defined ASCII identifier.
    Identifier,
    /// Unsigned decimal integer literal.
    Integer,
    /// JSON-compatible quoted text literal.
    Text,
    /// `0x` prefix of a bit-exact float literal.
    HexPrefix,
    /// One lowercase hexadecimal digit.
    HexDigit,
    /// Whitespace trivia.
    Whitespace,
    /// Line-comment trivia.
    Comment,
    /// Lexically invalid retained bytes.
    Invalid,
}

// A production with real, non-obvious grammar shape may carry an explicit
// `#[doc = "..."]` immediately before its name in the `productions!` call
// below; this TT-muncher gives that production's hand-written doc instead of
// the mechanical one. A production with no such override gets the mechanical
// "The `Name` production ..." doc, which is deliberately fine for the many
// self-evident productions (e.g. `EnumDeclaration`, `Field`). See
// `grammar.rs` for every production's actual rule.
macro_rules! productions {
    ($($input:tt)*) => {
        productions_impl! { @collect [] [] $($input)* }
    };
}

macro_rules! productions_impl {
    (@collect [$($variants:tt)*] [$($idents:tt)*]) => {
        /// Every named grammar production represented by the complete-V1 parser.
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub enum Production {
            $($variants)*
        }

        impl Production {
            /// Closed production inventory used by grammar/corpus coverage gates.
            pub const fn all() -> &'static [Self] {
                &[$($idents)*]
            }
        }
    };
    (@collect [$($variants:tt)*] [$($idents:tt)*] #[doc = $doc:literal] $production:ident, $($rest:tt)*) => {
        productions_impl! {
            @collect
            [$($variants)* #[doc = $doc] $production,]
            [$($idents)* Self::$production,]
            $($rest)*
        }
    };
    (@collect [$($variants:tt)*] [$($idents:tt)*] #[doc = $doc:literal] $production:ident) => {
        productions_impl! {
            @collect
            [$($variants)* #[doc = $doc] $production,]
            [$($idents)* Self::$production,]
        }
    };
    (@collect [$($variants:tt)*] [$($idents:tt)*] $production:ident, $($rest:tt)*) => {
        productions_impl! {
            @collect
            [$($variants)* #[doc = concat!(
                "The `", stringify!($production), "` production of the complete-V1 grammar."
            )] $production,]
            [$($idents)* Self::$production,]
            $($rest)*
        }
    };
    (@collect [$($variants:tt)*] [$($idents:tt)*] $production:ident) => {
        productions_impl! {
            @collect
            [$($variants)* #[doc = concat!(
                "The `", stringify!($production), "` production of the complete-V1 grammar."
            )] $production,]
            [$($idents)* Self::$production,]
        }
    };
}

productions! {
    CompleteUnit, Header, Profile, ImportDeclaration, Model, Declaration,
    TypeReference, QualifiedName, ModelName, TypeName, OperationName, ParameterType,
    RoundingMode, TextProfile,
    DimensionDeclaration, DimensionTerm, UnitDeclaration, EnumDeclaration,
    EnumMember, RecordDeclaration, Field, TupleDeclaration, AliasDeclaration,
    FunctionDeclaration, Predicate, Parameter, StateClause, Block, Expression,
    Implication, Disjunction, Conjunction, Comparison, Sum, Product, Unary,
    Postfix, Primary, ExactNumber, FloatValue,
    #[doc = "`0x` followed by exactly 8 hexadecimal digits: the bit-exact `float32` literal payload in `float32(bits: <Hex32>)`."]
    Hex32,
    #[doc = "`0x` followed by exactly 16 hexadecimal digits: the bit-exact `float64` literal payload in `float64(bits: <Hex64>)`."]
    Hex64,
    HexDigit,
    EnumValue, CollectionValue, RecordValue, FieldValue, TupleValue,
    CollectionCall, SignedInteger, TemporalClause,
    #[doc = "`on origin` or `on each ( <parameter> ) [when ( <expression> )]`: which events start a `temporal` clause."]
    Activation,
    #[doc = "`capture <parameter> = <expression> ;`: binds a named value inside a `temporal` clause body."]
    Capture,
    Interval, TemporalExpression, TemporalImplication, TemporalDisjunction,
    TemporalConjunction, TemporalRelation, TemporalUnary, TemporalPrimary,
    ProtocolClause, Role, RoleLifetime, Relationship, Channel, Ordering, DeliveryPolicy,
    Capacity, OverflowPolicy, ProtocolRequirement, NodeReference, Compensation,
    Control, Sequence,
    #[doc = "`visible ( <expression-list>? )`: attached to `choice` and `repeat` control blocks."]
    Visibility,
    Choice, Case, Parallel,
    #[doc = "`all`, `any`, `quorum ( <count> )`, or `predicate ( <identifier> )`: how a `parallel` block's branches converge."]
    JoinPolicy,
    Branch,
    Repetition, AwaitControl,
    #[doc = "`related by <identifier> ( <expression>, <expression> )`: attached to an event node to correlate it with another by a named relation."]
    Related,
    EventNode, Check, Commit, Finish,
    RelationClause, ExecutionBinding, HyperClause, TraceDomain, Quantifier,
    HybridDeclaration, HybridMode, Equation, SynthesisDeclaration,
    VerificationPlan, VerificationStep,
}

/// Stable within unchanged exact bytes and ancestor-production path.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct StableNodeId(String);

/// Revision-bound CST identity required when exchanging a node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeIdentity {
    /// Digest of the caller-selected source revision label.
    pub source_revision_digest: ByteDigest,
    /// Exact node byte range in that revision.
    pub span: Span,
    /// Child-node ordinals from the root production.
    pub structural_path: Vec<u32>,
    /// Production kinds from the root parent through this node's parent.
    pub ancestor_productions: Vec<Production>,
}

/// Ordered CST child reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CstElement {
    /// Index into [`LosslessCst::tokens`].
    Token(usize),
    /// Index into [`LosslessCst::nodes`].
    Node(usize),
}

/// Exact lossless source leaf.
#[derive(Clone, Debug)]
pub struct CstToken {
    class: TokenClass,
    kind: TokenKind,
    span: Span,
    spelling: Box<[u8]>,
}

impl CstToken {
    /// Leaf classification.
    pub fn class(&self) -> TokenClass {
        self.class
    }
    /// Exact lexical kind.
    pub fn kind(&self) -> TokenKind {
        self.kind
    }
    /// Exact half-open source range.
    pub fn span(&self) -> Span {
        self.span
    }
    /// Exact original bytes.
    pub fn spelling(&self) -> &[u8] {
        &self.spelling
    }
}

/// Named interior grammar node.
#[derive(Clone, Debug)]
pub struct CstNode {
    production: Production,
    span: Span,
    children: Vec<CstElement>,
    identity: NodeIdentity,
    stable_id: StableNodeId,
}

impl CstNode {
    /// Named grammar production.
    pub fn production(&self) -> Production {
        self.production
    }
    /// Exact half-open source range.
    pub fn span(&self) -> Span {
        self.span
    }
    /// Ordered lossless children.
    pub fn children(&self) -> &[CstElement] {
        &self.children
    }
    /// Revision-bound exchange identity.
    pub fn identity(&self) -> &NodeIdentity {
        &self.identity
    }
    /// Revision-independent reuse identity for unchanged byte-correspondent regions.
    pub fn stable_id(&self) -> &StableNodeId {
        &self.stable_id
    }
}

/// How a parser-proposed [`Recovery`] would repair the source at its span.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryKind {
    /// Proposed zero-width insertion.
    Insert,
    /// Proposed deletion of a fixed-width source region.
    Delete,
}

/// Proposed edit used to explain recovery. It never mutates source bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Recovery {
    /// Proposed edit operation.
    pub kind: RecoveryKind,
    /// Source-correspondent edit range.
    pub span: Span,
    /// Human-readable grammar expectation.
    pub expected: String,
}

/// Exact CST plus a separate non-mutating recovery stream.
#[derive(Clone, Debug)]
pub struct LosslessCst {
    source: Source,
    tokens: Vec<CstToken>,
    nodes: Vec<CstNode>,
    root: usize,
    recoveries: Vec<Recovery>,
}

impl LosslessCst {
    pub(crate) fn new(
        source: Source,
        tokens: Vec<CstToken>,
        mut nodes: Vec<RawNode>,
        root: usize,
        recoveries: Vec<Recovery>,
    ) -> Self {
        materialize_elements(&tokens, &mut nodes);
        let mut revision_preimage = b"quire.complete.document-revision/1\0".to_vec();
        let source_digest = source.digest().to_string();
        for value in [
            source.identity().identity.as_bytes(),
            source.identity().revision.as_bytes(),
            source_digest.as_bytes(),
        ] {
            revision_preimage
                .extend_from_slice(&u64::try_from(value.len()).unwrap_or(u64::MAX).to_be_bytes());
            revision_preimage.extend_from_slice(value);
        }
        let revision_digest = ByteDigest::of(&revision_preimage);
        let mut paths = vec![Vec::new(); nodes.len()];
        let mut ancestors = vec![Vec::new(); nodes.len()];
        let mut stable_occurrences = vec![0_u32; nodes.len()];
        IdentityTraversal {
            nodes: &nodes,
            source: &source,
            paths: &mut paths,
            ancestors: &mut ancestors,
            stable_occurrences: &mut stable_occurrences,
            occurrences: std::collections::BTreeMap::new(),
        }
        .assign(root, &[], &[]);
        let nodes = nodes
            .drain(..)
            .enumerate()
            .map(|(index, node)| {
                let path = std::mem::take(&mut paths[index]);
                let ancestor_productions = std::mem::take(&mut ancestors[index]);
                let occurrence = stable_occurrences[index];
                let mut stable = b"quire.complete.cst-node/1\0".to_vec();
                stable.extend_from_slice(source.identity().identity.as_bytes());
                stable.extend_from_slice(format!("|{:?}|", node.production).as_bytes());
                stable.extend_from_slice(source.slice(node.span).unwrap_or_default().as_bytes());
                stable.extend_from_slice(b"|");
                for production in &ancestor_productions {
                    stable.extend_from_slice(format!("{production:?}/").as_bytes());
                }
                stable.extend_from_slice(b"|");
                stable.extend_from_slice(&occurrence.to_be_bytes());
                CstNode {
                    production: node.production,
                    span: node.span,
                    children: node.children,
                    identity: NodeIdentity {
                        source_revision_digest: revision_digest,
                        span: node.span,
                        structural_path: path,
                        ancestor_productions,
                    },
                    stable_id: StableNodeId(ByteDigest::of(&stable).to_string()),
                }
            })
            .collect();
        Self {
            source,
            tokens,
            nodes,
            root,
            recoveries,
        }
    }

    /// Immutable source backing every token and node span.
    pub fn source(&self) -> &Source {
        &self.source
    }
    /// Lossless leaves in source order.
    pub fn tokens(&self) -> &[CstToken] {
        &self.tokens
    }
    /// Named production nodes.
    pub fn nodes(&self) -> &[CstNode] {
        &self.nodes
    }
    /// Complete-unit root.
    pub fn root(&self) -> &CstNode {
        &self.nodes[self.root]
    }
    /// Proposed recovery edits, never applied to source bytes.
    pub fn recoveries(&self) -> &[Recovery] {
        &self.recoveries
    }
    /// Reproduce every original source byte in order.
    pub fn render(&self) -> Vec<u8> {
        self.tokens
            .iter()
            .flat_map(|token| token.spelling.iter().copied())
            .collect()
    }
    /// Render one node through its ordered token/node children.
    pub fn render_node(&self, node: &CstNode) -> Result<Vec<u8>, Box<super::CompleteDiagnostic>> {
        let Some(index) = self
            .nodes
            .iter()
            .position(|candidate| candidate.identity() == node.identity())
        else {
            return Err(super::diagnostic::error(
                &self.source,
                super::CompleteCode::InvalidSourceIdentity,
                super::CompleteCause::Host(super::HostCause::ForeignNode),
                qsl_foundation::Phase::SourceMap,
                0,
                0,
                "CST node belongs to a different parsed source",
            ));
        };
        fn append(cst: &LosslessCst, element: CstElement, output: &mut Vec<u8>) {
            match element {
                CstElement::Token(token) => output.extend_from_slice(cst.tokens[token].spelling()),
                CstElement::Node(node) => {
                    for child in &cst.nodes[node].children {
                        append(cst, *child, output);
                    }
                }
            }
        }

        let mut output = Vec::new();
        for child in &self.nodes[index].children {
            append(self, *child, &mut output);
        }
        Ok(output)
    }
    /// Stable reuse identities for comparison with a reparsed revision.
    pub fn stable_node_ids(&self) -> BTreeSet<StableNodeId> {
        self.nodes
            .iter()
            .map(|node| node.stable_id.clone())
            .collect()
    }
    /// Smallest named node that completely contains the requested range.
    pub fn node_covering(&self, span: Span) -> Option<&CstNode> {
        self.nodes
            .iter()
            .filter(|node| node.span.start <= span.start && node.span.end >= span.end)
            .min_by_key(|node| node.span.end.saturating_sub(node.span.start))
    }

    /// Apply a single whitespace-only insertion to this CST without
    /// reparsing, or `None` if the fast path does not apply. Kept
    /// `pub(crate)`: `ParsedSource::with_whitespace_insertion` is the only
    /// public entry point, since it also checks the predecessor parse is
    /// admissible and pairs the result through `ParsedSource::from_parts`
    /// (QSL-178 review F2).
    pub(crate) fn with_whitespace_insertion(
        &self,
        source: Source,
        at: usize,
        inserted: &str,
    ) -> Option<Self> {
        if !self.recoveries.is_empty() || !crate::token::is_lexer_whitespace(inserted) {
            return None;
        }
        let target = self.tokens.iter().position(|token| {
            token.class == TokenClass::Whitespace && token.span.start <= at && at <= token.span.end
        })?;
        let added = inserted.len();
        let mut tokens = Vec::with_capacity(self.tokens.len());
        for (index, existing) in self.tokens.iter().enumerate() {
            let mut token = existing.clone();
            if index == target {
                let offset = at.checked_sub(token.span.start)?;
                let mut spelling = token.spelling.to_vec();
                spelling.splice(offset..offset, inserted.bytes());
                if !crate::token::is_lexer_whitespace(std::str::from_utf8(&spelling).ok()?) {
                    return None;
                }
                token.spelling = spelling.into_boxed_slice();
                token.span.end = token.span.end.checked_add(added)?;
            } else if index > target {
                token.span.start = token.span.start.checked_add(added)?;
                token.span.end = token.span.end.checked_add(added)?;
            }
            tokens.push(token);
        }
        let mut nodes = Vec::with_capacity(self.nodes.len());
        for existing in &self.nodes {
            let mut span = existing.span;
            if span.start >= at {
                span.start = span.start.checked_add(added)?;
                span.end = span.end.checked_add(added)?;
            } else if span.end > at {
                span.end = span.end.checked_add(added)?;
            }
            nodes.push(RawNode {
                production: existing.production,
                span,
                children: existing
                    .children
                    .iter()
                    .filter_map(|element| match element {
                        CstElement::Node(index) => Some(CstElement::Node(*index)),
                        CstElement::Token(_) => None,
                    })
                    .collect(),
            });
        }
        nodes.get_mut(self.root)?.span = Span {
            start: 0,
            end: source.text().len(),
        };
        Some(Self::new(source, tokens, nodes, self.root, Vec::new()))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RawNode {
    pub production: Production,
    pub span: Span,
    pub children: Vec<CstElement>,
}

pub(crate) fn token(class: TokenClass, kind: TokenKind, span: Span, spelling: &[u8]) -> CstToken {
    CstToken {
        class,
        kind,
        span,
        spelling: spelling.into(),
    }
}

struct IdentityTraversal<'a> {
    nodes: &'a [RawNode],
    source: &'a Source,
    paths: &'a mut [Vec<u32>],
    ancestors: &'a mut [Vec<Production>],
    stable_occurrences: &'a mut [u32],
    occurrences: std::collections::BTreeMap<(Vec<Production>, Production, Vec<u8>), u32>,
}

impl IdentityTraversal<'_> {
    fn assign(&mut self, index: usize, path: &[u32], ancestor_path: &[Production]) {
        self.paths[index] = path.to_vec();
        self.ancestors[index] = ancestor_path.to_vec();
        let occurrence = self
            .occurrences
            .entry((
                ancestor_path.to_vec(),
                self.nodes[index].production,
                self.source
                    .slice(self.nodes[index].span)
                    .unwrap_or_default()
                    .as_bytes()
                    .to_vec(),
            ))
            .or_insert(0);
        self.stable_occurrences[index] = *occurrence;
        *occurrence = occurrence.saturating_add(1);
        let child_nodes: Vec<_> = self.nodes[index]
            .children
            .iter()
            .filter_map(|child| match child {
                CstElement::Node(node) => Some(*node),
                CstElement::Token(_) => None,
            })
            .collect();
        for (child_index, node) in child_nodes.into_iter().enumerate() {
            let mut child_path = path.to_vec();
            child_path.push(u32::try_from(child_index).unwrap_or(u32::MAX));
            let mut child_ancestors = ancestor_path.to_vec();
            child_ancestors.push(self.nodes[index].production);
            self.assign(node, &child_path, &child_ancestors);
        }
    }
}

fn materialize_elements(tokens: &[CstToken], nodes: &mut [RawNode]) {
    for index in 0..nodes.len() {
        let child_nodes: Vec<_> = nodes[index]
            .children
            .iter()
            .filter_map(|child| match child {
                CstElement::Node(node) => Some(*node),
                CstElement::Token(_) => None,
            })
            .collect();
        let span = nodes[index].span;
        let mut children = Vec::new();
        let mut cursor = span.start;
        for child in child_nodes {
            let child_span = nodes[child].span;
            append_tokens(tokens, cursor, child_span.start, &mut children);
            children.push(CstElement::Node(child));
            cursor = child_span.end.max(cursor);
        }
        append_tokens(tokens, cursor, span.end, &mut children);
        nodes[index].children = children;
    }
}

fn append_tokens(tokens: &[CstToken], start: usize, end: usize, output: &mut Vec<CstElement>) {
    let first = tokens.partition_point(|token| token.span.end <= start);
    output.extend(
        tokens[first..]
            .iter()
            .enumerate()
            .take_while(|(_, token)| token.span.end <= end)
            .map(|(offset, _)| CstElement::Token(first + offset)),
    );
}

impl LosslessCst {
    /// Test-only fixture: a lossless CST over the given significant token
    /// spellings (space-joined into the backing source text, in order), one
    /// root node — spanning only the last spelling, not the whole text; see
    /// below — and the given recovery stream.
    ///
    /// The real complete-V1 grammar admits only `edition "1-draft"` and a
    /// closed declaration keyword set (`grammar.rs`'s `CompleteUnit`/`Header`
    /// rules), so it cannot produce a CST carrying an out-of-catalog literal
    /// edition or an unrecognized leading token — exactly the shapes the S2
    /// `forms` stage's own tests need to exercise the carry-through and
    /// dispatch mechanism without depending on a real family (FR-067-AC-1,
    /// AC-7, AC-8; TC-167). This fixture builds the CST directly instead.
    ///
    /// `spellings` is the whole token stream (any leading header/prelude
    /// tokens, in order); its last entry is the "root construct" the S2
    /// forms stage dispatches on, so the root node's span covers only that
    /// last token, not the whole text — matching a single declaration's own
    /// CST subtree, whose leading token is its own, not the file header's.
    /// The root node has no children: nothing in `forms` reads a node's
    /// children, only [`Self::tokens`] (the whole stream) and
    /// [`Self::root`]'s own span.
    ///
    /// Gated on `feature = "test-support"` (rather than a bare
    /// `#[cfg(test)]`) because `cfg(test)` gates only qsl-cst's own test
    /// build, never a downstream crate's: `qsl-forms`'s `dispatch` unit
    /// tests are the only caller, and reach this through its
    /// `[dev-dependencies]` enabling the feature, in both the
    /// default-feature and `--all-features` lanes (mirrors the repo's own
    /// convention, e.g. `src/model/key.rs`'s `DeclarationKey::fixture`).
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture(spellings: &[&str], recoveries: Vec<Recovery>) -> Self {
        assert!(!spellings.is_empty(), "a fixture needs at least one token");
        let mut text = String::new();
        let mut tokens = Vec::with_capacity(spellings.len());
        let mut root_span = Span { start: 0, end: 0 };
        for (index, spelling) in spellings.iter().enumerate() {
            if index > 0 {
                text.push(' ');
            }
            let start = text.len();
            text.push_str(spelling);
            let end = text.len();
            tokens.push(token(
                TokenClass::Token,
                TokenKind::Grammar,
                Span { start, end },
                spelling.as_bytes(),
            ));
            if index == spellings.len() - 1 {
                root_span = Span { start, end };
            }
        }
        let source = qsl_foundation::Source::read(
            qsl_foundation::SourceIdentity {
                identity: "forms-fixture".into(),
                revision: "0".into(),
            },
            "forms-fixture",
            text.as_bytes(),
            qsl_foundation::source::MAX_SOURCE_BYTES,
        )
        .expect("fixture text is within the byte limit");
        let root = RawNode {
            production: Production::CompleteUnit,
            span: root_span,
            children: Vec::new(),
        };
        Self::new(source, tokens, vec![root], 0, recoveries)
    }
}
