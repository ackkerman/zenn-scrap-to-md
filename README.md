
# Zenn Scrap to Markdown

A simple Rust CLI tool to fetch a Zenn scrap (threaded notes) and save it as a single Markdown file.
It supports:

* Fetching scrap JSON from the Zenn API
* Optional authentication via Zenn session cookie
* Converting Zenn-specific image syntax (`![](url =300x)`) to HTML `<img>` tags
* Optional skipping of comment headers (author and timestamp)
* Inserting horizontal rules (`---`) between messages

---

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/zenn-scrap-to-md.git
cd zenn-scrap-to-md

# Build with Cargo
cargo build --release

# (Optional) Install globally
cargo install --path .
```

## Usage

```bash
# Basic usage: fetch scrap and save to <slug>.md
zenn-scrap-to-md https://zenn.dev/youruser/scraps/your_slug

# Specify output file
zenn-scrap-to-md https://zenn.dev/youruser/scraps/your_slug -o my_notes.md

# Skip rendering comment headers
zenn-scrap-to-md https://zenn.dev/youruser/scraps/your_slug --skip-header
```

### Authenticating

Authenticated scraps require your Zenn session cookie. You can pass it directly:

```bash
zenn-scrap-to-md https://zenn.dev/... --cookie "_zenn_session=..."
```

Or set the environment variable `ZENN_AUTH_COOKIE`:

```bash
export ZENN_AUTH_COOKIE="_zenn_session=..."
zenn-scrap-to-md https://zenn.dev/...
```

---

## Options

| Flag            | Description                                         |
| --------------- | --------------------------------------------------- |
| `-o, --output`  | Path to output Markdown file (default: `<slug>.md`) |
| `--cookie`      | Zenn session cookie for authentication              |
| `--skip-header` | Skip rendering `**author (timestamp)**` headers     |
| `-h, --help`    | Show help message                                   |
| `-V, --version` | Show version information                            |

---

## Development

### Dependencies

* Rust 1.64+
* [reqwest](https://docs.rs/reqwest)
* [serde](https://docs.rs/serde)
* [clap](https://docs.rs/clap)
* [regex](https://docs.rs/regex)

### Testing

```bash
cargo test
```
