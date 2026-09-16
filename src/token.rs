// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-002: one declarative vocabulary; Logos generates the token recognizer.
use logos::Logos;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum LexError {
    #[default]
    Character,
    LeadingZero,
    String,
}

fn integer(lex: &mut logos::Lexer<'_, Kind>) -> Result<String, LexError> {
    let value = lex.slice();
    if value.len() > 1 && value.starts_with('0') {
        return Err(LexError::LeadingZero);
    }
    Ok(value.into())
}

// Recognize the complete quoted region, then let the JSON implementation
// validate escapes, controls and surrogate pairs. No second escape grammar.
fn string(lex: &mut logos::Lexer<'_, Kind>) -> Result<String, LexError> {
    serde_json::from_str(lex.slice()).map_err(|_| LexError::String)
}

pub(crate) fn is_lexer_whitespace(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b' ' | b'\t' | b'\n' => index += 1,
            b'\r' if bytes.get(index + 1) == Some(&b'\n') => index += 2,
            _ => return false,
        }
    }
    !bytes.is_empty()
}

macro_rules! vocabulary {
    (
        $($base_variant:ident => $base_text:literal),+;
        complete: $($complete_variant:ident => $complete_text:literal),+ $(,)?
    ) => {
        #[derive(Logos, Clone, Debug, PartialEq)]
        #[logos(error = LexError)]
        #[logos(skip r"[ \t\n]+|\r\n")]
        pub(crate) enum Kind {
            $(#[token($base_text)] $base_variant,)+
            $(#[token($complete_text)] $complete_variant,)+
            #[regex(r"[A-Za-z_][A-Za-z0-9_]*", |lex| lex.slice().to_owned())]
            Identifier(String),
            #[regex(r"[0-9]+", integer)]
            Integer(String),
            #[regex(r#""([^"\\\x00-\x1f]|\\[^\r\n])*""#, string)]
            Text(String),
            #[regex(r"[0-9]+(\.[0-9]+([eE][+-]?[0-9]+)?|[eE][+-]?[0-9]+)")]
            Fractional,
            #[regex(r"[0-9]+(\.[0-9]+)?[eE][+-]?")]
            BadExponent,
            #[regex(r"helper|rec|cast|Tuple", priority = 3)]
            Unsupported,
            #[regex(r"0x[0-9a-f]+", |lex| lex.slice().to_owned(), priority = 4)]
            Hex(String),
            #[regex(r"//[^\r\n]*", allow_greedy = true)]
            Comment,
            End,
        }
        macro_rules! complete_only_kind_pattern {
            () => { $(Kind::$complete_variant)|+ };
        }
        impl Kind {
            pub(crate) fn is_word(&self) -> bool {
                match self {
                    $(Self::$base_variant => $base_text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'),)+
                    $(Self::$complete_variant => $complete_text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'),)+
                    Self::Identifier(_) | Self::Unsupported => true,
                    _ => false,
                }
            }
            pub(crate) fn description(&self) -> &'static str {
                match self {
                    $(Self::$base_variant => $base_text,)+
                    $(Self::$complete_variant => $complete_text,)+
                    Self::Identifier(_) => "identifier", Self::Integer(_) => "integer",
                    Self::Text(_) => "quoted string", Self::Fractional => "fractional number",
                    Self::BadExponent => "exponent digits", Self::Unsupported => "unsupported construct",
                    Self::Hex(_) => "hexadecimal bit pattern",
                    Self::End => "end of source",
                    Self::Comment => "comment",
                }
            }
            fn complete_only(&self) -> bool {
                matches!(self, $(Self::$complete_variant)|+ | Self::Hex(_))
            }
        }
    }
}

vocabulary! {
    Language => "language", Edition => "edition", Profile => "profile", Model => "model",
    Version => "version", Digest => "digest", Invariant => "invariant", On => "on", At => "at",
    Current => "current", Pre => "pre", Post => "post", Let => "let", In => "in", If => "if",
    Then => "then", Else => "else", Implies => "implies", Or => "or", And => "and", Not => "not",
    Div => "div", Rem => "rem", SelfValue => "self", ResultValue => "result", True => "true",
    False => "false", Present => "present", Value => "value", Deref => "deref", Size => "size",
    Forall => "forall", Exists => "exists", Reaches => "reaches",
    Using => "using", Predicate => "predicate", BooleanType => "Boolean",
    Contains => "contains", Filter => "filter", Map => "map", Count => "count", Sum => "sum",
    Rational => "rational", Mod => "mod", Temporal => "temporal", Protocol => "protocol",
    Over => "over", Clock => "clock", Origin => "origin", Each => "each", When => "when",
    Capture => "capture", Holds => "holds", Eventually => "eventually", Always => "always",
    Until => "until", Release => "release", Once => "once", Historically => "historically",
    Since => "since", Triggered => "triggered", Role => "role", Relationship => "relationship",
    Channel => "channel", From => "from", To => "to", Carries => "carries",
    Ordering => "ordering", Unordered => "unordered", Fifo => "fifo", By => "by",
    Delivery => "delivery", Requires => "requires", Compensate => "compensate", For => "for",
    As => "as", Activate => "activate", First => "first", Within => "within",
    Attempts => "attempts", Of => "of", Retry => "retry", Commit => "commit", Never => "never",
    Recover => "recover", Run => "run", Sequence => "sequence", Choice => "choice",
    Visible => "visible", Case => "case", Parallel => "parallel", Branch => "branch",
    Join => "join", All => "all", Repeat => "repeat", Max => "max", While => "while",
    Exhausted => "exhausted", Await => "await", After => "after", Match => "match",
    Timeout => "timeout", Send => "send", Via => "via", Receive => "receive", Attempt => "attempt",
    Contracts => "contracts", Effect => "effect", Event => "event", Related => "related",
    Check => "check", Finish => "finish",
    OpenParen => "(", CloseParen => ")", OpenBrace => "{", CloseBrace => "}",
    OpenBracket => "[", CloseBracket => "]", Semicolon => ";", Colon => ":", Comma => ",",
    Dot => ".", Equal => "=", Less => "<", Greater => ">", Plus => "+", Minus => "-",
    Star => "*", Slash => "/", Qualify => "::", NotEqual => "!=", LessEqual => "<=", GreaterEqual => ">=";
    complete:
    Import => "import", Dimension => "dimension", Unit => "unit", Ordered => "ordered",
    Enum => "enum", Record => "record", Tuple => "tuple", Type => "type",
    Function => "function", Pure => "pure", Decreases => "decreases",
    IntegerType => "Integer", IntType => "Int", RationalType => "Rational",
    DecimalType => "Decimal", Float32Type => "Float32", Float64Type => "Float64",
    TextType => "Text", OptionType => "Option", SequenceType => "Sequence",
    SetType => "Set", BagType => "Bag", OrderedSetType => "OrderedSet",
    ReferenceType => "Reference", Null => "null", NoneValue => "none",
    Decimal => "decimal", Float32 => "float32", Float64 => "float64", Bits => "bits",
    Convert => "convert", AllInstances => "allInstances", Collect => "collect",
    FlatMap => "flatMap", Flatten => "flatten", Fold => "fold", Reduce => "reduce",
    Identity => "identity", Exact => "exact",
    Nfc => "nfc", Nfd => "nfd", Nfkc => "nfkc", Nfkd => "nfkd",
    SetValue => "set", BagValue => "bag",
    OrderedSetValue => "orderedSet", Lifetime => "lifetime", Workflow => "workflow",
    Scope => "scope", Capacity => "capacity", Symbolic => "symbolic", Overflow => "overflow",
    Reject => "reject", Block => "block", Loss => "loss", Unknown => "unknown",
    Any => "any", Quorum => "quorum",
    Outstanding => "outstanding", Continue => "continue", Cancel => "cancel",
    Variant => "variant", Relation => "relation", Hyper => "hyper", Trace => "trace",
    Bounded => "bounded", Hybrid => "hybrid", Mode => "mode", Flow => "flow",
    Transition => "transition", Reset => "reset", Synthesis => "synthesis",
    Grammar => "grammar", Satisfies => "satisfies", Verify => "verify", Claim => "claim",
    Method => "method", Domain => "domain", Depends => "depends", Bound => "bound",
    Caret => "^", Question => "?", Apostrophe => "'"
}

impl Kind {
    fn unreserved(self, spelling: &str) -> Self {
        if matches!(
            spelling,
            "set"
                | "bag"
                | "Set"
                | "Bag"
                | "OrderedSet"
                | "collect"
                | "flatten"
                | "tuple"
                | "Decimal"
                | "Rational"
                | "allInstances"
        ) {
            return Self::Unsupported;
        }
        if identifier_spelling(spelling) {
            Self::Identifier(spelling.into())
        } else {
            Self::Unsupported
        }
    }

    /// Remove complete-facet reservations when parsing the composed base grammar.
    pub(crate) fn composed_base(self, spelling: &str) -> Self {
        if self.complete_only() {
            self.unreserved(spelling)
        } else {
            self
        }
    }

    // Edition classification uses the recognized token's original spelling;
    // it does not tokenize source again or reserve new words in old editions.
    pub(crate) fn historical(self, spelling: &str) -> Self {
        if self.complete_only() {
            return self.unreserved(spelling);
        }
        match self {
            Self::Map | Self::Always | Self::Eventually | Self::Send | Self::Receive => {
                Self::Unsupported
            }
            Self::Using
            | Self::Predicate
            | Self::BooleanType
            | Self::Contains
            | Self::Filter
            | Self::Count
            | Self::Sum
            | Self::Rational
            | Self::Mod
            | Self::Temporal
            | Self::Protocol
            | Self::Over
            | Self::Clock
            | Self::Origin
            | Self::Each
            | Self::When
            | Self::Capture
            | Self::Holds
            | Self::Until
            | Self::Release
            | Self::Once
            | Self::Historically
            | Self::Since
            | Self::Triggered
            | Self::Role
            | Self::Relationship
            | Self::Channel
            | Self::From
            | Self::To
            | Self::Carries
            | Self::Ordering
            | Self::Unordered
            | Self::Fifo
            | Self::By
            | Self::Delivery
            | Self::Requires
            | Self::Compensate
            | Self::For
            | Self::As
            | Self::Activate
            | Self::First
            | Self::Within
            | Self::Attempts
            | Self::Of
            | Self::Retry
            | Self::Commit
            | Self::Never
            | Self::Recover
            | Self::Run
            | Self::Sequence
            | Self::Choice
            | Self::Visible
            | Self::Case
            | Self::Parallel
            | Self::Branch
            | Self::Join
            | Self::All
            | Self::Repeat
            | Self::Max
            | Self::While
            | Self::Exhausted
            | Self::Await
            | Self::After
            | Self::Match
            | Self::Timeout
            | Self::Via
            | Self::Attempt
            | Self::Contracts
            | Self::Effect
            | Self::Event
            | Self::Related
            | Self::Check
            | Self::Finish => Self::Identifier(spelling.into()),
            unchanged @ (Self::Language
            | Self::Edition
            | Self::Profile
            | Self::Model
            | Self::Version
            | Self::Digest
            | Self::Invariant
            | Self::On
            | Self::At
            | Self::Current
            | Self::Pre
            | Self::Post
            | Self::Let
            | Self::In
            | Self::If
            | Self::Then
            | Self::Else
            | Self::Implies
            | Self::Or
            | Self::And
            | Self::Not
            | Self::Div
            | Self::Rem
            | Self::SelfValue
            | Self::ResultValue
            | Self::True
            | Self::False
            | Self::Present
            | Self::Value
            | Self::Deref
            | Self::Size
            | Self::Forall
            | Self::Exists
            | Self::Reaches
            | Self::OpenParen
            | Self::CloseParen
            | Self::OpenBrace
            | Self::CloseBrace
            | Self::OpenBracket
            | Self::CloseBracket
            | Self::Semicolon
            | Self::Colon
            | Self::Comma
            | Self::Dot
            | Self::Equal
            | Self::Less
            | Self::Greater
            | Self::Plus
            | Self::Minus
            | Self::Star
            | Self::Slash
            | Self::Qualify
            | Self::NotEqual
            | Self::LessEqual
            | Self::GreaterEqual
            | Self::Identifier(_)
            | Self::Integer(_)
            | Self::Text(_)
            | Self::Fractional
            | Self::BadExponent
            | Self::Unsupported
            | Self::Comment
            | Self::End) => unchanged,
            complete_only_kind_pattern!() | Self::Hex(_) => {
                unreachable!("complete-only tokens are unreserved before historical matching")
            }
        }
    }
}

pub(crate) fn identifier_spelling(spelling: &str) -> bool {
    let mut bytes = spelling.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte == b'_' || byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
}
