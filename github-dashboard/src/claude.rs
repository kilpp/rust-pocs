use crate::github::Pr;

/// Max characters of each PR body included in the prompt, to keep it bounded.
const BODY_PREVIEW_LEN: usize = 280;

fn build_prompt(language: &str, prs: &[Pr]) -> String {
    let mut prompt = format!(
        "You are summarizing recent GitHub pull request activity for the language \"{language}\".\n\
         Below are the pull requests. In 3-6 sentences, explain what this work was about \
         overall — the themes, features, fixes, or areas touched. Be concrete and concise.\n\n\
         Pull requests:\n"
    );

    for pr in prs {
        let body: String = pr.body.trim().chars().take(BODY_PREVIEW_LEN).collect();
        let state = if pr.is_open() { "open" } else { "closed" };
        prompt.push_str(&format!(
            "- [{state}] {} (#{} in {})\n",
            pr.title.trim(),
            pr.number,
            pr.repo
        ));
        if !body.is_empty() {
            prompt.push_str(&format!("    {}\n", body.replace('\n', " ")));
        }
    }

    prompt
}

/// Ask the local `claude` CLI to explain what a language's PRs were about.
/// Returns the CLI's stdout, or an error string suitable for display.
pub async fn summarize(language: &str, prs: &[Pr]) -> Result<String, String> {
    if prs.is_empty() {
        return Err("no pull requests to summarize".to_string());
    }

    let prompt = build_prompt(language, prs);

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
