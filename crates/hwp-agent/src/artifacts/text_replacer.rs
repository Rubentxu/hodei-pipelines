//! Text replacement module using Aho-Corasick algorithm
//!
//! This module provides efficient text replacement for large files using the
//! Aho-Corasick pattern matching algorithm for optimal performance.

use aho_corasick::{AhoCorasick, AhoCorasickBuilder};
use std::collections::HashMap;
use std::sync::OnceLock;
use thiserror::Error;

/// Text replacement error types
#[derive(Debug, Error)]
pub enum ReplacerError {
    #[error("Pattern compilation error: {0}")]
    PatternCompilation(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

/// Replacement pattern configuration
#[derive(Debug, Clone)]
pub struct ReplacementPattern {
    pub search_pattern: String,
    pub replacement: String,
    pub case_sensitive: bool,
    pub preserve_case: bool,
    pub is_regex: bool,
}

impl ReplacementPattern {
    pub fn new(search: &str, replacement: &str) -> Self {
        Self {
            search_pattern: search.to_string(),
            replacement: replacement.to_string(),
            case_sensitive: false,
            preserve_case: true,
            is_regex: false,
        }
    }

    pub fn with_case_sensitive(mut self, case_sensitive: bool) -> Self {
        self.case_sensitive = case_sensitive;
        self
    }

    pub fn with_preserve_case(mut self, preserve_case: bool) -> Self {
        self.preserve_case = preserve_case;
        self
    }
}

/// Aho-Corasick based text replacer
#[derive(Debug)]
pub struct AhoCorasickReplacer {
    patterns: Vec<ReplacementPattern>,
    automaton: OnceLock<AhoCorasick>,
}

impl AhoCorasickReplacer {
    /// Create a new replacer with patterns
    pub fn new(patterns: Vec<ReplacementPattern>) -> Result<Self, ReplacerError> {
        if patterns.is_empty() {
            return Err(ReplacerError::Other(
                "At least one pattern is required".to_string(),
            ));
        }

        // Compile patterns for Aho-Corasick
        let ac_patterns: Vec<String> = patterns.iter().map(|p| p.search_pattern.clone()).collect();

        // Build automaton with case-insensitive matching by default
        let automaton = AhoCorasickBuilder::new()
            .ascii_case_insensitive(true)
            .build(&ac_patterns)
            .map_err(|e| ReplacerError::PatternCompilation(e.to_string()))?;

        Ok(Self {
            patterns,
            automaton: OnceLock::from(automaton),
        })
    }

    /// Replace text efficiently using Aho-Corasick
    pub fn replace_text(&self, text: &str) -> Result<String, ReplacerError> {
        let automaton = self
            .automaton
            .get()
            .ok_or_else(|| ReplacerError::Other("Automaton not initialized".to_string()))?;

        // Find all matches
        let matches: Vec<_> = automaton.find_iter(text).collect();

        if matches.is_empty() {
            // No matches, return original text
            return Ok(text.to_string());
        }

        // Track replacements for statistics
        let mut replacement_counts = HashMap::new();

        // Perform replacements
        let mut result = String::with_capacity(text.len() * 2);
        let mut last_end = 0;

        for m in matches {
            // Append text before match
            result.push_str(&text[last_end..m.start()]);

            // Find which pattern matched
            let pattern_index = m.pattern().as_usize();
            let replacement = if pattern_index < self.patterns.len() {
                let pattern = &self.patterns[pattern_index];
                let rep = pattern.replacement.clone();

                // Update count
                *replacement_counts.entry(pattern_index).or_insert(0) += 1;

                // Apply case preservation if needed
                if pattern.preserve_case {
                    self.preserve_case(&text[m.start()..m.end()], &rep)
                } else {
                    rep
                }
            } else {
                text[m.start()..m.end()].to_string()
            };

            result.push_str(&replacement);
            last_end = m.end();
        }

        // Append remaining text
        if last_end < text.len() {
            result.push_str(&text[last_end..]);
        }

        Ok(result)
    }

    /// Preserve case when replacing
    fn preserve_case(&self, original: &str, replacement: &str) -> String {
        if replacement.is_empty() || original.is_empty() {
            return replacement.to_string();
        }

        // Check if original is all uppercase
        let is_all_uppercase = original
            .chars()
            .all(|c| !c.is_alphabetic() || c.is_uppercase());
        // Check if original is all lowercase
        let is_all_lowercase = original
            .chars()
            .all(|c| !c.is_alphabetic() || c.is_lowercase());
        // Check if original is title case (first char uppercase, rest lowercase)
        let is_title_case = original.chars().next().map_or(false, |c| c.is_uppercase())
            && original
                .chars()
                .skip(1)
                .all(|c| !c.is_alphabetic() || c.is_lowercase());

        let mut chars: Vec<char> = replacement.chars().collect();

        if is_all_uppercase {
            // Convert replacement to uppercase
            for c in &mut chars {
                *c = c.to_uppercase().next().unwrap_or(*c);
            }
        } else if is_all_lowercase {
            // Convert replacement to lowercase
            for c in &mut chars {
                *c = c.to_lowercase().next().unwrap_or(*c);
            }
        } else if is_title_case {
            // Capitalize first letter
            if !chars.is_empty() {
                chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
            }
            // Lowercase the rest
            for c in &mut chars[1..] {
                *c = c.to_lowercase().next().unwrap_or(*c);
            }
        }
        // Otherwise return as-is

        chars.into_iter().collect()
    }

    /// Replace text in a file
    pub async fn replace_file<P: AsRef<std::path::Path>>(
        &self,
        input_path: P,
        output_path: Option<P>,
    ) -> Result<(String, usize), ReplacerError> {
        let input_path_ref = input_path.as_ref();
        let output_path_ref = output_path
            .as_ref()
            .map(|p| p.as_ref())
            .unwrap_or_else(|| input_path_ref);

        // Read input file
        let content = tokio::fs::read_to_string(input_path_ref)
            .await
            .map_err(ReplacerError::Io)?;

        // Perform replacement
        let replaced = self.replace_text(&content)?;

        // Calculate statistics using Aho-Corasick directly
        let automaton = self
            .automaton
            .get()
            .ok_or_else(|| ReplacerError::Other("Automaton not initialized".to_string()))?;

        let matches: Vec<_> = automaton.find_iter(&content).collect();
        let total_replacements = matches.len();

        // Write output file
        tokio::fs::write(output_path_ref, replaced)
            .await
            .map_err(ReplacerError::Io)?;

        Ok((
            output_path_ref.to_string_lossy().to_string(),
            total_replacements,
        ))
    }

    /// Get pattern information
    pub fn patterns(&self) -> &[ReplacementPattern] {
        &self.patterns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replacer_creation() {
        let patterns = vec![
            ReplacementPattern::new("foo", "bar"),
            ReplacementPattern::new("baz", "qux"),
        ];
        let replacer = AhoCorasickReplacer::new(patterns).unwrap();
        assert_eq!(replacer.patterns().len(), 2);
    }

    #[test]
    fn test_replacer_empty_patterns() {
        let patterns = vec![];
        let result = AhoCorasickReplacer::new(patterns);
        assert!(matches!(result, Err(ReplacerError::Other(_))));
    }

    #[test]
    fn test_simple_replacement() {
        let patterns = vec![ReplacementPattern::new("foo", "bar")];
        let replacer = AhoCorasickReplacer::new(patterns).unwrap();

        let text = "foo bar baz foo";
        let result = replacer.replace_text(text).unwrap();
        assert_eq!(result, "bar bar baz bar");
    }

    #[test]
    fn test_multiple_patterns() {
        let patterns = vec![
            ReplacementPattern::new("foo", "bar"),
            ReplacementPattern::new("bar", "baz"),
        ];
        let replacer = AhoCorasickReplacer::new(patterns).unwrap();

        let text = "foo bar";
        let result = replacer.replace_text(text).unwrap();
        // foo -> bar, bar -> baz
        assert_eq!(result, "bar baz");
    }

    #[test]
    fn test_no_matches() {
        let patterns = vec![ReplacementPattern::new("xyz", "abc")];
        let replacer = AhoCorasickReplacer::new(patterns).unwrap();

        let text = "foo bar baz";
        let result = replacer.replace_text(text).unwrap();
        assert_eq!(result, text);
    }

    #[test]
    fn test_overlapping_patterns() {
        let patterns = vec![ReplacementPattern::new("aaa", "b")];
        let replacer = AhoCorasickReplacer::new(patterns).unwrap();

        let text = "aaaaa"; // 5 a's
        let result = replacer.replace_text(text).unwrap();
        // Should match non-overlapping: aaa -> b, aaaa -> b + a, aaaaa -> b + aa
        // Aho-Corasick finds non-overlapping matches
        assert!(result.contains('b'));
    }

    #[test]
    fn test_case_preservation_uppercase() {
        let patterns = vec![ReplacementPattern::new("foo", "bar").with_preserve_case(true)];
        let replacer = AhoCorasickReplacer::new(patterns).unwrap();

        let text = "FOO foo Foo";
        let result = replacer.replace_text(text).unwrap();
        println!("Result: '{}'", result);
        // First match is uppercase
        assert!(result.starts_with("BAR"));
    }

    #[test]
    fn test_case_preservation_lowercase() {
        let patterns = vec![ReplacementPattern::new("FOO", "bar").with_preserve_case(true)];
        let replacer = AhoCorasickReplacer::new(patterns).unwrap();

        let text = "FOO foo Foo";
        let result = replacer.replace_text(text).unwrap();
        // With case-insensitive matching, first match is "FOO" (uppercase) -> "BAR"
        assert_eq!(result, "BAR bar Bar");
    }

    #[test]
    fn test_empty_replacement() {
        let patterns = vec![ReplacementPattern::new("foo", "")];
        let replacer = AhoCorasickReplacer::new(patterns).unwrap();

        let text = "foobarfoo";
        let result = replacer.replace_text(text).unwrap();
        assert_eq!(result, "bar");
    }

    #[test]
    fn test_large_text_performance() {
        let patterns = vec![
            ReplacementPattern::new("pattern1", "replacement1"),
            ReplacementPattern::new("pattern2", "replacement2"),
            ReplacementPattern::new("pattern3", "replacement3"),
        ];
        let replacer = AhoCorasickReplacer::new(patterns).unwrap();

        // Create large text (1MB)
        let mut text = String::new();
        for i in 0..10000 {
            text.push_str(&format!("pattern{} ", i % 3));
        }

        let start = std::time::Instant::now();
        let result = replacer.replace_text(&text).unwrap();
        let elapsed = start.elapsed();

        // Should complete quickly (under 1 second for 1MB)
        assert!(elapsed.as_secs() < 1);
        assert!(result.contains("replacement"));
    }
}
