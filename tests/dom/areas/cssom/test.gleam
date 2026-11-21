// The CSS object model from Gleam: inline style, computed style, a style
// sheet's rules, and media queries.
import bindings/areas/core as dom
import bindings/areas/core/document
import bindings/areas/core/dom_token_list
import bindings/areas/core/element
import bindings/areas/core/node
import bindings/areas/css/css_rule
import bindings/areas/css/css_rule_list
import bindings/areas/css/css_style_declaration_base as decl
import bindings/areas/css/css_style_properties as props
import bindings/areas/css/css_style_rule
import bindings/areas/css/css_style_sheet
import bindings/areas/html/html_element
import bindings/areas/html/html_element_tag_name_map as tag
import bindings/areas/html/html_style_element
import bindings/areas/media/media_query_list
import cell
import gleam/float
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}
import gleam/string

pub fn main() {
  let count = cell.new(0)
  let check = fn(ok: Bool, what: String) {
    case ok {
      True -> cell.set(count, cell.get(count) + 1)
      False -> panic as what
    }
  }
  let doc = dom.get_document()
  let body = document.get_body(doc)
  let head = document.get_head(doc)

  // Inline style: set, get, remove.
  let box = document.create_element(doc, tag.div, None)
  element.set_id(box, "box")
  node.append_child(body, box)
  let style = html_element.get_style(box)
  decl.set_property(style, "color", Some("red"), None)
  check(decl.get_property_value(style, "color") == "red", "setProperty / getPropertyValue")
  check(props.get_color(style) == "red", "color property")
  props.set_width(style, "120px")
  props.set_display(style, "block")
  check(props.get_width(style) == "120px", "width property")
  check(decl.get_length(style) == 3.0, "three declarations: " <> int.to_string(float.truncate(decl.get_length(style))))
  check(decl.item(style, 0.0) == "color", "item 0")
  check(string.contains(decl.get_css_text(style), "color: red"), "cssText")
  check(element.get_attribute(box, "style") == Some("color: red; width: 120px; display: block;"), "style attribute")
  check(decl.remove_property(style, "width") == "120px", "removeProperty returns the old value")
  check(props.get_width(style) == "", "removed property is empty")
  check(decl.get_length(style) == 2.0, "two declarations left")
  decl.set_property(style, "margin-top", Some("4px"), Some("important"))
  check(decl.get_property_priority(style, "margin-top") == "important", "priority")
  decl.set_css_text(style, "color: blue; padding: 2px")
  check(props.get_color(style) == "blue", "cssText replaced")
  check(decl.get_property_value(style, "padding-top") == "2px", "shorthand expands")
  html_element.set_style(box, "")
  check(decl.get_length(style) == 0.0, "cleared by element style setter")

  // Computed style resolves values.
  props.set_color(style, "red")
  props.set_font_size(style, "20px")
  props.set_width(style, "50%")
  let computed = dom.get_computed_style(box, None)
  check(props.get_color(computed) == "rgb(255, 0, 0)", "computed color")
  check(props.get_font_size(computed) == "20px", "computed font size")
  check(decl.get_property_value(computed, "display") == "block", "computed display of a div")
  props.set_display(style, "none")
  check(decl.get_property_value(dom.get_computed_style(box, None), "display") == "none", "computed display none")
  html_element.set_style(box, "width: 80px; height: 30px; padding: 5px")
  check(decl.get_property_value(dom.get_computed_style(box, None), "width") == "80px", "computed width")
  check(element.get_client_width(box) == 90.0, "clientWidth includes padding")
  check(html_element.get_offset_height(box) == 40.0, "offsetHeight")

  // A style element and its sheet.
  let sheet_el = document.create_element(doc, tag.style, None)
  element.set_text_content(
    sheet_el,
    Some(".big { font-size: 30px; } .big.bold { font-weight: 700; } p { margin: 0; }"),
  )
  node.append_child(head, sheet_el)
  let assert Some(sheet) = html_style_element.get_sheet(sheet_el)
  let rules = css_style_sheet.get_css_rules(sheet)
  check(css_rule_list.get_length(rules) == 3.0, "three rules")
  let assert Some(first) = css_rule_list.item(rules, 0.0)
  check(css_rule.get_type(first) == css_rule.style_rule, "rule type")
  let assert Ok(first_style) = css_style_rule.from(first)
  check(css_style_rule.get_selector_text(first_style) == ".big", "selectorText")
  check(props.get_font_size(css_style_rule.get_style(first_style)) == "30px", "rule font size")
  check(string.contains(css_rule.get_css_text(first), "font-size: 30px"), "rule cssText")
  check(
    option.map(css_rule.get_parent_style_sheet(first), fn(_) { True }) == Some(True),
    "rule knows its sheet",
  )
  let texts = list.map(css_rule_list.to_list(rules), css_rule.get_css_text)
  check(list.length(texts) == 3, "rules as a list")

  // Class-driven computed style.
  let target = document.create_element(doc, tag.div, None)
  node.append_child(body, target)
  check(props.get_font_size(dom.get_computed_style(target, None)) == "16px", "default font size")
  dom_token_list.add(element.get_class_list(target), ["big"])
  check(props.get_font_size(dom.get_computed_style(target, None)) == "30px", "class sets font size")
  check(decl.get_property_value(dom.get_computed_style(target, None), "font-weight") == "400", "not bold yet")
  dom_token_list.add(element.get_class_list(target), ["bold"])
  check(decl.get_property_value(dom.get_computed_style(target, None), "font-weight") == "700", "compound selector")
  dom_token_list.remove(element.get_class_list(target), ["big", "bold"])
  check(props.get_font_size(dom.get_computed_style(target, None)) == "16px", "class removed")

  // insertRule / deleteRule change the page.
  let at = css_style_sheet.insert_rule(sheet, ".tinted { color: rgb(0, 0, 255); }", Some(0.0))
  check(at == 0.0, "insertRule returns the index")
  check(css_rule_list.get_length(css_style_sheet.get_css_rules(sheet)) == 4.0, "four rules")
  dom_token_list.add(element.get_class_list(target), ["tinted"])
  check(props.get_color(dom.get_computed_style(target, None)) == "rgb(0, 0, 255)", "inserted rule applies")
  let assert Some(inserted) = css_rule_list.item(css_style_sheet.get_css_rules(sheet), 0.0)
  let assert Ok(inserted_style) = css_style_rule.from(inserted)
  check(css_style_rule.get_selector_text(inserted_style) == ".tinted", "inserted selector")
  css_style_rule.set_selector_text(inserted_style, ".tinted2")
  check(props.get_color(dom.get_computed_style(target, None)) != "rgb(0, 0, 255)", "selector change unapplies it")
  decl.set_property(css_style_rule.get_style(inserted_style), "color", Some("rgb(0, 128, 0)"), None)
  dom_token_list.add(element.get_class_list(target), ["tinted2"])
  check(props.get_color(dom.get_computed_style(target, None)) == "rgb(0, 128, 0)", "rule's style edited")
  css_style_sheet.delete_rule(sheet, 0.0)
  check(css_rule_list.get_length(css_style_sheet.get_css_rules(sheet)) == 3.0, "deleteRule")
  check(props.get_color(dom.get_computed_style(target, None)) != "rgb(0, 128, 0)", "deleted rule no longer applies")

  // A constructed sheet, replaced synchronously.
  let built = css_style_sheet.new(None)
  css_style_sheet.replace_sync(built, "a { color: red } b { color: blue }")
  check(css_rule_list.get_length(css_style_sheet.get_css_rules(built)) == 2.0, "constructed sheet rules")

  // Media queries.
  check(media_query_list.get_matches(dom.match_media("(min-width: 1px)")), "min-width matches")
  check(!media_query_list.get_matches(dom.match_media("(max-width: 1px)")), "max-width does not")
  check(media_query_list.get_media(dom.match_media("screen and (min-width: 1px)")) == "screen and (min-width: 1px)", "media text")
  check(media_query_list.get_matches(dom.match_media("all")), "all matches")
  check(!media_query_list.get_matches(dom.match_media("print")), "print does not")

  io.println(int.to_string(cell.get(count)) <> " checks passed")
}
