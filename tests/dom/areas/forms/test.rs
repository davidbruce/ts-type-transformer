// Forms from Rust: controls made and read, a form's data, validation and
// reset. Every check! must hold.
#![no_main]
use bindings::*;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "check"]
    fn check_at(ok: i32, line: u32);
}
macro_rules! check {
    ($c:expr) => {
        unsafe { check_at($c as i32, line!()) }
    };
}

// FINDING 2: the DOM's `Option` constructor (areas_core.rs:7804) is exported by `use bindings::*`
// and shadows the prelude's Option (awkward): the standard one must be spelled out.
fn text(v: std::option::Option<FormDataEntryValue>) -> std::option::Option<String> {
    match v {
        Some(FormDataEntryValue::String(s)) => Some(s),
        _ => None,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    let doc = document();
    let body = doc.body();

    // A form built from markup, its controls found by selector.
    let form: HTMLFormElement = doc.create_element(HTMLElementTagNameMap::FORM, None);
    form.set_id("signup");
    form.set_inner_html(
        "<input type=text name=user value=ada required pattern='[a-z]+'>\
         <input type=number name=age min=1 max=120 value=36>\
         <input type=checkbox name=news value=yes checked>\
         <input type=checkbox name=terms value=ok>\
         <select name=color><option value=r>Red</option><option value=g selected>Green</option><option value=b>Blue</option></select>\
         <textarea name=bio>hello</textarea>\
         <input type=text name=tags value=one><input type=text name=tags value=two>",
    );
    body.append_child(form.clone());
    check!(form.elements().length() == 8.0);
    check!(form.length() == 8.0);
    check!(form.id() == "signup" && form.method() == "get");
    form.set_name("signupForm");
    check!(form.name() == "signupForm");
    check!(form.get_attribute("name").as_deref() == Some("signupForm"));
    check!(doc.get_element_by_id("signup") == Some(form.clone().into()));

    let user = form.query_selector_as::<HTMLInputElement>("[name=user]").unwrap();
    let age = form.query_selector_as::<HTMLInputElement>("[name=age]").unwrap();
    let news = form.query_selector_as::<HTMLInputElement>("[name=news]").unwrap();
    let terms = form.query_selector_as::<HTMLInputElement>("[name=terms]").unwrap();
    let color = form.query_selector_as::<HTMLSelectElement>("select").unwrap();
    let bio = form.query_selector_as::<HTMLTextAreaElement>("textarea").unwrap();

    // Text input: value, attributes, properties.
    check!(user.value() == "ada" && user.type_() == "text" && user.name() == "user");
    check!(user.form() == Some(form.clone()));
    check!(user.required() && user.pattern() == "[a-z]+");
    user.set_value("grace");
    check!(user.value() == "grace");
    // The value property is not the attribute; defaultValue is.
    check!(user.default_value() == "ada" && user.get_attribute("value").as_deref() == Some("ada"));
    user.set_placeholder("your name");
    check!(user.get_attribute("placeholder").as_deref() == Some("your name"));
    user.set_value("hello world");
    user.set_selection_range(Some(0.0), Some(5.0), None);
    check!(user.selection_start() == Some(0.0) && user.selection_end() == Some(5.0));
    check!(user.max_length() == -1.0);
    user.set_max_length(8.0);
    check!(user.max_length() == 8.0);

    // Number input.
    check!(age.type_() == "number" && age.value() == "36" && age.value_as_number() == 36.0);
    age.set_value_as_number(41.0);
    check!(age.value() == "41");
    age.step_up(Some(2.0));
    check!(age.value() == "43");
    age.step_down(None);
    check!(age.value() == "42");
    age.set_value("500");
    check!(!age.validity().valid() && age.validity().range_overflow());
    check!(!age.check_validity());
    age.set_value("0");
    check!(age.validity().range_underflow());
    age.set_value("36");
    check!(age.validity().valid());

    // Checkboxes.
    check!(news.checked() && news.default_checked() && !terms.checked());
    terms.click();
    check!(terms.checked());
    terms.set_checked(false);
    check!(!terms.checked() && !terms.default_checked());
    news.set_indeterminate(true);
    check!(news.indeterminate());
    news.set_indeterminate(false);

    // A select: selectedIndex, value, options.
    check!(color.selected_index() == 1.0 && color.value() == "g");
    check!(color.length() == 3.0 && color.options().length() == 3.0);
    color.set_selected_index(2.0);
    check!(color.value() == "b");
    color.set_value("r");
    check!(color.selected_index() == 0.0);
    let green = color.options().item(1.0).unwrap();
    check!(green.value() == "g" && green.text() == "Green" && green.index() == 1.0 && green.default_selected());
    green.set_selected(true);
    check!(color.selected_index() == 1.0 && green.selected());
    check!(color.item(2.0).map(|o| o.text()).as_deref() == Some("Blue"));
    check!(color.selected_options().length() == 1.0);
    // An option made here and added.
    let extra: HTMLOptionElement = doc.create_element(HTMLElementTagNameMap::OPTION, None);
    extra.set_value("k");
    extra.set_text("Black");
    color.add(extra.clone(), None::<HTMLElementOrNumber>);
    check!(color.length() == 4.0 && extra.index() == 3.0);
    color.set_value("k");
    check!(color.selected_index() == 3.0);
    color.remove_with_index(3.0);
    check!(color.length() == 3.0);
    check!(color.selected_index() == -1.0 || color.selected_index() == 0.0);
    color.set_value("g");
    check!(color.type_() == HTMLSelectElementType::SelectOne);
    color.set_multiple(true);
    check!(color.type_() == HTMLSelectElementType::SelectMultiple);
    color.set_multiple(false);
    color.set_value("g");

    // A textarea.
    check!(bio.value() == "hello" && bio.default_value() == "hello" && bio.text_content() == "hello");
    bio.set_value("line one\nline two");
    check!(bio.text_length() == 17.0);
    bio.set_rows(4.0);
    bio.set_cols(30.0);
    check!(bio.rows() == 4.0 && bio.cols() == 30.0 && bio.type_() == "textarea");
    bio.set_value("hello");
    bio.set_selection_range(Some(0.0), Some(1.0), None);
    bio.set_range_text("J");
    check!(bio.value() == "Jello");

    // FormData from the form: names, values, only checked boxes, repeated names.
    bio.set_value("hello");
    user.set_value("grace");
    let data = FormData::new(Some(&form), None);
    check!(text(data.get("user")).as_deref() == Some("grace"));
    check!(text(data.get("age")).as_deref() == Some("36"));
    check!(text(data.get("news")).as_deref() == Some("yes"));
    check!(!data.has("terms") && data.get("terms").is_none());
    check!(text(data.get("color")).as_deref() == Some("g"));
    check!(text(data.get("bio")).as_deref() == Some("hello"));
    let tags: Vec<String> = data.get_all("tags").iter().filter_map(|v| text(Some(v))).collect();
    check!(tags == ["one", "two"]);
    // FormData's own edits.
    data.append_with_string("extra", "1");
    data.set_with_string("user", "linus");
    data.delete("age");
    check!(data.has("extra") && !data.has("age"));
    check!(text(data.get("user")).as_deref() == Some("linus"));
    let keys: Vec<String> = data.keys().iter().collect();
    check!(keys.first().map(String::as_str) == Some("user") && keys.contains(&"extra".to_string()));
    let entries = data.iter().filter(|e| e.get0() == "tags").count();
    check!(entries == 2);

    // Validity: required, pattern, and a custom message.
    user.set_value("");
    check!(user.will_validate() && user.validity().value_missing() && !user.validity().valid());
    check!(!user.check_validity() && !form.check_validity());
    check!(!user.validation_message().is_empty());
    user.set_value("ADA");
    check!(user.validity().pattern_mismatch() && !user.validity().value_missing());
    user.set_value("ada");
    check!(user.validity().valid() && user.check_validity() && form.check_validity());
    user.set_custom_validity("taken");
    check!(user.validity().custom_error() && !user.validity().valid());
    check!(user.validation_message() == "taken" && !user.check_validity());
    user.set_custom_validity("");
    check!(user.validity().valid() && user.validation_message().is_empty());
    // A form with an invalid control is invalid.
    user.set_value("Z");
    check!(!form.check_validity());
    form.set_no_validate(true);
    check!(form.no_validate() && form.has_attribute("novalidate"));
    form.set_no_validate(false);

    // Disabled controls are left out of the data and of validation.
    user.set_disabled(true);
    check!(!user.will_validate() && form.check_validity());
    check!(!FormData::new(Some(&form), None).has("user"));
    user.set_disabled(false);

    // reset restores each control's default.
    user.set_value("changed");
    age.set_value("99");
    news.set_checked(false);
    terms.set_checked(true);
    color.set_value("b");
    bio.set_value("other");
    form.reset();
    check!(user.value() == "ada" && age.value() == "36");
    check!(news.checked() && !terms.checked());
    check!(color.value() == "g" && bio.value() == "hello");

    // A form-less input made here; its form is None until it is added.
    let loose: HTMLInputElement = doc.create_element(HTMLElementTagNameMap::INPUT, None);
    loose.set_type("email");
    loose.set_value("not an email");
    check!(loose.form().is_none() && loose.validity().type_mismatch());
    loose.set_value("a@b.co");
    check!(loose.validity().valid());
    form.append_child(loose.clone());
    check!(loose.form() == Some(form.clone()) && form.length() == 9.0);
}
