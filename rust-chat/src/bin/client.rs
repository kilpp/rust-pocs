use std::io::{self, Write};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

const ADDR: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() {
    println!("\x1b[1;36m=== Rust Chat Client ===\x1b[0m");
    println!("Connecting to {}...", ADDR);

    let stream = match TcpStream::connect(ADDR).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("\x1b[1;31mFailed to connect: {}\x1b[0m", e);
            return;
        }
    };

    println!("\x1b[1;32mConnected!\x1b[0m\n");

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Spawn task to read from server and print to stdout
    let recv_task = tokio::spawn(async move {
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    println!("\n\x1b[1;31mServer disconnected.\x1b[0m");
                    break;
                }
                Ok(_) => {
                    // Move cursor to beginning of line, clear it, print message, then reprint prompt
                    print!("\r\x1b[K{}", line);
                    io::stdout().flush().ok();
                }
                Err(e) => {
                    eprintln!("\n\x1b[1;31mRead error: {}\x1b[0m", e);
                    break;
                }
            }
        }
    });

    // Read stdin and send to server
    let stdin_task = tokio::spawn(async move {
        let stdin = io::stdin();
        let mut input = String::new();
        loop {
            input.clear();
            match stdin.read_line(&mut input) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let trimmed = input.trim();
                    if trimmed == "/quit" {
                        let _ = writer.write_all(b"/quit\n").await;
                        break;
                    }
                    if writer.write_all(input.as_bytes()).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = recv_task => {}
        _ = stdin_task => {}
    }

    println!("\x1b[2mDisconnected. Bye!\x1b[0m");
}
