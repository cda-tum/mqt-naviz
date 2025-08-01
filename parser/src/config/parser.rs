use super::lexer::Token;
use crate::{common, ParseError};
use std::fmt::Debug;
use token::{block_close, block_open, identifier, ignore_comments, separator};
use try_into_value::TryIntoValue;
use winnow::combinator::{alt, preceded, repeat, terminated};
use winnow::prelude::*;

// Re-export the common parser
pub use common::parser::*;

/// A [ConfigItem] represents a single item of the config.
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub enum ConfigItem {
    // `key`, `value`
    Property(Value, Value),
    // `identifier`, `content`
    Block(String, Config),
    // `identifier`, `name`, `content`
    NamedBlock(String, Value, Config),
}

/// A [Config] is all [ConfigItem]s of a parsed config.
pub type Config = Vec<ConfigItem>;

/// Parse a full stream of [Token]s into a [Config].
pub fn parse<S: TryIntoValue + Clone + Debug + PartialEq>(
    input: &[Token<S>],
) -> Result<Config, ParseError<&[Token<S>]>> {
    config.parse(input)
}

/// Try to parse a [Config] from a stream of [Token]s.
pub fn config<S: TryIntoValue + Clone + Debug + PartialEq>(
    input: &mut &[Token<S>],
) -> PResult<Config> {
    preceded(
        ignore_comments,
        repeat(0.., terminated(config_item, ignore_comments)),
    )
    .parse_next(input)
}

/// Try to parse a single [ConfigItem] from a stream of [Token]s.
pub fn config_item<S: TryIntoValue + Clone + Debug + PartialEq>(
    input: &mut &[Token<S>],
) -> PResult<ConfigItem> {
    alt((property, block, named_block)).parse_next(input)
}

/// Try to parse a [ConfigItem::Property] from a stream of [Token]s.
pub fn property<S: TryIntoValue + Clone + Debug + PartialEq>(
    input: &mut &[Token<S>],
) -> PResult<ConfigItem> {
    (
        terminated(any_value, ignore_comments),
        terminated(separator, ignore_comments),
        any_value,
    )
        .map(|(k, _, v)| ConfigItem::Property(k, v))
        .parse_next(input)
}

/// Try to parse a [ConfigItem::Block] from a stream of [Token]s.
pub fn block<S: TryIntoValue + Clone + Debug + PartialEq>(
    input: &mut &[Token<S>],
) -> PResult<ConfigItem> {
    (
        terminated(identifier, ignore_comments),
        terminated(block_open, ignore_comments),
        terminated(config, ignore_comments),
        block_close,
    )
        .map(|(i, _, c, _)| ConfigItem::Block(i, c))
        .parse_next(input)
}

/// Try to parse a [ConfigItem::NamedBlock] from a stream of [Token]s.
pub fn named_block<S: TryIntoValue + Clone + Debug + PartialEq>(
    input: &mut &[Token<S>],
) -> PResult<ConfigItem> {
    (
        terminated(identifier, ignore_comments),
        terminated(any_value, ignore_comments),
        terminated(block_open, ignore_comments),
        terminated(config, ignore_comments),
        block_close,
    )
        .map(|(i, n, _, c, _)| ConfigItem::NamedBlock(i, n, c))
        .parse_next(input)
}

/// Parse single [Token]s into their abstract config counterparts.
pub mod token {
    use super::*;
    use winnow::token::one_of;

    // Re-export the common token-parsers
    pub use common::parser::token::*;

    /// Try to parse a single [Token::BlockOrSetOpen] for opening a block.
    pub fn block_open<S: Clone + Debug + PartialEq>(input: &mut &[Token<S>]) -> PResult<()> {
        one_of([Token::BlockOrSetOpen]).void().parse_next(input)
    }

    /// Try to parse a single [Token::BlockOrSetClose] for closing a block.
    pub fn block_close<S: Clone + Debug + PartialEq>(input: &mut &[Token<S>]) -> PResult<()> {
        one_of([Token::BlockOrSetClose]).void().parse_next(input)
    }

    /// Try to parse a single [Token::Separator].
    pub fn separator<S: Clone + Debug + PartialEq>(input: &mut &[Token<S>]) -> PResult<()> {
        one_of([Token::Separator]).void().parse_next(input)
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
    use fraction::Fraction;
    use regex::Regex;

    use super::*;
    use crate::common::{color::Color, lexer};

    #[test]
    pub fn simple_example() {
        let input = [
            Token::Identifier("property1"),
            Token::Separator,
            Token::Value(lexer::Value::String("some string")),
            Token::Identifier("property2"),
            Token::Separator,
            Token::Value(lexer::Value::Number("1.2")),
            Token::Value(lexer::Value::Regex("regex")),
            Token::Comment(" comment "),
            Token::Separator,
            Token::Identifier("identifier"),
            Token::Comment(" other comment"),
            Token::Identifier("block"),
            Token::BlockOrSetOpen,
            Token::Identifier("named_block"),
            Token::Value(lexer::Value::String("name")),
            Token::BlockOrSetOpen,
            Token::Identifier("prop"),
            Token::Separator,
            Token::Value(lexer::Value::Color("c01032")),
            Token::Value(lexer::Value::Number("42")),
            Token::Separator,
            Token::Value(lexer::Value::Boolean("true")),
            Token::BlockOrSetClose,
            Token::Comment(" }"),
            Token::BlockOrSetClose,
        ];

        let expected = vec![
            ConfigItem::Property(
                Value::Identifier("property1".to_string()),
                Value::String("some string".to_string()),
            ),
            ConfigItem::Property(
                Value::Identifier("property2".to_string()),
                Value::Number(Fraction::new(12u64, 10u64)),
            ),
            ConfigItem::Property(
                Value::Regex(Regex::new("regex").unwrap()),
                Value::Identifier("identifier".to_string()),
            ),
            ConfigItem::Block(
                "block".to_string(),
                vec![ConfigItem::NamedBlock(
                    "named_block".to_string(),
                    Value::String("name".to_string()),
                    vec![
                        ConfigItem::Property(
                            Value::Identifier("prop".to_string()),
                            Value::Color(Color {
                                r: 192,
                                g: 16,
                                b: 50,
                                a: 255,
                            }),
                        ),
                        ConfigItem::Property(
                            Value::Number(Fraction::new(42u64, 1u64)),
                            Value::Boolean(true),
                        ),
                    ],
                )],
            ),
        ];

        let actual = parse(&input).expect("Failed to parse");

        assert_eq!(actual, expected);
    }
}
