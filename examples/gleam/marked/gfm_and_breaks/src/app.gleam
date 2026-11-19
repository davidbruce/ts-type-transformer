import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import gleam/option.{None, Some}
import marked/areas/main as marked
import marked/areas/main/marked_options_no_async

fn make_pane(doc, title: String, html: String) {
  let card = document.create_element_div(doc, None)
  element.set_class_name(card, "card")
  let lbl = document.create_element_div(doc, None)
  element.set_class_name(lbl, "label")
  node.set_text_content(lbl, Some(title))
  let out = document.create_element_div(doc, None)
  element.set_class_name(out, "md-out")
  element.set_inner_html(out, html)
  let raw = document.create_element_pre(doc, None)
  element.set_class_name(raw, "html-raw")
  node.set_text_content(raw, Some(html))
  let _ = node.append_child(card, lbl)
  let _ = node.append_child(card, out)
  let _ = node.append_child(card, raw)
  card
}

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "app") {
    None -> Nil
    Some(app) -> {
      let head = document.create_element_div(doc, None)
      element.set_class_name(head, "head")
      let title = document.create_element_h1(doc, None)
      node.set_text_content(title, Some("Marked GFM & Line Breaks (Gleam)"))
      let badge = document.create_element_span(doc, None)
      element.set_class_name(badge, "badge")
      node.set_text_content(badge, Some("MarkedOptions"))
      let _ = node.append_child(head, title)
      let _ = node.append_child(head, badge)
      let _ = node.append_child(app, head)

      let gfm_src =
        "| Flag | Mode |\n|---|---|\n| gfm | table |\n\n~~deprecated~~\n"
      let gfm_on = marked_options_no_async.create()
      marked_options_no_async.set_gfm(gfm_on, Some(True))
      let gfm_off = marked_options_no_async.create()
      marked_options_no_async.set_gfm(gfm_off, Some(False))

      let gfm_sec = document.create_element_div(doc, None)
      element.set_class_name(gfm_sec, "section")
      let gfm_lbl = document.create_element_div(doc, None)
      element.set_class_name(gfm_lbl, "label")
      node.set_text_content(gfm_lbl, Some("GitHub Flavored Markdown (gfm)"))
      let gfm_grid = document.create_element_div(doc, None)
      element.set_class_name(gfm_grid, "grid")
      let _ =
        node.append_child(
          gfm_grid,
          make_pane(doc, "gfm: true", marked.marked(gfm_src, gfm_on)),
        )
      let _ =
        node.append_child(
          gfm_grid,
          make_pane(doc, "gfm: false", marked.marked(gfm_src, gfm_off)),
        )
      let _ = node.append_child(gfm_sec, gfm_lbl)
      let _ = node.append_child(gfm_sec, gfm_grid)
      let _ = node.append_child(app, gfm_sec)

      let breaks_src = "Line one\nLine two\nLine three"
      let br_off = marked_options_no_async.create()
      marked_options_no_async.set_breaks(br_off, Some(False))
      let br_on = marked_options_no_async.create()
      marked_options_no_async.set_breaks(br_on, Some(True))

      let br_sec = document.create_element_div(doc, None)
      element.set_class_name(br_sec, "section")
      let br_lbl = document.create_element_div(doc, None)
      element.set_class_name(br_lbl, "label")
      node.set_text_content(br_lbl, Some("Soft Line Breaks (breaks)"))
      let br_grid = document.create_element_div(doc, None)
      element.set_class_name(br_grid, "grid")
      let _ =
        node.append_child(
          br_grid,
          make_pane(doc, "breaks: false", marked.marked(breaks_src, br_off)),
        )
      let _ =
        node.append_child(
          br_grid,
          make_pane(doc, "breaks: true", marked.marked(breaks_src, br_on)),
        )
      let _ = node.append_child(br_sec, br_lbl)
      let _ = node.append_child(br_sec, br_grid)
      let _ = node.append_child(app, br_sec)
      Nil
    }
  }
}
