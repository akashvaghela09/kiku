//! Tidying a transcript before it is inserted into whatever the user is typing in.
//!
//! The model already produces punctuation and capitalisation, so this is not
//! post-processing of the language — only of the whitespace, which the recogniser has
//! no opinion about and which matters a great deal when text is being spliced into a
//! sentence someone else is writing.

/// Normalise a transcript for insertion.
///
/// Collapses internal whitespace runs, trims the ends, and optionally appends one
/// trailing space so consecutive dictations do not run together into oneword.
pub fn prepare(raw: &str, trailing_space: bool) -> String {
    let mut out = String::with_capacity(raw.len() + 1);
    let mut pending_space = false;

    for ch in raw.chars() {
        if ch.is_whitespace() {
            // Newlines included: a dictated utterance is a phrase, and a stray line
            // break would break out of single-line fields like a search box.
            pending_space = !out.is_empty();
            continue;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        out.push(ch);
    }

    if trailing_space && !out.is_empty() {
        out.push(' ');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_sentence_gains_only_a_trailing_space() {
        assert_eq!(prepare("Hello there.", true), "Hello there. ");
        assert_eq!(prepare("Hello there.", false), "Hello there.");
    }

    #[test]
    fn surrounding_whitespace_is_removed() {
        assert_eq!(prepare("   Hello.  ", false), "Hello.");
    }

    #[test]
    fn internal_whitespace_runs_collapse_to_one_space() {
        assert_eq!(prepare("Hello    there", false), "Hello there");
    }

    #[test]
    fn newlines_and_tabs_become_spaces() {
        // A stray newline would submit a form or escape a single-line field.
        assert_eq!(prepare("Hello\nthere\tfriend", false), "Hello there friend");
    }

    #[test]
    fn empty_input_stays_empty_and_gains_no_trailing_space() {
        assert_eq!(prepare("", true), "");
        assert_eq!(prepare("   \n  ", true), "");
    }

    #[test]
    fn punctuation_and_capitalisation_are_left_alone() {
        let raw = "Well, I don't wish to see it any more. It is like the old portrait.";
        assert_eq!(prepare(raw, false), raw);
    }

    #[test]
    fn non_ascii_text_survives_intact() {
        assert_eq!(prepare("  café   naïve  ", false), "café naïve");
    }
}
