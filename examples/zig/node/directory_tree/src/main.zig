const std = @import("std");
const b = @import("node");

extern "env" fn print(ptr: [*]const u8, len: u32) void;

var passed: u32 = 0;

fn check(ok: bool) void {
    if (!ok) @trap();
    passed += 1;
}

fn println(s: []const u8) void {
    print(s.ptr, @intCast(s.len));
    print("\n", 1);
}

fn str(text: []const u8) b.JsString {
    return b.JsString.from(text);
}

fn at(p: b.JsString) b.fs.PathLike {
    return .{ .string = p };
}

fn join2(a: b.JsString, c: []const u8) b.JsString {
    return b.path.join(b.JsArrayOf(b.JsString).of(&.{ a, str(c) }));
}

fn join3(a: b.JsString, c: []const u8, d: []const u8) b.JsString {
    return b.path.join(b.JsArrayOf(b.JsString).of(&.{ a, str(c), str(d) }));
}

var dir_count: u32 = 0;
var file_count: u32 = 0;
var total_bytes: f64 = 0;

fn walkDir(dir: b.JsString) void {
    const entries = b.fs.readdirSync(at(dir), null);
    var i: usize = 0;
    while (i < entries.len()) : (i += 1) {
        var nbuf: [128]u8 = undefined;
        const name = entries.get(i).copy(&nbuf);
        const full = join2(dir, name);
        const st = b.fs.statSync(at(full));
        if (st.isDirectory()) {
            dir_count += 1;
            walkDir(full);
        } else if (st.isFile()) {
            file_count += 1;
            total_bytes += st.getSize();
        }
    }
}

export fn start() void {
    const root = b.fs.mkdtempSync("/tmp/tsrs-node-tree-", null);
    check(b.fs.existsSync(at(root)));

    const mk = b.mkdir_options.create();
    _ = b.fs.mkdirSyncRecursive(at(join3(root, "src", "core")), mk);
    _ = b.fs.mkdirSyncRecursive(at(join3(root, "src", "cli")), mk);
    _ = b.fs.mkdirSyncRecursive(at(join2(root, "docs")), mk);

    b.fs.writeFileSync(.{ .string = join2(root, "README.md") }, b.JsValue.string("node tree demo"), null);
    b.fs.writeFileSync(.{ .string = join3(root, "src/core", "engine.txt") }, b.JsValue.string("engine core module!\n"), null);
    b.fs.writeFileSync(.{ .string = join3(root, "src/cli", "args.txt") }, b.JsValue.string("cli args parser\n"), null);
    b.fs.writeFileSync(.{ .string = join2(root, "docs/guide.md") }, b.JsValue.string("monochrome documentation\n"), null);

    walkDir(root);

    check(dir_count == 4);
    check(file_count == 4);
    check(total_bytes == 75);

    const rm = b.fs.RmOptions.create();
    rm.setRecursive(true);
    rm.setForce(true);
    b.fs.rmSync(at(root), .{ .options = rm });
    check(!b.fs.existsSync(at(root)));
    check(dir_count + file_count == 8);
    check(true);

    var rbuf: [128]u8 = undefined;
    var line_buf: [256]u8 = undefined;
    println(std.fmt.bufPrint(&line_buf, "root={s}", .{root.copy(&rbuf)}) catch "");
    println(std.fmt.bufPrint(&line_buf, "dirs={d} files={d} total_bytes={d:.0}", .{
        dir_count,
        file_count,
        total_bytes,
    }) catch "");
    println(std.fmt.bufPrint(&line_buf, "PASS: directory_tree ({d} checks)", .{passed}) catch "");
}
