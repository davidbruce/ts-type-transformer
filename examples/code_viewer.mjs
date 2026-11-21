let stylesInjected = false;

export function ensureCodeViewerStyles() {
  if (stylesInjected || document.getElementById('tsrs-code-viewer-styles')) {
    stylesInjected = true;
    return;
  }
  const style = document.createElement('style');
  style.id = 'tsrs-code-viewer-styles';
  style.textContent = `
    .cv-code {
      counter-reset: cv-line;
      white-space: pre;
      tab-size: 4;
      font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
      font-size: 12px;
      line-height: 1.55;
      color: #111111;
      background: #ffffff;
    }
    .cv-line {
      display: inline;
    }
    .cv-line::before {
      counter-increment: cv-line;
      content: counter(cv-line);
      display: inline-block;
      width: calc(var(--cv-gutter-ch, 3) * 1ch + 1.4em);
      padding: 0 0.75em 0 0.5em;
      margin-right: 1em;
      text-align: right;
      color: #8c8c8c;
      background: #f7f7f7;
      border-right: 1px solid #e5e5e5;
      user-select: none;
      -webkit-user-select: none;
    }
    .cv-kw { color: #8b2c1a; font-weight: 600; }
    .cv-ty { color: #1d4f8a; }
    .cv-str { color: #0f6d5c; }
    .cv-num { color: #6b3fa0; }
    .cv-cmt { color: #737373; font-style: italic; }
    .cv-fn { color: #111111; font-weight: 600; }
    .cv-attr { color: #475569; }
    .cv-op { color: #525252; }
  `;
  document.head.appendChild(style);
  stylesInjected = true;
}

export function langFromSource(targetId, sourcePath) {
  const ext = (sourcePath ?? '').split('.').pop()?.toLowerCase();
  if (ext === 'c' || ext === 'h') return 'c';
  if (ext === 'cpp' || ext === 'hpp' || ext === 'cc') return 'cpp';
  if (ext === 'zig') return 'zig';
  if (ext === 'rs') return 'rust';
  if (ext === 'gleam') return 'gleam';
  if (ext === 'fs' || ext === 'fsi' || ext === 'fsx') return 'fsharp';
  if (ext === 'mjs' || ext === 'js' || ext === 'ts') return 'js';
  if (ext === 'html') return 'html';
  return targetId || 'text';
}

const KEYWORDS = {
  c: new Set([
    'auto', 'break', 'case', 'const', 'continue', 'default', 'do', 'else', 'enum',
    'extern', 'for', 'goto', 'if', 'inline', 'register', 'restrict', 'return',
    'sizeof', 'static', 'struct', 'switch', 'typedef', 'union', 'volatile', 'while',
    'true', 'false', 'NULL', '_Alignof', '_Generic', '_Static_assert',
  ]),
  cpp: new Set([
    'alignas', 'alignof', 'and', 'auto', 'break', 'case', 'catch', 'class', 'co_await',
    'co_return', 'co_yield', 'concept', 'const', 'consteval', 'constexpr', 'constinit',
    'continue', 'decltype', 'default', 'delete', 'do', 'else', 'enum', 'explicit',
    'export', 'extern', 'false', 'for', 'friend', 'goto', 'if', 'inline', 'mutable',
    'namespace', 'new', 'noexcept', 'nullptr', 'operator', 'or', 'private', 'protected',
    'public', 'requires', 'return', 'sizeof', 'static', 'static_assert', 'struct',
    'switch', 'template', 'this', 'throw', 'true', 'try', 'typedef', 'typename',
    'union', 'using', 'virtual', 'volatile', 'while',
  ]),
  zig: new Set([
    'addrspace', 'align', 'allowzero', 'and', 'anyframe', 'anytype', 'asm', 'async',
    'await', 'break', 'catch', 'comptime', 'const', 'continue', 'defer', 'else', 'enum',
    'errdefer', 'error', 'export', 'extern', 'false', 'fn', 'for', 'if', 'inline',
    'linksection', 'noalias', 'noinline', 'nosuspend', 'null', 'opaque', 'or', 'orelse',
    'packed', 'pub', 'resume', 'return', 'struct', 'suspend', 'switch', 'test',
    'threadlocal', 'true', 'try', 'undefined', 'union', 'unreachable', 'usingnamespace',
    'var', 'volatile', 'while',
  ]),
  rust: new Set([
    'as', 'async', 'await', 'break', 'const', 'continue', 'crate', 'dyn', 'else', 'enum',
    'extern', 'false', 'fn', 'for', 'if', 'impl', 'in', 'let', 'loop', 'match', 'mod',
    'move', 'mut', 'pub', 'ref', 'return', 'self', 'Self', 'static', 'struct', 'super',
    'trait', 'true', 'type', 'unsafe', 'use', 'where', 'while', 'macro_rules',
  ]),
  gleam: new Set([
    'as', 'assert', 'case', 'const', 'echo', 'fn', 'if', 'import', 'let', 'opaque',
    'panic', 'pub', 'todo', 'type', 'use',
  ]),
  fsharp: new Set([
    'abstract', 'and', 'as', 'assert', 'async', 'base', 'begin', 'class', 'default',
    'delegate', 'do', 'done', 'downcast', 'downto', 'elif', 'else', 'end', 'exception',
    'extern', 'false', 'finally', 'fixed', 'for', 'fun', 'function', 'global', 'if',
    'in', 'inherit', 'inline', 'interface', 'internal', 'lazy', 'let', 'match', 'member',
    'module', 'mutable', 'namespace', 'new', 'not', 'null', 'of', 'open', 'or',
    'override', 'private', 'public', 'rec', 'return', 'select', 'static', 'struct',
    'then', 'to', 'true', 'try', 'type', 'upcast', 'use', 'val', 'void', 'when',
    'while', 'with', 'yield', 'box', 'unbox', 'failwith', 'ignore', 'task',
  ]),
  js: new Set([
    'async', 'await', 'break', 'case', 'catch', 'class', 'const', 'continue', 'debugger',
    'default', 'delete', 'do', 'else', 'export', 'extends', 'false', 'finally', 'for',
    'from', 'function', 'if', 'import', 'in', 'instanceof', 'let', 'new', 'null', 'of',
    'return', 'static', 'super', 'switch', 'this', 'throw', 'true', 'try', 'typeof',
    'undefined', 'var', 'void', 'while', 'with', 'yield',
  ]),
};

const TYPES = {
  c: new Set([
    'void', 'bool', 'char', 'short', 'int', 'long', 'signed', 'unsigned', 'float',
    'double', 'size_t', 'ssize_t', 'uint8_t', 'uint16_t', 'uint32_t', 'uint64_t',
    'int8_t', 'int16_t', 'int32_t', 'int64_t', 'uintptr_t', 'intptr_t',
  ]),
  cpp: new Set([
    'void', 'bool', 'char', 'char8_t', 'char16_t', 'char32_t', 'wchar_t', 'short', 'int',
    'long', 'signed', 'unsigned', 'float', 'double', 'size_t', 'uint8_t', 'uint16_t',
    'uint32_t', 'uint64_t', 'int8_t', 'int16_t', 'int32_t', 'int64_t', 'string',
    'string_view', 'vector', 'array', 'optional', 'variant', 'span', 'shared_ptr',
    'unique_ptr',
  ]),
  zig: new Set([
    'void', 'bool', 'type', 'anyopaque', 'anyerror', 'noreturn', 'usize', 'isize',
    'f16', 'f32', 'f64', 'f80', 'f128', 'u8', 'u16', 'u32', 'u64', 'u128',
    'i8', 'i16', 'i32', 'i64', 'i128', 'c_int', 'c_uint',
  ]),
  rust: new Set([
    'bool', 'char', 'str', 'usize', 'isize', 'f32', 'f64', 'u8', 'u16', 'u32', 'u64',
    'u128', 'i8', 'i16', 'i32', 'i64', 'i128', 'Option', 'Result', 'Vec', 'String',
    'Box', 'Rc', 'RefCell', 'Some', 'None', 'Ok', 'Err',
  ]),
  gleam: new Set([
    'Int', 'Float', 'String', 'Bool', 'Nil', 'List', 'Option', 'Result', 'BitArray',
    'Some', 'None', 'Ok', 'Error', 'True', 'False',
  ]),
  fsharp: new Set([
    'unit', 'int', 'int32', 'int64', 'float', 'float32', 'double', 'decimal', 'string',
    'bool', 'byte', 'sbyte', 'char', 'obj', 'exn', 'nativeint', 'option', 'list', 'array',
    'seq', 'ResizeArray', 'Option', 'Some', 'None', 'Async', 'Promise', 'JS', 'Math',
    'Array', 'List', 'Seq', 'String', 'Map', 'Set',
  ]),
  js: new Set(['Array', 'Map', 'Set', 'Promise', 'Object', 'Math', 'Number', 'String']),
};

const isIdentStart = c => /[A-Za-z_]/.test(c);
const isIdentCont = c => /[A-Za-z0-9_]/.test(c);
const isHexDigit = c => /[0-9a-fA-F_]/.test(c);

function tokenize(source, lang) {
  const kws = KEYWORDS[lang] ?? KEYWORDS.js;
  const tys = TYPES[lang] ?? new Set();
  const tokens = [];
  const n = source.length;
  let i = 0;

  const push = (cls, text) => {
    if (!text) return;
    const prev = tokens[tokens.length - 1];
    if (prev && prev.cls === cls) {
      prev.text += text;
    } else {
      tokens.push({ cls, text });
    }
  };

  while (i < n) {
    const ch = source[i];
    const next = source[i + 1] ?? '';

    // Line comments (`//`) for all 6 languages + JS
    if (ch === '/' && next === '/') {
      // Zig multiline string literals `\\` start with backslash, so `//` is always comment
      let j = i + 2;
      while (j < n && source[j] !== '\n') j++;
      push('cv-cmt', source.slice(i, j));
      i = j;
      continue;
    }

    // Zig multiline string literal `\\...`
    if (lang === 'zig' && ch === '\\' && next === '\\') {
      let j = i + 2;
      while (j < n && source[j] !== '\n') j++;
      push('cv-str', source.slice(i, j));
      i = j;
      continue;
    }

    // Block comments `/* ... */` (nested in Rust)
    if (ch === '/' && next === '*' && lang !== 'fsharp' && lang !== 'gleam') {
      let j = i + 2;
      let depth = 1;
      while (j < n && depth > 0) {
        if (lang === 'rust' && source[j] === '/' && source[j + 1] === '*') {
          depth++;
          j += 2;
        } else if (source[j] === '*' && source[j + 1] === '/') {
          depth--;
          j += 2;
        } else {
          j++;
        }
      }
      push('cv-cmt', source.slice(i, j));
      i = j;
      continue;
    }

    // F# nested block comments `(* ... *)`
    if (lang === 'fsharp' && ch === '(' && next === '*') {
      let j = i + 2;
      let depth = 1;
      while (j < n && depth > 0) {
        if (source[j] === '(' && source[j + 1] === '*') {
          depth++;
          j += 2;
        } else if (source[j] === '*' && source[j + 1] === ')') {
          depth--;
          j += 2;
        } else {
          j++;
        }
      }
      push('cv-cmt', source.slice(i, j));
      i = j;
      continue;
    }

    // C / C++ preprocessor directives `#...` at line start (ignoring horizontal whitespace)
    if ((lang === 'c' || lang === 'cpp') && ch === '#') {
      let k = i - 1;
      while (k >= 0 && (source[k] === ' ' || source[k] === '\t')) k--;
      if (k < 0 || source[k] === '\n') {
        let j = i + 1;
        while (j < n && isIdentCont(source[j])) j++;
        const directive = source.slice(i, j);
        push('cv-attr', directive);
        i = j;
        if (directive === '#include') {
          while (i < n && (source[i] === ' ' || source[i] === '\t')) {
            push(null, source[i++]);
          }
          if (source[i] === '<') {
            let h = i + 1;
            while (h < n && source[h] !== '>' && source[h] !== '\n') h++;
            if (source[h] === '>') h++;
            push('cv-str', source.slice(i, h));
            i = h;
          }
        }
        continue;
      }
    }

    // Rust attributes `#[...]` or `#![...]`
    if (lang === 'rust' && ch === '#' && (next === '[' || (next === '!' && source[i + 2] === '['))) {
      let j = i + (next === '!' ? 3 : 2);
      let depth = 1;
      while (j < n && depth > 0 && source[j] !== '\n') {
        if (source[j] === '[') depth++;
        else if (source[j] === ']') depth--;
        j++;
      }
      push('cv-attr', source.slice(i, j));
      i = j;
      continue;
    }

    // F# attributes `[<...>]`
    if (lang === 'fsharp' && ch === '[' && next === '<') {
      let j = i + 2;
      let depth = 1;
      while (j < n && depth > 0) {
        if (source[j] === '<') depth++;
        else if (source[j] === '>') {
          depth--;
          if (depth === 0 && source[j + 1] === ']') {
            j += 2;
            break;
          }
        }
        j++;
      }
      push('cv-attr', source.slice(i, j));
      i = j;
      continue;
    }

    // Rust raw strings `r#"..."#` / `br#"..."#`
    if (lang === 'rust' && (ch === 'r' || (ch === 'b' && next === 'r'))) {
      let p = ch === 'b' ? i + 2 : i + 1;
      let hashes = 0;
      while (source[p] === '#') {
        hashes++;
        p++;
      }
      if (source[p] === '"' && (hashes > 0 || p > i + 1 || source[i - 1] === ' ' || !isIdentCont(source[i - 1] ?? ' '))) {
        let j = p + 1;
        while (j < n) {
          if (source[j] === '"') {
            let h = 0;
            while (h < hashes && source[j + 1 + h] === '#') h++;
            if (h === hashes) {
              j += 1 + hashes;
              break;
            }
          }
          j++;
        }
        push('cv-str', source.slice(i, j));
        i = j;
        continue;
      }
    }

    // F# triple-quoted strings `"""..."""`
    if (lang === 'fsharp' && ch === '"' && next === '"' && source[i + 2] === '"') {
      let j = i + 3;
      while (j < n && !(source[j] === '"' && source[j + 1] === '"' && source[j + 2] === '"')) {
        j++;
      }
      if (j < n) j += 3;
      push('cv-str', source.slice(i, j));
      i = j;
      continue;
    }

    // F# interpolated/verbatim prefixes `$"..."` / `@"..."`
    if (lang === 'fsharp' && (ch === '$' || ch === '@') && next === '"') {
      const verbatim = ch === '@';
      let j = i + 2;
      while (j < n) {
        if (verbatim) {
          if (source[j] === '"' && source[j + 1] === '"') {
            j += 2;
            continue;
          }
          if (source[j] === '"') {
            j++;
            break;
          }
        } else {
          if (source[j] === '\\') {
            j += 2;
            continue;
          }
          if (source[j] === '"') {
            j++;
            break;
          }
        }
        if (source[j] === '\n') break;
        j++;
      }
      push('cv-str', source.slice(i, j));
      i = j;
      continue;
    }

    // Regular double-quoted string `"..."` (and Zig `@"..."` / Rust `b"..."`)
    if (ch === '"') {
      let j = i + 1;
      while (j < n && source[j] !== '\n') {
        if (source[j] === '\\') {
          j += 2;
          continue;
        }
        if (source[j] === '"') {
          j++;
          break;
        }
        j++;
      }
      push('cv-str', source.slice(i, j));
      i = j;
      continue;
    }

    // Backtick identifiers in F# ``foo`` or template literals in JS
    if (lang === 'fsharp' && ch === '`' && next === '`') {
      let j = i + 2;
      while (j < n && source[j] !== '\n' && !(source[j] === '`' && source[j + 1] === '`')) j++;
      if (j + 1 < n && source[j] === '`' && source[j + 1] === '`') j += 2;
      push(null, source.slice(i, j));
      i = j;
      continue;
    }

    // Single quotes: char literals vs Rust lifetimes vs F# type parameters
    if (ch === '\'') {
      if ((lang === 'fsharp' || lang === 'rust') && isIdentStart(next)) {
        let j = i + 2;
        while (j < n && isIdentCont(source[j])) j++;
        if (source[j] === '\'') {
          // Char literal `'a'`
          push('cv-str', source.slice(i, j + 1));
          i = j + 1;
          continue;
        }
        push(lang === 'fsharp' ? 'cv-ty' : 'cv-attr', source.slice(i, j));
        i = j;
        continue;
      }
      if (lang === 'c' || lang === 'cpp' || lang === 'zig' || lang === 'rust' || lang === 'fsharp') {
        let j = i + 1;
        while (j < n && source[j] !== '\n') {
          if (source[j] === '\\') {
            j += 2;
            continue;
          }
          if (source[j] === '\'') {
            j++;
            break;
          }
          j++;
        }
        push('cv-str', source.slice(i, j));
        i = j;
        continue;
      }
    }

    // Zig / Gleam `@ident` builtins & attributes
    if ((lang === 'zig' || lang === 'gleam' || lang === 'cpp') && ch === '@' && isIdentStart(next)) {
      let j = i + 2;
      while (j < n && isIdentCont(source[j])) j++;
      push('cv-attr', source.slice(i, j));
      i = j;
      continue;
    }

    // C++ `[[attr]]`
    if (lang === 'cpp' && ch === '[' && next === '[') {
      let j = i + 2;
      while (j < n && !(source[j] === ']' && source[j + 1] === ']')) j++;
      if (j + 1 < n) j += 2;
      push('cv-attr', source.slice(i, j));
      i = j;
      continue;
    }

    // Numbers
    if (/[0-9]/.test(ch)) {
      let j = i;
      if (ch === '0' && (next === 'x' || next === 'X' || next === 'b' || next === 'B' || next === 'o' || next === 'O')) {
        j += 2;
        while (j < n && isHexDigit(source[j])) j++;
      } else {
        while (j < n && /[0-9_]/.test(source[j])) j++;
        if (source[j] === '.' && /[0-9]/.test(source[j + 1] ?? '')) {
          j++;
          while (j < n && /[0-9_]/.test(source[j])) j++;
        }
        if ((source[j] === 'e' || source[j] === 'E') && /[0-9+\-]/.test(source[j + 1] ?? '')) {
          j++;
          if (source[j] === '+' || source[j] === '-') j++;
          while (j < n && /[0-9_]/.test(source[j])) j++;
        }
      }
      // Numeric type suffix like 0.0f, 1u32, 2usize
      while (j < n && /[A-Za-z0-9_]/.test(source[j])) j++;
      push('cv-num', source.slice(i, j));
      i = j;
      continue;
    }

    // Identifiers, keywords, types, macros, function calls
    if (isIdentStart(ch)) {
      let j = i + 1;
      while (j < n && isIdentCont(source[j])) j++;
      const word = source.slice(i, j);
      if (lang === 'rust' && source[j] === '!') {
        push('cv-attr', source.slice(i, j + 1));
        i = j + 1;
        continue;
      }
      if ((lang === 'c' || lang === 'cpp') && (word === '__attribute__' || word === 'TS_EXPORT' || word === 'TS_IMPORT_MOD')) {
        push('cv-attr', word);
      } else if (kws.has(word)) {
        push('cv-kw', word);
      } else if (tys.has(word)) {
        push('cv-ty', word);
      } else {
        let k = j;
        while (k < n && (source[k] === ' ' || source[k] === '\t')) k++;
        if (source[k] === '(' && (!/^[A-Z]/.test(word) || word.includes('_'))) {
          push('cv-fn', word);
        } else if (/^[A-Z][A-Za-z0-9_]*$/.test(word) && /[a-z]/.test(word)) {
          push('cv-ty', word);
        } else {
          push(null, word);
        }
      }
      i = j;
      continue;
    }

    // Punctuation / operators
    if ('{}[]().,:;+-*/%&|^~!<>?='.includes(ch)) {
      push('cv-op', ch);
      i++;
      continue;
    }

    // Whitespace / other characters
    let j = i + 1;
    while (j < n && !isIdentStart(source[j]) && !/[0-9]/.test(source[j]) && !'{}[]().,:;+-*/%&|^~!<>?="\'\`/\\#@['.includes(source[j])) {
      j++;
    }
    push(null, source.slice(i, j));
    i = j;
  }

  return tokens;
}

export function renderCode(container, source, lang = 'text') {
  ensureCodeViewerStyles();
  const normalized = String(source ?? '');
  const tokens = tokenize(normalized, lang);

  // Split token stream into lines while preserving exact verbatim textContent.
  const lines = [[]];
  for (const tok of tokens) {
    if (!tok.text.includes('\n')) {
      lines[lines.length - 1].push(tok);
      continue;
    }
    const parts = tok.text.split('\n');
    for (let idx = 0; idx < parts.length; idx++) {
      if (idx > 0) lines.push([]);
      if (parts[idx].length > 0) {
        lines[lines.length - 1].push({ cls: tok.cls, text: parts[idx] });
      }
    }
  }

  const endsWithNewline = normalized.endsWith('\n') && lines.length > 1;
  const visibleLines = endsWithNewline ? lines.slice(0, -1) : lines;
  const gutterCh = Math.max(2, String(visibleLines.length).length);

  container.classList.add('cv-code');
  container.style.setProperty('--cv-gutter-ch', String(gutterCh));
  container.dataset.rawSource = normalized;
  container.dataset.lang = lang;

  const frag = document.createDocumentFragment();
  for (let i = 0; i < visibleLines.length; i++) {
    const lineSpan = document.createElement('span');
    lineSpan.className = 'cv-line';
    for (const seg of visibleLines[i]) {
      if (seg.cls) {
        const s = document.createElement('span');
        s.className = seg.cls;
        s.textContent = seg.text;
        lineSpan.appendChild(s);
      } else {
        lineSpan.appendChild(document.createTextNode(seg.text));
      }
    }
    frag.appendChild(lineSpan);
    if (i < visibleLines.length - 1 || endsWithNewline) {
      frag.appendChild(document.createTextNode('\n'));
    }
  }

  container.replaceChildren(frag);
}
