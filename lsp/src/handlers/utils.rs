use tower_lsp::lsp_types::Position;

/// Convert LSP position to byte offset in the document
pub fn position_to_offset(content: &str, position: Position) -> usize {
    let mut offset = 0;
    let mut current_line = 0;
    let mut current_col = 0;

    for ch in content.chars() {
        if current_line == position.line as usize && current_col == position.character as usize {
            return offset;
        }

        if ch == '\n' {
            current_line += 1;
            current_col = 0;
        } else {
            current_col += 1;
        }

        offset += ch.len_utf8();
    }

    offset
}
