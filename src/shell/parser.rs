//! Command parser for shell input
//!
//! This module provides parsing capabilities for shell commands,
//! handling pipes, redirections, and command substitution.

use anyhow::{Result, anyhow};
use std::collections::HashMap;

/// Parsed command structure
#[derive(Debug, Clone)]
pub struct ParsedCommand {
    pub command: String,
    pub args: Vec<String>,
    pub env_vars: HashMap<String, String>,
    pub input_redirect: Option<String>,
    pub output_redirect: Option<String>,
    pub append_redirect: Option<String>,
    pub pipe_to: Option<Box<ParsedCommand>>,
    pub background: bool,
}

impl ParsedCommand {
    /// Create a new parsed command
    pub fn new(command: String) -> Self {
        Self {
            command,
            args: Vec::new(),
            env_vars: HashMap::new(),
            input_redirect: None,
            output_redirect: None,
            append_redirect: None,
            pipe_to: None,
            background: false,
        }
    }
}

/// Command parser
pub struct CommandParser;

impl CommandParser {
    /// Parse a command line into structured components
    pub fn parse(input: &str) -> Result<ParsedCommand> {
        let input = input.trim();
        
        if input.is_empty() {
            return Err(anyhow!("Empty command"));
        }
        
        // Check for background execution
        let (input, background) = if input.ends_with('&') {
            (input[..input.len()-1].trim(), true)
        } else {
            (input, false)
        };
        
        // Parse pipes
        if let Some(pipe_pos) = input.find('|') {
            let (left, right) = input.split_at(pipe_pos);
            let right = &right[1..]; // Skip the pipe character
            
            let mut left_cmd = Self::parse_simple_command(left)?;
            let right_cmd = Self::parse(right)?;
            
            left_cmd.pipe_to = Some(Box::new(right_cmd));
            left_cmd.background = background;
            
            return Ok(left_cmd);
        }
        
        // Parse simple command
        let mut cmd = Self::parse_simple_command(input)?;
        cmd.background = background;
        Ok(cmd)
    }
    
    /// Parse a simple command (no pipes)
    fn parse_simple_command(input: &str) -> Result<ParsedCommand> {
        let tokens = Self::tokenize(input)?;
        
        if tokens.is_empty() {
            return Err(anyhow!("Empty command"));
        }
        
        let mut cmd = ParsedCommand::new(String::new());
        let mut current_tokens = Vec::new();
        let mut i = 0;
        
        // Parse environment variables
        while i < tokens.len() {
            if let Some(eq_pos) = tokens[i].find('=') {
                if eq_pos > 0 && !tokens[i][..eq_pos].contains(' ') {
                    let (key, value) = tokens[i].split_at(eq_pos);
                    let value = &value[1..]; // Skip '='
                    cmd.env_vars.insert(key.to_string(), value.to_string());
                    i += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        
        // Parse command and arguments
        while i < tokens.len() {
            let token = &tokens[i];
            
            if token == "<" {
                // Input redirection
                i += 1;
                if i >= tokens.len() {
                    return Err(anyhow!("Missing input file after '<'"));
                }
                cmd.input_redirect = Some(tokens[i].clone());
            } else if token == ">" {
                // Output redirection
                i += 1;
                if i >= tokens.len() {
                    return Err(anyhow!("Missing output file after '>'"));
                }
                cmd.output_redirect = Some(tokens[i].clone());
            } else if token == ">>" {
                // Append redirection
                i += 1;
                if i >= tokens.len() {
                    return Err(anyhow!("Missing output file after '>>'"));
                }
                cmd.append_redirect = Some(tokens[i].clone());
            } else {
                current_tokens.push(token.clone());
            }
            
            i += 1;
        }
        
        if current_tokens.is_empty() {
            return Err(anyhow!("No command specified"));
        }
        
        cmd.command = current_tokens[0].clone();
        cmd.args = current_tokens[1..].to_vec();
        
        Ok(cmd)
    }
    
    /// Tokenize input string
    fn tokenize(input: &str) -> Result<Vec<String>> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut escape_next = false;
        
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;
        
        while i < chars.len() {
            let ch = chars[i];
            
            if escape_next {
                current.push(ch);
                escape_next = false;
                i += 1;
                continue;
            }
            
            if ch == '\\' && !in_single_quote {
                escape_next = true;
                i += 1;
                continue;
            }
            
            if ch == '\'' && !in_double_quote {
                in_single_quote = !in_single_quote;
                i += 1;
                continue;
            }
            
            if ch == '"' && !in_single_quote {
                in_double_quote = !in_double_quote;
                i += 1;
                continue;
            }
            
            if !in_single_quote && !in_double_quote {
                if ch.is_whitespace() {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                } else if ch == '<' || ch == '>' {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    
                    // Check for >>
                    if ch == '>' && i + 1 < chars.len() && chars[i + 1] == '>' {
                        tokens.push(">>".to_string());
                        i += 1;
                    } else {
                        tokens.push(ch.to_string());
                    }
                } else {
                    current.push(ch);
                }
            } else {
                current.push(ch);
            }
            
            i += 1;
        }
        
        if in_single_quote || in_double_quote {
            return Err(anyhow!("Unclosed quote"));
        }
        
        if !current.is_empty() {
            tokens.push(current);
        }
        
        Ok(tokens)
    }
    
    /// Expand environment variables in a string
    pub fn expand_env_vars(input: &str, env: &HashMap<String, String>) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == '$' {
                if let Some(&'{') = chars.peek() {
                    // ${VAR} format
                    chars.next(); // Skip '{'
                    let mut var_name = String::new();
                    
                    while let Some(ch) = chars.next() {
                        if ch == '}' {
                            break;
                        }
                        var_name.push(ch);
                    }
                    
                    if let Some(value) = env.get(&var_name) {
                        result.push_str(value);
                    }
                } else {
                    // $VAR format
                    let mut var_name = String::new();
                    
                    while let Some(&ch) = chars.peek() {
                        if ch.is_alphanumeric() || ch == '_' {
                            var_name.push(ch);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    
                    if let Some(value) = env.get(&var_name) {
                        result.push_str(value);
                    }
                }
            } else {
                result.push(ch);
            }
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_command() {
        let cmd = CommandParser::parse("ls -la").unwrap();
        assert_eq!(cmd.command, "ls");
        assert_eq!(cmd.args, vec!["-la"]);
        assert!(!cmd.background);
    }
    
    #[test]
    fn test_command_with_quotes() {
        let cmd = CommandParser::parse("echo 'hello world'").unwrap();
        assert_eq!(cmd.command, "echo");
        assert_eq!(cmd.args, vec!["hello world"]);
    }
    
    #[test]
    fn test_input_redirection() {
        let cmd = CommandParser::parse("cat < input.txt").unwrap();
        assert_eq!(cmd.command, "cat");
        assert_eq!(cmd.input_redirect, Some("input.txt".to_string()));
    }
    
    #[test]
    fn test_output_redirection() {
        let cmd = CommandParser::parse("echo hello > output.txt").unwrap();
        assert_eq!(cmd.command, "echo");
        assert_eq!(cmd.args, vec!["hello"]);
        assert_eq!(cmd.output_redirect, Some("output.txt".to_string()));
    }
    
    #[test]
    fn test_pipe() {
        let cmd = CommandParser::parse("ls | grep test").unwrap();
        assert_eq!(cmd.command, "ls");
        assert!(cmd.pipe_to.is_some());
        
        let piped = cmd.pipe_to.unwrap();
        assert_eq!(piped.command, "grep");
        assert_eq!(piped.args, vec!["test"]);
    }
    
    #[test]
    fn test_env_vars() {
        let cmd = CommandParser::parse("FOO=bar BAZ=qux echo hello").unwrap();
        assert_eq!(cmd.command, "echo");
        assert_eq!(cmd.args, vec!["hello"]);
        assert_eq!(cmd.env_vars.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(cmd.env_vars.get("BAZ"), Some(&"qux".to_string()));
    }
    
    #[test]
    fn test_background() {
        let cmd = CommandParser::parse("sleep 10 &").unwrap();
        assert_eq!(cmd.command, "sleep");
        assert_eq!(cmd.args, vec!["10"]);
        assert!(cmd.background);
    }
}