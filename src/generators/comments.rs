// A declaration's documentation (`front::doc`: text, then a line per tag) in
// the comment form of each target. Written once per declaration; text that
// cannot be written as it is has the part that breaks the code taken out.

#[derive(Clone, Copy, PartialEq)]
pub enum Style {
    // Zig, Gleam, C++: the text as it is.
    Plain,
    // C: `//` comments, where a backslash or `??/` ending a line joins the next one.
    C,
    // Markdown that rustdoc compiles: no fence or indented block it could run as a test.
    Rust,
}

// A tag line as a reader of plain text takes it.
fn tag(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix('@')?;
    let (name, text) = rest.split_once(' ').unwrap_or((rest, ""));
    matches!(name, "param" | "returns" | "deprecated" | "default" | "see").then_some((name, text))
}

fn label(name: &str) -> &'static str {
    match name {
        "returns" => "Returns",
        "deprecated" => "Deprecated",
        "default" => "Default",
        _ => "See",
    }
}

pub fn text_lines(doc: &str, style: Style) -> Vec<String> {
    let mut out = Vec::new();
    // The fence being inside of, to close it where the text does not.
    let mut fence: Option<String> = None;
    for line in doc.lines() {
        let line = line.trim_end();
        let mut line = match tag(line) {
            Some(("param", text)) if style == Style::Rust => match text.split_once(' ') {
                Some((name, text)) => format!("- `{name}`: {text}"),
                None => format!("- `{text}`"),
            },
            Some(("param", _)) => line.to_string(),
            Some((name, text)) if style == Style::Rust => format!("- **{}:** {text}", label(name)),
            Some((name, text)) => format!("{}: {text}", label(name)),
            None => line.to_string(),
        };
        if style == Style::Rust {
            let t = line.trim_start();
            let marker = t.chars().next().filter(|c| matches!(c, '`' | '~'));
            let run = marker.map_or(0, |m| t.chars().take_while(|c| *c == m).count());
            match &fence {
                None if run >= 3 => {
                    let m = marker.unwrap();
                    line = format!("{}{}text", &line[..line.len() - t.len()], m.to_string().repeat(run));
                    fence = Some(m.to_string().repeat(run));
                }
                Some(f) if t.starts_with(f.as_str()) && t.trim_start_matches(marker.unwrap_or(' ')).is_empty() => fence = None,
                // An indent of four or more is a block of code, and a test.
                None if line.len() - t.len() >= 4 && !t.starts_with("- ") => line = t.to_string(),
                _ => {}
            }
        }
        if style == Style::C {
            while let Some(shorter) = line.strip_suffix('\\').or_else(|| line.strip_suffix("??/")) {
                line = shorter.trim_end().to_string();
            }
        }
        out.push(line);
    }
    if let Some(f) = fence {
        out.push(f);
    }
    out
}

// The documentation as comment lines, each led by `marker` (`///`, `//`) and `indent`.
pub fn comment(doc: &str, marker: &str, indent: &str, style: Style) -> String {
    let mut out = String::new();
    for line in text_lines(doc, style) {
        out += indent;
        out += marker;
        if !line.is_empty() {
            out.push(' ');
            out += &line;
        }
        out.push('\n');
    }
    out
}

fn xml(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

// F# reads `///` as XML: a summary, a `param` and a `returns` where JSDoc has them.
pub fn fsharp(doc: &str, indent: &str) -> String {
    let mut summary: Vec<String> = Vec::new();
    let mut tags: Vec<String> = Vec::new();
    for line in doc.lines() {
        let line = line.trim_end();
        match tag(line) {
            Some(("param", text)) => {
                let (name, text) = text.split_once(' ').unwrap_or((text, ""));
                tags.push(format!("<param name=\"{}\">{}</param>", xml(name), xml(text)));
            }
            Some(("returns", text)) => tags.push(format!("<returns>{}</returns>", xml(text))),
            Some((name, text)) => summary.push(xml(&format!("{}: {text}", label(name)))),
            None => summary.push(xml(line)),
        }
    }
    while summary.last().is_some_and(String::is_empty) {
        summary.pop();
    }
    let mut out = String::new();
    let mut put = |s: &str| {
        out += indent;
        out += "///";
        if !s.is_empty() {
            out.push(' ');
            out += s;
        }
        out.push('\n');
    };
    match summary.as_slice() {
        [] => {}
        [one] => put(&format!("<summary>{one}</summary>")),
        many => {
            put("<summary>");
            many.iter().for_each(|l| put(l));
            put("</summary>");
        }
    }
    tags.iter().for_each(|t| put(t));
    out
}
