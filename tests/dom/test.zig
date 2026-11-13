const std = @import("std");
const b = @import("bindings.zig");
export fn run() void {
    // Reference every declaration so the whole module is checked, not just what is used.
    @setEvalBranchQuota(100_000_000);
    comptime {
        for (std.meta.declarations(b)) |name| {
            const v = @field(b, name);
            if (@TypeOf(v) == type and @typeInfo(v) == .@"struct") {
                for (std.meta.declarations(v)) |member| _ = &@field(v, member);
            } else _ = &v;
        }
    }
}
