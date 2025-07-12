use tower_lsp::lsp_types::Position;

/// Convert LSP position to byte offset in the document
pub fn position_to_offset(text: &str, position: Position) -> usize {
    let lines: Vec<&str> = text.lines().collect();
    let mut offset = 0;

    for (line_idx, line) in lines.iter().enumerate() {
        if line_idx < position.line as usize {
            offset += line.len() + 1; // +1 for newline character
        } else {
            offset += position.character as usize;
            break;
        }
    }

    let final_offset = offset.min(text.len());
    final_offset
}

/// Extract namespace from a file URI
/// Examples:
/// file:///path/to/common/buildings/01_buildings.txt -> Some("common/buildings")
/// file:///path/to/common/species_classes/species_classes.txt -> Some("common/species_classes")
/// file:///path/to/events/events.txt -> None (not a namespace file)
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

    // Look for "common/" in the path, but we want the last occurrence
    // to handle paths like "SteamLibrary/steamapps/common/Stellaris/common/ai_budget/file.txt"
    if let Some(common_pos) = path.rfind("common/") {
        let after_common = &path[common_pos..];
        let parts: Vec<&str> = after_common.split('/').collect();

        // Should have at least ["common", "namespace", "filename.txt"]
        if parts.len() >= 3 && parts[0] == "common" {
            return Some(format!("common/{}", parts[1]));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_namespace_from_uri() {
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
            extract_namespace_from_uri("file:///path/to/events/events.txt"),
            None
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
    }
}
