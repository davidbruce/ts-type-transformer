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

export fn start() void {
    const joined = b.path.join(b.JsArrayOf(b.JsString).of(&.{
        str("/workspace"),
        str("packages"),
        str("core"),
        str(".."),
        str("cli"),
        str("index.ts"),
    }));
    check(joined.eql("/workspace/packages/cli/index.ts"));

    const norm = b.path.normalize("/var//log/./app/../http/access.log");
    check(norm.eql("/var/log/http/access.log"));

    const resolved = b.path.resolve(b.JsArrayOf(b.JsString).of(&.{
        str("/srv/app"),
        str("../shared"),
        str("config/prod.json"),
    }));
    check(resolved.eql("/srv/shared/config/prod.json"));

    const rel = b.path.relative("/srv/app/src/ui", "/srv/app/dist/assets");
    check(rel.eql("../../dist/assets"));

    check(b.path.dirname("/srv/app/archive.tar.gz").eql("/srv/app"));
    check(b.path.basename("/srv/app/archive.tar.gz", .{}).eql("archive.tar.gz"));
    check(b.path.basename("/srv/app/archive.tar.gz", .{ .suffix = ".gz" }).eql("archive.tar"));
    check(b.path.extname("/srv/app/archive.tar.gz").eql(".gz"));
    check(b.path.isAbsolute("/srv/app") and !b.path.isAbsolute("src/app"));

    const parsed = b.path.parse("/srv/app/archive.tar.gz");
    check(parsed.getRoot().eql("/") and
        parsed.getDir().eql("/srv/app") and
        parsed.getBase().eql("archive.tar.gz") and
        parsed.getName().eql("archive.tar") and
        parsed.getExt().eql(".gz"));

    const fmt = b.path.FormatInputPathObject.create();
    fmt.setDir("/etc/service");
    fmt.setName("daemon");
    fmt.setExt(".conf");
    const formatted = b.path.format(fmt);
    check(formatted.eql("/etc/service/daemon.conf"));

    check(b.path.matchesGlob("/src/components/Button.tsx", "/src/**/*.tsx") and
        !b.path.matchesGlob("/src/components/Button.css", "/src/**/*.tsx"));

    var b1: [128]u8 = undefined;
    var b2: [128]u8 = undefined;
    var b3: [128]u8 = undefined;
    var b4: [128]u8 = undefined;
    var b5: [128]u8 = undefined;
    var r_root: [16]u8 = undefined;
    var r_dir: [64]u8 = undefined;
    var r_base: [64]u8 = undefined;
    var r_ext: [16]u8 = undefined;
    var line_buf: [512]u8 = undefined;

    println(std.fmt.bufPrint(&line_buf, "join={s} norm={s}", .{
        joined.copy(&b1),
        norm.copy(&b2),
    }) catch "");
    println(std.fmt.bufPrint(&line_buf, "resolve={s} relative={s} format={s}", .{
        resolved.copy(&b3),
        rel.copy(&b4),
        formatted.copy(&b5),
    }) catch "");
    println(std.fmt.bufPrint(&line_buf, "parsed=(root={s} dir={s} base={s} ext={s})", .{
        parsed.getRoot().copy(&r_root),
        parsed.getDir().copy(&r_dir),
        parsed.getBase().copy(&r_base),
        parsed.getExt().copy(&r_ext),
    }) catch "");
    println(std.fmt.bufPrint(&line_buf, "PASS: path_inspector ({d} checks)", .{passed}) catch "");
}
