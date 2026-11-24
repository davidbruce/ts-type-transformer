#include "dom/dom.hpp"
#include "marked/marked.hpp"
#include <vector>

static dom::HTMLElement make_el(dom::Document doc, std::string_view tag, std::string_view cls, std::string_view text) {
    dom::HTMLElement el = doc.createElement(tag);
    if (!cls.empty()) el.className(cls);
    if (!text.empty()) el.textContent(text);
    return el;
}

TS_EXPORT(start) void start() {
    auto doc = dom::get_document();
    auto app = *doc.getElementById("app");

    std::string md =
        "# Wireframe Spec\n\n"
        "## Custom Pipeline\n\n"
        "Paragraph with *italic* and **bold** and `code`.\n";

    std::vector<std::string> visited;

    using Mk = marked::Marked<std::string, std::string>;
    auto rend = marked::MarkedExtension_renderer<std::string, std::string>({.heading = [](ts::Tuple<marked::Tokens_Heading> args) {
        auto h = args.get<0>();
        auto d = std::to_string(int(h.depth()));
        return std::string("<h") + d + " class=\"wire-h\">\xC2\xA7 " + h.text() + "</h" + d + ">\n";
    }});
    ts::Scoped<marked::MarkedExtension_walkTokens> on_token = [&](const marked::Token& token) {
        std::visit([&](const auto& x) {
            using X = std::decay_t<decltype(x)>;
            if constexpr (std::is_same_v<X, marked::Tokens_Generic>) visited.push_back(x.type());
            else visited.push_back(std::string(X::type));
        }, token);
    };
    auto ext = marked::MarkedExtension<std::string, std::string>({.renderer = rend, .walkTokens = on_token});

    Mk custom_m = Mk(ext);
    std::string def_html = marked::marked(md, marked::marked_options_no_async::create());
    std::string cust_html = std::get<std::string>(custom_m.parse()(md));

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
