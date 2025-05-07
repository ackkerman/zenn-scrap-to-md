use std::error::Error;
use std::env;
use std::fs;
use reqwest::blocking::Client;
use reqwest::header::COOKIE;
use serde::Deserialize;
use clap::Parser;
use regex::Regex;

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
fn extract_slug(input: &str) -> Option<String> {
    let trimmed = input.trim_end_matches('/');
    if let Some(pos) = trimmed.find("/scraps/") {
        Some(trimmed[(pos + 8)..].to_string())
    } else {
        Some(trimmed.to_string())
    }
}

/// Fetch scrap JSON, using optional cookie.
fn fetch_scrap(slug: &str, cookie: Option<&str>) -> Result<Scrap, Box<dyn Error>> {
    let url = format!("https://zenn.dev/api/scraps/{}/blob.json", slug);
    let client = Client::builder().build()?;
    let mut req = client.get(&url);
    if let Some(c) = cookie {
        req = req.header(COOKIE, c.to_string());
    }
    let resp = req.send()?;
    if !resp.status().is_success() {
        return Err(format!("Failed to fetch scrap: HTTP {}", resp.status()).into());
    }
    Ok(resp.json()?)
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

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let cookie = args.cookie.or_else(|| env::var("ZENN_AUTH_COOKIE").ok());
    let slug = extract_slug(&args.url).ok_or("Invalid scrap URL or slug")?;
    let scrap = fetch_scrap(&slug, cookie.as_deref())?;
    let markdown = render_markdown(&scrap, &args.url, args.skip_header);
    let title = scrap.title;

    let out_path = args.output.clone().unwrap_or_else(|| format!("{}({}).md", title, slug));
    fs::write(&out_path, markdown)?;
    println!("Saved Markdown to {}", out_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_slug() {
        assert_eq!(extract_slug("https://zenn.dev/foo/scraps/barbaz"), Some("barbaz".into()));
        assert_eq!(extract_slug("barbaz"), Some("barbaz".into()));
    }

    #[test]
    fn test_render_comments_skip_and_img() {
        let comment = Comment {
            author: "u".into(),
            created_at: "2025-05-05".into(),
            body_markdown: "![](https://example.com/img1.png) Text".into(),
            children: vec![],
        };
        let md_with = {
            let mut s = String::new();
            render_comments(&[comment.clone()], &mut s, false);
            s
        };
        assert!(md_with.contains("**u (2025-05-05)**"));
        let md_without = {
            let mut s = String::new();
            render_comments(&[comment], &mut s, true);
            s
        };
        assert!(!md_without.contains("**u (2025-05-05)**"));
    }
}
