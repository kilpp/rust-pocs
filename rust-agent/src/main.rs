use colored::Colorize;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, Write};

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const MODEL: &str = "claude-sonnet-4-20250514";

// ── Claude API types ──

#[derive(Debug, Serialize)]
struct Request {
    model: String,
    max_tokens: u32,
    system: String,
    tools: Vec<Value>,
    messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    content: Value,
}

#[derive(Debug, Deserialize)]
struct Response {
    content: Vec<ContentBlock>,
    #[allow(dead_code)]
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
}

// ── Tool definitions ──

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "calculate",
            "description": "Evaluate a mathematical expression. Supports +, -, *, /, ^ and parentheses.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "The math expression to evaluate, e.g. '(2 + 3) * 4'"
                    }
                },
                "required": ["expression"]
            }
        }),
        json!({
            "name": "get_weather",
            "description": "Get the current weather for a city (simulated).",
            "input_schema": {
                "type": "object",
                "properties": {
                    "city": {
                        "type": "string",
                        "description": "City name, e.g. 'Paris'"
                    }
                },
                "required": ["city"]
            }
        }),
    ]
}

// ── Tool implementations ──

fn execute_tool(name: &str, input: &Value) -> String {
    match name {
        "calculate" => {
            let expr = input["expression"].as_str().unwrap_or("");
            match eval_expr(expr) {
                Some(result) => format!("{result}"),
                None => "Error: could not evaluate expression".into(),
            }
        }
        "get_weather" => {
            let city = input["city"].as_str().unwrap_or("Unknown");
            // Simulated weather
            let hash: u32 = city.bytes().map(|b| b as u32).sum();
            let temp = (hash % 35) as i32 + 5;
            let conditions = ["sunny", "cloudy", "rainy", "windy", "snowy"];
            let cond = conditions[(hash as usize) % conditions.len()];
            format!("{city}: {temp}°C, {cond}")
        }
        _ => format!("Unknown tool: {name}"),
    }
}

/// Simple recursive-descent math evaluator
fn eval_expr(input: &str) -> Option<f64> {
    let tokens: Vec<char> = input.chars().filter(|c| !c.is_whitespace()).collect();
    let mut pos = 0;
    let result = parse_add(&tokens, &mut pos)?;
    if pos == tokens.len() {
        Some(result)
    } else {
        None
    }
}

fn parse_add(tokens: &[char], pos: &mut usize) -> Option<f64> {
    let mut left = parse_mul(tokens, pos)?;
    while *pos < tokens.len() && matches!(tokens[*pos], '+' | '-') {
        let op = tokens[*pos];
        *pos += 1;
        let right = parse_mul(tokens, pos)?;
        left = if op == '+' { left + right } else { left - right };
    }
    Some(left)
}

fn parse_mul(tokens: &[char], pos: &mut usize) -> Option<f64> {
    let mut left = parse_pow(tokens, pos)?;
    while *pos < tokens.len() && matches!(tokens[*pos], '*' | '/') {
        let op = tokens[*pos];
        *pos += 1;
        let right = parse_pow(tokens, pos)?;
        left = if op == '*' { left * right } else { left / right };
    }
    Some(left)
}

fn parse_pow(tokens: &[char], pos: &mut usize) -> Option<f64> {
    let base = parse_atom(tokens, pos)?;
    if *pos < tokens.len() && tokens[*pos] == '^' {
        *pos += 1;
        let exp = parse_pow(tokens, pos)?;
        Some(base.powf(exp))
    } else {
        Some(base)
    }
}

fn parse_atom(tokens: &[char], pos: &mut usize) -> Option<f64> {
    if *pos >= tokens.len() {
        return None;
    }
    // Handle unary minus
    if tokens[*pos] == '-' {
        *pos += 1;
        return parse_atom(tokens, pos).map(|v| -v);
    }
    if tokens[*pos] == '(' {
        *pos += 1;
        let val = parse_add(tokens, pos)?;
        if *pos < tokens.len() && tokens[*pos] == ')' {
            *pos += 1;
        }
        return Some(val);
    }
    // Parse number
    let start = *pos;
    while *pos < tokens.len() && (tokens[*pos].is_ascii_digit() || tokens[*pos] == '.') {
        *pos += 1;
    }
    if start == *pos {
        return None;
    }
    let num_str: String = tokens[start..*pos].iter().collect();
    num_str.parse().ok()
}

// ── Agent loop ──

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key =
        std::env::var("ANTHROPIC_API_KEY").expect("Set ANTHROPIC_API_KEY environment variable");

    let client = Client::new();
    let mut messages: Vec<Message> = Vec::new();

    println!("{}", "Rust AI Agent".bold().cyan());
    println!(
        "{}",
        "Tools: calculate, get_weather (simulated)".dimmed()
    );
    println!("{}", "Type 'quit' to exit.\n".dimmed());

    loop {
        // Read user input
        print!("{}", "You > ".green().bold());
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }
        if input == "quit" || input == "exit" {
            break;
        }

        messages.push(Message {
            role: "user".into(),
            content: Value::String(input.to_string()),
        });

        // Agent loop: keep calling the API until we get a final text response
        loop {
            let request = Request {
                model: MODEL.into(),
                max_tokens: 1024,
                system: "You are a helpful assistant. Use the provided tools when needed. \
                         Be concise."
                    .into(),
                tools: tool_definitions(),
                messages: messages.clone(),
            };

            let resp = client
                .post(API_URL)
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&request)
                .send()
                .await?;

            let status = resp.status();
            if !status.is_success() {
                let body = resp.text().await?;
                eprintln!("{} {status}: {body}", "API error".red().bold());
                messages.pop();
                break;
            }

            let response: Response = resp.json().await?;

            // Build the assistant content array and collect tool calls
            let mut assistant_content: Vec<Value> = Vec::new();
            let mut tool_calls: Vec<(String, String, Value)> = Vec::new();

            for block in &response.content {
                match block {
                    ContentBlock::Text { text } => {
                        println!("\n{} {}\n", "Agent >".cyan().bold(), text);
                        assistant_content.push(json!({
                            "type": "text",
                            "text": text
                        }));
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        println!(
                            "  {} {}({})",
                            "tool:".yellow().bold(),
                            name.yellow(),
                            input.to_string().dimmed()
                        );
                        assistant_content.push(json!({
                            "type": "tool_use",
                            "id": id,
                            "name": name,
                            "input": input
                        }));
                        tool_calls.push((id.clone(), name.clone(), input.clone()));
                    }
                }
            }

            messages.push(Message {
                role: "assistant".into(),
                content: Value::Array(assistant_content),
            });

            if tool_calls.is_empty() {
                break;
            }

            // Execute tools and send results back
            let mut tool_results: Vec<Value> = Vec::new();
            for (id, name, input) in &tool_calls {
                let result = execute_tool(name, input);
                println!("  {} {}", "result:".green().bold(), result.green());
                tool_results.push(json!({
                    "type": "tool_result",
                    "tool_use_id": id,
                    "content": result
                }));
            }

            messages.push(Message {
                role: "user".into(),
                content: Value::Array(tool_results),
            });
        }
    }

    println!("{}", "Goodbye!".dimmed());
    Ok(())
}
