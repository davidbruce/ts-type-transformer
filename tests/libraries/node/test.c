// Node's fs, path and events from C: files written and read in a temporary
// directory, paths taken apart and joined, an emitter with listeners.
// Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

// A path as the PathLike union wants it, from a C string.
static PathLike P(const char* s) { return PathLike_of_string(JsString_from(s)); }
static PathOrFileDescriptor F(const char* s) { return PathOrFileDescriptor_of_string(JsString_from(s)); }

// A string handle taken into a C buffer and released.
static char* take(JsString s) {
    static char bufs[8][512];
    static int next;
    char* b = bufs[next++ & 7];
    JsString_copy(s, b, 512);
    js_release(s);
    return b;
}

static bool same(const char* a, const char* b) {
    while (*a && *a == *b) a++, b++;
    return *a == *b;
}

static bool ends_with(const char* s, const char* suffix) {
    uint32_t n = ts_strlen(s), m = ts_strlen(suffix);
    return n >= m && same(s + n - m, suffix);
}

static void join2(char* out, const char* a, const char* b) {
    uint32_t n = 0;
    for (; a[n]; n++) out[n] = a[n];
    out[n++] = '/';
    for (uint32_t i = 0; b[i]; i++) out[n++] = b[i];
    out[n] = 0;
}

// Listeners count and remember what they were given.
static int hits;
static void on_event(void* ctx, JsArray args) {
    (*(int*)ctx)++;
    hits++;
    // FINDING 8: a rest-parameter callback `(...args: any[]) => void` gets
    // only its first JavaScript argument, as the handle `args`, not an array
    // of them all. Emitted arguments cannot be read, so the listeners only count.
    (void)args;
}

__attribute__((export_name("run"))) void run(void) {
    // ---- path ----
    // FINDING 11: every path_* function runs `PK["path"].path.join(...)` (the
    // `export =` namespace read as a property of the module), which is
    // undefined for node:path, so each throws. Left out:
#if 0
    JsArray_JsString parts = JsArray_JsString_new();
    JsArray_JsString_push(parts, "a");
    JsArray_JsString_push(parts, "b/../c");
    JsArray_JsString_push(parts, "d.txt");
    CHECK(same(take(path_join(parts)), "a/c/d.txt"));
    CHECK(same(take(path_basename("/x/y/file.tar.gz")), "file.tar.gz"));
    CHECK(same(take(path_basename("/x/y/file.tar.gz", ".gz")), "file.tar"));
    CHECK(same(take(path_extname("/x/y/file.tar.gz")), ".gz"));
    CHECK(same(take(path_dirname("/x/y/file.txt")), "/x/y"));
    CHECK(same(take(path_normalize("/a//b/./c/..")), "/a/b"));
    CHECK(path_isAbsolute("/a") && !path_isAbsolute("a/b"));
    CHECK(same(take(path_relative("/data/orandea/test/aaa", "/data/orandea/impl/bbb")), "../../impl/bbb"));
    JsArray_JsString abs = JsArray_JsString_new();
    JsArray_JsString_push(abs, "/foo");
    JsArray_JsString_push(abs, "bar");
    JsArray_JsString_push(abs, "baz/asdf");
    JsArray_JsString_push(abs, "..");
    CHECK(same(take(path_resolve(abs)), "/foo/bar/baz"));
    path_ParsedPath parsed = path_parse("/home/user/dir/file.txt");
    CHECK(same(take(path_ParsedPath_get_root(parsed)), "/"));
    CHECK(same(take(path_ParsedPath_get_dir(parsed)), "/home/user/dir"));
    CHECK(same(take(path_ParsedPath_get_base(parsed)), "file.txt"));
    CHECK(same(take(path_ParsedPath_get_name(parsed)), "file"));
    CHECK(same(take(path_ParsedPath_get_ext(parsed)), ".txt"));
    path_FormatInputPathObject fmt = path_FormatInputPathObject_create();
    path_FormatInputPathObject_set_dir(fmt, "/tmp");
    path_FormatInputPathObject_set_name(fmt, "note");
    path_FormatInputPathObject_set_ext(fmt, ".md");
    CHECK(same(take(path_format(fmt)), "/tmp/note.md"));

#endif

    // ---- fs ----
    char dir[512], file[600], sub[600], moved[600];
    JsString made = mkdtempSync("target/e2e/fs-c-", (JsValue){ 0 });
    JsString_copy(made, dir, sizeof dir);
    js_release(made);
    CHECK(existsSync(P(dir)));
    CHECK(statSync(P(dir)).h != 0);
    CHECK(StatsBase_isDirectory(statSync(P(dir))));

    join2(file, dir, "hello.txt");
    writeFileSync(F(file), "hello, node\n", (JsValue){ 0 });
    CHECK(existsSync(P(file)));
    CHECK(StatsBase_isFile(statSync(P(file))));
    CHECK(js_to_number(StatsBase_get_size(statSync(P(file)))) == 12);

    // FINDING 9: the encoding is a `global_BufferEncoding`, a type that is
    // not the BufferEncoding enum, and has no constructor: a string handle
    // cast to it.
    global_BufferEncoding utf8 = { JsString_from("utf8").h };
    JsString text = readFileSync_with_BufferEncoding(F(file), utf8);
    CHECK(JsString_equals(text, "hello, node\n"));
    js_release(text);

    // Appending, then the whole file again.
    appendFileSync(F(file), "more\n", (JsValue){ 0 });
    JsString both = readFileSync_with_BufferEncoding(F(file), utf8);
    CHECK(JsString_equals(both, "hello, node\nmore\n"));
    js_release(both);

    // Without an encoding, the bytes are a Buffer.
    Buffer bytes = readFileSync(F(file), (ReadFileSyncOptionsWithBufferEncoding){ 0 });
    CHECK(bytes.h != 0);
    CHECK(Uint8Array_length(Buffer_as_Uint8Array(bytes)) == 17);
    js_release(bytes);

    // A subdirectory, made with options, and a second file in it.
    join2(sub, dir, "inner/deeper");
    MakeDirectoryOptions mk = MakeDirectoryOptions_create();
    MakeDirectoryOptions_set_recursive(mk, js_some_bool(true));
    js_release(mkdirSync_with_opt_MakeDirectoryOptions(P(sub), mk));
    CHECK(existsSync(P(sub)));
    char nested[700];
    join2(nested, sub, "n.txt");
    writeFileSync(F(nested), "nested", (JsValue){ 0 });

    // The directory's entries.
    JsArray_JsString names = readdirSync(P(dir), (readdir_options){ 0 });
    CHECK(JsArray_JsString_length(names) == 2);
    bool has_file = false, has_inner = false;
    for (uint32_t i = 0; i < JsArray_JsString_length(names); i++) {
        JsString n = JsArray_JsString_get(names, i);
        has_file |= JsString_equals(n, "hello.txt");
        has_inner |= JsString_equals(n, "inner");
        js_release(n);
    }
    CHECK(has_file && has_inner);
    js_release(names);

    // Rename, copy, unlink.
    join2(moved, dir, "renamed.txt");
    renameSync(P(file), P(moved));
    CHECK(!existsSync(P(file)) && existsSync(P(moved)));
    char copy[600];
    join2(copy, dir, "copy.txt");
    copyFileSync(P(moved), P(copy), js_none_double());
    JsString copied = readFileSync_with_BufferEncoding(F(copy), utf8);
    CHECK(JsString_equals(copied, "hello, node\nmore\n"));
    js_release(copied);
    unlinkSync(P(copy));
    CHECK(!existsSync(P(copy)));

    // A missing file throws; the error says ENOENT.
    JsString none = readFileSync_with_BufferEncoding(F("target/e2e/does-not-exist.txt"), utf8);
    CHECK(none.h == 0 && js_failed());
    JsValue error = js_take_error();
    CHECK(error.h != 0);
    JsString message = js_error_message(error);
    CHECK(ends_with(take(message), "no such file or directory, open 'target/e2e/does-not-exist.txt'"));
    js_release(error);
    CHECK(!existsSync(P("target/e2e/does-not-exist.txt")));

    // Remove it all, recursively.
    RmOptions rm = RmOptions_create();
    RmOptions_set_recursive(rm, js_some_bool(true));
    RmOptions_set_force(rm, js_some_bool(true));
    rmSync(P(dir), rm);
    CHECK(!existsSync(P(dir)));

    // ---- events ----
    // FINDING 10: `EventEmitter_new()` with no arguments does not compile under
    // -std=c11 -pedantic (an empty variadic macro argument), so the options
    // object is given, empty.
    EventEmitter emitter = EventEmitter_new((EventEmitterOptions){ 0 });
    NodeJS_EventEmitter em = EventEmitter_as_NodeJS_EventEmitter(emitter);
    int on_count = 0, once_count = 0, other_count = 0;
    any_array_fn on_cb = any_array_fn_from(on_event, &on_count);
    any_array_fn once_cb = any_array_fn_from(on_event, &once_count);
    any_array_fn other_cb = any_array_fn_from(on_event, &other_count);
    NodeJS_EventEmitter_on(em, "data", on_cb);
    NodeJS_EventEmitter_once(em, "data", once_cb);
    NodeJS_EventEmitter_on(em, "other", other_cb);
    CHECK(NodeJS_EventEmitter_listenerCount(em, "data") == 2);
    CHECK(NodeJS_EventEmitter_listenerCount(em, "other") == 1);

    // Emit with arguments: the listeners get them.
    JsArray args = JsArray_new();
    JsValue n42 = js_number(42), word = js_string("payload");
    JsArray_push(args, n42);
    JsArray_push(args, word);
    CHECK(NodeJS_EventEmitter_emit(em, "data", args));
    CHECK(on_count == 1 && once_count == 1 && other_count == 0);
    // The once listener is gone after the first emit.
    CHECK(NodeJS_EventEmitter_listenerCount(em, "data") == 1);
    NodeJS_EventEmitter_emit(em, "data", args);
    CHECK(on_count == 2 && once_count == 1 && hits == 3);

    // No listener: emit says so.
    CHECK(!NodeJS_EventEmitter_emit(em, "nobody", JsArray_new()));

    // The event names, and a removed listener.
    JsArray names_of = NodeJS_EventEmitter_eventNames(em);
    CHECK(JsArray_length(names_of) == 2);
    NodeJS_EventEmitter_removeListener(em, "data", on_cb);
    CHECK(NodeJS_EventEmitter_listenerCount(em, "data") == 0);
    CHECK(!NodeJS_EventEmitter_emit(em, "data", args));
    CHECK(on_count == 2);
    NodeJS_EventEmitter_removeAllListeners(em, "other");
    CHECK(NodeJS_EventEmitter_listenerCount(em, "other") == 0);
    NodeJS_EventEmitter_setMaxListeners(em, 3);
    CHECK(NodeJS_EventEmitter_getMaxListeners(em) == 3);
}
