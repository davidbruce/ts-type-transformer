// A form from F#: inputs, a select, a textarea; values, validity, FormData and
// reset. Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let same (a: obj) (b: obj) = obj.ReferenceEquals(a, b)

// The form is parsed from markup, then driven through the bindings.
let host: HTMLDivElement = document.createElement HTMLElementTagNameMapKeys.div
host.innerHTML <-
    """<form id="signup" action="/nowhere" method="post">
  <input id="name" name="name" type="text" value="Ada" required>
  <input id="age" name="age" type="number" min="0" max="120" value="36">
  <input id="agree" name="agree" type="checkbox" value="yes">
  <input id="code" name="code" type="text" pattern="[A-Z]{3}[0-9]{2}">
  <select id="color" name="color">
    <option value="r">Red</option>
    <option value="g" selected>Green</option>
    <option value="b">Blue</option>
  </select>
  <textarea id="bio" name="bio">hello</textarea>
</form>"""
document.body.appendChild host |> ignore

let find<'T when 'T :> Element> (selector: string) : 'T = (host.querySelector<'T> selector).Value

let form = find<HTMLFormElement> "form"
let name = find<HTMLInputElement> "#name"
let age = find<HTMLInputElement> "#age"
let agree = find<HTMLInputElement> "#agree"
let code = find<HTMLInputElement> "#code"
let color = find<HTMLSelectElement> "#color"
let bio = find<HTMLTextAreaElement> "#bio"

// The form and its controls.
check (form.length = 6.0) "form.length"
check (form.elements.length = 6.0) "form.elements"
check (form.method = "post") "form.method"
check (form.action.EndsWith "/nowhere") "form.action resolves"
check (same name.form form) "control.form"
check (same (form.Item 0.0) name) "form indexed by number"

// Text and number inputs.
check (name.value = "Ada") "text value from markup"
check (name.``type`` = "text") "input type"
name.value <- "Grace Hopper"
check (name.value = "Grace Hopper") "set text value"
check (name.getAttribute "value" = "Ada") "attribute is the default, not the value"
check (name.defaultValue = "Ada") "defaultValue"
check name.required "required property"
check (age.valueAsNumber = 36.0) "valueAsNumber"
age.valueAsNumber <- 41.0
check (age.value = "41") "valueAsNumber writes value"
age.value <- "abc"
check (age.value = "") "number input rejects text"
age.value <- "36"

// Checkbox.
check (not agree.``checked``) "checkbox starts unchecked"
agree.click ()
check agree.``checked`` "click checks"
agree.``checked`` <- false
check (not agree.``checked``) "checked set"
agree.indeterminate <- true
check agree.indeterminate "indeterminate"
agree.indeterminate <- false

// Select and options.
check (color.value = "g") "select value"
check (color.selectedIndex = 1.0) "selectedIndex from markup"
check (color.options.length = 3.0) "options length"
color.selectedIndex <- 2.0
check (color.value = "b") "selectedIndex sets value"
check (color.options.item(2.0).Value.selected) "option selected"
color.value <- "r"
check (color.selectedIndex = 0.0) "value sets selectedIndex"
check ((color.item 0.0).text = "Red") "option text"
let extra = Option.Create ("Violet", "v")
color.add extra
check (color.options.length = 4.0) "option added"
color.value <- "v"
check (color.selectedIndex = 3.0) "new option selectable"
check (color.selectedOptions.length = 1.0) "selectedOptions"
color.remove 3.0
check (color.options.length = 3.0) "option removed"

// Textarea.
check (bio.value = "hello") "textarea value"
bio.value <- "line one\nline two"
check (bio.textLength = 17.0) "textarea textLength"
check (bio.``type`` = "textarea") "textarea type"
bio.rows <- 5.0
check (bio.rows = 5.0) "textarea rows"

// Validity: required and pattern.
check name.validity.valid "name valid"
name.value <- ""
check name.validity.valueMissing "required: valueMissing"
check (not (name.checkValidity ())) "checkValidity false when missing"
check (name.validationMessage <> "") "validationMessage"
name.value <- "Ada"
check (name.checkValidity ()) "checkValidity true again"
check code.validity.valid "empty optional pattern field is valid"
code.value <- "abc"
check code.validity.patternMismatch "pattern mismatch"
check (not code.validity.valid) "invalid"
code.value <- "ABC12"
check (not code.validity.patternMismatch && code.checkValidity ()) "pattern matches"

// Range validity on the number input.
age.value <- "200"
check age.validity.rangeOverflow "rangeOverflow"
age.value <- "-3"
check age.validity.rangeUnderflow "rangeUnderflow"
age.value <- "36"
check age.validity.valid "age valid"

// A custom error.
code.setCustomValidity "taken"
check code.validity.customError "customError"
check (not (code.checkValidity ())) "custom error blocks checkValidity"
check (code.validationMessage = "taken") "custom message"
code.setCustomValidity ""
check (code.checkValidity ()) "custom error cleared"

// The whole form's validity follows its controls.
check (form.checkValidity ()) "form valid"
name.value <- ""
check (not (form.checkValidity ())) "form invalid with a missing field"
name.value <- "Ada"

// FormData reads what the form holds.
agree.``checked`` <- true
color.value <- "g"
let data = FormData.Create form
let str (v: FormDataEntryValue) =
    match v with
    | FormDataEntryValue.String s -> s
    | FormDataEntryValue.File f -> f.name

check (str (data.get "name") = "Ada") "FormData text"
check (str (data.get "age") = "36") "FormData number"
check (str (data.get "agree") = "yes") "FormData checked box"
check (str (data.get "color") = "g") "FormData select"
check (str (data.get "bio") = "line one\nline two") "FormData textarea"
check (data.has "code") "FormData has an empty text field"
agree.``checked`` <- false
let data2 = FormData.Create form
check (not (data2.has "agree")) "unchecked box is not in FormData"
data2.append ("extra", "1")
data2.append ("extra", "2")
check ((data2.getAll "extra").Count = 2) "FormData getAll"
data2.set ("extra", "3")
check (str (data2.get "extra") = "3" && (data2.getAll "extra").Count = 1) "FormData set"
data2.delete "extra"
check (not (data2.has "extra")) "FormData delete"
check (isNull (box (data2.get "missing"))) "FormData get of a missing name is null"
let keys = [ for k in data2.keys () -> k ]
check (keys.Length = 5 && List.head keys = "name") "FormData keys iterate"

// reset restores what the markup says.
name.value <- "changed"
age.value <- "99"
agree.``checked`` <- true
color.value <- "b"
bio.value <- "changed"
form.reset ()
check (name.value = "Ada") "reset text"
check (age.value = "36") "reset number"
check (not agree.``checked``) "reset checkbox"
check (color.value = "g") "reset select"
check (bio.value = "hello") "reset textarea"

// submit is intercepted: the event fires and can be cancelled.
let mutable submitted = 0
form.addEventListener (
    "submit",
    !^ (fun (e: Event) ->
        submitted <- submitted + 1
        e.preventDefault ())
)
form.requestSubmit ()
check (submitted = 1) "requestSubmit fires submit"
name.value <- ""
form.requestSubmit ()
check (submitted = 1) "invalid form does not submit"

printfn "%d checks passed" passed
