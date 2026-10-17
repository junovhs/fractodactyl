"use strict";
// Fractodactyl Explorer V2: the stone-and-parchment mockup controls the real
// fd explore engine through its same-origin embedded canvas.
(function () {
  const $ = s => document.querySelector(s);
  const $$ = s => Array.from(document.querySelectorAll(s));
  const frame = $("#live-view");
  const bar = $("#bar"), status = $("#status");
  const places = [
    { index: 1, look: "ice" },    // Master Ice / Seahorse Valley
    { index: 5, look: "ice" },    // Deep Blue
    { index: 4, look: "smoke" },  // Stone Spiral
    { index: 7, look: "ice" },    // Icy Caverns
    { index: 3, look: "steel" },  // Midnight Fossil
    { index: 2, look: "ice" },    // Ocean Trench
    { index: 1, look: "coral" },  // Coral
    { index: 8, look: "steel" },  // Steel
    { index: 0, look: "zebra" }   // Zebra
  ];
  let ready = false, initialized = false, snapshot = null, menuBox = null;
  const pending = [];
  function tell(cmd, extra) {
    const message = Object.assign({ source: "fd-v2-ui", cmd: cmd }, extra || {});
    if (!ready) pending.push(message);
    else frame.contentWindow.postMessage(message, location.origin);
  }
  function notice(text, error) {
    status.textContent = text;
    status.style.color = error ? "#8c122a" : "";
  }
  function choose(n) {
    if (n === 9) n = Math.floor(Math.random() * places.length);
    const p = places[n];
    if (!p) return;
    $$("#presets li").forEach((li, i) => {
      li.classList.toggle("sel", i === n);
      li.setAttribute("aria-selected", i === n ? "true" : "false");
    });
    tell("jump", { index: p.index });
    tell("look", { name: p.look });
  }
  function zoom(n) { tell("zoom", { delta: n > 0 ? -Math.log10(2) : Math.log10(2) }); }
  function pan(x, y) { tell("pan", { x: x, y: y }); }
  function save() {
    const value = prompt("Save this location as a place (letters, digits, - or _):", "my_discovery");
    if (value !== null) tell("save", { name: value.trim() });
  }
  function screenshot() { tell("snapshot"); }
  function copyCoords() {
    if (!snapshot) return;
    const coords = snapshot.re + " " + snapshot.im + " " + snapshot.width;
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(coords).then(() => notice("Copied exact coordinates")).catch(() => prompt("Copy coordinates:", coords));
    } else prompt("Copy coordinates:", coords);
  }
  function gotoCoords() {
    const re = prompt("Real coordinate:", snapshot ? snapshot.re : "-0.743643887");
    if (re === null) return;
    const im = prompt("Imaginary coordinate:", snapshot ? snapshot.im : "0.131825904");
    if (im === null) return;
    const width = prompt("Horizontal width (e.g. 1e-12):", snapshot ? snapshot.width : "0.0065");
    if (width === null) return;
    tell("jump-place", { name: "Coordinates", re: re.trim(), im: im.trim(), width: width.trim(), iter: 5000 });
  }
  function closeMenu() {
    if (menuBox) { menuBox.remove(); menuBox = null; }
  }
  function menu(anchor, items) {
    closeMenu();
    const box = document.createElement("div");
    box.id = "live-menu";
    items.forEach(item => {
      const b = document.createElement("button");
      b.type = "button";
      b.textContent = item[0];
      b.onclick = () => { closeMenu(); item[1](); };
      box.append(b);
    });
    document.body.append(box);
    const r = anchor.getBoundingClientRect();
    const maxLeft = Math.max(4, innerWidth - box.offsetWidth - 8);
    box.style.left = Math.max(4, Math.min(maxLeft, r.left)) + "px";
    box.style.top = Math.min(innerHeight - box.offsetHeight - 8, r.bottom + 3) + "px";
    menuBox = box;
  }
  async function fossils(anchor) {
    try {
      const r = await fetch("/places");
      if (!r.ok) throw Error(await r.text());
      const rows = (await r.text()).split("\n").filter(Boolean).map(row => row.split("\t"));
      if (!rows.length) { notice("No saved fossils yet. Use Nest to save a discovery."); return; }
      menu(anchor, rows.map(row => [row[0] + "  ·  " + row[3], () => {
        tell("jump-place", { name: row[0], re: row[1], im: row[2], width: row[3], iter: Number(row[4]) || 5000 });
      }]));
    } catch (err) { notice(String(err), true); }
  }
  function showHelp() {
    alert("Fractodactyl Explorer V2\n\nDrag the fractal to pan; wheel over it to zoom at the cursor. The arrow pad moves the camera and + / - zoom in or out. Presets change real locations and studio looks. Fossils opens saved places; Nest saves your current place. Snapshot exports a PNG. Map returns to the whole Mandelbrot set. View > Studio Look opens the original live look editor.\n\nAll rendering uses fd explore running on your machine.");
  }
  function fullscreen() {
    if (document.fullscreenElement) document.exitFullscreen();
    else document.documentElement.requestFullscreen().catch(err => notice(String(err), true));
  }
  const menus = [
    [["Snapshot PNG", screenshot], ["Save current place", save], ["Open classic viewer", () => { location.href = "/"; }]],
    [["Go to coordinates…", gotoCoords], ["Copy exact coordinates", copyCoords], ["Reset look", () => tell("reset-look")]],
    [["Toggle sidebar", () => $(".sidebar").classList.toggle("closed")], ["Studio Look (L)", () => tell("studio")], ["Fullscreen", fullscreen]],
    [["Seahorse Valley", () => choose(0)], ["Whole Mandelbrot Set", () => tell("jump", { index: 0 })], ["Random discovery", () => choose(9)], ["Saved fossils…", () => fossils($$(".menubar span")[3])]],
    [["Pause / resume flow", () => tell("pause")], ["Studio Look", () => tell("studio")], ["Reset look", () => tell("reset-look")]],
    [["Controls and shortcuts", showHelp], ["About Fractodactyl", () => alert("Fractodactyl Explorer V2\nLinux UI built around the real fd Mandelbrot renderer.")]]
  ];
  $$(".menubar span").forEach((entry, i) => {
    entry.tabIndex = 0;
    entry.setAttribute("role", "button");
    entry.onclick = () => menu(entry, menus[i]);
    entry.onkeydown = e => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); entry.click(); } };
  });
  const tools = $$(".toolbar .tool");
  const handlers = [
    () => menu(tools[0], [["Zoom in (+)", () => zoom(1)], ["Zoom out (-)", () => zoom(-1)], ["Whole set", () => tell("jump", { index: 0 })]]),
    () => { frame.contentWindow.focus(); notice("Drag the fractal or use the arrow pad to pan."); },
    () => choose(9),
    () => fossils(tools[3]),
    screenshot,
    () => tell("jump", { index: 0 }),
    save
  ];
  tools.forEach((button, i) => { button.onclick = handlers[i]; button.title = ["Zoom options", "Drag to pan", "Discover another location", "Open saved places", "Save image as PNG", "Whole set map", "Save your location"][i]; });
  const dpad = $$(".dpad .dbtn");
  const moves = [[0, -0.22], [-0.22, 0], null, [0.22, 0], [0, 0.22]];
  dpad.forEach((b, i) => b.onclick = () => { if (moves[i]) pan(moves[i][0], moves[i][1]); else choose(0); });
  $("#zoomIn").onclick = () => zoom(1);
  $("#zoomOut").onclick = () => zoom(-1);
  $$("#presets li").forEach((li, i) => {
    li.tabIndex = 0;
    li.setAttribute("role", "option");
    li.onclick = () => choose(i);
    li.onkeydown = e => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); choose(i); } };
  });
  $(".navthumb").role = "button";
  $(".navthumb").title = "Whole set overview";
  $(".navthumb").tabIndex = 0;
  $(".navthumb").onclick = () => tell("jump", { index: 0 });
  $(".navthumb").onkeydown = e => { if (e.key === "Enter") tell("jump", { index: 0 }); };
  const sb = $$(".vscroll .sbtn");
  if (sb[0]) sb[0].onclick = () => pan(0, -0.2);
  if (sb[1]) sb[1].onclick = () => pan(0, 0.2);
  const track = $(".vscroll .track");
  track.onclick = e => pan(0, e.offsetY < track.clientHeight / 2 ? -0.25 : 0.25);
  const windowButtons = $$(".winbtns .winbtn");
  windowButtons[0].title = "Collapse / expand sidebar";
  windowButtons[0].onclick = () => $(".sidebar").classList.toggle("closed");
  windowButtons[1].title = "Fullscreen";
  windowButtons[1].onclick = fullscreen;
  windowButtons[2].title = "Return to classic viewer";
  windowButtons[2].onclick = () => { location.href = "/"; };
  // VIEW-06: inside fd-viewer's frameless desktop window (window.ipc), the stone chrome
  // is the window's chrome: the three buttons minimise, maximise and close it, the
  // banner moves it, and the 6px rim resizes it.
  if (window.ipc) {
    const send = m => window.ipc.postMessage(m);
    [["Minimise", "minimize"], ["Maximise", "maximize"], ["Close", "close"]].forEach(([t, m], i) => {
      windowButtons[i].title = t;
      windowButtons[i].onclick = () => send(m);
    });
    const bar = document.getElementById("titlebar");
    bar.addEventListener("pointerdown", e => { if (e.button === 0 && !e.target.closest(".winbtns")) send("drag"); });
    bar.addEventListener("dblclick", e => { if (!e.target.closest(".winbtns")) send("maximize"); });
    const rim = e => (e.clientY < 6 ? "n" : e.clientY > innerHeight - 6 ? "s" : "")
      + (e.clientX < 6 ? "w" : e.clientX > innerWidth - 6 ? "e" : "");
    addEventListener("pointermove", e => {
      const d = rim(e);
      document.documentElement.style.cursor = d ? d + "-resize" : "";
    }, true);
    addEventListener("pointerdown", e => {
      const d = rim(e);
      if (d && e.button === 0) { e.preventDefault(); e.stopPropagation(); send("resize:" + d); }
    }, true);
  }
  addEventListener("pointerdown", e => { if (menuBox && !menuBox.contains(e.target) && !e.target.closest(".menubar,.toolbar")) closeMenu(); }, true);
  addEventListener("keydown", e => {
    if (e.target.matches("input,select,textarea")) return;
    if (e.key === "Escape") { closeMenu(); return; }
    if (e.key === "+" || e.key === "=") zoom(1);
    else if (e.key === "-") zoom(-1);
    else if (e.key === "ArrowUp") pan(0, -0.22);
    else if (e.key === "ArrowDown") pan(0, 0.22);
    else if (e.key === "ArrowLeft") pan(-0.22, 0);
    else if (e.key === "ArrowRight") pan(0.22, 0);
    else if (e.key.toLowerCase() === "s" && !e.ctrlKey) screenshot();
    else if (e.key.toLowerCase() === "l") tell("studio");
    else if (e.key === " ") { e.preventDefault(); tell("pause"); }
    else if ("1234567890".includes(e.key)) {
      const n = "1234567890".indexOf(e.key);
      if (n >= 0) tell("jump", { index: n });
    } else return;
    e.preventDefault();
  });
  const coordinateCell = $$(".statusbar .cell")[3], iterationCell = $$(".statusbar .cell")[2];
  function zoomString(v) {
    if (!Number.isFinite(v)) return "—";
    const exponent = Math.floor(v), mantissa = Math.pow(10, v - exponent);
    return mantissa.toFixed(1) + " × 10^" + exponent;
  }
  function showState(s) {
    snapshot = s;
    const zoomText = zoomString(s.zoom);
    $("#zoomVal").textContent = zoomText;
    $("#zoomStatus").textContent = "Zoom: " + zoomText;
    const values = $$(".kv dd");
    if (values[0]) values[0].textContent = s.location;
    if (values[2]) {
      values[2].textContent = s.re.slice(0, 18) + (s.re.length > 18 ? "…" : "") + " + " + s.im.slice(0, 18) + (s.im.length > 18 ? "…" : "") + "i";
      values[2].title = s.re + " + " + s.im + "i";
    }
    if (iterationCell) iterationCell.textContent = "Iter: " + Number(s.iter).toLocaleString();
    if (coordinateCell) coordinateCell.textContent = s.kernel ? "Kernel: " + s.kernel : "Rendering…";
    bar.style.width = Math.round(100 * s.progress) + "%";
    if (s.error) notice(s.error, true);
    else {
      status.style.color = "";
      status.textContent = s.studio ? (s.progress === 1 ? "Studio look · ready" : "Studio look · rendering…") :
        s.refining ? "Exploring · refining " + s.level + "/" + s.levels : "Ready · " + s.look;
    }
  }
  addEventListener("message", e => {
    if (e.source !== frame.contentWindow || e.origin !== location.origin || !e.data || e.data.source !== "fd-explorer-embed") return;
    const m = e.data;
    if (m.type === "ready") {
      ready = true;
      while (pending.length) frame.contentWindow.postMessage(pending.shift(), location.origin);
      if (!initialized) { initialized = true; choose(0); }
    } else if (m.type === "state") showState(m.state);
    else if (m.type === "notice") notice(m.message, m.error);
    else if (m.type === "snapshot") {
      const a = document.createElement("a");
      a.download = "fractodactyl-" + new Date().toISOString().replace(/[:.]/g, "-") + ".png";
      a.href = m.data;
      document.body.append(a); a.click(); a.remove();
      notice("Snapshot exported as PNG");
    }
  });
  notice("Connecting to fd explorer…");
})();
