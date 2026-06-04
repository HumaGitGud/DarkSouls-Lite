# Dark Souls Lite

A retro low-poly first-person dark fantasy game built with **Rust + Bevy**. Roam a moonlit field of fog, embers, gravestones and ruins, rest at bonfires, fight roaming undead, and storm a torch-lit castle to slay a dragon boss — while the Eye of Sauron watches from its tower.

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
| `Shift` | Sprint (uses stamina) |
| `Left Ctrl` | **Dodge roll** (i-frames, costs stamina) |
| `Space` | Jump |
| `Mouse` | Look around |
| `Left Click` | Grab cursor / use selected item (sword / gun / drink potion) |
| `Right Click (hold)` | **Lightning** (uses mana) |
| `Scroll Wheel` | Cycle hotbar item |
| `R` | **Rest at a nearby bonfire** (full heal, refill potions/mana) |
| `Esc` | Release cursor |

## Combat & survival

- **Sword** — flashes enemies red + knockback. **Lightning** — drains mana; enemies flash white/blue while shocked.
- **Glock** (hitscan beam) and **Rocket Launcher** (nuke mushroom) found in the castle.
- **Dodge roll** grants brief invulnerability — time it against attacks.
- **Hearts / Stamina / Mana** bars top-left; a **Minecraft-style hotbar** at the bottom with potion counts.
- **Red potions** heal 2 hearts, **blue potions** refill mana (drink animation).
- **Bonfires** (one at spawn, one outside the castle) restore everything when you press `R`.
- **Death** → a **"YOU DIED"** screen → respawn. **Slaying the dragon** → a **"VICTORY ACHIEVED"** screen.

## Enemies

| Enemy | Behavior |
|---|---|
| **Skeletons** | Patrol, then chase and jab with couched spears |
| **Witches** | Cast purple magic missiles from range |
| **Orcs** | Plant their feet and slam a hammer (dirt debris shockwave) |
| **Bats** | Fast flyers that swarm in groups of three |
| **Black Knights** | Rare blackened-plate elites with a greatsword — tanky and quick |
| **Eye of Sauron** | Spire tower that locks on and fires a continuous lightning beam |
| **Dragon (boss)** | See below — health bar appears when engaged |

### The Dragon — two phases
- **Phase 1**: grounded, lobs single huge fireballs.
- **Phase 2 (≤50% HP)**: roars at the sky (slows you 30%), stomps **3 expanding shockwaves** (jump to dodge), takes flight, then soars erratically and periodically swoops in for a **dragonbreath** — a continuous red laser with a yellow core that sweeps up toward you (near-instant death).

## World

- Large moonlit map ringed by **natural jagged snow-capped mountains** and foothills.
- A pine forest, a grand **lit castle** (banners, throne, royal carpet, boss fog gate) with the dragon inside.
- The **Eye of Sauron spire** looming over the far corner.
- Dark Souls atmosphere: **bonfires, gravestones, ruined pillars, drifting fog, rising embers, and soapstone messages**.
- **Solid collision** on walls, towers, pillars, ruins, the spire, and trees.

## Stack

[Rust](https://www.rust-lang.org/) + [Bevy 0.15](https://bevyengine.org/) — pure code, no editor, low-poly primitives only.
