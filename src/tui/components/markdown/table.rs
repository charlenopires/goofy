//! Advanced table rendering for markdown
//! 
//! This module provides sophisticated table rendering capabilities
//! with proper alignment, cell wrapping, and responsive layout.

use anyhow::Result;
use pulldown_cmark::Alignment;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Row, Table, Widget},
};
use std::cmp::max;

use super::styles::MarkdownStyles;

/// Table configuration
#[derive(Debug, Clone)]
pub struct TableConfig {
    /// Maximum table width
    pub max_width: u16,
    
    /// Minimum column width
    pub min_column_width: u16,
    
    /// Maximum column width
    pub max_column_width: u16,
    
    /// Whether to show borders
    pub show_borders: bool,
    
    /// Whether to show headers
    pub show_headers: bool,
    
    /// Whether to alternate row colors
    pub alternate_rows: bool,
    
    /// Cell padding
    pub cell_padding: u16,
}

/// Table data structure
#[derive(Debug, Clone)]
pub struct TableData {
    /// Table headers
    pub headers: Vec<String>,
    
    /// Table rows
    pub rows: Vec<Vec<String>>,
    
    /// Column alignments
    pub alignments: Vec<Alignment>,
}

/// Advanced table renderer
pub struct TableRenderer {
    config: TableConfig,
    styles: MarkdownStyles,
}

impl Default for TableConfig {
    fn default() -> Self {
        Self {
            max_width: 120,
            min_column_width: 8,
            max_column_width: 40,
            show_borders: true,
            show_headers: true,
            alternate_rows: true,
            cell_padding: 1,
        }
    }
}

impl TableRenderer {
    /// Create a new table renderer
    pub fn new(config: TableConfig, styles: MarkdownStyles) -> Self {
        Self { config, styles }
    }
    
    /// Render table to lines
    pub fn render(&self, data: &TableData, width: u16) -> Result<Vec<Line<'static>>> {
        if data.rows.is_empty() && data.headers.is_empty() {
            return Ok(vec![]);
        }
        
        let column_count = self.calculate_column_count(data);
        if column_count == 0 {
            return Ok(vec![]);
        }
        
        let column_widths = self.calculate_column_widths(data, width, column_count)?;
        let mut lines = Vec::new();
        
        // Render top border
        if self.config.show_borders {
            lines.push(self.render_border_line(&column_widths, BorderType::Top));
        }
        
        // Render headers
        if self.config.show_headers && !data.headers.is_empty() {
            let header_lines = self.render_header_row(&data.headers, &column_widths, &data.alignments)?;
            lines.extend(header_lines);
            
            // Header separator
            if self.config.show_borders {
                lines.push(self.render_border_line(&column_widths, BorderType::HeaderSeparator));
            }
        }
        
        // Render data rows
        for (index, row) in data.rows.iter().enumerate() {
            let is_alternate = self.config.alternate_rows && index % 2 == 1;
            let row_lines = self.render_data_row(row, &column_widths, &data.alignments, is_alternate)?;
            lines.extend(row_lines);
            
            // Row separator (optional)
            if self.config.show_borders && index < data.rows.len() - 1 {
                // lines.push(self.render_border_line(&column_widths, BorderType::RowSeparator));
            }
        }
        
        // Render bottom border
        if self.config.show_borders {
            lines.push(self.render_border_line(&column_widths, BorderType::Bottom));
        }
        
        Ok(lines)
    }
    
    /// Calculate the number of columns
    fn calculate_column_count(&self, data: &TableData) -> usize {
        let header_count = data.headers.len();
        let max_row_count = data.rows.iter()
            .map(|row| row.len())
            .max()
            .unwrap_or(0);
        
        max(header_count, max_row_count)
    }
    
    /// Calculate column widths
    fn calculate_column_widths(&self, data: &TableData, total_width: u16, column_count: usize) -> Result<Vec<u16>> {
        if column_count == 0 {
            return Ok(vec![]);
        }
        
        // Calculate content width for each column
        let mut content_widths = vec![0u16; column_count];
        
        // Check header widths
        for (i, header) in data.headers.iter().enumerate() {
            if i < column_count {
                content_widths[i] = max(content_widths[i], header.chars().count() as u16);
            }
        }
        
        // Check row widths
        for row in &data.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < column_count {
                    content_widths[i] = max(content_widths[i], cell.chars().count() as u16);
                }
            }
        }
        
        // Apply padding
        for width in &mut content_widths {
            *width += self.config.cell_padding * 2;
        }
        
        // Enforce minimum and maximum widths
        for width in &mut content_widths {
            *width = (*width).clamp(self.config.min_column_width, self.config.max_column_width);
        }
        
        // Calculate available width (accounting for borders)
        let border_width = if self.config.show_borders {
            (column_count + 1) as u16 // Vertical borders
        } else {
            0
        };
        
        let available_width = total_width.saturating_sub(border_width);
        let total_content_width: u16 = content_widths.iter().sum();
        
        // Adjust widths if they exceed available space
        if total_content_width > available_width {
            let scale_factor = available_width as f64 / total_content_width as f64;
            
            for width in &mut content_widths {
                *width = ((*width as f64 * scale_factor) as u16).max(self.config.min_column_width);
            }
        }
        
        Ok(content_widths)
    }
    
    /// Render header row
    fn render_header_row(
        &self,
        headers: &[String],
        column_widths: &[u16],
        alignments: &[Alignment],
    ) -> Result<Vec<Line<'static>>> {
        let mut lines = Vec::new();
        
        // For now, simple single-line headers
        let mut spans = Vec::new();
        
        if self.config.show_borders {
            spans.push(Span::styled("│", self.styles.table_separator));
        }
        
        for (i, header) in headers.iter().enumerate() {
            if i < column_widths.len() {
                let width = column_widths[i];
                let alignment = alignments.get(i).unwrap_or(&Alignment::Left);
                let content = self.format_cell_content(header, width, *alignment);
                
                spans.push(Span::styled(content, self.styles.table_header));
                
                if self.config.show_borders {
                    spans.push(Span::styled("│", self.styles.table_separator));
                }
            }
        }
        
        // Fill remaining columns if needed
        for i in headers.len()..column_widths.len() {
            let width = column_widths[i];
            let content = " ".repeat(width as usize);
            spans.push(Span::styled(content, self.styles.table_header));
            
            if self.config.show_borders {
                spans.push(Span::styled("│", self.styles.table_separator));
            }
        }
        
        lines.push(Line::from(spans));
        Ok(lines)
    }
    
    /// Render data row
    fn render_data_row(
        &self,
        row: &[String],
        column_widths: &[u16],
        alignments: &[Alignment],
        is_alternate: bool,
    ) -> Result<Vec<Line<'static>>> {
        let mut lines = Vec::new();
        
        // For now, simple single-line rows
        let mut spans = Vec::new();
        
        if self.config.show_borders {
            spans.push(Span::styled("│", self.styles.table_separator));
        }
        
        let cell_style = if is_alternate {
            self.styles.table_cell // Could add alternate styling here
        } else {
            self.styles.table_cell
        };
        
        for (i, cell) in row.iter().enumerate() {
            if i < column_widths.len() {
                let width = column_widths[i];
                let alignment = alignments.get(i).unwrap_or(&Alignment::Left);
                let content = self.format_cell_content(cell, width, *alignment);
                
                spans.push(Span::styled(content, cell_style));
                
                if self.config.show_borders {
                    spans.push(Span::styled("│", self.styles.table_separator));
                }
            }
        }
        
        // Fill remaining columns if needed
        for i in row.len()..column_widths.len() {
            let width = column_widths[i];
            let content = " ".repeat(width as usize);
            spans.push(Span::styled(content, cell_style));
            
            if self.config.show_borders {
                spans.push(Span::styled("│", self.styles.table_separator));
            }
        }
        
        lines.push(Line::from(spans));
        Ok(lines)
    }
    
    /// Format cell content with alignment and padding
    fn format_cell_content(&self, content: &str, width: u16, alignment: Alignment) -> String {
        let content_width = content.chars().count();
        let available_width = width.saturating_sub(self.config.cell_padding * 2) as usize;
        
        // Truncate content if too long
        let truncated_content = if content_width > available_width {
            let mut truncated = content.chars().take(available_width.saturating_sub(3)).collect::<String>();
            if available_width > 3 {
                truncated.push_str("...");
            }
            truncated
        } else {
            content.to_string()
        };
        
        let truncated_width = truncated_content.chars().count();
        let padding_needed = available_width.saturating_sub(truncated_width);
        
        let formatted = match alignment {
            Alignment::Left => {
                format!(
                    "{}{}{}",
                    " ".repeat(self.config.cell_padding as usize),
                    truncated_content,
                    " ".repeat(padding_needed + self.config.cell_padding as usize)
                )
            }
            Alignment::Right => {
                format!(
                    "{}{}{}",
                    " ".repeat(self.config.cell_padding as usize + padding_needed),
                    truncated_content,
                    " ".repeat(self.config.cell_padding as usize)
                )
            }
            Alignment::Center => {
                let left_padding = padding_needed / 2;
                let right_padding = padding_needed - left_padding;
                format!(
                    "{}{}{}",
                    " ".repeat(self.config.cell_padding as usize + left_padding),
                    truncated_content,
                    " ".repeat(right_padding + self.config.cell_padding as usize)
                )
            }
            Alignment::None => {
                // Default to left alignment
                format!(
                    "{}{}{}",
                    " ".repeat(self.config.cell_padding as usize),
                    truncated_content,
                    " ".repeat(padding_needed + self.config.cell_padding as usize)
                )
            }
        };
        
        formatted
    }
    
    /// Render border line
    fn render_border_line(&self, column_widths: &[u16], border_type: BorderType) -> Line<'static> {
        let mut content = String::new();
        
        match border_type {
            BorderType::Top => {
                content.push('┌');
                for (i, &width) in column_widths.iter().enumerate() {
                    content.push_str(&"─".repeat(width as usize));
                    if i < column_widths.len() - 1 {
                        content.push('┬');
                    }
                }
                content.push('┐');
            }
            BorderType::HeaderSeparator => {
                content.push('├');
                for (i, &width) in column_widths.iter().enumerate() {
                    content.push_str(&"─".repeat(width as usize));
                    if i < column_widths.len() - 1 {
                        content.push('┼');
                    }
                }
                content.push('┤');
            }
            BorderType::RowSeparator => {
                content.push('├');
                for (i, &width) in column_widths.iter().enumerate() {
                    content.push_str(&"─".repeat(width as usize));
                    if i < column_widths.len() - 1 {
                        content.push('┼');
                    }
                }
                content.push('┤');
            }
            BorderType::Bottom => {
                content.push('└');
                for (i, &width) in column_widths.iter().enumerate() {
                    content.push_str(&"─".repeat(width as usize));
                    if i < column_widths.len() - 1 {
                        content.push('┴');
                    }
                }
                content.push('┘');
            }
        }
        
        Line::from(Span::styled(content, self.styles.table_separator))
    }
}

/// Border type for table rendering
#[derive(Debug, Clone, Copy)]
enum BorderType {
    Top,
    HeaderSeparator,
    RowSeparator,
    Bottom,
}

/// Utility functions for table processing
pub mod utils {
    use super::*;
    
    /// Parse simple markdown table
    pub fn parse_markdown_table(content: &str) -> Option<TableData> {
        let lines: Vec<&str> = content.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect();
        
        if lines.len() < 2 {
            return None;
        }
        
        // Parse header row
        let header_line = lines[0];
        if !header_line.starts_with('|') || !header_line.ends_with('|') {
            return None;
        }
        
        let headers: Vec<String> = header_line[1..header_line.len()-1]
            .split('|')
            .map(|cell| cell.trim().to_string())
            .collect();
        
        // Parse separator row to determine alignments
        let separator_line = lines.get(1)?;
        if !separator_line.contains("---") {
            return None;
        }
        
        let alignments: Vec<Alignment> = separator_line[1..separator_line.len()-1]
            .split('|')
            .map(|cell| {
                let trimmed = cell.trim();
                if trimmed.starts_with(':') && trimmed.ends_with(':') {
                    Alignment::Center
                } else if trimmed.ends_with(':') {
                    Alignment::Right
                } else {
                    Alignment::Left
                }
            })
            .collect();
        
        // Parse data rows
        let mut rows = Vec::new();
        for line in lines.iter().skip(2) {
            if line.starts_with('|') && line.ends_with('|') {
                let row: Vec<String> = line[1..line.len()-1]
                    .split('|')
                    .map(|cell| cell.trim().to_string())
                    .collect();
                rows.push(row);
            }
        }
        
        Some(TableData {
            headers,
            rows,
            alignments,
        })
    }
    
    /// Convert table data to simple text representation
    pub fn table_to_text(data: &TableData) -> String {
        let mut result = String::new();
        
        // Headers
        if !data.headers.is_empty() {
            result.push_str(&data.headers.join(" | "));
            result.push('\n');
            
            // Separator
            let separator: Vec<String> = data.alignments.iter()
                .map(|alignment| match alignment {
                    Alignment::Center => ":---:",
                    Alignment::Right => "---:",
                    _ => "---",
                })
                .map(|s| s.to_string())
                .collect();
            result.push_str(&separator.join(" | "));
            result.push('\n');
        }
        
        // Rows
        for row in &data.rows {
            result.push_str(&row.join(" | "));
            result.push('\n');
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_table_config() {
        let config = TableConfig::default();
        assert_eq!(config.max_width, 120);
        assert_eq!(config.min_column_width, 8);
        assert!(config.show_borders);
    }
    
    #[test]
    fn test_cell_formatting() {
        let config = TableConfig::default();
        let styles = MarkdownStyles::default();
        let renderer = TableRenderer::new(config, styles);
        
        let content = renderer.format_cell_content("test", 10, Alignment::Left);
        assert!(content.starts_with(' '));
        assert!(content.ends_with(' '));
        
        let centered = renderer.format_cell_content("test", 10, Alignment::Center);
        assert!(centered.contains("test"));
        
        let right_aligned = renderer.format_cell_content("test", 10, Alignment::Right);
        assert!(right_aligned.ends_with("test "));
    }
    
    #[test]
    fn test_column_width_calculation() {
        let config = TableConfig::default();
        let min_column_width = config.min_column_width;
        let styles = MarkdownStyles::default();
        let renderer = TableRenderer::new(config, styles);

        let data = TableData {
            headers: vec!["Short".to_string(), "Very Long Header".to_string()],
            rows: vec![
                vec!["A".to_string(), "B".to_string()],
                vec!["Long Content".to_string(), "X".to_string()],
            ],
            alignments: vec![Alignment::Left, Alignment::Left],
        };

        let widths = renderer.calculate_column_widths(&data, 80, 2).unwrap();
        assert_eq!(widths.len(), 2);
        assert!(widths[0] >= min_column_width);
        assert!(widths[1] >= min_column_width);
    }
    
    #[test]
    fn test_markdown_table_parsing() {
        let markdown = r#"
| Name | Age | City |
|------|-----|------|
| John | 25  | NYC  |
| Jane | 30  | LA   |
"#;
        
        let table = utils::parse_markdown_table(markdown).unwrap();
        assert_eq!(table.headers, vec!["Name", "Age", "City"]);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0], vec!["John", "25", "NYC"]);
    }
    
    #[test]
    fn test_alignment_parsing() {
        let markdown = r#"
| Left | Center | Right |
|:-----|:------:|------:|
| A    | B      | C     |
"#;
        
        let table = utils::parse_markdown_table(markdown).unwrap();
        assert_eq!(table.alignments[0], Alignment::Left);
        assert_eq!(table.alignments[1], Alignment::Center);
        assert_eq!(table.alignments[2], Alignment::Right);
    }
}