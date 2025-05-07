// To use this script, add the following to your Cargo.toml:
//
// [dependencies]
// reqwest = { version = "0.11", features = ["blocking", "json"] }
// serde = { version = "1.0", features = ["derive"] }
// clap = { version = "4.0", features = ["derive"] }

use std::error::Error;
use reqwest::blocking::Client;
use serde::Deserialize;
use clap::Parser;

/// Fetch Zenn scrap and output reconstructed Markdown.
#[derive(Parser)]
#[command(author, version, about = "Fetch Zenn scrap and output reconstructed Markdown")]
struct Args {
    /// Zenn scrap URL or slug (e.g. https://zenn.dev/xxx/scraps/your_slug)
    url: String,
}

#[derive(Deserialize)]
struct Scrap {
    title: String,
    comments: Vec<Comment>,
}

#[derive(Deserialize, Default)]
struct Comment {
    author: String,
    created_at: String,
    body_markdown: String,
    #[serde(default)]
    children: Vec<Comment>,
}

/// Extracts the scrap slug from a URL or returns the input if already a slug.
fn extract_slug(input: &str) -> Option<String> {
    let trimmed = input.trim_end_matches('/');
    if let Some(pos) = trimmed.find("/scraps/") {
        Some(trimmed[(pos + 9)..].to_string())
    } else {
        Some(trimmed.to_string())
    }
}

/// Fetch the scrap JSON from Zenn API and deserialize.
fn fetch_scrap(slug: &str) -> Result<Scrap, Box<dyn Error>> {
    let url = format!("https://zenn.dev/api/scraps/{}/blob.json", slug);
    let client = Client::new();
    let resp = client.get(&url).send()?;
    if !resp.status().is_success() {
        return Err(format!("Failed to fetch scrap: HTTP {}", resp.status()).into());
    }
    let scrap: Scrap = resp.json()?;
    Ok(scrap)
}

/// Recursively render comments with nested blockquotes.
fn render_comments(comments: &[Comment], depth: usize, out: &mut String) {
    let prefix = "> ".repeat(depth);
    for comment in comments {
        // Author and timestamp
        out.push_str(&format!("{}**{} ({})**\n\n", prefix, comment.author, comment.created_at));
        // Body markdown lines
        for line in comment.body_markdown.lines() {
            out.push_str(&format!("{}{}\n", prefix, line));
        }
        out.push_str("\n");
        // Render children
        if !comment.children.is_empty() {
            render_comments(&comment.children, depth + 1, out);
        }
    }
}

/// Render entire scrap as Markdown.
fn render_markdown(scrap: &Scrap) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", scrap.title));
    render_comments(&scrap.comments, 0, &mut out);
    out
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let slug = extract_slug(&args.url).ok_or("Invalid scrap URL or slug")?;
    let scrap = fetch_scrap(&slug)?;
    let markdown = render_markdown(&scrap);
    println!("{}", markdown);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_slug() {
        assert_eq!(extract_slug("https://zenn.dev/foo/scraps/barbaz"), Some("barbaz".into()));
        assert_eq!(extract_slug("barbaz"), Some("barbaz".into()));
        assert_eq!(extract_slug("https://example.com/scraps/slug/"), Some("slug".into()));
    }

    #[test]
    fn test_render_markdown() {
        let scrap = Scrap {
            title: "Test Title".into(),
            comments: vec![Comment {
                author: "alice".into(),
                created_at: "2025-01-01".into(),
                body_markdown: "Hello\nWorld".into(),
                children: vec![Comment {
                    author: "bob".into(),
                    created_at: "2025-01-02".into(),
                    body_markdown: "Nested".into(),
                    children: vec![],
                }],
            }],
        };
        let md = render_markdown(&scrap);
        let expected = "# Test Title\n\n**alice (2025-01-01)**\n\nHello\nWorld\n\n> **bob (2025-01-02)**\n\n> Nested\n\n";
        assert_eq!(md, expected);
    }
}
