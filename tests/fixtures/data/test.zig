// Calls the generated Zig bindings; every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

fn strEq(s: b.JsString, expected: []const u8) bool {
    var buf: [128]u8 = undefined;
    if (s.len() > buf.len) return false;
    return std.mem.eql(u8, s.read(&buf), expected);
}

fn doubled(_: ?*anyopaque, value: b.JsValue) callconv(.c) b.JsValue {
    return b.JsValue.number(value.toNumber() * 2);
}

export fn run() void {
    // Option objects: built here, read by JavaScript.
    const o = b.Options.create();
    o.set_width(3);
    o.set_mode(.fill);
    expect(strEq(b.describe(o), "3x? fill"), @src());
    o.set_height(b.JsValue.number(2));
    o.set_title("hi");
    expect(strEq(b.describe(o), "3x2 fill \"hi\""), @src());

    // Built by JavaScript, read here. A missing optional property is null.
    const d = b.defaults();
    expect(d.get_width() == 4, @src());
    expect(d.get_mode() == .fit, @src());
    expect(d.get_height().isNull(), @src());

    // Partial<Options>.
    const changes = b.patch_changes.create();
    changes.set_width(b.JsValue.number(9));
    const patched = b.patch(d, changes);
    expect(patched.get_width() == 9, @src());
    expect(patched.get_mode() == .fit, @src());

    // Pick<Options, "width" | "height">.
    const size = b.area_size.create();
    size.set_width(3);
    expect(b.area(size) == 9, @src());
    size.set_height(b.JsValue.number(5));
    expect(b.area(size) == 15, @src());

    // keyof Options.
    expect(b.field() == .title, @src());

    // Record<string, number>.
    const table = b.lookup_table.create();
    table.set_at("a", 7);
    expect(b.lookup(table, "a") == 7, @src());
    expect(b.lookup(table, "b") == -1, @src());
    expect(table.get_at("a").toNumber() == 7, @src());
    expect(table.get_at("b").isNull(), @src());

    // An intersection of object types.
    const tagged = b.Tagged.create();
    tagged.set_name("x");
    tagged.set_id(5);
    expect(strEq(b.show(tagged), "x#5"), @src());

    // Inline object types.
    const point = b.norm_point.create();
    point.set_x(3);
    point.set_y(4);
    expect(b.norm(point) == 5, @src());
    expect(b.origin().get_x() == 0, @src());

    // Arrays are typed: number[] is JsArrayOf(f64).
    const r = b.range(4);
    expect(r.len() == 4, @src());
    expect(r.get(3) == 3, @src());
    expect(b.sum(r) == 6, @src());
    const mine = b.JsArrayOf(f64).of(&.{ 10, 5 });
    expect(b.sum(mine) == 15, @src());
    const w = b.words("a bc");
    expect(w.len() == 2, @src());
    expect(strEq(w.get(1), "bc"), @src());

    // Tuples are arrays.
    const p = b.pair(1, "one");
    expect(p.get(0).toNumber() == 1, @src());
    const swapped = b.swap(p);
    expect(strEq(swapped.get(0).toString(), "one"), @src());
    expect(swapped.get(1).toNumber() == 1, @src());

    // Arrays of objects.
    const all = b.items();
    expect(all.len() == 2, @src());
    const item = all.get(0);
    expect(item.get_id() == 1, @src());
    expect(item.get_tags().len() == 2, @src());

    // Generics are erased: values are untyped.
    const bx = b.wrap(b.JsValue.number(21));
    expect(bx.get_value().toNumber() == 21, @src());
    expect(bx.map(b.Box_map_f.from(doubled, null)).get_value().toNumber() == 42, @src());
    expect(b.first(r.any()).toNumber() == 0, @src());
    expect(b.first(b.JsArray.new()).isNull(), @src());

    // A union as an array element: Id is number | string.
    const some = b.ids();
    expect(some.get(0).number == 1 and strEq(some.get(1).string, "two"), @src());

    // A handle a function returns is the caller's to release.
    const live = b.jsLiveHandles();
    for (0..1000) |i| {
        const x: f64 = @floatFromInt(i);
        // Made here, so released here; wrap only borrows it.
        const n = b.JsValue.number(x);
        defer b.jsRelease(n);
        const t = b.wrap(n);
        defer b.jsRelease(t);
        const v = t.get_value();
        defer b.jsRelease(v);
        const parts = b.words("a b");
        defer b.jsRelease(parts);
        const second = parts.get(1);
        defer b.jsRelease(second);
        expect(v.toNumber() == x and strEq(second, "b"), @src());
    }
    expect(b.jsLiveHandles() == live, @src());
}
