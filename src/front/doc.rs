// The JSDoc before a declaration, as plain text: the description, then the tags a
// binding's user reads, one per line.

use tsrs_ast::{Kind, Node};
use tsrs_core::P;
use tsrs_scanner::get_leading_comment_ranges;

use super::Front;

impl<'c> Front<'c> {
    // The `/** */` block immediately before a declaration, as `clean` makes of it.
    // A variable's is its statement's.
    pub(super) fn doc(&self, node: P<Node>) -> Option<String> {
        let node = match node.kind() {
            Kind::VariableDeclaration => node.parent().and_then(|list| list.parent()).filter(|s| s.kind() == Kind::VariableStatement)?,
            _ => node,
        };
        let file = tsrs_ast::get_source_file_of_node(node)?;
        let pos = node.loc().pos().source_offset()?;
        let text = file.text();
        let block = get_leading_comment_ranges(text, pos).filter(|c| c.kind == Kind::MultiLineCommentTrivia && text[c.pos() as usize..].starts_with("/**")).last()?;
        clean(&text[block.pos() as usize..block.end() as usize])
    }
}

// Tags kept, by their spelling; the others describe types or visibility.
fn tag_name(name: &str) -> Option<&'static str> {
    Some(match name {
        "deprecated" => "deprecated",
        "param" | "arg" | "argument" => "param",
        "returns" | "return" => "returns",
        "default" | "defaultValue" => "default",
        "see" => "see",
        _ => return None,
    })
}

// A comment's text without its frame: paragraphs kept, tags folded to one
// line each (`@param name text`), links as their text.
pub fn clean(comment: &str) -> Option<String> {
    let body = comment.strip_prefix("/**").unwrap_or(comment);
    let body = body.strip_suffix("*/").unwrap_or(body);
    let mut description: Vec<String> = Vec::new();
    let mut tags: Vec<(&'static str, String)> = Vec::new();
    // In a tag: its text goes on. In one dropped: its lines go.
    let mut tag: Option<Option<usize>> = None;
    let mut fence: Option<(char, usize)> = None;
    for raw in body.lines() {
        let line: String = raw.trim_end().replace('\t', "    ").chars().filter(|c| !c.is_control()).collect();
        let line = line.trim_start();
        let line = line.strip_prefix('*').unwrap_or(line);
        let line = line.strip_prefix(' ').unwrap_or(line);
        let trimmed = line.trim_start();
        if let Some((c, n)) = fence {
            if trimmed.starts_with(&c.to_string().repeat(n)) && trimmed.trim_start_matches(c).is_empty() {
                fence = None;
            }
        } else if let Some(c) = trimmed.chars().next().filter(|c| matches!(c, '`' | '~')) {
            let n = trimmed.chars().take_while(|x| *x == c).count();
            if n >= 3 {
                fence = Some((c, n));
            }
        } else if let Some(rest) = trimmed.strip_prefix('@')
            && rest.starts_with(|c: char| c.is_ascii_alphabetic())
        {
            let (name, text) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
            tag = Some(tag_name(name).map(|n| {
                tags.push((n, text.trim().to_string()));
                tags.len() - 1
            }));
            continue;
        }
        match tag {
            Some(Some(i)) if fence.is_none() => {
                if !line.trim().is_empty() {
                    let t = &mut tags[i].1;
                    if !t.is_empty() {
                        t.push(' ');
                    }
                    t.push_str(line.trim());
                }
            }
            Some(_) => {}
            None => description.push(line.to_string()),
        }
    }
    let mut out = String::new();
    let mut blank = false;
    for line in description {
        if line.trim().is_empty() && fence_free(&out) {
            blank = !out.is_empty();
            continue;
        }
        if blank {
            out.push('\n');
        }
        blank = false;
        out.push_str(&line);
        out.push('\n');
    }
    let mut lines: Vec<String> = Vec::new();
    for (name, text) in tags {
        let text = links(&text);
        lines.push(match name {
            "param" => match param(&text) {
                Some((p, t)) => format!("@param {p} {t}"),
                None => continue,
            },
            _ if text.is_empty() => format!("@{name}"),
            _ => format!("@{name} {text}"),
        });
    }
    let mut out = links(out.trim_end());
    if !lines.is_empty() {
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(&lines.join("\n"));
    }
    Some(out).filter(|o| !o.is_empty())
}

// Whether every fence opened in the text is closed: a blank line inside one is code.
fn fence_free(text: &str) -> bool {
    text.lines().filter(|l| l.trim_start().starts_with("```") || l.trim_start().starts_with("~~~")).count() % 2 == 0
}

// `{@link Foo}`, `{@link Foo|text}` and `{@link Foo text}` as what a reader sees.
fn links(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find("{@link") {
        let Some(end) = rest[i..].find('}') else { break };
        let inner = &rest[i + 1..i + end];
        let inner = inner.trim_start_matches("@linkcode").trim_start_matches("@linkplain").trim_start_matches("@link").trim();
        let shown = match inner.split_once(['|', ' ']) {
            Some((_, text)) if !text.trim().is_empty() => text.trim(),
            _ => inner,
        };
        out.push_str(&rest[..i]);
        out.push_str(shown);
        rest = &rest[i + end + 1..];
    }
    out.push_str(rest);
    out
}

// `{type} name - text`, `[name=default] text`: the name and the text.
fn param(text: &str) -> Option<(String, String)> {
    let mut text = text.trim();
    if text.starts_with('{') {
        text = text[text.find('}')? + 1..].trim_start();
    }
    let (name, rest) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
    let name = name.trim_start_matches('[').split(['=', ']']).next().unwrap_or_default();
    let rest = rest.trim().trim_start_matches('-').trim();
    (!name.is_empty() && !rest.is_empty()).then(|| (name.to_string(), rest.to_string()))
}
