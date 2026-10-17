"use strict";
// Optional same-origin bridge for the Linux V2 shell. The classic viewer is unchanged.
(function () {
  if (window.parent === window || !new URLSearchParams(location.search).has("embed")) return;
  document.documentElement.classList.add("embedded");
  const origin = location.origin;
  let placeName = "Whole set";
  // VIEW-07: the shell picks the opening look, so take the page's remembered one away
  // from its own restore (which would race the shell) and hand it over with "ready".
  let remembered = null;
  try { remembered = localStorage.getItem("fd.navLook"); localStorage.removeItem("fd.navLook"); } catch (e) {}
  const send = (type, extra) => parent.postMessage(Object.assign({ source: "fd-explorer-embed", type: type }, extra || {}), origin);
  const clamp = (n, lo, hi) => Math.max(lo, Math.min(hi, n));
  // VIEW-08: buttons and keys ease the camera (zoom about the centre, glide when panning).
  function setZoom(delta) {
    if (!view || studio.on) return;
    zoomBy(delta);
  }
  function pan(x, y) {
    if (!view || studio.on) return;
    glide(x * W, y * H);
  }
  function currentState() {
    if (!view) return;
    const nav = navProgress();
    const progress = studio.on ? (studio.ready ? 1 : 0) : clamp(nav.progress, 0, 1);
    send("state", {
      state: {
        location: placeName,
        re: toDec(view.re, view.D),
        im: toDec(view.im, view.D),
        width: widthStr(view.l10w),
        zoom: Math.log10(4) - view.l10w,
        iter: iterFor(view.l10w),
        kernel: last.info.kernel || "",
        progress: progress,
        refining: nav.refining,
        level: Math.min(nav.levels, nav.level + 1),
        levels: nav.levels,
        look: navLook ? navLook.name : look.pal,
        studio: studio.on,
        paused: studio.on ? studio.paused : paused,
        error: errBox.style.display === "block" ? errBox.textContent : ""
      }
    });
  }
  async function command(m) {
    switch (m.cmd) {
      case "jump":
        if (!Number.isInteger(m.index) || m.index < 0 || m.index >= PRESETS.length) throw Error("Unknown destination");
        if (studio.on) await toggleStudio();
        placeName = PRESETS[m.index].name;
        jump(PRESETS[m.index]);
        break;
      case "jump-place":
        if (!/^[+-]?\d+(?:\.\d+)?$/.test(m.re) || !/^[+-]?\d+(?:\.\d+)?$/.test(m.im) ||
            !/^\d+(?:\.\d+)?(?:e[+-]?\d+)?$/i.test(m.width)) throw Error("Invalid coordinates");
        if (studio.on) await toggleStudio();
        placeName = m.name || "Custom coordinates";
        jump({ name: placeName, re: m.re, im: m.im, w: m.width, iter: Number(m.iter) || 5000 });
        break;
      case "hello": sendReady(); return;
      case "zoom": setZoom(clamp(Number(m.delta) || 0, -1, 1)); break;
      case "pan": pan(clamp(Number(m.x) || 0, -2, 2), clamp(Number(m.y) || 0, -2, 2)); break;
      case "look":
        if (PALETTES[m.name] && !m.studio) {
          look.pal = m.name; setNavLook(null);
        } else {
          setNavLook({ name: m.name, look: await fetchLook(m.name) });
        }
        break;
      case "studio": await toggleStudio(); break;
      case "pause":
        if (studio.on) studio.paused = !studio.paused;
        else paused = !paused;
        break;
      case "reset-look":
        Object.assign(look, LOOK0);
        phase = 0; hue = 0; clock = 0;
        setNavLook(null);
        break;
      case "save": {
        const name = String(m.name || "").trim();
        if (!/^[a-z0-9_-]{1,64}$/i.test(name)) throw Error("Names use letters, numbers, - or _");
        const body = "# fractodactyl place (saved by Explorer V2)\n" +
          "re " + toDec(view.re, view.D) + "\nim " + toDec(view.im, view.D) + "\n" +
          "width " + widthStr(view.l10w) + "\niter " + iterFor(view.l10w) + "\n";
        const r = await fetch("/place?name=" + encodeURIComponent(name), { method: "POST", body: body });
        const msg = await r.text();
        if (!r.ok) throw Error(msg);
        send("notice", { message: "Saved place " + name + " (available to fd film)" });
        break;
      }
      case "snapshot": {
        const source = studio.on && studio.ready ? glc : canvas;
        send("snapshot", { data: source.toDataURL("image/png") });
        break;
      }
      default: throw Error("Unknown explorer command");
    }
    currentState();
  }
  addEventListener("message", function (e) {
    if (e.source !== parent || e.origin !== origin || !e.data || e.data.source !== "fd-v2-ui") return;
    Promise.resolve(command(e.data)).catch(function (err) { send("notice", { message: String(err.message || err), error: true }); });
  });
  addEventListener("keydown", function (e) {
    if (/INPUT|TEXTAREA|SELECT/.test(e.target.tagName) || studio.on) return;
    const directions = { ArrowUp: [0, -0.22], ArrowDown: [0, 0.22], ArrowLeft: [-0.22, 0], ArrowRight: [0.22, 0] };
    if (directions[e.key]) {
      e.preventDefault();
      pan(directions[e.key][0], directions[e.key][1]);
    }
  });
  setInterval(currentState, 300);
  // The shell's script can load after this frame (first, uncached load), so it also asks
  // with "hello" and gets "ready" again.
  function sendReady() {
    send("ready", { places: PRESETS.map(function (p) { return p.name; }), look: remembered });
  }
  sendReady();
  currentState();
})();
