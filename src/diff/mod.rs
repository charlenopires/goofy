//! Diff generation utilities
//!
//! This module provides utilities for generating unified diffs between files.

use similar::{ChangeTag, TextDiff};
use std::fmt::Write;

/// Generate a unified diff from two file contents
///
/// Returns a tuple of (diff_string, additions_count, removals_count)
pub fn generate_diff(
    before_content: &str,
    after_content: &str,
    file_name: &str,
) -> (String, usize, usize) {
    let file_name = file_name.trim_start_matches('/');
    
    let diff = TextDiff::from_lines(before_content, after_content);
    
    let mut output = String::new();
    let mut additions = 0;
    let mut removals = 0;
    
    // Write diff header
    writeln!(&mut output, "--- a/{}", file_name).unwrap();
    writeln!(&mut output, "+++ b/{}", file_name).unwrap();
    
    // Generate unified diff with 3 lines of context
    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        writeln!(&mut output, "{}", hunk.header()).unwrap();
        
        for change in hunk.iter_changes() {
            let sign = match change.tag() {
                ChangeTag::Delete => {
                    removals += 1;
                    "-"
                }
                ChangeTag::Insert => {
                    additions += 1;
                    "+"
                }
                ChangeTag::Equal => " ",
            };
            
            write!(&mut output, "{}", sign).unwrap();
            write!(&mut output, "{}", change.value()).unwrap();
            
            // Add newline if the change doesn't end with one
            if !change.value().ends_with('\n') {
                writeln!(&mut output).unwrap();
            }
        }
    }
    
    (output, additions, removals)
}

/// Apply a patch to content
pub fn apply_patch(original: &str, patch: &str) -> Result<String, String> {
    // This is a simplified version - in production you'd use a proper patch library
    // For now, we'll return an error indicating this needs implementation
    Err("Patch application not yet implemented".to_string())
}

/// Check if two strings are different
pub fn is_different(a: &str, b: &str) -> bool {
    a != b
}

/// Calculate similarity ratio between two strings (0.0 to 1.0)
pub fn similarity_ratio(a: &str, b: &str) -> f64 {
    if a == b {
        return 1.0;
    }

    // Use character-level comparison for better accuracy with short strings
    let diff = TextDiff::from_chars(a, b);
    let total_chars = a.len().max(b.len()) as f64;

    if total_chars == 0.0 {
        return 1.0;
    }

    let equal_chars = diff.iter_all_changes()
        .filter(|c| c.tag() == ChangeTag::Equal)
        .map(|c| c.value().len())
        .sum::<usize>() as f64;

    equal_chars / total_chars
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_diff() {
        let before = "line1\nline2\nline3\n";
        let after = "line1\nmodified\nline3\nline4\n";
        
        let (diff, additions, removals) = generate_diff(before, after, "test.txt");
        
        assert!(diff.contains("--- a/test.txt"));
        assert!(diff.contains("+++ b/test.txt"));
        assert!(diff.contains("-line2"));
        assert!(diff.contains("+modified"));
        assert!(diff.contains("+line4"));
        assert_eq!(additions, 2);
        assert_eq!(removals, 1);
    }
    
    #[test]
    fn test_is_different() {
        assert!(is_different("hello", "world"));
        assert!(!is_different("same", "same"));
    }
    
    #[test]
    fn test_similarity_ratio() {
        assert_eq!(similarity_ratio("same", "same"), 1.0);
        assert!(similarity_ratio("hello", "world") < 1.0);
        assert!(similarity_ratio("hello", "world") > 0.0);
    }
}