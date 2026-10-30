//! Markdown to HTML, with comrak (feature `markdown`).

#[cfg(feature = "markdown")]
use comrak::{markdown_to_html, Options};

/// Render Markdown with GFM tables, strikethrough, autolinks and task
/// lists, footnotes, heading IDs and raw HTML passed through. No slots.
#[cfg(feature = "markdown")]
pub fn render(text: &str) -> String {
    markdown_to_html(text, &options())
}

#[cfg(feature = "markdown")]
fn options() -> Options<'static> {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.extension.header_id_prefix = Some(String::new());
    options.render.r#unsafe = true;
    options
}
