use std::path::Path;

use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

use noema_core::error::{NoemaError, Result};

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl Highlighter {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    pub fn highlight(&self, path: &Path, content: &str) -> Result<String> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let syntax = self.syntax_set
            .find_syntax_by_extension(ext)
            .or_else(|| self.syntax_set.find_syntax_by_first_line(content))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = &self.theme_set.themes["base16-ocean.dark"];

        highlighted_html_for_string(content, &self.syntax_set, syntax, theme)
            .map_err(|e| NoemaError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))
    }
}
