# Feature Map: what fd should recognise, and why (owner idea, 2026-10-09)

The owner's idea: fd should know every *type* of thing in the Mandelbrot set (minibrots,
spirals, valleys, embedded Julia sets, snake pits, ...) and find them quickly. Then it can
map paths between features automatically, and string them into journeys that stay
interesting and have deliberate climaxes, like a fireworks show. Because the fast path is
built from the same dynamics that define many of these features, knowing the map should
also make rendering cheaper.

Plan: "Feature map and trip design". Issues: MAP-01 (`fd features`), MAP-02 (explorer
overlay), MAP-03 (`fd trip`), MAP-04 (visual-character probe). The foundation is PROB-12
(`fd zone`, plan "Fast path everywhere").

## Three kinds of feature

How a feature can be found decides which issue owns it.

1. **Exact.** Defined by the dynamics of z² + c, so it is found by solving equations
   (period detection, Newton's method). The answer is a precise centre, size and type.
   Owner: MAP-01.
2. **Derived.** Named regions and decorations that are labels on, or predictable
   neighbourhoods of, exact features. Once (1) exists, these come almost free. Owner:
   MAP-01 (labels), MAP-03 (used as trip stops).
3. **Visual.** Nicknames with no mathematical definition. They need cheap image measures
   on previews, and stay fuzzy. Owner: MAP-04.

## 1. Exact features

| Feature (and its other names) | What it is | How fd finds it |
|---|---|---|
| Main cardioid (continent, body) | The period-1 hyperbolic component | Closed form |
| Bulbs: period-2 (primary bulb, big disk, head), period-3, period-4, ... | Hyperbolic components attached to a parent | Nucleus by Newton on f^p(0) = 0; size and orientation from the standard size estimate |
| Minibrot (baby brot, midget, mandelbrotie, island, distorted minibrot) | A small copy of the set: a primitive cardioid component and everything attached to it | Period detection (atom domain / box period), then nucleus Newton; size and skew |
| Satellite vs primitive component / minibrot | Attached at a parabolic root vs a cusp-rooted cardioid | Shape test on the component (disk vs cardioid) |
| Hyperbolic component, mu-atom | A region with an attracting cycle (mu-atom is Munafo's name) | As above |
| Mu-molecule | A component plus everything attached to it (Munafo) | Tree of components found around one nucleus |
| Centre, nucleus, superattracting parameter | Where 0 is periodic | Newton on f^p(0) = 0 |
| Root, bond point | Where a child component attaches to its parent | Newton where the cycle multiplier equals e^(2πi·p/q) |
| Internal angle, rotation number p/q | Which child attaches where | From the root solve |
| Cusp | A cardioid's sharp point (0.25 for the main one) | The multiplier = 1 root of a cardioid |
| Misiurewicz point, branch point, tip, terminal point | 0 is eventually periodic: preperiod q, period r (M(q,r)) | Newton on the preperiodic equation; |λ| > 1 (repelling) |
| Spiral centre: single, double, triple, quadruple, multi-arm spiral, pinwheel | A Misiurewicz point; the spiral's arm and branch count comes from its cycle | Misiurewicz solve, then arm count from the cycle and multiplier |
| Misiurewicz ladder | Minibrots lined up on a spiral centre m, each rung closer by 1/\|ρ\| and smaller by 1/\|ρ\|². v0 is rung 0 of a ladder on M(24,2), with rungs found down to 1e-1000 (docs/research/10-9-26/misiurewicz-ladders.md) | One-term guess m + (c_0 − m)ρ⁻ᵏ, then Newton (2-5 steps). This gives exact minibrot destinations at any chosen depth |
| Period-doubling cascade, Feigenbaum point | Bulbs of period 2, 4, 8, ... and their limit (−1.401155189 on the real axis) | Chain of root solves |
| Wake, limb, external ray | Region cut off by a pair of external rays; the branch hanging off a root | External-ray tracing (a later follow-up, if trips need it) |
| Equipotential line | Curves of constant escape potential, outside the set | Already in the smooth escape value fd computes |

## 2. Derived features

**Named valleys and coasts.** Each is the neighbourhood of a root or cusp:

| Name | Where |
|---|---|
| Seahorse Valley (Seahorse Valley East) | The 1/2 root between the main cardioid and the period-2 bulb, around −0.75 + 0.1i |
| Double Spiral Valley, Double Spiral Coast | The period-2 bulb's side of Seahorse Valley |
| Elephant Valley (East Valley), Elephant Coast | Near the main cardioid's cusp, on its right, around 0.28 + 0.01i |
| Scepter Valley (Seahorse Valley West), Dragon Valley | Between the period-2 bulb and its period-4 child, around −1.25 |
| Double Scepter Valley | Between the upper period-3 bulb and an attached period-6 bulb |
| Triple Spiral Valley, Triple Spiral Coast | Near the period-3 bulbs, around −0.088 + 0.654i |
| Quad Spiral Valley | Near period-4 components |
| Disk-3 Seahorse Valley | Between the main cardioid and a period-3 bulb |
| Fjord, cleft, pinch point | Narrow gaps between components: the neighbourhood of any root or cusp |

**Embedded Julia sets** (Julia island, embedded Julia set, medallion). Near a minibrot of
period p sitting close to a Misiurewicz point, a copy of the local Julia set appears. It
shows up at a zoom between the surrounding pattern's scale and the minibrot's own size.
Deep-zoom artists aim for it on purpose ("Julia morphing"). fd records the predicted width
as a hint, not a certified fact. The medallion's type follows from the local structure:
seahorse, elephant, scepter, double-spiral, triple-spiral, multiple-spiral, needle (near
an antenna tip), branch, non-spiral, carrot, and cauliflower (also called brain, near a
mini-cardioid cusp).

**Munafo's descriptive taxonomy** (nucleus, nucleolus, paramecium, mitochondrion). These
describe structure inside embedded Julia sets. They can be partly derived from (1), and
the rest falls back to (3).

## 3. Visual features (MAP-04)

These have no definition. MAP-04 tests cheap measures on low-resolution previews: spiral
arm count from the angular spectrum around a point, boundary density from the distance
estimate, radial symmetry, and filament fraction.

- **Shapes:** seahorse, seahorse tail, elephant and elephant trunk, scepter, double hook,
  shepherd's crook, spindle, lightning branch or bolt, dendrite, tendril, filament or
  hair, spoke, peacock eye (compound eye), shrub, shrub tip, cauliflower, carrot,
  starfish, spider, octopus arms, paperclip, peanut.
- **Artistic nicknames:** snake pit, whirlpool, galaxy, eye, backbone, ring, cloud,
  coral, flower or rosette, jewel, Mandelbrot bug.
- **Apple doll:** an old name for the whole set.

Some of these are partly exact. Arm count and spiral type come from (1). Seahorses,
elephants and scepters mark their valleys from (2).

## Starting coordinates (approximate)

| Location | c |
|---|---|
| Seahorse Valley | −0.75 + 0.10i |
| Famous two-arm spiral | −0.77568377 + 0.13646737i |
| Elephant Valley | 0.2793 + 0.0094i |
| Triple Spiral Valley | −0.088 + 0.654i |
| Period-3 antenna minibrot | −1.754877666 + 0i (the source list gave −1.7497, which is off) |
| Feigenbaum point | −1.401155189 + 0i |
| End of the main antenna | −2 + 0i (Misiurewicz: 0 → −2 → 2 → 2) |
| Main cardioid cusp | 0.25 + 0i |
| Famous deep-zoom neighbourhood | −0.7436438870371587 + 0.1318259042053120i |
| v0 benchmark: period-764 minibrot near M(24,2) | −0.7432918908524302… + 0.1312405523087976…i (bench/path-atlas-v0.txt) |
| spiral-on-spiral (the 10-08 film) | places/spiral-on-spiral.place (FILM-07 restores it) |

## Storytelling

The set is a tree of components, limbs and wakes, so a journey can be a walk down that
tree: cardioid → valley → branch point → spiral → minibrot. Arriving at a minibrot shows
the whole set again, which works as a climax and as a callback to the opening. From there
the trip can continue into that minibrot's own seahorse valley, a level deeper.

Beat types for `fd trip` (MAP-03):
- **build:** a long, steady dive along a spiral;
- **reveal:** arriving at a minibrot;
- **fireworks:** passing through an embedded Julia set as it forms and resolves;
- **breathe:** a slow dwell or drift;
- **travel:** zoom out, pan and zoom back in (the van Wijk–Nuij smooth zoom-pan path).

MAP-04's interest score shapes the pacing.

## Why this also helps speed

- **The same maths finds features and shortcuts.** The fast path (KERN-01, PROB-14)
  exists exactly where a pixel's orbit loops near a repelling cycle or minibrot, which is
  the structure MAP-01 finds. Probe E (2026-10-09) showed the jump also works at long
  cycles (period 197 and 655) on the "Eye of the Universe" zoom, with a smaller usable
  radius. A route through known features stays on fast ground, and one zone serves a
  whole segment.
- **Zooming back out is nearly free.** Keyframes are reusable maths, so the zoom-out half
  of a travel leg reuses the way in.
- **Cost is known up front.** Per-segment estimates come before any rendering.

Visual-only features (clouds, coral) give no speed. The gain comes from the exact ones.
