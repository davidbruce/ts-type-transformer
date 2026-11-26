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

export fn start() void {
    const scope = b.JsScope.open();
    defer scope.deinit();

    const dir = b.fs.mkdtempSync("/tmp/tsrs-node-ws-", null);
    check(b.fs.existsSync(dir) and b.fs.statSync(dir).isDirectory());

    const notes = b.path.join(&.{ dir, "notes.txt" });
    b.fs.writeFileSync(notes, "alpha\nbeta\n", null);
    const st1 = b.fs.statSync(notes);
    check(b.fs.existsSync(notes) and st1.isFile() and st1.getSize() == 11);

    b.fs.appendFileSync(notes, "gamma\n", null);
    const text = b.fs.readFileSyncString(notes, .{ .BufferEncoding = .utf8 });
    const st2 = b.fs.statSync(notes);
    check(text.eql("alpha\nbeta\ngamma\n") and st2.getSize() == 17);

    const backup = b.path.join(&.{ dir, "backup.txt" });
    const archive = b.path.join(&.{ dir, "archive.txt" });
    b.fs.copyFileSync(notes, backup, null);
    check(b.fs.existsSync(backup));

    b.fs.renameSync(backup, archive);
    check(!b.fs.existsSync(backup) and b.fs.existsSync(archive));

    const arch_text = b.fs.readFileSyncString(archive, .{ .BufferEncoding = .utf8 });
    check(b.jsSame(arch_text, text));

    b.fs.unlinkSync(archive);
    check(!b.fs.existsSync(archive));

    b.fs.rmSync(dir, .{ .recursive = true, .force = true });
    check(!b.fs.existsSync(dir));
    check(true);

    var line_buf: [256]u8 = undefined;
    println(std.fmt.bufPrint(&line_buf, "workspace={f}", .{dir}) catch "");
    println(std.fmt.bufPrint(&line_buf, "size_before={d:.0} size_after={d:.0}", .{ st1.getSize(), st2.getSize() }) catch "");
    println(std.fmt.bufPrint(&line_buf, "PASS: temp_workspace ({d} checks)", .{passed}) catch "");
}
