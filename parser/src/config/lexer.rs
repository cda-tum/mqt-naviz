//! Lexer for the config.
//!
//! Use [lex] to lex some input.

use crate::{common, ParseError};
use token::*;
use winnow::{
    ascii::multispace0,
    combinator::{delimited, repeat},
    prelude::*,
    stream::{AsChar, Compare, FindSlice, SliceLen, Stream, StreamIsPartial},
};

// Re-export the common lexer
pub use common::lexer::*;

/// A token of the config format
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Token<T> {
    /// Opening-symbol for a block or a set (overloaded due to same symbol)
    BlockOrSetOpen,
    /// Closing-symbol for a block or a set (overloaded due to same symbol)
    BlockOrSetClose,
    /// An identifier
    Identifier(T),
    /// A property-separator
    Separator,
    /// A value; see [Value]
    Value(Value<T>),
    /// A comment, either single- or multiline
    Comment(T),
    /// Opening-symbol for a tuple
    TupleOpen,
    /// Closing-symbol for a tuple
    TupleClose,
    /// A separator between elements (e.g., in tuples)
    ElementSeparator,
}

impl<T> From<GenericToken<T>> for Token<T> {
    fn from(value: GenericToken<T>) -> Self {
        match value {
            GenericToken::Identifier(i) => Self::Identifier(i),
            GenericToken::Value(v) => Self::Value(v),
            GenericToken::Comment(c) => Self::Comment(c),
            GenericToken::TupleOpen => Self::TupleOpen,
            GenericToken::TupleClose => Self::TupleClose,
            // convert to overloaded tokens
            GenericToken::SetOpen => Self::BlockOrSetOpen,
            GenericToken::SetClose => Self::BlockOrSetClose,
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
            // also convert from overloaded tokens
            Token::BlockOrSetOpen => Some(GenericToken::SetOpen),
            Token::BlockOrSetClose => Some(GenericToken::SetClose),
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
) -> Result<Vec<Token<<I as Stream>::Slice>>, ParseError<I>>
where
    <I as Stream>::Token: AsChar + Clone,
    I::Slice: SliceLen,
{
    repeat(0.., delimited(multispace0, token, multispace0)).parse(input)
}

/// Lexers to lex individual [Token]s.
///
/// All lexers assume their token starts immediately (i.e., no preceding whitespace)
/// and will not consume any subsequent whitespace,
/// except when that whitespace is used as a delimiter,
/// in which case it will be noted.
pub mod token {
    use super::*;
    use winnow::{
        combinator::alt,
        stream::{AsChar, Compare, FindSlice, SliceLen},
    };

    pub use common::lexer::token::*;

    /// Tries to parse a [Token::BlockOrSetOpen].
    pub fn block_or_set_open<I: Stream + StreamIsPartial + Compare<&'static str>>(
        input: &mut I,
    ) -> ModalResult<Token<<I as Stream>::Slice>> {
        "{".map(|_| Token::BlockOrSetOpen).parse_next(input)
    }

    /// Tries to parse a [Token::BlockClose].
    pub fn block_or_set_close<I: Stream + StreamIsPartial + Compare<&'static str>>(
        input: &mut I,
    ) -> ModalResult<Token<<I as Stream>::Slice>> {
        "}".map(|_| Token::BlockOrSetClose).parse_next(input)
    }

    /// Tries to parse a [Token::Separator].
    pub fn separator<I: Stream + StreamIsPartial + Compare<&'static str>>(
        input: &mut I,
    ) -> ModalResult<Token<<I as Stream>::Slice>> {
        ":".map(|_| Token::Separator).parse_next(input)
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
            block_or_set_open,
            block_or_set_close,
            separator,
            tuple_open,
            tuple_close,
            element_separator,
            comment,
            value,
            identifier,
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
        property1: "some string"
        property2: 1.2
        ^regex$ /* comment */ : identifier // other comment
        block {
            named_block "name" {
                prop: #c01032
                42: true
            }
        // }
        }
        "#;

        let expected = vec![
            Token::Identifier("property1"),
            Token::Separator,
            Token::Value(Value::String("some string")),
            Token::Identifier("property2"),
            Token::Separator,
            Token::Value(Value::Number("1.2")),
            Token::Value(Value::Regex("^regex$")),
            Token::Comment(" comment "),
            Token::Separator,
            Token::Identifier("identifier"),
            Token::Comment(" other comment"),
            Token::Identifier("block"),
            Token::BlockOrSetOpen,
            Token::Identifier("named_block"),
            Token::Value(Value::String("name")),
            Token::BlockOrSetOpen,
            Token::Identifier("prop"),
            Token::Separator,
            Token::Value(Value::Color("c01032")),
            Token::Value(Value::Number("42")),
            Token::Separator,
            Token::Value(Value::Boolean("true")),
            Token::BlockOrSetClose,
            Token::Comment(" }"),
            Token::BlockOrSetClose,
        ];

        let lexed = lex(input).expect("Failed to lex");

        assert_eq!(lexed, expected);
    }

    #[test]
    fn error_location_byte_offset_conversion() {
        let test_cases = vec![
            // (text, offset, expected_line, expected_column)
            ("property: value", 0, 1, 1),              // Start of text
            ("property: value", 8, 1, 9),              // At colon
            ("property: value", 10, 1, 11),            // After space and before 'v'
            ("name: \"test\"\ncolor: blue", 13, 2, 1), // Start of line 2 (offset after newline)
            ("name: \"test\"\ncolor: blue", 19, 2, 7), // At space before 'blue'
            ("a: b\nc: d\ne: f", 5, 2, 1), // Start of line 2 (offset after first newline)
            ("a: b\nc: d\ne: f", 10, 3, 1), // Start of line 3 (offset after second newline)
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
    fn error_location_config_unicode() {
        let test_cases = vec![
            // Config files with Unicode content
            ("name: \"🚀 test\"", 0, 1, 1),    // Start
            ("name: \"🚀 test\"", 7, 1, 8),    // At emoji (byte 7, char 8)
            ("name: \"🚀 test\"", 11, 1, 9),   // After emoji, at space
            ("🚀: value\n🔬: data", 12, 2, 1), // Start of line 2 with Unicode key (offset after newline)
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
    fn error_location_calculation_with_tabs() {
        let test_cases = vec![
            ("line1\n\tindented", 6, (2, 1)), // Start of line 2 with tab
            ("line1\n\tindented", 7, (2, 2)), // After tab character
            ("no_tabs", 3, (1, 4)),           // Simple case
            ("tab\there", 4, (1, 5)),         // After tab in same line
        ];

        for (input, offset, expected) in test_cases {
            let result = byte_offset_to_line_column(input, offset);
            assert_eq!(
                result,
                expected,
                "Failed for input '{}' at offset {}: expected {:?}, got {:?}",
                input.replace('\n', "\\n").replace('\t', "\\t"),
                offset,
                expected,
                result
            );
        }
    }

    #[test]
    fn error_location_edge_cases() {
        // Test edge cases for error location calculation

        // Empty string
        let result = byte_offset_to_line_column("", 0);
        assert_eq!(result, (1, 1));

        // Single character
        let result = byte_offset_to_line_column("a", 0);
        assert_eq!(result, (1, 1));

        // Just newlines
        let result = byte_offset_to_line_column("\n\n\n", 2);
        assert_eq!(result, (3, 1));

        // End of string
        let input = "hello\nworld";
        let result = byte_offset_to_line_column(input, input.len());
        assert_eq!(result, (2, 6)); // After 'world'
    }
}
