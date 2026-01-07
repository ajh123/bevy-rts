# Bevy RTS

A basic RTS template using Bevy engine that renders a heightmap divided into chunks, with basic camera controls and an object placement system.

## Overview

Bevy RTS is a small real-time strategy (RTS) template built with the Bevy game engine. It renders a heightmap divided into chunks, provides camera controls, and includes an object placement system. The project is modular and data-driven.

## Quick start

Clone and run (requires Rust and Cargo):

```powershell
git clone <repo-url>
cd rust-heightmap
cargo run
```

## Features

- Chunked heightmap rendering (flat and Perlin generators).
- Top-down camera with movement and zoom.
- Object placement system (freeform + rotational placement).
- Data-driven tiles and object definitions stored in `assets/*.ron`.

## Where to look

- Full development timeline: [docs/HISTORY.md](docs/HISTORY.md)
- Contributing guide: [CONTRIBUTING.md](CONTRIBUTING.md)
- Quick code entry points:
	- `src/main.rs` - Bevy app setup
	- `src/terrain.rs`, `src/terrain_renderer.rs` - terrain logic
	- `src/object_system.rs`, `src/object_renderer.rs` - object systems

## Architecture

- `terrain` / `terrain_renderer` - chunk generation and mesh creation
- `object_system` / `object_renderer` - object placement, transforms, rendering
- `camera` - input-driven camera movement and zoom systems
- `tile_types`, `assets` - RON-based data definitions

## Contributing

Please see [CONTRIBUTING.md](CONTRIBUTING.md) for build, style, and PR guidelines.

## License

Dual-licensed under MIT and Apache-2.0. See [`LICENSE-MIT`](LICENSE-MIT) and [`LICENSE-APACHE`](LICENSE-APACHE).

## Asset credits

- 2 Story House: (https://skfb.ly/o8Xus) by Designed By Jonathan is licensed under Creative Commons Attribution (http://creativecommons.org/licenses/by/4.0/).