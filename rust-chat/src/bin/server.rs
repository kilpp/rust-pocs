use std::collections::HashMap;
use std::sync::Arc;

use chrono::Local;
use colored::Colorize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, Mutex};

const ADDR: &str = "127.0.0.1:8080";
const COLORS: &[&str] = &["red", "green", "yellow", "blue", "magenta", "cyan"];

struct ChatServer {
    clients: Mutex<HashMap<String, usize>>,
    next_color: Mutex<usize>,
}

impl ChatServer {
    fn new() -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
            next_color: Mutex::new(0),
        }
    }

    async fn add_client(&self, name: &str) -> usize {
        let mut color_idx = self.next_color.lock().await;
        let idx = *color_idx;
        *color_idx = (*color_idx + 1) % COLORS.len();
        self.clients.lock().await.insert(name.to_string(), idx);
        idx
    }

    async fn remove_client(&self, name: &str) {
        self.clients.lock().await.remove(name);
    }

    async fn client_count(&self) -> usize {
        self.clients.lock().await.len()
    }

    async fn list_users(&self) -> Vec<String> {
        self.clients.lock().await.keys().cloned().collect()
    }
}

fn colorize(text: &str, color_idx: usize) -> String {
    match COLORS[color_idx % COLORS.len()] {
        "red" => text.red().to_string(),
        "green" => text.green().to_string(),
        "yellow" => text.yellow().to_string(),
        "blue" => text.blue().to_string(),
        "magenta" => text.magenta().to_string(),
        "cyan" => text.cyan().to_string(),
        _ => text.to_string(),
    }
}

fn timestamp() -> String {
    Local::now().format("%H:%M:%S").to_string()
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind(ADDR).await.expect("Failed to bind");
    println!(
        "{} Chat server running on {}",
        ">>>".green().bold(),
        ADDR.cyan().bold()
    );

    let (tx, _) = broadcast::channel::<String>(100);
    let server = Arc::new(ChatServer::new());

    loop {
        let (socket, addr) = listener.accept().await.expect("Failed to accept");
        println!(
            "{} [{}] New connection from {}",
            "-->".blue(),
            timestamp().dimmed(),
            addr.to_string().yellow()
        );

        let tx = tx.clone();
        let mut rx = tx.subscribe();
        let server = Arc::clone(&server);

        tokio::spawn(async move {
            let (reader, mut writer) = socket.into_split();
            let mut reader = BufReader::new(reader);

            // Ask for username
            let _ = writer.write_all(b"Enter your name: ").await;
            let mut name = String::new();
            if reader.read_line(&mut name).await.is_err() || name.trim().is_empty() {
                let _ = writer.write_all(b"Invalid name. Goodbye!\n").await;
                return;
            }
            let name = name.trim().to_string();
            let color_idx = server.add_client(&name).await;

            let welcome = format!(
                "Welcome, {}! ({} online)\nCommands: /users  /quit\n",
                name,
                server.client_count().await
            );
            let _ = writer.write_all(welcome.as_bytes()).await;

            // Broadcast join
            let join_msg = format!(
                "\x1b[1;32m*** {} joined the chat ({} online) ***\x1b[0m\n",
                name,
                server.client_count().await
            );
            let _ = tx.send(join_msg);
            println!(
                "{} [{}] {} joined (color: {})",
                "+++".green(),
                timestamp().dimmed(),
                colorize(&name, color_idx),
                COLORS[color_idx]
            );

            let client_name = name.clone();

            // Spawn task to forward broadcast messages to this client
            let mut write_half = writer;
            let forward_task = tokio::spawn(async move {
                loop {
                    match rx.recv().await {
                        Ok(msg) => {
                            if write_half.write_all(msg.as_bytes()).await.is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            let warn = format!("(missed {} messages)\n", n);
                            let _ = write_half.write_all(warn.as_bytes()).await;
                        }
                        Err(_) => break,
                    }
                }
            });

            // Read lines from client
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) | Err(_) => break, // Disconnected
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }

                        match trimmed {
                            "/quit" => break,
                            "/users" => {
                                let users = server.list_users().await;
                                let list = format!(
                                    "Online ({}): {}\n",
                                    users.len(),
                                    users.join(", ")
                                );
                                // Send directly via broadcast (user will see it too)
                                // Actually, we can't write to writer here since forward_task owns it.
                                // So we send a private-ish message via broadcast with a marker.
                                let private_msg =
                                    format!("\x1b[2m{}\x1b[0m", list);
                                let _ = tx.send(private_msg);
                            }
                            _ => {
                                let msg = format!(
                                    "\x1b[90m[{}]\x1b[0m {}: {}\n",
                                    timestamp(),
                                    colorize_ansi(&name, color_idx),
                                    trimmed
                                );
                                println!(
                                    "    [{}] {}: {}",
                                    timestamp().dimmed(),
                                    colorize(&name, color_idx),
                                    trimmed
                                );
                                let _ = tx.send(msg);
                            }
                        }
                    }
                }
            }

            // Cleanup
            forward_task.abort();
            server.remove_client(&name).await;
            let leave_msg = format!(
                "\x1b[1;31m*** {} left the chat ({} online) ***\x1b[0m\n",
                name,
                server.client_count().await
            );
            let _ = tx.send(leave_msg);
            println!(
                "{} [{}] {} disconnected",
                "---".red(),
                timestamp().dimmed(),
                colorize(&client_name, color_idx)
            );
        });
    }
}

/// Colorize using ANSI escape codes (for messages sent over the network)
fn colorize_ansi(text: &str, color_idx: usize) -> String {
    let code = match COLORS[color_idx % COLORS.len()] {
        "red" => "31",
        "green" => "32",
        "yellow" => "33",
        "blue" => "34",
        "magenta" => "35",
        "cyan" => "36",
        _ => "0",
    };
    format!("\x1b[1;{}m{}\x1b[0m", code, text)
}
