#include "dom/dom.hpp"
#include "marked/marked.hpp"
#include <vector>

static dom::HTMLElement make_el(dom::Document doc, std::string_view tag, std::string_view cls, std::string_view text) {
    dom::HTMLElement el = doc.createElement(tag);
    if (!cls.empty()) el.className(cls);
    if (!text.empty()) el.textContent(text);
    return el;
}

template <class T>
static T as_tok(const ts::Value& v) {
    T t;
    static_cast<ts::Handle&>(t) = v;
    return t;
}

extern "C" __attribute__((export_name("start"))) void start() {
    dom::Document doc = dom::get_document();
    dom::HTMLElement app = *doc.getElementById("app");

    std::string md =
        "# Wireframe Spec\n\n"
        "## Custom Pipeline\n\n"
        "Paragraph with *italic* and **bold** and `code`.\n";

    std::vector<std::string> visited;

    using Mk = marked::Marked<std::string, std::string>;
    marked::MarkedExtension<std::string, std::string> ext = marked::MarkedExtension<std::string, std::string>::create();
    marked::MarkedExtension_renderer<std::string, std::string> rend = marked::MarkedExtension_renderer<std::string, std::string>::create();
    rend.heading(marked::MarkedExtension_renderer_heading<std::string, std::string>([](ts::Tuple<marked::Tokens_Heading> args) {
        marked::Tokens_Heading h = args.get<0>();
        std::string d = std::to_string(int(h.depth()));
        return std::string("<h") + d + " class=\"wire-h\">\xC2\xA7 " + h.text() + "</h" + d + ">\n";
    }));
    ext.renderer(rend);
    ext.walkTokens(marked::Token_to_void_or_Promise_or_void_or_Promise_array_fn([&](ts::Value token) {
        visited.push_back(as_tok<marked::Tokens_Generic>(token).type());
        return marked::void_or_Promise_or_void_or_Promise_array(std::in_place_index<1>, ts::Array<>());
    }));

    Mk custom_m = Mk::create({ ext });
    std::string def_html = marked::marked(md, marked::marked_options_no_async::create());
    std::string cust_html = std::get<std::string>(custom_m.parse().invoke(md));

    dom::HTMLElement head = make_el(doc, "div", "head", "");
    head.appendChild(make_el(doc, "h1", "", "Marked Custom Renderer & Walker (C++)"));
    head.appendChild(make_el(doc, "span", "badge", std::to_string(visited.size()) + " tokens visited"));
    app.appendChild(head);

    dom::HTMLElement grid = make_el(doc, "div", "grid", "");

    dom::HTMLElement def_card = make_el(doc, "div", "card", "");
    def_card.appendChild(make_el(doc, "div", "label", "Default Renderer"));
    dom::HTMLElement def_out = make_el(doc, "div", "md-out", "");
    def_out.innerHTML(def_html);
    def_card.appendChild(def_out);

    dom::HTMLElement cust_card = make_el(doc, "div", "card", "");
    cust_card.appendChild(make_el(doc, "div", "label", "Custom Heading Renderer"));
    dom::HTMLElement cust_out = make_el(doc, "div", "md-out", "");
    cust_out.innerHTML(cust_html);
    cust_card.appendChild(cust_out);

    grid.appendChild(def_card);
    grid.appendChild(cust_card);
    app.appendChild(grid);

    dom::HTMLElement walk_card = make_el(doc, "div", "card", "");
    walk_card.appendChild(make_el(doc, "div", "label", "Visited Tokens (walkTokens)"));
    dom::HTMLElement strip = make_el(doc, "div", "tok-strip", "");
    for (const std::string& kind : visited) {
        strip.appendChild(make_el(doc, "span", "tok-pill", kind));
    }
    walk_card.appendChild(strip);
    app.appendChild(walk_card);
}
