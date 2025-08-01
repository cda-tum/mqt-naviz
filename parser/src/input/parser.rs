//! Parser for the `.naviz` format.
//! Takes tokens lexed by the [lexer][super::lexer].

use super::lexer::{TimeSpec, Token};
use crate::common::{self, parser::try_into_value::TryIntoValue};
use fraction::{Fraction, Zero};
use std::fmt::Debug;
use token::{
    comment, group_close, group_open, identifier, ignore_comments, number, separator, time_symbol,
};
use winnow::{
    combinator::{alt, opt, preceded, repeat, terminated},
    PResult, Parser,
};

// Re-export the common parser
pub use common::parser::*;

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub enum InstructionOrDirective {
    /// A single instruction
    Instruction {
        time: Option<(TimeSpec, Fraction)>,
        name: String,
        args: Vec<Value>,
    },
    /// A single time with multiple instructions
    GroupedTime {
        time: Option<(TimeSpec, Fraction)>,
        /// The durations are allowed to vary
        variable: bool,
        /// The grouped part: instruction and arguments
        group: Vec<(String, Vec<Value>)>,
    },
    /// A single time and instructions with multiple argument-instances
    GroupedInstruction {
        time: Option<(TimeSpec, Fraction)>,
        /// The durations are allowed to vary
        variable: bool,
        name: String,
        /// The grouped part: argument-instances
        group: Vec<Vec<Value>>,
    },
    /// A single directive
    Directive { name: String, args: Vec<Value> },
}

/// Parse a full stream of [Token]s into a [Vec] of [InstructionOrDirective]s.
pub fn parse<S: TryIntoValue + Clone + Debug + PartialEq>(
    input: &[Token<S>],
) -> Result<
    Vec<InstructionOrDirective>,
    winnow::error::ParseError<&[Token<S>], winnow::error::ContextError>,
> {
    instruction_or_directives.parse(input)
}

/// Parse all [Instruction][InstructionOrDirective::Instruction]s,
/// [GroupedTime][InstructionOrDirective::GroupedTime]s,,
/// [GroupedInstruction][InstructionOrDirective::GroupedInstruction]s,,
/// and [Directive][InstructionOrDirective::Directive]s
pub fn instruction_or_directives<S: TryIntoValue + Clone + Debug + PartialEq>(
    input: &mut &[Token<S>],
) -> PResult<Vec<InstructionOrDirective>> {
    preceded(
        ignore_comments_and_separators,
        repeat(
            0..,
            terminated(
                alt((instruction, directive, grouped_time, grouped_instruction)),
                ignore_comments_and_separators,
            ),
        ),
    )
    .parse_next(input)
}

/// Try to parse an [Instruction][InstructionOrDirective::Instruction] from a stream of [Token]s.
pub fn instruction<S: TryIntoValue + Clone + Debug + PartialEq>(
    input: &mut &[Token<S>],
) -> PResult<InstructionOrDirective> {
    (
        terminated(opt(time), ignore_comments),
        terminated(identifier, ignore_comments),
        repeat(0.., terminated(any_value, ignore_comments)),
        separator,
    )
        .map(|(time, name, args, _)| InstructionOrDirective::Instruction { time, name, args })
        .parse_next(input)
}

/// Try to parse an [Directive][InstructionOrDirective::Directive] from a stream of [Token]s.
pub fn directive<S: TryIntoValue + Clone + Debug + PartialEq>(
    input: &mut &[Token<S>],
) -> PResult<InstructionOrDirective> {
    (
        terminated(token::directive, ignore_comments),
        repeat(0.., terminated(any_value, ignore_comments)),
        separator,
    )
        .map(|(name, args, _)| InstructionOrDirective::Directive { name, args })
        .parse_next(input)
}

/// Try to parse a time ([TimeSpec] and accompanying number) from a stream of [Token]s.
pub fn time<S: TryIntoValue + Clone + Debug>(
    input: &mut &[Token<S>],
) -> PResult<(TimeSpec, Fraction)> {
    (
        time_symbol,
        opt(number).map(|n| n.unwrap_or_else(Fraction::zero)),
    )
        .parse_next(input)
}

/// Try to parse a [GroupedTime][InstructionOrDirective::GroupedTime] from a stream of [Token]s.
pub fn grouped_time<S: TryIntoValue + Clone + Debug + PartialEq>(
    input: &mut &[Token<S>],
) -> PResult<InstructionOrDirective> {
    let grouped_instruction = terminated(
        (
            terminated(identifier, ignore_comments),
            repeat(0.., terminated(any_value, ignore_comments)),
        ),
        separator,
    );

    let grouped_instructions = preceded(
        ignore_comments_and_separators,
        repeat(
            1..,
            terminated(grouped_instruction, ignore_comments_and_separators),
        ),
    );

    (
        opt(time),
        terminated((group_open, grouped_instructions), group_close),
    )
        .map(
            |(time, (variable, group))| InstructionOrDirective::GroupedTime {
                time,
                variable,
                group,
            },
        )
        .parse_next(input)
}

/// Try to parse a [GroupedInstruction][InstructionOrDirective::GroupedInstruction] from a stream of [Token]s.
pub fn grouped_instruction<S: TryIntoValue + Clone + Debug + PartialEq>(
    input: &mut &[Token<S>],
) -> PResult<InstructionOrDirective> {
    let grouped_value = terminated(
        repeat(0.., terminated(any_value, ignore_comments)),
        separator,
    );

    let grouped_values = preceded(
        ignore_comments_and_separators,
        repeat(
            1..,
            terminated::<_, Vec<_>, _, _, _, _>(grouped_value, ignore_comments_and_separators),
        ),
    );

    (
        opt(time),
        terminated(identifier, ignore_comments),
        terminated((group_open, grouped_values), group_close),
    )
        .map(
            |(time, name, (variable, group))| InstructionOrDirective::GroupedInstruction {
                time,
                variable,
                name,
                group,
            },
        )
        .parse_next(input)
}

/// Ignores all [Comment][Token::Comment]s and [Separator][Token::Separator]s
pub fn ignore_comments_and_separators<S: Clone + Debug + PartialEq + TryIntoValue>(
    input: &mut &[Token<S>],
) -> PResult<()> {
    repeat(0.., alt((comment.void(), separator))).parse_next(input)
}

pub mod token {
    use super::*;
    use crate::input::lexer::{self, TimeSpec};
    use winnow::{token::one_of, Parser};

    // Re-export the common token-parsers
    pub use common::parser::token::*;

    /// Try to parse a single [Token::TimeSymbol].
    pub fn time_symbol<S: Clone + Debug>(input: &mut &[Token<S>]) -> PResult<TimeSpec> {
        one_of(|t| matches!(t, Token::TimeSymbol(_)))
            .map(|t| match t {
                Token::TimeSymbol(t) => t,
                _ => unreachable!(),
            })
            .parse_next(input)
    }

    /// Try to parse a single [Token::GroupOpen].
    pub fn group_open<S: Clone + Debug + PartialEq>(input: &mut &[Token<S>]) -> PResult<bool> {
        one_of(|t| matches!(t, Token::GroupOpen { .. }))
            .map(|t| match t {
                Token::GroupOpen { variable } => variable,
                _ => unreachable!(),
            })
            .parse_next(input)
    }

    /// Try to parse a single [Token::GroupClose].
    pub fn group_close<S: Clone + Debug + PartialEq>(input: &mut &[Token<S>]) -> PResult<()> {
        one_of([Token::GroupClose]).void().parse_next(input)
    }

    /// Try to parse a single [Token::Separator].
    pub fn separator<S: Clone + Debug + PartialEq>(input: &mut &[Token<S>]) -> PResult<()> {
        one_of([Token::Separator]).void().parse_next(input)
    }

    /// Try to parse a single [Token::Value] where the value is a [Value::Number].
    pub fn number<S: TryIntoValue + Clone + Debug>(input: &mut &[Token<S>]) -> PResult<Fraction> {
        one_of(|t| matches!(t, Token::Value(lexer::Value::Number(_))))
            .map(|t| match t {
                Token::Value(lexer::Value::Number(n)) => n,
                _ => unreachable!(),
            })
            .try_map(TryIntoValue::number)
            .parse_next(input)
    }

    /// Try to parse a single [Token::Directive].
    pub fn directive<S: TryIntoValue + Clone + Debug>(input: &mut &[Token<S>]) -> PResult<String> {
        one_of(|t| matches!(t, Token::Directive(_)))
            .map(|t| match t {
                Token::Directive(d) => d,
                _ => unreachable!(),
            })
            .try_map(TryIntoValue::identifier)
            .parse_next(input)
    }
}

// Implement `ContainsToken` for `Token` and `Token`-slices.

impl<T: PartialEq> winnow::stream::ContainsToken<Token<T>> for Token<T> {
    #[inline(always)]
    fn contains_token(&self, token: Token<T>) -> bool {
        *self == token
    }
}

impl<T: PartialEq> winnow::stream::ContainsToken<Token<T>> for &'_ [Token<T>] {
    #[inline]
    fn contains_token(&self, token: Token<T>) -> bool {
        self.contains(&token)
    }
}

impl<T: PartialEq, const LEN: usize> winnow::stream::ContainsToken<Token<T>>
    for &'_ [Token<T>; LEN]
{
    #[inline]
    fn contains_token(&self, token: Token<T>) -> bool {
        self.contains(&token)
    }
}

impl<T: PartialEq, const LEN: usize> winnow::stream::ContainsToken<Token<T>> for [Token<T>; LEN] {
    #[inline]
    fn contains_token(&self, token: Token<T>) -> bool {
        self.contains(&token)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::common::lexer;
    use fraction::{ConstZero, Fraction};
    use regex::Regex;

    #[test]
    pub fn simple_example() {
        let input = vec![
            Token::Directive("directive"),
            Token::Identifier("value"),
            Token::Separator,
            Token::Directive("other_directive"),
            Token::Value(lexer::Value::String("string")),
            Token::Separator,
            Token::Identifier("instruction"),
            Token::Identifier("argument"),
            Token::Value(lexer::Value::String("argument")),
            Token::Value(lexer::Value::Regex("argument")),
            Token::Separator,
            Token::TimeSymbol(TimeSpec::Absolute),
            Token::Value(lexer::Value::Number("0")),
            Token::Identifier("timed_instruction"),
            Token::Identifier("arg"),
            Token::Separator,
            Token::TimeSymbol(TimeSpec::Relative {
                from_start: false,
                positive: false,
            }),
            Token::Value(lexer::Value::Number("1")),
            Token::Identifier("negative_timed_instruction"),
            Token::Identifier("arg"),
            Token::Separator,
            Token::TimeSymbol(TimeSpec::Relative {
                from_start: true,
                positive: true,
            }),
            Token::Value(lexer::Value::Number("2")),
            Token::Identifier("positive_start_timed_instruction"),
            Token::Identifier("arg"),
            Token::Separator,
            Token::TimeSymbol(TimeSpec::Relative {
                from_start: false,
                positive: true,
            }),
            Token::GroupOpen { variable: false },
            Token::Separator,
            Token::Identifier("group_instruction_a"),
            Token::Value(lexer::Value::Number("1")),
            Token::Separator,
            Token::Identifier("group_instruction_b"),
            Token::Value(lexer::Value::Number("2")),
            Token::Separator,
            Token::GroupClose,
            Token::Separator,
            Token::Identifier("group_instruction"),
            Token::GroupOpen { variable: true },
            Token::Separator,
            Token::Value(lexer::Value::Number("1")),
            Token::Separator,
            Token::Value(lexer::Value::Number("2")),
            Token::Separator,
            Token::GroupClose,
            Token::Separator,
        ];

        let expected = vec![
            InstructionOrDirective::Directive {
                name: "directive".to_string(),
                args: vec![Value::Identifier("value".to_string())],
            },
            InstructionOrDirective::Directive {
                name: "other_directive".to_string(),
                args: vec![Value::String("string".to_string())],
            },
            InstructionOrDirective::Instruction {
                time: None,
                name: "instruction".to_string(),
                args: vec![
                    Value::Identifier("argument".to_string()),
                    Value::String("argument".to_string()),
                    Value::Regex(Regex::new("argument").unwrap()),
                ],
            },
            InstructionOrDirective::Instruction {
                time: Some((TimeSpec::Absolute, Fraction::new(0u64, 1u64))),
                name: "timed_instruction".to_string(),
                args: vec![Value::Identifier("arg".to_string())],
            },
            InstructionOrDirective::Instruction {
                time: Some((
                    TimeSpec::Relative {
                        from_start: false,
                        positive: false,
                    },
                    Fraction::new(1u64, 1u64),
                )),
                name: "negative_timed_instruction".to_string(),
                args: vec![Value::Identifier("arg".to_string())],
            },
            InstructionOrDirective::Instruction {
                time: Some((
                    TimeSpec::Relative {
                        from_start: true,
                        positive: true,
                    },
                    Fraction::new(2u64, 1u64),
                )),
                name: "positive_start_timed_instruction".to_string(),
                args: vec![Value::Identifier("arg".to_string())],
            },
            InstructionOrDirective::GroupedTime {
                time: Some((
                    TimeSpec::Relative {
                        from_start: false,
                        positive: true,
                    },
                    Fraction::new(0u64, 1u64),
                )),
                variable: false,
                group: vec![
                    (
                        "group_instruction_a".to_string(),
                        vec![Value::Number(Fraction::new(1u64, 1u64))],
                    ),
                    (
                        "group_instruction_b".to_string(),
                        vec![Value::Number(Fraction::new(2u64, 1u64))],
                    ),
                ],
            },
            InstructionOrDirective::GroupedInstruction {
                time: None,
                variable: true,
                name: "group_instruction".to_string(),
                group: vec![
                    vec![Value::Number(Fraction::new(1u64, 1u64))],
                    vec![Value::Number(Fraction::new(2u64, 1u64))],
                ],
            },
        ];

        let actual = parse(&input).expect("Failed to parse");

        assert_eq!(actual, expected);
    }

    #[test]
    pub fn comments_and_whitespace() {
        // Example file:
        //
        // // Comment 1
        // // Comment 2

        // // Comment 3

        // #directive value // Comment 4

        // /* Comment 5 */ /* Comment 6 */
        // // Comment 6

        // #directive2 value2
        // // Comment 7

        let input = vec![
            Token::Comment(" Comment 1"),
            Token::Comment(" Comment 2"),
            Token::Separator,
            Token::Comment(" Comment 3"),
            Token::Separator,
            Token::Directive("directive"),
            Token::Identifier("value"),
            Token::Comment(" Comment 4"),
            Token::Separator,
            Token::Comment(" Comment 5 "),
            Token::Comment(" Comment 6 "),
            Token::Separator,
            Token::Comment(" Comment 6"),
            Token::Separator,
            Token::Directive("directive2"),
            Token::Identifier("value2"),
            Token::Separator,
            Token::Comment(" Comment 7"),
            Token::Separator,
        ];

        let expected = vec![
            InstructionOrDirective::Directive {
                name: "directive".to_string(),
                args: vec![Value::Identifier("value".to_string())],
            },
            InstructionOrDirective::Directive {
                name: "directive2".to_string(),
                args: vec![Value::Identifier("value2".to_string())],
            },
        ];

        let actual = parse(&input).expect("Failed to parse");

        assert_eq!(actual, expected);
    }

    #[test]
    fn valid_set_identifiers() {
        let input = vec![
            Token::TimeSymbol(TimeSpec::Absolute),
            Token::Value(lexer::Value::Number("0")),
            Token::Identifier("timed_instruction"),
            Token::SetOpen,
            Token::Identifier("t1"),
            Token::ElementSeparator,
            Token::Identifier("t2"),
            Token::ElementSeparator,
            Token::SetOpen,
            Token::Identifier("t3"),
            Token::SetClose,
            Token::ElementSeparator,
            Token::SetOpen,
            Token::SetOpen,
            Token::Identifier("t4"),
            Token::SetClose,
            Token::SetClose,
            Token::SetClose,
            Token::Separator,
        ];

        let expected = vec![InstructionOrDirective::Instruction {
            time: Some((TimeSpec::Absolute, Fraction::ZERO)),
            name: "timed_instruction".to_string(),
            args: vec![Value::Set(vec![
                Value::Identifier("t1".to_string()),
                Value::Identifier("t2".to_string()),
                Value::Set(vec![Value::Identifier("t3".to_string())]),
                Value::Set(vec![Value::Set(vec![Value::Identifier("t4".to_string())])]),
            ])],
        }];

        let actual = parse(&input).expect("Failed to parse");

        assert_eq!(actual, expected);
    }

    #[test]
    fn invalid_set_identifier_missing_close() {
        let input = vec![
            Token::TimeSymbol(TimeSpec::Absolute),
            Token::Value(lexer::Value::Number("0")),
            Token::Identifier("timed_instruction"),
            Token::SetOpen,
            Token::Identifier("t1"),
            Token::Separator,
        ];

        parse(&input).expect_err("Invalid input was parsed without error");
    }

    #[test]
    fn invalid_set_identifier_missing_open() {
        let input = vec![
            Token::TimeSymbol(TimeSpec::Absolute),
            Token::Value(lexer::Value::Number("0")),
            Token::Identifier("timed_instruction"),
            Token::Identifier("t1"),
            Token::SetClose,
            Token::Separator,
        ];

        parse(&input).expect_err("Invalid input was parsed without error");
    }

    #[test]
    fn invalid_set_identifier_missing_separator() {
        let input = vec![
            Token::TimeSymbol(TimeSpec::Absolute),
            Token::Value(lexer::Value::Number("0")),
            Token::Identifier("timed_instruction"),
            Token::SetOpen,
            Token::Identifier("t1"),
            Token::Identifier("t2"),
            Token::SetClose,
            Token::Separator,
        ];

        parse(&input).expect_err("Invalid input was parsed without error");
    }
}
