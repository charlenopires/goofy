//! Title generation prompt for conversations

pub struct TitlePrompt;

impl TitlePrompt {
    /// Generate a title prompt
    pub fn generate() -> String {
        Self::base_prompt().to_string()
    }
    
    fn base_prompt() -> &'static str {
        r#"Generate a concise, descriptive title for this conversation.

## Guidelines

1. **Length**: Keep it between 3-8 words
2. **Clarity**: Make it immediately understandable
3. **Specificity**: Capture the main topic or task
4. **Style**: Use title case (capitalize major words)
5. **Focus**: Emphasize the primary subject matter

## Examples

Good titles:
- "Python Web Scraping Implementation"
- "Fix Database Connection Issues"
- "Refactor Authentication Module"
- "React Component Performance Optimization"
- "Debug Memory Leak in Node.js"

Avoid:
- Generic titles like "Code Help" or "Programming Question"
- Overly long descriptions
- Technical jargon without context
- Questions as titles

Return ONLY the title text, without quotes or additional formatting."#
    }
}