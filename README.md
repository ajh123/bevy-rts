# Rust Heightmap Renderer

This project is a simple heightmap renderer written in Rust. It renders a basic flat plane split into chunks, using `wgpu` for graphics rendering and `winit` for window management.

## Features

- Renders a flat plane divided into chunks.
- Utilizes `wgpu` for efficient GPU rendering.
- Window management with `winit`.
- Basic camera controls for navigating the scene.
- Chunk-based terrain generation with support for flat and Perlin noise-based terrains.
- Modular design for easy extension and modification.

## Next Steps

Future improvements could include:
- Implementing heightmap loading from image files.
- Adding lighting and shading effects.
- Adding texture mapping to the heightmap.
- Incorporating GUI elements for better user interaction.

### Development timeline

## First Pass:

The first pass was implemented in two stages:
1. **Basic Setup**: Created a window using `winit` and set up a rendering loop with `wgpu`. This stage involved initializing the GPU, creating a swap chain, and rendering a simple 3D plane. *(commit `3d58d74c70df3ca1c6eb8f75c59eeb0556a16a4c`)*
2. **Chunking**: Divided the plane into smaller chunks to optimize rendering performance. Each chunk is rendered separately, allowing for better management of large heightmaps. *(commit `b286658a382ea06016a26bf3d3420fa559ce5702`)*

This first pass was implemented with the GLM-4.7 assistant on OpenCode in roughly one hour, *(see conversation: https://opncd.ai/share/HhULZY4C)*

![](./docs/Screenshot%202025-12-27%20123805.png)
*Screenshot of the heightmap renderer in action.*

### Second Pass:

The second pass focused on enhancing the terrain generation capabilities:
1. **Terrain Generation**: Implemented a basic terrain generator that creates flat terrain chunks. This involved defining a `TerrainGenerator` trait and a `FlatTerrainGenerator` struct that generates height data for each chunk. *(commit `7b62d9769d9b6b0e52c9bb8c2491b55b3e957471`)*
2. **Noise-based Terrain**: Added a `PerlinTerrainGenerator` that generates more complex terrain using Perlin noise. This allows for more realistic heightmaps and varied landscapes. *(commit `b15b506a3a0d2dc02e1fc958d3a711ae08b5d7d0`)*

This second pass was mostly implemented by hand, with some assistance from the GLM-4.7 model on OpenCode for chunk grid simplification. 

![](./docs/Screenshot%202025-12-27%20132051.png)
*Screenshot showcasing the Perlin noise-generated terrain.*

### Third Pass:

The third pass completely reset the entire codebase to implement a solution using the Bevy game engine:
1. **Bevy Integration**: Rewrote the renderer using the Bevy game engine to leverage its ECS architecture and built-in rendering capabilities. This involved setting up Bevy systems, components, and resources to manage the heightmap rendering process. *(commit `aacdf31ed23cbea22fcfec941939bf6a8deb5242`)*
2. **Camera and Controls**: Implemented a top-down camera system with basic controls for navigating the heightmap. This included setting up camera movement and zooming functionalities. *(commit `aacdf31ed23cbea22fcfec941939bf6a8deb5242`)*
3. **Modular codebase**: Structured the codebase into separate modules for terrain generation, rendering, and camera control to enhance maintainability and extensibility. *(commit `34b4fea9e01f915b56ec4fa8352792f72f638fbc`)*

This third pass was implemented entirely with the assistance of GPT-5.2 on GitHub Copilot.

![](./docs/Screenshot%202026-01-06%20145802.png)
*Screenshot of the Bevy-based heightmap renderer.*

### Fourth Pass:

The fourth pass involved object placement and data-driven architecture:
1. **Object Placement System**: Developed a system for placing objects on the terrain, allowing users to interactively add and manipulate objects within the heightmap environment. *(commit `1e3407902f8936a7749e6ec24ca9e2fec983b582`)*
2. **Data-Driven Design**: Refactored the codebase to adopt a data-driven approach, enabling easier configuration and extension of terrain and object properties. *(commit `8fd6af3a801c827ce5c7b11aed64e2c1cb608259`)*

This fourth pass was also implemented entirely with the assistance of GPT-5.2 on GitHub Copilot.

![](./docs/Screenshot%202026-01-06%20194258.png)
*Screenshot showcasing the object placement system.*

### Fifth Pass:

The fifth pass focused on implement freeform and rotational object placement:
1. **Freeform Object Placement**: Enhanced the object placement system to allow for freeform placement of objects on the terrain, providing more flexibility in positioning. *(commit `e037a653db87e6328376a386a7556c70ada6ed19`)*
2. **Rotational Placement**: Added functionality for rotating objects during placement, enabling users to customize the orientation of objects on the heightmap. *(commit `e037a653db87e6328376a386a7556c70ada6ed19`)*

This fifth pass was again implemented entirely with the assistance of GPT-5.2 on GitHub Copilot.

![](./docs/Screenshot%202026-01-06%20201904.png)
*Screenshot demonstrating freeform and rotational object placement.*

## License

This project is dual-licensed under the MIT License and the Apache License 2.0. See the [`LICENSE-MIT`](LICENSE-MIT) and [`LICENSE-APACHE`](LICENSE-APACHE) files for details.
Feel free to use and modify the code as per the terms of these licenses.


## Asset Credits

- 2 Story House: (https://skfb.ly/o8Xus) by Designed By Jonathan is licensed under Creative Commons Attribution (http://creativecommons.org/licenses/by/4.0/).