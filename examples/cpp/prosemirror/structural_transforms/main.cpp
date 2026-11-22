#include <cstdio>
#include <memory>
#include <string>

#include "dom/dom.hpp"
#include "prosemirror/prosemirror.hpp"

using namespace prosemirror;
using namespace prosemirror::prosemirror_transform;
using Sch = Schema<ts::Value, ts::Value>;

static DOMOutputSpec hole_spec(std::string_view tag) {
    return DOMOutputSpec(ts::Array<>::of(tag, 0));
}

static Sch make_schema() {
    NodeSpec doc = NodeSpec::create().content("block+");
    MarkSpec_attrs hattrs = MarkSpec_attrs::create().set_at("level", AttributeSpec::create().default_(ts::Value::number(2)));

    NodeSpec heading = NodeSpec::create()
        .attrs(hattrs)
        .content("inline*")
        .group("block")
        .defining(true)
        .toDOM([](Node n) {
            int l = static_cast<int>(n.attrs().at("level").to_number());
            return hole_spec(l == 1 ? "h1" : "h2");
        });

    auto tag_node = [](const char* content, const char* group, const char* tag, bool defining = false) {
        NodeSpec s = NodeSpec::create().content(content).toDOM([tag](Node) { return hole_spec(tag); });
        if (group) s.group(group);
        if (defining) s.defining(true);
        return s;
    };

    NodeSpec hr = NodeSpec::create()
        .group("block")
        .atom(true)
        .toDOM([](Node) { return DOMOutputSpec(ts::Array<>::of("hr")); });

    NodeSpec text = NodeSpec::create().group("inline");

    auto nodes = NodeSpec::create()
        .set_at("doc", doc)
        .set_at("heading", heading)
        .set_at("paragraph", tag_node("inline*", "block", "p"))
        .set_at("blockquote", tag_node("block+", "block", "blockquote", true))
        .set_at("bullet_list", tag_node("list_item+", "block", "ul"))
        .set_at("list_item", tag_node("paragraph block*", nullptr, "li", true))
        .set_at("horizontal_rule", hr)
        .set_at("text", text);

    auto make_mark = [](const char* tag) {
        return MarkSpec::create().toDOM([tag](Mark, bool) { return hole_spec(tag); });
    };
    auto marks = MarkSpec::create()
        .set_at("strong", make_mark("strong"))
        .set_at("code", make_mark("code"));

    return Sch::create(SchemaSpec<ts::Value, ts::Value>::create().nodes(nodes).marks(marks));
}

static Node p_node(const Sch& schema, std::string_view t) {
    return schema.node("paragraph", std::nullopt, schema.text(t));
}
static Node li_node(const Sch& schema, std::string_view t) {
    return schema.node("list_item", std::nullopt, p_node(schema, t));
}

static Node make_initial_doc(const Sch& schema) {
    Attrs ha = Attrs::create().set_at("level", ts::Value::number(2));
    Node h = schema.node("heading", ha, schema.text("Structural Transform Pipeline"));
    Node p1 = p_node(schema, "ProseMirror transforms express structural edits as invertible steps.");
    Node p2 = p_node(schema, "Wrap blocks in quotes, lift nested content, or split paragraphs cleanly.");
    Node quote = schema.node("blockquote", std::nullopt, p_node(schema, "Blockquote content can be lifted back to top-level document depth."));
    Node ulist = schema.node("bullet_list", std::nullopt, ts::Array<Node>{
        li_node(schema, "Split at cursor position"),
        li_node(schema, "Wrap range with blockquote"),
        li_node(schema, "Lift nested block to root"),
    });
    return schema.node("doc", std::nullopt, ts::Array<Node>{ h, p1, p2, quote, ulist });
}

struct AppState {
    Sch schema;
    EditorView view;
    dom::Document doc;
    dom::HTMLElement meta;
    int total_steps = 0;
    int note_counter = 0;
    std::string last_op = "Initialized document structure";
};

static EditorState make_state(const Sch& schema) {
    return EditorState::create(EditorStateConfig::create().schema(schema).doc(make_initial_doc(schema)));
}

static void update_meta(const std::shared_ptr<AppState>& app, Node doc) {
    int n = static_cast<int>(doc.childCount());
    std::string outline;
    for (int i = 0; i < n; i++) {
        if (i > 0) outline += " \xC2\xB7 ";
        outline += doc.child(i).type().name();
    }
    char buf[768];
    std::snprintf(
        buf, sizeof buf,
        "%s \xC2\xB7 blocks=%d \xC2\xB7 outline=[%s] \xC2\xB7 totalSteps=%d \xC2\xB7 nodeSize=%d",
        app->last_op.c_str(), n, outline.c_str(), app->total_steps, static_cast<int>(doc.nodeSize())
    );
    app->meta.textContent(buf);
}

TS_EXPORT(start) void start() {
    auto app = std::make_shared<AppState>();
    app->schema = make_schema();
    app->doc = dom::get_document();
    app->meta = *app->doc.getElementById("meta");

    DirectEditorProps props = DirectEditorProps::create()
        .state(make_state(app->schema))
        .dispatchTransaction([app](Transaction tr) {
            if (!app->view) return;
            app->total_steps += static_cast<int>(tr.steps().size());
            EditorState next = app->view.state().apply(tr);
            app->view.updateState(next);
            update_meta(app, next.doc());
        });

    app->view = EditorView::create(app->doc.getElementById("editor")->unsafe_as<Node_2>(), props);
    update_meta(app, app->view.state().doc());

    const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        app->doc.getElementById(ids[i])->addEventListener(dom::HTMLElementEventMapKeys::click, [app, i](dom::PointerEvent) {
            EditorState st = app->view.state();
            Node doc = st.doc();
            if (i == 0) {
                double sel_from = st.selection().from();
                std::optional<double> split_pos;
                if (canSplit(doc, sel_from)) {
                    split_pos = sel_from;
                } else {
                    double pos = 0.0;
                    int n = static_cast<int>(doc.childCount());
                    for (int k = 0; k < n; k++) {
                        Node ch = doc.child(k);
                        if (!split_pos.has_value() && ch.type().name() == "paragraph") {
                            double len = ch.content().size();
                            if (len >= 8.0) {
                                double mid = pos + 1.0 + static_cast<int>(len) / 2;
                                if (canSplit(doc, mid)) split_pos = mid;
                            }
                        }
                        pos += ch.nodeSize();
                    }
                }
                if (split_pos.has_value()) {
                    Transaction tr = st.tr();
                    tr.split(*split_pos);
                    app->last_op = "Split paragraph node";
                    app->view.dispatch(tr);
                } else {
                    app->last_op = "No splittable paragraph found";
                    update_meta(app, doc);
                }
            } else if (i == 1) {
                NodeType qtype = *app->schema.nodes().at("blockquote");
                Selection sel = st.selection();
                std::optional<NodeRange> found;
                if (sel._from().depth() == 1.0 && sel._from().parent().type().name() == "paragraph") {
                    auto r = sel._from().blockRange(sel._to());
                    if (r.has_value() && findWrapping(*r, qtype).has_value()) found = r;
                }
                if (!found.has_value()) {
                    double pos = 0.0;
                    int n = static_cast<int>(doc.childCount());
                    for (int k = 0; k < n; k++) {
                        Node ch = doc.child(k);
                        if (!found.has_value() && ch.type().name() == "paragraph") {
                            ResolvedPos rp = doc.resolve(pos + 1.0);
                            auto r = rp.blockRange(rp);
                            if (r.has_value() && findWrapping(*r, qtype).has_value()) found = r;
                        }
                        pos += ch.nodeSize();
                    }
                }
                if (found.has_value()) {
                    auto wrapped = findWrapping(*found, qtype);
                    if (wrapped.has_value()) {
                        Transaction tr = st.tr();
                        tr.wrap(*found, *wrapped);
                        app->last_op = "Wrapped paragraph block in blockquote";
                        app->view.dispatch(tr);
                        return;
                    }
                }
                app->last_op = "No top-level paragraph left to wrap";
                update_meta(app, doc);
            } else if (i == 2) {
                double pos = 0.0;
                int n = static_cast<int>(doc.childCount());
                for (int k = 0; k < n; k++) {
                    Node ch = doc.child(k);
                    if (ch.type().name() == "blockquote" && ch.childCount() > 0.0) {
                        ResolvedPos rp = doc.resolve(pos + 2.0);
                        auto r = rp.blockRange(rp);
                        if (r.has_value()) {
                            auto target = liftTarget(*r);
                            if (target.has_value()) {
                                Transaction tr = st.tr();
                                tr.lift(*r, *target);
                                app->last_op = "Lifted blockquote child to root depth";
                                app->view.dispatch(tr);
                                return;
                            }
                        }
                    }
                    pos += ch.nodeSize();
                }
                app->last_op = "No blockquote left to lift";
                update_meta(app, doc);
            } else if (i == 3) {
                app->note_counter++;
                Node hr = app->schema.node("horizontal_rule");
                char msg[128];
                std::snprintf(msg, sizeof msg, "Appended structural section #%d at doc offset %d.",
                              app->note_counter, static_cast<int>(doc.content().size()));
                Transaction tr = st.tr();
                tr.insert(doc.content().size(), ts::Array<Node>{ hr, p_node(app->schema, msg) });
                app->last_op = "Inserted horizontal_rule + note";
                app->view.dispatch(tr);
            } else {
                app->total_steps = 0;
                app->note_counter = 0;
                app->last_op = "Reset structural sample";
                EditorState fresh = make_state(app->schema);
                app->view.updateState(fresh);
                update_meta(app, fresh.doc());
            }
        });
    }
}
