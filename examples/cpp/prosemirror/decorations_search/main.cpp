#include <algorithm>
#include <cctype>
#include <cstdio>
#include <memory>
#include <string>
#include <utility>

#include "dom/dom.hpp"
#include "prosemirror/prosemirror.hpp"

using namespace prosemirror;
using Sch = Schema<ts::Value, ts::Value>;

static DOMOutputSpec hole_spec(std::string_view tag) {
    return DOMOutputSpec(ts::Array<>::of(tag, 0));
}

static std::string to_lower_ascii(std::string s) {
    for (char& c : s) c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
    return s;
}

static Sch make_schema() {
    NodeSpec doc({.content = "block+"});
    MarkSpec_attrs hattrs = MarkSpec_attrs::create().set_at("level", AttributeSpec::create().default_(ts::Value::number(2)));

    NodeSpec heading({
        .content = "inline*",
        .group = "block",
        .attrs = hattrs,
        .toDOM = [](Node n) {
            int l = static_cast<int>(n.attrs().at("level").to_number());
            return hole_spec(l == 1 ? "h1" : "h2");
        },
    });

    NodeSpec para({.content = "inline*", .group = "block", .toDOM = [](Node) { return hole_spec("p"); }});

    NodeSpec cblock({
        .content = "text*",
        .marks = "",
        .group = "block",
        .code = true,
        .toDOM = [](Node) { return DOMOutputSpec(ts::Array<>::of("pre", ts::Array<>::of("code", 0))); },
    });

    NodeSpec text({.group = "inline"});

    auto nodes = NodeSpec::create()
        .set_at("doc", doc)
        .set_at("heading", heading)
        .set_at("paragraph", para)
        .set_at("code_block", cblock)
        .set_at("text", text);

    auto make_mark = [](const char* tag) {
        return MarkSpec({.toDOM = [tag](Mark, bool) { return hole_spec(tag); }});
    };
    auto marks = MarkSpec::create()
        .set_at("strong", make_mark("strong"))
        .set_at("code", make_mark("code"));

    return Sch::create({{.nodes = nodes, .marks = marks}});
}

static Node p_node(const Sch& schema, std::string_view t) {
    return schema.node("paragraph", std::nullopt, schema.text(t));
}

static Node make_initial_doc(const Sch& schema) {
    Attrs ha = Attrs::create().set_at("level", ts::Value::number(2));
    Node h = schema.node("heading", ha, schema.text("Immutable State & View Decorations"));
    Node p1 = p_node(schema, "A ProseMirror schema validates every document tree before state updates commit to the view.");
    Node p2 = p_node(schema, "Each transaction maps positions forward so state plugins can recompute decorations without mutating the schema.");
    Node cb = schema.node("code_block", std::nullopt, schema.text("const nextState = state.apply(transaction);\nview.updateState(nextState); // sync view"));
    Node p3 = p_node(schema, "Inline decorations highlight matching tokens while node decorations outline matching blocks in the view.");
    return schema.node("doc", std::nullopt, ts::Array<Node>{ h, p1, p2, cb, p3 });
}

struct DecoStats {
    DecorationSet set;
    int inline_hits;
    int block_hits;
};

static DecoStats compute_decorations(Node doc, const std::string& raw_query) {
    std::string q = to_lower_ascii(raw_query);
    if (q.empty()) return { DecorationSet::empty(), 0, 0 };

    ts::Array<Decoration> decos = ts::Array<Decoration>::create();
    DecorationAttrs inl_attr({.class_ = "pm-match"});
    DecorationAttrs blk_attr({.class_ = "pm-hit-block"});

    int inl_hits = 0;
    int blk_hits = 0;
    double block_pos = 0.0;
    int nb = static_cast<int>(doc.childCount());
    for (int i = 0; i < nb; i++) {
        Node blk = doc.child(i);
        double inline_pos = block_pos + 1.0;
        bool matched = false;
        int nc = static_cast<int>(blk.childCount());
        for (int j = 0; j < nc; j++) {
            Node ch = blk.child(j);
            if (ch.isText()) {
                std::string text = to_lower_ascii(ch.textContent());
                size_t at = 0;
                while ((at = text.find(q, at)) != std::string::npos) {
                    double fp = inline_pos + static_cast<double>(at);
                    double tp = fp + static_cast<double>(q.size());
                    decos.push(Decoration::inline_(fp, tp, inl_attr));
                    inl_hits++;
                    matched = true;
                    at += q.size();
                }
            }
            inline_pos += ch.nodeSize();
        }
        if (matched) {
            decos.push(Decoration::node(block_pos, block_pos + blk.nodeSize(), blk_attr));
            blk_hits++;
        }
        block_pos += blk.nodeSize();
    }
    return { DecorationSet::create(doc, decos), inl_hits, blk_hits };
}

struct AppState {
    Sch schema;
    PluginKey<ts::Value> search_key;
    EditorView view;
    dom::Document doc;
    dom::HTMLElement meta;
    int active_btn = 0;
};

static void sync_meta(const std::shared_ptr<AppState>& app, EditorState st) {
    auto qv = app->search_key.getState(st);
    std::string q = (qv.has_value() && !qv->is_null()) ? qv->to_string() : "";
    auto stats = compute_decorations(st.doc(), q);
    int total = static_cast<int>(stats.set.find().size());
    std::string qdisp = q.empty() ? "(none)" : ("\"" + q + "\"");

    char buf[256];
    std::snprintf(
        buf, sizeof buf,
        "Query: %s \xC2\xB7 inline matches=%d \xC2\xB7 highlighted blocks=%d \xC2\xB7 decorations=%d",
        qdisp.c_str(), stats.inline_hits, stats.block_hits, total
    );
    app->meta.textContent(buf);

    const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        app->doc.getElementById(ids[i])->className(i == app->active_btn ? "active" : "");
    }
}

TS_EXPORT(start) void start() {
    auto app = std::make_shared<AppState>();
    app->schema = make_schema();
    app->search_key = PluginKey<ts::Value>::create("search");
    app->doc = dom::get_document();
    app->meta = *app->doc.getElementById("meta");

    StateField<ts::Value> field = StateField<ts::Value>::create()
        .init([](EditorStateConfig, EditorState) { return ts::Value::string("state"); })
        .apply([](Transaction tr, ts::Value prev, EditorState, EditorState) {
            ts::Value mv = tr.getMeta("search");
            return mv.is_null() ? prev : mv;
        });

    EditorProps<Plugin<ts::Value>> eprops({
        .decorations = [app](EditorState st) -> std::optional<DecorationSource> {
            auto qv = app->search_key.getState(st);
            std::string q = (qv.has_value() && !qv->is_null()) ? qv->to_string() : "";
            return DecorationSource(compute_decorations(st.doc(), q).set);
        },
    });

    PluginSpec<ts::Value> pspec({.props = eprops, .state = field, .key = app->search_key});

    EditorStateConfig scfg({
        .schema = app->schema,
        .doc = make_initial_doc(app->schema),
        .plugins = ts::Array<Plugin<ts::Value>>{ Plugin<ts::Value>::create(pspec) },
    });

    DirectEditorProps props({
        .state = EditorState::create(scfg),
        .dispatchTransaction = [app](Transaction tr) {
            if (!app->view) return;
            EditorState next = app->view.state().apply(tr);
            app->view.updateState(next);
            sync_meta(app, next);
        },
    });

    app->view = EditorView::create(app->doc.getElementById("editor")->unsafe_as<Node_2>(), props);
    sync_meta(app, app->view.state());

    const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    const char* queries[5] = { "state", "schema", "transaction", "view", "" };
    for (int i = 0; i < 5; i++) {
        std::string q = queries[i];
        app->doc.getElementById(ids[i])->addEventListener(dom::HTMLElementEventMapKeys::click, [app, i, q](dom::PointerEvent) {
            app->active_btn = i;
            Transaction tr = app->view.state().tr();
            tr.setMeta("search", ts::Value::string(q));
            app->view.dispatch(tr);
        });
    }
}
