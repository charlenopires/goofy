//! Fuzzy matching algorithms for flexible completion search

use std::cmp::{max, min};

/// Calculate fuzzy match score between needle and haystack
/// Returns a score between 0.0 and 1.0, where 1.0 is a perfect match
pub fn fuzzy_score(haystack: &str, needle: &str) -> f64 {
    if needle.is_empty() {
        return 1.0;
    }
    
    if haystack.is_empty() {
        return 0.0;
    }

    // Case-insensitive matching
    let haystack = haystack.to_lowercase();
    let needle = needle.to_lowercase();

    // Exact match gets highest score
    if haystack == needle {
        return 1.0;
    }

    // Prefix match gets high score
    if haystack.starts_with(&needle) {
        return 0.9 + (needle.len() as f64 / haystack.len() as f64) * 0.1;
    }

    // Substring match gets good score
    if haystack.contains(&needle) {
        let start_pos = haystack.find(&needle).unwrap() as f64;
        let position_score = 1.0 - (start_pos / haystack.len() as f64) * 0.3;
        let length_score = needle.len() as f64 / haystack.len() as f64;
        return 0.7 * position_score + 0.3 * length_score;
    }

    // Fuzzy character matching
    let score = fuzzy_match_characters(&haystack, &needle);
    if score > 0.0 {
        return min_f64(score, 0.6); // Cap fuzzy matches at 0.6
    }

    0.0
}

/// Check if needle fuzzy matches haystack
pub fn fuzzy_match(haystack: &str, needle: &str) -> bool {
    fuzzy_score(haystack, needle) > 0.0
}

/// Calculate character-by-character fuzzy match score
fn fuzzy_match_characters(haystack: &str, needle: &str) -> f64 {
    if needle.is_empty() {
        return 1.0;
    }

    let haystack_chars: Vec<char> = haystack.chars().collect();
    let needle_chars: Vec<char> = needle.chars().collect();
    
    let matches = count_matching_characters(&haystack_chars, &needle_chars);
    let max_possible = needle_chars.len() as f64;
    
    if matches == 0.0 {
        return 0.0;
    }

    // Base score from character matches
    let base_score = matches / max_possible;
    
    // Bonus for sequential matches
    let sequential_bonus = calculate_sequential_bonus(&haystack_chars, &needle_chars);
    
    // Penalty for length difference
    let length_penalty = calculate_length_penalty(haystack_chars.len(), needle_chars.len());
    
    (base_score + sequential_bonus - length_penalty).max(0.0)
}

/// Count matching characters between haystack and needle
fn count_matching_characters(haystack: &[char], needle: &[char]) -> f64 {
    let mut matches = 0.0;
    let mut haystack_idx = 0;
    
    for &needle_char in needle {
        while haystack_idx < haystack.len() {
            if haystack[haystack_idx] == needle_char {
                matches += 1.0;
                haystack_idx += 1;
                break;
            }
            haystack_idx += 1;
        }
    }
    
    matches
}

/// Calculate bonus for sequential character matches
fn calculate_sequential_bonus(haystack: &[char], needle: &[char]) -> f64 {
    let mut bonus = 0.0;
    let mut haystack_idx = 0;
    let mut last_match_idx = None;
    
    for &needle_char in needle {
        while haystack_idx < haystack.len() {
            if haystack[haystack_idx] == needle_char {
                if let Some(last_idx) = last_match_idx {
                    if haystack_idx == last_idx + 1 {
                        bonus += 0.1; // Sequential match bonus
                    }
                }
                last_match_idx = Some(haystack_idx);
                haystack_idx += 1;
                break;
            }
            haystack_idx += 1;
        }
    }
    
    bonus
}

/// Calculate penalty for length difference
fn calculate_length_penalty(haystack_len: usize, needle_len: usize) -> f64 {
    if haystack_len <= needle_len {
        return 0.0;
    }
    
    let diff = haystack_len - needle_len;
    let penalty_rate = 0.05; // 5% penalty per extra character
    (diff as f64 * penalty_rate).min(0.3) // Cap penalty at 30%
}

/// Camel case matching for identifiers
pub fn camel_case_score(haystack: &str, needle: &str) -> f64 {
    if needle.is_empty() {
        return 1.0;
    }
    
    let camel_chars = extract_camel_case_chars(haystack);
    let needle_lower = needle.to_lowercase();
    
    // Try to match against camel case characters
    let camel_string: String = camel_chars.iter().collect::<String>().to_lowercase();
    
    if camel_string.starts_with(&needle_lower) {
        return 0.8 + (needle_lower.len() as f64 / camel_string.len() as f64) * 0.2;
    }
    
    // Fuzzy match against camel case chars
    fuzzy_score(&camel_string, &needle_lower) * 0.6
}

/// Extract camel case characters from a string
fn extract_camel_case_chars(text: &str) -> Vec<char> {
    let mut chars = Vec::new();
    let chars_vec: Vec<char> = text.chars().collect();
    let mut after_boundary = true; // Start of string is a boundary

    for (i, &ch) in chars_vec.iter().enumerate() {
        if ch == '_' || ch == '-' || ch == '.' {
            // Separator is a word boundary; next char starts a new word
            after_boundary = true;
        } else if ch.is_uppercase() {
            // Uppercase always starts a word in camelCase
            chars.push(ch);
            after_boundary = false;
        } else if after_boundary && ch.is_alphabetic() {
            // First alphabetic char after a boundary
            chars.push(ch);
            after_boundary = false;
        } else {
            after_boundary = false;
        }
    }

    chars
}

/// Advanced fuzzy scoring with multiple strategies
pub fn advanced_fuzzy_score(haystack: &str, needle: &str) -> f64 {
    if needle.is_empty() {
        return 1.0;
    }
    
    // Try different matching strategies and take the best score
    let scores = vec![
        fuzzy_score(haystack, needle),
        camel_case_score(haystack, needle),
        acronym_score(haystack, needle),
        word_boundary_score(haystack, needle),
    ];
    
    scores.into_iter().fold(0.0, |acc, score| max_f64(acc, score))
}

/// Score based on acronym matching (first letters of words)
pub fn acronym_score(haystack: &str, needle: &str) -> f64 {
    let words: Vec<&str> = haystack.split_whitespace().collect();
    if words.is_empty() {
        return 0.0;
    }
    
    let acronym: String = words
        .iter()
        .filter_map(|word| word.chars().next())
        .collect::<String>()
        .to_lowercase();
    
    if acronym.starts_with(&needle.to_lowercase()) {
        return 0.7 + (needle.len() as f64 / acronym.len() as f64) * 0.3;
    }
    
    0.0
}

/// Score based on word boundary matching
pub fn word_boundary_score(haystack: &str, needle: &str) -> f64 {
    let needle_lower = needle.to_lowercase();
    let haystack_lower = haystack.to_lowercase();
    
    // Split on common word boundaries
    let words: Vec<&str> = haystack_lower
        .split(|c: char| c.is_whitespace() || c == '_' || c == '-' || c == '.')
        .filter(|s| !s.is_empty())
        .collect();
    
    for word in &words {
        if word.starts_with(&needle_lower) {
            let score = 0.6 + (needle_lower.len() as f64 / word.len() as f64) * 0.4;
            return score;
        }
    }
    
    // Check if any word contains the needle
    for word in &words {
        if word.contains(&needle_lower) {
            return 0.4;
        }
    }
    
    0.0
}

/// Rank completions by fuzzy score
pub fn rank_completions<T>(
    items: Vec<T>,
    needle: &str,
    extract_text: impl Fn(&T) -> &str,
) -> Vec<(T, f64)> {
    let mut scored_items: Vec<(T, f64)> = items
        .into_iter()
        .map(|item| {
            let text = extract_text(&item);
            let score = advanced_fuzzy_score(text, needle);
            (item, score)
        })
        .filter(|(_, score)| *score > 0.0)
        .collect();
    
    // Sort by score (highest first)
    scored_items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    
    scored_items
}

/// Helper function for f64 min
fn min_f64(a: f64, b: f64) -> f64 {
    if a < b { a } else { b }
}

/// Helper function for f64 max
fn max_f64(a: f64, b: f64) -> f64 {
    if a > b { a } else { b }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        assert_eq!(fuzzy_score("hello", "hello"), 1.0);
        assert_eq!(fuzzy_score("test", "test"), 1.0);
    }

    #[test]
    fn test_prefix_match() {
        let score = fuzzy_score("hello_world", "hello");
        assert!(score > 0.9 && score < 1.0);
        
        let score = fuzzy_score("test_file", "test");
        assert!(score > 0.9);
    }

    #[test]
    fn test_substring_match() {
        let score = fuzzy_score("hello_world", "world");
        assert!(score > 0.5 && score < 0.9);
        
        let score = fuzzy_score("test_file_name", "file");
        assert!(score > 0.5);
    }

    #[test]
    fn test_fuzzy_match() {
        assert!(fuzzy_match("hello_world", "hlw"));
        assert!(fuzzy_match("test_file", "tf"));
        assert!(fuzzy_match("CompletionProvider", "CP"));
        assert!(!fuzzy_match("hello", "xyz"));
    }

    #[test]
    fn test_camel_case_matching() {
        let score = camel_case_score("CompletionProvider", "CP");
        assert!(score > 0.6);
        
        let score = camel_case_score("getUserName", "gun");
        assert!(score > 0.5);
        
        let score = camel_case_score("FileCompletionProvider", "FCP");
        assert!(score > 0.7);
    }

    #[test]
    fn test_acronym_matching() {
        let score = acronym_score("File Completion Provider", "fcp");
        assert!(score > 0.7);
        
        let score = acronym_score("Advanced Search System", "ass");
        assert!(score > 0.7);
        
        let score = acronym_score("hello world", "hw");
        assert!(score > 0.7);
    }

    #[test]
    fn test_word_boundary_matching() {
        let score = word_boundary_score("file_completion_provider", "comp");
        assert!(score > 0.5);
        
        let score = word_boundary_score("test-file-name", "file");
        assert!(score > 0.5);
        
        let score = word_boundary_score("my.config.file", "config");
        assert!(score > 0.5);
    }

    #[test]
    fn test_advanced_fuzzy_score() {
        // Should pick the best score from all strategies
        let score = advanced_fuzzy_score("CompletionProvider", "CP");
        assert!(score > 0.7);
        
        let score = advanced_fuzzy_score("file_completion_provider", "fcp");
        assert!(score > 0.6);
        
        let score = advanced_fuzzy_score("getUserData", "gud");
        assert!(score > 0.5);
    }

    #[test]
    fn test_rank_completions() {
        let items = vec!["hello_world", "help_text", "application", "hello"];
        let ranked = rank_completions(items, "hel", |s| s);
        
        assert!(!ranked.is_empty());
        // "hello" should rank higher than "hello_world" due to shorter length
        assert!(ranked[0].1 >= ranked[1].1);
    }

    #[test]
    fn test_empty_needle() {
        assert_eq!(fuzzy_score("anything", ""), 1.0);
        assert_eq!(camel_case_score("anything", ""), 1.0);
        assert_eq!(advanced_fuzzy_score("anything", ""), 1.0);
    }

    #[test]
    fn test_empty_haystack() {
        assert_eq!(fuzzy_score("", "needle"), 0.0);
        assert_eq!(camel_case_score("", "needle"), 0.0);
        assert_eq!(advanced_fuzzy_score("", "needle"), 0.0);
    }
}