const std = @import("std");
const b = @import("node");

extern "env" fn print(ptr: [*]const u8, len: u32) void;

const PipeState = struct {
    passed: u32 = 0,
    stages: u32 = 0,
    done_count: u32 = 0,

    fn check(self: *PipeState, ok: bool) void {
        if (!ok) @trap();
        self.passed += 1;
    }
};

fn println(s: []const u8) void {
    print(s.ptr, @intCast(s.len));
    print("\n", 1);
}

fn onStage(state: *PipeState, _: b.JsArray) void {
    state.stages += 1;
}

fn onDone(state: *PipeState, _: b.JsArray) void {
    state.done_count += 1;
}

export fn start() void {
    const scope = b.JsScope.open();
    defer scope.deinit();

    var state: PipeState = .{};
    const bus = b.events.EventEmitter.new(null);
    _ = bus.on("stage", .{ onStage, &state });
    _ = bus.once("done", .{ onDone, &state });

    const root = b.fs.mkdtempSync("/tmp/tsrs-node-pipe-", null);
    const in_dir = b.path.join(&.{ root, "in" });
    const out_dir = b.path.join(&.{ root, "out" });
    _ = b.fs.mkdirSyncRecursive(in_dir, .{});
    _ = b.fs.mkdirSyncRecursive(out_dir, .{});

    // Stage 1: seed
    b.fs.writeFileSync(b.path.join(&.{ in_dir, "a.log" }), "100 OK\n", null);
    b.fs.writeFileSync(b.path.join(&.{ in_dir, "b.log" }), "200 OK\n", null);
    b.fs.writeFileSync(b.path.join(&.{ in_dir, "c.log" }), "300 OK\n", null);
    _ = bus.emit("stage", .{"seed"});
    state.check(state.stages == 1);

    // Stage 2: bundle
    const raw_logs = b.fs.readdirSync(in_dir, null);
    state.check(raw_logs.len() == 3);
    var logs_buf: [3][32]u8 = undefined;
    var logs_len: [3]usize = undefined;
    for (0..3) |i| {
        const s = raw_logs.get(i).copy(&logs_buf[i]);
        logs_len[i] = s.len;
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
    state.check(std.mem.eql(u8, n0, "a.log") and std.mem.eql(u8, n1, "b.log") and std.mem.eql(u8, n2, "c.log"));

    const utf8 = b.JsString.from("utf8").as(b.global_BufferEncoding);
    const combined = b.path.join(&.{ out_dir, "combined.log" });
    for (order) |k| {
        const name = logs_buf[k][0..logs_len[k]];
        state.check(b.path.extname(name).eql(".log"));
        const chunk = b.fs.readFileSyncOrBufferEncoding(b.path.join(&.{ in_dir, name }), utf8);
        b.fs.appendFileSync(combined, chunk, null);
    }

    _ = bus.emit("stage", .{"bundle"});
    state.check(state.stages == 2);

    // Stage 3: verify
    const comb_text = b.fs.readFileSyncOrBufferEncoding(combined, utf8);
    var comb_buf: [128]u8 = undefined;
    const comb_slice = comb_text.copy(&comb_buf);
    state.check(b.fs.statSync(combined).getSize() == 21 and std.mem.indexOf(u8, comb_slice, "200 OK") != null);

    var enoent_caught = false;
    if (b.jsTry(b.fs.readFileSyncOrBufferEncoding(b.path.join(&.{ out_dir, "missing.log" }), utf8))) |_| {} else |_| {
        if (b.jsTakeError()) |e| {
            var mbuf: [256]u8 = undefined;
            const msg_s = std.fmt.bufPrint(&mbuf, "{f}", .{b.jsErrorMessage(e)}) catch "";
            enoent_caught = std.mem.indexOf(u8, msg_s, "ENOENT") != null;
        }
    }
    state.check(enoent_caught);

    _ = bus.emit("stage", .{"verify"});
    _ = bus.emit("done", .{comb_slice.len});
    state.check(state.stages == 3 and state.done_count == 1);

    b.fs.rmSync(root, .{ .recursive = true, .force = true });
    state.check(!b.fs.existsSync(root));

    var line_buf: [256]u8 = undefined;
    println(std.fmt.bufPrint(&line_buf, "pipeline_root={f} combined_bytes={d}", .{
        root,
        comb_slice.len,
    }) catch "");
    println(std.fmt.bufPrint(&line_buf, "stages={d} done={d} enoent={s}", .{
        state.stages,
        state.done_count,
        if (enoent_caught) "true" else "false",
    }) catch "");
    println(std.fmt.bufPrint(&line_buf, "PASS: file_pipeline ({d} checks)", .{state.passed}) catch "");
}
