# Fractodactyl Explorer V2 (Linux)

V2 is a second UI for the real fd explore renderer, not a screenshot.
The original explorer remains at http://127.0.0.1:8737/ and V2 opens at
http://127.0.0.1:8737/v2/.

From the repository root:

    viewer/explore-v2.sh             # build/start fd; open V2
    viewer/explore-v2.sh --install   # install a separate Linux desktop launcher
    viewer/explore-v2.sh --stop      # stop the shared fd explore server

Both versions share the same loopback-only Rust backend. V2 derives its artwork,
window, controls and textures from viewer/mockup/index.html while removing the
fake operating-system desktop, taskbar, static fractal and simulated timer. The
central pane embeds the original explorer in canvas-only mode (`/?embed=1`), keeping
exact BigInt coordinates, the tile engine (docs/spec/EXPLORE.md) and the GPU studio
look. The shell (viewer/mockup/live.js) drives it through embed.js messages.

Controls:
- Fractal pane: drag to pan (it glides on release), wheel to zoom toward the cursor.
- Keys: + / - zoom, arrow keys pan, 1-9 and 0 jump to places, L opens the studio look.
- Toolbar: **Explore** jumps to a random landmark; **Places ▼** lists fd explore's
  landmarks; **Saved ▼** has "Save this spot…" (writes places/NAME.place) and your
  saved spots; **Snapshot** saves the rendered view as a PNG.
- Sidebar: **Expedition** shows location, zoom and coordinates; **Navigation** has the
  overview thumbnail (click for the whole set), the direction pad and - / + zoom;
  **Looks** lists /looks. Picking a look only recolours; the camera stays put. The
  last look is remembered, else master-ice.
- Menus: File (snapshot, save spot, classic viewer), Edit (go to / copy exact
  coordinates, reset look), View (sidebar, Studio Look, fullscreen), Expedition
  (places, random landmark, saved spots, whole set), Options (pause flow, studio look,
  reset look), Help (controls, about). Prompts open as in-page stone dialogs.

The existing viewer/explore.sh and Windows viewer/explore.cmd keep opening
the classic version by default. V2 can run simultaneously in another tab.

Verification on a Linux machine with Rust and a browser:

    cargo test -p fd-cli v2_
    viewer/explore-v2.sh

Exercise drag/wheel navigation, save/reopen a place, snapshot and studio
tuning. The Rust tests cover page construction; interactive checks are manual.

## Desktop window (VIEW-06)

`fd-viewer` shows V2 in its own desktop window (Dioxus 0.7, as ../ishoo's window),
with no browser:

    cargo build --release -p fd-cli -p fd-viewer
    target/release/fd-viewer            # or: fd-viewer --fd path/to/fd
    target/release/fd-viewer --check    # start the renderer, fetch a render, exit

It starts `fd explore --exit-with-stdin 1` on a free loopback port (from the repository
root, so looks/, places/ and data/zones/ resolve) and loads /v2/ in a wry webview
filling a frameless window. On Linux the webview is a GTK widget in the window's own box
(Dioxus's own webview is hidden); elsewhere it is a child webview. As an X11 child window
it stopped receiving the mouse wheel after the first click (VIEW-09). The stone banner is the title bar: its three buttons
minimise, maximise and close the window, dragging the banner moves it, the 6 px rim
resizes it. The renderer exits whenever the window does (its stdin pipe closes).
On Linux it needs WebKitGTK 4.1. `viewer/wheel-after-drag.sh` (X11, xdotool) checks
that the wheel still zooms after a drag.
