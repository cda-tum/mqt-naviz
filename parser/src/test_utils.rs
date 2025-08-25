// Test support utilities (only compiled during tests)
// Shared helpers to reduce duplication across lexer/parser tests.

// Re-export public utility for convenience in tests.
pub use crate::common::position::byte_offset_to_line_column;

/// Collect stringified context frames from a winnow ParseError.
pub fn collect_context<I>(
    err: winnow::error::ParseError<I, winnow::error::ContextError>,
) -> Vec<String> {
    err.into_inner().context().map(|c| c.to_string()).collect()
}
