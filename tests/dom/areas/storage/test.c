// localStorage and sessionStorage from C: items set, read, replaced, removed and
// cleared, and a JSON text stored, read back and parsed. Every CHECK must hold.
// The parse is asynchronous (Response.json()): busy() waits and finish() checks it.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)
#define STR(s, text) JsString_equals((s), (text))

static int pending;
static int parsed;
__attribute__((export_name("busy"))) int busy(void) { return pending; }

static char big[6000000];

// The stored text, parsed as JSON by a Response: the object has the fields
// of a ResponseInit, so that dictionary's getters read it.
static void on_parsed(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    CHECK(ok);
    ResponseInit back = { value.h };
    CHECK(ResponseInit_get_status(back).has && ResponseInit_get_status(back).value == 202);
    CHECK(STR(ResponseInit_get_statusText(back), "stored \"quoted\" \xe2\x82\xac"));
    parsed++;
    pending--;
}
static void on_text(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    CHECK(ok);
    // The JSON text goes into storage, comes out the same, and is parsed from there.
    Storage local = get_localStorage();
    JsString text = js_to_string(value);
    uint32_t n = JsString_len(text);
    CHECK(n > 10);
    static char buf[256];
    JsString_copy(text, buf, sizeof buf);
    Storage_setItem(local, "doc", buf);
    JsString back = Storage_getItem(local, "doc");
    CHECK(JsString_len(back) == n && STR(back, buf));
    Response r = Response_new(BodyInit_of(back));
    Promise_then(Body_json(r), on_parsed, 0);
}

__attribute__((export_name("run"))) void run(void) {
    Storage local = get_localStorage();
    Storage session = get_sessionStorage();
    Storage_clear(local);
    Storage_clear(session);
    CHECK(Storage_get_length(local) == 0 && Storage_get_length(session) == 0);

    // Set and get.
    Storage_setItem(local, "name", "ann");
    Storage_setItem(local, "colour", "green");
    CHECK(Storage_get_length(local) == 2);
    CHECK(STR(Storage_getItem(local, "name"), "ann") && STR(Storage_getItem(local, "colour"), "green"));
    CHECK(Storage_getItem(local, "missing").h == 0);

    // Replaced, not duplicated.
    Storage_setItem(local, "name", "bob");
    CHECK(Storage_get_length(local) == 2 && STR(Storage_getItem(local, "name"), "bob"));

    // key(i) names each item once, in no promised order; out of range is null.
    JsString k0 = Storage_key(local, 0), k1 = Storage_key(local, 1);
    CHECK((JsString_equals(k0, "name") && JsString_equals(k1, "colour")) || (JsString_equals(k0, "colour") && JsString_equals(k1, "name")));
    CHECK(Storage_key(local, 2).h == 0);

    // The two areas are separate.
    CHECK(Storage_getItem(session, "name").h == 0);
    Storage_setItem(session, "name", "session ann");
    CHECK(STR(Storage_getItem(session, "name"), "session ann") && STR(Storage_getItem(local, "name"), "bob"));
    CHECK(Storage_get_length(session) == 1);

    // Empty strings, odd keys and non-ASCII text survive.
    Storage_setItem(local, "empty", "");
    CHECK(Storage_getItem(local, "empty").h != 0 && JsString_len(Storage_getItem(local, "empty")) == 0);
    Storage_setItem(local, "", "empty key");
    CHECK(STR(Storage_getItem(local, ""), "empty key"));
    Storage_setItem(local, "sp ace/\"q\"", "caf\xc3\xa9 \xe2\x82\xac \xf0\x9f\x98\x80");
    CHECK(STR(Storage_getItem(local, "sp ace/\"q\""), "caf\xc3\xa9 \xe2\x82\xac \xf0\x9f\x98\x80"));
    CHECK(JsString_len(Storage_getItem(local, "sp ace/\"q\"")) == 5 + 1 + 3 + 1 + 4);  // UTF-8 bytes
    Storage_setItem(local, "line", "a\nb\tc\\d");
    CHECK(STR(Storage_getItem(local, "line"), "a\nb\tc\\d"));
    CHECK(Storage_get_length(local) == 6);

    // Items are also properties of the storage object.
    Storage_set_at(local, "prop", js_string("via property"));
    CHECK(STR(Storage_getItem(local, "prop"), "via property"));
    CHECK(STR(js_to_string(Storage_get_at(local, "name")), "bob"));
    // A value that is not a string is stored as its string form.
    Storage_set_at(local, "num", js_number(12.5));
    CHECK(STR(Storage_getItem(local, "num"), "12.5"));

    // Removed.
    Storage_removeItem(local, "colour");
    CHECK(Storage_getItem(local, "colour").h == 0 && Storage_get_length(local) == 8 - 1);
    Storage_removeItem(local, "never-set");
    CHECK(Storage_get_length(local) == 7);

    // Too much for the quota throws a QuotaExceededError and stores nothing.
    for (unsigned i = 0; i < sizeof big - 1; i++) big[i] = 'x';
    Storage_setItem(local, "huge", big);
    CHECK(js_failed());
    JsValue quota = js_take_error();
    CHECK(quota.h != 0 && STR(js_error_name(quota), "QuotaExceededError"));
    CHECK(Storage_getItem(local, "huge").h == 0);

    // Cleared.
    Storage_clear(local);
    CHECK(Storage_get_length(local) == 0 && Storage_getItem(local, "name").h == 0 && Storage_key(local, 0).h == 0);
    CHECK(Storage_get_length(session) == 1);
    Storage_clear(session);
    CHECK(Storage_get_length(session) == 0);

    // JSON: a Response makes the text of an object, storage keeps it, a Response parses it.
    ResponseInit shape = ResponseInit_create();
    ResponseInit_set_status(shape, js_some_double(202));
    ResponseInit_set_statusText(shape, "stored \"quoted\" \xe2\x82\xac");
    pending++;
    Promise_then(Body_text(Response_json((JsValue){ shape.h })), on_text, 0);
}

__attribute__((export_name("finish"))) void finish(void) {
    CHECK(pending == 0 && parsed == 1);
    Storage local = get_localStorage();
    CHECK(Storage_get_length(local) == 1 && Storage_getItem(local, "doc").h != 0);
    Storage_clear(local);
    CHECK(Storage_get_length(local) == 0);
}
