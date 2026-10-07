// Drives the explorer through Chrome DevTools: wheel zoom, drag pan, preset key; screenshots.
import { spawn } from "node:child_process";
import { writeFileSync } from "node:fs";
const [chrome, prof, out] = process.argv.slice(2);
const p = spawn(chrome, ["--headless=new", "--disable-gpu", "--window-size=1280,720", `--user-data-dir=${prof}`, "--remote-debugging-port=9333", "about:blank"]);
const sleep = ms => new Promise(r => setTimeout(r, ms));
let targets;
for (let i = 0; i < 50; i++) { try { targets = await (await fetch("http://127.0.0.1:9333/json")).json(); if (targets.find(t => t.type === "page")) break; } catch {} await sleep(200); }
const ws = new WebSocket(targets.find(t => t.type === "page").webSocketDebuggerUrl);
await new Promise(r => ws.onopen = r);
let id = 0; const pend = new Map();
ws.onmessage = m => { const d = JSON.parse(m.data); if (d.id && pend.has(d.id)) { pend.get(d.id)(d.result); pend.delete(d.id); } };
const cmd = (method, params = {}) => new Promise(r => { const i = ++id; pend.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
const shot = async n => { const r = await cmd("Page.captureScreenshot", { format: "png" }); writeFileSync(`${out}/${n}.png`, Buffer.from(r.data, "base64")); };
const hud = async () => (await cmd("Runtime.evaluate", { expression: "document.getElementById('hud').innerText" })).result.value;
await cmd("Emulation.setDeviceMetricsOverride", { width: 1280, height: 720, deviceScaleFactor: 1, mobile: false });
await cmd("Page.navigate", { url: "http://127.0.0.1:8737/" });
await sleep(2500);
console.log("start:", JSON.stringify(await hud()));
// Zoom in at the seahorse valley gap with 25 wheel notches, quickly.
for (let i = 0; i < 25; i++) { await cmd("Input.dispatchMouseEvent", { type: "mouseWheel", x: 611, y: 326, deltaX: 0, deltaY: -100 }); await sleep(40); }
await shot("a-mid-motion");
console.log("mid-motion:", JSON.stringify(await hud()));
await sleep(3000);
await shot("b-settled");
console.log("settled:", JSON.stringify(await hud()));
// Drag 300 px left.
await cmd("Input.dispatchMouseEvent", { type: "mousePressed", x: 800, y: 400, button: "left", clickCount: 1 });
for (let x = 800; x >= 500; x -= 30) { await cmd("Input.dispatchMouseEvent", { type: "mouseMoved", x, y: 400, button: "left", buttons: 1 }); await sleep(30); }
await cmd("Input.dispatchMouseEvent", { type: "mouseReleased", x: 500, y: 400, button: "left", clickCount: 1 });
await sleep(3000);
await shot("c-panned");
console.log("panned:", JSON.stringify(await hud()));
// Preset 0 (1e-1000), then wheel in a few notches there.
await cmd("Input.dispatchKeyEvent", { type: "keyDown", key: "0", text: "0" });
await sleep(1500);
await shot("d-deep-coarse");
console.log("deep early:", JSON.stringify(await hud()));
for (let i = 0; i < 10; i++) { await cmd("Input.dispatchMouseEvent", { type: "mouseWheel", x: 640, y: 360, deltaX: 0, deltaY: -100 }); await sleep(40); }
await sleep(12000);
await shot("e-deep-later");
console.log("deep later:", JSON.stringify(await hud()));
const err = (await cmd("Runtime.evaluate", { expression: "document.getElementById('err').style.display + ' ' + document.getElementById('err').innerText" })).result.value;
console.log("err box:", JSON.stringify(err));
ws.close(); p.kill();
process.exit(0);
