// Forms from C++: inputs, a select, a textarea, FormData, validity and reset.
// Every CHECK must hold.
#include <string>

#include "areas/core/Document.hpp"
#include "areas/core/functions.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

extern "C" __attribute__((export_name("run"))) void run() {
    Document doc = get_document();
    HTMLElement body = doc.body();

    // A signup form: some fields made with createElement, a select from markup.
    HTMLFormElement form = doc.createElement(HTMLElementTagNameMapKeys::form);
    form.id("signup");
    form.name("signup");
    form.innerHTML(
        "<input type=text name=user value=ada required pattern='[a-z]+'>"
        "<input type=number name=age min=1 max=120 value=36>"
        "<input type=checkbox name=news value=yes>"
        "<select name=color><option value=r>Red</option><option value=g selected>Green</option>"
        "<option value=b>Blue</option></select>"
        "<textarea name=bio>hello</textarea>");
    body.appendChild(form);
    CHECK(form.length() == 5);
    CHECK(form.elements().length() == 5);

    HTMLInputElement user = *HTMLInputElement::cast(*form.querySelector("input[name=user]"));
    HTMLInputElement age = *HTMLInputElement::cast(*form.querySelector("input[name=age]"));
    HTMLInputElement news = *HTMLInputElement::cast(*form.querySelector("input[name=news]"));
    HTMLSelectElement color = *HTMLSelectElement::cast(*form.querySelector("select"));
    HTMLTextAreaElement bio = *HTMLTextAreaElement::cast(*form.querySelector("textarea"));

    // Text input: value, type, form, defaultValue.
    CHECK(user.value() == "ada" && user.type() == "text");
    CHECK(user.form() && *user.form() == form);
    CHECK(user.required() && user.pattern() == "[a-z]+");
    user.value("grace");
    CHECK(user.value() == "grace" && user.defaultValue() == "ada");
    CHECK(user.getAttribute("value") == "ada");

    // Number input.
    CHECK(age.type() == "number" && age.value() == "36" && age.valueAsNumber() == 36);
    age.valueAsNumber(41);
    CHECK(age.value() == "41");
    age.value("200");
    CHECK(age.validity().rangeOverflow() && !age.checkValidity());
    age.value("0");
    CHECK(age.validity().rangeUnderflow());
    age.value("41");
    CHECK(age.checkValidity() && age.validity().valid());

    // Checkbox: checked.
    CHECK(!news.checked());
    news.checked(true);
    CHECK(news.checked() && news.type() == "checkbox");
    news.click();
    CHECK(!news.checked());
    news.checked(true);

    // Select: selectedIndex, value, options.
    CHECK(color.selectedIndex() == 1 && color.value() == "g");
    CHECK(color.options().length() == 3 && color.length() == 3);
    color.selectedIndex(2);
    CHECK(color.value() == "b");
    color.value("r");
    CHECK(color.selectedIndex() == 0);
    HTMLOptionElement green = *color.item(1);
    CHECK(green.text() == "Green" && green.value() == "g" && green.index() == 1 && !green.selected());
    green.selected(true);
    CHECK(color.selectedIndex() == 1 && color.selectedOptions().length() == 1);
    HTMLOptionElement extra = doc.createElement(HTMLElementTagNameMapKeys::option);
    extra.value("y");
    extra.text("Yellow");
    color.add(extra);
    CHECK(color.length() == 4);
    color.value("y");
    CHECK(color.selectedIndex() == 3);
    color.remove(3);
    CHECK(color.length() == 3);
    std::string labels;
    for (HTMLOptionElement o : color) labels += o.text() + ",";
    CHECK(labels == "Red,Green,Blue,");
    color.value("g");

    // Textarea.
    CHECK(bio.value() == "hello" && bio.textLength() == 5 && bio.type() == "textarea");
    bio.value("line one\nline two");
    CHECK(bio.value() == "line one\nline two");
    bio.setSelectionRange(0, 4);
    CHECK(bio.selectionStart() == 0 && bio.selectionEnd() == 4);
    CHECK(bio.defaultValue() == "hello");

    // FormData reads the form's fields.
    FormData data = FormData::create(form);
    CHECK(data.has("user") && data.has("age") && data.has("color") && data.has("bio") && data.has("news"));
    CHECK(!data.has("nothing"));
    CHECK(std::get<std::string>(*data.get("user")) == "grace");
    CHECK(std::get<std::string>(*data.get("age")) == "41");
    CHECK(std::get<std::string>(*data.get("news")) == "yes");
    CHECK(std::get<std::string>(*data.get("color")) == "g");
    news.checked(false);
    CHECK(!FormData::create(form).has("news"));

    // FormData built by hand: append, set, delete.
    // FINDING 3: a string literal is ambiguous between append(name, string_view) and
    // append(name, const string_or_Blob&); a std::string_view picks the first.
    using sv = std::string_view;
    FormData made = FormData::create();
    made.append("tag", sv("a"));
    made.append("tag", sv("b"));
    CHECK(made.getAll("tag").size() == 2);
    made.set("tag", sv("c"));
    CHECK(made.getAll("tag").size() == 1 && std::get<std::string>(*made.get("tag")) == "c");
    made.delete_("tag");
    CHECK(!made.has("tag"));
    CHECK(!made.get("tag"));

    // Validity: required, pattern, custom errors.
    user.value("");
    CHECK(user.validity().valueMissing() && !user.checkValidity() && !user.validity().valid());
    CHECK(user.validationMessage() != "");
    user.value("ADA9");
    CHECK(user.validity().patternMismatch() && !user.validity().valueMissing());
    user.value("ada");
    CHECK(user.checkValidity() && user.validationMessage() == "");
    user.setCustomValidity("taken");
    CHECK(user.validity().customError() && !user.checkValidity() && user.validationMessage() == "taken");
    CHECK(!form.checkValidity());
    user.setCustomValidity("");
    CHECK(user.checkValidity() && form.checkValidity());
    CHECK(user.willValidate());

    // Reset restores the defaults.
    user.value("changed");
    age.value("99");
    color.value("b");
    bio.value("other");
    news.checked(true);
    form.reset();
    CHECK(user.value() == "ada" && age.value() == "36" && color.value() == "g");
    CHECK(bio.value() == "hello" && !news.checked());

    // Disabled fields are left out of FormData.
    age.disabled(true);
    CHECK(!FormData::create(form).has("age"));
    age.disabled(false);
    CHECK(FormData::create(form).has("age"));
}
