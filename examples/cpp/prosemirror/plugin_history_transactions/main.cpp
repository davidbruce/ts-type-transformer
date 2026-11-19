#include <cstdio>
#include <memory>
#include <string>
#include <vector>

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
    heading.toDOM(Node_to_DOMOutputSpec_fn([](Node n) {
        int l = static_cast<int>(n.attrs().at("level").to_number());
        return hole_spec(l == 1 ? "h1" : "h2");
    }));

    NodeSpec para = NodeSpec::create();
    para.content("inline*");
    para.group("block");
    para.toDOM(Node_to_DOMOutputSpec_fn([](Node) { return hole_spec("p"); }));

    NodeSpec text = NodeSpec::create();
    text.group("inline");

    NodeSpec nodes = NodeSpec::create();
    nodes.set_at("doc", val_of(doc));
    nodes.set_at("heading", val_of(heading));
    nodes.set_at("paragraph", val_of(para));
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
        Fragment_or_Node_or_Node_array(schema.text("Step-Inversion Undo & Transaction Filter"))
    );
    ts::Array<Node> blocks{
        h,
        p_node(schema, "Every document change produces atomic Transform steps that can be inverted against prior snapshots."),
        p_node(schema, "A ProseMirror Plugin inspects transactions in filterTransaction and can enforce a read-only lock."),
        p_node(schema, "Click Append Revision or Bold First Line, then step backward with Undo and forward with Redo."),
    };
    return schema.node(string_or_NodeType(std::string("doc")), std::nullopt, Fragment_or_Node_or_Node_array(blocks));
}

struct AppState {
    Sch schema;
    PluginKey<ts::Value> audit_key;
    EditorView view;
    dom::Document doc;
    dom::HTMLElement meta;
    bool locked = false;
    int blocked = 0;
    int rev = 0;
    std::string last_action = "Ready \xE2\x80\x94 open write access";
    std::vector<std::vector<Step>> undo_stack;
    std::vector<std::vector<Step>> redo_stack;
};

static void update_meta(const std::shared_ptr<AppState>& app, EditorState st) {
    auto ev = app->audit_key.getState(st);
    int edits = (ev.has_value() && !ev->is_null()) ? static_cast<int>(ev->to_number()) : 0;
    char buf[512];
    std::snprintf(
        buf, sizeof buf,
        "%s \xC2\xB7 edits=%d \xC2\xB7 undoDepth=%zu \xC2\xB7 redoDepth=%zu \xC2\xB7 lock=%s \xC2\xB7 blocked=%d \xC2\xB7 blocks=%d",
        app->last_action.c_str(), edits, app->undo_stack.size(), app->redo_stack.size(),
        app->locked ? "LOCKED" : "OPEN", app->blocked, static_cast<int>(st.doc().childCount())
    );
    app->meta.textContent(buf);
    app->doc.getElementById("btn-4")->className(app->locked ? "active" : "");
}

extern "C" __attribute__((export_name("start"))) void start() {
    auto app = std::make_shared<AppState>();
    app->schema = make_schema();
    app->audit_key = PluginKey<ts::Value>::create("audit");
    app->doc = dom::get_document();
    app->meta = *app->doc.getElementById("meta");

    StateField<ts::Value> field = StateField<ts::Value>::create();
    field.init(StateField_init<ts::Value>([](EditorStateConfig, EditorState) { return ts::Value::number(0); }));
    field.apply(StateField_apply<ts::Value>([](Transaction tr, ts::Value prev, EditorState, EditorState) {
        return ts::Value::number(prev.to_number() + (tr.docChanged() ? 1.0 : 0.0));
    }));

    PluginSpec<ts::Value> pspec = PluginSpec<ts::Value>::create();
    pspec.key(app->audit_key);
    pspec.state(field);
    pspec.filterTransaction(PluginSpec_filterTransaction([app](Transaction tr, EditorState) {
        if (app->locked && tr.docChanged()) {
            app->blocked++;
            app->last_action = "Blocked document mutation (filterTransaction=false)";
            return false;
        }
        return true;
    }));

    EditorStateConfig scfg = EditorStateConfig::create();
    scfg.schema(app->schema);
    scfg.doc(make_initial_doc(app->schema));
    scfg.plugins(ts::Array<Plugin<ts::Value>>{ Plugin<ts::Value>::create(pspec) });

    DirectEditorProps props = DirectEditorProps::create();
    props.state(EditorState::create(scfg));
    props.dispatchTransaction(Transaction_fn([app](Transaction tr) {
        if (app->view.h == 0) return;
        auto res = app->view.state().applyTransaction(tr);
        if (res.transactions().size() > 0) {
            if (tr.docChanged()) {
                auto steps = tr.steps();
                auto docs = tr.docs();
                std::vector<Step> inv;
                for (int k = static_cast<int>(steps.size()) - 1; k >= 0; k--) {
                    inv.push_back(steps[k].invert(docs[k]));
                }
                ts::Value hm = tr.getMeta(string_or_Plugin_or_PluginKey(std::string("history")));
                if (hm.is_null()) {
                    app->undo_stack.push_back(std::move(inv));
                    app->redo_stack.clear();
                } else if (hm.to_string() == "undo") {
                    app->redo_stack.push_back(std::move(inv));
                } else {
                    app->undo_stack.push_back(std::move(inv));
                }
            }
            app->view.updateState(res.state());
        }
        update_meta(app, app->view.state());
    }));

    dom::HTMLElement mount = *app->doc.getElementById("editor");
    dist_DOMNode place;
    static_cast<ts::Handle&>(place) = mount;
    app->view = EditorView::create(EditorView_new_place_or_dist_DOMNode(place), props);
    update_meta(app, app->view.state());

    const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        dom::HTMLElement btn = *app->doc.getElementById(ids[i]);
        ts::Listener<dom::PointerEvent> cb = [app, i](dom::PointerEvent) {
            EditorState st = app->view.state();
            if (i == 0) {
                app->rev++;
                char msg[96];
                std::snprintf(msg, sizeof msg, "Revision #%d recorded via Transform step.", app->rev);
                Transaction tr = st.tr();
                tr.insert(st.doc().content().size(), Fragment_or_Node_or_Node_array(p_node(app->schema, msg)));
                if (!app->locked) app->last_action = std::string("Appended Revision #") + std::to_string(app->rev);
                call_Transaction_fn(app->view.dispatch(), tr);
            } else if (i == 1) {
                Node doc = st.doc();
                if (doc.childCount() >= 2.0) {
                    double fp = doc.child(0).nodeSize() + 1.0;
                    double tp = fp + doc.child(1).content().size();
                    if (tp > fp) {
                        Transaction tr = st.tr();
                        tr.addMark(fp, tp, app->schema.mark(string_or_MarkType(std::string("strong"))));
                        if (!app->locked) app->last_action = "Bolded first body paragraph";
                        call_Transaction_fn(app->view.dispatch(), tr);
                    }
                }
            } else if (i == 2) {
                if (app->undo_stack.empty()) {
                    app->last_action = "Undo stack empty";
                    update_meta(app, st);
                } else {
                    std::vector<Step> batch = app->undo_stack.back();
                    if (!app->locked) app->undo_stack.pop_back();
                    Transaction tr = st.tr();
                    tr.setMeta(string_or_Plugin_or_PluginKey(std::string("history")), ts::Value::string("undo"));
                    for (const Step& s : batch) tr.step(s);
                    if (!app->locked) app->last_action = "Undid step(s) via Step.invert";
                    call_Transaction_fn(app->view.dispatch(), tr);
                }
            } else if (i == 3) {
                if (app->redo_stack.empty()) {
                    app->last_action = "Redo stack empty";
                    update_meta(app, st);
                } else {
                    std::vector<Step> batch = app->redo_stack.back();
                    if (!app->locked) app->redo_stack.pop_back();
                    Transaction tr = st.tr();
                    tr.setMeta(string_or_Plugin_or_PluginKey(std::string("history")), ts::Value::string("redo"));
                    for (const Step& s : batch) tr.step(s);
                    if (!app->locked) app->last_action = "Redid inverted step(s)";
                    call_Transaction_fn(app->view.dispatch(), tr);
                }
            } else {
                app->locked = !app->locked;
                app->last_action = app->locked ? "Enabled read-only lock" : "Disabled read-only lock";
                update_meta(app, st);
            }
        };
        btn.addEventListener(dom::HTMLElementEventMapKeys::click, cb);
    }
}
