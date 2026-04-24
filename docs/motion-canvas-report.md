# Motion Canvas Report

## 1. What Motion Canvas is

Motion Canvas is a free, open-source toolchain for building animations in code. Its official docs and repository describe it as two pieces: a TypeScript library that uses generators to program animations, and an editor that provides a real-time preview. It is positioned mainly for informative vector animations and voice-over-synced work, not as a replacement for a traditional video editor.

Sources: [Docs: Introduction](https://motioncanvas.io/docs/), [Repo README](https://github.com/motion-canvas/motion-canvas)

## 2. Main concepts / architecture

- A video is defined as a project created with `makeProject`, which points to one or more scenes.
- Scenes are usually created with `makeScene2D(function* (view) { ... })`; the generator controls timing and sequencing.
- The scene graph is a tree of `Node` objects rooted at the scene view. Motion Canvas uses custom JSX syntax for readability, but the docs explicitly note it is not React and has no virtual DOM.
- Properties are reactive signals; the same signal API is used for reading, setting, and tweening values over time.
- Effects exist for non-lazy reactive side effects, and time events let the editor expose timeline markers that can be adjusted without hard-coding exact wait durations.
- The official monorepo is split into packages such as `core`, `2d`, `ui`, `player`, `create`, and `vite-plugin`.

Sources: [Quickstart](https://motioncanvas.io/docs/quickstart/), [Animation flow](https://motioncanvas.io/docs/flow/), [Scene hierarchy](https://motioncanvas.io/docs/hierarchy/), [Signals](https://motioncanvas.io/docs/signals/), [Effects](https://motioncanvas.io/docs/effects/), [Time Events](https://motioncanvas.io/docs/time-events/), [Repo README](https://github.com/motion-canvas/motion-canvas)

## 3. Install and runtime model

- Official quickstart requires Node.js 16+.
- New projects are scaffolded with `npm init @motion-canvas@latest`, then `npm install`, then `npm start`.
- The editor runs locally at `http://localhost:9000/`.
- Projects are configured through Vite using `@motion-canvas/vite-plugin`.
- Rendering writes output under `./output`; the built-in exporter produces image sequences, and an optional `@motion-canvas/ffmpeg` exporter can generate finished video files.
- Inference from the docs: the normal authoring loop is a local Vite + browser workflow with live preview, then export to frames or video.

Sources: [Quickstart](https://motioncanvas.io/docs/quickstart/), [Configuration](https://motioncanvas.io/docs/configuration/), [Rendering](https://motioncanvas.io/docs/rendering/), [Image sequence exporter](https://motioncanvas.io/docs/rendering/image-sequence/), [Video (FFmpeg) exporter](https://motioncanvas.io/docs/rendering/video/)

## 4. Strengths

- Strong fit for programmatic, repeatable animation where version control and code review matter.
- Real-time preview and hot reload reduce the edit-run loop.
- The generator model is expressive for sequencing, concurrency, and reusable animation logic.
- Signals and the node tree make it suitable for reactive, data-driven visuals.
- Time events directly address the common pain of syncing animation timing to narration.
- Built-in image-sequence export is a practical bridge into standard video finishing tools.

Sources: [Quickstart](https://motioncanvas.io/docs/quickstart/), [Animation flow](https://motioncanvas.io/docs/flow/), [Signals](https://motioncanvas.io/docs/signals/), [Time Events](https://motioncanvas.io/docs/time-events/), [Image sequence exporter](https://motioncanvas.io/docs/rendering/image-sequence/)

## 5. Limitations or risks

- The project explicitly says it is not meant to replace traditional video editing software.
- It assumes comfort with TypeScript/JavaScript, Node, npm, and Vite; that is a real adoption barrier for non-developer editors.
- The workflow looks optimized for vector / motion-graphics style output, not footage-heavy editorial work.
- Some extension areas are marked experimental in the docs, so teams depending on those APIs should expect churn.
- Browser/runtime details matter in places; for example, docs note some export behavior depends on browser capabilities.
- On the GitHub releases page I checked, the latest stable release shown was `v3.17.2` dated `2024-12-14`, so teams with strict maintenance requirements should verify current activity before committing heavily.

Sources: [Docs: Introduction](https://motioncanvas.io/docs/), [Configuration](https://motioncanvas.io/docs/configuration/), [Image sequence exporter](https://motioncanvas.io/docs/rendering/image-sequence/), [GitHub releases](https://github.com/motion-canvas/motion-canvas/releases)

## 6. Whether it looks useful for animation / video workflows

Yes, with a narrow but real sweet spot.

It looks useful for developer-led animation workflows, especially explanatory videos, technical visuals, code animations, diagram motion, reusable branded sequences, and projects where animation logic benefits from abstraction, parameterization, or source control. It also looks useful when the intended finish is an image sequence or generated video handed off to a standard editor.

It looks less useful as the primary tool for general-purpose video editing, footage assembly, or teams that need a timeline-first UI for non-coders. In short: promising for code-native motion graphics; not a broad replacement for conventional post-production software.
