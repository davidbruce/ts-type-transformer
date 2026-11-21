// Shared casing and naming utilities across language generators and lowering.

use super::model::Call;

fn snake_impl(name: &str, split_after_any_digit: bool, alnum_only: bool) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() {
            let prev = i.checked_sub(1).map(|j| chars[j]);
            let next = chars.get(i + 1);
            let after_digit = split_after_any_digit || next.is_some_and(|n| n.is_lowercase());
            let after_word = prev.is_some_and(|p| p.is_lowercase() || (p.is_ascii_digit() && after_digit));
            let ends_acronym = prev.is_some_and(|p| p.is_uppercase()) && next.is_some_and(|n| n.is_lowercase());
            if (after_word || ends_acronym) && !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else if !alnum_only || c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out
}

/// `getElementById` -> `get_element_by_id`, `rotate2D` -> `rotate2d`.
pub fn snake_case(name: &str) -> String {
    snake_impl(name, false, false)
}

/// `rotate2D` -> `rotate2_d`, `NodeJS` -> `node_js`.
pub fn snake_case_after_digit(name: &str) -> String {
    snake_impl(name, true, false)
}

/// `getElementById` -> `get_element_by_id`, non-ASCII-alnum -> `_` (collapsed).
pub fn snake_case_alnum(name: &str) -> String {
    snake_impl(name, false, true)
}

/// `Options_mode` -> `OptionsMode`, `fill` -> `Fill` (splits on `_`).
pub fn pascal_underscore(name: &str) -> String {
    name.split('_')
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut cs = s.chars();
            cs.next()
                .map(|c| c.to_uppercase().collect::<String>() + cs.as_str())
                .unwrap_or_default()
        })
        .collect()
}

/// `async` -> `Async`, `no-cache` -> `NoCache` (splits on non-alphanumeric).
pub fn pascal_alnum(word: &str) -> String {
    let mut out = String::new();
    let mut up = true;
    for c in word.chars() {
        if c.is_alphanumeric() {
            if up {
                out.extend(c.to_uppercase());
            } else {
                out.push(c);
            }
            up = false;
        } else {
            up = true;
        }
    }
    out
}

/// UpperCamelCase splitting on non-ASCII-alphanumeric boundaries.
pub fn pascal_ascii_alnum(name: &str) -> String {
    name.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut cs = s.chars();
            cs.next()
                .map(|c| c.to_uppercase().collect::<String>() + cs.as_str())
                .unwrap_or_default()
        })
        .collect()
}

/// Converts `SCREAMING_SNAKE_CASE` (when all alphabetic characters are uppercase and `_` is present)
/// using `to_pascal`, leaving mixed-case names intact via `to_pascal(name)`.
pub fn screaming_to_pascal(name: &str, to_pascal: fn(&str) -> String) -> String {
    if name.contains('_') && name.chars().all(|c| !c.is_alphabetic() || c.is_uppercase()) {
        let lower_words = name
            .split('_')
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase)
            .collect::<Vec<_>>()
            .join("_");
        to_pascal(&lower_words)
    } else {
        to_pascal(name)
    }
}

/// Whether `call`'s JS member or global path segment starts with `$` (e.g. Angular/jQuery/RxJS internals).
pub fn has_dollar_js_name(call: &Call) -> bool {
    matches!(
        call,
        Call::Method(js)
            | Call::Static(js)
            | Call::Get(js)
            | Call::GetBound(js)
            | Call::Set(js)
            | Call::StaticGet(js)
            | Call::StaticSet(js)
            if js.rsplit('.').next().is_some_and(|n| n.starts_with('$'))
    )
}
