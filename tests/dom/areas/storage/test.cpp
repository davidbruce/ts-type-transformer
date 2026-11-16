// localStorage and sessionStorage from C++: items set, read, removed and
// counted, and a JSON string stored and parsed back. Every CHECK must hold.
#include <string>
#include <utility>

#include "areas/core/Document.hpp"
#include "areas/core/functions.hpp"
#include "areas/network/Response.hpp"
#include "areas/network/ResponseInit.hpp"
#include "areas/storage/Storage.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static int pending = 0;
static int completed = 0;

// JSON text is parsed by the browser: a Response holds it and json() reads it.
// There is no JSON object in the bindings, and a Value cannot be read into, so
// the parsed object is read as a dictionary type with the fields wanted.
// FINDING 7: the same cast is missing as in the fetch program; the handle is
// copied and given to the dictionary type.
static ts::Task parse_stored(std::string stored) {
    pending++;
    Response holder = Response::create(stored);
    ts::Result<ts::Value> parsed = co_await static_cast<const Body&>(holder).json();
    CHECK(bool(parsed));
    if (parsed) {
        ts::Value copy = *parsed;
        ResponseInit obj(ts::handle, std::exchange(copy.h, 0));
        CHECK(obj.status() == 200 && obj.statusText() == "quote \" and \\ and \n newline");
    }
    completed++;
    pending--;
}

extern "C" __attribute__((export_name("busy"))) int busy() { return pending; }
extern "C" __attribute__((export_name("finish"))) void finish() { CHECK(completed == 1 && pending == 0); }

extern "C" __attribute__((export_name("run"))) void run() {
    Storage local = get_localStorage();
    Storage session = get_sessionStorage();
    local.clear();
    session.clear();
    CHECK(local.length() == 0 && session.length() == 0);

    // Set and get.
    local.setItem("name", "ada");
    local.setItem("lang", "c++");
    CHECK(local.length() == 2);
    CHECK(local.getItem("name") == "ada" && local.getItem("lang") == "c++");
    CHECK(!local.getItem("missing"));

    // Overwriting keeps the count.
    local.setItem("name", "grace");
    CHECK(local.getItem("name") == "grace" && local.length() == 2);

    // Values are strings: numbers go in as text.
    local.setItem("count", std::to_string(42));
    CHECK(local.getItem("count") == "42");

    // key(index) lists the keys, in some order; each is one of ours.
    std::string keys;
    for (double i = 0; i < local.length(); i++) {
        std::optional<std::string> k = local.key(i);
        CHECK(k.has_value() && local.getItem(*k).has_value());
        keys += *k + ",";
    }
    CHECK(keys.find("name,") != std::string::npos && keys.find("lang,") != std::string::npos && keys.find("count,") != std::string::npos);
    CHECK(!local.key(3));
    CHECK(!local.key(100));

    // Remove one.
    local.removeItem("lang");
    CHECK(local.length() == 2 && !local.getItem("lang"));
    local.removeItem("never-there");
    CHECK(local.length() == 2);

    // The two areas are separate.
    session.setItem("name", "temp");
    CHECK(session.getItem("name") == "temp" && local.getItem("name") == "grace");
    CHECK(session.length() == 1 && local.length() == 2);

    // Index access to the same items.
    local.set_at("viaIndex", ts::Value::string("indexed"));
    CHECK(local.getItem("viaIndex") == "indexed");
    CHECK(local.at("viaIndex").to_string() == "indexed");
    CHECK(local.length() == 3);

    // The empty string and unusual text are kept.
    local.setItem("empty", "");
    CHECK(local.getItem("empty") == "");
    local.setItem("caf\xc3\xa9", "na\xc3\xafve \xe2\x9c\x93");
    CHECK(local.getItem("caf\xc3\xa9") == "na\xc3\xafve \xe2\x9c\x93");

    // A JSON string survives storage byte for byte, and parses back.
    std::string json = "{\"status\":200,\"statusText\":\"quote \\\" and \\\\ and \\n newline\"}";
    session.setItem("doc", json);
    std::string stored = *session.getItem("doc");
    CHECK(stored == json);
    parse_stored(stored);

    // A long value.
    std::string big(100000, 'x');
    local.setItem("big", big);
    CHECK(local.getItem("big")->size() == 100000);
    local.removeItem("big");

    // clear empties one area only.
    local.clear();
    CHECK(local.length() == 0 && !local.getItem("name"));
    CHECK(session.length() == 2 && session.getItem("name") == "temp");
    session.clear();
    CHECK(session.length() == 0);
    CHECK(!session.key(0));
    CHECK(pending == 1);
}
