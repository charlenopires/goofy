//! Core markdown rendering engine
//! 
//! This module provides the main rendering logic for converting
//! markdown to ratatui Text with proper styling and layout.

use anyhow::Result;
use pulldown_cmark::{Parser, Event, Tag, TagEnd, CodeBlockKind, CowStr, HeadingLevel, Alignment};
use ratatui::{
    style::{Color, Style, Modifier},
    text::{Line, Span, Text},
};
use std::collections::HashMap;

use super::{MarkdownConfig, RenderContext, TableState, styles::MarkdownStyles};
use crate::tui::components::highlighting::{SyntaxHighlighter, HighlightConfig};

/// Core markdown renderer
pub struct MarkdownRenderer {
    config: MarkdownConfig,
    styles: MarkdownStyles,
    highlighter: SyntaxHighlighter,
}

impl MarkdownRenderer {
    /// Create a new markdown renderer
    pub fn new(config: &MarkdownConfig, styles: MarkdownStyles) -> Self {
        let highlighter = SyntaxHighlighter::with_config(config.highlight_config.clone())
            .unwrap_or_else(|_| SyntaxHighlighter::new().expect("Failed to create syntax highlighter"));

        Self {
            config: config.clone(),
            styles,
            highlighter,
        }
    }
    
    /// Render markdown content to Text
    pub fn render(&self, content: &str, width: u16) -> Result<Text<'static>> {
        let parser = Parser::new(content);
        let mut context = RenderContext {
            current_line: Vec::new(),
            lines: Vec::new(),
            indent_level: self.config.base_indent,
            list_level: 0,
            in_code_block: false,
            code_language: None,
            in_quote: false,
            table_state: None,
            styles: self.styles.clone(),
        };
        
        self.process_events(parser, &mut context, width)?;
        
        // Finalize any remaining content
        self.finalize_context(&mut context);
        
        Ok(Text::from(context.lines))
    }
    
    /// Process markdown events
    fn process_events(
        &self,
        parser: Parser,
        context: &mut RenderContext,
        width: u16,
    ) -> Result<()> {
        for event in parser {
            match event {
                Event::Start(tag) => self.handle_start_tag(tag, context, width)?,
                Event::End(tag_end) => self.handle_end_tag(tag_end, context, width)?,
                Event::Text(text) => self.handle_text(text, context)?,
                Event::Code(code) => self.handle_inline_code(code, context)?,
                Event::Html(html) => self.handle_html(html, context)?,
                Event::SoftBreak => self.handle_soft_break(context)?,
                Event::HardBreak => self.handle_hard_break(context)?,
                Event::Rule => self.handle_rule(context)?,
                Event::FootnoteReference(reference) => self.handle_footnote_reference(reference, context)?,
                Event::TaskListMarker(checked) => self.handle_task_list_marker(checked, context)?,
                Event::InlineHtml(html) => self.handle_html(html, context)?,
            }
        }
        
        Ok(())
    }
    
    /// Handle start tags
    fn handle_start_tag(&self, tag: Tag, context: &mut RenderContext, width: u16) -> Result<()> {
        match tag {
            Tag::Paragraph => {
                self.ensure_blank_line(context);
            }
            Tag::Heading { level, .. } => {
                self.start_heading(level, context);
            }
            Tag::BlockQuote => {
                self.start_blockquote(context);
            }
            Tag::CodeBlock(kind) => {
                self.start_code_block(kind, context);
            }
            Tag::List(start_num) => {
                self.start_list(start_num, context);
            }
            Tag::Item => {
                self.start_list_item(context);
            }
            Tag::Emphasis => {
                // Will be handled in text processing
            }
            Tag::Strong => {
                // Will be handled in text processing
            }
            Tag::Strikethrough => {
                // Will be handled in text processing
            }
            Tag::Link { dest_url, title, .. } => {
                self.start_link(dest_url, title, context);
            }
            Tag::Image { dest_url, title, .. } => {
                self.handle_image(dest_url, title, context)?;
            }
            Tag::Table(alignments) => {
                self.start_table(alignments, context);
            }
            Tag::TableHead => {
                self.start_table_head(context);
            }
            Tag::TableRow => {
                self.start_table_row(context);
            }
            Tag::TableCell => {
                self.start_table_cell(context);
            }
            Tag::FootnoteDefinition(label) => {
                self.start_footnote_definition(label, context);
            }
            Tag::HtmlBlock => {
                // HTML blocks are handled during text processing
            }
            Tag::MetadataBlock(_) => {
                // Metadata blocks are not rendered
            }
        }
        
        Ok(())
    }
    
    /// Handle end tags
    fn handle_end_tag(&self, tag_end: TagEnd, context: &mut RenderContext, width: u16) -> Result<()> {
        match tag_end {
            TagEnd::Paragraph => {
                self.end_paragraph(context);
            }
            TagEnd::Heading(level) => {
                self.end_heading(level, context);
            }
            TagEnd::BlockQuote => {
                self.end_blockquote(context);
            }
            TagEnd::CodeBlock => {
                self.end_code_block(context, width)?;
            }
            TagEnd::List(_) => {
                self.end_list(context);
            }
            TagEnd::Item => {
                self.end_list_item(context);
            }
            TagEnd::Emphasis => {
                // Handled in text processing
            }
            TagEnd::Strong => {
                // Handled in text processing
            }
            TagEnd::Strikethrough => {
                // Handled in text processing
            }
            TagEnd::Link => {
                self.end_link(context);
            }
            TagEnd::Image => {
                // Images are handled immediately
            }
            TagEnd::Table => {
                self.end_table(context, width)?;
            }
            TagEnd::TableHead => {
                self.end_table_head(context);
            }
            TagEnd::TableRow => {
                self.end_table_row(context);
            }
            TagEnd::TableCell => {
                self.end_table_cell(context);
            }
            TagEnd::FootnoteDefinition => {
                self.end_footnote_definition(context);
            }
            TagEnd::HtmlBlock => {
                // HTML blocks are handled during text processing
            }
            TagEnd::MetadataBlock(_) => {
                // Metadata blocks are not rendered
            }
        }
        
        Ok(())
    }
    
    /// Handle text content
    fn handle_text(&self, text: CowStr, context: &mut RenderContext) -> Result<()> {
        if context.in_code_block {
            // Accumulate code block content
            return Ok(());
        }
        
        if let Some(ref mut table_state) = context.table_state {
            table_state.current_cell.push_str(&text);
            return Ok(());
        }
        
        let style = if context.in_quote {
            self.styles.quote_text
        } else {
            self.styles.text
        };
        
        let span = Span::styled(text.to_string(), style);
        context.current_line.push(span);
        
        Ok(())
    }
    
    /// Handle inline code
    fn handle_inline_code(&self, code: CowStr, context: &mut RenderContext) -> Result<()> {
        let span = Span::styled(
            format!(" {} ", code),
            self.styles.inline_code
        );
        context.current_line.push(span);
        Ok(())
    }
    
    /// Handle HTML content
    fn handle_html(&self, html: CowStr, context: &mut RenderContext) -> Result<()> {
        // For now, just treat HTML as plain text
        let span = Span::styled(html.to_string(), self.styles.text);
        context.current_line.push(span);
        Ok(())
    }
    
    /// Handle soft break
    fn handle_soft_break(&self, context: &mut RenderContext) -> Result<()> {
        context.current_line.push(Span::raw(" "));
        Ok(())
    }
    
    /// Handle hard break
    fn handle_hard_break(&self, context: &mut RenderContext) -> Result<()> {
        self.flush_current_line(context);
        Ok(())
    }
    
    /// Handle horizontal rule
    fn handle_rule(&self, context: &mut RenderContext) -> Result<()> {
        self.flush_current_line(context);
        self.ensure_blank_line(context);
        
        let rule_line = Line::from(vec![
            Span::styled("─".repeat(80), self.styles.rule)
        ]);
        context.lines.push(rule_line);
        
        self.ensure_blank_line(context);
        Ok(())
    }
    
    /// Handle footnote reference
    fn handle_footnote_reference(&self, reference: CowStr, context: &mut RenderContext) -> Result<()> {
        let span = Span::styled(
            format!("[{}]", reference),
            self.styles.footnote_reference
        );
        context.current_line.push(span);
        Ok(())
    }
    
    /// Handle task list marker
    fn handle_task_list_marker(&self, checked: bool, context: &mut RenderContext) -> Result<()> {
        let marker = if checked { "[✓] " } else { "[ ] " };
        let span = Span::styled(marker, self.styles.task_marker);
        context.current_line.push(span);
        Ok(())
    }
    
    /// Start heading
    fn start_heading(&self, level: HeadingLevel, context: &mut RenderContext) {
        self.flush_current_line(context);
        self.ensure_blank_line(context);
        
        let prefix = match level {
            HeadingLevel::H1 => " ",
            HeadingLevel::H2 => "## ",
            HeadingLevel::H3 => "### ",
            HeadingLevel::H4 => "#### ",
            HeadingLevel::H5 => "##### ",
            HeadingLevel::H6 => "###### ",
        };
        
        let style = match level {
            HeadingLevel::H1 => self.styles.heading_1,
            HeadingLevel::H2 => self.styles.heading_2,
            HeadingLevel::H3 => self.styles.heading_3,
            HeadingLevel::H4 => self.styles.heading_4,
            HeadingLevel::H5 => self.styles.heading_5,
            HeadingLevel::H6 => self.styles.heading_6,
        };
        
        context.current_line.push(Span::styled(prefix, style));
    }
    
    /// End heading
    fn end_heading(&self, level: HeadingLevel, context: &mut RenderContext) {
        if level == HeadingLevel::H1 {
            context.current_line.push(Span::styled(" ", self.styles.heading_1));
        }
        
        self.flush_current_line(context);
        self.ensure_blank_line(context);
    }
    
    /// Start blockquote
    fn start_blockquote(&self, context: &mut RenderContext) {
        self.flush_current_line(context);
        context.in_quote = true;
        context.indent_level += self.config.quote_indent;
    }
    
    /// End blockquote
    fn end_blockquote(&self, context: &mut RenderContext) {
        self.flush_current_line(context);
        context.in_quote = false;
        context.indent_level = context.indent_level.saturating_sub(self.config.quote_indent);
    }
    
    /// Start code block
    fn start_code_block(&self, kind: CodeBlockKind, context: &mut RenderContext) {
        self.flush_current_line(context);
        self.ensure_blank_line(context);
        
        context.in_code_block = true;
        context.code_language = match kind {
            CodeBlockKind::Fenced(lang) => {
                if lang.is_empty() {
                    None
                } else {
                    Some(lang.to_string())
                }
            }
            CodeBlockKind::Indented => None,
        };
    }
    
    /// End code block
    fn end_code_block(&self, context: &mut RenderContext, width: u16) -> Result<()> {
        // Here we would collect the code block content and highlight it
        // For now, just add a placeholder
        context.in_code_block = false;
        
        let code_line = Line::from(vec![
            Span::styled("    [Code Block]", self.styles.code_block)
        ]);
        context.lines.push(code_line);
        
        self.ensure_blank_line(context);
        context.code_language = None;
        
        Ok(())
    }
    
    /// Start list
    fn start_list(&self, start_num: Option<u64>, context: &mut RenderContext) {
        self.flush_current_line(context);
        context.list_level += 1;
        context.indent_level += self.config.list_indent;
    }
    
    /// End list
    fn end_list(&self, context: &mut RenderContext) {
        self.flush_current_line(context);
        context.list_level = context.list_level.saturating_sub(1);
        context.indent_level = context.indent_level.saturating_sub(self.config.list_indent);
    }
    
    /// Start list item
    fn start_list_item(&self, context: &mut RenderContext) {
        self.flush_current_line(context);
        
        let marker = if context.list_level % 2 == 1 { "• " } else { "◦ " };
        let span = Span::styled(marker, self.styles.list_marker);
        context.current_line.push(span);
    }
    
    /// End list item
    fn end_list_item(&self, context: &mut RenderContext) {
        self.flush_current_line(context);
    }
    
    /// Start link
    fn start_link(&self, dest_url: CowStr, title: CowStr, context: &mut RenderContext) {
        // Links will be styled when text is processed
    }
    
    /// End link
    fn end_link(&self, context: &mut RenderContext) {
        // Link styling is handled during text processing
    }
    
    /// Handle image
    fn handle_image(&self, dest_url: CowStr, title: CowStr, context: &mut RenderContext) -> Result<()> {
        if !self.config.render_images {
            return Ok(());
        }
        
        let image_text = if title.is_empty() {
            format!("🖼 Image: {}", dest_url)
        } else {
            format!("🖼 {}: {}", title, dest_url)
        };
        
        let span = Span::styled(image_text, self.styles.image);
        context.current_line.push(span);
        
        Ok(())
    }
    
    /// Start table
    fn start_table(&self, alignments: Vec<Alignment>, context: &mut RenderContext) {
        if !self.config.render_tables {
            return;
        }
        
        self.flush_current_line(context);
        self.ensure_blank_line(context);
        
        context.table_state = Some(TableState {
            headers: Vec::new(),
            rows: Vec::new(),
            current_row: Vec::new(),
            current_cell: String::new(),
            in_header: false,
        });
    }
    
    /// End table
    fn end_table(&self, context: &mut RenderContext, width: u16) -> Result<()> {
        if let Some(table_state) = context.table_state.take() {
            self.render_table(table_state, context, width)?;
        }
        
        self.ensure_blank_line(context);
        Ok(())
    }
    
    /// Start table head
    fn start_table_head(&self, context: &mut RenderContext) {
        if let Some(ref mut table_state) = context.table_state {
            table_state.in_header = true;
        }
    }
    
    /// End table head
    fn end_table_head(&self, context: &mut RenderContext) {
        if let Some(ref mut table_state) = context.table_state {
            table_state.in_header = false;
        }
    }
    
    /// Start table row
    fn start_table_row(&self, context: &mut RenderContext) {
        if let Some(ref mut table_state) = context.table_state {
            table_state.current_row.clear();
        }
    }
    
    /// End table row
    fn end_table_row(&self, context: &mut RenderContext) {
        if let Some(ref mut table_state) = context.table_state {
            if table_state.in_header {
                table_state.headers = table_state.current_row.clone();
            } else {
                table_state.rows.push(table_state.current_row.clone());
            }
        }
    }
    
    /// Start table cell
    fn start_table_cell(&self, context: &mut RenderContext) {
        if let Some(ref mut table_state) = context.table_state {
            table_state.current_cell.clear();
        }
    }
    
    /// End table cell
    fn end_table_cell(&self, context: &mut RenderContext) {
        if let Some(ref mut table_state) = context.table_state {
            table_state.current_row.push(table_state.current_cell.clone());
        }
    }
    
    /// Start footnote definition
    fn start_footnote_definition(&self, label: CowStr, context: &mut RenderContext) {
        self.flush_current_line(context);
        
        let footnote_label = format!("[{}]: ", label);
        let span = Span::styled(footnote_label, self.styles.footnote_definition);
        context.current_line.push(span);
    }
    
    /// End footnote definition
    fn end_footnote_definition(&self, context: &mut RenderContext) {
        self.flush_current_line(context);
    }
    
    /// Render table
    fn render_table(&self, table_state: TableState, context: &mut RenderContext, width: u16) -> Result<()> {
        // Simple table rendering - could be enhanced with proper alignment
        
        // Render headers
        if !table_state.headers.is_empty() {
            let mut header_spans: Vec<Span> = table_state.headers
                .iter()
                .map(|header| Span::styled(format!("| {} ", header), self.styles.table_header))
                .collect();
            if !header_spans.is_empty() {
                header_spans.push(Span::styled("|", self.styles.table_header));
                context.lines.push(Line::from(header_spans));
            }
            
            // Separator row
            let separator = "|".to_string() + &"---|".repeat(table_state.headers.len());
            context.lines.push(Line::from(Span::styled(separator, self.styles.table_separator)));
        }
        
        // Render rows
        for row in table_state.rows {
            let mut row_spans: Vec<Span> = row
                .iter()
                .map(|cell| Span::styled(format!("| {} ", cell), self.styles.table_cell))
                .collect();
            if !row_spans.is_empty() {
                row_spans.push(Span::styled("|", self.styles.table_cell));
                context.lines.push(Line::from(row_spans));
            }
        }
        
        Ok(())
    }
    
    /// Ensure blank line
    fn ensure_blank_line(&self, context: &mut RenderContext) {
        if !context.lines.is_empty() {
            if let Some(last_line) = context.lines.last() {
                if !last_line.spans.is_empty() {
                    context.lines.push(Line::from(""));
                }
            }
        }
    }
    
    /// Flush current line
    fn flush_current_line(&self, context: &mut RenderContext) {
        if !context.current_line.is_empty() {
            // Add indentation
            let mut spans = Vec::new();
            if context.indent_level > 0 {
                spans.push(Span::raw(" ".repeat(context.indent_level as usize)));
            }
            
            // Add quote marker if in quote
            if context.in_quote {
                spans.push(Span::styled("│ ", context.styles.quote_marker));
            }
            
            spans.extend(context.current_line.drain(..));
            context.lines.push(Line::from(spans));
        }
    }
    
    /// Finalize context
    fn finalize_context(&self, context: &mut RenderContext) {
        self.flush_current_line(context);
    }
    
    /// End paragraph
    fn end_paragraph(&self, context: &mut RenderContext) {
        self.flush_current_line(context);
        if !context.in_quote && context.list_level == 0 {
            context.lines.push(Line::from(""));
        }
    }
}