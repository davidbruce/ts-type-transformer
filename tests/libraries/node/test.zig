// Node's fs, path and events from Zig. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

fn str(text: []const u8) b.JsString {
    return b.JsString.from(text);
}

// A path as the union fs wants.
fn at(p: b.JsString) b.fs.PathLike {
    return .{ .string = p };
}

var hits: u32 = 0;
var last: f64 = 0;
var argc: usize = 0;
fn onTick(ctx: ?*anyopaque, args: b.JsArray) void {
    const total: *u32 = @ptrCast(@alignCast(ctx));
    total.* += 1;
    // The listener's rest parameter arrives as the array of all the emit arguments.
    argc = args.len();
    last = args.get(argc - 1).toNumber();
}
var once_hits: u32 = 0;
var other_hits: u32 = 0;

export fn run() void {
    // path.
    const parts = b.JsArrayOf(b.JsString).new();
    parts.push(str("/a"));
    parts.push(str("b/../c"));
    parts.push(str("d.txt"));
    expect(b.path.join(parts).eql("/a/c/d.txt"), @src());
    expect(b.path.normalize("/a//b/./c/..").eql("/a/b"), @src());
    expect(b.path.basename("/x/y/file.txt", .{}).eql("file.txt"), @src());
    expect(b.path.basename("/x/y/file.txt", .{ .suffix = ".txt" }).eql("file"), @src());
    expect(b.path.extname("archive.tar.gz").eql(".gz"), @src());
    expect(b.path.dirname("/x/y/file.txt").eql("/x/y"), @src());
    expect(b.path.isAbsolute("/x") and !b.path.isAbsolute("x/y"), @src());
    expect(b.path.relative("/a/b/c", "/a/d").eql("../../d"), @src());
    const abs = b.JsArrayOf(b.JsString).of(&.{ str("/r"), str("s"), str("../t") });
    expect(b.path.resolve(abs).eql("/r/t"), @src());
    const parsed = b.path.parse("/home/u/doc.md");
    expect(parsed.getRoot().eql("/") and parsed.getDir().eql("/home/u"), @src());
    expect(parsed.getBase().eql("doc.md") and parsed.getExt().eql(".md") and parsed.getName().eql("doc"), @src());
    const fmt = b.path.FormatInputPathObject.create();
    fmt.setDir("/tmp");
    fmt.setBase("x.js");
    expect(b.path.format(fmt).eql("/tmp/x.js"), @src());
    expect(b.path.getSep() == .@"/" and b.path.getDelimiter() == .@":", @src());
    expect(b.path.matchesGlob("/a/b.txt", "/a/*.txt"), @src());

    // fs: a scratch directory, files in it, and cleaning up.
    const dir = b.fs.mkdtempSync("target/e2e/node-zig-", null);
    expect(dir.len() > "target/e2e/node-zig-".len, @src());
    expect(b.fs.existsSync(at(dir)), @src());
    const stat_dir = b.fs.statSync(at(dir));
    expect(stat_dir.isDirectory() and !stat_dir.isFile(), @src());

    var path_buf: [256]u8 = undefined;
    const dir_text = dir.copy(&path_buf);
    var hello_buf: [320]u8 = undefined;
    const hello_path = std.fmt.bufPrint(&hello_buf, "{s}/hello.txt", .{dir_text}) catch "";
    b.fs.writeFileSync(.{ .string = str(hello_path) }, b.JsValue.string("hello, node\nsecond line\n"), null);
    expect(b.fs.existsSync(at(str(hello_path))), @src());
    const text = b.fs.readFileSyncOrBufferEncoding(.{ .string = str(hello_path) }, .{ .BufferEncoding = .utf8 });
    expect(text.eql("hello, node\nsecond line\n"), @src());
    expect(b.fs.statSync(at(str(hello_path))).isFile(), @src());
    expect(b.fs.statSync(at(str(hello_path))).getSize() == 24, @src());

    // Append, copy, rename, a nested directory and a listing.
    b.fs.appendFileSync(.{ .string = str(hello_path) }, .{ .string = str("third\n") }, null);
    expect(b.fs.statSync(at(str(hello_path))).getSize() == 30, @src());
    var copy_buf: [320]u8 = undefined;
    const copy_path = std.fmt.bufPrint(&copy_buf, "{s}/copy.txt", .{dir_text}) catch "";
    b.fs.copyFileSync(at(str(hello_path)), at(str(copy_path)), .{});
    var renamed_buf: [320]u8 = undefined;
    const renamed_path = std.fmt.bufPrint(&renamed_buf, "{s}/renamed.txt", .{dir_text}) catch "";
    b.fs.renameSync(at(str(copy_path)), at(str(renamed_path)));
    expect(!b.fs.existsSync(at(str(copy_path))) and b.fs.existsSync(at(str(renamed_path))), @src());
    var nested_buf: [320]u8 = undefined;
    const nested = std.fmt.bufPrint(&nested_buf, "{s}/a/b/c", .{dir_text}) catch "";
    // The recursive overload: its options already hold recursive: true.
    const made = b.fs.mkdirSyncRecursive(at(str(nested)), b.mkdir_options.create());
    expect(made != null and made.?.len() > 0, @src());
    expect(b.fs.statSync(at(str(nested))).isDirectory(), @src());

    const names = b.fs.readdirSync(at(dir), null);
    expect(names.len() == 3, @src());
    var seen: u8 = 0;
    var i: usize = 0;
    while (i < names.len()) : (i += 1) {
        const n = names.get(i);
        if (n.eql("a")) seen |= 1;
        if (n.eql("hello.txt")) seen |= 2;
        if (n.eql("renamed.txt")) seen |= 4;
    }
    expect(seen == 7, @src());

    // Errors: a missing file throws ENOENT; the call returns an empty value and the error is kept.
    var missing_buf: [320]u8 = undefined;
    const missing = std.fmt.bufPrint(&missing_buf, "{s}/nope.txt", .{dir_text}) catch "";
    expect(!b.fs.existsSync(at(str(missing))), @src());
    _ = b.fs.statSync(at(str(missing)));
    const err = b.jsTakeError();
    expect(err != null, @src());
    if (err) |e| {
        defer b.jsRelease(e);
        const msg = b.jsErrorMessage(e);
        defer b.jsRelease(msg);
        var mbuf: [256]u8 = undefined;
        expect(std.mem.indexOf(u8, msg.copy(&mbuf), "ENOENT") != null, @src());
    }

    // Remove it all; the directory is gone.
    b.fs.unlinkSync(at(str(renamed_path)));
    expect(!b.fs.existsSync(at(str(renamed_path))), @src());
    const rm = b.fs.RmOptions.create();
    rm.setRecursive(true);
    rm.setForce(true);
    b.fs.rmSync(at(dir), .{ .options = rm });
    expect(!b.fs.existsSync(at(dir)), @src());

    // events: an EventEmitter.
    const em = b.events.EventEmitter.new(null);
    const tick = b.JsValue.string("tick");
    expect(em.listenerCount(tick, .{}) == 0, @src());
    const listener = b.any_array_fn.from(onTick, &hits);
    _ = em.on(tick, listener);
    expect(em.listenerCount(tick, .{}) == 1, @src());
    const args = b.JsArray.new();
    args.push(b.JsValue.number(7));
    expect(em.emit(tick, args), @src());
    expect(hits == 1 and last == 7 and argc == 1, @src());
    args.push(b.JsValue.number(9));
    _ = em.emit(tick, args);
    // Every argument arrives, not just the first.
    expect(hits == 2 and argc == 2 and last == 9, @src());

    // once fires one time only.
    const once_listener = b.any_array_fn.from(onTick, &once_hits);
    _ = em.once(tick, once_listener);
    expect(em.listenerCount(tick, .{}) == 2, @src());
    _ = em.emit(tick, args);
    _ = em.emit(tick, args);
    expect(once_hits == 1 and hits == 4, @src());
    expect(em.listenerCount(tick, .{}) == 1, @src());

    // An event nobody listens to: emit says so.
    expect(!em.emit(b.JsValue.string("quiet"), b.JsArray.new()), @src());

    // removeListener, with the same function and context, is the same listener.
    _ = em.removeListener(tick, b.any_array_fn.from(onTick, &hits));
    expect(em.listenerCount(tick, .{}) == 0, @src());
    _ = em.emit(tick, args);
    expect(hits == 4, @src());

    // eventNames and removeAllListeners; a second event, and prepend order.
    _ = em.on(b.JsValue.string("a"), b.any_array_fn.from(onTick, &other_hits));
    _ = em.on(b.JsValue.string("b"), b.any_array_fn.from(onTick, &other_hits));
    expect(em.eventNames().len() == 2, @src());
    _ = em.removeAllListeners(b.JsValue.string("a"));
    expect(em.eventNames().len() == 1, @src());
    _ = em.removeAllListeners(null);
    expect(em.eventNames().len() == 0, @src());
    expect(em.listeners(tick).len() == 0, @src());

    // The default max listeners is ten.
    expect(em.getMaxListeners() == 10, @src());
    _ = em.setMaxListeners(20);
    expect(em.getMaxListeners() == 20, @src());
}
