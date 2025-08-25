//! Lexer for the `.naviz` format.
//! Use [lex] to lex into a stream of [Token]s.

use crate::common;
use token::token;
use winnow::{
    ascii::{multispace0, space0},
    combinator::{preceded, repeat, terminated},
    stream::{AsChar, Compare, FindSlice, SliceLen, Stream, StreamIsPartial},
    Parser,
};

// Re-export the common lexer
pub use common::lexer::*;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TimeSpec {
    Absolute,
    Relative { from_start: bool, positive: bool },
}

/// A token of the input format
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Token<T> {
    /// An identifier
    Identifier(T),
    /// A value; see [Value]
    Value(Value<T>),
    /// A comment, either single- or multiline
    Comment(T),
    /// Opening-symbol for a tuple
    TupleOpen,
    /// Closing-symbol for a tuple
    TupleClose,
    /// Opening-symbol for a group
    GroupOpen {
        /// The timing of the subcommands can be variable
        variable: bool,
    },
    /// Closing-symbol for a group
    GroupClose,
    // Opening-symbol for a set of values
    SetOpen,
    // Closing-symbol for a set of values
    SetClose,
    /// A separator between elements (e.g., in tuples)
    ElementSeparator,
    /// The symbol to denote the starting-time
    TimeSymbol(TimeSpec),
    /// A directive (`#<directive`)
    Directive(T),
    /// The separator between instructions
    Separator,
}

impl<T> From<GenericToken<T>> for Token<T> {
    fn from(value: GenericToken<T>) -> Self {
        match value {
            GenericToken::Identifier(i) => Self::Identifier(i),
            GenericToken::Value(v) => Self::Value(v),
            GenericToken::Comment(c) => Self::Comment(c),
            GenericToken::TupleOpen => Self::TupleOpen,
            GenericToken::TupleClose => Self::TupleClose,
            GenericToken::SetOpen => Self::SetOpen,
            GenericToken::SetClose => Self::SetClose,
            GenericToken::ElementSeparator => Self::ElementSeparator,
        }
    }
}

impl<T> From<Token<T>> for Option<GenericToken<T>> {
    fn from(value: Token<T>) -> Self {
        match value {
            Token::Identifier(i) => Some(GenericToken::Identifier(i)),
            Token::Value(v) => Some(GenericToken::Value(v)),
            Token::Comment(c) => Some(GenericToken::Comment(c)),
            Token::TupleOpen => Some(GenericToken::TupleOpen),
            Token::TupleClose => Some(GenericToken::TupleClose),
            Token::SetOpen => Some(GenericToken::SetOpen),
            Token::SetClose => Some(GenericToken::SetClose),
            Token::ElementSeparator => Some(GenericToken::ElementSeparator),
            _ => None,
        }
    }
}

/// Lexes a [str] into a [Vec] of [Token]s,
/// or returns an [Err] if lexing failed.
pub fn lex<
    I: Stream
        + StreamIsPartial
        + Compare<&'static str>
        + FindSlice<&'static str>
        + FindSlice<(char, char)>
        + Copy,
>(
    input: I,
) -> Result<
    Vec<Token<<I as Stream>::Slice>>,
    winnow::error::ParseError<I, winnow::error::ContextError>,
>
where
    <I as Stream>::Token: AsChar + Clone,
    I::Slice: SliceLen,
{
    preceded(multispace0, repeat(0.., terminated(token, space0)))
        .parse(input)
        .map(|mut tokens: Vec<_>| {
            // Ensure separator at end of token-stream
            match tokens.last() {
                Some(Token::Separator) => { /* Already exists */ }
                _ => tokens.push(Token::Separator),
            }
            tokens
        })
}

pub mod token {
    use super::*;
    use winnow::{
        ascii::{line_ending, multispace0},
        combinator::{alt, opt, terminated},
        stream::{AsChar, Compare, FindSlice, SliceLen, Stream, StreamIsPartial},
        token::take_till,
        ModalResult, Parser,
    };

    pub use common::lexer::token::*;

    /// Tries to parse a [Token::GroupOpen].
    pub fn group_open<Tok, I: Stream + StreamIsPartial + Compare<&'static str>>(
        input: &mut I,
    ) -> ModalResult<Tok>
    where
        Token<I::Slice>: Into<Tok>,
    {
        (opt("~"), "[")
            .map(|(variable, _)| Token::GroupOpen {
                variable: variable.is_some(),
            })
            .output_into()
            .parse_next(input)
    }

    /// Tries to parse a [Token::GroupClose].
    pub fn group_close<Tok, I: Stream + StreamIsPartial + Compare<&'static str>>(
        input: &mut I,
    ) -> ModalResult<Tok>
    where
        Token<I::Slice>: Into<Tok>,
    {
        "]".map(|_| Token::GroupClose)
            .output_into()
            .parse_next(input)
    }

    /// Tries to parse a single [Token::TimeSymbol].
    pub fn time_symbol<I: Stream + StreamIsPartial + Compare<&'static str>>(
        input: &mut I,
    ) -> ModalResult<Token<<I as Stream>::Slice>> {
        (
            "@",
            opt("=".void()).map(|o| o.is_some()),
            opt(alt(["+".value(true), "-".value(false)])),
        )
            .map(|(_, from_start, sign)| {
                Token::TimeSymbol(match sign {
                    Some(positive) => TimeSpec::Relative {
                        from_start,
                        positive,
                    },
                    None => {
                        if from_start {
                            // Handle `@=<time>`
                            TimeSpec::Relative {
                                from_start: true,
                                positive: true,
                            }
                        } else {
                            TimeSpec::Absolute
                        }
                    }
                })
            })
            .parse_next(input)
    }

    /// Tries to parse a single [Token::Directive].
    pub fn directive<I: Stream + StreamIsPartial + Compare<&'static str>>(
        input: &mut I,
    ) -> ModalResult<Token<<I as Stream>::Slice>>
    where
        I::Token: AsChar + Clone,
    {
        (
            "#",
            take_till(0.., |x: I::Token| x.clone().is_newline() || x.is_space()),
        )
            .map(|(_, dir)| Token::Directive(dir))
            .parse_next(input)
    }

    /// Tries to parse a single [Token::Separator].
    pub fn separator<I: Stream + StreamIsPartial + Compare<&'static str>>(
        input: &mut I,
    ) -> ModalResult<Token<<I as Stream>::Slice>>
    where
        I::Token: AsChar + Clone,
    {
        terminated(line_ending, multispace0)
            .map(|_| Token::Separator)
            .parse_next(input)
    }

    /// Tries to parse any [Token].
    pub fn token<
        I: Stream
            + StreamIsPartial
            + Compare<&'static str>
            + FindSlice<&'static str>
            + FindSlice<(char, char)>
            + Copy,
    >(
        input: &mut I,
    ) -> ModalResult<Token<<I as Stream>::Slice>>
    where
        <I as Stream>::Token: AsChar + Clone,
        I::Slice: SliceLen,
    {
        alt((
            value,
            identifier,
            comment,
            tuple_open,
            tuple_close,
            group_open,
            group_close,
            set_open,
            set_close,
            element_separator,
            time_symbol,
            directive,
            separator,
        ))
        .parse_next(input)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::byte_offset_to_line_column;

    #[test]
    pub fn simple_example() {
        let input = r#"
        #directive value
        #other_directive "string"

        instruction argument "argument" ^argument$
        @0 timed_instruction arg
        @-1 negative_timed_instruction arg
        @=2 positive_start_timed_instruction arg
        @+ [
            group_instruction_a 1
            group_instruction_b 2
        ]
        group_instruction ~[
            1
            2
        ]"#;

        let expected = vec![
            Token::Directive("directive"),
            Token::Identifier("value"),
            Token::Separator,
            Token::Directive("other_directive"),
            Token::Value(Value::String("string")),
            Token::Separator,
            Token::Identifier("instruction"),
            Token::Identifier("argument"),
            Token::Value(Value::String("argument")),
            Token::Value(Value::Regex("^argument$")),
            Token::Separator,
            Token::TimeSymbol(TimeSpec::Absolute),
            Token::Value(Value::Number("0")),
            Token::Identifier("timed_instruction"),
            Token::Identifier("arg"),
            Token::Separator,
            Token::TimeSymbol(TimeSpec::Relative {
                from_start: false,
                positive: false,
            }),
            Token::Value(Value::Number("1")),
            Token::Identifier("negative_timed_instruction"),
            Token::Identifier("arg"),
            Token::Separator,
            Token::TimeSymbol(TimeSpec::Relative {
                from_start: true,
                positive: true,
            }),
            Token::Value(Value::Number("2")),
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
            Token::Value(Value::Number("1")),
            Token::Separator,
            Token::Identifier("group_instruction_b"),
            Token::Value(Value::Number("2")),
            Token::Separator,
            Token::GroupClose,
            Token::Separator,
            Token::Identifier("group_instruction"),
            Token::GroupOpen { variable: true },
            Token::Separator,
            Token::Value(Value::Number("1")),
            Token::Separator,
            Token::Value(Value::Number("2")),
            Token::Separator,
            Token::GroupClose,
            Token::Separator,
        ];

        let actual = lex(input).expect("Failed to lex");

        assert_eq!(actual, expected);
    }

    #[test]
    fn identifier_set() {
        let input = r#"
        @0 timed_instruction { t1, t2, { t3 }, { t4 }}
        "#;

        let expected = vec![
            Token::TimeSymbol(TimeSpec::Absolute),
            Token::Value(Value::Number("0")),
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
            Token::Identifier("t4"),
            Token::SetClose,
            Token::SetClose,
            Token::Separator,
        ];

        let actual = lex(input).expect("Failed to lex");

        assert_eq!(actual, expected);
    }

    #[test]
    fn error_location_byte_offset_conversion() {
        let test_cases = vec![
            // (text, offset, expected_line, expected_column)
            ("hello", 0, 1, 1),        // Start of text
            ("hello", 2, 1, 3),        // Middle of first line
            ("hello", 5, 1, 6),        // End of first line
            ("hello\n", 5, 1, 6),      // Before newline
            ("hello\n", 6, 2, 1),      // After newline (start of line 2)
            ("hello\nworld", 6, 2, 1), // Start of line 2
            ("hello\nworld", 8, 2, 3), // Middle of line 2
            ("a\nb\nc", 0, 1, 1),      // Start
            ("a\nb\nc", 2, 2, 1),      // Start of line 2
            ("a\nb\nc", 4, 3, 1),      // Start of line 3
            ("a\nb\nc", 5, 3, 2),      // End of text
        ];

        for (text, offset, expected_line, expected_column) in test_cases {
            let (line, column) = byte_offset_to_line_column(text, offset);
            assert_eq!(
                (line, column),
                (expected_line, expected_column),
                "Failed for text '{}' at offset {}: expected line {}, column {}, got line {}, column {}",
                text.replace('\n', "\\n"), offset, expected_line, expected_column, line, column
            );
        }
    }

    #[test]
    fn error_location_unicode_handling() {
        let test_cases = vec![
            // Unicode characters should be counted as single characters for column positions
            ("🚀", 0, 1, 1),     // Start of emoji
            ("🚀", 4, 1, 2),     // After emoji (emojis are 4 bytes in UTF-8)
            ("🚀a", 4, 1, 2),    // Between emoji and ASCII
            ("🚀a", 5, 1, 3),    // After ASCII following emoji
            ("🚀\n🔬", 5, 2, 1), // Start of line 2 after emoji and newline
            ("🚀\n🔬", 9, 2, 2), // After second emoji
            ("café", 0, 1, 1),   // Start
            ("café", 3, 1, 4),   // Before the é (é is 2 bytes)
            ("café", 5, 1, 5),   // End of text with accented char
        ];

        for (text, offset, expected_line, expected_column) in test_cases {
            let (line, column) = byte_offset_to_line_column(text, offset);
            assert_eq!(
                (line, column),
                (expected_line, expected_column),
                "Failed for text '{text}' at offset {offset}: expected line {expected_line}, column {expected_column}, got line {line}, column {column}",
            );
        }
    }

    #[test]
    fn error_location_different_line_endings() {
        let test_cases = vec![
            // Unix line endings (\n)
            ("line1\nline2", 6, 2, 1),
            // Windows line endings (\r\n) - \r should be treated as regular character
            ("line1\r\nline2", 6, 1, 7), // \r is at position 6, still on line 1
            ("line1\r\nline2", 7, 2, 1), // \n at position 7 starts line 2
            ("line1\r\nline2", 8, 2, 2), // First char of line2
            // Old Mac line endings (\r only) - \r should NOT start new line
            ("line1\rline2", 6, 1, 7), // \r is just another character
            ("line1\rline2", 7, 1, 8), // Next char after \r
        ];

        for (text, offset, expected_line, expected_column) in test_cases {
            let (line, column) = byte_offset_to_line_column(text, offset);
            assert_eq!(
                (line, column),
                (expected_line, expected_column),
                "Failed for text '{}' at offset {}: expected line {}, column {}, got line {}, column {}",
                text.replace('\n', "\\n").replace('\r', "\\r"), offset, expected_line, expected_column, line, column
            );
        }
    }

    #[test]
    fn error_location_edge_cases() {
        // Empty string
        let (line, column) = byte_offset_to_line_column("", 0);
        assert_eq!((line, column), (1, 1));

        // Offset beyond text length should not panic and should give reasonable result
        let (line, column) = byte_offset_to_line_column("hello", 10);
        assert_eq!((line, column), (1, 6)); // Should stop at end of text

        // Multiple consecutive newlines
        let (line, column) = byte_offset_to_line_column("\n\n\n", 0);
        assert_eq!((line, column), (1, 1));
        let (line, column) = byte_offset_to_line_column("\n\n\n", 1);
        assert_eq!((line, column), (2, 1));
        let (line, column) = byte_offset_to_line_column("\n\n\n", 2);
        assert_eq!((line, column), (3, 1));
        let (line, column) = byte_offset_to_line_column("\n\n\n", 3);
        assert_eq!((line, column), (4, 1));
    }
}
