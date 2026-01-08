# Development Plan

This document outlines future features and improvements organized by category. These are **not blocking issues**—they are enhancements planned after the codebase reaches a stable state and critical networking foundations are in place.

See `docs/reviews/08-01-2026-c.md` for critical issues that should be addressed now.

---

## Phase 1: Main Menu and Mod Selection (Future)

### Main Menu
- [ ] Add main menu UI with new game/load game/settings/exit options.
- [ ] Allow player to select mods before loading a save or starting new game.
- [ ] Display available mods and currently selected mod set.

### Mod Selection
- [ ] Implement mod enable/disable UI before game load.
- [ ] Store selected mod set with save file.
- [ ] Validate mod compatibility when loading save files.

---

## Phase 2: Networking Foundation (Future)

### Server/Client Architecture
- [ ] Create separate `server` and `client` binaries (or shared code with feature flags).
- [ ] Implement networking transport (WebSocket or UDP).
- [ ] Add message routing between server and clients.

### State Replication
- [ ] Implement server-authoritative object placement.
- [ ] Send object state updates to clients.
- [ ] Handle client prediction and server reconciliation.

### Mod Sync
- [ ] Server sends list of mod URLs to connecting clients.
- [ ] Clients download mods from URLs before joining.
- [ ] Verify mod integrity and version matching.

### Gameplay Networking
- [ ] Implement networked placement validation.
- [ ] Handle overlapping placements and conflicts.
- [ ] Broadcast destruction events to all clients.

---

## Phase 3: Polish and Expansion (Future)

### Settings and Customization
- [ ] Data-driven UI configuration (colors, layouts, fonts).
- [ ] Keybinding customization.
- [ ] Game settings (camera sensitivity, etc.).
- [ ] Persistent settings storage.

### User Experience
- [ ] Improve error messages and fallbacks for asset/mod failures.
- [ ] Add logging and profiling infrastructure.
- [ ] Performance diagnostics (FPS, entity count, etc.).

### Content
- [ ] Support multiple mod directories (official, community, user mods).
- [ ] Mod manager UI (install, enable, disable, delete).

---

## Non-Goals

The following are **not planned** and conflict with core design:

- ❌ **Hot-reload during gameplay** – Causes desync, save corruption, player confusion.

---

## Estimated Timelines (After Core Foundations Complete)

| Phase | Effort | Timeline |
|-------|--------|----------|
| Main Menu & Mod Selection | 1–2 weeks | Q2 2026 |
| Networking Foundation | 3–4 weeks | Q3 2026 |
| Polish and Expansion | 2–3 weeks | Q3–Q4 2026 |

**Note:** These are estimates with AI assistance and may shift based on unforeseen challenges.

---

## Priority Alignment

**Critical (Complete First - In Progress):**
1. Serialization of game state
2. GameAction message abstraction
3. System scope markers (ServerOnly/ClientOnly/Shared)
4. Error handling improvements

**Important (After Critical):**
1. Main menu and mod selection UI
2. Networking foundation
3. Mod sync for multiplayer

**Nice-to-Have (Continuous):**
1. Settings and customization
2. Performance optimization
3. Additional polish

---
