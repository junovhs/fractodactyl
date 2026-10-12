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
central pane embeds the original explorer in canvas-only mode, retaining exact
BigInt coordinates, real progressive rendering and the original GPU studio look.

Controls:
- Drag/wheel in the fractal pane: precise pan and zoom toward the cursor.
- +/- buttons, zoom menu and directional pad: real changes to camera coordinates.
- Presets: combinations of genuine destinations and saved studio looks.
- Explore: discover another destination. Map: whole set.
- Fossils: load saved places. Nest: save current location to places/*.place.
- Snapshot: export the actual rendered view as a PNG.
- View > Studio Look: original full-quality freeze-and-tune studio editor.
- Menus: save, enter/copy coordinates, pause, reset look, fullscreen, help.

The existing viewer/explore.sh and Windows viewer/explore.cmd keep opening
the classic version by default. V2 can run simultaneously in another tab.

Verification on a Linux machine with Rust and a browser:

    cargo test -p fd-cli v2_is_real_viewer_not_fake_mockup_timer
    viewer/explore-v2.sh

Exercise drag/wheel navigation, save/reopen a place, snapshot and studio
tuning. The Rust test covers page construction; an interactive browser test
on Linux is still required.
