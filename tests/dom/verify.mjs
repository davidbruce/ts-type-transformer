// Inspects the page after a target's program has run. Independent of the
// bindings: plain DOM calls from JavaScript.
export function verify(lang) {
    const problems = [];
    let checks = 0;
    const expect = (ok, what) => {
        checks++;
        if (!ok) problems.push(what);
    };
    const box = document.getElementById(`made-by-${lang}`);
    if (!box) return { problems: ['the element was never added to the page'], checks: 1 };
    expect(box.parentElement === document.body, 'element is not in <body>');
    expect(box.tagName === 'DIV', 'element is not a <div>');
    expect(box.firstChild?.nodeValue === `hello from ${lang}`, `text is ${JSON.stringify(box.firstChild?.nodeValue)}`);
    expect(box.classList.contains('box'), 'class "box" is missing');
    expect(box.querySelector('p')?.textContent === 'seed text', 'paragraph copied from #seed is missing');
    expect(box.querySelector('button')?.textContent === 'press', 'button is missing');
    expect(box.style.color === 'red', `style.color is ${JSON.stringify(box.style.color)}`);
    expect(document.title === `driven by ${lang}`, `title is ${JSON.stringify(document.title)}`);
    // The click listener wrote this attribute from inside its callback.
    expect(box.getAttribute('data-count') === '2', `data-count is ${box.getAttribute('data-count')} after two clicks`);
    expect(box.getAttribute('data-pings') === '1', `data-pings is ${box.getAttribute('data-pings')} for a once-listener fired twice`);
    // The program removed its click listener: another click must change nothing.
    box.querySelector('button')?.click();
    expect(box.getAttribute('data-count') === '2', 'the click listener was not removed');
    return { problems, checks };
}
