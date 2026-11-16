// Form controls from Gleam: values, selection, validation, FormData and reset.
import bindings/areas/core as dom
import bindings/areas/core/document
import bindings/areas/core/element
import bindings/areas/core/node
import bindings/areas/core/validity_state
import bindings/areas/html/html_collection_base as collection
import bindings/areas/html/html_form_element as form
import bindings/areas/html/html_input_element as input
import bindings/areas/html/html_option_element as option_el
import bindings/areas/html/html_select_element as select
import bindings/areas/html/html_text_area_element as textarea
import bindings/areas/network/form_data
import bindings/types as t
import cell
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}

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

  let the_form = document.create_element_form(doc, None)
  form.set_name(the_form, "signup")
  node.append_child(body, the_form)

  // A text input: value, default value, attributes.
  let user = document.create_element_input(doc, None)
  input.set_type(user, "text")
  input.set_name(user, "user")
  input.set_default_value(user, "anon")
  input.set_placeholder(user, "your name")
  node.append_child(the_form, user)
  check(input.get_value(user) == "anon", "default value shows")
  input.set_value(user, "alice")
  check(input.get_value(user) == "alice", "value set")
  check(input.get_default_value(user) == "anon", "default value unchanged")
  check(element.get_attribute(user, "value") == Some("anon"), "value attribute is the default")
  check(input.get_placeholder(user) == "your name", "placeholder")
  check(input.get_type(user) == "text", "type")
  check(
    option.map(input.get_form(user), fn(f) { form.get_name(f) }) == Some("signup"),
    "input knows its form",
  )

  // A number input.
  let age = document.create_element_input(doc, None)
  input.set_type(age, "number")
  input.set_name(age, "age")
  input.set_min(age, "0")
  input.set_max(age, "120")
  input.set_step(age, "1")
  node.append_child(the_form, age)
  input.set_value(age, "42")
  check(input.get_value_as_number(age) == 42.0, "valueAsNumber")
  input.set_value_as_number(age, 50.0)
  check(input.get_value(age) == "50", "set valueAsNumber")
  input.step_up(age, Some(5.0))
  check(input.get_value(age) == "55", "stepUp")
  input.step_down(age, None)
  check(input.get_value(age) == "54", "stepDown")
  input.set_value(age, "150")
  check(validity_state.get_range_overflow(input.get_validity(age)), "range overflow")
  check(!input.check_validity(age), "overflow is invalid")
  input.set_value(age, "-3")
  check(validity_state.get_range_underflow(input.get_validity(age)), "range underflow")
  input.set_value(age, "30")
  check(input.check_validity(age), "in range is valid")

  // A checkbox.
  let agree = document.create_element_input(doc, None)
  input.set_type(agree, "checkbox")
  input.set_name(agree, "agree")
  node.append_child(the_form, agree)
  check(!input.get_checked(agree), "unchecked at first")
  input.set_checked(agree, True)
  check(input.get_checked(agree), "checked")
  check(input.get_value(agree) == "on", "checkbox default value")
  check(!input.get_default_checked(agree), "default unchecked")

  // A select with options.
  let color = document.create_element_select(doc, None)
  select.set_name(color, "color")
  list.each([#("red", "Red"), #("green", "Green"), #("blue", "Blue")], fn(c) {
    let o = document.create_element_option(doc, None)
    option_el.set_value(o, c.0)
    option_el.set_text(o, c.1)
    node.append_child(color, o)
  })
  node.append_child(the_form, color)
  check(select.get_length(color) == 3.0, "three options")
  check(select.get_selected_index(color) == 0.0, "first selected")
  check(select.get_value(color) == "red", "select value")
  select.set_selected_index(color, 2.0)
  check(select.get_value(color) == "blue", "select by index")
  select.set_value(color, "green")
  check(select.get_selected_index(color) == 1.0, "select by value")
  let options = select.to_list(color)
  check(list.map(options, option_el.get_text) == ["Red", "Green", "Blue"], "option texts")
  check(list.map(options, option_el.get_selected) == [False, True, False], "option selected flags")
  check(
    list.map(collection.to_list(element.get_children(color)), element.get_text_content)
      == ["Red", "Green", "Blue"],
    "options are children",
  )
  let assert Some(green) = select.item(color, 1.0)
  check(option_el.get_index(green) == 1.0, "option index")
  select.set_value(color, "nothing")
  check(select.get_selected_index(color) == -1.0, "no such value selects nothing")
  select.set_selected_index(color, 0.0)

  // A textarea.
  let notes = document.create_element_textarea(doc, None)
  textarea.set_name(notes, "notes")
  textarea.set_value(notes, "hello world")
  node.append_child(the_form, notes)
  check(textarea.get_value(notes) == "hello world", "textarea value")
  check(textarea.get_text_length(notes) == 11.0, "text length")
  textarea.set_selection_range(notes, Some(0.0), Some(5.0), None)
  check(textarea.get_selection_start(notes) == 0.0 && textarea.get_selection_end(notes) == 5.0, "selection range")
  textarea.set_range_text(notes, "HOWDY")
  check(textarea.get_value(notes) == "HOWDY world", "setRangeText")
  check(textarea.get_type(notes) == "textarea", "textarea type")

  // The form's elements.
  check(form.get_length(the_form) == 5.0, "form length")
  check(collection.get_length(form.get_elements(the_form)) == 5.0, "form elements")

  // FormData from the form: text, number, a checked box, the select and text.
  let data = form_data.new(Some(the_form), None)
  check(form_data.get(data, "user") == Some(t.FormDataEntryValueString("alice")), "FormData user")
  check(form_data.get(data, "age") == Some(t.FormDataEntryValueString("30")), "FormData age")
  check(form_data.get(data, "agree") == Some(t.FormDataEntryValueString("on")), "FormData checked box")
  check(form_data.get(data, "color") == Some(t.FormDataEntryValueString("red")), "FormData select")
  check(form_data.get(data, "notes") == Some(t.FormDataEntryValueString("HOWDY world")), "FormData textarea")
  check(form_data.get(data, "missing") == None, "FormData missing")
  input.set_checked(agree, False)
  check(!form_data.has(form_data.new(Some(the_form), None), "agree"), "unchecked box is left out")
  form_data.append_with_string(data, "extra", "1")
  form_data.append_with_string(data, "extra", "2")
  check(form_data.get_all(data, "extra") == [t.FormDataEntryValueString("1"), t.FormDataEntryValueString("2")], "FormData getAll")
  form_data.set_with_string(data, "extra", "3")
  check(form_data.get_all(data, "extra") == [t.FormDataEntryValueString("3")], "FormData set replaces")
  form_data.delete(data, "extra")
  check(!form_data.has(data, "extra"), "FormData delete")
  check(list.length(form_data.to_list(data)) == 5, "FormData entries")

  // Required and pattern validation.
  let code = document.create_element_input(doc, None)
  input.set_type(code, "text")
  input.set_name(code, "code")
  input.set_required(code, True)
  input.set_pattern(code, "[0-9]{3}")
  node.append_child(the_form, code)
  check(validity_state.get_value_missing(input.get_validity(code)), "required: value missing")
  check(!input.check_validity(code), "required and empty is invalid")
  check(!form.check_validity(the_form), "the form is invalid too")
  check(input.get_validation_message(code) != "", "there is a message")
  input.set_value(code, "ab")
  check(validity_state.get_pattern_mismatch(input.get_validity(code)), "pattern mismatch")
  check(!validity_state.get_value_missing(input.get_validity(code)), "no longer missing")
  input.set_value(code, "123")
  check(input.get_validity(code) |> validity_state.get_valid, "pattern matched")
  check(input.check_validity(code), "valid")
  check(form.check_validity(the_form), "the form is valid")

  // A custom validity message.
  input.set_custom_validity(code, "no such code")
  check(validity_state.get_custom_error(input.get_validity(code)), "custom error")
  check(input.get_validation_message(code) == "no such code", "custom message")
  check(!input.check_validity(code), "custom error is invalid")
  input.set_custom_validity(code, "")
  check(input.check_validity(code), "cleared custom error")

  // An email input flags a type mismatch.
  let mail = document.create_element_input(doc, None)
  input.set_type(mail, "email")
  input.set_value(mail, "not an email")
  check(validity_state.get_type_mismatch(input.get_validity(mail)), "email type mismatch")
  input.set_value(mail, "a@b.example")
  check(!validity_state.get_type_mismatch(input.get_validity(mail)), "email ok")

  // A required textarea and select.
  textarea.set_required(notes, True)
  textarea.set_value(notes, "")
  check(validity_state.get_value_missing(textarea.get_validity(notes)), "required textarea")
  textarea.set_value(notes, "x")
  check(textarea.check_validity(notes), "filled textarea")

  // Reset restores defaults.
  input.set_value(user, "bob")
  input.set_checked(agree, True)
  select.set_selected_index(color, 2.0)
  form.reset(the_form)
  check(input.get_value(user) == "anon", "reset text")
  check(!input.get_checked(agree), "reset checkbox")
  check(select.get_selected_index(color) == 0.0, "reset select")
  check(textarea.get_value(notes) == "", "reset textarea")

  io.println(int.to_string(cell.get(count)) <> " checks passed")
}
