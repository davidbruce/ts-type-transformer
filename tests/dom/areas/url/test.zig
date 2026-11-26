// URL and URLSearchParams from Zig: parsed, built, resolved against a base and
// iterated. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

fn str(s: []const u8) b.UrlInput {
    return .{ .string = b.JsString.from(s) };
}

fn url(s: []const u8) b.URL {
    return b.URL.new(str(s), .{});
}

fn resolve(rel: []const u8, base: []const u8) b.URL {
    return b.URL.new(str(rel), .{ .base = str(base) });
}

// What forEach saw, joined as "name=value;".
var joined: [64]u8 = undefined;
var joined_len: usize = 0;
fn onEach(_: ?*anyopaque, value: b.JsString, name: b.JsString, _: b.URLSearchParams) void {
    var buf: [16]u8 = undefined;
    for (name.copy(&buf)) |c| {
        joined[joined_len] = c;
        joined_len += 1;
    }
    joined[joined_len] = '=';
    joined_len += 1;
    for (value.copy(&buf)) |c| {
        joined[joined_len] = c;
        joined_len += 1;
    }
    joined[joined_len] = ';';
    joined_len += 1;
}

export fn run() void {
    // A URL taken apart.
    const u = url("https://user:pw@example.com:8080/a/b/c.html?x=1&y=two#frag");
    expect(u.getProtocol().eql("https:"), @src());
    expect(u.getUsername().eql("user") and u.getPassword().eql("pw"), @src());
    expect(u.getHost().eql("example.com:8080"), @src());
    expect(u.getHostname().eql("example.com") and u.getPort().eql("8080"), @src());
    expect(u.getPathname().eql("/a/b/c.html"), @src());
    expect(u.getSearch().eql("?x=1&y=two") and u.getHash().eql("#frag"), @src());
    expect(u.getOrigin().eql("https://example.com:8080"), @src());
    expect(u.getHref().eql("https://user:pw@example.com:8080/a/b/c.html?x=1&y=two#frag"), @src());
    expect(u.toString().eql(u.getHref().copy(&joined)), @src());

    // Defaults are dropped: https on 443 has no port.
    const plain = url("https://example.com:443/");
    expect(plain.getPort().len() == 0 and plain.getHost().eql("example.com"), @src());
    expect(url("HTTP://EXAMPLE.com/Path").getHref().eql("http://example.com/Path"), @src());

    // A URL put together by its setters.
    const built = url("http://localhost/");
    built.setProtocol("https:");
    built.setHostname("api.example.org");
    built.setPort("9000");
    built.setPathname("/v1/items list");
    built.setSearch("q=5");
    built.setHash("top");
    expect(built.getHref().eql("https://api.example.org:9000/v1/items%20list?q=5#top"), @src());
    built.setHost("example.net");
    built.setUsername("me");
    expect(built.getHref().eql("https://me@example.net:9000/v1/items%20list?q=5#top"), @src());
    built.setHref("ftp://files.example.com/pub");
    expect(built.getProtocol().eql("ftp:") and built.getPathname().eql("/pub"), @src());
    built.setSearch("");
    built.setHash("");
    expect(built.getSearch().len() == 0 and built.getHash().len() == 0, @src());

    // The search parameters are live: each side changes the other.
    const linked = url("https://example.com/?a=1");
    const params = linked.getSearchParams();
    params.append("b", "two words");
    expect(linked.getSearch().eql("?a=1&b=two+words"), @src());
    linked.setSearch("?z=9");
    expect(params.get("z").?.eql("9") and params.get("a") == null, @src());

    // Relative references resolved against a base.
    const base = "https://example.com/a/b/c?q=1#h";
    expect(resolve("d", base).getHref().eql("https://example.com/a/b/d"), @src());
    expect(resolve("../x?y#z", base).getHref().eql("https://example.com/a/x?y#z"), @src());
    expect(resolve("/root", base).getHref().eql("https://example.com/root"), @src());
    expect(resolve("//other.org/p", base).getHref().eql("https://other.org/p"), @src());
    expect(resolve("?only=query", base).getHref().eql("https://example.com/a/b/c?only=query"), @src());
    expect(resolve("#only", base).getHref().eql("https://example.com/a/b/c?q=1#only"), @src());
    expect(resolve("http://abs.example/x", base).getHost().eql("abs.example"), @src());
    expect(resolve("./.././../up", "https://example.com/a/b/c/d").getPathname().eql("/a/up"), @src());

    // Whether a string parses, without throwing.
    expect(b.URL.canParse(str("https://example.com/"), .{}), @src());
    expect(!b.URL.canParse(str("not a url"), .{}), @src());
    expect(b.URL.canParse(str("/path"), .{ .base = str("https://example.com") }), @src());
    expect(!b.URL.canParse(str("/path"), .{ .base = str("nope") }), @src());
    expect(b.URL.parse(str("not a url"), .{}) == null, @src());
    expect(b.URL.parse(str("mailto:a@b.c"), .{}).?.getProtocol().eql("mailto:"), @src());

    // A string that does not parse throws, and the call returns nothing.
    const none = b.URL.new(str("not a url"), .{});
    expect(none.h == 0, @src());
    const err = b.jsTakeError();
    expect(err != null and b.jsErrorName(err.?).eql("TypeError"), @src());

    // URLSearchParams: read.
    const p = b.URLSearchParams.new(.{ .string = b.JsString.from("a=1&b=2&a=3&c=x%20y") });
    expect(p.getSize() == 4, @src());
    expect(p.get("a").?.eql("1") and p.get("nope") == null, @src());
    const all = p.getAll("a");
    expect(all.len() == 2 and all.get(0).eql("1") and all.get(1).eql("3"), @src());
    expect(p.has("b", .{}) and !p.has("zzz", .{}), @src());
    expect(p.get("c").?.eql("x y"), @src());
    expect(p.has("a", .{ .value = "3" }) and !p.has("a", .{ .value = "9" }), @src());

    // Written: set replaces every value, append adds, delete removes.
    p.set("a", "9");
    expect(p.getAll("a").len() == 1 and p.get("a").?.eql("9"), @src());
    p.append("d", "&=?");
    expect(p.get("d").?.eql("&=?"), @src());
    p.delete("b", .{});
    expect(!p.has("b", .{}) and p.getSize() == 3, @src());
    expect(p.toString().eql("a=9&c=x+y&d=%26%3D%3F"), @src());

    // Sorted by name; the order of equal names is kept.
    const sorted = b.URLSearchParams.new(.{ .string = b.JsString.from("z=1&a=2&m=3&a=1") });
    sorted.sort();
    expect(sorted.toString().eql("a=2&a=1&m=3&z=1"), @src());
    sorted.delete("a", .{ .value = "2" });
    expect(sorted.toString().eql("a=1&m=3&z=1"), @src());

    // Iterated as pairs, and with forEach.
    var names: [3][2]u8 = undefined;
    var count: usize = 0;
    const it = sorted.iterate();
    while (it.next()) |pair| {
        const k = pair.get(0).copy(&names[count]);
        const v = pair.get(1);
        if (count == 1) expect(std.mem.eql(u8, k, "m") and v.eql("3"), @src());
        count += 1;
    }
    expect(count == 3, @src());
    sorted.forEach(b.URLSearchParamsForEachFn.from(onEach, null), .{});
    expect(std.mem.eql(u8, joined[0..joined_len], "a=1;m=3;z=1;"), @src());

    // Made from a plain object, from another set of params, and empty.
    const map = b.HeadersInit_map.create();
    map.setAt("k", "v");
    map.setAt("n", "1");
    expect(b.URLSearchParams.new(.{ .HeadersInit_map = map }).toString().eql("k=v&n=1"), @src());
    expect(b.URLSearchParams.new(.{ .URLSearchParams = p }).toString().eql(p.toString().copy(&joined)), @src());
    expect(b.URLSearchParams.new(null).getSize() == 0, @src());
    expect(b.URLSearchParams.new(null).toString().len() == 0, @src());
}
