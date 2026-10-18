Fractodactyl @VERSION@
======================

Deep Mandelbrot zooms that show the real set. https://fractodactyl.dev

What is in this folder
  fd-viewer   the explorer: drag to pan, scroll to zoom, change looks, save places
  fd          the command-line renderer: films (fd film), stills (fd shade) and more
  looks/      colour looks, used by both
  places/     saved places; the explorer adds yours here
  CHANGELOG.md  what changed in each version

Run it
  Windows: double-click fd-viewer.exe. It needs Microsoft Edge WebView2, which
           Windows 10 and 11 already include.
  Linux:   ./fd-viewer   It needs WebKitGTK 4.1 (Debian/Ubuntu: libwebkit2gtk-4.1-0).
  Keep fd and fd-viewer in the same folder: the explorer starts fd for you.

Manual:  https://fractodactyl.dev/docs/
Source:  https://github.com/junovhs/fractodactyl
Licence: MIT OR Apache-2.0, at your choice (LICENSE-MIT, LICENSE-APACHE)
