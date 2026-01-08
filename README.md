# Bevy RTS

## Project Goals

- Build a game-engine framework for RTS games in Rust using Bevy.
- Implement core RTS features: terrain, object placement, pathfinding, basic AI.
- Support modding via data-driven RON files and asset packs.
- Implement a server/client architecture for multiplayer.
- Push AI driven development to its limits, with only essential human refactoring.

## Quick start

Clone and run (requires Rust and Cargo):

```powershell
git clone https://github.com/ajh123/bevy-rts.git
cd bevy-rts
cargo run -p bevy-rts-client
```

## Features

- Chunked heightmap rendering (Perlin noise).
- Top-down camera with movement and zoom.
- Object placement system (freeform + rotational placement).
- Data-driven tiles and object definitions stored in `assets/*.ron`.

## Where to look

- Full development timeline: [docs/HISTORY.md](docs/HISTORY.md)
- Contributing guide: [CONTRIBUTING.md](CONTRIBUTING.md)
- Design plan: [docs/plan.md](docs/plan.md)
- Code reviews: [docs/reviews/](docs/reviews/)

## Contributing

Please see [CONTRIBUTING.md](CONTRIBUTING.md) for build, style, and PR guidelines.

## License

Dual-licensed under MIT and Apache-2.0. See [`LICENSE-MIT`](LICENSE-MIT) and [`LICENSE-APACHE`](LICENSE-APACHE).

## Asset credits

- 2 Story House: (https://skfb.ly/o8Xus) by Designed By Jonathan is licensed under Creative Commons Attribution (http://creativecommons.org/licenses/by/4.0/).