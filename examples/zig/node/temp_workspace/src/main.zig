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

export fn start() void {
    const dir = b.fs.mkdtempSync("/tmp/tsrs-node-ws-", null);
    check(b.fs.existsSync(at(dir)) and b.fs.statSync(at(dir)).isDirectory());

    const notes = join2(dir, "notes.txt");
    b.fs.writeFileSync(.{ .string = notes }, b.JsValue.string("alpha\nbeta\n"), null);
    const st1 = b.fs.statSync(at(notes));
    check(b.fs.existsSync(at(notes)) and st1.isFile() and st1.getSize() == 11);

    b.fs.appendFileSync(.{ .string = notes }, .{ .string = str("gamma\n") }, null);
    const encoding: b.global_BufferEncoding = .{ .h = str("utf8").h };
    const text = b.fs.readFileSyncOrBufferEncoding(.{ .string = notes }, .{ .global_BufferEncoding = encoding });
    const st2 = b.fs.statSync(at(notes));
    check(text.eql("alpha\nbeta\ngamma\n") and st2.getSize() == 17);

    const backup = join2(dir, "backup.txt");
    const archive = join2(dir, "archive.txt");
    b.fs.copyFileSync(at(notes), at(backup), .{});
    check(b.fs.existsSync(at(backup)));

    b.fs.renameSync(at(backup), at(archive));
    check(!b.fs.existsSync(at(backup)) and b.fs.existsSync(at(archive)));

    const arch_text = b.fs.readFileSyncOrBufferEncoding(.{ .string = archive }, .{ .global_BufferEncoding = encoding });
    var tbuf: [128]u8 = undefined;
    check(arch_text.eql(text.copy(&tbuf)));

    b.fs.unlinkSync(at(archive));
    check(!b.fs.existsSync(at(archive)));

    const rm = b.fs.RmOptions.create();
    rm.setRecursive(true);
    rm.setForce(true);
    b.fs.rmSync(at(dir), .{ .options = rm });
    check(!b.fs.existsSync(at(dir)));
    check(true);

    var dbuf: [128]u8 = undefined;
    var line_buf: [256]u8 = undefined;
    println(std.fmt.bufPrint(&line_buf, "workspace={s}", .{dir.copy(&dbuf)}) catch "");
    println(std.fmt.bufPrint(&line_buf, "size_before={d:.0} size_after={d:.0}", .{ st1.getSize(), st2.getSize() }) catch "");
    println(std.fmt.bufPrint(&line_buf, "PASS: temp_workspace ({d} checks)", .{passed}) catch "");
}
