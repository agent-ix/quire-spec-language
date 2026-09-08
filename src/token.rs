// SPDX-License-Identifier: AGPL-3.0-only
//! One declarative vocabulary; Logos generates the token recognizer.
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

macro_rules! vocabulary {
    ($($variant:ident => $text:literal),+ $(,)?) => {
        #[derive(Logos, Clone, Debug, PartialEq)]
        #[logos(error = LexError)]
        #[logos(skip r"[ \t\n]+|\r\n")]
        pub(crate) enum Kind {
            $(#[token($text)] $variant,)+
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
            #[regex(r"helper|rec|set|bag|Set|Bag|OrderedSet|collect|flatten|cast|tuple|Tuple|map|always|eventually|send|receive|Decimal|Rational|allInstances", priority = 3)]
            Unsupported,
            #[regex(r"//[^\r\n]*", allow_greedy = true)]
            Comment,
            End,
        }
        impl Kind {
            pub(crate) fn description(&self) -> &'static str {
                match self {
                    $(Self::$variant => $text,)+
                    Self::Identifier(_) => "identifier", Self::Integer(_) => "integer",
                    Self::Text(_) => "quoted string", Self::Fractional => "fractional number",
                    Self::BadExponent => "exponent digits", Self::Unsupported => "unsupported construct",
                    Self::End => "end of source",
                    Self::Comment => "comment",
                }
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
    OpenParen => "(", CloseParen => ")", OpenBrace => "{", CloseBrace => "}",
    OpenBracket => "[", CloseBracket => "]", Semicolon => ";", Colon => ":", Comma => ",",
    Dot => ".", Equal => "=", Less => "<", Greater => ">", Plus => "+", Minus => "-",
    Star => "*", Slash => "/", Qualify => "::", NotEqual => "!=", LessEqual => "<=", GreaterEqual => ">="
}
