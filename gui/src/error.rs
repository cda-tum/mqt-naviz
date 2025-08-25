use std::str::Utf8Error;

use naviz_import::ImportError;
use naviz_parser::{
    byte_offset_to_line_column, config, input::concrete::ParseInstructionsError, ParseErrorInner,
};

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
    Lex(ParseErrorInner, Option<ErrorLocation>),
    Parse(ParseErrorInner, Option<ErrorLocation>),
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
    Lex(ParseErrorInner, Option<ErrorLocation>),
    Parse(ParseErrorInner, Option<ErrorLocation>),
    Convert(config::error::Error),
}
/// An error to do with the [Repository][naviz_repository::Repository]
#[derive(Debug)]
pub enum RepositoryError {
    Load(RepositoryLoadSource, naviz_repository::error::Error),
    Open(naviz_repository::error::Error),
    Import(naviz_repository::error::Error, Option<ErrorLocation>),
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
            Self::Repository(RepositoryError::Import(_, _), ConfigFormat::Machine) => "Failed to import machine to user-dir",
            Self::Repository(RepositoryError::Import(_, _), ConfigFormat::Style) => "Failed to import style to user-dir",
            Self::Repository(RepositoryError::Remove(_), ConfigFormat::Machine) => "Failed to remove machine from user-dir",
            Self::Repository(RepositoryError::Remove(_), ConfigFormat::Style) => "Failed to remove style from user-dir",
        }
    }

    /// A longer text representation of this [Error].
    /// Contains details and location information when available.
    pub fn body(&self) -> String {
        match self {
            Self::FileOpen(InputType::Config(format, config_error)) => {
                format_config_error(format, config_error)
            }
            Self::FileOpen(InputType::Instruction(input_error)) => format_input_error(input_error),
            Self::Import(import_error) => format_import_error(import_error),
            Self::Repository(repo_error, config_format) => {
                format_repository_error(repo_error, config_format)
            }
        }
    }
}

/// Format a config error with location information
fn format_config_error(format: &ConfigFormat, error: &ConfigError) -> String {
    let file_type = match format {
        ConfigFormat::Machine => "machine configuration",
        ConfigFormat::Style => "style configuration",
    };

    match error {
        ConfigError::UTF8(utf8_error) => {
            format!(
                "The {file_type} file contains invalid UTF-8 encoding.\n\n\
                Error details: {utf8_error}\n\n\
                Please ensure the file is saved with proper UTF-8 encoding."
            )
        }
        ConfigError::Lex(parse_error, location) => {
            let location_info = location
                .as_ref()
                .map(|loc| format!(" at line {}, column {}", loc.line, loc.column))
                .unwrap_or_default();
            format!(
                "Failed to parse {} file due to invalid syntax{}.\n\n\
                Lexical analysis error: {}\n\n\
                This typically means:\n\
                • Unexpected characters or symbols\n\
                • Invalid token sequences\n\
                • Malformed identifiers or literals\n\n\
                Please check the file syntax and ensure it follows the expected format.",
                file_type,
                location_info,
                format_parse_error_context(parse_error)
            )
        }
        ConfigError::Parse(parse_error, location) => {
            let location_info = location
                .as_ref()
                .map(|loc| format!(" at line {}, column {}", loc.line, loc.column))
                .unwrap_or_default();
            format!(
                "Failed to parse {} file structure{}.\n\n\
                Parsing error: {}\n\n\
                This typically means:\n\
                • Brackets, braces, or parentheses are not properly matched\n\
                • Missing or extra punctuation\n\
                • Field names are misspelled or invalid\n\
                • Values are in an unexpected format\n\n\
                Please verify the file structure and syntax.",
                file_type,
                location_info,
                format_parse_error_context(parse_error)
            )
        }
        ConfigError::Convert(config_error) => {
            format!(
                "Failed to process {file_type} file content.\n\n\
                {config_error}\n\n\
                The file syntax is correct, but the content validation failed.\n\
                Please check that all required fields are present and have valid values."
            )
        }
    }
}

/// Format an input error with location information
fn format_input_error(error: &InputError) -> String {
    match error {
        InputError::UTF8(utf8_error) => {
            format!(
                "The instruction file contains invalid UTF-8 encoding.\n\n\
                Error details: {utf8_error}\n\n\
                Please ensure the file is saved with proper UTF-8 encoding."
            )
        }
        InputError::Lex(parse_error, location) => {
            let location_info = location
                .as_ref()
                .map(|loc| format!(" at line {}, column {}", loc.line, loc.column))
                .unwrap_or_default();
            format!(
                "Failed to parse instruction file due to invalid syntax{}.\n\n\
                Lexical analysis error: {}\n\n\
                Common issues:\n\
                • Unrecognized characters or symbols\n\
                • Incomplete string literals or comments\n\
                • Invalid number formats\n\
                • Malformed identifiers\n\n\
                Please check the file syntax.",
                location_info,
                format_parse_error_context(parse_error)
            )
        }
        InputError::Parse(parse_error, location) => {
            let location_info = location
                .as_ref()
                .map(|loc| format!(" at line {}, column {}", loc.line, loc.column))
                .unwrap_or_default();
            format!(
                "Failed to parse instruction file structure{}.\n\n\
                Parsing error: {}\n\n\
                Common issues:\n\
                • Instructions are not properly formatted\n\
                • Parentheses, brackets, or braces are unbalanced\n\
                • Missing semicolons or separators\n\
                • Invalid gate names or parameters\n\n\
                Please verify the instruction syntax.",
                location_info,
                format_parse_error_context(parse_error)
            )
        }
        InputError::Convert(convert_error) => format_parse_instructions_error(convert_error),
    }
}

/// Format parse error context information in a user-friendly way
fn format_parse_error_context(error: &ParseErrorInner) -> String {
    // Extract context information from winnow's ContextError
    let context_info: Vec<String> = error.context().map(|ctx| ctx.to_string()).collect();

    if !context_info.is_empty() {
        let contexts = context_info.join(", ");
        format!("Expected: {contexts}")
    } else {
        // If no context is available, provide more generic information
        "Syntax error encountered while parsing".to_string()
    }
}

/// Format an import error with helpful context
fn format_import_error(error: &ImportError) -> String {
    match error {
        ImportError::InvalidUtf8(utf8_error) => {
            format!(
                "The imported file contains invalid UTF-8 encoding.\n\n\
                Error details: {utf8_error}\n\n\
                Please ensure the file is saved with proper UTF-8 encoding."
            )
        }
        ImportError::MqtNqParse(parse_error) => {
            format!(
                "Failed to parse the MQT-NA format file.\n\n\
                Parsing error: {}\n\n\
                This typically means:\n\
                • Invalid MQT-NA syntax or format\n\
                • Unrecognized quantum operation names\n\
                • Malformed operation parameters or arguments\n\
                • Missing or incorrect punctuation (semicolons, parentheses)\n\
                • Invalid qubit register declarations\n\n\
                Please verify that:\n\
                • The file follows the MQT-NA format specification\n\
                • All quantum operations are properly formatted\n\
                • Register declarations are valid",
                format_parse_error_context(parse_error)
            )
        }
        ImportError::MqtNqConvert(convert_error) => {
            format!(
                "Failed to convert MQT-NA operations.\n\n\
                Conversion error: {convert_error:?}\n\n\
                The file was parsed successfully, but some operations could not be converted.\n\
                This may indicate:\n\
                • Unsupported gate types or operations\n\
                • Invalid operation parameters or qubit indices\n\
                • Incompatible quantum circuit structure\n\
                • Operations not supported by the target format"
            )
        }
    }
}

/// Format a repository error with context
fn format_repository_error(error: &RepositoryError, config_format: &ConfigFormat) -> String {
    let item_type = match config_format {
        ConfigFormat::Machine => "machine",
        ConfigFormat::Style => "style",
    };

    match error {
        RepositoryError::Load(source, repo_error) => {
            let source_desc = match source {
                RepositoryLoadSource::Bundled => "bundled",
                RepositoryLoadSource::UserDir => "user directory",
            };
            format!(
                "Failed to load {item_type} definitions from {source_desc}.\n\n\
                Error: {repo_error:?}\n\n\
                This may indicate:\n\
                • Corrupted files in the {source_desc} repository\n\
                • Missing or inaccessible files\n\
                • Permission issues"
            )
        }
        RepositoryError::Open(repo_error) => {
            format!(
                "Failed to open {item_type} from repository.\n\n\
                Error: {repo_error:?}\n\n\
                The {item_type} file may be corrupted or in an incompatible format."
            )
        }
        RepositoryError::Import(repo_error, location) => {
            use naviz_repository::error::Error as RErr;
            match repo_error {
                RErr::IoError(ioe) => format!(
                    "Failed to import {item_type} to user directory.\n\n\
                    I/O error: {ioe}\n\n\
                    This may be due to:\n\
                    • Insufficient permissions to read or write the file\n\
                    • The file being locked by another process\n\
                    • Disk space limitations or file system errors\n\
                    Please verify file permissions and available disk space."
                ),
                RErr::UTF8Error(utf8) => format!(
                    "Failed to import {item_type} – the file is not valid UTF-8.\n\n\
                    Error details: {utf8}\n\n\
                    Ensure the file is saved using UTF-8 encoding (without BOM)."
                ),
                RErr::LexError(offset, inner) => {
                    let loc_info = location
                        .as_ref()
                        .map(|l| format!(" (line {}, column {})", l.line, l.column))
                        .unwrap_or_else(String::new);
                    format!(
                        "Failed to lex {} definition{} at byte offset {}.\n\n\
                        {}\n\
                        Probable causes:\n\
                        • Unexpected or invalid character\n\
                        • Unterminated string / regex / comment\n\
                        • Malformed number, percentage, or color literal\n\
                        • Missing ':' between key and value\n\
                        {}\n\
                        Please correct the syntax and try again.",
                        item_type,
                        loc_info,
                        offset,
                        format_parse_error_context(inner),
                        format_expected_hint(inner)
                    )
                }
                RErr::ParseError(offset, inner) => {
                    let loc_info = location
                        .as_ref()
                        .map(|l| format!(" (line {}, column {})", l.line, l.column))
                        .unwrap_or_else(String::new);
                    format!(
                        "Failed to parse {} structure{} at byte offset {}.\n\n\
                        {}\n\
                        Probable causes:\n\
                        • Unbalanced braces / parentheses / brackets\n\
                        • Misplaced or missing block delimiters\n\
                        • Extra or missing commas / separators\n\
                        • Incorrect ordering of keys or blocks\n\
                        {}\n\
                        Please review structural syntax and retry.",
                        item_type,
                        loc_info,
                        offset,
                        format_parse_error_context(inner),
                        format_expected_hint(inner)
                    )
                }
                RErr::ConfigReadError(cfg_err) => format!(
                    "Imported {item_type} file has invalid semantic content.\n\n\
                    Validation error: {cfg_err}\n\
                    The syntax was correct but values failed validation.\n\
                    Check that required fields exist and values are within allowed ranges."
                ),
                RErr::IdError => format!(
                    "Imported {item_type} contains invalid or duplicate identifiers.\n\n\
                    Ensure all identifiers are unique and follow naming rules."
                ),
                RErr::NotRemovableError => format!(
                    "Internal error: attempted to treat a non-removable {item_type} as removable during import."
                ),
             }
        }
        RepositoryError::Remove(repo_error) => format!(
            "Failed to remove {item_type} from user directory.\n\n\
                Error: {repo_error:?}\n\n\
                This may be due to:\n\
                • Insufficient permissions\n\
                • File is currently in use\n\
                • File system errors"
        ),
        RepositoryError::Search => format!(
            "Could not find the requested {item_type} in the repository.\n\n\
                Please verify:\n\
                • The {item_type} name is spelled correctly\n\
                • The {item_type} exists in the repository\n\
                • The repository was loaded successfully"
        ),
    }
}
/// Provide an extra hint listing expected contexts if available (repository import helper)
fn format_expected_hint(inner: &ParseErrorInner) -> String {
    let contexts: Vec<String> = inner.context().map(|c| c.to_string()).collect();
    if contexts.is_empty() {
        String::new()
    } else {
        format!("Expected one of: {}", contexts.join(", "))
    }
}

/// Location information for parsing errors
#[derive(Debug, Clone)]
pub struct ErrorLocation {
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
    /// Byte offset in the original text
    pub offset: usize,
}

impl ErrorLocation {
    /// Create ErrorLocation from byte offset and original text
    pub fn from_offset(text: &str, offset: usize) -> Self {
        let (line, column) = byte_offset_to_line_column(text, offset);
        Self {
            line,
            column,
            offset,
        }
    }
}

/// Format a parse instructions error with detailed messages
fn format_parse_instructions_error(err: &ParseInstructionsError) -> String {
    match err {
        ParseInstructionsError::UnknownInstruction { name } => format!(
            "Unknown instruction '{name}'.\n\n\
             This instruction name is not recognized.\n\
             Possible causes:\n\
             • Typo in the instruction name\n\
             • The instruction isn't supported yet\n\
             • A missing feature or version mismatch\n\n\
             Check the documentation for the list of supported instructions."
        ),
        ParseInstructionsError::UnknownDirective { name } => format!(
            "Unknown directive '{name}'.\n\n\
             The directive name is not recognized.\n\
             Ensure the directive is spelled correctly and supported."
        ),
        ParseInstructionsError::WrongNumberOfArguments {
            name,
            expected,
            actual,
        } => {
            let expected_list = if expected.len() == 1 {
                expected[0].to_string()
            } else {
                expected
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" or ")
            };
            format!(
                "Instruction/directive '{name}' called with wrong number of arguments.\n\n\
                 Expected: {expected_list} argument(s)\n\
                 Actual: {actual}\n\n\
                 Adjust the argument list to match the expected arity."
            )
        }
        ParseInstructionsError::WrongTypeOfArgument { name, expected } => {
            // expected is &[ &[&str] ] representing alternative signatures
            let sigs = expected
                .iter()
                .map(|alts| alts.join(", "))
                .collect::<Vec<_>>();
            let expected_str = if sigs.len() == 1 {
                sigs[0].clone()
            } else {
                sigs.join(" | ")
            };
            format!(
                "Instruction/directive '{name}' called with wrong argument types.\n\n\
                 Expected one of:\n  {expected_str}\n\n\
                 Ensure each argument matches the required type."
            )
        }
        ParseInstructionsError::MissingTime { name } => {
            let list = name.join(", ");
            format!(
                "Missing time specifier for timed instruction(s): {list}.\n\n\
                 Timed instructions require a leading time marker such as '@0', '@+3', '@=5', etc.\n\
                 Add an absolute '@<time>' or relative '@+/-<delta>' before the instruction."
            )
        }
        ParseInstructionsError::SuperfluousTime { name } => {
            let list = name.join(", ");
            format!(
                "Superfluous time specifier for setup instruction(s): {list}.\n\n\
                 Setup instructions must not be preceded by a time marker.\n\
                 Remove the '@<time>' prefix."
            )
        }
    }
}

#[cfg(test)]
mod parse_instructions_error_format_tests {
    use super::*;

    #[test]
    fn unknown_instruction() {
        let e = ParseInstructionsError::UnknownInstruction { name: "foo".into() };
        let msg = format_parse_instructions_error(&e);
        assert!(msg.contains("Unknown instruction 'foo'"));
    }

    #[test]
    fn wrong_number_args() {
        let e = ParseInstructionsError::WrongNumberOfArguments {
            name: "load",
            expected: &[1, 2],
            actual: 3,
        };
        let msg = format_parse_instructions_error(&e);
        assert!(msg.contains("3"));
        assert!(msg.contains("1 or 2"));
    }

    #[test]
    fn wrong_type_args() {
        let expected: &[&[&str]] = &[&["position", "id"], &["id"]];
        let e = ParseInstructionsError::WrongTypeOfArgument {
            name: "load",
            expected,
        };
        let msg = format_parse_instructions_error(&e);
        assert!(msg.contains("position, id"));
    }

    #[test]
    fn missing_time() {
        let e = ParseInstructionsError::MissingTime {
            name: vec!["load", "store"],
        };
        let msg = format_parse_instructions_error(&e);
        assert!(msg.contains("load, store"));
    }

    #[test]
    fn superfluous_time() {
        let e = ParseInstructionsError::SuperfluousTime { name: vec!["atom"] };
        let msg = format_parse_instructions_error(&e);
        assert!(msg.contains("Superfluous time"));
    }
}
