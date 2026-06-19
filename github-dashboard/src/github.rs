use chrono::{Duration, Utc};
use reqwest::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
use serde::Deserialize;

/// Safety cap on pagination so a huge account can't loop forever.
const MAX_PAGES: u32 = 10;

/// Aggregate review decision on a PR, mirroring GitHub's `reviewDecision`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewState {
    Approved,
    ChangesRequested,
    ReviewRequired,
    /// No decision yet, or reviews not required on this PR.
    None,
}

/// Rolled-up status of the latest commit's checks (CI), mirroring
/// GitHub's `statusCheckRollup.state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiState {
    Success,
    Failure,
    Pending,
    /// No checks configured / reported.
    None,
}

#[derive(Debug, Clone)]
pub struct Pr {
    pub number: u64,
    pub title: String,
    pub body: String,
    /// "open", "closed", or "merged".
    pub state: String,
    /// "owner/name".
    pub repo: String,
    /// Configured users matched by the `involves:` search for this PR.
    pub involved_users: Vec<String>,
    /// Whether the PR is still a draft.
    pub is_draft: bool,
    pub review: ReviewState,
    pub ci: CiState,
    pub additions: u64,
    pub deletions: u64,
    /// Logins a review is currently requested from (pending reviewers).
    pub review_requested_from: Vec<String>,
}

impl Pr {
    pub fn is_open(&self) -> bool {
        self.state == "open"
    }

    pub fn is_merged(&self) -> bool {
        self.state == "merged"
    }

    /// True when this PR is open, ready (not a draft), and awaiting a review
    /// from one of the given users — i.e. the ball is in their court.
    pub fn waiting_on(&self, users: &[String]) -> bool {
        self.is_open()
            && !self.is_draft
            && self
                .review_requested_from
                .iter()
                .any(|r| users.iter().any(|u| u.eq_ignore_ascii_case(r)))
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

// --- PR search (GraphQL) ------------------------------------------------------

/// GraphQL `search` over issues, restricted to PRs. Returns the rich per-PR
/// fields (review decision, draft, CI rollup, diff size, pending reviewers)
/// that the REST search endpoint does not expose.
const PR_SEARCH_QUERY: &str = r#"
query($q:String!,$cursor:String){
  search(query:$q, type:ISSUE, first:100, after:$cursor){
    pageInfo{ hasNextPage endCursor }
    nodes{
      ... on PullRequest {
        number
        title
        body
        state
        isDraft
        reviewDecision
        additions
        deletions
        repository{ nameWithOwner }
        reviewRequests(first:25){
          nodes{ requestedReviewer{ ... on User { login } } }
        }
        commits(last:1){
          nodes{ commit{ statusCheckRollup{ state } } }
        }
      }
    }
  }
}"#;

#[derive(Deserialize)]
struct PrSearchResponse {
    data: Option<PrSearchData>,
    errors: Option<Vec<GqlError>>,
}

#[derive(Deserialize)]
struct PrSearchData {
    search: SearchConn,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchConn {
    page_info: PageInfo,
    nodes: Vec<PrNode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: bool,
    end_cursor: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct PrNode {
    number: u64,
    title: String,
    body: Option<String>,
    state: String,
    is_draft: bool,
    review_decision: Option<String>,
    additions: u64,
    deletions: u64,
    repository: RepoRef,
    review_requests: ReviewRequests,
    commits: CommitsConn,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RepoRef {
    name_with_owner: String,
}

#[derive(Deserialize, Default)]
struct ReviewRequests {
    nodes: Vec<ReviewRequestNode>,
}

#[derive(Deserialize, Default)]
struct ReviewRequestNode {
    #[serde(rename = "requestedReviewer")]
    reviewer: Option<Reviewer>,
}

#[derive(Deserialize, Default)]
struct Reviewer {
    /// Absent for team reviewers (only `... on User` is selected).
    login: Option<String>,
}

#[derive(Deserialize, Default)]
struct CommitsConn {
    nodes: Vec<CommitNode>,
}

#[derive(Deserialize, Default)]
struct CommitNode {
    commit: CommitInner,
}

#[derive(Deserialize, Default)]
struct CommitInner {
    #[serde(rename = "statusCheckRollup")]
    rollup: Option<StatusRollup>,
}

#[derive(Deserialize, Default)]
struct StatusRollup {
    state: String,
}

fn map_review(decision: Option<&str>) -> ReviewState {
    match decision {
        Some("APPROVED") => ReviewState::Approved,
        Some("CHANGES_REQUESTED") => ReviewState::ChangesRequested,
        Some("REVIEW_REQUIRED") => ReviewState::ReviewRequired,
        _ => ReviewState::None,
    }
}

fn map_ci(state: Option<&str>) -> CiState {
    match state {
        Some("SUCCESS") => CiState::Success,
        Some("FAILURE") | Some("ERROR") => CiState::Failure,
        Some("PENDING") | Some("EXPECTED") => CiState::Pending,
        _ => CiState::None,
    }
}

/// Search for PRs that each user authored or was involved in, since `days` ago.
/// Results are de-duplicated by repo+number (involves: overlaps between users).
pub async fn fetch_prs(
    client: &Client,
    base_url: &str,
    users: &[String],
    days: u32,
) -> Result<Vec<Pr>, String> {
    let url = graphql_url(base_url);
    let since = (Utc::now() - Duration::days(days as i64))
        .format("%Y-%m-%d")
        .to_string();

    let mut prs: Vec<Pr> = Vec::new();
    // repo#number -> index into `prs`, so a PR involving several users is
    // recorded once with all involved users accumulated.
    let mut key_to_index: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for user in users {
        let query = format!("involves:{user} type:pr created:>={since}");
        let mut cursor: Option<String> = None;
        let mut page = 0u32;

        loop {
            let body = serde_json::json!({
                "query": PR_SEARCH_QUERY,
                "variables": { "q": query, "cursor": cursor },
            });

            let resp = client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("search request failed for {user}: {e}"))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(format!("search for {user} returned {status}: {text}"));
            }

            let parsed: PrSearchResponse = resp
                .json()
                .await
                .map_err(|e| format!("could not parse search response for {user}: {e}"))?;

            if let Some(errors) = parsed.errors
                && let Some(first) = errors.first()
            {
                return Err(format!("GraphQL search error for {user}: {}", first.message));
            }

            let Some(data) = parsed.data else { break };

            for node in data.search.nodes {
                // Non-PR nodes deserialize empty; `type:pr` should exclude them.
                if node.number == 0 {
                    continue;
                }
                let key = format!("{}#{}", node.repository.name_with_owner, node.number);
                if let Some(&idx) = key_to_index.get(&key) {
                    // Already seen via another user's search — note this user too.
                    let involved = &mut prs[idx].involved_users;
                    if !involved.iter().any(|u| u == user) {
                        involved.push(user.clone());
                    }
                    continue;
                }

                let requested = node
                    .review_requests
                    .nodes
                    .into_iter()
                    .filter_map(|n| n.reviewer.and_then(|r| r.login))
                    .collect();
                let ci = map_ci(
                    node.commits
                        .nodes
                        .first()
                        .and_then(|c| c.commit.rollup.as_ref())
                        .map(|r| r.state.as_str()),
                );

                key_to_index.insert(key, prs.len());
                prs.push(Pr {
                    number: node.number,
                    title: node.title,
                    body: node.body.unwrap_or_default(),
                    state: node.state.to_lowercase(),
                    repo: node.repository.name_with_owner,
                    involved_users: vec![user.clone()],
                    is_draft: node.is_draft,
                    review: map_review(node.review_decision.as_deref()),
                    ci,
                    additions: node.additions,
                    deletions: node.deletions,
                    review_requested_from: requested,
                });
            }

            page += 1;
            if !data.search.page_info.has_next_page || page >= MAX_PAGES {
                break;
            }
            cursor = data.search.page_info.end_cursor;
        }
    }

    Ok(prs)
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
