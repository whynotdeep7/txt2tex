//! Escaping of user text for LaTeX.
//!
//! Everything in a resume is attacker-free but not LaTeX-free: names contain
//! `&`, GitHub handles contain `_`, and salary figures contain `%`. Anything
//! that reaches the `.tex` file goes through here first.

/// Escapes `input` for use as LaTeX body text.
///
/// Single pass on purpose. A chain of `str::replace` calls re-escapes the
/// braces its own replacements introduce, so `\` would come out as
/// `\textbackslash\{\}` and typeset as a literal `\{}`.
pub fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str(r"\textbackslash{}"),
            '^' => out.push_str(r"\textasciicircum{}"),
            '~' => out.push_str(r"\textasciitilde{}"),
            '{' => out.push_str(r"\{"),
            '}' => out.push_str(r"\}"),
            '_' => out.push_str(r"\_"),
            '&' => out.push_str(r"\&"),
            '%' => out.push_str(r"\%"),
            '$' => out.push_str(r"\$"),
            '#' => out.push_str(r"\#"),
            _ => out.push(ch),
        }
    }
    out
}

/// Escapes `input` for the URL argument of `\href`.
///
/// hyperref reads that argument almost verbatim, so the body rules would
/// corrupt the link: `~` and `_` are legal URL characters and must survive as
/// themselves. Only `%` and `#`, which TeX acts on before hyperref sees them,
/// need escaping.
pub fn escape_url(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '%' => out.push_str(r"\%"),
            '#' => out.push_str(r"\#"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_each_special_character() {
        assert_eq!(escape("a&b"), r"a\&b");
        assert_eq!(escape("100%"), r"100\%");
        assert_eq!(escape("a_b"), r"a\_b");
        assert_eq!(escape("C#"), r"C\#");
        assert_eq!(escape("$5"), r"\$5");
    }

    /// The regression this module exists for: the old `replace` chain turned a
    /// backslash into `\textbackslash\{\}`.
    #[test]
    fn does_not_re_escape_its_own_replacements() {
        assert_eq!(escape(r"a\b"), r"a\textbackslash{}b");
        assert_eq!(escape("a^b"), r"a\textasciicircum{}b");
        assert_eq!(escape("a~b"), r"a\textasciitilde{}b");
    }

    #[test]
    fn leaves_ordinary_text_alone() {
        assert_eq!(escape("Alex Johnson"), "Alex Johnson");
        assert_eq!(escape("Zürich — 2024"), "Zürich — 2024");
    }

    /// A URL's `~` and `_` are meaningful; escaping them breaks the link.
    #[test]
    fn url_keeps_legal_url_characters() {
        assert_eq!(escape_url("github.com/user_name"), "github.com/user_name");
        assert_eq!(escape_url("example.com/~alex"), "example.com/~alex");
        assert_eq!(escape_url("example.com/a#b"), r"example.com/a\#b");
    }
}
