// localStorage and sessionStorage from Zig: items set, read, removed and
// counted, and a JSON text kept and read back. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

fn has(s: b.Storage, key: []const u8, want: []const u8) bool {
    return if (s.getItem(key)) |v| v.eql(want) else false;
}

export fn run() void {
    const local = b.getLocalStorage();
    const session = b.getSessionStorage();
    local.clear();
    session.clear();
    expect(local.getLength() == 0 and session.getLength() == 0, @src());

    // Set and get.
    local.setItem("name", "ada");
    expect(local.getLength() == 1, @src());
    expect(has(local, "name", "ada"), @src());
    expect(local.getItem("missing") == null, @src());

    // Setting again replaces the value, and does not add a key.
    local.setItem("name", "grace");
    expect(has(local, "name", "grace") and local.getLength() == 1, @src());

    // Values are strings: numbers and empty text keep their text.
    local.setItem("count", "42");
    local.setItem("empty", "");
    expect(has(local, "count", "42"), @src());
    expect(has(local, "empty", "") and local.getItem("empty") != null, @src());
    expect(local.getLength() == 3, @src());

    // key(i) names the keys, in some order, each once.
    var seen: [3]bool = .{ false, false, false };
    var i: f64 = 0;
    while (i < 3) : (i += 1) {
        const k = local.key(i).?;
        if (k.eql("name")) seen[0] = true;
        if (k.eql("count")) seen[1] = true;
        if (k.eql("empty")) seen[2] = true;
    }
    expect(seen[0] and seen[1] and seen[2], @src());
    expect(local.key(3) == null, @src());

    // Removed.
    local.removeItem("count");
    expect(local.getItem("count") == null and local.getLength() == 2, @src());
    local.removeItem("never-there");
    expect(local.getLength() == 2, @src());

    // The two storages are separate.
    expect(session.getItem("name") == null and session.getLength() == 0, @src());
    session.setItem("name", "session-value");
    expect(has(session, "name", "session-value") and has(local, "name", "grace"), @src());
    session.setItem("only-session", "1");
    expect(session.getLength() == 2 and local.getItem("only-session") == null, @src());

    // Text that is not ASCII, and long text, comes back whole.
    local.setItem("unicode", "h\xc3\xa9llo \xe2\x9c\x93 \xf0\x9f\x98\x80");
    expect(has(local, "unicode", "h\xc3\xa9llo \xe2\x9c\x93 \xf0\x9f\x98\x80"), @src());
    var long: [4096]u8 = undefined;
    for (&long, 0..) |*c, n| c.* = 'a' + @as(u8, @intCast(n % 26));
    local.setItem("long", &long);
    var back: [4096]u8 = undefined;
    expect(std.mem.eql(u8, local.getItem("long").?.read(&back), &long), @src());
    expect(local.getItem("long").?.len() == 4096, @src());

    // A key can be any text, an empty one too.
    local.setItem("", "blank key");
    expect(has(local, "", "blank key"), @src());
    local.setItem("a key with spaces/and=signs", "ok");
    expect(has(local, "a key with spaces/and=signs", "ok"), @src());

    // A JSON text stored and read back is the same text, and still the same JSON.
    const json = "{\"name\":\"ada\",\"langs\":[\"zig\",\"rust\"],\"n\":3.5,\"ok\":true,\"none\":null}";
    local.setItem("profile", json);
    var json_back: [128]u8 = undefined;
    const stored = local.getItem("profile").?.read(&json_back);
    expect(std.mem.eql(u8, stored, json), @src());
    // Stored under another key from the copy read back, it is still the same text.
    local.setItem("profile-copy", stored);
    expect(has(local, "profile-copy", json), @src());

    // The index form of access reads and writes the same items.
    expect(local.getAt("name").toString().eql("grace"), @src());
    local.setAt("via-index", b.JsValue.string("indexed"));
    expect(has(local, "via-index", "indexed"), @src());
    // Reading what is not there through the index form gives undefined, not a string.
    expect(local.getAt("not-set").isNull(), @src());

    // Cleared.
    const before = local.getLength();
    expect(before > 5, @src());
    local.clear();
    expect(local.getLength() == 0 and local.getItem("name") == null, @src());
    expect(session.getLength() == 2, @src());
    session.clear();
    expect(session.getLength() == 0, @src());
}
