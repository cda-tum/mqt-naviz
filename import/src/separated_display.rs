use std::fmt::Display;

/// Implements [Display] for a slice by separating the entries with a separator.
pub struct SeparatedDisplay<'a, T: Display>(pub &'a str, pub &'a [T]);

impl<T: Display> Display for SeparatedDisplay<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iterator = self.1.iter();
        if let Some(value) = iterator.next() {
            write!(f, "{}", value)?;
        }
        for value in iterator {
            write!(f, "{}{}", self.0, value)?;
        }
        Ok(())
    }
}

impl<'a, T: Display> SeparatedDisplay<'a, T> {
    // Separates entries with `", "`
    pub fn comma(value: &'a [T]) -> Self {
        Self(", ", value)
    }

    // Separates entries with `"\n"`
    pub fn newline(value: &'a [T]) -> Self {
        Self("\n", value)
    }
}
