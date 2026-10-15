use smartstring::{LazyCompact, SmartString};
use textwrap::{core::display_width, Options, WordSplitter::NoHyphenation};

/// Given a slice of text, return the text re-wrapped to fit it
/// within the given width.
///
/// If every line starts with the same indentation and comment token, that
/// prefix is kept verbatim and only the text after it is re-wrapped.
pub fn reflow_hard_wrap(
    text: &str,
    text_width: usize,
    comment_tokens: &[String],
) -> SmartString<LazyCompact> {
    match comment_prefix(text, comment_tokens) {
        Some(prefix) => reflow_commented(text, prefix, text_width),
        None => refill(text, text_width),
    }
    .into()
}

fn refill(text: &str, text_width: usize) -> String {
    let options = Options::new(text_width)
        .word_splitter(NoHyphenation)
        .word_separator(textwrap::WordSeparator::AsciiSpace);
    textwrap::refill(text, options)
}

/// The indentation and comment token that begin every line of `text`.
///
/// Each line's token is the longest one it starts with, so that Rust's `//`
/// does not match a `///` line.
fn comment_prefix<'a>(text: &'a str, comment_tokens: &[String]) -> Option<&'a str> {
    let first = text.lines().next()?;
    let indent = first.len() - first.trim_start_matches([' ', '\t']).len();
    let token = longest_token(&first[indent..], comment_tokens)?;
    text.lines()
        .all(|line| {
            line.starts_with(&first[..indent])
                && longest_token(&line[indent..], comment_tokens) == Some(token)
        })
        .then_some(&first[..indent + token.len()])
}

fn longest_token<'a>(text: &str, comment_tokens: &'a [String]) -> Option<&'a str> {
    comment_tokens
        .iter()
        .map(String::as_str)
        .filter(|token| text.starts_with(token))
        .max_by_key(|token| token.len())
}

/// Re-wrap `text`, every line of which starts with `prefix`, keeping `prefix`
/// at the start of every output line.
///
/// Whitespace that follows `prefix` on every line is kept with `prefix`.
fn reflow_commented(text: &str, prefix: &str, text_width: usize) -> String {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let prefix = paragraph_prefix(&lines, prefix);
    let body: String = lines.iter().map(|line| &line[prefix.len()..]).collect();
    let width = text_width.saturating_sub(display_width(prefix));
    refill(&body, width)
        .split_inclusive('\n')
        .flat_map(|line| [prefix, line])
        .collect()
}

/// The longest prefix of `lines[0]` that is `prefix` followed by whitespace
/// and that begins every line.
fn paragraph_prefix<'a>(lines: &[&'a str], prefix: &str) -> &'a str {
    let first = lines[0];
    let gap = first[prefix.len()..]
        .find(|c| c != ' ' && c != '\t')
        .unwrap_or(first.len() - prefix.len());
    let len = lines.iter().fold(prefix.len() + gap, |len, line| {
        first[..len]
            .bytes()
            .zip(line.bytes())
            .take_while(|(a, b)| a == b)
            .count()
    });
    &first[..len]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rust_tokens() -> Vec<String> {
        ["//", "///", "//!"].map(String::from).to_vec()
    }

    #[test]
    fn inner_doc_comment() {
        let text = "    //! aaa bbb\n    //! ccc\n    //! ddd eee\n";
        assert_eq!(
            reflow_hard_wrap(text, 16, &rust_tokens()),
            "    //! aaa bbb\n    //! ccc ddd\n    //! eee\n"
        );
    }

    #[test]
    fn space_after_token_on_wrapped_lines() {
        let text = "//! aaa bbb ccc ddd";
        assert_eq!(
            reflow_hard_wrap(text, 11, &rust_tokens()),
            "//! aaa bbb\n//! ccc ddd"
        );
    }

    #[test]
    fn outer_doc_comment() {
        let text = "/// aaa\n/// bbb ccc\n";
        assert_eq!(
            reflow_hard_wrap(text, 80, &rust_tokens()),
            "/// aaa bbb ccc\n"
        );
    }

    #[test]
    fn mixed_tokens_fall_back() {
        let text = "// aaa\n/// bbb\n";
        assert_eq!(reflow_hard_wrap(text, 80, &rust_tokens()), refill(text, 80));
    }

    #[test]
    fn plain_text() {
        let text = "aaa\nbbb ccc";
        assert_eq!(reflow_hard_wrap(text, 80, &rust_tokens()), "aaa bbb ccc");
    }
}
