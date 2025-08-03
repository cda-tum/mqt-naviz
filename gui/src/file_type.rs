/// The available FileTypes that can be opened
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileType {
    Instructions,
    Machine,
    Style,
}

/// Something which can be used to filter files by extension
pub trait FileFilter {
    /// The name of this filter
    fn name(&self) -> &str;
    /// Allowed extensions
    fn extensions(&self) -> &[&str];
}

impl FileFilter for FileType {
    fn name(&self) -> &'static str {
        match self {
            FileType::Instructions => "NAViz instructions",
            FileType::Machine => "NAViz machine",
            FileType::Style => "NAViz style",
        }
    }
    fn extensions(&self) -> &'static [&'static str] {
        match self {
            FileType::Instructions => &["naviz"],
            FileType::Machine => &["namachine"],
            FileType::Style => &["nastyle"],
        }
    }
}
