const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

export fn run() void {
    const rw = b.Perms.Read.@"or"(.Write);
    expect(rw == b.Perms.ReadWrite, @src());
    expect(rw.has(.Read) and rw.has(.Write) and !rw.has(.Exec), @src());
    expect(rw.@"and"(.Read) == b.Perms.Read, @src());
    expect(rw.xor(.Read) == b.Perms.Write, @src());
    expect(b.Perms.fromBits(5).bits() == 5, @src());

    const re = b.combinePerms(.Read, .Exec);
    expect(b.hasPerm(re, .Read) and b.hasPerm(re, .Exec) and !b.hasPerm(re, .Write), @src());
    expect(b.defaultPerms(null) == b.Perms.Read.@"or"(.Exec), @src());
    expect(b.defaultPerms(.Write) == b.Perms.Write, @src());
}
