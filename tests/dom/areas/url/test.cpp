// URL and URLSearchParams from C++: parsed, built, resolved and iterated.
// Every CHECK must hold.
#include <string>
#include <vector>

#include "areas/core/Document.hpp"
#include "areas/core/functions.hpp"
#include "areas/network/URL.hpp"
#include "areas/network/URLSearchParams.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

extern "C" __attribute__((export_name("run"))) void run() {
    // Parsed into parts.
    URL url = URL::create("https://user:pw@example.com:8080/a/b/c.html?x=1&y=two#frag");
    CHECK(url.protocol() == "https:");
    CHECK(url.host() == "example.com:8080" && url.hostname() == "example.com" && url.port() == "8080");
    CHECK(url.pathname() == "/a/b/c.html");
    CHECK(url.search() == "?x=1&y=two" && url.hash() == "#frag");
    CHECK(url.username() == "user" && url.password() == "pw");
    CHECK(url.origin() == "https://example.com:8080");
    CHECK(url.href() == "https://user:pw@example.com:8080/a/b/c.html?x=1&y=two#frag");
    CHECK(url.toString() == url.href() && url.toJSON() == url.href());

    // Built by setting the parts.
    URL built = URL::create("http://example.org/");
    built.protocol("https");
    built.hostname("api.example.org");
    built.port("9000");
    built.pathname("/v1/items list");
    built.search("?q=a b");
    built.hash("top");
    CHECK(built.href() == "https://api.example.org:9000/v1/items%20list?q=a%20b#top");
    built.port("");
    CHECK(built.host() == "api.example.org");
    built.port("443");
    CHECK(built.port() == "");
    built.hash("");
    CHECK(built.hash() == "");
    built.href("ftp://files.example.net/pub/");
    CHECK(built.protocol() == "ftp:" && built.pathname() == "/pub/");

    // Defaults and normalisation.
    CHECK(URL::create("HTTP://EXAMPLE.com").href() == "http://example.com/");
    CHECK(URL::create("http://example.com:80/x").port() == "");
    CHECK(URL::create("https://example.com/a/./b/../c").pathname() == "/a/c");
    CHECK(URL::create("https://example.com/caf\xc3\xa9").pathname() == "/caf%C3%A9");

    // Relative resolution against a base.
    URL base = URL::create("https://example.com/dir/page.html?old=1#h");
    CHECK(URL::create("other.html", base).href() == "https://example.com/dir/other.html");
    CHECK(URL::create("/root", base).href() == "https://example.com/root");
    CHECK(URL::create("../up", base).href() == "https://example.com/up");
    CHECK(URL::create("?new=2", base).href() == "https://example.com/dir/page.html?new=2");
    CHECK(URL::create("#only", base).href() == "https://example.com/dir/page.html?old=1#only");
    CHECK(URL::create("//cdn.example.com/lib.js", base).href() == "https://cdn.example.com/lib.js");
    CHECK(URL::create("../x", std::string("https://h.example/a/b/c")).href() == "https://h.example/a/x");

    // canParse and parse: no exception, a value or nothing.
    CHECK(URL::canParse("https://example.com"));
    CHECK(!URL::canParse("not a url"));
    CHECK(URL::canParse("/relative", "https://example.com"));
    CHECK(!URL::canParse("/relative"));
    CHECK(URL::parse("mailto:a@b.c")->protocol() == "mailto:");
    CHECK(!URL::parse("::::"));
    CHECK(URL::parse("x", "http://e.com/d/")->pathname() == "/d/x");

    // An invalid URL throws; the exception is kept.
    ts::Result<URL> bad = ts::attempt([] { return URL::create("nope"); });
    CHECK(!bad);
    CHECK(!bad && bad.error().name() == "TypeError");

    // The page's own address is a URL, too.
    URL here = URL::create(get_location().href());
    CHECK(here.protocol() == "http:" && (here.hostname() == "localhost" || here.hostname() == "127.0.0.1"));
    CHECK(URL::create("rel.txt", here).href().ends_with("/rel.txt"));

    // URLSearchParams: get, getAll, has, append.
    URLSearchParams params = URLSearchParams::create(std::string("a=1&b=2&a=3&c=%20x+y"));
    CHECK(params.get("a") == "1" && params.get("b") == "2");
    CHECK(!params.get("zzz"));
    CHECK(params.getAll("a").size() == 2 && params.getAll("a")[1] == "3");
    CHECK(params.get("c") == " x y");
    CHECK(params.has("b") && !params.has("d"));
    CHECK(params.has("a", "3") && !params.has("a", "9"));
    CHECK(params.size() == 4);
    params.append("d", "4");
    CHECK(params.has("d") && params.size() == 5);

    // set replaces every value of a name; delete removes them.
    params.set("a", "9");
    CHECK(params.getAll("a").size() == 1 && params.get("a") == "9");
    params.delete_("d");
    CHECK(!params.has("d"));
    params.delete_("b", "nope");
    CHECK(params.has("b"));
    params.delete_("b", "2");
    CHECK(!params.has("b"));

    // toString encodes; sort orders by name.
    URLSearchParams q = URLSearchParams::create();
    q.append("z", "last");
    q.append("m", "a b&c");
    q.append("a", "1");
    CHECK(q.toString() == "z=last&m=a+b%26c&a=1");
    q.sort();
    CHECK(q.toString() == "a=1&m=a+b%26c&z=last");

    // Iteration: entries in order.
    std::string seen;
    for (ts::Tuple<std::string, std::string> kv : q) seen += kv.get<0>() + ":" + kv.get<1>() + ";";
    CHECK(seen == "a:1;m:a b&c;z:last;");
    std::string keys;
    for (std::string k : q.keys()) keys += k;
    CHECK(keys == "amz");
    std::string values;
    for (std::string v : q.values()) values += v + "|";
    CHECK(values == "1|a b&c|last|");

    // forEach with a callback.
    std::string each;
    ts::Scoped<URLSearchParams_forEach_callbackfn> cb = [&](std::string value, std::string key, URLSearchParams) { each += key + "=" + value + ","; };
    q.forEach(cb);
    CHECK(each == "a=1,m=a b&c,z=last,");

    // The searchParams of a URL are live both ways.
    URL link = URL::create("https://example.com/search?q=cats");
    URLSearchParams live = link.searchParams();
    CHECK(live.get("q") == "cats");
    live.append("page", "2");
    CHECK(link.search() == "?q=cats&page=2");
    link.search("?q=dogs");
    CHECK(live.get("q") == "dogs" && !live.has("page"));
    live.set("q", "a&b");
    CHECK(link.href() == "https://example.com/search?q=a%26b");
}
