//! The one tokenizer: priority, tags and assignees, wherever they sit.
//!
//! `parts` answers "what does this body hold"; `segments` answers "which bytes
//! of it are a priority, a tag or an assignee" — used for colouring, so the
//! highlight and the parse can never drift apart.

use std::ops::Range;

use crate::model::{Assignee, Parts, Prio, Segment, Tag};

/// A task body cut into what this tool recognises.
pub fn parts(body: &str) -> Parts {
    let mut parts = Parts::default();
    let mut words: Vec<&str> = Vec::new();
    for token in tokens(body) {
        match token.kind {
            Kind::Prio(prio) => parts.prio = Some(prio),
            Kind::Tag => {
                if let Some(tag) = Tag::parse(&body[token.range]) {
                    parts.tags.push(tag);
                }
            }
            Kind::Assignee => {
                if let Some(assignee) = Assignee::parse(&body[token.range]) {
                    parts.assignees.push(assignee);
                }
            }
            Kind::Text => words.push(&body[token.range]),
        }
    }
    parts.title = words.join(" ");
    parts
}

/// The recognised pieces of a body as byte ranges into it, in body order.
///
/// Gaps — the spaces between pieces — belong to no range; a caller that renders
/// the body styles them like a title.
pub fn segments(body: &str) -> Vec<(Segment, Range<usize>)> {
    let mut segments: Vec<(Segment, Range<usize>)> = Vec::new();
    for token in tokens(body) {
        let segment = match token.kind {
            Kind::Prio(_) => Segment::Prio,
            Kind::Tag => Segment::Tag,
            Kind::Assignee => Segment::Assignee,
            // Consecutive words are one title run, gaps included.
            Kind::Text => match segments.last_mut() {
                Some((Segment::Title, range)) => {
                    range.end = token.range.end;
                    continue;
                }
                _ => Segment::Title,
            },
        };
        segments.push((segment, token.range));
    }
    segments
}

/// What one whitespace-delimited token is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Prio(Prio),
    Text,
    Tag,
    Assignee,
}

struct Token {
    range: Range<usize>,
    kind: Kind,
}

/// Split the body on whitespace runs, then classify each token. The first one
/// may be the priority token; nothing else is positional.
fn tokens(body: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut start: Option<usize> = None;
    for (index, character) in body.char_indices() {
        if character.is_whitespace() {
            if let Some(begin) = start.take() {
                tokens.push(classify(&body[begin..index], begin));
            }
        } else if start.is_none() {
            start = Some(index);
        }
    }
    if let Some(begin) = start {
        tokens.push(classify(&body[begin..], begin));
    }
    if let Some(first) = tokens.first_mut() {
        if let Some(prio) = priority(&body[first.range.clone()]) {
            first.kind = Kind::Prio(prio);
        }
    }
    tokens
}

fn classify(text: &str, start: usize) -> Token {
    let kind = if Tag::parse(text).is_some() {
        Kind::Tag
    } else if Assignee::parse(text).is_some() {
        Kind::Assignee
    } else {
        Kind::Text
    };
    Token {
        range: start..start + text.len(),
        kind,
    }
}

/// A whole token as a priority: `(A)`..`(D)`, one letter, nothing else.
fn priority(token: &str) -> Option<Prio> {
    let inner = token.strip_prefix('(')?.strip_suffix(')')?;
    let mut characters = inner.chars();
    let (Some(letter), None) = (characters.next(), characters.next()) else {
        return None;
    };
    Prio::from_letter(letter)
}

#[cfg(test)]
mod tests {
    use super::{parts, segments};
    use crate::model::{Assignee, Prio, Segment, Tag};

    fn tags(body: &str) -> Vec<String> {
        parts(body)
            .tags
            .iter()
            .map(|t| t.as_str().to_owned())
            .collect()
    }

    fn assignees(body: &str) -> Vec<String> {
        parts(body)
            .assignees
            .iter()
            .map(Assignee::as_str)
            .map(str::to_owned)
            .collect()
    }

    fn slices(body: &str) -> Vec<(Segment, &str)> {
        segments(body)
            .into_iter()
            .map(|(segment, range)| (segment, &body[range]))
            .collect()
    }

    #[test]
    fn tags_and_assignees_are_found_in_any_position() {
        let parsed = parts("@dave fix you #worst-bug-of-the-century");
        assert_eq!(parsed.title, "fix you");
        assert_eq!(
            parsed.tags,
            [Tag::parse("#worst-bug-of-the-century").unwrap()]
        );
        assert_eq!(parsed.assignees, [Assignee::parse("@dave").unwrap()]);
    }

    #[test]
    fn several_of_each_keep_their_order() {
        let body = "#home a #money @sam b @aricma";
        assert_eq!(tags(body), ["home", "money"]);
        assert_eq!(assignees(body), ["sam", "aricma"]);
        assert_eq!(parts(body).title, "a b");
    }

    #[test]
    fn an_email_is_not_an_assignee() {
        let body = "mail bob@host.com @sam";
        assert_eq!(assignees(body), ["sam"]);
        assert_eq!(parts(body).title, "mail bob@host.com");
    }

    #[test]
    fn a_non_ascii_hash_is_plain_text() {
        let body = "ship #café";
        assert!(tags(body).is_empty());
        assert_eq!(parts(body).title, "ship #café");
        assert_eq!(slices(body), [(Segment::Title, "ship #café")]);
    }

    #[test]
    fn a_leading_priority_token_is_parted_out() {
        let parsed = parts("(B) ship it #x");
        assert_eq!(parsed.prio, Some(Prio::B));
        assert_eq!(parsed.title, "ship it");
        assert_eq!(tags("(B) ship it #x"), ["x"]);
    }

    #[test]
    fn a_priority_only_counts_at_the_start_and_only_as_one_letter() {
        assert_eq!(parts("ship (B) it").prio, None);
        assert_eq!(parts("(B:e2) it").prio, None);
        assert_eq!(parts("(p1) it").prio, None);
        assert_eq!(parts("(E) it").prio, None);
    }

    #[test]
    fn a_title_of_only_tokens_is_empty() {
        assert_eq!(parts("(A) #x @y").title, "");
        assert_eq!(parts("#x").title, "");
    }

    #[test]
    fn segments_walk_the_body_in_order() {
        assert_eq!(
            slices("(B) a b #x @y"),
            [
                (Segment::Prio, "(B)"),
                (Segment::Title, "a b"),
                (Segment::Tag, "#x"),
                (Segment::Assignee, "@y"),
            ]
        );
    }

    #[test]
    fn segments_reconstruct_the_body_except_the_gaps() {
        let body = "  (B)  two   words  #x  @y ";
        let body = body.trim();
        let mut rebuilt = String::new();
        let mut last = 0;
        for (_, range) in segments(body) {
            rebuilt.push_str(&body[last..range.start]);
            rebuilt.push_str(&body[range.clone()]);
            last = range.end;
        }
        rebuilt.push_str(&body[last..]);
        assert_eq!(rebuilt, body);
    }

    #[test]
    fn a_body_of_whitespace_parses_to_nothing() {
        assert_eq!(parts("   ").title, "");
        assert!(segments("   ").is_empty());
    }
}
