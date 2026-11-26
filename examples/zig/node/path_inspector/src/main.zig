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

    const joined = b.path.join(&.{
        "/workspace",
        "packages",
        "core",
        "..",
        "cli",
        "index.ts",
    });
    check(joined.eql("/workspace/packages/cli/index.ts"));

    const norm = b.path.normalize("/var//log/./app/../http/access.log");
    check(norm.eql("/var/log/http/access.log"));

    const resolved = b.path.resolve(&.{
        "/srv/app",
        "../shared",
        "config/prod.json",
    });
    check(resolved.eql("/srv/shared/config/prod.json"));

    const rel = b.path.relative("/srv/app/src/ui", "/srv/app/dist/assets");
    check(rel.eql("../../dist/assets"));

    check(b.path.dirname("/srv/app/archive.tar.gz").eql("/srv/app"));
    check(b.path.basename("/srv/app/archive.tar.gz").eql("archive.tar.gz"));
    check(b.path.basenameWith("/srv/app/archive.tar.gz", ".gz").eql("archive.tar"));
    check(b.path.extname("/srv/app/archive.tar.gz").eql(".gz"));
    check(b.path.isAbsolute("/srv/app") and !b.path.isAbsolute("src/app"));

    const parsed = b.path.parse("/srv/app/archive.tar.gz");
    check(parsed.getRoot().eql("/") and
        parsed.getDir().eql("/srv/app") and
        parsed.getBase().eql("archive.tar.gz") and
        parsed.getName().eql("archive.tar") and
        parsed.getExt().eql(".gz"));

    const formatted = b.path.format(.{
        .dir = "/etc/service",
        .name = "daemon",
        .ext = ".conf",
    });
    check(formatted.eql("/etc/service/daemon.conf"));

    check(b.path.matchesGlob("/src/components/Button.tsx", "/src/**/*.tsx") and
        !b.path.matchesGlob("/src/components/Button.css", "/src/**/*.tsx"));

    var line_buf: [512]u8 = undefined;
    println(std.fmt.bufPrint(&line_buf, "join={f} norm={f}", .{ joined, norm }) catch "");
    println(std.fmt.bufPrint(&line_buf, "resolve={f} relative={f} format={f}", .{ resolved, rel, formatted }) catch "");
    println(std.fmt.bufPrint(&line_buf, "parsed=(root={f} dir={f} base={f} ext={f})", .{
        parsed.getRoot(),
        parsed.getDir(),
        parsed.getBase(),
        parsed.getExt(),
    }) catch "");
    println(std.fmt.bufPrint(&line_buf, "PASS: path_inspector ({d} checks)", .{passed}) catch "");
}
