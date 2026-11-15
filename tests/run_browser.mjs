// node run_browser.mjs <dir> [<dir>...]
// Serves the directories, opens /index.html in headless Chrome, and waits for
// the page to set `window.__result = { passed, failed, dom }`.
// Exits 0 on success, 1 on failure, 77 if no browser is installed.
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';

const roots = process.argv.slice(2).map(r => path.resolve(r));
const candidates = [
    process.env.CHROME,
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    '/Applications/Chromium.app/Contents/MacOS/Chromium',
    '/usr/bin/google-chrome',
    '/usr/bin/chromium',
    '/usr/bin/chromium-browser',
];
const chromePath = candidates.find(p => p && fs.existsSync(p));
if (!chromePath) {
    console.error('SKIPPED: no Chrome or Chromium found (set CHROME to its path)');
    process.exit(77);
}

const missing = [];
const types = { '.html': 'text/html', '.mjs': 'text/javascript', '.js': 'text/javascript', '.wasm': 'application/wasm' };
const server = http.createServer((req, res) => {
    const name = decodeURIComponent(new URL(req.url, 'http://x').pathname);
    // Only files inside one of the roots.
    const file = roots.map(r => [r, path.join(r, name)]).find(([r, f]) => f.startsWith(r + path.sep) && fs.existsSync(f) && fs.statSync(f).isFile())?.[1];
    if (!file) {
        // The browser asks for an icon by itself.
        if (name !== '/favicon.ico') missing.push(name);
        return res.writeHead(404).end();
    }
    res.writeHead(200, { 'content-type': types[path.extname(file)] ?? 'application/octet-stream' });
    fs.createReadStream(file).pipe(res);
});
await new Promise(done => server.listen(0, '127.0.0.1', done));
const url = `http://127.0.0.1:${server.address().port}/index.html`;

const profile = fs.mkdtempSync(path.join(os.tmpdir(), 'ts-dom-test-'));
const chrome = spawn(chromePath, ['--headless=new', '--disable-gpu', '--no-first-run', '--no-default-browser-check', '--remote-debugging-port=0', `--user-data-dir=${profile}`, 'about:blank']);
let finished = false;
const finish = code => {
    if (finished) return;
    finished = true;
    chrome.kill();
    server.close();
    setTimeout(() => {
        fs.rmSync(profile, { recursive: true, force: true });
        process.exit(code);
    }, 200);
};
const abort = msg => {
    if (!finished) {
        console.error(msg);
        finish(1);
    }
    return new Promise(() => {});
};

// Chrome prints the address of its DevTools endpoint on stderr.
let stderrText = '';
const endpoint = await new Promise((found, failed) => {
    chrome.stderr.on('data', chunk => {
        stderrText += chunk;
        const m = stderrText.match(/DevTools listening on (ws:\/\/\S+)/);
        if (m) found(m[1]);
    });
    chrome.on('error', err => failed(err));
    chrome.on('exit', (code, sig) => failed(new Error(`Chrome exited early (${code ?? sig}):\n${stderrText}`)));
    setTimeout(() => failed(new Error('Chrome did not start')), 20000);
}).catch(err => abort(err.message));

chrome.on('exit', (code, sig) => abort(`Chrome exited unexpectedly (${code ?? sig}):\n${stderrText}`));

const socket = new WebSocket(endpoint);
await new Promise((open, failed) => {
    socket.onopen = open;
    socket.onerror = () => failed(new Error('DevTools WebSocket error'));
    socket.onclose = () => failed(new Error('DevTools WebSocket closed'));
    setTimeout(() => failed(new Error('DevTools WebSocket timed out')), 10000);
}).catch(err => abort(err.message));
let nextId = 0;
const pending = new Map();
const errors = [];
socket.onmessage = ({ data }) => {
    const msg = JSON.parse(data);
    if (msg.id !== undefined) pending.get(msg.id)?.(msg);
    if (msg.method === 'Runtime.exceptionThrown') errors.push(msg.params.exceptionDetails.exception?.description ?? msg.params.exceptionDetails.text);
    if (msg.method === 'Runtime.consoleAPICalled' && msg.params.type === 'error') errors.push(msg.params.args.map(a => a.value ?? a.description).join(' '));
};
socket.onclose = () => abort('DevTools WebSocket closed');
const send = (method, params = {}, sessionId) =>
    new Promise((ok, fail) => {
        const id = ++nextId;
        const timer = setTimeout(() => {
            pending.delete(id);
            fail(new Error(`${method} timed out`));
        }, 15000);
        pending.set(id, msg => {
            clearTimeout(timer);
            pending.delete(id);
            msg.error ? fail(new Error(`${method}: ${msg.error.message}`)) : ok(msg.result);
        });
        socket.send(JSON.stringify({ id, method, params, sessionId }));
    });

const { targetId } = await send('Target.createTarget', { url: 'about:blank' });
const { sessionId } = await send('Target.attachToTarget', { targetId, flatten: true });
await send('Runtime.enable', {}, sessionId);
await send('Page.navigate', { url }, sessionId);

const deadline = Date.now() + 60000;
let result = null;
while (!result && Date.now() < deadline && errors.length === 0 && missing.length === 0) {
    await new Promise(r => setTimeout(r, 100));
    const { result: value } = await send('Runtime.evaluate', { expression: 'JSON.stringify(window.__result ?? null)', returnByValue: true }, sessionId);
    result = JSON.parse(value.value ?? 'null');
}
const { result: agent } = await send('Runtime.evaluate', { expression: 'navigator.userAgent', returnByValue: true }, sessionId);
const browser = agent.value.match(/Chrome\/[\d.]+/)?.[0] ?? agent.value;

if (!result) {
    const why = [...missing.map(m => `not found: ${m}`), ...errors];
    console.error(`no result from the page in ${browser}${why.length ? ':\n  ' + why.join('\n  ') : ' (timed out)'}`);
    finish(1);
} else {
    const problems = [...missing.map(m => `not found: ${m}`), ...result.failed.map(f => `check failed: ${f}`), ...result.dom.map(d => `page is wrong: ${d}`), ...errors.map(e => `error: ${e}`)];
    console.log(`${browser}: ${result.passed} checks passed in the program, ${result.domChecks} on the page`);
    if (problems.length || result.passed === 0) {
        console.error(problems.join('\n') || 'no checks ran');
        finish(1);
    } else finish(0);
}
