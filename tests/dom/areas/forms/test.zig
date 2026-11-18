// A form from Zig: controls read and written, validated, collected into
// FormData and reset. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

// The text of a FormData entry that is a string, or null.
fn entry(data: b.FormData, name: []const u8, buf: []u8) ?[]const u8 {
    const v = data.get(name) orelse return null;
    return switch (v) {
        .string => |s| s.copy(buf),
        .File => null,
    };
}

fn isEntry(data: b.FormData, name: []const u8, want: []const u8) bool {
    var buf: [64]u8 = undefined;
    const got = entry(data, name, &buf) orelse return false;
    return std.mem.eql(u8, got, want);
}

export fn run() void {
    const doc = b.getDocument();
    const body = doc.getBody();

    // A form built from markup.
    const host = doc.createElement(.div, .{}).asHTMLElement();
    host.setInnerHTML(
        \\<form id="f" action="/submit" method="post">
        \\  <input id="user" name="user" type="text" value="ada" required pattern="[a-z]+">
        \\  <input id="age" name="age" type="number" min="1" max="120" value="36">
        \\  <input id="agree" name="agree" type="checkbox" value="yes">
        \\  <select id="color" name="color">
        \\    <option value="r">Red</option>
        \\    <option value="g" selected>Green</option>
        \\    <option value="b">Blue</option>
        \\  </select>
        \\  <textarea id="note" name="note">hello</textarea>
        \\</form>
    );
    _ = body.appendChild(host.asNode());
    const form = b.HTMLFormElement{ .h = doc.getElementById("f").?.h };
    const user = b.HTMLInputElement{ .h = doc.getElementById("user").?.h };
    const age = b.HTMLInputElement{ .h = doc.getElementById("age").?.h };
    const agree = b.HTMLInputElement{ .h = doc.getElementById("agree").?.h };
    const color = b.HTMLSelectElement{ .h = doc.getElementById("color").?.h };
    const note = b.HTMLTextAreaElement{ .h = doc.getElementById("note").?.h };

    // The form and its parts.
    expect(form.getElements().getLength() == 5, @src());
    expect(form.getLength() == 5, @src());
    expect(form.getMethod().eql("post"), @src());
    expect(form.getAction().len() > 0, @src());
    expect(b.jsSame(user.getForm().?, form), @src());
    expect(user.getType().eql("text") and age.getType().eql("number") and agree.getType().eql("checkbox"), @src());
    expect(user.getName().eql("user"), @src());

    // Text: value read, written, and different from the default once written.
    expect(user.getValue().eql("ada") and user.getDefaultValue().eql("ada"), @src());
    user.setValue("grace");
    expect(user.getValue().eql("grace") and user.getDefaultValue().eql("ada"), @src());
    expect(user.getRequired(), @src());
    expect(user.getPattern().eql("[a-z]+"), @src());

    // Numbers.
    expect(age.getValueAsNumber() == 36, @src());
    age.setValueAsNumber(41);
    expect(age.getValue().eql("41"), @src());
    age.stepUp(null);
    expect(age.getValueAsNumber() == 42, @src());
    expect(age.getMin().eql("1") and age.getMax().eql("120"), @src());

    // A checkbox.
    expect(!agree.getChecked(), @src());
    agree.click();
    expect(agree.getChecked(), @src());
    agree.setChecked(false);
    expect(!agree.getChecked() and !agree.getDefaultChecked(), @src());

    // A select and its options.
    expect(color.getOptions().getLength() == 3 and color.getLength() == 3, @src());
    expect(color.getSelectedIndex() == 1 and color.getValue().eql("g"), @src());
    color.setSelectedIndex(2);
    expect(color.getValue().eql("b"), @src());
    expect(color.item(2).?.getSelected() and !color.item(1).?.getSelected(), @src());
    expect(color.item(0).?.getText().eql("Red") and color.item(0).?.getValue().eql("r"), @src());
    expect(color.item(2).?.getIndex() == 2, @src());
    color.setValue("r");
    expect(color.getSelectedIndex() == 0, @src());
    expect(color.getSelectedOptions().getLength() == 1, @src());
    expect(!color.getMultiple(), @src());

    // Options made via the global Option constructor (and Image/Audio constructors).
    const extra = b.Option.new(.{ .text = "Yellow", .value = "y" });
    color.add(.{ .HTMLOptionElement = extra }, null);
    expect(color.getLength() == 4, @src());
    expect(color.item(3).?.getText().eql("Yellow") and color.item(3).?.getValue().eql("y"), @src());
    const selected_opt = b.Option.new(.{ .text = "Purple", .value = "p", .defaultSelected = true, .selected = true });
    expect(selected_opt.getText().eql("Purple") and selected_opt.getValue().eql("p") and selected_opt.getDefaultSelected() and selected_opt.getSelected(), @src());
    const img = b.Image.new(.{ .width = 16, .height = 8 });
    expect(img.getWidth() == 16 and img.getHeight() == 8, @src());
    const audio = b.Audio.new(null);
    expect(audio.getTagName().eql("AUDIO"), @src());

    // A textarea.
    expect(note.getValue().eql("hello") and note.getTextLength() == 5, @src());
    note.setValue("hello\nworld");
    expect(note.getValue().eql("hello\nworld"), @src());

    // FormData collects the named controls that count: an unchecked box does not.
    const data = b.FormData.new(form, null);
    expect(isEntry(data, "user", "grace"), @src());
    expect(isEntry(data, "age", "42"), @src());
    expect(isEntry(data, "color", "r"), @src());
    expect(isEntry(data, "note", "hello\nworld"), @src());
    expect(!data.has("agree"), @src());
    agree.setChecked(true);
    const again = b.FormData.new(form, null);
    expect(isEntry(again, "agree", "yes"), @src());

    // FormData built by hand.
    const made = b.FormData.new(null, null);
    made.appendWithString("tag", "x");
    made.appendWithString("tag", "y");
    made.setWithString("one", "1");
    expect(made.has("tag") and made.getAll("tag").len() == 2, @src());
    expect(isEntry(made, "one", "1"), @src());
    made.setWithString("one", "2");
    expect(isEntry(made, "one", "2") and made.getAll("one").len() == 1, @src());
    made.delete("tag");
    expect(!made.has("tag") and made.get("tag") == null, @src());

    // Validity: pattern, required, range, a custom message.
    expect(user.checkValidity() and user.getValidity().getValid(), @src());
    user.setValue("123");
    expect(!user.checkValidity(), @src());
    expect(user.getValidity().getPatternMismatch() and !user.getValidity().getValid(), @src());
    expect(user.getValidationMessage().len() > 0, @src());
    user.setValue("");
    expect(user.getValidity().getValueMissing() and !user.checkValidity(), @src());
    user.setValue("ada");
    expect(user.checkValidity(), @src());
    age.setValue("500");
    expect(age.getValidity().getRangeOverflow() and !age.checkValidity(), @src());
    age.setValue("0");
    expect(age.getValidity().getRangeUnderflow(), @src());
    age.setValue("36");
    expect(age.checkValidity(), @src());
    expect(form.checkValidity(), @src());
    user.setCustomValidity("taken");
    expect(!user.checkValidity() and user.getValidity().getCustomError(), @src());
    expect(user.getValidationMessage().eql("taken"), @src());
    expect(!form.checkValidity(), @src());
    user.setCustomValidity("");
    expect(user.checkValidity() and form.checkValidity(), @src());
    expect(user.getWillValidate(), @src());

    // Reset puts every control back to its default.
    user.setValue("changed");
    agree.setChecked(true);
    color.setSelectedIndex(2);
    note.setValue("other");
    form.reset();
    expect(user.getValue().eql("ada"), @src());
    expect(!agree.getChecked(), @src());
    expect(color.getSelectedIndex() == 1, @src());
    expect(note.getValue().eql("hello"), @src());
    expect(age.getValue().eql("36"), @src());
}
