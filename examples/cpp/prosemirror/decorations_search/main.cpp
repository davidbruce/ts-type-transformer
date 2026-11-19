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

static ts::Value val_of(const ts::Handle& h) {
    ts::Value v;
    static_cast<ts::Handle&>(v) = h;
    return v;
}

static DOMOutputSpec hole_spec(std::string_view tag) {
    ts::Array<> a = ts::Array<>::create();
    a.push(ts::Value::string(tag));
    a.push(ts::Value::number(0));
    return DOMOutputSpec(a);
}

static std::string to_lower_ascii(std::string s) {
    for (char& c : s) c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
    return s;
}

static Sch make_schema() {
    NodeSpec doc = NodeSpec::create();
    doc.content("block+");

    AttributeSpec lvl = AttributeSpec::create();
    lvl.default_(ts::Value::number(2));
    MarkSpec_attrs hattrs = MarkSpec_attrs::create();
    hattrs.set_at("level", lvl);

    NodeSpec heading = NodeSpec::create();
    heading.attrs(hattrs);
    heading.content("inline*");
    heading.group("block");
    heading.toDOM(Node_to_DOMOutputSpec_fn([](Node n) {
        int l = static_cast<int>(n.attrs().at("level").to_number());
        return hole_spec(l == 1 ? "h1" : "h2");
    }));

    NodeSpec para = NodeSpec::create();
    para.content("inline*");
    para.group("block");
    para.toDOM(Node_to_DOMOutputSpec_fn([](Node) { return hole_spec("p"); }));

    NodeSpec cblock = NodeSpec::create();
    cblock.content("text*");
    cblock.marks("");
    cblock.group("block");
    cblock.code(true);
    cblock.toDOM(Node_to_DOMOutputSpec_fn([](Node) {
        ts::Array<> inner = ts::Array<>::create();
        inner.push(ts::Value::string("code"));
        inner.push(ts::Value::number(0));
        ts::Array<> outer = ts::Array<>::create();
        outer.push(ts::Value::string("pre"));
        outer.push(val_of(inner));
        return DOMOutputSpec(outer);
    }));

    NodeSpec text = NodeSpec::create();
    text.group("inline");

    NodeSpec nodes = NodeSpec::create();
    nodes.set_at("doc", val_of(doc));
    nodes.set_at("heading", val_of(heading));
    nodes.set_at("paragraph", val_of(para));
    nodes.set_at("code_block", val_of(cblock));
    nodes.set_at("text", val_of(text));

    auto make_mark = [](const char* tag) {
        MarkSpec m = MarkSpec::create();
        m.toDOM(Mark_boolean_to_DOMOutputSpec_fn([tag](Mark, bool) { return hole_spec(tag); }));
        return m;
    };
    MarkSpec marks = MarkSpec::create();
    marks.set_at("strong", val_of(make_mark("strong")));
    marks.set_at("code", val_of(make_mark("code")));

    SchemaSpec<ts::Value, ts::Value> spec = SchemaSpec<ts::Value, ts::Value>::create();
    spec.nodes(val_of(nodes));
    spec.marks(val_of(marks));
    return Sch::create(spec);
}

static Node p_node(const Sch& schema, std::string_view t) {
    return schema.node(string_or_NodeType(std::string("paragraph")), std::nullopt, Fragment_or_Node_or_Node_array(schema.text(t)));
}

static Node make_initial_doc(const Sch& schema) {
    Attrs ha = Attrs::create();
    ha.set_at("level", ts::Value::number(2));
    Node h = schema.node(
        string_or_NodeType(std::string("heading")), ha,
        Fragment_or_Node_or_Node_array(schema.text("Immutable State & View Decorations"))
    );
    Node p1 = p_node(schema, "A ProseMirror schema validates every document tree before state updates commit to the view.");
    Node p2 = p_node(schema, "Each transaction maps positions forward so state plugins can recompute decorations without mutating the schema.");
    Node cb = schema.node(
        string_or_NodeType(std::string("code_block")), std::nullopt,
        Fragment_or_Node_or_Node_array(schema.text("const nextState = state.apply(transaction);\nview.updateState(nextState); // sync view"))
    );
    Node p3 = p_node(schema, "Inline decorations highlight matching tokens while node decorations outline matching blocks in the view.");
    ts::Array<Node> blocks{ h, p1, p2, cb, p3 };
    return schema.node(string_or_NodeType(std::string("doc")), std::nullopt, Fragment_or_Node_or_Node_array(blocks));
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
    DecorationAttrs inl_attr = DecorationAttrs::create();
    inl_attr.class_("pm-match");
    DecorationAttrs blk_attr = DecorationAttrs::create();
    blk_attr.class_("pm-hit-block");

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

extern "C" __attribute__((export_name("start"))) void start() {
    auto app = std::make_shared<AppState>();
    app->schema = make_schema();
    app->search_key = PluginKey<ts::Value>::create("search");
    app->doc = dom::get_document();
    app->meta = *app->doc.getElementById("meta");

    StateField<ts::Value> field = StateField<ts::Value>::create();
    field.init(StateField_init<ts::Value>([](EditorStateConfig, EditorState) { return ts::Value::string("state"); }));
    field.apply(StateField_apply<ts::Value>([](Transaction tr, ts::Value prev, EditorState, EditorState) {
        ts::Value mv = tr.getMeta(string_or_Plugin_or_PluginKey(std::string("search")));
        return mv.is_null() ? prev : mv;
    }));

    EditorProps<Plugin<ts::Value>> eprops = EditorProps<Plugin<ts::Value>>::create();
    eprops.decorations(EditorProps_decorations<Plugin<ts::Value>>([app](EditorState st) -> std::optional<DecorationSource> {
        auto qv = app->search_key.getState(st);
        std::string q = (qv.has_value() && !qv->is_null()) ? qv->to_string() : "";
        return DecorationSource(compute_decorations(st.doc(), q).set);
    }));

    PluginSpec<ts::Value> pspec = PluginSpec<ts::Value>::create();
    pspec.key(app->search_key);
    pspec.state(field);
    pspec.props(eprops);

    EditorStateConfig scfg = EditorStateConfig::create();
    scfg.schema(app->schema);
    scfg.doc(make_initial_doc(app->schema));
    scfg.plugins(ts::Array<Plugin<ts::Value>>{ Plugin<ts::Value>::create(pspec) });

    DirectEditorProps props = DirectEditorProps::create();
    props.state(EditorState::create(scfg));
    props.dispatchTransaction(Transaction_fn([app](Transaction tr) {
        if (app->view.h == 0) return;
        EditorState next = app->view.state().apply(tr);
        app->view.updateState(next);
        sync_meta(app, next);
    }));

    dom::HTMLElement mount = *app->doc.getElementById("editor");
    dist_DOMNode place;
    static_cast<ts::Handle&>(place) = mount;
    app->view = EditorView::create(EditorView_new_place_or_dist_DOMNode(place), props);
    sync_meta(app, app->view.state());

    const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    const char* queries[5] = { "state", "schema", "transaction", "view", "" };
    for (int i = 0; i < 5; i++) {
        dom::HTMLElement btn = *app->doc.getElementById(ids[i]);
        std::string q = queries[i];
        ts::Listener<dom::PointerEvent> cb = [app, i, q](dom::PointerEvent) {
            app->active_btn = i;
            Transaction tr = app->view.state().tr();
            tr.setMeta(string_or_Plugin_or_PluginKey(std::string("search")), ts::Value::string(q));
            call_Transaction_fn(app->view.dispatch(), tr);
        };
        btn.addEventListener(dom::HTMLElementEventMapKeys::click, cb);
    }
}
