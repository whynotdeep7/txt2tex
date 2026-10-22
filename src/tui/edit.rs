//! A one-line text editor.
//!
//! ratatui ships no text input, so this is hand-rolled — but the resume model
//! is entirely single-line strings, which means one line is the whole job. No
//! buffer, no wrapping, no selection.

/// Text plus a cursor.
pub struct LineEditor {
    text: String,
    /// A *character* index, not a byte offset.
    ///
    /// Bytes would be a bug waiting to happen: one press of ← next to an
    /// em-dash lands mid-codepoint, and the next insert panics. Character
    /// indices convert to bytes only at the moment of use.
    cursor: usize,
}

impl LineEditor {
    /// Starts with the cursor at the end, ready to keep typing.
    pub fn new(text: String) -> Self {
        let cursor = text.chars().count();
        Self { text, cursor }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn into_text(self) -> String {
        self.text
    }

    fn len(&self) -> usize {
        self.text.chars().count()
    }

    /// The byte offset of character `index`, or the end of the string.
    fn byte_at(&self, index: usize) -> usize {
        self.text
            .char_indices()
            .nth(index)
            .map_or(self.text.len(), |(byte, _)| byte)
    }

    pub fn insert(&mut self, ch: char) {
        let at = self.byte_at(self.cursor);
        self.text.insert(at, ch);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor -= 1;
        let at = self.byte_at(self.cursor);
        self.text.remove(at);
    }

    pub fn delete(&mut self) {
        if self.cursor >= self.len() {
            return;
        }
        let at = self.byte_at(self.cursor);
        self.text.remove(at);
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.len());
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.len();
    }

    /// The text to draw in a field `width` columns wide, split into the part
    /// before the cursor, the character under it, and the part after.
    ///
    /// Long values scroll horizontally: the window slides to keep the cursor
    /// on screen, rather than letting a 200-character description run off the
    /// end of the pane where you cannot see what you are typing.
    pub fn view(&self, width: usize) -> (String, String, String) {
        if width == 0 {
            return (String::new(), String::new(), String::new());
        }

        let chars: Vec<char> = self.text.chars().collect();
        // Keep the cursor inside the window; it sits at the right edge once
        // the text is longer than the field.
        let start = (self.cursor + 1).saturating_sub(width);
        let end = (start + width).min(chars.len());
        let offset = self.cursor - start;

        let visible = &chars[start..end];
        let before: String = visible[..offset.min(visible.len())].iter().collect();
        let under = visible.get(offset).copied();
        let after: String = visible.get(offset + 1..).unwrap_or(&[]).iter().collect();

        // A space stands in for the character under the cursor at end of line,
        // so the block cursor is still visible there.
        (before, under.unwrap_or(' ').to_string(), after)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor(text: &str) -> LineEditor {
        LineEditor::new(text.to_string())
    }

    #[test]
    fn typing_appends_at_the_cursor() {
        let mut e = editor("ab");
        e.insert('c');
        assert_eq!(e.text(), "abc");
        e.home();
        e.insert('z');
        assert_eq!(e.text(), "zabc");
    }

    #[test]
    fn backspace_and_delete_remove_the_right_side() {
        let mut e = editor("abc");
        e.backspace();
        assert_eq!(e.text(), "ab");
        e.home();
        e.delete();
        assert_eq!(e.text(), "b");
    }

    #[test]
    fn editing_at_the_edges_is_a_no_op() {
        let mut e = editor("");
        e.backspace();
        e.delete();
        e.left();
        e.right();
        assert_eq!(e.text(), "");
    }

    /// The whole reason the cursor counts characters: every one of these
    /// panics if the cursor is a byte offset.
    #[test]
    fn multi_byte_characters_survive_editing() {
        let mut e = editor("Zürich — 北京");
        e.backspace();
        assert_eq!(e.text(), "Zürich — 北");

        e.home();
        e.right();
        e.insert('!');
        assert_eq!(e.text(), "Z!ürich — 北");

        e.home();
        e.delete();
        assert_eq!(e.text(), "!ürich — 北");
    }

    #[test]
    fn arrow_keys_stop_at_the_ends() {
        let mut e = editor("ab");
        e.home();
        e.left();
        e.left();
        e.insert('x');
        assert_eq!(e.text(), "xab");

        e.end();
        e.right();
        e.insert('y');
        assert_eq!(e.text(), "xaby");
    }

    #[test]
    fn view_shows_the_cursor_at_end_of_line() {
        let e = editor("ab");
        let (before, under, after) = e.view(20);
        assert_eq!(
            (before.as_str(), under.as_str(), after.as_str()),
            ("ab", " ", "")
        );
    }

    #[test]
    fn view_splits_around_the_character_under_the_cursor() {
        let mut e = editor("abc");
        e.home();
        let (before, under, after) = e.view(20);
        assert_eq!(
            (before.as_str(), under.as_str(), after.as_str()),
            ("", "a", "bc")
        );
    }

    /// Long values must scroll rather than run off the pane.
    #[test]
    fn view_scrolls_to_keep_the_cursor_visible() {
        let e = editor("0123456789");
        let (before, under, after) = e.view(4);
        // Cursor is at the end, so the window shows the tail.
        assert_eq!(before, "789");
        assert_eq!(under, " ");
        assert_eq!(after, "");
    }

    #[test]
    fn view_of_a_narrow_field_does_not_panic() {
        let e = editor("hello");
        assert_eq!(e.view(0), (String::new(), String::new(), String::new()));
        let (_, under, _) = e.view(1);
        assert_eq!(under, " ");
    }
}
