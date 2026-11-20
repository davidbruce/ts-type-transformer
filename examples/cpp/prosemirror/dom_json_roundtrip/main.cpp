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

static TagParseRule tag_rule(std::string_view tag, int level = 0) {
    TagParseRule r = TagParseRule::create();
    r.tag(tag);
    if (level > 0) {
        Attrs a = Attrs::create();
        a.set_at("level", ts::Value::number(level));
        r.attrs(a);
    }
    return r;
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
    heading.parseDOM(ts::Array<TagParseRule>{ tag_rule("h1", 1), tag_rule("h2", 2) });
    heading.toDOM(Node_to_DOMOutputSpec_fn([](Node n) {
        int l = static_cast<int>(n.attrs().at("level").to_number());
        return hole_spec(l == 1 ? "h1" : "h2");
    }));

    NodeSpec para = NodeSpec::create();
    para.content("inline*");
    para.group("block");
    para.parseDOM(ts::Array<TagParseRule>{ tag_rule("p") });
    para.toDOM(Node_to_DOMOutputSpec_fn([](Node) { return hole_spec("p"); }));

    NodeSpec quote = NodeSpec::create();
    quote.content("block+");
    quote.group("block");
    quote.defining(true);
    quote.parseDOM(ts::Array<TagParseRule>{ tag_rule("blockquote") });
    quote.toDOM(Node_to_DOMOutputSpec_fn([](Node) { return hole_spec("blockquote"); }));

    NodeSpec text = NodeSpec::create();
    text.group("inline");

    NodeSpec nodes = NodeSpec::create();
    nodes.set_at("doc", val_of(doc));
    nodes.set_at("heading", val_of(heading));
    nodes.set_at("paragraph", val_of(para));
    nodes.set_at("blockquote", val_of(quote));
    nodes.set_at("text", val_of(text));

    auto mark_spec = [](const char* tag) {
        MarkSpec m = MarkSpec::create();
        m.parseDOM(ts::Array<ParseRule>{ ParseRule(tag_rule(tag)) });
        m.toDOM(Mark_boolean_to_DOMOutputSpec_fn([tag](Mark, bool) { return hole_spec(tag); }));
        return m;
    };
    MarkSpec marks = MarkSpec::create();
    marks.set_at("strong", val_of(mark_spec("strong")));
    marks.set_at("em", val_of(mark_spec("em")));
    marks.set_at("code", val_of(mark_spec("code")));

    SchemaSpec<ts::Value, ts::Value> spec = SchemaSpec<ts::Value, ts::Value>::create();
    spec.nodes(val_of(nodes));
    spec.marks(val_of(marks));
    return Sch::create(spec);
}

static constexpr const char* SPEC_HTML =
    "<h2>DOMParser &amp; JSON Serialization</h2>"
    "<p>ProseMirror converts <strong>HTML</strong> via <code>DOMParser</code> and renders back through <em>DOMSerializer</em>.</p>"
    "<blockquote><p>Schema parseDOM rules keep DOM and JSON representations aligned.</p></blockquote>"
    "<p>Trigger any action above to inspect live DOMSerializer HTML and Node.eq() round-trips.</p>";

static constexpr const char* CHANGELOG_HTML =
    "<h2>Release 1.4 Notes</h2>"
    "<p>Added <strong>typed bindings</strong> for <code>DOMParser</code> and <em>DOMSerializer</em>.</p>"
    "<blockquote><p>All round-trips verified with Node.eq().</p></blockquote>"
    "<p>Edit directly above to watch HTML output update live.</p>";

struct AppState {
    Sch schema;
    DOMParser parser;
    DOMSerializer serializer;
    EditorView view;
    dom::Document doc;
    dom::HTMLElement output;
    dom::HTMLElement meta;
    std::string status = "JSON Node.eq() pending";
};

static Node parse_html(const std::shared_ptr<AppState>& app, const char* html) {
    dom::HTMLElement div = app->doc.createElement("div");
    div.innerHTML(html);
    Node_2 dn;
    static_cast<ts::Handle&>(dn) = div;
    return app->parser.parse(dn);
}

static void refresh_output(const std::shared_ptr<AppState>& app, Node doc) {
    dom::HTMLElement div = app->doc.createElement("div");
    app->serializer.serializeFragment(
        doc.content(),
        DOMSerializer_serializeFragment_options::create(),
        DocumentFragment_or_HTMLElement(std::in_place_index<1>, div)
    );
    std::string html_out = div.innerHTML();

    int nb = static_cast<int>(doc.childCount());
    char header[256];
    std::snprintf(header, sizeof header, "doc(blocks=%d, nodeSize=%d)\n", nb, static_cast<int>(doc.nodeSize()));
    std::string ast = header;
    for (int i = 0; i < nb; i++) {
        Node b = doc.child(i);
        std::string tname = b.type().name();
        std::string kind = (tname == "heading")
            ? ("heading[level=" + std::to_string(static_cast<int>(b.attrs().at("level").to_number())) + "]")
            : tname;
        ast += "  [" + std::to_string(i) + "] " + kind + " (" + std::to_string(static_cast<int>(b.nodeSize())) + " bytes): " + b.textContent() + "\n";
    }

    app->output.textContent(
        "SERIALIZED HTML (DOMSerializer):\n" + html_out +
        "\n\nDOCUMENT AST & JSON ROUND-TRIP:\n" + ast +
        "\nROUND-TRIP STATUS: " + app->status
    );

    char mbuf[256];
    std::snprintf(
        mbuf, sizeof mbuf,
        "%s \xC2\xB7 blocks=%d \xC2\xB7 nodeSize=%d \xC2\xB7 htmlBytes=%zu",
        app->status.c_str(), nb, static_cast<int>(doc.nodeSize()), html_out.size()
    );
    app->meta.textContent(mbuf);
}

extern "C" __attribute__((export_name("start"))) void start() {
    auto app = std::make_shared<AppState>();
    app->schema = make_schema();
    app->parser = DOMParser::fromSchema(app->schema);
    app->serializer = DOMSerializer::fromSchema(app->schema);
    app->doc = dom::get_document();
    app->output = *app->doc.getElementById("output");
    app->meta = *app->doc.getElementById("meta");

    EditorStateConfig scfg = EditorStateConfig::create();
    scfg.schema(app->schema);
    scfg.doc(parse_html(app, SPEC_HTML));

    DirectEditorProps props = DirectEditorProps::create();
    props.state(EditorState::create(scfg));
    props.dispatchTransaction(Transaction_fn([app](Transaction tr) {
        if (app->view.h == 0) return;
        EditorState next = app->view.state().apply(tr);
        app->view.updateState(next);
        refresh_output(app, next.doc());
    }));

    dom::HTMLElement mount = *app->doc.getElementById("editor");
    app->view = EditorView::create(EditorView_new_place_or_dist_DOMNode(Node_2(mount)), props);
    app->status = "Parsed Spec HTML via DOMParser";
    refresh_output(app, app->view.state().doc());

    const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        dom::HTMLElement btn = *app->doc.getElementById(ids[i]);
        ts::Listener<dom::PointerEvent> cb = [app, i](dom::PointerEvent) {
            EditorState st = app->view.state();
            Node doc = st.doc();
            if (i == 0 || i == 1) {
                app->status = (i == 0) ? "Parsed Spec HTML via DOMParser" : "Parsed Changelog HTML via DOMParser";
                EditorStateConfig cfg = EditorStateConfig::create();
                cfg.schema(app->schema);
                cfg.doc(parse_html(app, i == 0 ? SPEC_HTML : CHANGELOG_HTML));
                EditorState next = EditorState::create(cfg);
                app->view.updateState(next);
                refresh_output(app, next.doc());
            } else if (i == 2) {
                Node rebuilt = Node::fromJSON(app->schema, doc.toJSON());
                bool ok = doc.eq(rebuilt);
                app->status = ok ? "JSON toJSON->fromJSON eq() = true" : "JSON toJSON->fromJSON eq() = false";
                refresh_output(app, doc);
            } else if (i == 3) {
                if (doc.childCount() >= 2.0) {
                    double insert_pos = doc.child(0).nodeSize() + doc.child(1).nodeSize() - 1.0;
                    Node span = app->schema.text(" roundtrip()", ts::Array<Mark>{ app->schema.mark(string_or_MarkType(std::string("code"))) });
                    Transaction tr = st.tr();
                    tr.insert(insert_pos, Fragment_or_Node_or_Node_array(span));
                    app->status = "Inserted code span into paragraph";
                    call_Transaction_fn(app->view.dispatch(), tr);
                }
            } else {
                dom::HTMLElement div = app->doc.createElement("div");
                app->serializer.serializeFragment(
                    doc.content(),
                    DOMSerializer_serializeFragment_options::create(),
                    DocumentFragment_or_HTMLElement(std::in_place_index<1>, div)
                );
                Node_2 dn;
                static_cast<ts::Handle&>(dn) = div;
                Node reparsed = app->parser.parse(dn);
                bool ok = doc.eq(reparsed);
                app->status = ok ? "DOM serializeFragment->parse eq() = true" : "DOM serializeFragment->parse eq() = false";
                EditorStateConfig cfg = EditorStateConfig::create();
                cfg.schema(app->schema);
                cfg.doc(reparsed);
                EditorState next = EditorState::create(cfg);
                app->view.updateState(next);
                refresh_output(app, next.doc());
            }
        };
        btn.addEventListener(dom::HTMLElementEventMapKeys::click, cb);
    }
}
