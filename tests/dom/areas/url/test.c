// URL and URLSearchParams from C: parsed, built, resolved against a base,
// and their query strings edited. Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)
#define STR(s, text) JsString_equals((s), (text))

// A pair from a params iterator: [name, value].
static bool pair_is(JsValue entry, const char* name, const char* value) {
    JsArray pair = { entry.h };
    return JsArray_length(pair) == 2 && STR(js_to_string(JsArray_get(pair, 0)), name) && STR(js_to_string(JsArray_get(pair, 1)), value);
}

__attribute__((export_name("run"))) void run(void) {
    // Parsed into its parts.
    URL url = URL_new("https://user:pw@example.com:8080/a/b/c.html?x=1&y=two#frag");
    CHECK(STR(URL_get_protocol(url), "https:"));
    CHECK(STR(URL_get_username(url), "user") && STR(URL_get_password(url), "pw"));
    CHECK(STR(URL_get_host(url), "example.com:8080"));
    CHECK(STR(URL_get_hostname(url), "example.com") && STR(URL_get_port(url), "8080"));
    CHECK(STR(URL_get_pathname(url), "/a/b/c.html"));
    CHECK(STR(URL_get_search(url), "?x=1&y=two") && STR(URL_get_hash(url), "#frag"));
    CHECK(STR(URL_get_origin(url), "https://example.com:8080"));
    CHECK(STR(URL_get_href(url), "https://user:pw@example.com:8080/a/b/c.html?x=1&y=two#frag"));
    CHECK(STR(URL_toString(url), "https://user:pw@example.com:8080/a/b/c.html?x=1&y=two#frag"));
    CHECK(STR(URL_toJSON(url), "https://user:pw@example.com:8080/a/b/c.html?x=1&y=two#frag"));

    // Built piece by piece: the parts are normalised as they are set.
    URL built = URL_new("https://example.com");
    CHECK(STR(URL_get_pathname(built), "/") && STR(URL_get_search(built), "") && STR(URL_get_hash(built), ""));
    URL_set_pathname(built, "/my dir/file name.txt");
    CHECK(STR(URL_get_pathname(built), "/my%20dir/file%20name.txt"));
    URL_set_search(built, "q=a b&r=1");
    CHECK(STR(URL_get_search(built), "?q=a%20b&r=1"));
    URL_set_hash(built, "top");
    CHECK(STR(URL_get_hash(built), "#top"));
    URL_set_port(built, "443");
    CHECK(STR(URL_get_port(built), ""));
    URL_set_port(built, "8443");
    URL_set_hostname(built, "example.org");
    CHECK(STR(URL_get_host(built), "example.org:8443"));
    URL_set_protocol(built, "http");
    CHECK(STR(URL_get_protocol(built), "http:"));
    CHECK(STR(URL_get_href(built), "http://example.org:8443/my%20dir/file%20name.txt?q=a%20b&r=1#top"));
    URL_set_href(built, "ftp://files.example.net/pub/");
    CHECK(STR(URL_get_protocol(built), "ftp:") && STR(URL_get_pathname(built), "/pub/") && STR(URL_get_search(built), ""));
    URL_set_username(built, "me");
    CHECK(STR(URL_get_href(built), "ftp://me@files.example.net/pub/"));

    // Relative references resolve against a base.
    const char* base = "https://example.com/a/b/c?old=1#h";
    CHECK(STR(URL_get_href(URL_new("../d?z=1", base)), "https://example.com/a/d?z=1"));
    CHECK(STR(URL_get_href(URL_new("./e", base)), "https://example.com/a/b/e"));
    CHECK(STR(URL_get_href(URL_new("/root", base)), "https://example.com/root"));
    CHECK(STR(URL_get_href(URL_new("?only=query", base)), "https://example.com/a/b/c?only=query"));
    CHECK(STR(URL_get_href(URL_new("#next", base)), "https://example.com/a/b/c?old=1#next"));
    CHECK(STR(URL_get_href(URL_new("//other.org/p", base)), "https://other.org/p"));
    CHECK(STR(URL_get_href(URL_new("http://abs.example/x", base)), "http://abs.example/x"));
    CHECK(STR(URL_get_pathname(URL_new("../../../../up", base)), "/up"));
    URL base_url = URL_new(base);
    CHECK(STR(URL_get_href(URL_new_with_string("sub/x.js", base_url)), "https://example.com/a/b/sub/x.js"));
    CHECK(STR(URL_get_href(URL_new_with_URL_and_URL(URL_new("../z", "http://q.example/m/n"), base_url)), "http://q.example/z"));

    // canParse and parse say yes or no; the constructor throws.
    CHECK(URL_canParse("https://example.com/x"));
    CHECK(!URL_canParse("not a url"));
    CHECK(!URL_canParse("/relative"));
    CHECK(URL_canParse("/relative", "https://example.com"));
    CHECK(URL_canParse_with_string("/relative", base_url));
    CHECK(URL_parse("not a url").h == 0 && !js_failed());
    CHECK(STR(URL_get_pathname(URL_parse("/p/q", "https://example.com")), "/p/q"));
    CHECK(URL_new("not a url").h == 0 && js_failed());
    JsValue bad = js_take_error();
    CHECK(bad.h != 0 && STR(js_error_name(bad), "TypeError"));
    CHECK(!js_failed());

    // searchParams is live: editing it edits the URL, and the other way round.
    URL page = URL_new("https://example.com/search?q=cats&page=2");
    URLSearchParams live = URL_get_searchParams(page);
    CHECK(STR(URLSearchParams_get(live, "q"), "cats") && STR(URLSearchParams_get(live, "page"), "2"));
    URLSearchParams_set(live, "page", "3");
    URLSearchParams_append(live, "tag", "a b");
    CHECK(STR(URL_get_search(page), "?q=cats&page=3&tag=a+b"));
    URL_set_search(page, "?z=26");
    CHECK(STR(URLSearchParams_get(live, "z"), "26") && URLSearchParams_get(live, "q").h == 0);
    CHECK(URLSearchParams_get_size(live) == 1);

    // URLSearchParams on its own: get, getAll, has, set, append, delete.
    URLSearchParams params = URLSearchParams_new_with_string("?a=1&b=2&a=3&c=");
    CHECK(URLSearchParams_get_size(params) == 4);
    CHECK(STR(URLSearchParams_get(params, "a"), "1"));
    CHECK(URLSearchParams_get(params, "missing").h == 0);
    CHECK(JsString_len(URLSearchParams_get(params, "c")) == 0 && URLSearchParams_get(params, "c").h != 0);
    JsArray_JsString all = URLSearchParams_getAll(params, "a");
    CHECK(JsArray_JsString_length(all) == 2);
    CHECK(STR(JsArray_JsString_get(all, 1), "3"));
    CHECK(URLSearchParams_has(params, "b") && !URLSearchParams_has(params, "z"));
    CHECK(URLSearchParams_has(params, "a", "3") && !URLSearchParams_has(params, "a", "2"));
    URLSearchParams_set(params, "a", "9");
    CHECK(STR(URLSearchParams_toString(params), "a=9&b=2&c="));
    URLSearchParams_append(params, "b", "4");
    CHECK(JsArray_JsString_length(URLSearchParams_getAll(params, "b")) == 2);
    URLSearchParams_delete(params, "b", "2");
    CHECK(STR(URLSearchParams_toString(params), "a=9&c=&b=4"));
    URLSearchParams_delete(params, "b");
    CHECK(!URLSearchParams_has(params, "b") && URLSearchParams_get_size(params) == 2);

    // Sorting is stable by name; iteration yields [name, value] pairs in order.
    URLSearchParams unsorted = URLSearchParams_new_with_string("z=1&b=2&a=3&b=1");
    URLSearchParams_sort(unsorted);
    CHECK(STR(URLSearchParams_toString(unsorted), "a=3&b=2&b=1&z=1"));
    JsIterator it = URLSearchParams_iterate(unsorted);
    JsValue entry;
    int seen = 0;
    bool in_order = true;
    const char* names[] = { "a", "b", "b", "z" };
    const char* values[] = { "3", "2", "1", "1" };
    while (URLSearchParams_iterate_next(it, &entry)) {
        if (seen < 4 && !pair_is(entry, names[seen], values[seen])) in_order = false;
        seen++;
    }
    CHECK(seen == 4 && in_order);

    // Encoding: what encodeURIComponent would escape, with spaces as plus.
    URLSearchParams encoded = URLSearchParams_new_with_string("");
    URLSearchParams_set(encoded, "k", "a/b c+d&e=f#g");
    CHECK(STR(URLSearchParams_toString(encoded), "k=a%2Fb+c%2Bd%26e%3Df%23g"));
    CHECK(STR(URLSearchParams_get(encoded, "k"), "a/b c+d&e=f#g"));
    URLSearchParams_set(encoded, "name", "caf\xc3\xa9 \xe2\x82\xac");
    CHECK(STR(URLSearchParams_toString(encoded), "k=a%2Fb+c%2Bd%26e%3Df%23g&name=caf%C3%A9+%E2%82%AC"));
    URLSearchParams decoded = URLSearchParams_new_with_string("x=%E2%82%AC+1%2B1&%C3%A9=ok");
    CHECK(STR(URLSearchParams_get(decoded, "x"), "\xe2\x82\xac 1+1") && STR(URLSearchParams_get(decoded, "\xc3\xa9"), "ok"));

    // From pairs, and a copy from another set that stays independent.
    JsArray_JsArray_JsString list = JsArray_JsArray_JsString_new();
    JsArray_JsString p1 = JsArray_JsString_new();
    JsArray_JsString_push(p1, "one");
    JsArray_JsString_push(p1, "1");
    JsArray_JsArray_JsString_push(list, p1);
    URLSearchParams from_pairs = URLSearchParams_new(list);
    CHECK(STR(URLSearchParams_toString(from_pairs), "one=1"));
    URLSearchParams copy = URLSearchParams_new_with_URLSearchParams(from_pairs);
    URLSearchParams_append(copy, "two", "2");
    CHECK(URLSearchParams_get_size(from_pairs) == 1 && URLSearchParams_get_size(copy) == 2);

    // A URL as a fetch-style base for a path made from query parts.
    URL api = URL_new("/v1/items", "https://api.example.com");
    URLSearchParams_append(URL_get_searchParams(api), "limit", "10");
    URLSearchParams_append(URL_get_searchParams(api), "sort", "name desc");
    CHECK(STR(URL_get_href(api), "https://api.example.com/v1/items?limit=10&sort=name+desc"));
}
