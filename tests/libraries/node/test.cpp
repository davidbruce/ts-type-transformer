// Node's fs, path and events from C++: files made, read and removed in a
// scratch directory, paths taken apart, an emitter listened to.
// Every CHECK must hold.
#include <algorithm>
#include <string>

#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool has(const std::string& s, std::string_view part) { return s.find(part) != std::string::npos; }

using Strings = ts::Array<std::string>;

static std::string join(const std::string& a, const std::string& b) { return a + "/" + b; }

extern "C" __attribute__((export_name("run"))) void run() {
    // ---- path ----
    CHECK(path::join({"a", "b", "../c", "d.txt"}) == "a/c/d.txt");
    CHECK(path::normalize("/x//y/./z/../w") == "/x/y/w");
    CHECK(path::basename("/a/b/file.tar.gz") == "file.tar.gz");
    CHECK(path::basename("/a/b/file.tar.gz", ".gz") == "file.tar");
    CHECK(path::dirname("/a/b/file.txt") == "/a/b");
    CHECK(path::extname("/a/b/file.tar.gz") == ".gz");
    CHECK(path::isAbsolute("/a") && !path::isAbsolute("a/b"));
    CHECK(path::relative("/a/b/c", "/a/d") == "../../d");
    CHECK(path::resolve({"/a/b", "../c", "d"}) == "/a/c/d");
    path_ParsedPath pp = path::parse("/home/u/file.txt");
    CHECK(pp.root() == "/" && pp.dir() == "/home/u" && pp.base() == "file.txt" && pp.ext() == ".txt" && pp.name() == "file");
    path_FormatInputPathObject fo = path_FormatInputPathObject::create();
    fo.dir("/q");
    fo.base("z.md");
    CHECK(path::format(fo) == "/q/z.md");
    CHECK(path::matchesGlob("/a/b/c.txt", "/a/**/*.txt"));
    // ---- fs, in a scratch directory made under target/e2e ----
    std::string dir = fs::mkdtempSync("target/e2e/cpp-node-");
    CHECK(has(dir, "target/e2e/cpp-node-"));
    CHECK(fs::existsSync(dir));
    CHECK(fs::statSync(dir).isDirectory());
    CHECK(!fs::statSync(dir).isFile());

    std::string file = join(dir, "hello.txt");
    fs::writeFileSync(file, std::string("héllo\nworld\n"));
    CHECK(fs::existsSync(file));
    CHECK(!fs::existsSync(join(dir, "missing.txt")));

    Stats st = fs::statSync(file);
    CHECK(st.isFile());
    CHECK(st.size() == 13);  // é is two bytes
    CHECK(st.mtimeMs() > 0);

    // Read back: a Buffer of bytes, which prints as utf8 text.
    Buffer<ArrayBuffer> buf = fs::readFileSync(file);
    CHECK(buf.byte_length() == 13);
    CHECK(buf.toString() == "héllo\nworld\n");
    std::vector<uint8_t> bytes = buf.to_vector();
    CHECK(bytes.size() == 13 && bytes[0] == 'h' && bytes[1] == 0xC3 && bytes[2] == 0xA9);

    // FINDING 7: the encoding `"utf8"` of fs::readFileSync(path, "utf8") is typed
    // global_BufferEncoding, an opaque handle struct with no way to make one from
    // the string or from the BufferEncoding enum (also in Buffer::toString(enc),
    // writeFileSync options). A raw JavaScript string handle is made by hand.
    global_BufferEncoding utf8(ts::handle, ts_raw_js_from_string("utf8", 4));
    std::string text = fs::readFileSync(file, ReadFileSyncOptionsWithStringEncoding_or_BufferEncoding(utf8));
    CHECK(text == "héllo\nworld\n");

    fs::appendFileSync(file, std::string("more\n"));
    CHECK(fs::readFileSync(file).toString() == "héllo\nworld\nmore\n");

    // Bytes in, from a Uint8Array.
    std::string bin = join(dir, "data.bin");
    std::vector<uint8_t> raw = {0, 1, 2, 254, 255};
    fs::writeFileSync(bin, Uint8Array<ArrayBufferLike>::from(raw));
    CHECK(fs::statSync(bin).size() == 5);
    CHECK(fs::readFileSync(bin).to_vector() == raw);

    // A directory with a few files; readdirSync lists them.
    std::string sub = join(join(dir, "sub"), "deeper");
    mkdir_options mk = mkdir_options::create();
    CHECK(mkdir_options::recursive == true);
    CHECK(fs::mkdirSync(sub, mk).has_value());
    CHECK(fs::statSync(sub).isDirectory());
    fs::writeFileSync(join(sub, "x.txt"), std::string("x"));
    fs::copyFileSync(file, join(dir, "copy.txt"));
    fs::renameSync(join(dir, "copy.txt"), join(dir, "moved.txt"));
    CHECK(!fs::existsSync(join(dir, "copy.txt")));
    CHECK(fs::readFileSync(join(dir, "moved.txt")).toString() == fs::readFileSync(file).toString());

    Strings names = fs::readdirSync(dir);
    std::vector<std::string> listed;
    for (std::string n : names) listed.push_back(n);
    std::sort(listed.begin(), listed.end());
    CHECK(listed.size() == 4);
    CHECK(listed[0] == "data.bin" && listed[1] == "hello.txt" && listed[2] == "moved.txt" && listed[3] == "sub");

    // A missing file throws: an error to take, with Node's code and message.
    CHECK(!ts::take_error());
    ts::Result<Buffer<ArrayBuffer>> missing = ts::attempt([&] { return fs::readFileSync(join(dir, "nope.txt")); });
    CHECK(!missing);
    CHECK(has(missing.error().message(), "ENOENT"));
    CHECK(has(missing.error().message(), "nope.txt"));
    CHECK(!ts::take_error());

    fs::unlinkSync(file);
    CHECK(!fs::existsSync(file));

    // Removed with everything under it.
    RmOptions rm = RmOptions::create();
    rm.recursive(true);
    rm.force(true);
    fs::rmSync(dir, rm);
    CHECK(!fs::existsSync(dir));
    CHECK(!ts::take_error());

    // ---- events ----
    EventEmitter<ts::Value> emitter = EventEmitter<ts::Value>::create();
    ts::Value ping = ts::Value::string("ping");
    CHECK(emitter.listenerCount<void>(ping) == 0);

    int pings = 0;
    // A rest-parameter listener is given all the arguments, gathered.
    std::string heard;
    any_array_fn onPing = [&](ts::Array<> args) {
        pings++;
        heard += std::to_string(args.size()) + ":";
        if (args.size() == 2) CHECK(ts::Value(args[0]).to_number() == 5 && ts::Value(args[1]).to_string() == "x");
    };
    emitter.on<void>(ping, onPing);
    CHECK(emitter.listenerCount<void>(ping) == 1);
    CHECK(emitter.emit<void>(ping, ts::Array<>{ts::Value::number(5), ts::Value::string("x")}));
    CHECK(pings == 1);
    emitter.emit<void>(ping, ts::Array<>{ts::Value::number(7)});
    CHECK(pings == 2);
    CHECK(heard == "2:1:");

    // once: heard one time only.
    int onces = 0;
    any_array_fn onOnce = [&](ts::Array<>) { onces++; };
    ts::Value shot = ts::Value::string("shot");
    emitter.once<void>(shot, onOnce);
    CHECK(emitter.listenerCount<void>(shot) == 1);
    CHECK(emitter.emit<void>(shot, ts::Array<>{}));
    CHECK(!emitter.emit<void>(shot, ts::Array<>{}));
    CHECK(onces == 1);
    CHECK(emitter.listenerCount<void>(shot) == 0);

    // Removal; an event with no listener emits false.
    emitter.removeListener<void>(ping, onPing);
    CHECK(emitter.listenerCount<void>(ping) == 0);
    CHECK(!emitter.emit<void>(ping, ts::Array<>{ts::Value::number(1)}));
    CHECK(pings == 2);

    // Several listeners, in the order added; prependListener goes first.
    std::string order;
    any_array_fn a = [&](ts::Array<>) { order += "a"; };
    any_array_fn b = [&](ts::Array<>) { order += "b"; };
    any_array_fn c = [&](ts::Array<>) { order += "c"; };
    ts::Value ev = ts::Value::string("ev");
    emitter.on<void>(ev, a);
    emitter.on<void>(ev, b);
    emitter.prependListener<void>(ev, c);
    emitter.emit<void>(ev, ts::Array<>{});
    CHECK(order == "cab");
    CHECK(emitter.listenerCount<void>(ev) == 3);
    CHECK(emitter.eventNames().size() == 1);
    emitter.removeAllListeners<void>(ev);
    CHECK(emitter.listenerCount<void>(ev) == 0);
    emitter.setMaxListeners(20);
    CHECK(emitter.getMaxListeners() == 20);

    onPing.release();
    onOnce.release();
    a.release();
    b.release();
    c.release();
}
