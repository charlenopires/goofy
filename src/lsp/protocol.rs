//! LSP protocol message handling

use crate::lsp::types::*;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader as AsyncBufReader};
use tracing::{debug, error, trace};

/// LSP protocol handler for message parsing and serialization
pub struct LspProtocol;

impl LspProtocol {
    /// Parse an LSP message from a header and content
    pub fn parse_message(header: &str, content: &str) -> Result<LspMessage> {
        trace!("Parsing LSP message with header: {}", header);
        
        let content_length = Self::extract_content_length(header)?;
        
        if content.len() != content_length {
            return Err(anyhow!(
                "Content length mismatch: expected {}, got {}",
                content_length,
                content.len()
            ));
        }

        let json_value: Value = serde_json::from_str(content)?;
        
        if let Some(id) = json_value.get("id") {
            if json_value.get("method").is_some() {
                // Request
                return Ok(LspMessage::Request {
                    id: id.as_i64().unwrap_or(0) as i32,
                    method: json_value["method"].as_str().unwrap_or("").to_string(),
                    params: json_value.get("params").cloned(),
                });
            } else {
                // Response
                return Ok(LspMessage::Response {
                    id: id.as_i64().unwrap_or(0) as i32,
                    result: json_value.get("result").cloned(),
                    error: json_value.get("error").and_then(|e| {
                        serde_json::from_value(e.clone()).ok()
                    }),
                });
            }
        } else if json_value.get("method").is_some() {
            // Notification
            return Ok(LspMessage::Notification {
                method: json_value["method"].as_str().unwrap_or("").to_string(),
                params: json_value.get("params").cloned(),
            });
        }

        Err(anyhow!("Invalid LSP message format"))
    }

    /// Serialize an LSP message to the wire format
    pub fn serialize_message(message: &LspMessage) -> Result<String> {
        let json_content = match message {
            LspMessage::Request { id, method, params } => {
                let mut obj = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "method": method
                });
                
                if let Some(params) = params {
                    obj["params"] = params.clone();
                }
                obj
            }
            LspMessage::Response { id, result, error } => {
                let mut obj = json!({
                    "jsonrpc": "2.0",
                    "id": id
                });
                
                if let Some(result) = result {
                    obj["result"] = result.clone();
                } else if let Some(error) = error {
                    obj["error"] = serde_json::to_value(error)?;
                }
                obj
            }
            LspMessage::Notification { method, params } => {
                let mut obj = json!({
                    "jsonrpc": "2.0",
                    "method": method
                });
                
                if let Some(params) = params {
                    obj["params"] = params.clone();
                }
                obj
            }
        };

        let content = serde_json::to_string(&json_content)?;
        let header = format!("Content-Length: {}\\r\\n\\r\\n", content.len());
        
        debug!("Serialized LSP message: {}{}", header, content);
        Ok(format!("{}{}", header, content))
    }

    /// Extract content length from LSP header
    fn extract_content_length(header: &str) -> Result<usize> {
        for line in header.lines() {
            if line.starts_with("Content-Length:") {
                let length_str = line
                    .strip_prefix("Content-Length:")
                    .ok_or_else(|| anyhow!("Invalid Content-Length header"))?
                    .trim();
                
                return length_str.parse::<usize>()
                    .map_err(|e| anyhow!("Failed to parse content length: {}", e));
            }
        }
        
        Err(anyhow!("Content-Length header not found"))
    }

    /// Read a complete LSP message from an async reader
    pub async fn read_message<R: AsyncRead + Unpin>(reader: &mut AsyncBufReader<R>) -> Result<LspMessage> {
        let mut header = String::new();
        
        // Read header lines until we find an empty line
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).await?;
            
            if line.trim().is_empty() {
                break;
            }
            
            header.push_str(&line);
        }

        // Extract content length
        let content_length = Self::extract_content_length(&header)?;
        
        // Read the exact amount of content
        let mut content = vec![0u8; content_length];
        reader.read_exact(&mut content).await?;
        
        let content_str = String::from_utf8(content)?;
        Self::parse_message(&header, &content_str)
    }

    /// Write an LSP message to an async writer
    pub async fn write_message<W: AsyncWrite + Unpin>(
        writer: &mut W,
        message: &LspMessage,
    ) -> Result<()> {
        let serialized = Self::serialize_message(message)?;
        writer.write_all(serialized.as_bytes()).await?;
        writer.flush().await?;
        Ok(())
    }

    /// Create an initialize request
    pub fn create_initialize_request(
        id: i32,
        root_uri: Option<String>,
        capabilities: Value,
    ) -> LspMessage {
        LspMessage::Request {
            id,
            method: methods::INITIALIZE.to_string(),
            params: Some(json!({
                "processId": std::process::id(),
                "rootUri": root_uri,
                "capabilities": capabilities,
                "clientInfo": {
                    "name": "goofy",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
        }
    }

    /// Create an initialized notification
    pub fn create_initialized_notification() -> LspMessage {
        LspMessage::Notification {
            method: methods::INITIALIZED.to_string(),
            params: Some(json!({})),
        }
    }

    /// Create a shutdown request
    pub fn create_shutdown_request(id: i32) -> LspMessage {
        LspMessage::Request {
            id,
            method: methods::SHUTDOWN.to_string(),
            params: None,
        }
    }

    /// Create an exit notification
    pub fn create_exit_notification() -> LspMessage {
        LspMessage::Notification {
            method: methods::EXIT.to_string(),
            params: None,
        }
    }

    /// Create a text document did open notification
    pub fn create_did_open_notification(
        uri: &str,
        language_id: &str,
        version: i32,
        text: &str,
    ) -> LspMessage {
        LspMessage::Notification {
            method: methods::TEXT_DOCUMENT_DID_OPEN.to_string(),
            params: Some(json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": version,
                    "text": text
                }
            })),
        }
    }

    /// Create a text document did close notification
    pub fn create_did_close_notification(uri: &str) -> LspMessage {
        LspMessage::Notification {
            method: methods::TEXT_DOCUMENT_DID_CLOSE.to_string(),
            params: Some(json!({
                "textDocument": {
                    "uri": uri
                }
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_request_message() {
        let content = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":1234}}"#;
        let header = format!("Content-Length: {}\r\n\r\n", content.len());

        let message = LspProtocol::parse_message(&header, content).unwrap();

        match message {
            LspMessage::Request { id, method, params } => {
                assert_eq!(id, 1);
                assert_eq!(method, "initialize");
                assert!(params.is_some());
            }
            _ => panic!("Expected request message"),
        }
    }

    #[test]
    fn test_serialize_request_message() {
        let message = LspMessage::Request {
            id: 1,
            method: "initialize".to_string(),
            params: Some(json!({"processId": 1234})),
        };

        let serialized = LspProtocol::serialize_message(&message).unwrap();
        assert!(serialized.contains("Content-Length:"));
        assert!(serialized.contains("initialize"));
        assert!(serialized.contains("processId"));
    }

    #[test]
    fn test_extract_content_length() {
        let header = "Content-Length: 123\r\nContent-Type: application/vscode-jsonrpc; charset=utf-8\r\n";
        let length = LspProtocol::extract_content_length(header).unwrap();
        assert_eq!(length, 123);
    }
}