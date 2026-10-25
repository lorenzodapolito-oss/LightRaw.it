# LightRaw app icon

A **camera aperture with six blades** in a cyan-to-magenta gradient, with the letters **LR** at its
centre, inside a glowing ring, on a dark navy rounded-square tile.

This replaces the upstream LightCraft lynx in the LightRaw fork. The upstream artwork is still here
(`lightcraft.svg`, `lightcraft-small.svg`) for reference and is not used by the app.

## Geometry

The source is a 512 px picture of the tile (`lightraw-source.png`), almost edge to edge. The tile is
cut out, its corners made transparent (corner radius about 19 % of the side) and upscaled with
Lanczos for the large sizes. The macOS renders pad it onto Apple's 824/1024 icon grid; the Windows
and Linux renders use the tile edge to edge. `python packaging/icons.py` regenerates every file.

## Provenance

Picture supplied by the LightRaw project owner (Lorenzo D'Apolito) as `lightraw-source.png`. If it was
made with an AI image generator, note that purely AI-generated images may not be protected by
copyright in some countries. License: see `LICENSE.txt`.

## Files

- `lightraw-source.png`: the source picture every other file is rendered from.
- `lightraw-1024.png`, `lightraw-macos-512.png`, `lightraw.icns`: macOS (padded on Apple's grid).
- `lightraw.ico`, `hicolor/`: Windows and Linux (tile edge to edge).
