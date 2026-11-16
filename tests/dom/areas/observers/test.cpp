// Observers from C++: a MutationObserver's records (taken and delivered), a
// ResizeObserver and an IntersectionObserver firing, and a timeout signal.
// Delivery is asynchronous: `busy` says so and `finish` checks the tally.
// Every CHECK must hold.
#include <string>
#include <utility>
#include <vector>

#include "areas/core/AbortController.hpp"
#include "areas/core/AbortSignal.hpp"
#include "areas/core/Document.hpp"
#include "areas/core/DOMRectReadOnly.hpp"
#include "areas/core/IntersectionObserver.hpp"
#include "areas/core/IntersectionObserverEntry.hpp"
#include "areas/core/MutationObserver.hpp"
#include "areas/core/MutationObserverInit.hpp"
#include "areas/core/MutationRecord.hpp"
#include "areas/core/ResizeObserver.hpp"
#include "areas/core/ResizeObserverEntry.hpp"
#include "areas/core/ResizeObserverSize.hpp"
#include "areas/core/functions.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

// FINDING 9 (runtime bug): assigning a temporary to a handle type with several
// virtual-base paths (Document, Element, HTMLDivElement, ...) leaves it null: the
// implicit move assignment runs Handle::operator=(Handle) (ts_runtime.hpp:81)
// once per base path, and every run after the first takes from a source already
// emptied. `doc = get_document();` made `doc` null. A copy assignment from an
// lvalue is fine, so values go through a named reference.
template <class T>
static void assign(T& to, const T& from) {
    to = from;
}

static int pending = 0;
static int completed = 0;

static Document doc;
static HTMLElement box;
static MutationObserver delivered;
static ResizeObserver resizer;
static IntersectionObserver watcher;
static AbortSignal timeout_signal;
static std::string mutation_log;
static std::vector<double> widths;
static bool intersected = false;
static double ratio = 0;
static bool timeout_event = false;
static std::string timeout_reason;
static ts::Listener<Event> on_abort;
static ts::Listener<Event> on_never;
static bool aborted_ran = false;

extern "C" __attribute__((export_name("busy"))) int busy() { return pending; }

extern "C" __attribute__((export_name("finish"))) void finish() {
    // The mutation callback got both batches' records, in order.
    CHECK(mutation_log == "childList:1 attributes:data-x ");
    // The ResizeObserver reported the starting size, then the new one.
    CHECK(widths.size() >= 2);
    CHECK(widths.size() >= 2 && widths[0] == 100 && widths[1] == 160);
    // The IntersectionObserver saw the visible element.
    CHECK(intersected && ratio > 0 && ratio <= 1);
    // The timeout signal aborted by itself, with a TimeoutError.
    CHECK(timeout_signal.aborted() && timeout_event);
    CHECK(timeout_reason == "TimeoutError");
    CHECK(!aborted_ran);
    CHECK(completed == 4 && pending == 0);
    delivered.disconnect();
    resizer.disconnect();
    watcher.disconnect();
}

extern "C" __attribute__((export_name("run"))) void run() {
    assign(doc, get_document());
    HTMLElement body = doc.body();

    // takeRecords: the pending records, at once; the callback never sees them.
    HTMLDivElement list = doc.createElement(HTMLElementTagNameMapKeys::div);
    body.appendChild(list);
    bool called = false;
    MutationObserver taker = MutationObserver::create([&](ts::Array<MutationRecord>, MutationObserver) { called = true; });
    MutationObserverInit kids = MutationObserverInit::create();
    kids.childList(true);
    taker.observe(list, kids);
    HTMLSpanElement s1 = doc.createElement(HTMLElementTagNameMapKeys::span);
    HTMLSpanElement s2 = doc.createElement(HTMLElementTagNameMapKeys::span);
    list.appendChild(s1);
    list.appendChild(s2);
    ts::Array<MutationRecord> records = taker.takeRecords();
    CHECK(records.size() == 2);
    MutationRecord first = records[0];
    CHECK(first.type() == MutationRecordType::childList);
    CHECK(first.target() == list);
    CHECK(first.addedNodes().length() == 1 && first.addedNodes().item(0) == s1);
    CHECK(first.removedNodes().length() == 0);
    CHECK(first.previousSibling() == std::nullopt);
    CHECK(records[1].previousSibling() == s1);
    CHECK(taker.takeRecords().size() == 0);

    // Removals are recorded with what was removed.
    list.removeChild(s1);
    ts::Array<MutationRecord> removal = taker.takeRecords();
    CHECK(removal.size() == 1 && removal[0].removedNodes().length() == 1 && removal[0].nextSibling() == s2);

    // Not observed: attribute changes with only childList asked for.
    list.setAttribute("data-x", "1");
    CHECK(taker.takeRecords().size() == 0);

    // disconnect stops it; takeRecords after is empty.
    taker.disconnect();
    list.appendChild(s1);
    CHECK(taker.takeRecords().size() == 0);
    CHECK(!called);

    // Attributes, with the old value and a filter.
    HTMLDivElement target = doc.createElement(HTMLElementTagNameMapKeys::div);
    body.appendChild(target);
    MutationObserver attrs = MutationObserver::create([](ts::Array<MutationRecord>, MutationObserver) {});
    MutationObserverInit ao = MutationObserverInit::create();
    ao.attributes(true);
    ao.attributeOldValue(true);
    ao.attributeFilter(ts::Array<std::string>{"title", "lang"});
    attrs.observe(target, ao);
    target.setAttribute("title", "one");
    target.setAttribute("title", "two");
    target.setAttribute("id", "ignored");
    target.setAttribute("lang", "en");
    ts::Array<MutationRecord> changes = attrs.takeRecords();
    CHECK(changes.size() == 3);
    CHECK(changes[0].type() == MutationRecordType::attributes && changes[0].attributeName() == "title");
    CHECK(!changes[0].oldValue());
    CHECK(changes[1].oldValue() == "one");
    CHECK(changes[2].attributeName() == "lang");
    attrs.disconnect();

    // Character data, on a text node.
    Text text = doc.createTextNode("before");
    target.appendChild(text);
    MutationObserver chars = MutationObserver::create([](ts::Array<MutationRecord>, MutationObserver) {});
    MutationObserverInit co = MutationObserverInit::create();
    co.characterData(true);
    co.characterDataOldValue(true);
    chars.observe(text, co);
    text.data("after");
    ts::Array<MutationRecord> edits = chars.takeRecords();
    CHECK(edits.size() == 1 && edits[0].type() == MutationRecordType::characterData);
    CHECK(edits[0].oldValue() == "before" && edits[0].target() == text);
    chars.disconnect();

    // subtree reaches the descendants.
    MutationObserver deep = MutationObserver::create([](ts::Array<MutationRecord>, MutationObserver) {});
    MutationObserverInit so = MutationObserverInit::create();
    so.childList(true);
    so.subtree(true);
    deep.observe(list, so);
    Element inner = list.firstElementChild().value();
    inner.appendChild(doc.createTextNode("nested"));
    CHECK(deep.takeRecords().size() == 1);
    deep.disconnect();

    // A delivered batch: both changes arrive in one callback after run.
    assign<HTMLElement>(box, doc.createElement(HTMLElementTagNameMapKeys::div));
    box.id("watched");
    box.style().setProperty("width", "100px");
    box.style().setProperty("height", "50px");
    box.style().setProperty("background", "orange");
    body.appendChild(box);
    pending++;
    assign(delivered, MutationObserver::create([](ts::Array<MutationRecord> rs, MutationObserver) {
        for (MutationRecord r : rs) {
            if (r.type() == MutationRecordType::childList) mutation_log += "childList:" + std::to_string(int(r.addedNodes().length())) + " ";
            else mutation_log += "attributes:" + *r.attributeName() + " ";
        }
        completed++;
        pending--;
    }));
    MutationObserverInit both = MutationObserverInit::create();
    both.childList(true);
    both.attributes(true);
    both.attributeFilter(ts::Array<std::string>{"data-x"});
    delivered.observe(box, both);
    box.appendChild(doc.createElement(HTMLElementTagNameMapKeys::i));
    box.setAttribute("data-x", "1");
    box.setAttribute("data-y", "ignored");
    CHECK(mutation_log == "");

    // ResizeObserver: the first report, then a report after a resize.
    pending++;
    assign(resizer, ResizeObserver::create([](ts::Array<ResizeObserverEntry> entries, ResizeObserver) {
        for (ResizeObserverEntry e : entries) {
            CHECK(e.target() == box);
            double w = e.contentRect().width();
            widths.push_back(w);
            CHECK(e.contentBoxSize().size() == 1 && e.contentBoxSize()[0].inlineSize() == w);
            if (widths.size() == 1) {
                box.style().setProperty("width", "160px");
            } else if (widths.size() == 2) {
                completed++;
                pending--;
            }
        }
    }));
    resizer.observe(box);

    // IntersectionObserver: the box is on screen.
    pending++;
    assign(watcher, IntersectionObserver::create([](ts::Array<IntersectionObserverEntry> entries, IntersectionObserver) {
        for (IntersectionObserverEntry e : entries) {
            if (e.isIntersecting() && !intersected) {
                intersected = true;
                ratio = e.intersectionRatio();
                CHECK(e.target() == box);
                CHECK(e.boundingClientRect().width() == 100 || e.boundingClientRect().width() == 160);
                CHECK(e.intersectionRect().height() > 0);
                completed++;
                pending--;
            }
        }
    }));
    watcher.observe(box);
    CHECK(watcher.thresholds().size() == 1 && watcher.rootMargin() == "0px 0px 0px 0px");

    // AbortSignal.timeout aborts by itself; one already aborted says so.
    AbortSignal fresh = AbortSignal::abort();
    CHECK(fresh.aborted());
    assign(timeout_signal, AbortSignal::timeout(30));
    CHECK(!timeout_signal.aborted());
    pending++;
    on_abort = [](Event e) {
        timeout_event = true;
        CHECK(e.type() == "abort");
        // The reason is a DOMException; read its name through an Error handle.
        ts::Value reason = timeout_signal.reason();
        ts::Error err(ts::handle, std::exchange(reason.h, 0));
        timeout_reason = err.name();
        reason = ts::Value();
        completed++;
        pending--;
    };
    timeout_signal.addEventListener(AbortSignalEventMapKeys::abort, on_abort);
    // A signal never aborted never calls.
    AbortController never = AbortController::create();
    on_never = [](Event) { aborted_ran = true; };
    never.signal().addEventListener(AbortSignalEventMapKeys::abort, on_never);
    CHECK(!never.signal().aborted());
}
