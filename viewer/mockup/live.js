"use strict";
// Fractodactyl Explorer V2: the stone-and-parchment mockup controls the real
// fd explore engine through its same-origin embedded canvas.
(function () {
  const $ = s => document.querySelector(s);
  const $$ = s => Array.from(document.querySelectorAll(s));
  const frame = $("#live-view");
  const bar = $("#bar"), status = $("#status");
  // Built-in places (fd explore's PRESETS, sent with "ready"); index 0 is the whole set.
  let places = ["Whole set"];
  let looks = [];
  let ready = false, initialized = false, snapshot = null, menuBox = null, dialogBox = null;
  const pending = [];
  function tell(cmd, extra) {
    const message = Object.assign({ source: "fd-v2-ui", cmd: cmd }, extra || {});
    if (!ready) pending.push(message);
    else frame.contentWindow.postMessage(message, location.origin);
  }
  // A notice holds the status line for a few seconds before the live state takes it back.
  let noticeUntil = 0;
  function notice(text, error) {
    noticeUntil = performance.now() + 3500;
    status.textContent = text;
    status.style.color = error ? "#8c122a" : "";
  }
  // VIEW-07: dialogs live in the page. WebKitGTK showed the native prompt and alert boxes in the
  // frameless window only after it lost and regained focus. Resolves to the field values,
  // or null when cancelled.
  function dialog(title, text, fields, okLabel) {
    closeDialog();
    return new Promise(resolve => {
      const shade = document.createElement("div");
      shade.id = "live-dialog";
      const box = document.createElement("form");
      const h = document.createElement("h4");
      h.textContent = title;
      box.append(h);
      if (text) {
        const p = document.createElement("p");
        p.textContent = text;
        box.append(p);
      }
      const inputs = (fields || []).map(f => {
        const label = document.createElement("label");
        label.textContent = f.label;
        const input = document.createElement(f.multiline ? "textarea" : "input");
        input.value = f.value || "";
        input.spellcheck = false;
        if (f.readonly) input.readOnly = true;
        label.append(input);
        box.append(label);
        return input;
      });
      const row = document.createElement("div");
      row.className = "buttons";
      const finish = value => { closeDialog(); resolve(value); };
      if (fields && fields.length) {
        const cancel = document.createElement("button");
        cancel.type = "button";
        cancel.textContent = "Cancel";
        cancel.onclick = () => finish(null);
        row.append(cancel);
      }
      const ok = document.createElement("button");
      ok.type = "submit";
      ok.textContent = okLabel || "OK";
      row.append(ok);
      box.append(row);
      box.onsubmit = e => { e.preventDefault(); finish(inputs.map(i => i.value.trim())); };
      shade.onpointerdown = e => { if (e.target === shade) finish(null); };
      box.onkeydown = e => { if (e.key === "Escape") { e.preventDefault(); finish(null); } };
      shade.append(box);
      document.body.append(shade);
      dialogBox = shade;
      const first = inputs[0] || ok;
      first.focus();
      if (inputs[0]) inputs[0].select();
    });
  }
  function closeDialog() {
    if (dialogBox) { dialogBox.remove(); dialogBox = null; }
  }
  function goTo(index) {
    if (index < 0 || index >= places.length) return;
    tell("jump", { index: index });
  }
  function explore() {
    // A different built-in landmark each time (never the whole set).
    const here = snapshot ? places.indexOf(snapshot.location) : -1;
    const options = places.map((_, i) => i).filter(i => i > 0 && i !== here);
    if (options.length) goTo(options[Math.floor(Math.random() * options.length)]);
  }
  function lookTitle(name) {
    return name.split(/[-_]/).map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(" ");
  }
  function setLook(name) {
    tell("look", { name: name, studio: true });
  }
  function showLooks(current) {
    $$("#looks li").forEach(li => {
      const on = li.dataset.look === current;
      li.classList.toggle("sel", on);
      li.setAttribute("aria-selected", on ? "true" : "false");
    });
  }
  let looksLoaded = null;
  async function loadLooks() {
    try {
      const r = await fetch("/looks");
      if (!r.ok) throw Error(await r.text());
      looks = (await r.text()).split("\n").filter(Boolean);
    } catch (err) { notice(String(err), true); return; }
    // Your own looks (looks/*.look, e.g. master-ice) first, then the built-ins.
    looks.sort((a, b) => (BUILTIN_LOOKS.includes(a) - BUILTIN_LOOKS.includes(b)) || a.localeCompare(b));
    const list = $("#looks");
    list.replaceChildren(...looks.map(name => {
      const li = document.createElement("li");
      li.textContent = lookTitle(name);
      li.dataset.look = name;
      li.tabIndex = 0;
      li.setAttribute("role", "option");
      li.onclick = () => setLook(name);
      li.onkeydown = e => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); setLook(name); } };
      return li;
    }));
    if (snapshot) showLooks(snapshot.look);
  }
  const BUILTIN_LOOKS = ["ice", "coral", "steel", "zebra", "smoke"];
  function zoom(n) { tell("zoom", { delta: n > 0 ? -Math.log10(2) : Math.log10(2) }); }
  function pan(x, y) { tell("pan", { x: x, y: y }); }
  async function save() {
    const v = await dialog("Save this spot",
      "Saved spots appear under Saved, and fd film --place NAME can render them.",
      [{ label: "Name (letters, digits, - or _)", value: "my_discovery" }], "Save");
    if (v && v[0]) tell("save", { name: v[0] });
  }
  function screenshot() { tell("snapshot"); }
  function copyCoords() {
    if (!snapshot) return;
    const coords = snapshot.re + " " + snapshot.im + " " + snapshot.width;
    const fallback = () => dialog("Exact coordinates", "Copy them from here:", [{ label: "re im width", value: coords, multiline: true, readonly: true }], "Done");
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(coords).then(() => notice("Copied exact coordinates")).catch(fallback);
    } else fallback();
  }
  async function gotoCoords() {
    const v = await dialog("Go to coordinates", null, [
      { label: "Real", value: snapshot ? snapshot.re : "-0.743643887" },
      { label: "Imaginary", value: snapshot ? snapshot.im : "0.131825904" },
      { label: "Width (e.g. 1e-12)", value: snapshot ? snapshot.width : "0.0065" }
    ], "Go");
    if (v) tell("jump-place", { name: "Coordinates", re: v[0], im: v[1], width: v[2], iter: 5000 });
  }
  function closeMenu() {
    if (menuBox) { menuBox.remove(); menuBox = null; }
  }
  function menu(anchor, items) {
    closeMenu();
    const box = document.createElement("div");
    box.id = "live-menu";
    items.forEach(item => {
      if (!item) { box.append(document.createElement("hr")); return; }
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
  function placesMenu(anchor) {
    menu(anchor, places.map((name, i) => [(i < 10 ? ((i + 1) % 10) + "  " : "") + name, () => goTo(i)]));
  }
  async function savedMenu(anchor) {
    let rows = [];
    try {
      const r = await fetch("/places");
      if (!r.ok) throw Error(await r.text());
      rows = (await r.text()).split("\n").filter(Boolean).map(row => row.split("\t"));
    } catch (err) { notice(String(err), true); }
    menu(anchor, [["Save this spot…", save], null].concat(rows.length ? rows.map(row => [row[0] + "  ·  " + row[3], () => {
      tell("jump-place", { name: row[0], re: row[1], im: row[2], width: row[3], iter: Number(row[4]) || 5000 });
    }]) : [["(no saved spots yet)", () => {}]]));
  }
  function showHelp() {
    dialog("Controls", "Drag the fractal to pan. Use the mouse wheel to zoom in or out at the cursor; the arrow keys and + / - work too.\n\n" +
      "Explore jumps to a random landmark. Places lists the landmarks (keys 1–9, 0). Saved holds the spots you saved with Save this spot. " +
      "Snapshot saves a PNG (key S). Looks in the sidebar change only the colours; L opens the look studio, space pauses the colour flow.\n\n" +
      "All rendering uses fd explore on your machine.");
  }
  function fullscreen() {
    if (document.fullscreenElement) document.exitFullscreen();
    else document.documentElement.requestFullscreen().catch(err => notice(String(err), true));
  }
  const menus = [
    [["Snapshot PNG", screenshot], ["Save this spot…", save], ["Open classic viewer", () => { location.href = "/"; }]],
    [["Go to coordinates…", gotoCoords], ["Copy exact coordinates", copyCoords], ["Reset look", () => tell("reset-look")]],
    [["Toggle sidebar", () => $(".sidebar").classList.toggle("closed")], ["Studio Look (L)", () => tell("studio")], ["Fullscreen", fullscreen]],
    [["Places…", () => placesMenu($$(".menubar span")[3])], ["Random landmark", explore], ["Saved spots…", () => savedMenu($$(".menubar span")[3])], ["Whole Mandelbrot set", () => goTo(0)]],
    [["Pause / resume flow", () => tell("pause")], ["Studio Look", () => tell("studio")], ["Reset look", () => tell("reset-look")]],
    [["Controls and shortcuts", showHelp], ["About Fractodactyl", () => dialog("About Fractodactyl", "Fractodactyl Explorer V2: the stone viewer around the real fd Mandelbrot renderer.")]]
  ];
  $$(".menubar span").forEach((entry, i) => {
    entry.tabIndex = 0;
    entry.setAttribute("role", "button");
    entry.onclick = () => menu(entry, menus[i]);
    entry.onkeydown = e => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); entry.click(); } };
  });
  const tools = $$(".toolbar .tool");
  const handlers = [explore, () => placesMenu(tools[1]), () => savedMenu(tools[2]), screenshot];
  const tips = ["Jump to a random landmark", "Go to a landmark", "Your saved spots, and save this one", "Save the view as a PNG"];
  tools.forEach((button, i) => { button.onclick = handlers[i]; button.title = tips[i]; });
  const dpad = $$(".dpad .dbtn");
  const moves = [[0, -0.22], [-0.22, 0], null, [0.22, 0], [0, 0.22]];
  dpad.forEach((b, i) => b.onclick = () => { if (moves[i]) pan(moves[i][0], moves[i][1]); else goTo(0); });
  dpad[2].title = "Whole set";
  $("#zoomIn").onclick = () => zoom(1);
  $("#zoomOut").onclick = () => zoom(-1);
  looksLoaded = loadLooks();
  $(".navthumb").role = "button";
  $(".navthumb").title = "Whole set overview";
  $(".navthumb").tabIndex = 0;
  $(".navthumb").onclick = () => goTo(0);
  $(".navthumb").onkeydown = e => { if (e.key === "Enter") goTo(0); };
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
    // Minimal native-window glue: the styling UI itself is Rust/Dioxus.
    menus[4].unshift(["Styling workspace…", () => send("style")]);
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
    if (dialogBox || e.target.matches("input,select,textarea")) return;
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
      if (n >= 0) goTo(n);
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
    showLooks(s.look);
    if (coordinateCell) coordinateCell.textContent = s.kernel ? "Kernel: " + s.kernel : "Rendering…";
    bar.style.width = Math.round(100 * s.progress) + "%";
    if (s.error) notice(s.error, true);
    else if (performance.now() >= noticeUntil) {
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
      if (Array.isArray(m.places) && m.places.length) places = m.places;
      while (pending.length) frame.contentWindow.postMessage(pending.shift(), location.origin);
      if (!initialized) {
        initialized = true;
        goTo(1);
        looksLoaded.then(() => setLook(looks.includes(m.look) ? m.look : looks.includes("master-ice") ? "master-ice" : "ice"));
      }
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
  // VIEW-07: the frame may have said "ready" before this script ran; ask it again.
  try { frame.contentWindow.postMessage({ source: "fd-v2-ui", cmd: "hello" }, location.origin); } catch (e) {}
})();
