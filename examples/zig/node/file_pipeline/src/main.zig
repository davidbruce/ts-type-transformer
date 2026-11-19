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

var stages: u32 = 0;
var done_count: u32 = 0;

fn onStage(_: ?*anyopaque, _: b.JsArray) void {
    stages += 1;
}

fn onDone(_: ?*anyopaque, _: b.JsArray) void {
    done_count += 1;
}

export fn start() void {
    const bus = b.events.EventEmitter.new(null);
    const ev_stage = b.JsValue.string("stage");
    const ev_done = b.JsValue.string("done");
    _ = bus.on(ev_stage, b.any_array_fn.from(onStage, null));
    _ = bus.once(ev_done, b.any_array_fn.from(onDone, null));

    const root = b.fs.mkdtempSync("/tmp/tsrs-node-pipe-", null);
    const in_dir = join2(root, "in");
    const out_dir = join2(root, "out");
    const mk = b.mkdir_options.create();
    _ = b.fs.mkdirSyncRecursive(at(in_dir), mk);
    _ = b.fs.mkdirSyncRecursive(at(out_dir), mk);

    // Stage 1: seed
    b.fs.writeFileSync(.{ .string = join2(in_dir, "a.log") }, b.JsValue.string("100 OK\n"), null);
    b.fs.writeFileSync(.{ .string = join2(in_dir, "b.log") }, b.JsValue.string("200 OK\n"), null);
    b.fs.writeFileSync(.{ .string = join2(in_dir, "c.log") }, b.JsValue.string("300 OK\n"), null);
    const stage_args = b.JsArray.new();
    stage_args.push(b.JsValue.string("seed"));
    _ = bus.emit(ev_stage, stage_args);
    check(stages == 1);

    // Stage 2: bundle
    const raw_logs = b.fs.readdirSync(at(in_dir), null);
    check(raw_logs.len() == 3);
    var logs_buf: [3][32]u8 = undefined;
    var logs_len: [3]usize = undefined;
    var idx: usize = 0;
    while (idx < 3) : (idx += 1) {
        const s = raw_logs.get(idx).copy(&logs_buf[idx]);
        logs_len[idx] = s.len;
    }
    var order: [3]usize = .{ 0, 1, 2 };
    for (0..3) |i| {
        for ((i + 1)..3) |j| {
            const si = logs_buf[order[i]][0..logs_len[order[i]]];
            const sj = logs_buf[order[j]][0..logs_len[order[j]]];
            if (std.mem.order(u8, sj, si) == .lt) {
                const tmp = order[i];
                order[i] = order[j];
                order[j] = tmp;
            }
        }
    }
    const n0 = logs_buf[order[0]][0..logs_len[order[0]]];
    const n1 = logs_buf[order[1]][0..logs_len[order[1]]];
    const n2 = logs_buf[order[2]][0..logs_len[order[2]]];
    check(std.mem.eql(u8, n0, "a.log") and std.mem.eql(u8, n1, "b.log") and std.mem.eql(u8, n2, "c.log"));

    const encoding: b.global_BufferEncoding = .{ .h = str("utf8").h };
    const combined = join2(out_dir, "combined.log");
    for (order) |k| {
        const name = logs_buf[k][0..logs_len[k]];
        check(b.path.extname(name).eql(".log"));
        const chunk = b.fs.readFileSyncOrBufferEncoding(.{ .string = join2(in_dir, name) }, .{ .global_BufferEncoding = encoding });
        var cbuf: [64]u8 = undefined;
        b.fs.appendFileSync(.{ .string = combined }, .{ .string = str(chunk.copy(&cbuf)) }, null);
    }

    const bundle_args = b.JsArray.new();
    bundle_args.push(b.JsValue.string("bundle"));
    _ = bus.emit(ev_stage, bundle_args);
    check(stages == 2);

    // Stage 3: verify
    const comb_text = b.fs.readFileSyncOrBufferEncoding(.{ .string = combined }, .{ .global_BufferEncoding = encoding });
    var comb_buf: [128]u8 = undefined;
    const comb_slice = comb_text.copy(&comb_buf);
    check(b.fs.statSync(at(combined)).getSize() == 21 and std.mem.indexOf(u8, comb_slice, "200 OK") != null);

    _ = b.fs.readFileSyncOrBufferEncoding(.{ .string = join2(out_dir, "missing.log") }, .{ .global_BufferEncoding = encoding });
    const err = b.jsTakeError();
    var enoent_caught = false;
    if (err) |e| {
        defer b.jsRelease(e);
        const msg = b.jsErrorMessage(e);
        defer b.jsRelease(msg);
        var mbuf: [256]u8 = undefined;
        enoent_caught = std.mem.indexOf(u8, msg.copy(&mbuf), "ENOENT") != null;
    }
    check(enoent_caught);

    const verify_args = b.JsArray.new();
    verify_args.push(b.JsValue.string("verify"));
    _ = bus.emit(ev_stage, verify_args);
    const done_args = b.JsArray.new();
    done_args.push(b.JsValue.number(@floatFromInt(comb_slice.len)));
    _ = bus.emit(ev_done, done_args);
    check(stages == 3 and done_count == 1);

    const rm = b.fs.RmOptions.create();
    rm.setRecursive(true);
    rm.setForce(true);
    b.fs.rmSync(at(root), .{ .options = rm });
    check(!b.fs.existsSync(at(root)));

    var rbuf: [128]u8 = undefined;
    var line_buf: [256]u8 = undefined;
    println(std.fmt.bufPrint(&line_buf, "pipeline_root={s} combined_bytes={d}", .{
        root.copy(&rbuf),
        comb_slice.len,
    }) catch "");
    println(std.fmt.bufPrint(&line_buf, "stages={d} done={d} enoent={s}", .{
        stages,
        done_count,
        if (enoent_caught) "true" else "false",
    }) catch "");
    println(std.fmt.bufPrint(&line_buf, "PASS: file_pipeline ({d} checks)", .{passed}) catch "");
}
