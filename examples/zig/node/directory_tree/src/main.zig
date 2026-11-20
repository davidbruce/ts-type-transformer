const std = @import("std");
const b = @import("node");

extern "env" fn print(ptr: [*]const u8, len: u32) void;

const TreeStats = struct {
    passed: u32 = 0,
    dirs: u32 = 0,
    files: u32 = 0,
    total_bytes: f64 = 0,

    fn check(self: *TreeStats, ok: bool) void {
        if (!ok) @trap();
        self.passed += 1;
    }
};

fn println(s: []const u8) void {
    print(s.ptr, @intCast(s.len));
    print("\n", 1);
}

fn walkDir(stats: *TreeStats, dir: b.JsString) void {
    const entries = b.fs.readdirSync(dir, null);
    var nbuf: [128]u8 = undefined;
    for (0..entries.len()) |i| {
        const name = entries.get(i).copy(&nbuf);
        const full = b.path.join(&.{ dir, name });
        const st = b.fs.statSync(full);
        if (st.isDirectory()) {
            stats.dirs += 1;
            walkDir(stats, full);
        } else if (st.isFile()) {
            stats.files += 1;
            stats.total_bytes += st.getSize();
        }
    }
}

export fn start() void {
    const scope = b.JsScope.open();
    defer scope.deinit();

    var stats: TreeStats = .{};
    const root = b.fs.mkdtempSync("/tmp/tsrs-node-tree-", null);
    stats.check(b.fs.existsSync(root));

    _ = b.fs.mkdirSyncRecursive(b.path.join(&.{ root, "src", "core" }), .{});
    _ = b.fs.mkdirSyncRecursive(b.path.join(&.{ root, "src", "cli" }), .{});
    _ = b.fs.mkdirSyncRecursive(b.path.join(&.{ root, "docs" }), .{});

    b.fs.writeFileSync(b.path.join(&.{ root, "README.md" }), "node tree demo", null);
    b.fs.writeFileSync(b.path.join(&.{ root, "src/core", "engine.txt" }), "engine core module!\n", null);
    b.fs.writeFileSync(b.path.join(&.{ root, "src/cli", "args.txt" }), "cli args parser\n", null);
    b.fs.writeFileSync(b.path.join(&.{ root, "docs/guide.md" }), "monochrome documentation\n", null);

    walkDir(&stats, root);

    stats.check(stats.dirs == 4);
    stats.check(stats.files == 4);
    stats.check(stats.total_bytes == 75);

    b.fs.rmSync(root, .{ .recursive = true, .force = true });
    stats.check(!b.fs.existsSync(root));
    stats.check(stats.dirs + stats.files == 8);
    stats.check(true);

    var line_buf: [256]u8 = undefined;
    println(std.fmt.bufPrint(&line_buf, "root={f}", .{root}) catch "");
    println(std.fmt.bufPrint(&line_buf, "dirs={d} files={d} total_bytes={d:.0}", .{
        stats.dirs,
        stats.files,
        stats.total_bytes,
    }) catch "");
    println(std.fmt.bufPrint(&line_buf, "PASS: directory_tree ({d} checks)", .{stats.passed}) catch "");
}
