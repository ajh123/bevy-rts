# Project History

## Phase 1 (27th December 2025): Heightmap Renderer

- **Pass 1**: Basic setup (`winit` + `wgpu`), plane chunking (commits `3d58d74c`, `b286658`).  
![Screenshot](./Screenshot%202025-12-27%20123805.png)

- **Pass 2**: Flat terrain generator + Perlin noise terrain (commits `7b62d97`, `b15b506`).  
![Screenshot](./Screenshot%202025-12-27%20132051.png)


## Phase 2 (6th-7th January 2026): Bevy-based Renderer with Object Placement and GUI

- **Pass 1**: Complete nuking of the project. Rewrite all features with Bevy (commits `aacdf31`, `34b4fea`).  
![Screenshot](./Screenshot%202026-01-06%20145802.png)

- **Pass 2**: Object placement system, data-driven design (commits `1e34079`, `8fd6af3`).  
![Screenshot](./Screenshot%202026-01-06%20194258.png)

- **Pass 3**: Freeform & rotational object placement (commit `e037a65`).  
![Screenshot](./Screenshot%202026-01-06%20201904.png)

- **Pass 4**: Per-model scaling, GUI (using `egui`), toolbar for object placement/destruction (commits `d91c3c9`, `cfd5984`, `b8dba91`, `f1881d8`, `c069526`).  
![Screenshot](./Screenshot%202026-01-07%20173346.png)