//! Code completion provider with LSP integration

use super::{CompletionItem, CompletionContext, CompletionProvider, ProviderConfig};
use anyhow::{Result, Context as AnyhowContext};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, warn};

/// Code completion provider with LSP support
#[derive(Debug, Clone)]
pub struct CodeProvider {
    config: ProviderConfig,
    supported_languages: HashMap<String, LanguageConfig>,
    enable_lsp: bool,
    fallback_completions: bool,
}

/// Configuration for a specific programming language
#[derive(Debug, Clone)]
struct LanguageConfig {
    name: String,
    file_extensions: Vec<String>,
    keywords: Vec<String>,
    common_patterns: Vec<String>,
    lsp_enabled: bool,
}

impl CodeProvider {
    /// Create a new code completion provider
    pub fn new() -> Self {
        let mut provider = Self {
            config: ProviderConfig::default(),
            supported_languages: HashMap::new(),
            enable_lsp: false, // LSP not implemented yet
            fallback_completions: true,
        };
        
        provider.register_default_languages();
        provider
    }

    /// Enable or disable LSP integration
    pub fn with_lsp_enabled(mut self, enabled: bool) -> Self {
        self.enable_lsp = enabled;
        self
    }

    /// Enable or disable fallback completions when LSP is unavailable
    pub fn with_fallback_completions(mut self, enabled: bool) -> Self {
        self.fallback_completions = enabled;
        self
    }

    /// Register default language configurations
    fn register_default_languages(&mut self) {
        // Rust
        self.supported_languages.insert("rust".to_string(), LanguageConfig {
            name: "Rust".to_string(),
            file_extensions: vec!["rs".to_string()],
            keywords: vec![
                "fn", "let", "mut", "const", "static", "struct", "enum", "impl", "trait",
                "pub", "use", "mod", "crate", "super", "self", "Self", "match", "if", "else",
                "while", "for", "loop", "break", "continue", "return", "async", "await",
                "unsafe", "extern", "type", "where", "dyn", "ref", "move", "Box", "Vec",
                "String", "str", "u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64",
                "f32", "f64", "bool", "char", "usize", "isize", "Option", "Result",
                "Some", "None", "Ok", "Err", "derive", "Debug", "Clone", "Copy",
            ].into_iter().map(String::from).collect(),
            common_patterns: vec![
                "println!", "eprintln!", "dbg!", "todo!", "unimplemented!", "unreachable!",
                "vec!", "format!", "assert!", "assert_eq!", "assert_ne!",
                "#[derive(", "#[cfg(", "#[allow(", "#[warn(", "#[deny(",
                "std::", "use std::", "impl<", "fn main(", "pub fn", "async fn",
            ].into_iter().map(String::from).collect(),
            lsp_enabled: false,
        });

        // Python
        self.supported_languages.insert("python".to_string(), LanguageConfig {
            name: "Python".to_string(),
            file_extensions: vec!["py".to_string(), "pyw".to_string()],
            keywords: vec![
                "def", "class", "if", "elif", "else", "for", "while", "try", "except",
                "finally", "with", "as", "import", "from", "return", "yield", "lambda",
                "and", "or", "not", "in", "is", "True", "False", "None", "pass", "break",
                "continue", "global", "nonlocal", "assert", "del", "raise", "async", "await",
            ].into_iter().map(String::from).collect(),
            common_patterns: vec![
                "print(", "len(", "range(", "enumerate(", "zip(", "list(", "dict(",
                "str(", "int(", "float(", "bool(", "type(", "isinstance(", "hasattr(",
                "if __name__ == '__main__':", "def __init__(self", "import os", "import sys",
                "from typing import", "from collections import", "import json", "import re",
            ].into_iter().map(String::from).collect(),
            lsp_enabled: false,
        });

        // JavaScript/TypeScript
        self.supported_languages.insert("javascript".to_string(), LanguageConfig {
            name: "JavaScript".to_string(),
            file_extensions: vec!["js".to_string(), "jsx".to_string(), "mjs".to_string()],
            keywords: vec![
                "function", "const", "let", "var", "if", "else", "for", "while", "do",
                "switch", "case", "default", "break", "continue", "return", "try", "catch",
                "finally", "throw", "new", "this", "super", "class", "extends", "static",
                "async", "await", "import", "export", "from", "as", "typeof", "instanceof",
                "true", "false", "null", "undefined", "void", "delete", "in", "of",
            ].into_iter().map(String::from).collect(),
            common_patterns: vec![
                "console.log(", "console.error(", "console.warn(", "JSON.stringify(",
                "JSON.parse(", "Array.from(", "Object.keys(", "Object.values(",
                "Promise.resolve(", "Promise.reject(", "async function", "=> {",
                "import React from", "export default", "module.exports", "require(",
            ].into_iter().map(String::from).collect(),
            lsp_enabled: false,
        });

        // TypeScript
        self.supported_languages.insert("typescript".to_string(), LanguageConfig {
            name: "TypeScript".to_string(),
            file_extensions: vec!["ts".to_string(), "tsx".to_string()],
            keywords: vec![
                "interface", "type", "enum", "namespace", "module", "declare", "abstract",
                "implements", "public", "private", "protected", "readonly", "static",
                "string", "number", "boolean", "object", "any", "unknown", "never", "void",
                "Array", "Promise", "Record", "Partial", "Required", "Pick", "Omit",
            ].into_iter().map(String::from).collect(),
            common_patterns: vec![
                "interface ", "type ", "enum ", "declare ", "export interface",
                "export type", "as const", ": string", ": number", ": boolean",
                "Array<", "Promise<", "Record<", "Partial<", "keyof ", "typeof ",
            ].into_iter().map(String::from).collect(),
            lsp_enabled: false,
        });

        // Go
        self.supported_languages.insert("go".to_string(), LanguageConfig {
            name: "Go".to_string(),
            file_extensions: vec!["go".to_string()],
            keywords: vec![
                "package", "import", "func", "var", "const", "type", "struct", "interface",
                "if", "else", "for", "range", "switch", "case", "default", "fallthrough",
                "break", "continue", "return", "go", "defer", "select", "chan", "map",
                "make", "new", "len", "cap", "append", "copy", "close", "delete",
                "panic", "recover", "nil", "true", "false", "iota",
            ].into_iter().map(String::from).collect(),
            common_patterns: vec![
                "func main(", "func (", "package main", "import (", "fmt.Println(",
                "fmt.Printf(", "log.Fatal(", "log.Println(", "if err != nil",
                "make([]", "make(map[", "make(chan", ":= range", "go func(",
            ].into_iter().map(String::from).collect(),
            lsp_enabled: false,
        });
    }

    /// Detect language from context
    fn detect_language(&self, context: &CompletionContext) -> Option<&LanguageConfig> {
        // First check explicit language context
        if let Some(ref lang) = context.language {
            return self.supported_languages.get(lang);
        }

        // Try to detect from file extension in working directory or file mentions
        let text = &context.text;
        for (_, lang_config) in &self.supported_languages {
            for ext in &lang_config.file_extensions {
                if text.contains(&format!(".{}", ext)) {
                    return Some(lang_config);
                }
            }
        }

        // Try to detect from keywords in the current text
        let words: Vec<&str> = context.text.split_whitespace().collect();
        let mut best_match: Option<(&LanguageConfig, usize)> = None;
        for (_, lang_config) in &self.supported_languages {
            let keyword_matches = words.iter()
                .filter(|word| lang_config.keywords.contains(&word.to_string()))
                .count();

            if keyword_matches >= 1 {
                if best_match.map_or(true, |(_, count)| keyword_matches > count) {
                    best_match = Some((lang_config, keyword_matches));
                }
            }
        }
        if let Some((lang, _)) = best_match {
            return Some(lang);
        }

        None
    }

    /// Get LSP completions (placeholder for future implementation)
    async fn get_lsp_completions(&self, _context: &CompletionContext, _language: &LanguageConfig) -> Result<Vec<CompletionItem>> {
        // TODO: Implement actual LSP integration
        // This would involve:
        // 1. Starting LSP server for the language
        // 2. Sending textDocument/completion request
        // 3. Parsing LSP completion response
        // 4. Converting to CompletionItems
        
        warn!("LSP completions not yet implemented");
        Ok(Vec::new())
    }

    /// Get fallback completions based on static analysis
    async fn get_fallback_completions(&self, context: &CompletionContext, language: &LanguageConfig) -> Result<Vec<CompletionItem>> {
        let mut items = Vec::new();
        let query = context.current_word().to_lowercase();

        // Add keyword completions
        for keyword in &language.keywords {
            if keyword.to_lowercase().starts_with(&query) {
                items.push(
                    CompletionItem::new(keyword, keyword, "keyword")
                        .with_description(format!("{} keyword", language.name))
                        .with_score(0.8)
                );
            }
        }

        // Add common pattern completions
        for pattern in &language.common_patterns {
            if pattern.to_lowercase().contains(&query) {
                let score = if pattern.to_lowercase().starts_with(&query) { 0.9 } else { 0.6 };
                items.push(
                    CompletionItem::new(pattern, pattern, "pattern")
                        .with_description(format!("{} pattern", language.name))
                        .with_score(score)
                );
            }
        }

        // Add context-specific completions
        items.extend(self.get_context_specific_completions(context, language).await?);

        Ok(items)
    }

    /// Get context-specific completions based on surrounding code
    async fn get_context_specific_completions(&self, context: &CompletionContext, language: &LanguageConfig) -> Result<Vec<CompletionItem>> {
        let mut items = Vec::new();
        let text = &context.text;
        let query = context.current_word();

        match language.name.as_str() {
            "Rust" => {
                // Rust-specific context completions
                if text.contains("use ") && !text.contains("::") {
                    let std_modules = ["std::collections", "std::fs", "std::io", "std::env", 
                                     "std::thread", "std::sync", "std::net", "std::path"];
                    for &module in &std_modules {
                        if module.contains(&query.to_lowercase()) {
                            items.push(
                                CompletionItem::new(module, module, "module")
                                    .with_description("Standard library module".to_string())
                                    .with_score(0.7)
                            );
                        }
                    }
                }

                if text.contains("Result<") || text.contains("Option<") {
                    let methods = ["unwrap()", "expect()", "unwrap_or()", "unwrap_or_else()", 
                                  "map()", "and_then()", "or_else()", "is_some()", "is_none()"];
                    for &method in &methods {
                        if method.starts_with(&query) {
                            items.push(
                                CompletionItem::new(method, method, "method")
                                    .with_description("Result/Option method".to_string())
                                    .with_score(0.8)
                            );
                        }
                    }
                }
            },
            "Python" => {
                // Python-specific context completions
                if text.contains("import ") {
                    let common_modules = ["os", "sys", "json", "re", "datetime", "collections",
                                         "itertools", "functools", "typing", "pathlib"];
                    for &module in &common_modules {
                        if module.starts_with(&query) {
                            items.push(
                                CompletionItem::new(module, module, "module")
                                    .with_description("Python module".to_string())
                                    .with_score(0.7)
                            );
                        }
                    }
                }

                if text.contains("self.") {
                    let common_methods = ["__init__", "__str__", "__repr__", "__len__", 
                                        "__getitem__", "__setitem__", "__contains__"];
                    for &method in &common_methods {
                        if method.starts_with(&query) {
                            items.push(
                                CompletionItem::new(method, method, "method")
                                    .with_description("Special method".to_string())
                                    .with_score(0.8)
                            );
                        }
                    }
                }
            },
            "JavaScript" | "TypeScript" => {
                // JS/TS-specific context completions
                if text.contains("import ") || text.contains("from ") {
                    let common_packages = ["react", "lodash", "axios", "express", "moment",
                                          "uuid", "crypto", "path", "fs", "util"];
                    for &package in &common_packages {
                        if package.starts_with(&query) {
                            items.push(
                                CompletionItem::new(package, package, "package")
                                    .with_description("NPM package".to_string())
                                    .with_score(0.7)
                            );
                        }
                    }
                }

                if text.contains("Array.") || text.contains("[].") {
                    let array_methods = ["map()", "filter()", "reduce()", "forEach()", "find()",
                                       "some()", "every()", "includes()", "indexOf()", "slice()"];
                    for &method in &array_methods {
                        if method.starts_with(&query) {
                            items.push(
                                CompletionItem::new(method, method, "method")
                                    .with_description("Array method".to_string())
                                    .with_score(0.8)
                            );
                        }
                    }
                }
            },
            "Go" => {
                // Go-specific context completions
                if text.contains("fmt.") {
                    let fmt_functions = ["Println()", "Printf()", "Print()", "Sprintf()", 
                                       "Errorf()", "Fprintf()", "Scanf()", "Sscanf()"];
                    for &func in &fmt_functions {
                        if func.starts_with(&query) {
                            items.push(
                                CompletionItem::new(func, func, "function")
                                    .with_description("fmt package function".to_string())
                                    .with_score(0.8)
                            );
                        }
                    }
                }
            },
            _ => {}
        }

        Ok(items)
    }
}

impl Default for CodeProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CompletionProvider for CodeProvider {
    fn name(&self) -> &str {
        "code"
    }

    async fn get_completions(&self, context: &CompletionContext) -> Result<Vec<CompletionItem>> {
        let language = match self.detect_language(context) {
            Some(lang) => lang,
            None => {
                debug!("No language detected for code completion");
                return Ok(Vec::new());
            }
        };

        debug!("Code completion for language: {}", language.name);

        // Try LSP completions first if enabled
        if self.enable_lsp && language.lsp_enabled {
            match self.get_lsp_completions(context, language).await {
                Ok(items) if !items.is_empty() => return Ok(items),
                Ok(_) => debug!("LSP returned no completions"),
                Err(e) => warn!("LSP completion failed: {}", e),
            }
        }

        // Fall back to static completions
        if self.fallback_completions {
            self.get_fallback_completions(context, language).await
        } else {
            Ok(Vec::new())
        }
    }

    fn is_applicable(&self, context: &CompletionContext) -> bool {
        // Check if we can detect a supported language
        self.detect_language(context).is_some()
    }

    fn get_priority(&self, context: &CompletionContext) -> i32 {
        if let Some(language) = self.detect_language(context) {
            if self.enable_lsp && language.lsp_enabled {
                20 // Highest priority for LSP-enabled languages
            } else {
                12 // High priority for supported languages
            }
        } else {
            0 // Not applicable
        }
    }

    fn supports_caching(&self) -> bool {
        // LSP completions are context-sensitive, but fallback completions can be cached
        !self.enable_lsp
    }

    fn cache_ttl(&self) -> Option<u64> {
        Some(600) // Cache for 10 minutes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        let provider = CodeProvider::new();
        
        // Test explicit language context
        let context = CompletionContext {
            language: Some("rust".to_string()),
            ..Default::default()
        };
        let detected = provider.detect_language(&context);
        assert!(detected.is_some());
        assert_eq!(detected.unwrap().name, "Rust");

        // Test file extension detection
        let context = CompletionContext {
            text: "edit main.rs".to_string(),
            cursor_pos: 8,
            ..Default::default()
        };
        let detected = provider.detect_language(&context);
        assert!(detected.is_some());
        assert_eq!(detected.unwrap().name, "Rust");

        // Test keyword detection
        let context = CompletionContext {
            text: "fn main() { let x = 5; }".to_string(),
            cursor_pos: 10,
            ..Default::default()
        };
        let detected = provider.detect_language(&context);
        assert!(detected.is_some());
        assert_eq!(detected.unwrap().name, "Rust");
    }

    #[tokio::test]
    async fn test_fallback_completions() {
        let provider = CodeProvider::new();

        let context = CompletionContext {
            text: "fn ".to_string(),
            cursor_pos: 3,
            language: Some("rust".to_string()),
            ..Default::default()
        };

        let completions = provider.get_completions(&context).await.unwrap();

        // With empty current word, should return all keywords
        assert!(!completions.is_empty());
    }

    #[tokio::test]
    async fn test_context_specific_completions() {
        let provider = CodeProvider::new();
        let rust_lang = provider.supported_languages.get("rust").unwrap();

        // Test "use " without "::" so the std module completions trigger
        let context = CompletionContext {
            text: "use col".to_string(),
            cursor_pos: 7,
            language: Some("rust".to_string()),
            ..Default::default()
        };

        let completions = provider.get_context_specific_completions(&context, rust_lang).await.unwrap();

        // Should suggest std::collections
        assert!(completions.iter().any(|c| c.title.contains("collections")));
    }

    #[test]
    fn test_provider_applicability() {
        let provider = CodeProvider::new();
        
        // Should apply to contexts with detectable languages
        let context1 = CompletionContext {
            text: "fn main() {".to_string(),
            cursor_pos: 5,
            ..Default::default()
        };
        assert!(provider.is_applicable(&context1));
        
        // Should not apply to plain text
        let context2 = CompletionContext {
            text: "hello world".to_string(),
            cursor_pos: 5,
            ..Default::default()
        };
        assert!(!provider.is_applicable(&context2));
    }
}