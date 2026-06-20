# Monkey Miner

Monkey Miner is a small 3D maze game built with Rust and Bevy. You play a miner monkey trapped inside a randomly generated, Windows 3D Maze-inspired labyrinth. Find the smiley exit, collect what you can, mine shortcuts, and do not let the rat catch you.

![Monkey Miner gameplay](docs/screenshots/gameplay.png)

## Current Game

Each run starts in one corner of a connected maze. The exit is in the far corner, but the path is blocked by a locked door. The key spawns deeper in the maze on the route to the exit, away from both the player spawn and the rat. Treasures, chests, ore-deposit walls, a compass, and a treasure map are scattered through the maze. Mine internal walls to create shortcuts or escape routes, but boundary walls are unbreakable.

The maze is randomized every time you restart a run. The rooms are intentionally roomy, but fog, the roof, the rat, and limited resources keep the route tense.

```mermaid
flowchart LR
    Start[Start in corner] --> Explore[Explore maze]
    Explore --> Collect[Collect key, treasure, compass, map]
    Explore --> Mine[Mine shortcut and ore-deposit walls]
    Mine --> Ore[Pick up ore nuggets]
    Ore --> Upgrade[Upgrade mining or speed]
    Collect --> Markers[Earn trail markers from treasure]
    Collect --> Door[Unlock exit door]
    Door --> Exit[Reach smiley exit]
    Explore --> Rat[Stay ahead of the rat]
    Rat --> Caught[Caught: score finalizes]
    Exit --> Win[Escape: score finalizes]
    Caught --> Restart[Press R for new maze]
    Win --> Restart
```

## Example Maze Layout

The real maze is larger and generated at runtime, but a run starts with this kind of structure. The player begins near one corner, the exit is far away, the key is on the route to the door, and navigation pickups help only after you find them.

```text
#############
#P..M#.....T#
###..#.###..#
#....#O..#..#
#.######.#D##
#T....C#.#G.#
#.####.#.####
#....#.#..K.#
####.#.###R.#
#..T.#......#
#############
```

Legend:

| Mark | Meaning |
| --- | --- |
| `#` | Wall |
| `.` | Walkable corridor |
| `P` | Player start |
| `K` | Key on the route to the exit |
| `D` | Locked door before the exit |
| `G` | Smiley exit goal |
| `R` | Rat enemy |
| `T` | Treasure |
| `C` | Chest |
| `O` | Ore-deposit wall |
| `M` | Treasure map pickup |
| Compass | Compass pickup that unlocks the exit compass UI |

## Controls

| Input | Action |
| --- | --- |
| `W` / `S` | Move forward / backward |
| `A` / `D` | Strafe left / right |
| Mouse drag | Orbit camera |
| `Q` / `E` | Turn camera left / right |
| `T` | Drop a trail marker in the current maze cell, if you have one |
| `F` | Mine the wall you are facing |
| `U` | Open / close upgrade menu |
| `M` | Open / close treasure map after finding it |
| `Up` / `Down` | Select upgrade while the menu is open |
| `Enter` / `Space` | Buy selected upgrade while the menu is open |
| `Esc` | Close upgrade menu |
| `R` | Restart after winning or getting caught |

## Mining And Upgrades

Mining consumes energy. If energy hits zero, upgrade mining with ore to keep opening shortcuts. Ore comes from chests and special ore-deposit walls. Normal walls do not drop ore, and boundary walls cannot be mined.

Ore-deposit walls burst into bouncing nuggets. Nuggets settle before they can be collected, and bounce off maze walls instead of passing through them.

Ore can also be spent on movement speed if you want a faster escape route instead.

Upgrade rules:

| Upgrade | Cost | Effect |
| --- | --- | --- |
| Mining energy | Current mining level in ore | Mining level +1, max energy +1, energy refills |
| Movement speed | Current speed level in ore | Speed level +1, movement speed increases |

## Navigation Tools

You start with 5 trail markers. Dropping a marker spends 1 marker and marks the current cell. Each loose treasure adds 2 more markers.

The compass and treasure map are world pickups, not free HUD features:

| Tool | Behavior |
| --- | --- |
| Compass | Shows an exit arrow after you find the compass pickup. The arrow is red until you have the key, then green. |
| Treasure map | Toggle with `M` after pickup. It points to the key first, then to the compass if the key is collected but the compass is still missing. |

## Scoring

Score updates during the run and finalizes when you escape or get caught.

| Event | Points |
| --- | ---: |
| Treasure | 100 |
| Ore held | 25 |
| Key held | 20 |
| Door unlocked | 150 |
| Chest opened | 75 |
| Wall mined | 10 |
| Escape bonus | 1000 |
| Time | -1 point per second |

The HUD shows current score and best score for the current process.

## Run From Source

Requirements:

| Tool | Version |
| --- | --- |
| Rust | Stable, from `rust-toolchain.toml` |
| Cargo | Installed with Rust |

Run the development build:

```sh
cargo run
```

Run the release build through the local launcher:

```sh
bin/monkey-miner
```

The launcher builds `target/release/monkey-miner` if it does not already exist, then runs it.

You can also use Make targets if `make` is installed:

```sh
make run
make check
make package
```

## Build A Packaged Executable

Create a local macOS/Linux package with the executable and assets copied together:

```sh
scripts/build-release.sh
```

The script writes a platform-specific directory under `dist/`, for example:

```sh
dist/monkey-miner-darwin-x86_64/monkey-miner
```

Run the packaged executable from inside that directory:

```sh
cd dist/monkey-miner-darwin-x86_64
./monkey-miner
```

The executable also looks for `assets/` next to itself, so launching it by full path works too:

```sh
dist/monkey-miner-darwin-x86_64/monkey-miner
```

### Windows `.exe`

Build the Windows executable on a Windows machine with Rust installed:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-release.ps1
```

That creates:

```powershell
dist\monkey-miner-windows-x86_64\monkey-miner.exe
```

Run it with:

```powershell
dist\monkey-miner-windows-x86_64\monkey-miner.exe
```

There is also a Makefile target for environments that have both `make` and PowerShell:

```sh
make package-windows
```

For this Bevy prototype, native packaging is the reliable path. Cross-compiling a Windows `.exe` from macOS is possible in theory, but it usually requires extra linker and Windows SDK setup that is not worth baking into this repo yet.

`dist/` is ignored because it is generated output and the release binary is large.

## Project Layout

| Path | Purpose |
| --- | --- |
| `src/main.rs` | Gameplay systems, camera, HUD, pickups, rat, mining, scoring |
| `src/maze.rs` | Maze generation, maze geometry, wall data, fog cells |
| `assets/images/` | Pixel art textures and sprites |
| `assets/audio/` | Short 8-bit sound effects |
| `assets/icons/` | App icon source plus `.ico` and `.icns` outputs |
| `bin/monkey-miner` | Local release launcher |
| `scripts/build-release.sh` | Packaged executable builder |
| `scripts/build-release.ps1` | Windows `.exe` package builder |
| `Makefile` | Convenience targets for run/check/package |
| `docs/screenshots/` | README screenshots |

## Notes

This is still a prototype. The core loop works: explore, collect, place markers, mine, upgrade, unlock, escape, or restart after the rat catches you.
