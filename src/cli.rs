// SPDX-License-Identifier: AGPL-3.0-only
//! FR-010/026: the binary's bounded positional command grammar.

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
        identity: &'a str,
        revision: &'a str,
        path: &'a Path,
    },
    Run {
        path: &'a Path,
    },
    Compile {
        path: &'a Path,
    },
    Lower {
        path: &'a Path,
    },
}

#[derive(Debug, thiserror::Error)]
pub(super) enum UsageError<'a> {
    #[error("usage: quire-spec <parse|format> <source-id> <source-revision> <file> | quire-spec <run|compile|lower> <request-file>")]
    MissingCommand,
    #[error("{operand} must be UTF-8")]
    NonUtf8 { operand: &'static str },
    #[error("unknown command: {0}")]
    UnknownCommand(&'a str),
    #[error("usage: quire-spec {command} {operands}")]
    Arity {
        command: &'a str,
        operands: &'static str,
    },
}

impl<'a> TryFrom<&'a [OsString]> for Command<'a> {
    type Error = UsageError<'a>;

    fn try_from(arguments: &'a [OsString]) -> Result<Self, Self::Error> {
        let (command, operands) = arguments.split_first().ok_or(UsageError::MissingCommand)?;
        let command = command
            .to_str()
            .ok_or(UsageError::NonUtf8 { operand: "command" })?;
        match command {
            "parse" | "format" => {
                let [identity, revision, path] = operands else {
                    return Err(UsageError::Arity {
                        command,
                        operands: "<source-id> <source-revision> <file>",
                    });
                };
                let identity = identity.to_str().ok_or(UsageError::NonUtf8 {
                    operand: "source identity",
                })?;
                let revision = revision.to_str().ok_or(UsageError::NonUtf8 {
                    operand: "source revision",
                })?;
                Ok(Self::Syntax {
                    kind: if command == "parse" {
                        SyntaxCommand::Parse
                    } else {
                        SyntaxCommand::Format
                    },
                    identity,
                    revision,
                    path: Path::new(path),
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
                let [path] = operands else {
                    return Err(UsageError::Arity {
                        command,
                        operands: "<request-file>",
                    });
                };
                Ok(Self::Lower {
                    path: Path::new(path),
                })
            }
            _ => Err(UsageError::UnknownCommand(command)),
        }
    }
}
