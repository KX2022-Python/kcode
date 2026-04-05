use unicode_width::UnicodeWidthChar;

pub(crate) fn wrap_display_text(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut wrapped = Vec::new();

    for raw_line in text.lines() {
        if raw_line.is_empty() {
            wrapped.push(String::new());
            continue;
        }

        let mut current = String::new();
        let mut current_width = 0;

        for ch in raw_line.chars() {
            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
            let would_overflow =
                current_width > 0 && ch_width > 0 && current_width + ch_width > width;

            if would_overflow {
                wrapped.push(std::mem::take(&mut current));
                current_width = 0;
            }

            current.push(ch);
            current_width += ch_width;

            if current_width >= width {
                wrapped.push(std::mem::take(&mut current));
                current_width = 0;
            }
        }

        if current.is_empty() {
            if wrapped.is_empty() {
                wrapped.push(String::new());
            }
        } else {
            wrapped.push(current);
        }
    }

    if wrapped.is_empty() {
        wrapped.push(String::new());
    }

    wrapped
}

pub(crate) fn display_line_count(text: &str, width: usize) -> usize {
    wrap_display_text(text, width).len().max(1)
}

#[cfg(test)]
mod tests {
    use super::{display_line_count, wrap_display_text};

    #[test]
    fn wraps_ascii_by_display_width() {
        assert_eq!(
            wrap_display_text("abcdefgh", 3),
            vec!["abc".to_string(), "def".to_string(), "gh".to_string()]
        );
    }

    #[test]
    fn wraps_cjk_using_terminal_cell_width() {
        assert_eq!(
            wrap_display_text("你好世界", 4),
            vec!["你好".to_string(), "世界".to_string()]
        );
        assert_eq!(display_line_count("你好世界", 4), 2);
    }
}
