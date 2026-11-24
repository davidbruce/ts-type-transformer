#include <cstdio>
#include <memory>
#include <string>
#include <vector>

#include "dom/dom.hpp"
#include "prosemirror/prosemirror.hpp"

using namespace prosemirror;
using Sch = Schema<ts::Value, ts::Value>;

static DOMOutputSpec hole_spec(std::string_view tag) {
    return DOMOutputSpec(ts::Array<>::of(tag, 0));
}

static Sch make_schema() {
    NodeSpec doc({.content = "block+"});
    MarkSpec_attrs hattrs = MarkSpec_attrs::create().set_at("level", AttributeSpec::create().default_(ts::Value::number(2)));

    NodeSpec heading({
        .content = "inline*",
        .group = "block",
        .attrs = hattrs,
        .defining = true,
        .toDOM = [](Node n) {
            int l = static_cast<int>(n.attrs().at("level").to_number());
            return hole_spec(l == 1 ? "h1" : "h2");
        },
    });

    NodeSpec para({.content = "inline*", .group = "block", .toDOM = [](Node) { return hole_spec("p"); }});

    NodeSpec quote({.content = "block+", .group = "block", .defining = true, .toDOM = [](Node) { return hole_spec("blockquote"); }});

    NodeSpec cblock({
        .content = "text*",
        .marks = "",
        .group = "block",
        .code = true,
        .defining = true,
        .toDOM = [](Node) { return DOMOutputSpec(ts::Array<>::of("pre", ts::Array<>::of("code", 0))); },
    });

    NodeSpec hr({.group = "block", .atom = true, .toDOM = [](Node) { return DOMOutputSpec(ts::Array<>::of("hr")); }});

    NodeSpec text({.group = "inline"});

    auto nodes = NodeSpec::create()
        .set_at("doc", doc)
        .set_at("heading", heading)
        .set_at("paragraph", para)
        .set_at("blockquote", quote)
        .set_at("code_block", cblock)
        .set_at("horizontal_rule", hr)
        .set_at("text", text);

    auto make_mark = [](const char* tag) {
        return MarkSpec({.toDOM = [tag](Mark, bool) { return hole_spec(tag); }});
    };
    auto marks = MarkSpec::create()
        .set_at("strong", make_mark("strong"))
        .set_at("em", make_mark("em"))
        .set_at("code", make_mark("code"));

    return Sch::create({{.nodes = nodes, .marks = marks}});
}

static Node make_initial_doc(const Sch& schema) {
    Attrs ha = Attrs::create().set_at("level", ts::Value::number(2));
    Node h = schema.node("heading", ha, schema.text("ProseMirror Document Schema"));

    ts::Array<Node> p1_runs{
        schema.text("Inline marks compose cleanly: "),
        schema.text("bold", ts::Array<Mark>{ schema.mark("strong") }),
        schema.text(", "),
        schema.text("italic", ts::Array<Mark>{ schema.mark("em") }),
        schema.text(", and "),
        schema.text("inline_code", ts::Array<Mark>{ schema.mark("code") }),
        schema.text(" spans."),
    };
    Node p1 = schema.node("paragraph", std::nullopt, p1_runs);

    Node q_inner = schema.node("paragraph", std::nullopt, schema.text("Every document conforms to a strict schema enforced on every transaction."));
    Node quote = schema.node("blockquote", std::nullopt, q_inner);
    Node cblock = schema.node("code_block", std::nullopt, schema.text("schema.node(\"paragraph\", null, [schema.text(\"typed\")])"));
    Node outro = schema.node("paragraph", std::nullopt, schema.text("Select any text in this editor and click a toolbar action to toggle marks or switch block types."));

    return schema.node("doc", std::nullopt, ts::Array<Node>{ h, p1, quote, cblock, outro });
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
    bool hb = doc.rangeHasMark(f, t, *marks.at("strong"));
    bool he = doc.rangeHasMark(f, t, *marks.at("em"));
    bool hc = doc.rangeHasMark(f, t, *marks.at("code"));

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
    return EditorState::create({{.schema = schema, .doc = make_initial_doc(schema)}});
}

TS_EXPORT(start) void start() {
    auto app = std::make_shared<AppState>();
    app->schema = make_schema();
    app->doc = dom::get_document();
    app->meta = *app->doc.getElementById("meta");

    DirectEditorProps props({
        .state = make_state(app->schema),
        .dispatchTransaction = [app](Transaction tr) {
            if (!app->view) return;
            app->dispatches++;
            EditorState next = app->view.state().apply(tr);
            app->view.updateState(next);
            update_meta(app, next);
        },
    });

    app->view = EditorView(app->doc.getElementById("editor")->unsafe_as<Node_2>(), props);
    update_meta(app, app->view.state());

    auto toggle_mark = [app](const char* name, const char* label) {
        EditorState st = app->view.state();
        auto [f, t] = target_range(st);
        if (t <= f) return;
        MarkType mt = *app->schema.marks().at(name);
        Transaction tr = st.tr();
        if (st.doc().rangeHasMark(f, t, mt)) {
            tr.removeMark(f, t, mt);
            app->last_action = std::string("Removed ") + label;
        } else {
            tr.addMark(f, t, app->schema.mark(mt));
            app->last_action = std::string("Added ") + label;
        }
        app->view.dispatch(tr);
    };

    const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        app->doc.getElementById(ids[i])->addEventListener(dom::HTMLElementEventMapKeys::click, [app, toggle_mark, i](dom::PointerEvent) {
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
                    Attrs ha = Attrs::create().set_at("level", ts::Value::number(2));
                    tr.setBlockType(sel.from(), sel.to(), *app->schema.nodes().at("heading"), ha);
                    app->last_action = "Converted block -> heading[level=2]";
                }
                app->view.dispatch(tr);
            } else {
                app->dispatches = 0;
                app->last_action = "Reset initial schema document";
                EditorState fresh = make_state(app->schema);
                app->view.updateState(fresh);
                update_meta(app, fresh);
            }
        });
    }
}
