use std::str::Utf8Error;

use naviz_parser::input::concrete::Instructions;

pub mod mqt;
pub mod separated_display;

/// The available import formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ImportFormat {
    /// [mqt::na]
    MQTNA,
}

/// List of all import-formats (all entries of [ImportFormat]).
pub static IMPORT_FORMATS: [ImportFormat; 1] = [ImportFormat::MQTNA];

/// The options for the different import formats
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ImportOptions {
    /// [mqt::na]
    MQTNA(mqt::na::convert::ConvertOptions<'static>),
}

/// An error that can occur during import
#[derive(Debug, Clone, PartialEq)]
pub enum ImportError {
    /// Something was not valid UTF-8
    InvalidUtf8(Utf8Error),
    /// An error occurred while parsing [mqt::na]
    MQTNAParse(mqt::na::format::ParseErrorInner),
    /// An error occurred while converting [mqt::na]
    MQTNAConvert(mqt::na::convert::OperationConversionError),
}

impl ImportFormat {
    /// A human-readable name of this [ImportFormat]
    pub fn name(&self) -> &'static str {
        match self {
            Self::MQTNA => "mqt na",
        }
    }

    /// A list of file-extensions commonly used by this [ImportFormat]
    pub fn file_extensions(&self) -> &'static [&'static str] {
        match self {
            Self::MQTNA => &["na"],
        }
    }
}

impl From<ImportFormat> for ImportOptions {
    fn from(value: ImportFormat) -> Self {
        match value {
            ImportFormat::MQTNA => ImportOptions::MQTNA(Default::default()),
        }
    }
}

impl From<&ImportOptions> for ImportFormat {
    fn from(value: &ImportOptions) -> Self {
        match value {
            &ImportOptions::MQTNA(_) => ImportFormat::MQTNA,
        }
    }
}

impl ImportOptions {
    /// Imports the `data` using the options in `self`
    pub fn import(self, data: &[u8]) -> Result<Instructions, ImportError> {
        match self {
            Self::MQTNA(options) => mqt::na::convert::convert(
                &mqt::na::format::parse(
                    std::str::from_utf8(data).map_err(ImportError::InvalidUtf8)?,
                )
                .map_err(|e| e.into_inner())
                .map_err(ImportError::MQTNAParse)?,
                options,
            )
            .map_err(ImportError::MQTNAConvert),
        }
    }
}
