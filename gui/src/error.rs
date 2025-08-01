use std::str::Utf8Error;

use naviz_import::ImportError;
use naviz_parser::{config, input::concrete::ParseInstructionsError, ParseErrorInner};

/// A [Result][std::result::Result] pre-filled with [Error]
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error that can occur in the gui
#[derive(Debug)]
pub enum Error {
    /// Error while opening a file.
    /// For errors when importing, see [Error::Import].
    FileOpen(InputType),
    /// Error while importing from another format.
    Import(ImportError),
    /// Error when interacting with the repository.
    Repository(RepositoryError, ConfigFormat),
}

/// An Error occurred while opening one of the input-types
#[derive(Debug)]
pub enum InputType {
    Instruction(InputError),
    Config(ConfigFormat, ConfigError),
}

/// An error to do with the instruction input
#[derive(Debug)]
pub enum InputError {
    UTF8(Utf8Error),
    Lex(ParseErrorInner),
    Parse(ParseErrorInner),
    Convert(ParseInstructionsError),
}

/// A config-format
#[derive(Debug)]
pub enum ConfigFormat {
    Machine,
    Style,
}

/// An error to do with the config files
#[derive(Debug)]
pub enum ConfigError {
    UTF8(Utf8Error),
    Lex(ParseErrorInner),
    Parse(ParseErrorInner),
    Convert(config::error::Error),
}
/// An error to do with the [Repository][naviz_repository::Repository]
#[derive(Debug)]
pub enum RepositoryError {
    Load(RepositoryLoadSource, naviz_repository::error::Error),
    Open(naviz_repository::error::Error),
    Import(naviz_repository::error::Error),
    Remove(naviz_repository::error::Error),
    Search,
}

/// The source where the [Repository][naviz_repository::Repository] was loaded from
#[derive(Debug)]
pub enum RepositoryLoadSource {
    Bundled,
    UserDir,
}

impl Error {
    /// Short message of this [Error].
    /// Can be displayed in the title-bar.
    #[rustfmt::skip]
    pub fn title(&self) -> &'static str {
        match self {
            Self::FileOpen(InputType::Config(ConfigFormat::Machine, _)) => "Invalid machine definition",
            Self::FileOpen(InputType::Config(ConfigFormat::Style, _)) => "Invalid style definition",
            Self::FileOpen(InputType::Instruction(_)) => "Invalid instructions",
            Self::Import(_) => "Failed to import",
            Self::Repository(RepositoryError::Search, ConfigFormat::Machine) => "Machine not found",
            Self::Repository(RepositoryError::Search, ConfigFormat::Style) => "Style not found",
            Self::Repository(RepositoryError::Open(_), ConfigFormat::Machine) => "Invalid machine in repository",
            Self::Repository(RepositoryError::Open(_), ConfigFormat::Style) => "Invalid style in repository",
            Self::Repository(RepositoryError::Load(RepositoryLoadSource::Bundled, _), ConfigFormat::Machine) => "Failed to load bundled machines",
            Self::Repository(RepositoryError::Load(RepositoryLoadSource::Bundled, _), ConfigFormat::Style) => "Failed to load bundled styles",
            Self::Repository(RepositoryError::Load(RepositoryLoadSource::UserDir, _), ConfigFormat::Machine) => "Failed to load machines from user-dir",
            Self::Repository(RepositoryError::Load(RepositoryLoadSource::UserDir, _), ConfigFormat::Style) => "Failed to load styles from user-dir",
            Self::Repository(RepositoryError::Import(_), ConfigFormat::Machine) => "Failed to import machine to user-dir",
            Self::Repository(RepositoryError::Import(_), ConfigFormat::Style) => "Failed to import machine to user-dir",
            Self::Repository(RepositoryError::Remove(_), ConfigFormat::Machine) => "Failed to remove machine from user-dir",
            Self::Repository(RepositoryError::Remove(_), ConfigFormat::Style) => "Failed to remove style from user-dir",
        }
    }

    /// A longer text representation of this [Error].
    /// Contains details.
    pub fn body(&self) -> String {
        // Just print out debug for now
        format!("{self:#?}")
    }
}
