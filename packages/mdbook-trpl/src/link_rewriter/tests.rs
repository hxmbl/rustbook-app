use super::*;
use std::collections::HashMap;

#[test]
fn rewrite_url_keeps_non_book_urls() {
    let map = HashMap::new();
    let url = "https://doc.rust-lang.org/cargo/";
    assert_eq!(rewrite_url(url, &map), url);
}

#[test]
fn rewrite_url_keeps_nomicon_urls() {
    let map = HashMap::new();
    let url = "https://doc.rust-lang.org/nomicon/";
    assert_eq!(rewrite_url(url, &map), url);
}

#[test]
fn rewrite_url_keeps_reference_urls() {
    let map = HashMap::new();
    let url = "https://doc.rust-lang.org/reference/items/traits.html";
    assert_eq!(rewrite_url(url, &map), url);
}

#[test]
fn rewrite_url_converts_base_book_url() {
    let map = HashMap::new();
    let url = "https://doc.rust-lang.org/stable/book/";
    assert_eq!(rewrite_url(url, &map), "index.html");
}

#[test]
fn rewrite_url_converts_base_book_url_with_trailing_slash() {
    let map = HashMap::new();
    let url = "https://doc.rust-lang.org/stable/book";
    assert_eq!(rewrite_url(url, &map), "index.html");
}

#[test]
fn rewrite_url_converts_chapter_url() {
    let mut map = HashMap::new();
    map.insert("ch01-01-installation".to_string(), "ch01-01-installation.html".to_string());

    let url = "https://doc.rust-lang.org/stable/book/ch01-01-installation.html";
    assert_eq!(rewrite_url(url, &map), "ch01-01-installation.html");
}

#[test]
fn rewrite_url_converts_chapter_url_with_fragment() {
    let mut map = HashMap::new();
    map.insert("ch01-01-installation".to_string(), "ch01-01-installation.html".to_string());

    let url = "https://doc.rust-lang.org/stable/book/ch01-01-installation.html#installation";
    assert_eq!(
        rewrite_url(url, &map),
        "ch01-01-installation.html#installation"
    );
}

#[test]
fn rewrite_url_keeps_unknown_chapter_urls() {
    let map = HashMap::new();
    let url = "https://doc.rust-lang.org/stable/book/unknown-chapter.html";
    assert_eq!(rewrite_url(url, &map), url);
}

#[test]
fn rewrite_url_keeps_unknown_chapter_urls_with_fragment() {
    let map = HashMap::new();
    let url = "https://doc.rust-lang.org/stable/book/unknown-chapter.html#section";
    assert_eq!(rewrite_url(url, &map), url);
}

#[test]
fn rewrite_links_handles_simple_text() {
    let map = HashMap::new();
    let text = "This is some basic text.";
    assert_eq!(rewrite_links(text, &map), text);
}

#[test]
fn rewrite_links_converts_inline_link() {
    let mut map = HashMap::new();
    map.insert("ch01-01-installation".to_string(), "ch01-01-installation.html".to_string());

    let text = "[Installation](https://doc.rust-lang.org/stable/book/ch01-01-installation.html)";
    let expected = "[Installation](ch01-01-installation.html)";
    assert_eq!(rewrite_links(text, &map), expected);
}

#[test]
fn rewrite_links_converts_inline_link_with_fragment() {
    let mut map = HashMap::new();
    map.insert("ch01-01-installation".to_string(), "ch01-01-installation.html".to_string());

    let text = "[Installation](https://doc.rust-lang.org/stable/book/ch01-01-installation.html#installation)";
    let expected = "[Installation](ch01-01-installation.html#installation)";
    assert_eq!(rewrite_links(text, &map), expected);
}

#[test]
fn rewrite_links_keeps_external_inline_link() {
    let map = HashMap::new();
    let text = "[Cargo](https://doc.rust-lang.org/cargo/)";
    assert_eq!(rewrite_links(text, &map), text);
}

#[test]
fn rewrite_links_converts_reference_link() {
    let mut map = HashMap::new();
    map.insert("ch01-01-installation".to_string(), "ch01-01-installation.html".to_string());

    let text = "[Installation][install]\n\n[install]: https://doc.rust-lang.org/stable/book/ch01-01-installation.html";
    let expected = "[Installation][install]\n\n[install]: ch01-01-installation.html";
    assert_eq!(rewrite_links(text, &map), expected);
}

#[test]
fn rewrite_links_converts_reference_link_with_fragment() {
    let mut map = HashMap::new();
    map.insert("ch01-01-installation".to_string(), "ch01-01-installation.html".to_string());

    let text = "[Installation][install]\n\n[install]: https://doc.rust-lang.org/stable/book/ch01-01-installation.html#installation";
    let expected = "[Installation][install]\n\n[install]: ch01-01-installation.html#installation";
    assert_eq!(rewrite_links(text, &map), expected);
}

#[test]
fn rewrite_links_keeps_external_reference_link() {
    let map = HashMap::new();
    let text = "[Cargo][cargo]\n\n[cargo]: https://doc.rust-lang.org/cargo/";
    assert_eq!(rewrite_links(text, &map), text);
}

#[test]
fn rewrite_links_handles_mixed_links() {
    let mut map = HashMap::new();
    map.insert("ch01-01-installation".to_string(), "ch01-01-installation.html".to_string());
    map.insert("ch01-02-hello-world".to_string(), "ch01-02-hello-world.html".to_string());

    let text = r#"See [Installation](https://doc.rust-lang.org/stable/book/ch01-01-installation.html) and [Hello World](https://doc.rust-lang.org/stable/book/ch01-02-hello-world.html). For more info, see [Cargo](https://doc.rust-lang.org/cargo/)."#;
    let expected = r#"See [Installation](ch01-01-installation.html) and [Hello World](ch01-02-hello-world.html). For more info, see [Cargo](https://doc.rust-lang.org/cargo/)."#;
    assert_eq!(rewrite_links(text, &map), expected);
}

#[test]
fn rewrite_links_handles_base_book_link() {
    let map = HashMap::new();
    let text = "[The Rust Book](https://doc.rust-lang.org/stable/book/)";
    let expected = "[The Rust Book](index.html)";
    assert_eq!(rewrite_links(text, &map), expected);
}
