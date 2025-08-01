use std::{error::Error, fmt::Display, num::ParseIntError, str::FromStr};

/// A 32-bit color with `red`, `green`, `blue`, and `alpha` components
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// An error when parsing a [Color] using [FromStr]
#[derive(Debug)]
pub enum ParseColorError {
    ParseIntError(ParseIntError),
    InvalidLengthError,
}

impl Display for ParseColorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseIntError(e) => write!(f, "error while parsing number: {e}"),
            Self::InvalidLengthError => write!(f, "invalid length for color"),
        }
    }
}

impl Error for ParseColorError {
    #[allow(deprecated)]
    fn description(&self) -> &str {
        match self {
            Self::InvalidLengthError => "invalid length for color",
            Self::ParseIntError(e) => e.description(),
        }
    }
}

impl FromStr for Color {
    type Err = ParseColorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Function to parse a 8-bit number from a hex-string starting from the specified offset.
        let hex = |start: usize| {
            u8::from_str_radix(&s[start..=(start + 1)], 16).map_err(ParseColorError::ParseIntError)
        };

        match s.len() {
            6 => Ok(Color {
                r: hex(0)?,
                g: hex(2)?,
                b: hex(4)?,
                a: u8::MAX,
            }),
            8 => Ok(Color {
                r: hex(0)?,
                g: hex(2)?,
                b: hex(4)?,
                a: hex(6)?,
            }),
            _ => Err(ParseColorError::InvalidLengthError),
        }
    }
}

impl Color {
    /// Gets this color in `RGBA`-format
    pub fn rgba(&self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Gets this color in `ARGB`-format
    pub fn argb(&self) -> [u8; 4] {
        [self.a, self.r, self.g, self.b]
    }
}
