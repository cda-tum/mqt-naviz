// Shared position utilities.
//
// Provides conversion from byte offsets into (line, column) coordinates
// using 1-based indexing for both line and column. Newlines are detected
// solely via '\n'; a preceding '\r' (Windows line endings) is treated as a
// normal character. This matches how the lexer counts lines.
//
// Public so downstream crates (e.g., GUI, repository) can present
// consistent location information without re-implementing logic.

/// Convert a byte offset into (line, column) 1-based coordinates.
///
/// Behaviour:
/// - The first line and column are 1.
/// - Only the '\n' character advances the line counter and resets the column.
/// - Any other character (including '\r' and '\t') advances the column by 1.
/// - Offsets past the end of the string yield the position just after the
///   final character (i.e., end-of-line column + 1).
pub fn byte_offset_to_line_column(text: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut column = 1usize;
    // Iterate over character start byte indices.
    for (i, ch) in text.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}
