use crate::github::Pr;

/// Max characters of the PR body included in the prompt, to keep it bounded.
const BODY_PREVIEW_LEN: usize = 1200;

fn build_prompt(pr: &Pr) -> String {
    let state = if pr.is_open() { "open" } else { "closed" };
    let body: String = pr.body.trim().chars().take(BODY_PREVIEW_LEN).collect();

    let mut prompt = format!(
        "You are summarizing a single GitHub pull request.\n\
         In 2-4 sentences, explain what this pull request does — the change, its purpose, \
         and the area it touches. Be concrete and concise.\n\n\
         Title: {}\n\
         Repository: {}\n\
         State: {state}\n\
         Number: #{}\n",
        pr.title.trim(),
        pr.repo,
        pr.number,
    );

    if !body.is_empty() {
        prompt.push_str(&format!("\nDescription:\n{}\n", body.replace('\n', " ")));
    }

    prompt
}

/// Ask the local `claude` CLI to explain what a single PR is about.
/// Returns the CLI's stdout, or an error string suitable for display.
pub async fn summarize(pr: &Pr) -> Result<String, String> {
    let prompt = build_prompt(pr);

    let output = tokio::process::Command::new("claude")
        .arg("-p")
        .arg(&prompt)
        .output()
        .await
        .map_err(|e| format!("could not run `claude` (is the CLI on your PATH?): {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("claude exited with {}: {}", output.status, stderr.trim()));
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        return Err("claude returned an empty response".to_string());
    }
    Ok(text)
}
