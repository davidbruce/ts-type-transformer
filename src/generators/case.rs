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

fn capitalize_first(s: &str) -> String {
    let mut cs = s.chars();
    cs.next()
        .map(|c| c.to_uppercase().collect::<String>() + cs.as_str())
        .unwrap_or_default()
}

/// `Options_mode` -> `OptionsMode`, `fill` -> `Fill` (splits on `_`),
/// preserving `_` between adjacent ASCII digits so `Foo2_1` does not become `Foo21`.
pub fn pascal_underscore(name: &str) -> String {
    let mut out = String::new();
    for part in name.split('_').filter(|s| !s.is_empty()) {
        let seg = capitalize_first(part);
        if out.as_bytes().last().is_some_and(u8::is_ascii_digit)
            && seg.as_bytes().first().is_some_and(u8::is_ascii_digit)
        {
            out.push('_');
        }
        out.push_str(&seg);
    }
    out
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
        .map(capitalize_first)
        .collect()
}

/// Whether `name` begins with the identifier word `word` at a camelCase or underscore boundary
/// (e.g. `starts_with_word("getCamera", "get")`, `starts_with_word("get_camera", "get")`,
/// and `starts_with_word("get", "get")` are `true`; `starts_with_word("getter", "get")` is `false`).
pub fn starts_with_word(name: &str, word: &str) -> bool {
    name.strip_prefix(word).is_some_and(|rest| {
        matches!(rest.as_bytes().first(), None | Some(b'_'))
            || rest.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pascal_underscore_preserves_digit_boundary() {
        assert_eq!(pascal_underscore("Options_mode"), "OptionsMode");
        assert_eq!(pascal_underscore("fill"), "Fill");
        assert_eq!(pascal_underscore("FormatterOrString2_1"), "FormatterOrString2_1");
        assert_eq!(pascal_underscore("foo2_3_bar"), "Foo2_3Bar");
        assert_eq!(pascal_underscore("vec3_f32"), "Vec3F32");
    }

    #[test]
    fn starts_with_word_boundaries() {
        assert!(starts_with_word("get", "get"));
        assert!(starts_with_word("getCamera", "get"));
        assert!(starts_with_word("get_camera", "get"));
        assert!(!starts_with_word("getter", "get"));
        assert!(!starts_with_word("get2", "get"));

        assert!(starts_with_word("set", "set"));
        assert!(starts_with_word("setFocalLength", "set"));
        assert!(starts_with_word("set_selection", "set"));
        assert!(!starts_with_word("settings", "set"));
        assert!(!starts_with_word("setup", "set"));
    }
}
