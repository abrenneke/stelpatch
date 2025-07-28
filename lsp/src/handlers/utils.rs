use lasso::Spur;
use path_slash::PathExt;
use std::path::Path;
use tower_lsp::lsp_types::Position;
use tower_lsp::{Client, lsp_types::MessageType};
use url::Url;

use crate::interner::get_interner;

/// Log a message synchronously by using block_in_place
pub fn log_message_sync(_client: &Client, message_type: MessageType, message: String) {
    eprintln!("{:?}: {}", message_type, message);
}

/// Convert LSP position to byte offset in the document
pub fn position_to_offset(text: &str, position: Position) -> usize {
    let target_line = position.line as usize;
    let target_char = position.character as usize;

    // Split text into lines (this removes line endings)
    let lines: Vec<&str> = text.lines().collect();

    if target_line >= lines.len() {
        return text.len();
    }

    let mut offset = 0;

    // Add up all complete lines before target line
    for i in 0..target_line {
        offset += lines[i].len();

        // Add line ending bytes - check what type of line ending this line actually has
        // We need to look at the original text to see the actual line ending
        let line_end_pos = offset;
        if line_end_pos < text.len() {
            let remaining = &text[line_end_pos..];
            if remaining.starts_with("\r\n") {
                offset += 2; // Windows line ending
            } else if remaining.starts_with('\n') {
                offset += 1; // Unix line ending
            }
            // Note: we don't handle lone \r (old Mac) as it's very rare
        }
    }

    // Now add the character offset within the target line
    let target_line_text = lines[target_line];
    let char_offset = target_line_text
        .char_indices()
        .nth(target_char)
        .map(|(i, _)| i)
        .unwrap_or(target_line_text.len());

    offset + char_offset
}

/// Extract namespace from a file URI relative to a root directory
/// Examples:
/// file:///some/path/stellaris/common/buildings/01_buildings.txt, root_dir="/some/path/stellaris" -> Some("game/common/buildings")
/// file:///some/path/stellaris/events/events.txt, root_dir="/some/path/stellaris" -> Some("game/events")
/// file:///some/path/stellaris/interface/main.gui, root_dir="/some/path/stellaris" -> Some("game/interface")
pub fn extract_namespace_from_uri(uri: &str, root_dir: &Path) -> Option<String> {
    let file_path = Url::parse(uri).ok()?.to_file_path().ok()?;

    // Calculate the relative path from root_dir to the file
    let relative_path = match file_path.strip_prefix(root_dir) {
        Ok(rel_path) => rel_path,
        Err(_) => {
            // If stripping fails, return None
            return None;
        }
    };

    // Get the parent directory of the file as the namespace, prefixed with "game"
    let namespace = if let Some(parent_dir) = relative_path.parent() {
        if parent_dir.as_os_str().is_empty() {
            // File is directly in the root directory
            "game".to_string()
        } else {
            // Prepend "game/" to the parent directory path, converting to forward slashes
            format!("game/{}", parent_dir.to_slash_lossy())
        }
    } else {
        // This shouldn't happen, but provide a fallback
        "game".to_string()
    };

    Some(namespace)
}

pub fn contains_scripted_argument(identifier: Spur) -> bool {
    let interner = get_interner();
    let identifier = interner.resolve(&identifier);
    if let Some(first_dollar_index) = identifier.find('$') {
        if let Some(last_dollar_index) = identifier.rfind('$') {
            if first_dollar_index != last_dollar_index {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_namespace_from_uri() {
        use std::path::Path;

        // Test common directory with subdirectories
        assert_eq!(
            extract_namespace_from_uri(
                "file:///C:/Stellaris/common/buildings/01_buildings.txt",
                Path::new("C:/Stellaris")
            ),
            Some("game/common/buildings".to_string())
        );

        assert_eq!(
            extract_namespace_from_uri(
                "file:///home/user/stellaris/common/species_classes/test.txt",
                Path::new("/home/user/stellaris")
            ),
            Some("game/common/species_classes".to_string())
        );

        assert_eq!(
            extract_namespace_from_uri(
                "file:///path/to/common/armies/armies.txt",
                Path::new("/path/to")
            ),
            Some("game/common/armies".to_string())
        );

        // Test case with multiple "common/" occurrences (like Steam path)
        assert_eq!(
            extract_namespace_from_uri(
                "file:///d%3A/SteamLibrary/steamapps/common/Stellaris/common/ai_budget/00_astral_threads_budget.txt",
                Path::new("d:/SteamLibrary/steamapps/common/Stellaris")
            ),
            Some("game/common/ai_budget".to_string())
        );

        // Test case with nested subdirectories in common
        assert_eq!(
            extract_namespace_from_uri(
                "file:///D:/SteamLibrary/steamapps/common/Stellaris/common/governments/civics/00_civics.txt",
                Path::new("D:/SteamLibrary/steamapps/common/Stellaris")
            ),
            Some("game/common/governments/civics".to_string())
        );

        // Test non-common directories
        assert_eq!(
            extract_namespace_from_uri("file:///path/to/events/events.txt", Path::new("/path/to")),
            Some("game/events".to_string())
        );

        assert_eq!(
            extract_namespace_from_uri("file:///path/to/interface/main.gui", Path::new("/path/to")),
            Some("game/interface".to_string())
        );

        assert_eq!(
            extract_namespace_from_uri(
                "file:///path/to/localisation/english/l_english.yml",
                Path::new("/path/to")
            ),
            Some("game/localisation/english".to_string())
        );

        assert_eq!(
            extract_namespace_from_uri(
                "file:///path/to/gfx/portraits/species.gfx",
                Path::new("/path/to")
            ),
            Some("game/gfx/portraits".to_string())
        );

        assert_eq!(
            extract_namespace_from_uri("file:///path/to/map/galaxy.txt", Path::new("/path/to")),
            Some("game/map".to_string())
        );

        // Test subdirectories in other directories
        assert_eq!(
            extract_namespace_from_uri(
                "file:///path/to/interface/game_setup/main.gui",
                Path::new("/path/to")
            ),
            Some("game/interface/game_setup".to_string())
        );

        assert_eq!(
            extract_namespace_from_uri(
                "file:///path/to/gfx/portraits/species/humanoid.gfx",
                Path::new("/path/to")
            ),
            Some("game/gfx/portraits/species".to_string())
        );

        // Test files that don't match the root directory (should return None)
        assert_eq!(
            extract_namespace_from_uri(
                "file:///path/to/unknown/file.txt",
                Path::new("/different/root")
            ),
            None
        );

        assert_eq!(
            extract_namespace_from_uri(
                "file:///path/to/random/folder/file.txt",
                Path::new("/other/path")
            ),
            None
        );
    }
}
