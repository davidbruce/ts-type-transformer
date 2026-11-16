// KaTeX from F#, in Node: formulas rendered to HTML and MathML, with options,
// and errors for bad input. Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings
open type Bindings.Globals

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let has (needle: string) (s: string) = s.Contains needle

check (version <> "" && version.Contains ".") "version"
check (katex.version = version) "the default export's version"

// Plain rendering: HTML and MathML together.
let simple = renderToString "x^2"
check (simple.Contains "katex") "the katex class"
check (simple.Contains "katex-html") "HTML output"
check (simple.Contains "<math") "MathML output"
check (simple.Contains "<annotation encoding=\"application/x-tex\">x^2</annotation>") "the TeX source is kept"
check (simple.Contains "msupsub") "a superscript"
check (not (simple.Contains "katex-display")) "inline by default"

// Display mode.
let display = renderToString ("x^2", KatexOptions.Create (displayMode = true))
check (display.Contains "katex-display") "displayMode"
check (display.Contains "display=\"block\"") "displayMode in MathML"

// The output option picks the HTML, the MathML or both.
let onlyHtml = renderToString ("a+b", KatexOptions.Create (output = KatexOptionsOutput.Html))
check (onlyHtml.Contains "katex-html" && not (onlyHtml.Contains "<math")) "output html"
let onlyMath = renderToString ("a+b", KatexOptions.Create (output = KatexOptionsOutput.Mathml))
check (onlyMath.Contains "<math" && not (onlyMath.Contains "katex-html")) "output mathml"
let both = renderToString ("a+b", KatexOptions.Create (output = KatexOptionsOutput.HtmlAndMathml))
check (both.Contains "katex-html" && both.Contains "<math") "output htmlAndMathml"

// Structures.
check ((renderToString @"\frac{a}{b}").Contains "mfrac") "a fraction"
check ((renderToString @"\sqrt{2}").Contains "msqrt") "a square root"
check ((renderToString @"\sum_{i=0}^n i").Contains "∑") "a sum"
check ((renderToString @"\alpha").Contains "α") "a Greek letter"
check ((renderToString "x_1").Contains "mathnormal") "a subscript"
check ((renderToString @"\begin{matrix} a & b \\ c & d \end{matrix}").Contains "mtable") "a matrix"

// Equation numbering on the left.
let leqno = renderToString ("x", KatexOptions.Create (displayMode = true, leqno = true))
check (leqno.Contains "leqno") "leqno"
let fleqn = renderToString ("x", KatexOptions.Create (displayMode = true, fleqn = true))
check (fleqn.Contains "fleqn") "fleqn"

// Bad input throws a ParseError, unless asked not to.
let message =
    try
        renderToString @"\frac{1" |> ignore
        ""
    with e -> e.Message
check (message.StartsWith "KaTeX parse error") "a ParseError is thrown"
check (message.Contains "expected '}'") "its message says what was expected"

let unknown =
    try
        renderToString @"\notacommand" |> ignore
        ""
    with e -> e.Message
check (unknown.Contains "Undefined control sequence: \\notacommand") "an undefined command"

let soft = renderToString (@"\notacommand", KatexOptions.Create (throwOnError = false))
check (soft.Contains "\\notacommand" && soft.Contains "mathcolor") "throwOnError false renders the source in colour"
check (soft.Contains "color:#cc0000") "in the default error colour"
let tinted = renderToString (@"\notacommand", KatexOptions.Create (throwOnError = false, errorColor = "#00ff00"))
check (tinted.Contains "color:#00ff00;") "errorColor"

// Macros: an object of strings.
let macros = createEmpty<JsRecord<string, StringOrObjectOrFn>> // FINDING 1: JsRecord has no constructor
macros.["\\RR"] <- U3.Case1 @"\mathbb{R}"
let withMacro = renderToString (@"\RR", KatexOptions.Create (macros = macros))
check (withMacro.Contains "mathbb") "a macro"
check (withMacro.Contains "ℝ" || withMacro.Contains "R") "its expansion"

// Strict mode: a boolean or a function.
let loose = renderToString ("é", KatexOptions.Create (strict = StringOrBooleanOrStrictFunction.Bool false))
check (loose.Contains "katex") "strict false"
let strictFails =
    try
        renderToString ("é", KatexOptions.Create (strict = StringOrBooleanOrStrictFunction.String "error")) |> ignore
        false
    with _ -> true
check strictFails "strict error throws"

// Trust: a function deciding on each \href.
// FINDING 2: matching the TrustContext the function is given never succeeds.
let trusting = KatexOptions.Create (trust = BooleanOrFn.Function (fun _ -> true))
let link = renderToString (@"\href{https://example.com}{x}", trusting)
check (link.Contains "https://example.com") "a trusted link"
let untrusted = renderToString @"\href{https://example.com}{x}"
check (not (untrusted.Contains "href=\"https")) "untrusted links are not rendered as links"

// maxSize caps the sizes.
let big = renderToString (@"\rule{100em}{1em}", KatexOptions.Create (maxSize = 2.0))
check (big.Contains "width:2em") "maxSize"

printfn "%d checks passed" passed
