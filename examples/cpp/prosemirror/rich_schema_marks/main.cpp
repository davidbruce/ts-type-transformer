#include <cstdio>
#include <memory>
#include <string>

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
    heading.defining(true);
    heading.toDOM(Node_to_DOMOutputSpec_fn([](Node n) {
        int l = static_cast<int>(n.attrs().at("level").to_number());
        return hole_spec(l == 1 ? "h1" : "h2");
    }));

    NodeSpec para = NodeSpec::create();
    para.content("inline*");
    para.group("block");
    para.toDOM(Node_to_DOMOutputSpec_fn([](Node) { return hole_spec("p"); }));

    NodeSpec quote = NodeSpec::create();
    quote.content("block+");
    quote.group("block");
    quote.defining(true);
    quote.toDOM(Node_to_DOMOutputSpec_fn([](Node) { return hole_spec("blockquote"); }));

    NodeSpec cblock = NodeSpec::create();
    cblock.content("text*");
    cblock.marks("");
    cblock.group("block");
    cblock.code(true);
    cblock.defining(true);
    cblock.toDOM(Node_to_DOMOutputSpec_fn([](Node) {
        ts::Array<> inner = ts::Array<>::create();
        inner.push(ts::Value::string("code"));
        inner.push(ts::Value::number(0));
        ts::Array<> outer = ts::Array<>::create();
        outer.push(ts::Value::string("pre"));
        outer.push(val_of(inner));
        return DOMOutputSpec(outer);
    }));

    NodeSpec hr = NodeSpec::create();
    hr.group("block");
    hr.atom(true);
    hr.toDOM(Node_to_DOMOutputSpec_fn([](Node) {
        ts::Array<> a = ts::Array<>::create();
        a.push(ts::Value::string("hr"));
        return DOMOutputSpec(a);
    }));

    NodeSpec text = NodeSpec::create();
    text.group("inline");

    NodeSpec nodes = NodeSpec::create();
    nodes.set_at("doc", val_of(doc));
    nodes.set_at("heading", val_of(heading));
    nodes.set_at("paragraph", val_of(para));
    nodes.set_at("blockquote", val_of(quote));
    nodes.set_at("code_block", val_of(cblock));
    nodes.set_at("horizontal_rule", val_of(hr));
    nodes.set_at("text", val_of(text));

    auto make_mark = [](const char* tag) {
        MarkSpec m = MarkSpec::create();
        m.toDOM(Mark_boolean_to_DOMOutputSpec_fn([tag](Mark, bool) { return hole_spec(tag); }));
        return m;
    };
    MarkSpec marks = MarkSpec::create();
    marks.set_at("strong", val_of(make_mark("strong")));
    marks.set_at("em", val_of(make_mark("em")));
    marks.set_at("code", val_of(make_mark("code")));

    SchemaSpec<ts::Value, ts::Value> spec = SchemaSpec<ts::Value, ts::Value>::create();
    spec.nodes(val_of(nodes));
    spec.marks(val_of(marks));
    return Sch::create(spec);
}

static Node make_initial_doc(const Sch& schema) {
    Attrs ha = Attrs::create();
    ha.set_at("level", ts::Value::number(2));
    Node h = schema.node(
        string_or_NodeType(std::string("heading")), ha,
        Fragment_or_Node_or_Node_array(schema.text("ProseMirror Document Schema"))
    );

    Mark m_strong = schema.mark(string_or_MarkType(std::string("strong")));
    Mark m_em = schema.mark(string_or_MarkType(std::string("em")));
    Mark m_code = schema.mark(string_or_MarkType(std::string("code")));

    ts::Array<Node> p1_runs{
        schema.text("Inline marks compose cleanly: "),
        schema.text("bold", ts::Array<Mark>{ m_strong }),
        schema.text(", "),
        schema.text("italic", ts::Array<Mark>{ m_em }),
        schema.text(", and "),
        schema.text("inline_code", ts::Array<Mark>{ m_code }),
        schema.text(" spans."),
    };
    Node p1 = schema.node(string_or_NodeType(std::string("paragraph")), std::nullopt, Fragment_or_Node_or_Node_array(p1_runs));

    Node q_inner = schema.node(
        string_or_NodeType(std::string("paragraph")), std::nullopt,
        Fragment_or_Node_or_Node_array(schema.text("Every document conforms to a strict schema enforced on every transaction."))
    );
    Node quote = schema.node(string_or_NodeType(std::string("blockquote")), std::nullopt, Fragment_or_Node_or_Node_array(q_inner));

    Node cblock = schema.node(
        string_or_NodeType(std::string("code_block")), std::nullopt,
        Fragment_or_Node_or_Node_array(schema.text("schema.node(\"paragraph\", null, [schema.text(\"typed\")])"))
    );

    Node outro = schema.node(
        string_or_NodeType(std::string("paragraph")), std::nullopt,
        Fragment_or_Node_or_Node_array(schema.text("Select any text in this editor and click a toolbar action to toggle marks or switch block types."))
    );

    ts::Array<Node> blocks{ h, p1, quote, cblock, outro };
    return schema.node(string_or_NodeType(std::string("doc")), std::nullopt, Fragment_or_Node_or_Node_array(blocks));
}

struct AppState {
    Sch schema;
    EditorView view;
    dom::Document doc;
    dom::HTMLElement meta;
    int dispatches = 0;
    std::string last_action = "Initialized schema with 7 node types & 3 marks";
};

static std::pair<double, double> target_range(EditorState st) {
    Selection sel = st.selection();
    if (!sel.empty()) return { sel.from(), sel.to() };
    Node doc = st.doc();
    if (doc.childCount() >= 2.0) {
        Node c0 = doc.child(0);
        Node c1 = doc.child(1);
        double start_pos = c0.nodeSize() + 1.0;
        double len = c1.content().size();
        if (len > 0.0) return { start_pos, start_pos + (len > 12.0 ? 12.0 : len) };
    }
    double sz = doc.content().size();
    return { 1.0, sz > 2.0 ? 2.0 : sz };
}

static void update_meta(const std::shared_ptr<AppState>& app, EditorState st) {
    auto [f, t] = target_range(st);
    Node doc = st.doc();
    auto marks = app->schema.marks();
    bool hb = doc.rangeHasMark(f, t, Mark_or_MarkType(*marks.at("strong")));
    bool he = doc.rangeHasMark(f, t, Mark_or_MarkType(*marks.at("em")));
    bool hc = doc.rangeHasMark(f, t, Mark_or_MarkType(*marks.at("code")));

    app->doc.getElementById("btn-0")->className(hb ? "active" : "");
    app->doc.getElementById("btn-1")->className(he ? "active" : "");
    app->doc.getElementById("btn-2")->className(hc ? "active" : "");

    std::string bname = st.selection()._from().parent().type().name();
    char buf[512];
    std::snprintf(
        buf, sizeof buf,
        "%s \xC2\xB7 blocks=%d \xC2\xB7 nodeSize=%d \xC2\xB7 range=[%d..%d] \xC2\xB7 block=%s \xC2\xB7 dispatches=%d",
        app->last_action.c_str(),
        static_cast<int>(doc.childCount()),
        static_cast<int>(doc.nodeSize()),
        static_cast<int>(f),
        static_cast<int>(t),
        bname.c_str(),
        app->dispatches
    );
    app->meta.textContent(buf);
}

static EditorState make_state(const Sch& schema) {
    EditorStateConfig cfg = EditorStateConfig::create();
    cfg.schema(schema);
    cfg.doc(make_initial_doc(schema));
    return EditorState::create(cfg);
}

extern "C" __attribute__((export_name("start"))) void start() {
    auto app = std::make_shared<AppState>();
    app->schema = make_schema();
    app->doc = dom::get_document();
    app->meta = *app->doc.getElementById("meta");

    DirectEditorProps props = DirectEditorProps::create();
    props.state(make_state(app->schema));
    props.dispatchTransaction(Transaction_fn([app](Transaction tr) {
        if (app->view.h == 0) return;
        app->dispatches++;
        EditorState next = app->view.state().apply(tr);
        app->view.updateState(next);
        update_meta(app, next);
    }));

    dom::HTMLElement mount = *app->doc.getElementById("editor");
    dist_DOMNode place;
    static_cast<ts::Handle&>(place) = mount;
    app->view = EditorView::create(EditorView_new_place_or_dist_DOMNode(place), props);
    update_meta(app, app->view.state());

    auto toggle_mark = [app](const char* name, const char* label) {
        EditorState st = app->view.state();
        auto [f, t] = target_range(st);
        if (t <= f) return;
        MarkType mt = *app->schema.marks().at(name);
        Transaction tr = st.tr();
        if (st.doc().rangeHasMark(f, t, Mark_or_MarkType(mt))) {
            tr.removeMark(f, t, Mark_or_MarkType(mt));
            app->last_action = std::string("Removed ") + label;
        } else {
            tr.addMark(f, t, app->schema.mark(string_or_MarkType(mt)));
            app->last_action = std::string("Added ") + label;
        }
        call_Transaction_fn(app->view.dispatch(), tr);
    };

    const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        dom::HTMLElement btn = *app->doc.getElementById(ids[i]);
        ts::Listener<dom::PointerEvent> cb = [app, toggle_mark, i](dom::PointerEvent) {
            if (i == 0) toggle_mark("strong", "bold");
            else if (i == 1) toggle_mark("em", "italic");
            else if (i == 2) toggle_mark("code", "code");
            else if (i == 3) {
                EditorState st = app->view.state();
                Selection sel = st.selection();
                std::string pname = sel._from().parent().type().name();
                Transaction tr = st.tr();
                if (pname == "heading") {
                    tr.setBlockType(sel.from(), sel.to(), *app->schema.nodes().at("paragraph"));
                    app->last_action = "Converted heading -> paragraph";
                } else {
                    Attrs ha = Attrs::create();
                    ha.set_at("level", ts::Value::number(2));
                    tr.setBlockType(sel.from(), sel.to(), *app->schema.nodes().at("heading"), Attrs_or_Transform_setBlockType_attrs(ha));
                    app->last_action = "Converted block -> heading[level=2]";
                }
                call_Transaction_fn(app->view.dispatch(), tr);
            } else {
                app->dispatches = 0;
                app->last_action = "Reset initial schema document";
                EditorState fresh = make_state(app->schema);
                app->view.updateState(fresh);
                update_meta(app, fresh);
            }
        };
        btn.addEventListener(dom::HTMLElementEventMapKeys::click, cb);
    }
}
