// Forms from C: controls made and filled, read back, validated, gathered into
// FormData and reset. Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)
#define STR(s, text) JsString_equals((s), (text))

static int submits;
static void on_submit(void* ctx, Event ev) {
    (void)ctx;
    submits++;
    Event_preventDefault(ev);
}

static HTMLInputElement input(Document doc, HTMLFormElement form, const char* type, const char* name) {
    HTMLInputElement in = Document_createElement_input(doc);
    HTMLInputElement_set_type(in, type);
    HTMLInputElement_set_name(in, name);
    Node_appendChild(form, in);
    return in;
}

static bool entry_is(FormData data, const char* name, const char* text) {
    FormDataEntryValue v = FormData_get(data, name);
    return v.tag == FormDataEntryValue_string && JsString_equals(v.as.string, text);
}

__attribute__((export_name("run"))) void run(void) {
    Document doc = get_document();
    HTMLElement body = Document_get_body(doc);

    HTMLFormElement form = Document_createElement_form(doc);
    HTMLFormElement_set_name(form, "signup");
    HTMLFormElement_set_action(form, "#submitted");
    HTMLFormElement_set_method(form, "post");
    Node_appendChild(body, form);
    CHECK(STR(HTMLFormElement_get_name(form), "signup"));
    CHECK(STR(HTMLFormElement_get_method(form), "post"));
    CHECK(HTMLFormElement_get_length(form) == 0);

    // Text, number and checkbox inputs.
    HTMLInputElement user = input(doc, form, "text", "user");
    Element_setAttribute(user, "value", "ann");
    HTMLInputElement age = input(doc, form, "number", "age");
    HTMLInputElement agree = input(doc, form, "checkbox", "agree");
    HTMLInputElement_set_defaultChecked(agree, true);
    HTMLInputElement_set_value(agree, "yes");
    CHECK(STR(HTMLInputElement_get_type(user), "text") && STR(HTMLInputElement_get_type(agree), "checkbox"));
    CHECK(STR(HTMLInputElement_get_value(user), "ann") && STR(HTMLInputElement_get_defaultValue(user), "ann"));
    CHECK(HTMLInputElement_get_checked(agree));
    HTMLInputElement_set_value(age, "41");
    CHECK(HTMLInputElement_get_valueAsNumber(age) == 41);
    HTMLInputElement_set_valueAsNumber(age, 42.5);
    CHECK(STR(HTMLInputElement_get_value(age), "42.5"));
    Element_setAttribute(age, "step", "any");
    HTMLInputElement_set_value(age, "7");
    Element_setAttribute(age, "step", "1");
    HTMLInputElement_stepUp(age, js_some_double(2));
    CHECK(STR(HTMLInputElement_get_value(age), "9"));
    HTMLInputElement_stepDown(age, js_none_double());
    CHECK(HTMLInputElement_get_valueAsNumber(age) == 8);
    CHECK(js_same(HTMLInputElement_get_form(user), form));
    CHECK(HTMLFormElement_get_length(form) == 3);

    // A select with options, a textarea.
    HTMLSelectElement color = Document_createElement_select(doc);
    HTMLSelectElement_set_name(color, "color");
    Node_appendChild(form, color);
    const char* values[] = { "red", "green", "blue" };
    for (int i = 0; i < 3; i++) {
        HTMLOptionElement option = Document_createElement_option(doc);
        HTMLOptionElement_set_value(option, values[i]);
        HTMLOptionElement_set_text(option, values[i]);
        HTMLSelectElement_add(color, option, (HTMLElement){ 0 });
    }
    CHECK(HTMLSelectElement_get_length(color) == 3);
    CHECK(HTMLSelectElement_get_selectedIndex(color) == 0);
    CHECK(STR(HTMLSelectElement_get_value(color), "red"));
    HTMLSelectElement_set_selectedIndex(color, 2);
    CHECK(STR(HTMLSelectElement_get_value(color), "blue"));
    HTMLSelectElement_set_value(color, "green");
    CHECK(HTMLSelectElement_get_selectedIndex(color) == 1);
    HTMLOptionElement picked = HTMLSelectElement_item(color, 1);
    CHECK(HTMLOptionElement_get_selected(picked) && HTMLOptionElement_get_index(picked) == 1);
    CHECK(STR(HTMLOptionElement_get_text(picked), "green"));
    CHECK(HTMLCollectionBase_get_length(HTMLSelectElement_get_selectedOptions(color)) == 1);
    CHECK(HTMLOptionsCollection_get_length(HTMLSelectElement_get_options(color)) == 3);
    CHECK(HTMLSelectElement_get_type(color) == HTMLSelectElement_type_select_one);
    HTMLSelectElement_set_value(color, "nope");
    CHECK(HTMLSelectElement_get_selectedIndex(color) == -1);
    HTMLSelectElement_set_selectedIndex(color, 0);

    HTMLTextAreaElement note = Document_createElement_textarea(doc);
    HTMLTextAreaElement_set_name(note, "note");
    HTMLTextAreaElement_set_defaultValue(note, "first");
    Node_appendChild(form, note);
    CHECK(STR(HTMLTextAreaElement_get_value(note), "first"));
    HTMLTextAreaElement_set_value(note, "line one\nline two");
    CHECK(HTMLTextAreaElement_get_textLength(note) == 17);
    CHECK(STR(HTMLTextAreaElement_get_type(note), "textarea"));
    HTMLTextAreaElement_setSelectionRange(note, js_some_double(0), js_some_double(4), SelectionDirection_forward);
    CHECK(HTMLTextAreaElement_get_selectionStart(note) == 0 && HTMLTextAreaElement_get_selectionEnd(note) == 4);
    CHECK(HTMLFormElement_get_length(form) == 5);
    CHECK(HTMLCollectionBase_get_length(HTMLFormElement_get_elements(form)) == 5);

    // FormData gathers the successful controls: the unchecked box is absent.
    // FINDING 3: new FormData(form?, submitter?) has no default macros: FormData_new needs both
    // arguments, null handles spelled out, even for an empty FormData.
    FormData data = FormData_new(form, (HTMLElement){ 0 });
    CHECK(entry_is(data, "user", "ann") && entry_is(data, "age", "8"));
    CHECK(entry_is(data, "agree", "yes") && entry_is(data, "color", "red"));
    CHECK(entry_is(data, "note", "line one\nline two"));
    HTMLInputElement_set_checked(agree, false);
    HTMLInputElement_set_disabled(user, true);
    FormData fewer = FormData_new(form, (HTMLElement){ 0 });
    CHECK(!FormData_has(fewer, "agree") && !FormData_has(fewer, "user") && FormData_has(fewer, "age"));
    HTMLInputElement_set_disabled(user, false);
    HTMLInputElement_set_checked(agree, true);

    // FormData by hand: append keeps both, set replaces them.
    FormData made = FormData_new((HTMLFormElement){ 0 }, (HTMLElement){ 0 });
    FormData_append(made, "tag", "a");
    FormData_append(made, "tag", "b");
    JsArray_FormDataEntryValue all = FormData_getAll(made, "tag");
    CHECK(JsArray_FormDataEntryValue_length(all) == 2);
    CHECK(JsArray_FormDataEntryValue_get(all, 1).tag == FormDataEntryValue_string);
    CHECK(STR(JsArray_FormDataEntryValue_get(all, 1).as.string, "b"));
    FormData_set(made, "tag", "c");
    CHECK(JsArray_FormDataEntryValue_length(FormData_getAll(made, "tag")) == 1 && entry_is(made, "tag", "c"));
    FormData_delete(made, "tag");
    CHECK(!FormData_has(made, "tag") && FormData_get(made, "tag").tag == -1);
    FormData_append(made, "k1", "v1");
    FormData_append(made, "k2", "v2");
    JsIterator it = FormData_iterate(made);
    JsValue pair;
    int pairs = 0;
    while (FormData_iterate_next(it, &pair)) pairs++;
    CHECK(pairs == 2);

    // Validity: required, pattern, range, custom.
    HTMLInputElement code = input(doc, form, "text", "code");
    HTMLInputElement_set_required(code, true);
    HTMLInputElement_set_pattern(code, "[0-9]{3}");
    CHECK(HTMLInputElement_get_willValidate(code));
    CHECK(!HTMLInputElement_checkValidity(code));
    ValidityState validity = HTMLInputElement_get_validity(code);
    CHECK(ValidityState_get_valueMissing(validity) && !ValidityState_get_valid(validity));
    HTMLInputElement_set_value(code, "12");
    CHECK(ValidityState_get_patternMismatch(HTMLInputElement_get_validity(code)));
    CHECK(!ValidityState_get_valueMissing(HTMLInputElement_get_validity(code)));
    CHECK(HTMLInputElement_get_validationMessage(code).h != 0 && JsString_len(HTMLInputElement_get_validationMessage(code)) > 0);
    CHECK(!HTMLFormElement_checkValidity(form));
    HTMLInputElement_set_value(code, "123");
    CHECK(HTMLInputElement_checkValidity(code) && ValidityState_get_valid(HTMLInputElement_get_validity(code)));
    HTMLInputElement_setCustomValidity(code, "taken");
    CHECK(!HTMLInputElement_checkValidity(code) && ValidityState_get_customError(HTMLInputElement_get_validity(code)));
    CHECK(STR(HTMLInputElement_get_validationMessage(code), "taken"));
    HTMLInputElement_setCustomValidity(code, "");
    CHECK(HTMLInputElement_checkValidity(code));
    Element_setAttribute(age, "max", "10");
    HTMLInputElement_set_value(age, "11");
    CHECK(ValidityState_get_rangeOverflow(HTMLInputElement_get_validity(age)));
    Element_setAttribute(age, "min", "5");
    HTMLInputElement_set_value(age, "4");
    CHECK(ValidityState_get_rangeUnderflow(HTMLInputElement_get_validity(age)));
    HTMLInputElement_set_value(age, "8");
    CHECK(HTMLFormElement_checkValidity(form));
    HTMLSelectElement_setCustomValidity(color, "pick another");
    CHECK(!HTMLSelectElement_checkValidity(color) && !HTMLFormElement_checkValidity(form));
    HTMLSelectElement_setCustomValidity(color, "");
    HTMLTextAreaElement_set_required(note, true);
    HTMLTextAreaElement_set_value(note, "");
    CHECK(!HTMLTextAreaElement_checkValidity(note) && ValidityState_get_valueMissing(HTMLTextAreaElement_get_validity(note)));
    HTMLTextAreaElement_set_value(note, "x");

    // requestSubmit validates first, then fires "submit"; a listener cancels navigation.
    EventTarget_addEventListener(form, "submit", EventListenerOrEventListenerObject_of_EventListener(EventListener_from(on_submit, 0)));
    HTMLInputElement_set_value(code, "");
    HTMLFormElement_requestSubmit(form, (HTMLElement){ 0 });
    CHECK(submits == 0);
    HTMLInputElement_set_value(code, "999");
    HTMLFormElement_requestSubmit(form, (HTMLElement){ 0 });
    CHECK(submits == 1);

    // reset puts every control back to its default.
    HTMLInputElement_set_value(user, "bob");
    HTMLInputElement_set_checked(agree, false);
    HTMLSelectElement_set_selectedIndex(color, 2);
    HTMLTextAreaElement_set_value(note, "changed");
    CHECK(STR(HTMLInputElement_get_value(user), "bob"));
    HTMLFormElement_reset(form);
    CHECK(STR(HTMLInputElement_get_value(user), "ann"));
    CHECK(HTMLInputElement_get_checked(agree));
    CHECK(HTMLSelectElement_get_selectedIndex(color) == 0);
    CHECK(STR(HTMLTextAreaElement_get_value(note), "first"));
    CHECK(STR(HTMLInputElement_get_value(code), ""));
}
