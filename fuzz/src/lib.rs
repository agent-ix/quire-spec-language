// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deeply nested QSL sources for the `deep_input` fuzz target (FR-356-AC-7,
//! TC-903).
//!
//! [`DeepInput::from_bytes`] turns fuzzer bytes into one complete-V1 unit
//! nested 1 to 100,000 levels deep: nested brackets, a sum, an `else if`
//! chain, nested `let`s, a mix of those four chosen level by level, or a
//! nested `Option` type. [`DeepInput::compile`] runs it through the S1
//! parser, S2 forms and the S3 checker (`qsl_replay::spine::compile`)
//! under limits raised to fit its size, so a refusal is the stage's own
//! outcome for the input, not a default size limit.

use std::collections::BTreeMap;

use qsl_replay::spine::{compile, CompileRefusal, Compiled, DependencyInput, SpineLimits};
use qsl_replay::SourceIdentity;
use quire_semantic_value::checking::{CheckingLimits, MAX_CHECKING_DEPTH};

/// The deepest input the generator builds.
pub const MAX_DEPTH: usize = 100_000;

/// The fixture whose header (its `language` and `profile` lines) every
/// generated unit uses, so the profile resolves against the same
/// `DefinitionLock` catalog row the spine tests compile against.
const HEADER_FIXTURE: &str = include_str!("../../tests/fixtures/spine-run.native");

/// One way a level nests inside the level above it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// `( ... )`.
    Bracket,
    /// `x + ...`.
    Sum,
    /// `if true then x else ...`.
    ElseIf,
    /// `let v = x in ...`.
    Let,
}

impl Level {
    const ALL: [Self; 4] = [Self::Bracket, Self::Sum, Self::ElseIf, Self::Let];

    fn from_byte(byte: u8) -> Self {
        Self::ALL[usize::from(byte) % Self::ALL.len()]
    }

    fn prefix(self) -> &'static str {
        match self {
            Self::Bracket => "(",
            Self::Sum => "x + ",
            Self::ElseIf => "if true then x else ",
            Self::Let => "let v = x in ",
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            Self::Bracket => ")",
            Self::Sum | Self::ElseIf | Self::Let => "",
        }
    }
}

/// The shape of one generated unit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    /// A function body nested by these levels, outermost first.
    Expression(Vec<Level>),
    /// A type alias `Option<...<Integer>...>` this many levels deep.
    OptionType(usize),
}

/// One generated unit and its source text.
#[derive(Clone, Debug)]
pub struct DeepInput {
    /// How the unit nests.
    pub shape: Shape,
    /// The complete-V1 source.
    pub source: String,
}

impl DeepInput {
    /// Build a unit from fuzzer bytes. Byte 0 picks the shape, bytes 1 to 4
    /// the depth (1 to [`MAX_DEPTH`]), and for the mixed shape each further
    /// byte picks a level's kind, cycling.
    pub fn from_bytes(data: &[u8]) -> Self {
        let selector = data.first().copied().unwrap_or(0);
        let mut depth_bytes = [0_u8; 4];
        for (slot, byte) in depth_bytes.iter_mut().zip(data.iter().skip(1)) {
            *slot = *byte;
        }
        let depth = usize::try_from(u32::from_le_bytes(depth_bytes))
            .map_or(MAX_DEPTH, |raw| raw % MAX_DEPTH + 1);
        let kinds = data.get(5..).unwrap_or(&[]);
        let shape = match selector % 6 {
            0..=3 => Shape::Expression(vec![Level::ALL[usize::from(selector % 6)]; depth]),
            4 if !kinds.is_empty() => Shape::Expression(
                (0..depth)
                    .map(|level| Level::from_byte(kinds[level % kinds.len()]))
                    .collect(),
            ),
            4 => Shape::Expression(vec![Level::Bracket; depth]),
            _ => Shape::OptionType(depth),
        };
        Self::new(shape)
    }

    /// The unit for `shape`.
    pub fn new(shape: Shape) -> Self {
        let mut source = String::new();
        for line in HEADER_FIXTURE.lines() {
            if line.starts_with("language ") || line.starts_with("profile ") {
                source.push_str(line);
                source.push('\n');
            }
        }
        match &shape {
            Shape::Expression(levels) => {
                source.push_str("function f using v(x: Integer): Integer pure { ");
                for level in levels {
                    source.push_str(level.prefix());
                }
                source.push('x');
                for level in levels.iter().rev() {
                    source.push_str(level.suffix());
                }
                source.push_str(" }\n");
            }
            Shape::OptionType(depth) => {
                source.push_str("type T = ");
                source.push_str(&"Option<".repeat(*depth));
                source.push_str("Integer");
                source.push_str(&">".repeat(*depth));
                source.push_str(";\nfunction f using v(x: T): T pure { x }\n");
            }
        }
        Self { shape, source }
    }

    /// The input's nesting depth.
    pub fn depth(&self) -> usize {
        match &self.shape {
            Shape::Expression(levels) => levels.len(),
            Shape::OptionType(depth) => *depth,
        }
    }

    /// Stage limits raised to fit this input: every size and work limit
    /// covers the source, so what refuses it is the stage's own outcome.
    pub fn limits(&self) -> SpineLimits {
        let bytes = self.source.len();
        let mut limits = SpineLimits::default();
        limits.source.source_bytes = limits.source.source_bytes.max(bytes);
        limits.source.tokens = limits.source.tokens.max(bytes);
        limits.source.nodes = limits.source.nodes.max(bytes);
        limits.source.nesting = limits.source.nesting.max(self.depth() + 8);
        let wide = u64::try_from(bytes).unwrap_or(u64::MAX).saturating_mul(64);
        if let Ok(checking) = CheckingLimits::new(wide, MAX_CHECKING_DEPTH) {
            limits.checking = checking.with_input_bytes(wide).with_work_budget(wide);
        }
        limits
    }

    /// Compile the input through S1 to S4.
    pub fn compile(&self) -> Result<Compiled, Box<CompileRefusal>> {
        compile(
            SourceIdentity::new("agent-ix", "qsl-fuzz", "fuzz", "1"),
            "deep-input.native",
            self.source.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            self.limits(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A shallow unit of every shape compiles or is refused with an
    /// ordinary outcome, and a shallow sum compiles, which shows the
    /// header resolves and the checker is reached.
    #[test]
    fn shallow_units_reach_the_checker() {
        let sum = DeepInput::new(Shape::Expression(vec![Level::Sum; 3]));
        sum.compile().expect("a three-term sum compiles");
        for level in Level::ALL {
            let _ = DeepInput::new(Shape::Expression(vec![level; 3])).compile();
        }
        let _ = DeepInput::new(Shape::OptionType(3)).compile();
    }

    /// The selector and depth bytes decode to the documented shape and
    /// depth range.
    #[test]
    fn bytes_decode_to_shape_and_depth() {
        let input = DeepInput::from_bytes(&[1, 9, 0, 0, 0]);
        assert_eq!(input.shape, Shape::Expression(vec![Level::Sum; 10]));
        let input = DeepInput::from_bytes(&[5, 0xff, 0xff, 0xff, 0xff]);
        assert_eq!(input.depth(), (u32::MAX as usize) % MAX_DEPTH + 1);
        assert!(matches!(input.shape, Shape::OptionType(_)));
        assert_eq!(DeepInput::from_bytes(&[]).depth(), 1);
    }
}
