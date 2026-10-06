// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-010/026: the binary's bounded positional command grammar.

use qsl_foundation::SourceIdentity;
use quire_spec_language::lowering::{ProjectionTarget, UnknownProjectionTarget};
use quire_spec_language::Limits;
use std::ffi::OsString;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SyntaxCommand {
    Parse,
    Format,
}

pub(super) enum Command<'a> {
    Syntax {
        kind: SyntaxCommand,
        /// FR-001's four labels, exactly as given.
        source: SourceIdentity,
        path: &'a Path,
        /// `parse` only; `format` always carries the default.
        limits: Limits,
    },
    Run {
        path: &'a Path,
    },
    Compile {
        path: &'a Path,
    },
    Lower {
        path: &'a Path,
        target: ProjectionTarget,
    },
}

#[derive(Debug, thiserror::Error)]
pub(super) enum UsageError<'a> {
    #[error("usage: quire-spec <parse|format> <source-authority> <source-id> <revision-namespace> <revision> <file> [--source-bytes <n>] [--tokens <n>] [--nodes <n>] (limits for parse only) | quire-spec <run|compile|lower> <request-file>")]
    MissingCommand,
    #[error("{operand} must be UTF-8")]
    NonUtf8 { operand: &'static str },
    #[error("unknown command: {0}")]
    UnknownCommand(&'a str),
    #[error("{0}")]
    UnknownTarget(#[from] UnknownProjectionTarget),
    #[error("usage: quire-spec {command} {operands}")]
    Arity {
        command: &'a str,
        operands: &'static str,
    },
    #[error("{option} takes a non-negative integer count")]
    LimitValue { option: &'a str },
}

impl<'a> TryFrom<&'a [OsString]> for Command<'a> {
    type Error = UsageError<'a>;

    #[qsl_attrs::string_edge]
    fn try_from(arguments: &'a [OsString]) -> Result<Self, Self::Error> {
        let (command, operands) = arguments.split_first().ok_or(UsageError::MissingCommand)?;
        let command = command
            .to_str()
            .ok_or(UsageError::NonUtf8 { operand: "command" })?;
        match command {
            "parse" | "format" => {
                let arity = || UsageError::Arity {
                    command,
                    operands: if command == "parse" {
                        "<source-authority> <source-id> <revision-namespace> <revision> <file> [--source-bytes <n>] [--tokens <n>] [--nodes <n>]"
                    } else {
                        "<source-authority> <source-id> <revision-namespace> <revision> <file>"
                    },
                };
                let Some(([authority, identity, namespace, revision, path], options)) =
                    operands.split_first_chunk::<5>()
                else {
                    return Err(arity());
                };
                if command == "format" && !options.is_empty() {
                    return Err(arity());
                }
                let limits = limit_options(options).ok_or_else(arity)??;
                let label = |value: &'a OsString, operand| {
                    value.to_str().ok_or(UsageError::NonUtf8 { operand })
                };
                Ok(Self::Syntax {
                    kind: if command == "parse" {
                        SyntaxCommand::Parse
                    } else {
                        SyntaxCommand::Format
                    },
                    source: SourceIdentity::new(
                        label(authority, "source authority")?,
                        label(identity, "source identity")?,
                        label(namespace, "revision namespace")?,
                        label(revision, "source revision")?,
                    ),
                    path: Path::new(path),
                    limits,
                })
            }
            "run" => {
                let [path] = operands else {
                    return Err(UsageError::Arity {
                        command,
                        operands: "<request-file>",
                    });
                };
                Ok(Self::Run {
                    path: Path::new(path),
                })
            }
            "compile" => {
                let [path] = operands else {
                    return Err(UsageError::Arity {
                        command,
                        operands: "<request-file>",
                    });
                };
                Ok(Self::Compile {
                    path: Path::new(path),
                })
            }
            "lower" => {
                let (path, target) = match operands {
                    [path] => (path, ProjectionTarget::BooleanOracleV1),
                    [path, option, target] if option == "--target" => {
                        let name = target.to_str().ok_or(UsageError::NonUtf8 {
                            operand: "lowering target",
                        })?;
                        (path, name.parse()?)
                    }
                    _ => {
                        return Err(UsageError::Arity {
                            command,
                            operands: "<request-file> [--target <target>]",
                        })
                    }
                };
                Ok(Self::Lower {
                    path: Path::new(path),
                    target,
                })
            }
            _ => Err(UsageError::UnknownCommand(command)),
        }
    }
}

/// `--source-bytes`, `--tokens` and `--nodes`, each followed by its count;
/// an option left out keeps its default. `None` is a malformed option list.
fn limit_options(options: &[OsString]) -> Option<Result<Limits, UsageError<'_>>> {
    let mut limits = Limits::default();
    for pair in options.chunks(2) {
        let [option, value] = pair else {
            return None;
        };
        let option = option.to_str()?;
        let slot = match option {
            "--source-bytes" => &mut limits.source_bytes,
            "--tokens" => &mut limits.tokens,
            "--nodes" => &mut limits.nodes,
            _ => return None,
        };
        match value.to_str().and_then(|text| text.parse().ok()) {
            Some(count) => *slot = count,
            None => return Some(Err(UsageError::LimitValue { option })),
        }
    }
    Some(Ok(limits))
}
