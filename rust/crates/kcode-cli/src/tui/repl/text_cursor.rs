pub fn clamp_cursor_to_boundary(text: &str, cursor: usize) -> usize {
    let mut cursor = cursor.min(text.len());
    while cursor > 0 && !text.is_char_boundary(cursor) {
        cursor -= 1;
    }
    cursor
}

pub fn previous_char_boundary(text: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor_to_boundary(text, cursor);
    text[..cursor]
        .char_indices()
        .last()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

pub fn next_char_boundary(text: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor_to_boundary(text, cursor);
    if cursor >= text.len() {
        return text.len();
    }

    text[cursor..]
        .chars()
        .next()
        .map(|character| cursor + character.len_utf8())
        .unwrap_or(cursor)
}
