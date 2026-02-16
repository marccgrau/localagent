---
sidebar_position: 14
---

# LocalGPT Gen (Experimental)

:::warning Experimental
LocalGPT Gen is an early-stage experimental feature. Scene output quality depends heavily on the LLM's spatial reasoning. Consider it a proof of concept.
:::

**LocalGPT Gen** is a built-in 3D scene generation mode. You type natural language, and the AI composes 3D scenes from geometric primitives — spheres, cubes, cylinders, tori — with full material control, lighting, and camera positioning. All inside the same single Rust binary, powered by [Bevy](https://bevyengine.org/).

## Demo Videos

<iframe width="100%" height="400" src="https://www.youtube.com/embed/n18qnSDmBK0" title="LocalGPT Gen Demo" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowfullscreen></iframe>

<br/>

<iframe width="100%" height="400" src="https://www.youtube.com/embed/cMCGW7eMUNE" title="LocalGPT Gen Demo" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowfullscreen></iframe>

## Installation

```bash
# Preferred — standalone binary
cargo install localgpt-gen

# Or enable the gen feature on the main crate
cargo install localgpt --features gen
```

## Usage

```bash
localgpt gen "create a heart outline with spheres and cubes"
```

The agent receives your prompt and iteratively builds a 3D scene — spawning shapes, adjusting materials, positioning the camera, and taking screenshots to course-correct.

## Gen Tools

The gen agent has access to 11 specialized tools:

| Tool | Description |
|------|-------------|
| `gen_spawn_primitive` | Spawn geometric primitives (sphere, cube, cylinder, torus, etc.) |
| `gen_modify_entity` | Modify an existing entity's transform, material, or visibility |
| `gen_delete_entity` | Remove an entity from the scene |
| `gen_set_camera` | Position and orient the camera |
| `gen_set_light` | Configure scene lighting |
| `gen_spawn_mesh` | Spawn custom mesh geometry |
| `gen_screenshot` | Capture a screenshot for visual feedback |
| `gen_list_entities` | List all entities in the scene |
| `gen_clear_scene` | Remove all spawned entities |
| `gen_set_background` | Set the background/clear color |
| `gen_get_entity` | Get detailed info about a specific entity |

## Architecture

Bevy requires ownership of the main thread (macOS windowing/GPU requirement), so LocalGPT Gen uses a split-thread architecture:

- **Main thread** — Bevy engine runs the render loop and processes scene commands
- **Background thread** — Tokio runtime runs the agent loop, making LLM calls and issuing tool commands
- **Communication** — Async mpsc channels bridge the two threads

```
┌─────────────────────┐     mpsc channels     ┌─────────────────────┐
│    Main Thread      │◄─────────────────────►│  Background Thread  │
│                     │                        │                     │
│  Bevy Engine        │   ToolRequest ──►      │  Tokio Runtime      │
│  - Rendering        │   ◄── ToolResult       │  - Agent Loop       │
│  - Scene Graph      │                        │  - LLM API Calls    │
│  - Window/GPU       │                        │  - Tool Execution   │
└─────────────────────┘                        └─────────────────────┘
```

## Limitations

- Scene quality depends on the LLM's spatial reasoning ability
- Only geometric primitives are supported (no imported 3D models yet)
- Requires a GPU-capable display for rendering
- The `gen` feature adds Bevy as a dependency, significantly increasing compile time
