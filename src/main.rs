use std::{env, fs, io};
use clap::Parser;
use serde::Deserialize;
use regex::Regex;
use fantoccini::{Client, Locator};
use fantoccini::error::NewSessionError;
use tokio::time::{sleep, Duration};
use thiserror::Error;

/// Fetch Zenn scrap and save as Markdown file.
#[derive(Parser)]
#[command(author, version, about = "Fetch Zenn scrap and save as Markdown file")]
struct Args {
    /// Zenn scrap URL or slug (e.g. https://zenn.dev/xxx/scraps/your_slug)
    url: String,
    /// Output Markdown file path (defaults to `<slug>.md`)
    #[arg(short, long)]
    output: Option<String>,
    /// Zenn session cookie, falls back to env ZENN_AUTH_COOKIE
    #[arg(long)]
    cookie: Option<String>,
    /// Skip rendering comment headers (author and timestamp)
    #[arg(long)]
    skip_header: bool,
}

#[derive(Error, Debug)]
enum AppError {
    #[error("Fantoccini new session error: {0}")]
    NewSession(#[from] NewSessionError),
    #[error("Fantoccini error: {0}")]
    WebDriver(#[from] fantoccini::error::CmdError),
    #[error("Reqwest error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Env var missing: {0}")]
    MissingEnv(String),
    #[error("Invalid scrap URL or slug")]
    BadSlug,
}

#[derive(Deserialize)]
struct Scrap {
    title: String,
    comments: Vec<Comment>,
}

#[derive(Deserialize, Default, Clone)]
struct Comment {
    author: String,
    created_at: String,
    body_markdown: String,
    #[serde(default)]
    children: Vec<Comment>,
}

/// Extract slug from URL or return input if already slug.
fn extract_slug(input: &str) -> Result<String, AppError> {
    let trimmed = input.trim_end_matches('/');
    if let Some(pos) = trimmed.find("/scraps/") {
        Ok(trimmed[(pos + 8)..].to_string())
    } else if !trimmed.is_empty() {
        Ok(trimmed.to_string())
    } else {
        Err(AppError::BadSlug)
    }
}

async fn manual_login_cookie() -> Result<String, AppError> {
    // Load .env if exists (optional)
    dotenv::dotenv().ok();

    // Start WebDriver (Chromedriver/Geckodriver) at localhost:9515
    let mut client = Client::new("http://localhost:9515").await?;
    // Navigate to Zenn sign-in
    client.goto("https://zenn.dev/sign_in").await?;
    println!("Browser opened. Please log in (including Google OAuth) and then press ENTER here...");
    // Wait for user input
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    // Short pause to allow cookies to set
    sleep(Duration::from_secs(2)).await;
    // Retrieve cookies
    let cookies = client.get_all_cookies().await?;
    client.close().await?;
    // Find session cookie
    if let Some(c) = cookies.iter().find(|c| c.name() == "_zenn_session") {
        Ok(format!("_zenn_session={}", c.value()))
    } else {
        Err(AppError::BadSlug)
    }
}

/// Fetch scrap JSON, using optional cookie.
async fn fetch_scrap(slug: &str, cookie: &str) -> Result<Scrap, AppError> {
    let url = format!("https://zenn.dev/api/scraps/{}/blob.json", slug);
    let client = reqwest::Client::builder().build()?;
    let resp = client.get(&url)
        .header(reqwest::header::COOKIE, cookie)
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(AppError::Http(resp.error_for_status().unwrap_err()));
    }
    Ok(resp.json().await?)
}

/// Recursively render comments, converting Zenn image syntax to HTML and separating messages with lines.
fn render_comments(comments: &[Comment], out: &mut String, skip_header: bool) {
    // Regex to match Zenn image syntax: ![](url) or ![](url =200x)
    let img_re = Regex::new(r"!\[\]\((?P<url>[^ )]+)(?: =(?P<width>\d+)x)?\)").unwrap();

    for (i, comment) in comments.iter().enumerate() {
        // Optionally render header line for each comment
        if !skip_header {
            out.push_str(&format!("**{} ({})**\n\n", comment.author, comment.created_at));
        }

        // Convert all image syntaxes in body_markdown
        let processed = img_re.replace_all(&comment.body_markdown, |caps: &regex::Captures| {
            let url = &caps["url"];
            if let Some(w) = caps.name("width") {
                format!("<img src=\"{}\" width=\"{}\">", url, w.as_str())
            } else {
                format!("<img src=\"{}\">", url)
            }
        });

        // Write body (after image conversion)
        out.push_str(&processed);
        out.push_str("\n\n");

        // Render child comments, passing the same skip_header flag
        if !comment.children.is_empty() {
            render_comments(&comment.children, out, skip_header);
        }
        if !skip_header {
            // Insert horizontal rule between top-level comments
            if i < comments.len() - 1 {
                out.push_str("---\n\n");
            }
        }
    }
}

/// Render entire scrap as Markdown.
fn render_markdown(scrap: &Scrap, url: &str, skip_header: bool) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", scrap.title));
    out.push_str(&format!("Original: [{}]({})\n\n", url.replace("https://zenn.dev/", ""), url));
    render_comments(&scrap.comments, &mut out, skip_header);
    out
}


#[tokio::main]
async fn main() -> Result<(), AppError> {
    let args = Args::parse();
    // Determine scrap slug
    let slug = extract_slug(&args.url)?;
    // Determine session cookie: CLI > ENV > manual login
    let cookie = if let Some(c) = args.cookie.clone() {
        c
    } else if let Ok(envc) = env::var("ZENN_AUTH_COOKIE") {
        envc
    } else {
        manual_login_cookie().await?
    };
    
    let scrap = fetch_scrap(&slug, &cookie).await?;
    let title = scrap.title.clone();
    let md = render_markdown(&scrap, &args.url, args.skip_header);
    let out = args.output.clone().unwrap_or_else(|| format!("{}({}).md", title, slug));
    fs::write(&out, md)?;
    println!("Saved Markdown to {}", out);
    Ok(())
}
