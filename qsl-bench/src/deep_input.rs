// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deeply nested QSL sources for the `deep_input` fuzz target in `fuzz/`
//! (FR-356-AC-7, TC-903). They live here, in a workspace crate, so
//! `make ci` builds and tests them against the current stage APIs; the fuzz
//! crate is a thin libFuzzer entry point over [`DeepInput`].
//!
//! [`DeepInput::from_bytes`] turns fuzzer bytes into one complete-V1 unit
//! nested 1 to 100,000 levels deep: nested brackets, a sum, an `else if`
//! chain, nested `let`s, a mix of those four chosen level by level, or a
//! nested `Option` type. [`DeepInput::compile`] runs it through the S1
//! parser, S2 forms and the S3 checker (`qsl_replay::spine::compile`)
//! under size and work limits raised to fit it, so a refusal is the stage's
//! own outcome for the input, not a default size limit.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use qsl_replay::spine::{compile, CompileRefusal, Compiled, DependencyInput, SpineLimits};
use qsl_replay::SourceIdentity;
use quire_semantic_value::checking::CheckingLimits;

/// The deepest input the generator builds.
pub const MAX_DEPTH: usize = 100_000;

/// The fixture whose header (its `language` and `profile` lines) every
/// generated unit uses, so the profile resolves against the same
/// `DefinitionLock` catalog row the spine tests compile against.
const HEADER_FIXTURE: &str = include_str!("../../tests/fixtures/spine-run.native");

/// The `language` and `profile` lines of [`HEADER_FIXTURE`].
///
/// `#[string_edge]`: picks the fixture's header lines by their leading
/// keyword; it selects no family semantics.
#[qsl_attrs::string_edge]
fn header() -> String {
    let mut header = String::new();
    for line in HEADER_FIXTURE.lines() {
        if line.starts_with("language ") || line.starts_with("profile ") {
            header.push_str(line);
            header.push('\n');
        }
    }
    header
}

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

    /// The text that opens level `index`. Each `let` binds its own name,
    /// so the chain nests rather than redeclaring one name.
    fn open(self, index: usize, source: &mut String) {
        match self {
            Self::Bracket => source.push('('),
            Self::Sum => source.push_str("x + "),
            Self::ElseIf => source.push_str("if true then x else "),
            Self::Let => {
                let _ = write!(source, "let v{index} = x in ");
            }
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
    /// Build a unit from fuzzer bytes. Byte 0 picks the shape and the
    /// depth's order of magnitude, bytes 1 to 4 the depth within it (1 to
    /// [`MAX_DEPTH`] in all), and for the mixed shape each further byte
    /// picks a level's kind, cycling.
    ///
    /// Depths are spread over orders of magnitude rather than uniformly, so
    /// a run of 10,000 inputs covers every scale from 1 to 100,000 levels
    /// in hours of compile time, not days: of the 16 magnitude classes,
    /// ten reach at most 100 levels, three at most 1,000, two at most
    /// 10,000 and one [`MAX_DEPTH`].
    pub fn from_bytes(data: &[u8]) -> Self {
        let selector = data.first().copied().unwrap_or(0);
        let mut depth_bytes = [0_u8; 4];
        for (slot, byte) in depth_bytes.iter_mut().zip(data.iter().skip(1)) {
            *slot = *byte;
        }
        let ceiling = match selector / 16 {
            0..=9 => 100,
            10..=12 => 1_000,
            13 | 14 => 10_000,
            _ => MAX_DEPTH,
        };
        let depth = usize::try_from(u32::from_le_bytes(depth_bytes))
            .map_or(ceiling, |raw| raw % ceiling + 1);
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
        let mut source = header();
        match &shape {
            Shape::Expression(levels) => {
                source.push_str("function f using v(x: Integer): Integer pure { ");
                for (index, level) in levels.iter().enumerate() {
                    level.open(index, &mut source);
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
    ///
    /// No stage takes a depth limit: S1, S2 and the checker each walk an
    /// input of any depth on an explicit stack (FR-258).
    pub fn limits(&self) -> SpineLimits {
        let bytes = self.source.len();
        let mut limits = SpineLimits::default();
        limits.source.source_bytes = limits.source.source_bytes.max(bytes);
        limits.source.tokens = limits.source.tokens.max(bytes);
        limits.source.nodes = limits.source.nodes.max(bytes.saturating_mul(16));
        let wide = u64::try_from(bytes).unwrap_or(u64::MAX).saturating_mul(64);
        limits.checking = CheckingLimits::new(wide)
            .with_input_bytes(wide)
            .with_work_budget(wide);
        limits
    }

    /// Compile the input through S1 to S4.
    pub fn compile(&self) -> Result<Compiled, Box<CompileRefusal>> {
        compile(
            SourceIdentity::new("agent-ix", "qsl-bench", "deep-input", "1"),
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

    /// A shallow unit of every shape compiles, which shows the header
    /// resolves and every shape reaches the checker and passes it.
    #[test]
    fn shallow_units_of_every_shape_compile() {
        for level in Level::ALL {
            let input = DeepInput::new(Shape::Expression(vec![level; 3]));
            if let Err(refusal) = input.compile() {
                panic!("{level:?}: {refusal}\n{}", input.source);
            }
        }
        let input = DeepInput::new(Shape::OptionType(3));
        if let Err(refusal) = input.compile() {
            panic!("Option: {refusal}\n{}", input.source);
        }
    }

    /// The deepest `Option` nest S1 admits at its default limits, found by
    /// bisection.
    fn deepest_admitted_options() -> usize {
        let limits = SpineLimits::default().source;
        let (mut low, mut high) = (0_usize, 200_000_usize);
        while low + 1 < high {
            let middle = (low + high) / 2;
            let input = DeepInput::new(Shape::OptionType(middle));
            let parsed = qsl_cst::parse(
                SourceIdentity::new("agent-ix", "qsl-bench", "deep-input", "1"),
                "deep-input.native",
                input.source.as_bytes(),
                limits,
            );
            if parsed.is_ok_and(|parsed| parsed.is_admissible()) {
                low = middle;
            } else {
                high = middle;
            }
        }
        low
    }

    /// The deepest `Option` type S1 admits at default limits compiles
    /// through every stage at `SpineLimits::default()` on a 2 MiB thread,
    /// ending in a unit or a refusal, never a stack overflow.
    #[test]
    fn the_deepest_admitted_option_type_compiles_or_refuses_on_a_2_mib_stack() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                let depth = deepest_admitted_options();
                assert!(depth > 64, "S1 has no nesting ceiling: {depth}");
                let input = DeepInput::new(Shape::OptionType(depth));
                // Either outcome is a result; the test is that one arrives.
                let _outcome: Result<Compiled, Box<CompileRefusal>> = compile(
                    SourceIdentity::new("agent-ix", "qsl-bench", "deep-input", "1"),
                    "deep-input.native",
                    input.source.as_bytes(),
                    &BTreeMap::new(),
                    &DependencyInput::default(),
                    SpineLimits::default(),
                );
            })
            .expect("spawn a 2 MiB thread")
            .join()
            .expect("the compile does not overflow a 2 MiB stack");
    }

    /// The selector and depth bytes decode to the documented shape and
    /// depth range.
    #[test]
    fn bytes_decode_to_shape_and_depth() {
        let input = DeepInput::from_bytes(&[1, 9, 0, 0, 0]);
        assert_eq!(input.shape, Shape::Expression(vec![Level::Sum; 10]));
        // Selector 251: magnitude class 15 (up to MAX_DEPTH), shape 5.
        let input = DeepInput::from_bytes(&[251, 0x9f, 0x86, 0x01, 0]);
        assert_eq!(input.depth(), MAX_DEPTH);
        assert!(matches!(input.shape, Shape::OptionType(_)));
        assert_eq!(DeepInput::from_bytes(&[]).depth(), 1);
        let mixed = DeepInput::from_bytes(&[4, 3, 0, 0, 0, 1, 3]);
        assert_eq!(
            mixed.shape,
            Shape::Expression(vec![Level::Sum, Level::Let, Level::Sum, Level::Let])
        );
    }
}
