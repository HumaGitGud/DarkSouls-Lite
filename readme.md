# Dark Slop Souls 

A retro low-poly first-person dark fantasy game built with **Rust + Bevy**. Roam a moonlit field ringed by snowy mountains, fight roaming undead, and storm a torch-lit castle to face a dragon boss — while the Eye of Sauron watches from its tower. Claude Opus 4.8 was heavily utilized as part of the exercise to agentic developing.

## Prerequisites

- **Rust** — install from [rust-lang.org/tools/install](https://www.rust-lang.org/tools/install)
- **Windows**: Visual Studio C++ build tools (installed automatically by the Rust installer)

## Build & Run

```bash
cargo run
```

First build compiles Bevy (~5–10 min); later builds are ~15s.

## Controls

| Key / Button | Action |
|---|---|
| `W A S D` | Move |
| `Shift` | Sprint |
| `Space` | Jump |
| `Mouse` | Look around |
| `Left Click` | Grab cursor / **Sword swing** |
| `Right Click (hold)` | **Lightning** (Palpatine-style forked bolts) |
| `Esc` | Release cursor |

## Your Character

- **Right hand** — a detailed knight's sword (steel blade, gold guard, crimson grip)
- **Left hand** — an open hand holding a **glowing spell tome** that casts lightning

## Combat

- **Sword** — 1 damage per swing; enemies flash red and get knocked back
- **Lightning** — 2 damage/sec in a forward cone; enemies crackle with a **blue shock aura**, and a white flash lights the area
- **Health** — 5 hearts (top-left). Taking a hit flashes red screen corners and knocks you back
- **Death** — at 0 hearts a **"YOU'RE DEAD"** screen appears with a **Respawn** button that restarts the run

## Enemies

| Enemy | Behavior |
|---|---|
| **Skeletons** | Patrol, then chase and jab with couched spears (5 HP) |
| **Witches** | Fast-chasing sorceresses (4 HP) |
| **Knights** | Slow but tanky armored foes (7 HP) |
| **Bats** | Fast flyers that swarm in groups of 3 (2 HP) |
| **Dragon** | Boss at the castle gate — shoots 3-fireball bursts (20 HP) |
| **Eye of Sauron** | Spire tower that locks on and fires a continuous lightning beam when you get close |

## World

- Large open map enclosed by **chunky snow-capped mountains**
- A pine forest, a **lit castle** with towers/keep/braziers, and the **Eye of Sauron spire**
- **Solid collision** on walls, towers, pillars, the spire, and trees
- Stylized night sky: square moon, scattered stars, head-bob, dodge-jump feel

## Stack

[Rust](https://www.rust-lang.org/) + [Bevy 0.15](https://bevyengine.org/) — pure code, no editor, low-poly primitives only.
