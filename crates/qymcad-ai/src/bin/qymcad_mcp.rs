//! Standalone Model Context Protocol (MCP) JSON-RPC 2.0 server over stdio.

use qymcad_ai::McpServer;
use std::io::{self, BufRead, Write};

const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MB guard limit

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut server = McpServer::new();

    eprintln!("qymcad-mcp: Model Context Protocol server starting on stdio");

    let mut reader = stdin.lock();
    let mut buffer = String::new();

    loop {
        buffer.clear();
        match reader.read_line(&mut buffer) {
            Ok(0) => {
                // EOF reached - clean client disconnect
                eprintln!("qymcad-mcp: standard input closed, exiting cleanly");
                break;
            }
            Ok(n) => {
                if n > MAX_MESSAGE_SIZE {
                    eprintln!("qymcad-mcp: incoming message exceeded maximum size of {MAX_MESSAGE_SIZE} bytes");
                    let err_res = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": serde_json::Value::Null,
                        "error": {
                            "code": -32600,
                            "message": "Invalid Request: payload exceeds maximum allowable size"
                        }
                    });
                    writeln!(stdout, "{}", err_res)?;
                    stdout.flush()?;
                    continue;
                }

                let trimmed = buffer.trim();
                if trimmed.is_empty() {
                    continue;
                }

                if let Some(response_str) = server.handle_message(trimmed) {
                    writeln!(stdout, "{}", response_str)?;
                    stdout.flush()?;
                }
            }
            Err(e) => {
                eprintln!("qymcad-mcp: error reading line from stdin: {e}");
                // Break on fatal stream errors, but flush stdout first
                break;
            }
        }
    }

    Ok(())
}
