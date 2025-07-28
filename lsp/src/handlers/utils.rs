use lasso::Spur;
use tower_lsp::lsp_types::Position;
use tower_lsp::{Client, lsp_types::MessageType};

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

/// Extract namespace from a file URI
/// Examples:
/// file:///path/to/common/buildings/01_buildings.txt -> Some("common/buildings")
/// file:///path/to/common/species_classes/species_classes.txt -> Some("common/species_classes")
/// file:///path/to/events/events.txt -> Some("events")
/// file:///path/to/interface/main.gui -> Some("interface")
pub fn extract_namespace_from_uri(uri: &str) -> Option<String> {
    let uri = uri.replace("file://", "").replace("\\", "/");

    // Parse the URI and extract the path
    let path = if uri.starts_with("file://") {
        // Remove file:// prefix and handle Windows/Unix paths
        let path_part = &uri[7..];
        if path_part.starts_with('/')
            && path_part.len() > 1
            && path_part.chars().nth(2) == Some(':')
        {
            // Windows path like /C:/path/to/file
            &path_part[1..]
        } else {
            path_part
        }
    } else {
        &uri
    };

    // Split the path into segments
    let segments: Vec<&str> = path.split('/').collect();

    // Look for known Stellaris directory patterns
    let known_directories = [
        "common",
        "events",
        "interface",
        "localisation",
        "gfx",
        "sound",
        "music",
        "flags",
        "map",
        "prescripted_countries",
        "fonts",
        "dlc_list",
        "effects",
        "enums",
        "ethics",
        "folders",
        "links",
        "modifier_categories",
        "modifier_rule",
        "modifier",
        "pre_triggers",
        "scope_changes",
        "scopes",
        "triggers",
    ];

    // Find the last occurrence of a known directory
    for i in (0..segments.len()).rev() {
        if known_directories.contains(&segments[i]) {
            let mut namespace_parts = vec![segments[i]];

            // Include all subdirectories until we reach a file (segment with extension)
            let mut j = i + 1;
            while j < segments.len() {
                let segment = segments[j];
                // If this segment contains a dot, it's likely a file, so stop
                if segment.contains('.') {
                    break;
                }
                namespace_parts.push(segment);
                j += 1;
            }

            return Some(namespace_parts.join("/"));
        }
    }

    None
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
        // Test common directory with subdirectories
        assert_eq!(
            extract_namespace_from_uri("file:///C:/Stellaris/common/buildings/01_buildings.txt"),
            Some("common/buildings".to_string())
        );

        assert_eq!(
            extract_namespace_from_uri(
                "file:///home/user/stellaris/common/species_classes/test.txt"
            ),
            Some("common/species_classes".to_string())
        );

        assert_eq!(
            extract_namespace_from_uri("file:///path/to/common/armies/armies.txt"),
            Some("common/armies".to_string())
        );

        // Test case with multiple "common/" occurrences (like Steam path)
        assert_eq!(
            extract_namespace_from_uri(
                "file:///d%3A/SteamLibrary/steamapps/common/Stellaris/common/ai_budget/00_astral_threads_budget.txt"
            ),
            Some("common/ai_budget".to_string())
        );

        // Test case with nested subdirectories in common
        assert_eq!(
            extract_namespace_from_uri(
                "file:///D:/SteamLibrary/steamapps/common/Stellaris/common/governments/civics/00_civics.txt"
            ),
            Some("common/governments/civics".to_string())
        );

        // Test non-common directories
        assert_eq!(
            extract_namespace_from_uri("file:///path/to/events/events.txt"),
            Some("events".to_string())
        );

        assert_eq!(
            extract_namespace_from_uri("file:///path/to/interface/main.gui"),
            Some("interface".to_string())
        );

        assert_eq!(
            extract_namespace_from_uri("file:///path/to/localisation/english/l_english.yml"),
            Some("localisation/english".to_string())
        );

        assert_eq!(
            extract_namespace_from_uri("file:///path/to/gfx/portraits/species.gfx"),
            Some("gfx/portraits".to_string())
        );

        assert_eq!(
            extract_namespace_from_uri("file:///path/to/map/galaxy.txt"),
            Some("map".to_string())
        );

        // Test subdirectories in other directories
        assert_eq!(
            extract_namespace_from_uri("file:///path/to/interface/game_setup/main.gui"),
            Some("interface/game_setup".to_string())
        );

        assert_eq!(
            extract_namespace_from_uri("file:///path/to/gfx/portraits/species/humanoid.gfx"),
            Some("gfx/portraits/species".to_string())
        );

        // Test files that don't match known directories
        assert_eq!(
            extract_namespace_from_uri("file:///path/to/unknown/file.txt"),
            None
        );

        assert_eq!(
            extract_namespace_from_uri("file:///path/to/random/folder/file.txt"),
            None
        );
    }
}
