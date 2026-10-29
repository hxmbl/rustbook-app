use mdbook_preprocessor::{
    book::{Book, BookItem},
    errors::Result,
    Preprocessor, PreprocessorContext,
};
use pulldown_cmark::{Event, Tag};
use pulldown_cmark_to_cmark::cmark;
use std::collections::HashMap;

/// A preprocessor for rewriting external book links to local equivalents.
///
/// This preprocessor converts links to the online Rust Book
/// (https://doc.rust-lang.org/stable/book/*) to local relative links,
/// while preserving links to other external documentation.
pub struct TrplLinkRewriter;

impl Preprocessor for TrplLinkRewriter {
    fn name(&self) -> &str {
        "trpl-link-rewriter"
    }

    fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book> {
        // Build a mapping from chapter names to their local HTML paths
        let chapter_map = build_chapter_map(&book);

        book.for_each_mut(|item| {
            if let BookItem::Chapter(ref mut chapter) = item {
                chapter.content = rewrite_links(&chapter.content, &chapter_map);
            }
        });
        Ok(book)
    }

    fn supports_renderer(&self, renderer: &str) -> Result<bool> {
        Ok(renderer == "html" || renderer == "markdown" || renderer == "test")
    }
}

/// Builds a mapping from chapter source paths to their HTML output paths.
fn build_chapter_map(book: &Book) -> HashMap<String, String> {
    let mut map = HashMap::new();

    // Add special pages
    map.insert("index".to_string(), "index.html".to_string());
    map.insert("title-page".to_string(), "title-page.html".to_string());

    for item in book.iter() {
        if let BookItem::Chapter(chapter) = item {
            if let Some(path) = &chapter.path {
                // The source path is like "ch01-01-installation.md"
                // The output path is like "ch01-01-installation.html"
                let source_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                let html_name = format!("{}.html", source_name);
                map.insert(source_name.to_string(), html_name);
            }
        }
    }

    map
}

/// Rewrites links in the markdown content to use local paths.
fn rewrite_links(text: &str, chapter_map: &HashMap<String, String>) -> String {
    let parser = crate::parser(text);

    let mut events = Vec::new();

    for event in parser {
        match event {
            Event::Start(Tag::Link { link_type, dest_url, title, id }) => {
                let rewritten_dest = rewrite_url(&dest_url.to_string(), chapter_map);
                events.push(Event::Start(Tag::Link {
                    link_type,
                    dest_url: rewritten_dest.into(),
                    title,
                    id,
                }));
            }
            event => {
                events.push(event);
            }
        }
    }

    let mut buf = String::new();
    cmark(events.into_iter(), &mut buf).unwrap();

    // Post-process to handle reference-style links
    rewrite_reference_links(&buf, chapter_map)
}

/// Rewrites a single URL if it points to the online book.
fn rewrite_url(url: &str, chapter_map: &HashMap<String, String>) -> String {
    // Match URLs like: https://doc.rust-lang.org/stable/book/ch01-01-installation.html
    // or: https://doc.rust-lang.org/stable/book/ch01-01-installation.html#installation
    // or: https://doc.rust-lang.org/stable/book/ (the base book URL)
    if let Some(rest) = url.strip_prefix("https://doc.rust-lang.org/stable/book/") {
        // Handle the base book URL
        if rest.is_empty() || rest == "/" {
            return "index.html".to_string();
        }

        // Extract the chapter name (without .html and without fragments)
        let chapter_part = rest
            .split('#')
            .next()
            .unwrap_or(rest)
            .trim_end_matches(".html")
            .trim_start_matches('/');

        // Check if this is a book chapter we have locally
        if chapter_map.contains_key(chapter_part) {
            // Return relative link with fragment if present
            if let Some(fragment) = url.split('#').nth(1) {
                format!("{}.html#{}", chapter_part, fragment)
            } else {
                format!("{}.html", chapter_part)
            }
        } else {
            // Not a book chapter, keep as external
            url.to_string()
        }
    } else if url == "https://doc.rust-lang.org/stable/book" {
        // Handle base book URL without trailing slash
        return "index.html".to_string();
    } else {
        // Not a book URL, keep as-is
        url.to_string()
    }
}

/// Rewrites reference-style links like [name]: url
fn rewrite_reference_links(text: &str, chapter_map: &HashMap<String, String>) -> String {
    let mut result = String::new();
    let mut lines = text.lines().peekable();

    while let Some(line) = lines.next() {
        if line.starts_with('[') && line.contains("]:") {
            // This looks like a reference definition
            if let Some(ref_end) = line.find("]:") {
                let ref_name = &line[0..=ref_end];
                let url_part = line[ref_end + 2..].trim();

                // Rewrite the URL if it's a book link
                let rewritten_url = rewrite_url(url_part, chapter_map);
                result.push_str(&format!("{}: {}", ref_name, rewritten_url));
            } else {
                result.push_str(line);
            }
        } else {
            result.push_str(line);
        }

        if lines.peek().is_some() {
            result.push('\n');
        }
    }

    result
}

#[cfg(test)]
mod tests;
