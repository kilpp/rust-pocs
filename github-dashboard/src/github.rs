use std::sync::Arc;

use chrono::{Duration, Utc};
use reqwest::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
use serde::Deserialize;
use tokio::sync::Semaphore;

/// Maximum number of file-listing requests in flight at once.
const MAX_CONCURRENT_FILE_FETCHES: usize = 8;
/// Safety cap on pagination so a huge account can't loop forever.
const MAX_PAGES: u32 = 10;

#[derive(Debug, Clone)]
pub struct Pr {
    pub number: u64,
    pub title: String,
    pub body: String,
    /// "open" or "closed".
    pub state: String,
    /// "owner/name".
    pub repo: String,
    /// Configured users matched by the `involves:` search for this PR.
    pub involved_users: Vec<String>,
}

impl Pr {
    pub fn is_open(&self) -> bool {
        self.state == "open"
    }
}

/// Build the browser URL for a PR, deriving the web host from the API base URL.
/// Public:     https://api.github.com  -> https://github.com/owner/name/pull/N
/// Enterprise: https://host/api/v3     -> https://host/owner/name/pull/N
pub fn pr_web_url(base_url: &str, repo: &str, number: u64) -> String {
    let host = if base_url == "https://api.github.com" {
        "https://github.com".to_string()
    } else if let Some(host) = base_url.strip_suffix("/api/v3") {
        host.to_string()
    } else {
        base_url.to_string()
    };
    format!("{host}/{repo}/pull/{number}")
}

/// Build a reqwest client with the headers GitHub expects on every request.
pub fn build_client(token: &str) -> Result<Client, String> {
    let mut headers = HeaderMap::new();
    let mut auth = HeaderValue::from_str(&format!("Bearer {token}"))
        .map_err(|e| format!("invalid token: {e}"))?;
    auth.set_sensitive(true);
    headers.insert(AUTHORIZATION, auth);
    headers.insert(USER_AGENT, HeaderValue::from_static("github-dashboard"));
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/vnd.github+json"),
    );

    Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))
}

// --- Search API ---------------------------------------------------------------

#[derive(Deserialize)]
struct SearchResponse {
    items: Vec<SearchItem>,
}

#[derive(Deserialize)]
struct SearchItem {
    id: u64,
    number: u64,
    #[serde(default)]
    title: String,
    #[serde(default)]
    body: Option<String>,
    state: String,
    repository_url: String,
}

fn repo_from_url(repository_url: &str) -> String {
    // .../repos/owner/name  ->  owner/name
    repository_url
        .rsplit("/repos/")
        .next()
        .unwrap_or(repository_url)
        .to_string()
}

/// Search for PRs that each user authored or was involved in, since `days` ago.
/// Results are de-duplicated by PR id (involves: overlaps between users).
pub async fn fetch_prs(
    client: &Client,
    base_url: &str,
    users: &[String],
    days: u32,
) -> Result<Vec<Pr>, String> {
    let since = (Utc::now() - Duration::days(days as i64))
        .format("%Y-%m-%d")
        .to_string();

    let mut prs: Vec<Pr> = Vec::new();
    // PR id -> index into `prs`, so a PR involving several users is recorded once.
    let mut id_to_index: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();

    for user in users {
        let query = format!("involves:{user} type:pr created:>={since}");
        let mut page = 1u32;

        loop {
            let url = format!("{base_url}/search/issues");
            let resp = client
                .get(&url)
                .query(&[
                    ("q", query.as_str()),
                    ("per_page", "100"),
                    ("page", &page.to_string()),
                ])
                .send()
                .await
                .map_err(|e| format!("search request failed for {user}: {e}"))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(format!("search for {user} returned {status}: {body}"));
            }

            let parsed: SearchResponse = resp
                .json()
                .await
                .map_err(|e| format!("could not parse search response for {user}: {e}"))?;

            let count = parsed.items.len();
            for item in parsed.items {
                match id_to_index.get(&item.id) {
                    Some(&idx) => {
                        // Already seen via another user's search — note this user too.
                        let involved = &mut prs[idx].involved_users;
                        if !involved.iter().any(|u| u == user) {
                            involved.push(user.clone());
                        }
                    }
                    None => {
                        id_to_index.insert(item.id, prs.len());
                        prs.push(Pr {
                            number: item.number,
                            title: item.title,
                            body: item.body.unwrap_or_default(),
                            state: item.state,
                            repo: repo_from_url(&item.repository_url),
                            involved_users: vec![user.clone()],
                        });
                    }
                }
            }

            if count < 100 || page >= MAX_PAGES {
                break;
            }
            page += 1;
        }
    }

    Ok(prs)
}

// --- PR files -----------------------------------------------------------------

#[derive(Deserialize)]
struct PrFile {
    filename: String,
}

/// Fetch the list of changed filenames for a single PR.
async fn fetch_pr_files(client: &Client, base_url: &str, pr: &Pr) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    let mut page = 1u32;

    loop {
        let url = format!("{base_url}/repos/{}/pulls/{}/files", pr.repo, pr.number);
        let resp = client
            .get(&url)
            .query(&[("per_page", "100"), ("page", &page.to_string())])
            .send()
            .await
            .map_err(|e| format!("files request failed for {}#{}: {e}", pr.repo, pr.number))?;

        if !resp.status().is_success() {
            // A single inaccessible PR shouldn't sink the whole dashboard.
            return Ok(files);
        }

        let parsed: Vec<PrFile> = resp.json().await.unwrap_or_default();
        let count = parsed.len();
        files.extend(parsed.into_iter().map(|f| f.filename));

        if count < 100 || page >= MAX_PAGES {
            break;
        }
        page += 1;
    }

    Ok(files)
}

/// High-level fetch: find the PRs, then fetch each PR's changed files
/// concurrently (bounded). Returns each PR paired with its filenames.
pub async fn fetch_prs_with_files(
    client: &Client,
    base_url: &str,
    users: &[String],
    days: u32,
) -> Result<Vec<(Pr, Vec<String>)>, String> {
    let prs = fetch_prs(client, base_url, users, days).await?;

    let client = Arc::new(client.clone());
    let base_url = Arc::new(base_url.to_string());
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_FILE_FETCHES));
    let mut set = tokio::task::JoinSet::new();

    for pr in prs {
        let client = Arc::clone(&client);
        let base_url = Arc::clone(&base_url);
        let semaphore = Arc::clone(&semaphore);
        set.spawn(async move {
            let _permit = semaphore.acquire().await.expect("semaphore not closed");
            let files = fetch_pr_files(&client, &base_url, &pr)
                .await
                .unwrap_or_default();
            (pr, files)
        });
    }

    let mut results = Vec::new();
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok(pair) => results.push(pair),
            Err(e) => return Err(format!("file fetch task failed: {e}")),
        }
    }

    Ok(results)
}

// --- Contributions (GraphQL) --------------------------------------------------

/// GitHub-style contribution totals for a single user over the window.
#[derive(Debug, Clone)]
pub struct Contributions {
    pub user: String,
    pub commits: u64,
    pub prs: u64,
    pub reviews: u64,
    pub issues: u64,
    /// Contributions to private repos not otherwise itemized.
    pub private: u64,
    /// Canonical total shown on the GitHub profile graph.
    pub total: u64,
    /// Per-day contribution counts within the window (chronological).
    pub daily: Vec<u64>,
}

const CONTRIB_QUERY: &str = r#"
query($login:String!,$from:DateTime!,$to:DateTime!){
  user(login:$login){
    contributionsCollection(from:$from,to:$to){
      totalCommitContributions
      totalPullRequestContributions
      totalPullRequestReviewContributions
      totalIssueContributions
      restrictedContributionsCount
      contributionCalendar{
        totalContributions
        weeks{ contributionDays{ date contributionCount } }
      }
    }
  }
}"#;

/// Derive the GraphQL endpoint from the REST base URL.
/// Public:     https://api.github.com        -> https://api.github.com/graphql
/// Enterprise: https://host/api/v3           -> https://host/api/graphql
fn graphql_url(base_url: &str) -> String {
    if let Some(host) = base_url.strip_suffix("/api/v3") {
        format!("{host}/api/graphql")
    } else {
        format!("{base_url}/graphql")
    }
}

#[derive(Deserialize)]
struct GqlResponse {
    data: Option<GqlData>,
    errors: Option<Vec<GqlError>>,
}

#[derive(Deserialize)]
struct GqlError {
    message: String,
}

#[derive(Deserialize)]
struct GqlData {
    user: Option<GqlUser>,
}

#[derive(Deserialize)]
struct GqlUser {
    #[serde(rename = "contributionsCollection")]
    cc: Cc,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Cc {
    total_commit_contributions: u64,
    total_pull_request_contributions: u64,
    total_pull_request_review_contributions: u64,
    total_issue_contributions: u64,
    restricted_contributions_count: u64,
    contribution_calendar: Calendar,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Calendar {
    total_contributions: u64,
    weeks: Vec<Week>,
}

#[derive(Deserialize)]
struct Week {
    #[serde(rename = "contributionDays")]
    days: Vec<Day>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Day {
    date: String,
    contribution_count: u64,
}

/// Fetch GitHub contribution totals for each user over the last `days`.
pub async fn fetch_contributions(
    client: &Client,
    base_url: &str,
    users: &[String],
    days: u32,
) -> Result<Vec<Contributions>, String> {
    let url = graphql_url(base_url);
    let to = Utc::now();
    let from = to - Duration::days(days as i64);
    let from_s = from.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let to_s = to.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let since_date = from.format("%Y-%m-%d").to_string();

    let mut out = Vec::with_capacity(users.len());

    for user in users {
        let body = serde_json::json!({
            "query": CONTRIB_QUERY,
            "variables": { "login": user, "from": from_s, "to": to_s },
        });

        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("contributions request failed for {user}: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("contributions for {user} returned {status}: {text}"));
        }

        let parsed: GqlResponse = resp
            .json()
            .await
            .map_err(|e| format!("could not parse contributions for {user}: {e}"))?;

        if let Some(errors) = parsed.errors
            && let Some(first) = errors.first()
        {
            return Err(format!("GraphQL error for {user}: {}", first.message));
        }

        let Some(cc) = parsed.data.and_then(|d| d.user).map(|u| u.cc) else {
            // Unknown user or no access — show zeros rather than failing everything.
            out.push(Contributions {
                user: user.clone(),
                commits: 0,
                prs: 0,
                reviews: 0,
                issues: 0,
                private: 0,
                total: 0,
                daily: Vec::new(),
            });
            continue;
        };

        let daily: Vec<u64> = cc
            .contribution_calendar
            .weeks
            .iter()
            .flat_map(|w| w.days.iter())
            .filter(|d| d.date.as_str() >= since_date.as_str())
            .map(|d| d.contribution_count)
            .collect();

        out.push(Contributions {
            user: user.clone(),
            commits: cc.total_commit_contributions,
            prs: cc.total_pull_request_contributions,
            reviews: cc.total_pull_request_review_contributions,
            issues: cc.total_issue_contributions,
            private: cc.restricted_contributions_count,
            total: cc.contribution_calendar.total_contributions,
            daily,
        });
    }

    Ok(out)
}
