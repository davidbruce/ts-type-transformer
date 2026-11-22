// Names that would be one Zig identifier, each given its own. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

export fn run() void {
    const g = b.Gizmo.new();
    expect(g.isTransforming2(), @src());
    expect(!b.Gizmo.isTransforming(), @src());
    // `fooBar` and `foo_bar` are one `fooBar`.
    expect(g.fooBar() == 1 and g.fooBar2() == 2, @src());

    const s = b.shape();
    expect(s.area() == 3 and b.ShapeArea(s) == 30, @src());

    const p = b.point(4);
    expect(p.getX() == 4, @src());
    expect(p.create2(5).getX() == 5, @src());
    expect(b.Point.create().h != 0, @src());

    expect(b.ns_Node.new().id() == 2, @src());
    // The namespace's `Node`, in a struct of its namespace.
    expect(b.ns.Node.new().id() == 1, @src());
}
