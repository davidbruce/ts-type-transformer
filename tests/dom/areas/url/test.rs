// URL and URLSearchParams from Rust: parsed, built, resolved and iterated.
// Every check! must hold.
#![no_main]
use bindings::*;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "check"]
    fn check_at(ok: i32, line: u32);
}
macro_rules! check {
    ($c:expr) => {
        unsafe { check_at($c as i32, line!()) }
    };
}

// FINDING 2 (again): `Option` is the DOM's constructor here, not the prelude's.
const NO_BASE: std::option::Option<StringOrURL> = None;

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    // Parsed into parts.
    let url = URL::new("https://user:pw@example.com:8080/a/b/c.html?x=1&y=two#frag", NO_BASE);
    check!(url.protocol() == "https:" && url.username() == "user" && url.password() == "pw");
    check!(url.host() == "example.com:8080" && url.hostname() == "example.com" && url.port() == "8080");
    check!(url.origin() == "https://example.com:8080");
    check!(url.pathname() == "/a/b/c.html" && url.search() == "?x=1&y=two" && url.hash() == "#frag");
    check!(url.href() == "https://user:pw@example.com:8080/a/b/c.html?x=1&y=two#frag");
    check!(url.to_string() == url.href() && url.to_json() == url.href());

    // Normalized: default ports and case, dot segments, escapes.
    let norm = URL::new("HTTP://Example.COM:80/a/./b/../c d?q=é#h h", NO_BASE);
    check!(norm.href() == "http://example.com/a/c%20d?q=%C3%A9#h%20h");
    check!(norm.port().is_empty() && norm.pathname() == "/a/c%20d");
    check!(URL::new("https://example.com", NO_BASE).pathname() == "/");

    // Built by setting parts.
    let built = URL::new("http://localhost/", NO_BASE);
    built.set_protocol("https");
    built.set_hostname("api.example.org");
    built.set_port("9000");
    built.set_pathname("/v1/items");
    built.set_search("page=2");
    built.set_hash("top");
    check!(built.href() == "https://api.example.org:9000/v1/items?page=2#top");
    built.set_port("443");
    check!(built.port().is_empty() && built.host() == "api.example.org");
    built.set_host("example.net:81");
    check!(built.origin() == "https://example.net:81");
    built.set_username("me");
    check!(built.href().starts_with("https://me@example.net:81/"));
    built.set_search("");
    built.set_hash("");
    check!(built.href() == "https://me@example.net:81/v1/items");
    built.set_href("ftp://files.example.com/pub");
    check!(built.protocol() == "ftp:" && built.pathname() == "/pub");

    // Relative resolution against a base, as a string and as a URL.
    let base = URL::new("https://example.com/dir/sub/page.html?q=1#x", NO_BASE);
    let resolve = |rel: &str| URL::new(rel, &base).href();
    check!(resolve("other.html") == "https://example.com/dir/sub/other.html");
    check!(resolve("../up.html") == "https://example.com/dir/up.html");
    check!(resolve("../../../way/up.html") == "https://example.com/way/up.html");
    check!(resolve("/root.html") == "https://example.com/root.html");
    check!(resolve("?only=query") == "https://example.com/dir/sub/page.html?only=query");
    check!(resolve("#only") == "https://example.com/dir/sub/page.html?q=1#only");
    check!(resolve("//cdn.example.org/lib.js") == "https://cdn.example.org/lib.js");
    check!(resolve("http://elsewhere.test/") == "http://elsewhere.test/");
    check!(URL::new("x/y", "https://example.com/a/b").href() == "https://example.com/a/x/y");

    // Invalid input: a throw, parse's None, and canParse.
    check!(js::attempt(|| URL::new("not a url", NO_BASE)).is_err());
    check!(js::take_error().is_none());
    let error = js::attempt(|| URL::new("/relative", NO_BASE)).err().unwrap();
    check!(error.name() == "TypeError");
    check!(URL::parse("nope", NO_BASE).is_none() && js::take_error().is_none());
    check!(URL::parse("https://ok.example/", NO_BASE).unwrap().hostname() == "ok.example");
    check!(URL::parse("rel", "https://ok.example/dir/").unwrap().href() == "https://ok.example/dir/rel");
    check!(URL::can_parse("https://example.com/", NO_BASE));
    check!(!URL::can_parse("example.com", NO_BASE));
    check!(!URL::can_parse("/path", NO_BASE) && URL::can_parse("/path", "https://example.com"));
    check!(URL::can_parse("mailto:me@example.com", NO_BASE) && URL::can_parse("data:text/plain,hi", NO_BASE));
    check!(!URL::can_parse("http://", NO_BASE));
    let mail = URL::new("mailto:me@example.com?subject=hi", NO_BASE);
    check!(mail.protocol() == "mailto:" && mail.pathname() == "me@example.com" && mail.origin() == "null");

    // search_params is live in both directions.
    let live = URL::new("https://example.com/?a=1&b=2", NO_BASE);
    let params = live.search_params();
    check!(params.get("a").as_deref() == Some("1"));
    params.append("c", "3");
    check!(live.search() == "?a=1&b=2&c=3");
    live.set_search("z=9");
    check!(params.get("z").as_deref() == Some("9") && params.get("a").is_none());
    params.delete("z", None);
    check!(live.search().is_empty() && live.href() == "https://example.com/");

    // URLSearchParams on its own: get, getAll, has, append, set, delete.
    let q = URLSearchParams::new("a=1&b=2&a=3&c=%20x+y");
    check!(q.get("a").as_deref() == Some("1") && q.get("none").is_none());
    check!(q.get_all("a").iter().collect::<Vec<_>>() == ["1", "3"]);
    check!(q.get_all("none").len() == 0);
    check!(q.get("c").as_deref() == Some(" x y"));
    check!(q.has("b", None) && !q.has("d", None));
    check!(q.has("a", Some("3")) && !q.has("a", Some("2")));
    check!(q.size() == 4.0);
    q.append("d", "4");
    q.append("a", "5");
    check!(q.get_all("a").len() == 3 && q.size() == 6.0);
    q.set("a", "only");
    check!(q.get_all("a").iter().collect::<Vec<_>>() == ["only"] && q.size() == 4.0);
    check!(q.to_string() == "a=only&b=2&c=+x+y&d=4");
    q.delete("b", None);
    q.delete("a", Some("nomatch"));
    check!(!q.has("b", None) && q.has("a", None));
    // Values are encoded on the way out.
    let enc = URLSearchParams::new(None::<URLSearchParamsNewInit>);
    enc.append("q", "a&b=c d/é");
    enc.append("empty", "");
    check!(enc.to_string() == "q=a%26b%3Dc+d%2F%C3%A9&empty=");
    check!(enc.size() == 2.0);
    check!(URLSearchParams::new(None::<URLSearchParamsNewInit>).to_string().is_empty());

    // sort is by name, stable.
    let unsorted = URLSearchParams::new("b=2&a=2&c=1&a=1&B=0");
    unsorted.sort();
    check!(unsorted.to_string() == "B=0&a=2&a=1&b=2&c=1");

    // Iteration: pairs, keys, values.
    let it = URLSearchParams::new("k1=v1&k2=v2&k1=v3");
    let pairs: Vec<(String, String)> = it.iter().map(|p| (p.get0(), p.get1())).collect();
    check!(pairs == [("k1".to_string(), "v1".to_string()), ("k2".to_string(), "v2".to_string()), ("k1".to_string(), "v3".to_string())]);
    check!(it.keys().iter().collect::<Vec<_>>() == ["k1", "k2", "k1"]);
    check!(it.values().iter().collect::<Vec<_>>() == ["v1", "v2", "v3"]);
    check!(it.entries().iter().count() == 3);

    // A leading question mark is ignored; one URLSearchParams from another is a copy.
    check!(URLSearchParams::new("?x=1").get("x").as_deref() == Some("1"));
    let copy = URLSearchParams::new(&it);
    copy.append("extra", "1");
    check!(copy.size() == 4.0 && it.size() == 3.0);

    // A query round trip through a URL.
    let target = URL::new("https://example.com/search", NO_BASE);
    for (k, v) in [("q", "rust & wasm"), ("lang", "en")] {
        target.search_params().append(k, v);
    }
    check!(target.href() == "https://example.com/search?q=rust+%26+wasm&lang=en");
    check!(URL::new(target.href().as_str(), NO_BASE).search_params().get("q").as_deref() == Some("rust & wasm"));
    // The page's own URL is a URL too.
    let here = URL::new(window().location().href(), NO_BASE);
    check!(here.protocol() == "http:" || here.protocol() == "https:");
    check!(here.pathname().ends_with("index.html") || here.pathname().ends_with('/'));
}
