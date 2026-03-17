//! Message formatting and styling utilities
//!
//! This module provides utilities for formatting chat messages, applying themes,
//! syntax highlighting, and creating consistent visual presentation.

use crate::llm::types::{ContentBlock, MessageRole, ToolCall};
use crate::tui::themes::{Theme, ThemeManager};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};
use std::collections::HashMap;
use unicode_width::UnicodeWidthStr;

/// Text formatter for rich message display
pub struct MessageFormatter {
    theme_manager: ThemeManager,
    syntax_highlighter: SyntaxHighlighter,
    markdown_renderer: MarkdownRenderer,
    code_highlighter: CodeHighlighter,
    emoji_support: bool,
    max_line_width: Option<usize>,
}

/// Syntax highlighter for code blocks
pub struct SyntaxHighlighter {
    language_configs: HashMap<String, LanguageConfig>,
    default_style: Style,
}

/// Markdown renderer for text formatting
pub struct MarkdownRenderer {
    heading_styles: Vec<Style>,
    emphasis_styles: EmphasisStyles,
    list_markers: ListMarkers,
    code_style: Style,
    quote_style: Style,
    link_style: Style,
}

/// Code highlighter for programming languages
pub struct CodeHighlighter {
    languages: HashMap<String, LanguageHighlighter>,
    fallback_style: Style,
}

/// Language-specific highlighting configuration
#[derive(Debug, Clone)]
pub struct LanguageConfig {
    pub name: String,
    pub keywords: Vec<String>,
    pub operators: Vec<String>,
    pub delimiters: Vec<String>,
    pub comment_prefixes: Vec<String>,
    pub string_delimiters: Vec<(String, String)>, // (start, end)
    pub styles: LanguageStyles,
}

/// Styles for different language elements
#[derive(Debug, Clone)]
pub struct LanguageStyles {
    pub keyword: Style,
    pub operator: Style,
    pub string: Style,
    pub number: Style,
    pub comment: Style,
    pub function: Style,
    pub type_name: Style,
    pub variable: Style,
    pub constant: Style,
}

/// Emphasis styling configuration
#[derive(Debug, Clone)]
pub struct EmphasisStyles {
    pub bold: Style,
    pub italic: Style,
    pub underline: Style,
    pub strikethrough: Style,
    pub code: Style,
}

/// List marker configuration
#[derive(Debug, Clone)]
pub struct ListMarkers {
    pub unordered: Vec<String>,
    pub ordered_format: String, // e.g., "{}."
}

/// Language-specific highlighter
pub struct LanguageHighlighter {
    config: LanguageConfig,
}

/// Formatting options for messages
#[derive(Debug, Clone)]
pub struct FormatOptions {
    pub show_timestamps: bool,
    pub show_role_icons: bool,
    pub compact_mode: bool,
    pub word_wrap: bool,
    pub max_width: Option<usize>,
    pub indent_level: usize,
    pub preserve_whitespace: bool,
    pub highlight_mentions: bool,
    pub highlight_code: bool,
    pub render_markdown: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            show_timestamps: true,
            show_role_icons: true,
            compact_mode: false,
            word_wrap: true,
            max_width: None,
            indent_level: 0,
            preserve_whitespace: false,
            highlight_mentions: true,
            highlight_code: true,
            render_markdown: true,
        }
    }
}

/// Formatted text result
#[derive(Debug, Clone)]
pub struct FormattedText<'a> {
    pub lines: Vec<Line<'a>>,
    pub width: usize,
    pub height: usize,
    pub metadata: FormatMetadata,
}

/// Metadata about formatted text
#[derive(Debug, Clone)]
pub struct FormatMetadata {
    pub has_code_blocks: bool,
    pub has_links: bool,
    pub has_images: bool,
    pub has_tables: bool,
    pub estimated_reading_time: std::time::Duration,
    pub character_count: usize,
    pub word_count: usize,
    pub line_count: usize,
}

impl MessageFormatter {
    /// Create a new message formatter
    pub fn new() -> Self {
        let theme_manager = ThemeManager::new();
        let current_theme = theme_manager.current_theme().clone();

        Self {
            syntax_highlighter: SyntaxHighlighter::new(&current_theme),
            markdown_renderer: MarkdownRenderer::new(&current_theme),
            code_highlighter: CodeHighlighter::new(&current_theme),
            theme_manager,
            emoji_support: true,
            max_line_width: None,
        }
    }

    /// Set theme
    pub fn set_theme(&mut self, theme_name: &str) -> Result<(), String> {
        self.theme_manager.set_theme(theme_name)?;
        let current_theme = self.theme_manager.current_theme();
        
        self.syntax_highlighter.update_theme(current_theme);
        self.markdown_renderer.update_theme(current_theme);
        self.code_highlighter.update_theme(current_theme);
        
        Ok(())
    }

    /// Set maximum line width
    pub fn set_max_line_width(&mut self, width: Option<usize>) {
        self.max_line_width = width;
    }

    /// Format message content
    pub fn format_content(&self, content: &[ContentBlock], options: &FormatOptions) -> FormattedText<'static> {
        let mut lines = Vec::new();
        let mut metadata = FormatMetadata::default();
        
        for block in content {
            let formatted_block = self.format_content_block(block, options);
            lines.extend(formatted_block.lines);
            metadata.merge(&formatted_block.metadata);
        }
        
        let width = lines.iter()
            .map(|line| line.width())
            .max()
            .unwrap_or(0);
        
        let height = lines.len();
        
        FormattedText {
            lines,
            width,
            height,
            metadata,
        }
    }

    /// Format a single content block
    pub fn format_content_block(&self, block: &ContentBlock, options: &FormatOptions) -> FormattedText<'static> {
        match block {
            ContentBlock::Text { text } => {
                if options.render_markdown {
                    self.markdown_renderer.render(text, options)
                } else {
                    self.format_plain_text(text, options)
                }
            }
            ContentBlock::Image { .. } => {
                self.format_image_placeholder(options)
            }
            ContentBlock::ToolUse { id, name, input } => {
                self.format_tool_use(id, name, input, options)
            }
            ContentBlock::ToolResult { tool_call_id, content } => {
                self.format_tool_result(tool_call_id, content, options)
            }
        }
    }

    /// Format plain text without markdown
    fn format_plain_text(&self, text: &str, options: &FormatOptions) -> FormattedText<'static> {
        let theme = self.theme_manager.current_theme();
        let mut lines = Vec::new();
        
        for line in text.lines() {
            if options.word_wrap && options.max_width.is_some() {
                let wrapped_lines = self.wrap_text(line, options.max_width.unwrap());
                for wrapped_line in wrapped_lines {
                    lines.push(Line::from(Span::styled(wrapped_line, theme.styles.text)));
                }
            } else {
                lines.push(Line::from(Span::styled(line.to_string(), theme.styles.text)));
            }
        }
        
        let metadata = self.calculate_metadata(text);
        
        FormattedText {
            width: lines.iter().map(|l| l.width()).max().unwrap_or(0),
            height: lines.len(),
            lines,
            metadata,
        }
    }

    /// Format image placeholder
    fn format_image_placeholder(&self, _options: &FormatOptions) -> FormattedText<'static> {
        let theme = self.theme_manager.current_theme();
        
        let line = Line::from(vec![
            Span::styled("🖼️ ", theme.styles.info),
            Span::styled("[Image]", theme.styles.info),
        ]);
        
        FormattedText {
            lines: vec![line],
            width: 8, // Approximate width
            height: 1,
            metadata: FormatMetadata {
                has_images: true,
                character_count: 7,
                word_count: 1,
                line_count: 1,
                ..Default::default()
            },
        }
    }

    /// Format tool use block
    fn format_tool_use(&self, id: &str, name: &str, input: &serde_json::Value, options: &FormatOptions) -> FormattedText<'static> {
        let theme = self.theme_manager.current_theme();
        let mut lines: Vec<Line<'static>> = Vec::new();

        // Tool header
        lines.push(Line::from(vec![
            Span::styled("🔧 ".to_string(), theme.styles.chat_tool_message),
            Span::styled("Tool: ".to_string(), theme.styles.chat_tool_message),
            Span::styled(name.to_string(), theme.styles.chat_tool_message.add_modifier(Modifier::BOLD)),
        ]));

        // Tool ID (if not compact)
        if !options.compact_mode {
            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled("ID: ".to_string(), theme.styles.muted),
                Span::styled(id.to_string(), theme.styles.muted),
            ]));
        }

        // Format input
        if let Ok(formatted_input) = serde_json::to_string_pretty(input) {
            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled("Input:".to_string(), theme.styles.muted),
            ]));

            let code_lines = self.code_highlighter.highlight("json", &formatted_input);
            for code_line in code_lines {
                let mut indented_spans = vec![Span::raw("     ")]; // Extra indentation
                indented_spans.extend(code_line.spans);
                lines.push(Line::from(indented_spans));
            }
        }

        let width = lines.iter().map(|l| l.width()).max().unwrap_or(0);
        let height = lines.len();
        let metadata = FormatMetadata {
            character_count: name.len() + id.len(),
            word_count: 2,
            line_count: height,
            ..Default::default()
        };

        FormattedText {
            lines,
            width,
            height,
            metadata,
        }
    }

    /// Format tool result block
    fn format_tool_result(&self, tool_call_id: &str, content: &str, options: &FormatOptions) -> FormattedText<'static> {
        let theme = self.theme_manager.current_theme();
        let mut lines: Vec<Line<'static>> = Vec::new();

        // Result header
        lines.push(Line::from(vec![
            Span::styled("📋 ".to_string(), theme.styles.success),
            Span::styled("Tool Result:".to_string(), theme.styles.success),
        ]));

        // Tool call ID (if not compact)
        if !options.compact_mode {
            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled("Call ID: ".to_string(), theme.styles.muted),
                Span::styled(tool_call_id.to_string(), theme.styles.muted),
            ]));
        }

        // Format content
        for line in content.lines() {
            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled(line.to_string(), theme.styles.text),
            ]));
        }

        let width = lines.iter().map(|l| l.width()).max().unwrap_or(0);
        let height = lines.len();
        let metadata = self.calculate_metadata(content);

        FormattedText {
            lines,
            width,
            height,
            metadata,
        }
    }

    /// Wrap text to specified width
    fn wrap_text(&self, text: &str, max_width: usize) -> Vec<String> {
        if text.width() <= max_width {
            return vec![text.to_string()];
        }
        
        let mut lines = Vec::new();
        let mut current_line = String::new();
        let mut current_width = 0;
        
        for word in text.split_whitespace() {
            let word_width = word.width();
            
            if current_width + word_width + 1 > max_width && !current_line.is_empty() {
                lines.push(current_line);
                current_line = word.to_string();
                current_width = word_width;
            } else {
                if !current_line.is_empty() {
                    current_line.push(' ');
                    current_width += 1;
                }
                current_line.push_str(word);
                current_width += word_width;
            }
        }
        
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        lines
    }

    /// Calculate metadata for text
    fn calculate_metadata(&self, text: &str) -> FormatMetadata {
        let character_count = text.chars().count();
        let word_count = text.split_whitespace().count();
        let line_count = text.lines().count();
        
        // Estimate reading time (average 200 words per minute)
        let reading_time = std::time::Duration::from_secs((word_count * 60 / 200) as u64);
        
        FormatMetadata {
            has_code_blocks: text.contains("```"),
            has_links: text.contains("http://") || text.contains("https://") || text.contains("[") && text.contains("]("),
            has_images: text.contains("!["),
            has_tables: text.contains('|') && text.lines().any(|line| line.matches('|').count() > 1),
            estimated_reading_time: reading_time,
            character_count,
            word_count,
            line_count,
        }
    }

    /// Format role indicator
    pub fn format_role(&self, role: &MessageRole, options: &FormatOptions) -> Vec<Span<'static>> {
        let theme = self.theme_manager.current_theme();
        let mut spans = Vec::new();
        
        if options.show_role_icons {
            let (icon, style) = match role {
                MessageRole::User => (&theme.icons.user, theme.styles.chat_user_message),
                MessageRole::Assistant => (&theme.icons.assistant, theme.styles.chat_assistant_message),
                MessageRole::System => (&theme.icons.system, theme.styles.chat_system_message),
                MessageRole::Tool => (&theme.icons.tool, theme.styles.chat_tool_message),
            };
            
            spans.push(Span::styled(icon.clone(), style));
            spans.push(Span::raw(" "));
        }
        
        let role_text = match role {
            MessageRole::User => "You",
            MessageRole::Assistant => "Assistant",
            MessageRole::System => "System",
            MessageRole::Tool => "Tool",
        };
        
        let style = match role {
            MessageRole::User => theme.styles.chat_user_message,
            MessageRole::Assistant => theme.styles.chat_assistant_message,
            MessageRole::System => theme.styles.chat_system_message,
            MessageRole::Tool => theme.styles.chat_tool_message,
        };
        
        spans.push(Span::styled(role_text.to_string(), style));
        spans
    }

    /// Format timestamp
    pub fn format_timestamp(&self, timestamp: chrono::DateTime<chrono::Utc>, options: &FormatOptions) -> Option<Vec<Span<'static>>> {
        if !options.show_timestamps {
            return None;
        }
        
        let theme = self.theme_manager.current_theme();
        let formatted_time = if options.compact_mode {
            timestamp.format("%H:%M").to_string()
        } else {
            timestamp.format("%H:%M:%S").to_string()
        };
        
        Some(vec![
            Span::raw(" • "),
            Span::styled(formatted_time, theme.styles.muted),
        ])
    }
}

impl SyntaxHighlighter {
    fn new(theme: &Theme) -> Self {
        let mut language_configs = HashMap::new();
        
        // Add common language configurations
        language_configs.insert("rust".to_string(), create_rust_config(theme));
        language_configs.insert("python".to_string(), create_python_config(theme));
        language_configs.insert("javascript".to_string(), create_javascript_config(theme));
        language_configs.insert("json".to_string(), create_json_config(theme));
        
        Self {
            language_configs,
            default_style: theme.styles.text,
        }
    }

    fn update_theme(&mut self, theme: &Theme) {
        self.default_style = theme.styles.text;
        
        // Update all language configs with new theme
        self.language_configs.insert("rust".to_string(), create_rust_config(theme));
        self.language_configs.insert("python".to_string(), create_python_config(theme));
        self.language_configs.insert("javascript".to_string(), create_javascript_config(theme));
        self.language_configs.insert("json".to_string(), create_json_config(theme));
    }

    /// Highlight code with syntax coloring
    pub fn highlight(&self, language: &str, code: &str) -> Vec<Line<'static>> {
        if let Some(config) = self.language_configs.get(language) {
            let highlighter = LanguageHighlighter { config: config.clone() };
            highlighter.highlight(code)
        } else {
            // Return code with default styling
            code.lines()
                .map(|line| Line::from(Span::styled(line.to_string(), self.default_style)))
                .collect()
        }
    }
}

impl MarkdownRenderer {
    fn new(theme: &Theme) -> Self {
        Self {
            heading_styles: vec![
                theme.styles.title.add_modifier(Modifier::BOLD),
                theme.styles.subtitle.add_modifier(Modifier::BOLD),
                theme.styles.text.add_modifier(Modifier::BOLD),
            ],
            emphasis_styles: EmphasisStyles {
                bold: theme.styles.text.add_modifier(Modifier::BOLD),
                italic: theme.styles.text.add_modifier(Modifier::ITALIC),
                underline: theme.styles.text.add_modifier(Modifier::UNDERLINED),
                strikethrough: theme.styles.muted,
                code: Style::default().fg(theme.colors.green),
            },
            list_markers: ListMarkers {
                unordered: vec!["•".to_string(), "◦".to_string(), "▪".to_string()],
                ordered_format: "{}.".to_string(),
            },
            code_style: Style::default().fg(theme.colors.green),
            quote_style: theme.styles.muted.add_modifier(Modifier::ITALIC),
            link_style: Style::default().fg(theme.colors.blue).add_modifier(Modifier::UNDERLINED),
        }
    }

    fn update_theme(&mut self, theme: &Theme) {
        self.heading_styles = vec![
            theme.styles.title.add_modifier(Modifier::BOLD),
            theme.styles.subtitle.add_modifier(Modifier::BOLD),
            theme.styles.text.add_modifier(Modifier::BOLD),
        ];
        self.emphasis_styles.bold = theme.styles.text.add_modifier(Modifier::BOLD);
        self.emphasis_styles.italic = theme.styles.text.add_modifier(Modifier::ITALIC);
        self.emphasis_styles.code = Style::default().fg(theme.colors.green);
        self.code_style = Style::default().fg(theme.colors.green);
        self.quote_style = theme.styles.muted.add_modifier(Modifier::ITALIC);
        self.link_style = Style::default().fg(theme.colors.blue).add_modifier(Modifier::UNDERLINED);
    }

    /// Render markdown text
    pub fn render(&self, text: &str, options: &FormatOptions) -> FormattedText<'static> {
        let mut lines = Vec::new();
        let mut in_code_block = false;
        let mut _code_language: Option<String> = None;
        let _list_depth = 0;
        
        for line in text.lines() {
            if line.starts_with("```") {
                if in_code_block {
                    in_code_block = false;
                    _code_language = None;
                } else {
                    in_code_block = true;
                    let lang = line.trim_start_matches("```").trim();
                    _code_language = if lang.is_empty() { None } else { Some(lang.to_string()) };
                    
                    lines.push(Line::from(vec![
                        Span::styled("📄 ".to_string(), Style::default().fg(Color::Cyan)),
                        Span::styled(lang.to_string(), self.code_style),
                    ]));
                    continue;
                }
            }
            
            if in_code_block {
                lines.push(Line::from(Span::styled(line.to_string(), self.code_style)));
            } else {
                lines.push(self.render_markdown_line(line, _list_depth, options));
            }
        }
        
        let width = lines.iter().map(|l| l.width()).max().unwrap_or(0);
        let height = lines.len();
        let metadata = FormatMetadata {
            has_code_blocks: text.contains("```"),
            character_count: text.chars().count(),
            word_count: text.split_whitespace().count(),
            line_count: lines.len(),
            ..Default::default()
        };
        
        FormattedText {
            lines,
            width,
            height,
            metadata,
        }
    }

    /// Render a single markdown line
    fn render_markdown_line(&self, line: &str, _list_depth: usize, _options: &FormatOptions) -> Line<'static> {
        // Handle headers
        if line.starts_with("# ") {
            return Line::from(Span::styled(
                line.trim_start_matches("# ").to_string(),
                self.heading_styles[0],
            ));
        } else if line.starts_with("## ") {
            return Line::from(Span::styled(
                line.trim_start_matches("## ").to_string(),
                self.heading_styles[1],
            ));
        } else if line.starts_with("### ") {
            return Line::from(Span::styled(
                line.trim_start_matches("### ").to_string(),
                self.heading_styles[2],
            ));
        }
        
        // Handle quotes
        if line.starts_with("> ") {
            return Line::from(vec![
                Span::styled("▎ ", self.quote_style),
                Span::styled(line.trim_start_matches("> ").to_string(), self.quote_style),
            ]);
        }
        
        // Handle lists
        if line.trim().starts_with("- ") || line.trim().starts_with("* ") {
            return Line::from(vec![
                Span::raw("  "),
                Span::styled("• ", Style::default().fg(Color::Cyan)),
                Span::raw(line.trim().trim_start_matches("- ").trim_start_matches("* ").to_string()),
            ]);
        }
        
        // Handle inline formatting
        self.render_inline_formatting(line)
    }

    /// Render inline formatting (bold, italic, code, links)
    fn render_inline_formatting(&self, line: &str) -> Line<'static> {
        let mut spans = Vec::new();
        let mut chars = line.chars().peekable();
        let mut current_text = String::new();
        
        while let Some(ch) = chars.next() {
            match ch {
                '*' if chars.peek() == Some(&'*') => {
                    // Bold text **text**
                    if !current_text.is_empty() {
                        spans.push(Span::raw(current_text.clone()));
                        current_text.clear();
                    }
                    chars.next(); // consume second *
                    
                    let bold_text = self.consume_until(&mut chars, "**");
                    spans.push(Span::styled(bold_text, self.emphasis_styles.bold));
                }
                '*' => {
                    // Italic text *text*
                    if !current_text.is_empty() {
                        spans.push(Span::raw(current_text.clone()));
                        current_text.clear();
                    }
                    
                    let italic_text = self.consume_until(&mut chars, "*");
                    spans.push(Span::styled(italic_text, self.emphasis_styles.italic));
                }
                '`' => {
                    // Inline code `code`
                    if !current_text.is_empty() {
                        spans.push(Span::raw(current_text.clone()));
                        current_text.clear();
                    }
                    
                    let code_text = self.consume_until(&mut chars, "`");
                    spans.push(Span::styled(code_text, self.emphasis_styles.code));
                }
                _ => {
                    current_text.push(ch);
                }
            }
        }
        
        if !current_text.is_empty() {
            spans.push(Span::raw(current_text));
        }
        
        Line::from(spans)
    }

    /// Consume characters until delimiter is found
    fn consume_until(&self, chars: &mut std::iter::Peekable<std::str::Chars>, delimiter: &str) -> String {
        let mut text = String::new();
        let delimiter_chars: Vec<char> = delimiter.chars().collect();
        
        while let Some(ch) = chars.next() {
            if ch == delimiter_chars[0] {
                // Check if this is the start of our delimiter
                let mut is_delimiter = true;
                let mut lookahead = Vec::new();
                
                for &expected_char in &delimiter_chars[1..] {
                    if let Some(next_ch) = chars.next() {
                        lookahead.push(next_ch);
                        if next_ch != expected_char {
                            is_delimiter = false;
                            break;
                        }
                    } else {
                        is_delimiter = false;
                        break;
                    }
                }
                
                if is_delimiter {
                    return text;
                } else {
                    text.push(ch);
                    text.extend(lookahead);
                }
            } else {
                text.push(ch);
            }
        }
        
        text
    }
}

impl CodeHighlighter {
    fn new(theme: &Theme) -> Self {
        let mut languages = HashMap::new();
        
        languages.insert("rust".to_string(), LanguageHighlighter {
            config: create_rust_config(theme),
        });
        
        Self {
            languages,
            fallback_style: theme.styles.text,
        }
    }

    fn update_theme(&mut self, theme: &Theme) {
        self.fallback_style = theme.styles.text;
        
        // Update language highlighters
        for (name, highlighter) in &mut self.languages {
            match name.as_str() {
                "rust" => highlighter.config = create_rust_config(theme),
                _ => {}
            }
        }
    }

    /// Highlight code with language-specific syntax
    pub fn highlight(&self, language: &str, code: &str) -> Vec<Line<'static>> {
        if let Some(highlighter) = self.languages.get(language) {
            highlighter.highlight(code)
        } else {
            code.lines()
                .map(|line| Line::from(Span::styled(line.to_string(), self.fallback_style)))
                .collect()
        }
    }
}

impl LanguageHighlighter {
    fn highlight(&self, code: &str) -> Vec<Line<'static>> {
        code.lines()
            .map(|line| self.highlight_line(line))
            .collect()
    }

    fn highlight_line(&self, line: &str) -> Line<'static> {
        let mut spans = Vec::new();
        let mut current_word = String::new();
        let mut in_string = false;
        let mut in_comment = false;
        
        for ch in line.chars() {
            if in_comment {
                current_word.push(ch);
                continue;
            }
            
            if ch.is_whitespace() || "(){}[];,".contains(ch) {
                if !current_word.is_empty() {
                    spans.push(self.style_word(&current_word, in_string));
                    current_word.clear();
                }
                spans.push(Span::raw(ch.to_string()));
                in_string = false;
            } else if ch == '"' || ch == '\'' {
                if !current_word.is_empty() {
                    spans.push(self.style_word(&current_word, in_string));
                    current_word.clear();
                }
                in_string = !in_string;
                current_word.push(ch);
            } else if line[..line.len() - line.split_at(line.len() - 1).1.len()].ends_with("//") {
                in_comment = true;
                current_word.push(ch);
            } else {
                current_word.push(ch);
            }
        }
        
        if !current_word.is_empty() {
            spans.push(self.style_word(&current_word, in_string || in_comment));
        }
        
        Line::from(spans)
    }

    fn style_word(&self, word: &str, is_string_or_comment: bool) -> Span<'static> {
        if is_string_or_comment {
            if word.starts_with("//") || word.starts_with("/*") {
                Span::styled(word.to_string(), self.config.styles.comment)
            } else {
                Span::styled(word.to_string(), self.config.styles.string)
            }
        } else if self.config.keywords.contains(&word.to_string()) {
            Span::styled(word.to_string(), self.config.styles.keyword)
        } else if word.chars().all(|c| c.is_ascii_digit() || c == '.') {
            Span::styled(word.to_string(), self.config.styles.number)
        } else if word.chars().next().map_or(false, |c| c.is_uppercase()) {
            Span::styled(word.to_string(), self.config.styles.type_name)
        } else {
            Span::styled(word.to_string(), self.config.styles.variable)
        }
    }
}

impl Default for FormatMetadata {
    fn default() -> Self {
        Self {
            has_code_blocks: false,
            has_links: false,
            has_images: false,
            has_tables: false,
            estimated_reading_time: std::time::Duration::from_secs(0),
            character_count: 0,
            word_count: 0,
            line_count: 0,
        }
    }
}

impl FormatMetadata {
    fn merge(&mut self, other: &FormatMetadata) {
        self.has_code_blocks |= other.has_code_blocks;
        self.has_links |= other.has_links;
        self.has_images |= other.has_images;
        self.has_tables |= other.has_tables;
        self.estimated_reading_time += other.estimated_reading_time;
        self.character_count += other.character_count;
        self.word_count += other.word_count;
        self.line_count += other.line_count;
    }
}

impl Default for MessageFormatter {
    fn default() -> Self {
        Self::new()
    }
}

// Language configuration helpers

fn create_rust_config(theme: &Theme) -> LanguageConfig {
    LanguageConfig {
        name: "Rust".to_string(),
        keywords: vec![
            "fn", "let", "mut", "const", "static", "if", "else", "while", "for", "loop",
            "match", "struct", "enum", "impl", "trait", "mod", "use", "pub", "crate",
            "self", "Self", "super", "return", "break", "continue", "true", "false",
        ].into_iter().map(|s| s.to_string()).collect(),
        operators: vec!["=", "+", "-", "*", "/", "%", "==", "!=", "<", ">", "<=", ">="]
            .into_iter().map(|s| s.to_string()).collect(),
        delimiters: vec!["(", ")", "{", "}", "[", "]"]
            .into_iter().map(|s| s.to_string()).collect(),
        comment_prefixes: vec!["//".to_string(), "/*".to_string()],
        string_delimiters: vec![("\"".to_string(), "\"".to_string())],
        styles: LanguageStyles {
            keyword: Style::default().fg(theme.colors.blue).add_modifier(Modifier::BOLD),
            operator: Style::default().fg(theme.colors.yellow),
            string: Style::default().fg(theme.colors.green),
            number: Style::default().fg(theme.colors.red),
            comment: theme.styles.muted,
            function: Style::default().fg(theme.colors.blue_light),
            type_name: Style::default().fg(theme.colors.green_light),
            variable: theme.styles.text,
            constant: Style::default().fg(theme.colors.red).add_modifier(Modifier::BOLD),
        },
    }
}

fn create_python_config(theme: &Theme) -> LanguageConfig {
    LanguageConfig {
        name: "Python".to_string(),
        keywords: vec![
            "def", "class", "if", "else", "elif", "while", "for", "in", "try", "except",
            "finally", "with", "as", "import", "from", "return", "yield", "lambda",
            "and", "or", "not", "True", "False", "None",
        ].into_iter().map(|s| s.to_string()).collect(),
        operators: vec!["=", "+", "-", "*", "/", "//", "%", "**", "==", "!=", "<", ">", "<=", ">="]
            .into_iter().map(|s| s.to_string()).collect(),
        delimiters: vec!["(", ")", "{", "}", "[", "]"]
            .into_iter().map(|s| s.to_string()).collect(),
        comment_prefixes: vec!["#".to_string()],
        string_delimiters: vec![
            ("\"".to_string(), "\"".to_string()),
            ("'".to_string(), "'".to_string()),
            ("\"\"\"".to_string(), "\"\"\"".to_string()),
        ],
        styles: LanguageStyles {
            keyword: Style::default().fg(theme.colors.blue).add_modifier(Modifier::BOLD),
            operator: Style::default().fg(theme.colors.yellow),
            string: Style::default().fg(theme.colors.green),
            number: Style::default().fg(theme.colors.red),
            comment: theme.styles.muted,
            function: Style::default().fg(theme.colors.blue_light),
            type_name: Style::default().fg(theme.colors.green_light),
            variable: theme.styles.text,
            constant: Style::default().fg(theme.colors.red).add_modifier(Modifier::BOLD),
        },
    }
}

fn create_javascript_config(theme: &Theme) -> LanguageConfig {
    LanguageConfig {
        name: "JavaScript".to_string(),
        keywords: vec![
            "function", "var", "let", "const", "if", "else", "while", "for", "do",
            "switch", "case", "default", "break", "continue", "return", "try", "catch",
            "finally", "throw", "new", "this", "super", "class", "extends", "true", "false",
        ].into_iter().map(|s| s.to_string()).collect(),
        operators: vec!["=", "+", "-", "*", "/", "%", "==", "===", "!=", "!==", "<", ">", "<=", ">="]
            .into_iter().map(|s| s.to_string()).collect(),
        delimiters: vec!["(", ")", "{", "}", "[", "]"]
            .into_iter().map(|s| s.to_string()).collect(),
        comment_prefixes: vec!["//".to_string(), "/*".to_string()],
        string_delimiters: vec![
            ("\"".to_string(), "\"".to_string()),
            ("'".to_string(), "'".to_string()),
            ("`".to_string(), "`".to_string()),
        ],
        styles: LanguageStyles {
            keyword: Style::default().fg(theme.colors.blue).add_modifier(Modifier::BOLD),
            operator: Style::default().fg(theme.colors.yellow),
            string: Style::default().fg(theme.colors.green),
            number: Style::default().fg(theme.colors.red),
            comment: theme.styles.muted,
            function: Style::default().fg(theme.colors.blue_light),
            type_name: Style::default().fg(theme.colors.green_light),
            variable: theme.styles.text,
            constant: Style::default().fg(theme.colors.red).add_modifier(Modifier::BOLD),
        },
    }
}

fn create_json_config(theme: &Theme) -> LanguageConfig {
    LanguageConfig {
        name: "JSON".to_string(),
        keywords: vec!["true".to_string(), "false".to_string(), "null".to_string()],
        operators: vec![":".to_string(), ",".to_string()],
        delimiters: vec!["{", "}", "[", "]"]
            .into_iter().map(|s| s.to_string()).collect(),
        comment_prefixes: vec![],
        string_delimiters: vec![("\"".to_string(), "\"".to_string())],
        styles: LanguageStyles {
            keyword: Style::default().fg(theme.colors.blue).add_modifier(Modifier::BOLD),
            operator: Style::default().fg(theme.colors.yellow),
            string: Style::default().fg(theme.colors.green),
            number: Style::default().fg(theme.colors.red),
            comment: theme.styles.muted,
            function: theme.styles.text,
            type_name: theme.styles.text,
            variable: theme.styles.text,
            constant: Style::default().fg(theme.colors.red).add_modifier(Modifier::BOLD),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formatter_creation() {
        let formatter = MessageFormatter::new();
        assert!(formatter.emoji_support);
        assert!(formatter.max_line_width.is_none());
    }

    #[test]
    fn test_plain_text_formatting() {
        let formatter = MessageFormatter::new();
        let options = FormatOptions::default();
        
        let result = formatter.format_plain_text("Hello, world!", &options);
        assert_eq!(result.lines.len(), 1);
        assert!(result.width > 0);
        assert_eq!(result.metadata.word_count, 2);
    }

    #[test]
    fn test_text_wrapping() {
        let formatter = MessageFormatter::new();
        let long_text = "This is a very long line that should be wrapped at a certain width";
        
        let wrapped = formatter.wrap_text(long_text, 20);
        assert!(wrapped.len() > 1);
        assert!(wrapped.iter().all(|line| line.width() <= 20));
    }

    #[test]
    fn test_metadata_calculation() {
        let formatter = MessageFormatter::new();
        let text = "# Header\n\nSome text with `code` and [link](url).\n\n```rust\nfn main() {}\n```";
        
        let metadata = formatter.calculate_metadata(text);
        assert!(metadata.has_code_blocks);
        assert!(metadata.has_links);
        assert!(metadata.word_count > 0);
        assert!(metadata.character_count > 0);
    }
}