#include <cstdio>
#include <memory>
#include <string>

#include "dom/dom.hpp"
#include "prosemirror/prosemirror.hpp"

using namespace prosemirror;
using Sch = Schema<ts::Value, ts::Value>;

static DOMOutputSpec hole_spec(std::string_view tag) {
    return DOMOutputSpec(ts::Array<>::of(tag, 0));
}

static TagParseRule tag_rule(std::string_view tag, int level = 0) {
    TagParseRule r = TagParseRule::create().tag(tag);
    if (level > 0) r.attrs(Attrs::create().set_at("level", ts::Value::number(level)));
    return r;
}

static Sch make_schema() {
    NodeSpec doc = NodeSpec::create().content("block+");
    MarkSpec_attrs hattrs = MarkSpec_attrs::create().set_at("level", AttributeSpec::create().default_(ts::Value::number(2)));

    NodeSpec heading = NodeSpec::create()
        .attrs(hattrs)
        .content("inline*")
        .group("block")
        .defining(true)
        .parseDOM(ts::Array<TagParseRule>{ tag_rule("h1", 1), tag_rule("h2", 2) })
        .toDOM([](Node n) {
            int l = static_cast<int>(n.attrs().at("level").to_number());
            return hole_spec(l == 1 ? "h1" : "h2");
        });

    NodeSpec para = NodeSpec::create()
        .content("inline*")
        .group("block")
        .parseDOM(ts::Array<TagParseRule>{ tag_rule("p") })
        .toDOM([](Node) { return hole_spec("p"); });

    NodeSpec quote = NodeSpec::create()
        .content("block+")
        .group("block")
        .defining(true)
        .parseDOM(ts::Array<TagParseRule>{ tag_rule("blockquote") })
        .toDOM([](Node) { return hole_spec("blockquote"); });

    NodeSpec text = NodeSpec::create().group("inline");

    auto nodes = NodeSpec::create()
        .set_at("doc", doc)
        .set_at("heading", heading)
        .set_at("paragraph", para)
        .set_at("blockquote", quote)
        .set_at("text", text);

    auto mark_spec = [](const char* tag) {
        return MarkSpec::create()
            .parseDOM(ts::Array<ParseRule>::of(tag_rule(tag)))
            .toDOM([tag](Mark, bool) { return hole_spec(tag); });
    };
    auto marks = MarkSpec::create()
        .set_at("strong", mark_spec("strong"))
        .set_at("em", mark_spec("em"))
        .set_at("code", mark_spec("code"));

    return Sch::create(SchemaSpec<ts::Value, ts::Value>::create().nodes(nodes).marks(marks));
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
    return app->parser.parse(div.as<Node_2>());
}

static void refresh_output(const std::shared_ptr<AppState>& app, Node doc) {
    dom::HTMLElement div = app->doc.createElement("div");
    app->serializer.serializeFragment(doc.content(), DOMSerializer_serializeFragment_options::create(), div);
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

TS_EXPORT(start) void start() {
    auto app = std::make_shared<AppState>();
    app->schema = make_schema();
    app->parser = DOMParser::fromSchema(app->schema);
    app->serializer = DOMSerializer::fromSchema(app->schema);
    app->doc = dom::get_document();
    app->output = *app->doc.getElementById("output");
    app->meta = *app->doc.getElementById("meta");

    EditorStateConfig scfg = EditorStateConfig::create()
        .schema(app->schema)
        .doc(parse_html(app, SPEC_HTML));

    DirectEditorProps props = DirectEditorProps::create()
        .state(EditorState::create(scfg))
        .dispatchTransaction([app](Transaction tr) {
            if (!app->view) return;
            EditorState next = app->view.state().apply(tr);
            app->view.updateState(next);
            refresh_output(app, next.doc());
        });

    app->view = EditorView::create(app->doc.getElementById("editor")->as<Node_2>(), props);
    app->status = "Parsed Spec HTML via DOMParser";
    refresh_output(app, app->view.state().doc());

    const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        app->doc.getElementById(ids[i])->addEventListener(dom::HTMLElementEventMapKeys::click, [app, i](dom::PointerEvent) {
            EditorState st = app->view.state();
            Node doc = st.doc();
            if (i == 0 || i == 1) {
                app->status = (i == 0) ? "Parsed Spec HTML via DOMParser" : "Parsed Changelog HTML via DOMParser";
                auto cfg = EditorStateConfig::create()
                    .schema(app->schema)
                    .doc(parse_html(app, i == 0 ? SPEC_HTML : CHANGELOG_HTML));
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
                    Node span = app->schema.text(" roundtrip()", ts::Array<Mark>{ app->schema.mark("code") });
                    Transaction tr = st.tr();
                    tr.insert(insert_pos, span);
                    app->status = "Inserted code span into paragraph";
                    app->view.dispatch()(tr);
                }
            } else {
                dom::HTMLElement div = app->doc.createElement("div");
                app->serializer.serializeFragment(doc.content(), DOMSerializer_serializeFragment_options::create(), div);
                Node reparsed = app->parser.parse(div.as<Node_2>());
                bool ok = doc.eq(reparsed);
                app->status = ok ? "DOM serializeFragment->parse eq() = true" : "DOM serializeFragment->parse eq() = false";
                auto cfg = EditorStateConfig::create().schema(app->schema).doc(reparsed);
                EditorState next = EditorState::create(cfg);
                app->view.updateState(next);
                refresh_output(app, next.doc());
            }
        });
    }
}
