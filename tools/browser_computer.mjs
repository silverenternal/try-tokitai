import { spawn } from "node:child_process";
import { mkdir } from "node:fs/promises";
import process from "node:process";

const input = JSON.parse(await new Promise((resolve, reject) => {
  let value = "";
  process.stdin.setEncoding("utf8");
  process.stdin.on("data", (chunk) => { value += chunk; });
  process.stdin.on("end", () => resolve(value));
  process.stdin.on("error", reject);
}));

const port = Number(input.port || 9333);
const endpoint = `http://127.0.0.1:${port}`;
const edge = input.edge_path || String.raw`C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe`;

async function waitForEndpoint() {
  for (let attempt = 0; attempt < 30; attempt += 1) {
    try {
      const response = await fetch(`${endpoint}/json/version`);
      if (response.ok) return;
    } catch {}
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  throw new Error("Edge computer-use session did not become ready");
}

async function ensureBrowser() {
  try {
    const response = await fetch(`${endpoint}/json/version`);
    if (response.ok) return;
  } catch {}
  await mkdir(input.profile_dir, { recursive: true });
  const child = spawn(edge, [
    `--remote-debugging-port=${port}`,
    `--user-data-dir=${input.profile_dir}`,
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-features=msEdgeSidebarV2",
    input.url || "about:blank",
  ], { detached: true, stdio: "ignore", windowsHide: false });
  child.unref();
  await waitForEndpoint();
}

async function target() {
  const response = await fetch(`${endpoint}/json/list`);
  const targets = await response.json();
  const pages = targets.filter((item) => item.type === "page" && item.webSocketDebuggerUrl);
  if (!pages.length) throw new Error("No controllable Edge page is open");
  return pages.find((item) => !String(item.url || "").startsWith("edge://")) || pages[0];
}

async function withCdp(callback) {
  const page = await target();
  const socket = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });
  let sequence = 0;
  const pending = new Map();
  socket.addEventListener("message", (event) => {
    const message = JSON.parse(String(event.data));
    const waiter = pending.get(message.id);
    if (!waiter) return;
    pending.delete(message.id);
    if (message.error) waiter.reject(new Error(message.error.message));
    else waiter.resolve(message.result || {});
  });
  const send = (method, params = {}) => new Promise((resolve, reject) => {
    const id = ++sequence;
    pending.set(id, { resolve, reject });
    socket.send(JSON.stringify({ id, method, params }));
  });
  try {
    return await callback(send, page);
  } finally {
    socket.close();
  }
}

const inspectExpression = `(() => {
  const selector = 'a[href],button,input,textarea,select,[role="button"],[role="link"],[contenteditable="true"],summary';
  const visible = (element) => {
    const style = getComputedStyle(element);
    const rect = element.getBoundingClientRect();
    return style.visibility !== 'hidden' && style.display !== 'none' && rect.width > 0 && rect.height > 0;
  };
  const elements = [...document.querySelectorAll(selector)].filter(visible).slice(0, 120);
  window.__tokitaiComputerRefs = elements;
  return {
    url: location.href,
    title: document.title,
    text: (document.body?.innerText || '').replace(/\\s+/g, ' ').trim().slice(0, 12000),
    elements: elements.map((element, ref) => ({
      ref,
      tag: element.tagName.toLowerCase(),
      role: element.getAttribute('role') || '',
      label: (element.getAttribute('aria-label') || element.innerText || element.value || element.title || '').replace(/\\s+/g, ' ').trim().slice(0, 180),
      href: element.href || '',
      type: element.type || ''
    }))
  };
})()`;

async function evaluate(send, expression) {
  const result = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  if (result.exceptionDetails) throw new Error(result.exceptionDetails.text || "page evaluation failed");
  return result.result?.value;
}

await ensureBrowser();
const output = await withCdp(async (send) => {
  await send("Runtime.enable");
  await send("Page.enable");
  const action = String(input.action || "inspect");
  if (action === "navigate") {
    const url = new URL(String(input.url || ""));
    if (!/^https?:$/.test(url.protocol)) throw new Error("Only HTTP/HTTPS navigation is allowed");
    await send("Page.navigate", { url: url.toString() });
    await new Promise((resolve) => setTimeout(resolve, Number(input.wait_ms || 1200)));
  } else if (action === "click") {
    const ref = Number(input.ref);
    const clicked = await evaluate(send, `(() => { const e = window.__tokitaiComputerRefs?.[${ref}]; if (!e) return false; e.scrollIntoView({block:'center'}); e.click(); return true; })()`);
    if (!clicked) throw new Error("Element reference is stale; inspect the page again");
    await new Promise((resolve) => setTimeout(resolve, Number(input.wait_ms || 700)));
  } else if (action === "type") {
    const ref = Number(input.ref);
    const text = JSON.stringify(String(input.text || ""));
    const typed = await evaluate(send, `(() => { const e = window.__tokitaiComputerRefs?.[${ref}]; if (!e) return false; e.focus(); if ('value' in e) { e.value = ${text}; e.dispatchEvent(new Event('input',{bubbles:true})); e.dispatchEvent(new Event('change',{bubbles:true})); } else { e.textContent = ${text}; e.dispatchEvent(new Event('input',{bubbles:true})); } return true; })()`);
    if (!typed) throw new Error("Element reference is stale; inspect the page again");
  } else if (action === "key") {
    await send("Input.dispatchKeyEvent", { type: "keyDown", key: String(input.key || "Enter") });
    await send("Input.dispatchKeyEvent", { type: "keyUp", key: String(input.key || "Enter") });
    await new Promise((resolve) => setTimeout(resolve, Number(input.wait_ms || 500)));
  } else if (action === "scroll") {
    await evaluate(send, `window.scrollBy(${Number(input.x || 0)}, ${Number(input.y || 600)})`);
  } else if (action === "screenshot") {
    const capture = await send("Page.captureScreenshot", { format: "png", captureBeyondViewport: false });
    return { action, url: (await target()).url, screenshot_base64: capture.data };
  } else if (action !== "inspect") {
    throw new Error(`Unsupported computer-use action: ${action}`);
  }
  return { action, ...(await evaluate(send, inspectExpression)) };
});

process.stdout.write(JSON.stringify({ operation: "browser_computer", success: true, ...output }));
