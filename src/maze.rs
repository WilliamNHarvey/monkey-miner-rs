use bevy::prelude::*;
use rand::seq::SliceRandom;

pub const MAZE_COLUMNS: usize = 18;
pub const MAZE_ROWS: usize = 18;
pub const CELL_SIZE: f32 = 120.0;
pub const WALL_HEIGHT: f32 = 90.0;
pub const WALL_THICKNESS: f32 = 10.0;
pub const FLOOR_THICKNESS: f32 = 2.5;
const FOG_HEIGHT: f32 = WALL_HEIGHT - 5.0;
const VISUAL_WALL_LENGTH: f32 = CELL_SIZE - WALL_THICKNESS;
const MIN_FORK_CELLS: usize = 6;
const MIN_FIRST_FORK_DISTANCE: usize = 5;
const MAX_FIRST_FORK_DISTANCE: usize = 8;
const SECONDARY_FORK_CORRIDOR_LENGTH: usize = 3;
const MIN_WALL_SPAWN_OFFSET: usize = 4;
const MIN_WALL_SPAWN_EXIT_DISTANCE: usize = 8;

#[derive(Component)]
pub struct Wall;

#[derive(Component)]
pub struct Floor;

#[derive(Component)]
pub struct Roof;

#[derive(Component)]
pub struct FogCell {
    pub x: usize,
    pub z: usize,
    pub material: Handle<StandardMaterial>,
}

#[derive(Component, Clone, Copy)]
pub struct WallCollider {
    pub center: Vec2,
    pub half_extents: Vec2,
}

#[derive(Component, Clone, Copy)]
pub struct WallSegment {
    pub cell: (usize, usize),
    pub direction: Direction,
    pub mineable: bool,
    pub hits_remaining: u8,
    pub max_hits: u8,
    pub ore_deposit: bool,
}

#[derive(Resource, Clone)]
pub struct MazeMap {
    pub cells: [[MazeCell; MAZE_COLUMNS]; MAZE_ROWS],
}

#[derive(Clone, Copy)]
pub struct MazeCell {
    pub walls: [bool; 4],
}

#[derive(Clone, Copy)]
struct GeneratorCell {
    visited: bool,
    walls: [bool; 4],
}

impl Default for GeneratorCell {
    fn default() -> Self {
        Self {
            visited: false,
            walls: [true; 4],
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    pub fn index(self) -> usize {
        match self {
            Self::North => 0,
            Self::East => 1,
            Self::South => 2,
            Self::West => 3,
        }
    }

    pub fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::East => Self::West,
            Self::South => Self::North,
            Self::West => Self::East,
        }
    }

    pub fn delta(self) -> (isize, isize) {
        match self {
            Self::North => (0, 1),
            Self::East => (1, 0),
            Self::South => (0, -1),
            Self::West => (-1, 0),
        }
    }
}

pub fn create_maze(
    commands: &mut Commands,
    wall_texture: Handle<Image>,
    ore_wall_texture: Handle<Image>,
    hard_wall_textures: [Handle<Image>; 3],
    floor_texture: Handle<Image>,
    roof_texture: Handle<Image>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> (Vec3, MazeMap) {
    let (mut maze, start_cell) = generate_connected_maze();
    seal_extra_exit_passages(&mut maze, start_cell);
    repair_maze_connectivity(&mut maze, start_cell);
    info!("Maze has {} fork cells", fork_cell_count(&maze));
    // Temporarily uncomment to refresh the README example maze layout.
    // info!("Generated maze layout:\n{}", debug_maze_ascii(&maze, start_cell));
    let maze_map = MazeMap { cells: maze };
    commands.insert_resource(maze_map.clone());

    let wall_material = materials.add(StandardMaterial {
        base_color_texture: Some(wall_texture),
        base_color: Color::WHITE,
        perceptual_roughness: 0.85,
        ..default()
    });
    let ore_wall_material = materials.add(StandardMaterial {
        base_color_texture: Some(ore_wall_texture),
        base_color: Color::WHITE,
        perceptual_roughness: 0.85,
        ..default()
    });
    let hard_wall_materials = hard_wall_textures.map(|texture| {
        materials.add(StandardMaterial {
            base_color_texture: Some(texture),
            base_color: Color::WHITE,
            perceptual_roughness: 0.9,
            ..default()
        })
    });
    let floor_material = materials.add(StandardMaterial {
        base_color_texture: Some(floor_texture),
        perceptual_roughness: 0.9,
        ..default()
    });
    let roof_material = materials.add(StandardMaterial {
        base_color_texture: Some(roof_texture),
        base_color: Color::srgba(1.0, 1.0, 1.0, 0.25),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    let floor_mesh = meshes.add(Cuboid::new(CELL_SIZE, FLOOR_THICKNESS, CELL_SIZE));
    let roof_mesh = meshes.add(Cuboid::new(CELL_SIZE, FLOOR_THICKNESS, CELL_SIZE));
    let fog_mesh = meshes.add(Cuboid::new(CELL_SIZE, FOG_HEIGHT, CELL_SIZE));
    let horizontal_wall_mesh =
        meshes.add(Cuboid::new(VISUAL_WALL_LENGTH, WALL_HEIGHT, WALL_THICKNESS));
    let vertical_wall_mesh =
        meshes.add(Cuboid::new(WALL_THICKNESS, WALL_HEIGHT, VISUAL_WALL_LENGTH));
    let wall_post_mesh = meshes.add(Cuboid::new(WALL_THICKNESS, WALL_HEIGHT, WALL_THICKNESS));
    let ore_deposit_edges = choose_ore_deposit_edges(&maze, start_cell);
    info!("Spawned {} ore deposit walls", ore_deposit_edges.len());

    for z in 0..MAZE_ROWS {
        for x in 0..MAZE_COLUMNS {
            let center = cell_center(x, z);
            commands.spawn((
                Mesh3d(floor_mesh.clone()),
                MeshMaterial3d(floor_material.clone()),
                Transform::from_xyz(center.x, -FLOOR_THICKNESS * 0.5, center.y),
                Floor,
            ));
            commands.spawn((
                Mesh3d(roof_mesh.clone()),
                MeshMaterial3d(roof_material.clone()),
                Transform::from_xyz(center.x, WALL_HEIGHT + FLOOR_THICKNESS * 0.5, center.y),
                Roof,
            ));

            let fog_material = materials.add(StandardMaterial {
                base_color: Color::srgba(0.0, 0.0, 0.0, 0.9),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            });
            commands.spawn((
                Mesh3d(fog_mesh.clone()),
                MeshMaterial3d(fog_material.clone()),
                Transform::from_xyz(center.x, FOG_HEIGHT * 0.5, center.y),
                FogCell {
                    x,
                    z,
                    material: fog_material,
                },
            ));

            let cell = maze[z][x];
            if cell.walls[Direction::North.index()] {
                let hard = hard_wall_near_exit((x, z), Direction::North);
                let ore_deposit = !hard && ore_deposit_edges.contains(&((x, z), Direction::North));
                spawn_wall(
                    commands,
                    horizontal_wall_mesh.clone(),
                    if hard {
                        hard_wall_materials[0].clone()
                    } else if ore_deposit {
                        ore_wall_material.clone()
                    } else {
                        wall_material.clone()
                    },
                    Vec2::new(center.x, center.y + CELL_SIZE * 0.5),
                    Vec2::new((CELL_SIZE + WALL_THICKNESS) * 0.5, WALL_THICKNESS * 0.5),
                    (x, z),
                    Direction::North,
                    z < MAZE_ROWS - 1,
                    if hard { 5 } else { 1 },
                    ore_deposit,
                    false,
                );
            }
            if cell.walls[Direction::West.index()] {
                let hard = hard_wall_near_exit((x, z), Direction::West);
                let ore_deposit = !hard && ore_deposit_edges.contains(&((x, z), Direction::West));
                spawn_wall(
                    commands,
                    vertical_wall_mesh.clone(),
                    if hard {
                        hard_wall_materials[0].clone()
                    } else if ore_deposit {
                        ore_wall_material.clone()
                    } else {
                        wall_material.clone()
                    },
                    Vec2::new(center.x - CELL_SIZE * 0.5, center.y),
                    Vec2::new(WALL_THICKNESS * 0.5, (CELL_SIZE + WALL_THICKNESS) * 0.5),
                    (x, z),
                    Direction::West,
                    x > 0,
                    if hard { 5 } else { 1 },
                    ore_deposit,
                    true,
                );
            }
            if z == 0 && cell.walls[Direction::South.index()] {
                spawn_wall(
                    commands,
                    horizontal_wall_mesh.clone(),
                    wall_material.clone(),
                    Vec2::new(center.x, center.y - CELL_SIZE * 0.5),
                    Vec2::new((CELL_SIZE + WALL_THICKNESS) * 0.5, WALL_THICKNESS * 0.5),
                    (x, z),
                    Direction::South,
                    false,
                    1,
                    false,
                    false,
                );
            }
            if x == MAZE_COLUMNS - 1 && cell.walls[Direction::East.index()] {
                spawn_wall(
                    commands,
                    vertical_wall_mesh.clone(),
                    wall_material.clone(),
                    Vec2::new(center.x + CELL_SIZE * 0.5, center.y),
                    Vec2::new(WALL_THICKNESS * 0.5, (CELL_SIZE + WALL_THICKNESS) * 0.5),
                    (x, z),
                    Direction::East,
                    false,
                    1,
                    false,
                    true,
                );
            }
        }
    }
    spawn_wall_posts(commands, wall_post_mesh, wall_material.clone(), &maze);

    let start = cell_center(start_cell.0, start_cell.1);
    (Vec3::new(start.x, 0.0, start.y), maze_map)
}

fn spawn_wall_posts(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    maze: &[[MazeCell; MAZE_COLUMNS]; MAZE_ROWS],
) {
    let width = MAZE_COLUMNS as f32 * CELL_SIZE;
    let depth = MAZE_ROWS as f32 * CELL_SIZE;
    for z in 0..=MAZE_ROWS {
        for x in 0..=MAZE_COLUMNS {
            if !wall_touches_vertex(x, z, maze) {
                continue;
            }
            let position = Vec2::new(
                x as f32 * CELL_SIZE - width * 0.5,
                z as f32 * CELL_SIZE - depth * 0.5,
            );
            commands.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(position.x, WALL_HEIGHT * 0.5, position.y),
                Wall,
            ));
        }
    }
}

fn wall_touches_vertex(x: usize, z: usize, maze: &[[MazeCell; MAZE_COLUMNS]; MAZE_ROWS]) -> bool {
    (x > 0 && horizontal_wall_at(x - 1, z, maze))
        || (x < MAZE_COLUMNS && horizontal_wall_at(x, z, maze))
        || (z > 0 && vertical_wall_at(x, z - 1, maze))
        || (z < MAZE_ROWS && vertical_wall_at(x, z, maze))
}

#[allow(dead_code)]
fn debug_maze_ascii(maze: &[[MazeCell; MAZE_COLUMNS]; MAZE_ROWS], start: (usize, usize)) -> String {
    let width = MAZE_COLUMNS * 2 + 1;
    let height = MAZE_ROWS * 2 + 1;
    let mut rows = vec![vec!['#'; width]; height];
    let exit = (MAZE_COLUMNS - 1, MAZE_ROWS - 1);

    for z in 0..MAZE_ROWS {
        for x in 0..MAZE_COLUMNS {
            let row = height - 2 - z * 2;
            let col = x * 2 + 1;
            rows[row][col] = if (x, z) == start {
                'P'
            } else if (x, z) == exit {
                'G'
            } else {
                '.'
            };

            for direction in [
                Direction::North,
                Direction::East,
                Direction::South,
                Direction::West,
            ] {
                if maze[z][x].walls[direction.index()] {
                    continue;
                }
                let (dx, dz) = direction.delta();
                let passage_row = (row as isize - dz) as usize;
                let passage_col = (col as isize + dx) as usize;
                rows[passage_row][passage_col] = '.';
            }
        }
    }

    for direction in [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ] {
        if !maze[exit.1][exit.0].walls[direction.index()] {
            let row = height - 2 - exit.1 * 2;
            let col = exit.0 * 2 + 1;
            let (dx, dz) = direction.delta();
            rows[(row as isize - dz) as usize][(col as isize + dx) as usize] = 'D';
            break;
        }
    }

    rows.into_iter()
        .map(|row| row.into_iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn horizontal_wall_at(
    x: usize,
    z_line: usize,
    maze: &[[MazeCell; MAZE_COLUMNS]; MAZE_ROWS],
) -> bool {
    if x >= MAZE_COLUMNS || z_line > MAZE_ROWS {
        return false;
    }
    if z_line == 0 {
        maze[0][x].walls[Direction::South.index()]
    } else {
        maze[z_line - 1][x].walls[Direction::North.index()]
    }
}

fn vertical_wall_at(x_line: usize, z: usize, maze: &[[MazeCell; MAZE_COLUMNS]; MAZE_ROWS]) -> bool {
    if x_line > MAZE_COLUMNS || z >= MAZE_ROWS {
        return false;
    }
    if x_line == 0 {
        maze[z][0].walls[Direction::West.index()]
    } else {
        maze[z][x_line - 1].walls[Direction::East.index()]
    }
}

pub fn cell_center(x: usize, z: usize) -> Vec2 {
    let width = MAZE_COLUMNS as f32 * CELL_SIZE;
    let depth = MAZE_ROWS as f32 * CELL_SIZE;
    Vec2::new(
        x as f32 * CELL_SIZE - width * 0.5 + CELL_SIZE * 0.5,
        z as f32 * CELL_SIZE - depth * 0.5 + CELL_SIZE * 0.5,
    )
}

pub fn world_to_cell(position: Vec3) -> Option<(usize, usize)> {
    let width = MAZE_COLUMNS as f32 * CELL_SIZE;
    let depth = MAZE_ROWS as f32 * CELL_SIZE;
    let x = ((position.x + width * 0.5) / CELL_SIZE).floor() as isize;
    let z = ((position.z + depth * 0.5) / CELL_SIZE).floor() as isize;
    if x < 0 || z < 0 || x >= MAZE_COLUMNS as isize || z >= MAZE_ROWS as isize {
        return None;
    }
    Some((x as usize, z as usize))
}

fn spawn_wall(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    center: Vec2,
    half_extents: Vec2,
    cell: (usize, usize),
    direction: Direction,
    mineable: bool,
    hits_required: u8,
    ore_deposit: bool,
    vertical: bool,
) {
    let hits_required = hits_required.max(1);
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(center.x, WALL_HEIGHT * 0.5, center.y),
        Wall,
        WallCollider {
            center,
            half_extents,
        },
        WallSegment {
            cell,
            direction,
            mineable,
            hits_remaining: hits_required,
            max_hits: hits_required,
            ore_deposit,
        },
    ));

    if vertical {
        debug!("spawned vertical wall at {:?}", center);
    }
}

fn choose_ore_deposit_edges(
    maze: &[[MazeCell; MAZE_COLUMNS]; MAZE_ROWS],
    start: (usize, usize),
) -> Vec<((usize, usize), Direction)> {
    let mut candidates = Vec::new();

    for z in 0..MAZE_ROWS {
        for x in 0..MAZE_COLUMNS {
            for direction in [Direction::North, Direction::West] {
                if !maze[z][x].walls[direction.index()] || hard_wall_near_exit((x, z), direction) {
                    continue;
                }
                let Some(neighbor) = neighbor_cell((x, z), direction) else {
                    continue;
                };
                if start.0.abs_diff(x) + start.1.abs_diff(z) <= 3
                    || start.0.abs_diff(neighbor.0) + start.1.abs_diff(neighbor.1) <= 3
                {
                    continue;
                }
                candidates.push(((x, z), direction));
            }
        }
    }

    candidates.shuffle(&mut rand::thread_rng());
    candidates.truncate(12);
    candidates
}

fn seal_extra_exit_passages(
    maze: &mut [[MazeCell; MAZE_COLUMNS]; MAZE_ROWS],
    start: (usize, usize),
) {
    let exit = (MAZE_COLUMNS - 1, MAZE_ROWS - 1);
    let mut open_edges = Vec::new();

    for direction in [
        Direction::South,
        Direction::West,
        Direction::North,
        Direction::East,
    ] {
        if maze[exit.1][exit.0].walls[direction.index()] {
            continue;
        }
        let Some(neighbor) = neighbor_cell(exit, direction) else {
            continue;
        };
        open_edges.push((direction, neighbor));
    }

    let Some((keep_direction, _)) = open_edges
        .iter()
        .copied()
        .find(|(_, neighbor)| connects_to_start_without_exit(maze, *neighbor, exit, start))
        .or_else(|| open_edges.first().copied())
    else {
        return;
    };

    for (direction, neighbor) in open_edges.into_iter() {
        if direction == keep_direction {
            continue;
        }
        maze[exit.1][exit.0].walls[direction.index()] = true;
        maze[neighbor.1][neighbor.0].walls[direction.opposite().index()] = true;
    }

    info!(
        "Exit chamber has one locked-door passage facing {:?}",
        keep_direction.index()
    );
}

fn fork_cell_count(maze: &[[MazeCell; MAZE_COLUMNS]; MAZE_ROWS]) -> usize {
    maze.iter()
        .flatten()
        .filter(|cell| cell.walls.iter().filter(|wall| !**wall).count() >= 3)
        .count()
}

fn repair_maze_connectivity(
    maze: &mut [[MazeCell; MAZE_COLUMNS]; MAZE_ROWS],
    start: (usize, usize),
) {
    while reachable_cell_count(maze, start) < MAZE_COLUMNS * MAZE_ROWS {
        let reachable = reachable_cells(maze, start);
        let Some((cell, direction, next)) = reconnect_edge(maze, &reachable) else {
            return;
        };
        open_maze_edge(maze, cell, direction, next);
    }
}

fn reconnect_edge(
    maze: &[[MazeCell; MAZE_COLUMNS]; MAZE_ROWS],
    reachable: &[[bool; MAZE_COLUMNS]; MAZE_ROWS],
) -> Option<((usize, usize), Direction, (usize, usize))> {
    let exit = (MAZE_COLUMNS - 1, MAZE_ROWS - 1);
    for z in 0..MAZE_ROWS {
        for x in 0..MAZE_COLUMNS {
            let cell = (x, z);
            if !reachable[z][x] || cell == exit {
                continue;
            }
            for direction in [
                Direction::North,
                Direction::East,
                Direction::South,
                Direction::West,
            ] {
                if !maze[z][x].walls[direction.index()] {
                    continue;
                }
                let Some(next) = neighbor_cell(cell, direction) else {
                    continue;
                };
                if next != exit && !reachable[next.1][next.0] {
                    return Some((cell, direction, next));
                }
            }
        }
    }

    None
}

fn open_maze_edge(
    maze: &mut [[MazeCell; MAZE_COLUMNS]; MAZE_ROWS],
    cell: (usize, usize),
    direction: Direction,
    next: (usize, usize),
) {
    maze[cell.1][cell.0].walls[direction.index()] = false;
    maze[next.1][next.0].walls[direction.opposite().index()] = false;
}

fn reachable_cell_count(
    maze: &[[MazeCell; MAZE_COLUMNS]; MAZE_ROWS],
    start: (usize, usize),
) -> usize {
    reachable_cells(maze, start)
        .iter()
        .flatten()
        .filter(|visited| **visited)
        .count()
}

fn reachable_cells(
    maze: &[[MazeCell; MAZE_COLUMNS]; MAZE_ROWS],
    start: (usize, usize),
) -> [[bool; MAZE_COLUMNS]; MAZE_ROWS] {
    let mut visited = [[false; MAZE_COLUMNS]; MAZE_ROWS];
    let mut stack = vec![start];

    while let Some(cell) = stack.pop() {
        if visited[cell.1][cell.0] {
            continue;
        }
        visited[cell.1][cell.0] = true;

        for direction in [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ] {
            if maze[cell.1][cell.0].walls[direction.index()] {
                continue;
            }
            let Some(next) = neighbor_cell(cell, direction) else {
                continue;
            };
            if !maze[next.1][next.0].walls[direction.opposite().index()] {
                stack.push(next);
            }
        }
    }

    visited
}

fn connects_to_start_without_exit(
    maze: &[[MazeCell; MAZE_COLUMNS]; MAZE_ROWS],
    from: (usize, usize),
    exit: (usize, usize),
    start: (usize, usize),
) -> bool {
    let mut visited = [[false; MAZE_COLUMNS]; MAZE_ROWS];
    let mut stack = vec![from];
    visited[exit.1][exit.0] = true;

    while let Some(cell) = stack.pop() {
        if cell == start {
            return true;
        }
        if visited[cell.1][cell.0] {
            continue;
        }
        visited[cell.1][cell.0] = true;

        for direction in [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ] {
            if maze[cell.1][cell.0].walls[direction.index()] {
                continue;
            }
            let Some(next) = neighbor_cell(cell, direction) else {
                continue;
            };
            if maze[next.1][next.0].walls[direction.opposite().index()] || visited[next.1][next.0] {
                continue;
            }
            stack.push(next);
        }
    }

    false
}

fn neighbor_cell(cell: (usize, usize), direction: Direction) -> Option<(usize, usize)> {
    let (dx, dz) = direction.delta();
    let nx = cell.0 as isize + dx;
    let nz = cell.1 as isize + dz;
    if nx < 0 || nz < 0 || nx >= MAZE_COLUMNS as isize || nz >= MAZE_ROWS as isize {
        return None;
    }

    Some((nx as usize, nz as usize))
}

fn hard_wall_near_exit(cell: (usize, usize), direction: Direction) -> bool {
    let exit = (MAZE_COLUMNS - 1, MAZE_ROWS - 1);
    if cell_near_exit(cell, exit) {
        return true;
    }

    let (dx, dz) = direction.delta();
    let nx = cell.0 as isize + dx;
    let nz = cell.1 as isize + dz;
    if nx < 0 || nz < 0 || nx >= MAZE_COLUMNS as isize || nz >= MAZE_ROWS as isize {
        return false;
    }
    cell_near_exit((nx as usize, nz as usize), exit)
}

fn cell_near_exit(cell: (usize, usize), exit: (usize, usize)) -> bool {
    cell.0.abs_diff(exit.0) + cell.1.abs_diff(exit.1) <= 1
}

fn generate_connected_maze() -> ([[MazeCell; MAZE_COLUMNS]; MAZE_ROWS], (usize, usize)) {
    let mut rng = rand::thread_rng();
    let start_cell = choose_wall_start_cell(&mut rng);
    let (mut maze, protected, mut stack, required_forks) =
        seed_early_fork_structure(start_cell, &mut rng);
    let mut fork_seeds = choose_fork_seed_cells(&mut rng, &required_forks);

    while let Some(&(x, z)) = stack.last() {
        if fork_seeds[z][x] {
            fork_seeds[z][x] = false;
            if carve_seeded_fork_branches(&mut maze, (x, z), &mut stack, &mut rng) {
                continue;
            }
        }

        let mut directions = [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ];
        directions.shuffle(&mut rng);

        let mut fallback_next = None;
        let next = directions
            .into_iter()
            .find_map(|direction| {
                let (dx, dz) = direction.delta();
                let next_x = x as isize + dx;
                let next_z = z as isize + dz;
                if next_x < 0
                    || next_z < 0
                    || next_x >= MAZE_COLUMNS as isize
                    || next_z >= MAZE_ROWS as isize
                {
                    return None;
                }

                let next_x = next_x as usize;
                let next_z = next_z as usize;
                if maze[next_z][next_x].visited {
                    return None;
                }
                let step = (direction, next_x, next_z);
                if !fork_seeds[next_z][next_x]
                    && !move_preserves_fork_seed_capacity(&maze, &fork_seeds, (next_x, next_z))
                {
                    fallback_next.get_or_insert(step);
                    return None;
                }

                Some(step)
            })
            .or(fallback_next);

        if let Some((direction, next_x, next_z)) = next {
            carve_generator_edge(&mut maze, (x, z), direction, (next_x, next_z));
            maze[next_z][next_x].visited = true;
            stack.push((next_x, next_z));
        } else {
            stack.pop();
        }
    }
    connect_remaining_unvisited_cells(&mut maze, &protected, &mut rng);

    let mut cells = [[MazeCell { walls: [true; 4] }; MAZE_COLUMNS]; MAZE_ROWS];
    for z in 0..MAZE_ROWS {
        for x in 0..MAZE_COLUMNS {
            cells[z][x] = MazeCell {
                walls: maze[z][x].walls,
            };
        }
    }

    (cells, start_cell)
}

fn seed_early_fork_structure(
    start_cell: (usize, usize),
    rng: &mut rand::rngs::ThreadRng,
) -> (
    [[GeneratorCell; MAZE_COLUMNS]; MAZE_ROWS],
    [[bool; MAZE_COLUMNS]; MAZE_ROWS],
    Vec<(usize, usize)>,
    Vec<(usize, usize)>,
) {
    let mut candidates = first_fork_candidates(start_cell);
    candidates.shuffle(rng);

    for first_fork in candidates {
        let mut maze = [[GeneratorCell::default(); MAZE_COLUMNS]; MAZE_ROWS];
        let mut protected = [[false; MAZE_COLUMNS]; MAZE_ROWS];
        let incoming_edge =
            seed_start_corridor(&mut maze, &mut protected, start_cell, first_fork, rng);
        let (stack, secondary_forks) = seed_secondary_fork_corridors(
            &mut maze,
            &mut protected,
            first_fork,
            incoming_edge,
            rng,
        );
        if secondary_forks.len() == 2 {
            let mut required_forks = vec![first_fork];
            required_forks.extend(secondary_forks);
            return (maze, protected, stack, required_forks);
        }
    }

    let mut maze = [[GeneratorCell::default(); MAZE_COLUMNS]; MAZE_ROWS];
    let mut protected = [[false; MAZE_COLUMNS]; MAZE_ROWS];
    let first_fork = fallback_first_fork_cell(start_cell);
    let incoming_edge = seed_start_corridor(&mut maze, &mut protected, start_cell, first_fork, rng);
    let (mut stack, secondary_forks) =
        seed_secondary_fork_corridors(&mut maze, &mut protected, first_fork, incoming_edge, rng);
    let mut required_forks = vec![first_fork];
    required_forks.extend(secondary_forks);
    if stack.is_empty() {
        stack.push(first_fork);
    }
    (maze, protected, stack, required_forks)
}

fn choose_wall_start_cell(rng: &mut rand::rngs::ThreadRng) -> (usize, usize) {
    let mut candidates = Vec::new();
    let exit = (MAZE_COLUMNS - 1, MAZE_ROWS - 1);
    for offset in MIN_WALL_SPAWN_OFFSET..MAZE_COLUMNS - MIN_WALL_SPAWN_OFFSET {
        for cell in [(offset, 0), (offset, MAZE_ROWS - 1)] {
            if manhattan_distance(cell, exit) >= MIN_WALL_SPAWN_EXIT_DISTANCE {
                candidates.push(cell);
            }
        }
    }
    for offset in MIN_WALL_SPAWN_OFFSET..MAZE_ROWS - MIN_WALL_SPAWN_OFFSET {
        for cell in [(0, offset), (MAZE_COLUMNS - 1, offset)] {
            if manhattan_distance(cell, exit) >= MIN_WALL_SPAWN_EXIT_DISTANCE {
                candidates.push(cell);
            }
        }
    }

    candidates.shuffle(rng);
    candidates.first().copied().unwrap_or((MAZE_COLUMNS / 2, 0))
}

fn first_fork_candidates(start_cell: (usize, usize)) -> Vec<(usize, usize)> {
    let mut candidates = Vec::new();
    for z in 2..MAZE_ROWS - 2 {
        for x in 2..MAZE_COLUMNS - 2 {
            let cell = (x, z);
            let distance = manhattan_distance(start_cell, cell);
            if (MIN_FIRST_FORK_DISTANCE..=MAX_FIRST_FORK_DISTANCE).contains(&distance) {
                candidates.push(cell);
            }
        }
    }

    candidates
}

fn fallback_first_fork_cell(start_cell: (usize, usize)) -> (usize, usize) {
    let x = start_cell.0.clamp(3, MAZE_COLUMNS - 4);
    let z = start_cell.1.clamp(3, MAZE_ROWS - 4);
    (x, z)
}

fn choose_fork_seed_cells(
    rng: &mut rand::rngs::ThreadRng,
    required_forks: &[(usize, usize)],
) -> [[bool; MAZE_COLUMNS]; MAZE_ROWS] {
    let mut seeds = [[false; MAZE_COLUMNS]; MAZE_ROWS];
    let mut selected = required_forks.to_vec();
    let mut candidates = Vec::new();

    for z in 2..MAZE_ROWS - 2 {
        for x in 2..MAZE_COLUMNS - 2 {
            let cell = (x, z);
            if selected.contains(&cell) || cell_near_exit(cell, (MAZE_COLUMNS - 1, MAZE_ROWS - 1)) {
                continue;
            }
            candidates.push(cell);
        }
    }

    candidates.shuffle(rng);
    for cell in candidates.iter().copied() {
        if selected.len() >= MIN_FORK_CELLS {
            break;
        }
        if selected
            .iter()
            .all(|seed| manhattan_distance(*seed, cell) >= 5)
        {
            selected.push(cell);
        }
    }
    for cell in candidates.into_iter() {
        if selected.len() >= MIN_FORK_CELLS {
            break;
        }
        if !selected.contains(&cell) {
            selected.push(cell);
        }
    }

    for (x, z) in selected {
        seeds[z][x] = true;
    }
    seeds
}

fn seed_start_corridor(
    maze: &mut [[GeneratorCell; MAZE_COLUMNS]; MAZE_ROWS],
    protected: &mut [[bool; MAZE_COLUMNS]; MAZE_ROWS],
    start_cell: (usize, usize),
    first_fork: (usize, usize),
    rng: &mut rand::rngs::ThreadRng,
) -> Direction {
    let path = path_to_cell(start_cell, first_fork, rng);
    maze[start_cell.1][start_cell.0].visited = true;

    for pair in path.windows(2) {
        let from = pair[0];
        let to = pair[1];
        let direction = direction_between_cells(from, to).expect("path cells must be neighbors");
        carve_generator_edge(maze, from, direction, to);
        maze[to.1][to.0].visited = true;
        if to != first_fork {
            protected[to.1][to.0] = true;
        }
    }
    protected[start_cell.1][start_cell.0] = true;

    let previous = path[path.len() - 2];
    direction_between_cells(first_fork, previous)
        .expect("first fork path must have an incoming edge")
}

fn seed_secondary_fork_corridors(
    maze: &mut [[GeneratorCell; MAZE_COLUMNS]; MAZE_ROWS],
    protected: &mut [[bool; MAZE_COLUMNS]; MAZE_ROWS],
    first_fork: (usize, usize),
    incoming_edge: Direction,
    rng: &mut rand::rngs::ThreadRng,
) -> (Vec<(usize, usize)>, Vec<(usize, usize)>) {
    let mut branches = Vec::new();
    let mut stack = Vec::new();
    let mut directions = [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];
    directions.shuffle(rng);

    for direction in directions {
        if direction == incoming_edge || branches.len() >= 2 {
            continue;
        }
        if let Some(cell) =
            carve_secondary_fork_corridor(maze, protected, first_fork, direction, rng, &mut stack)
        {
            branches.push(cell);
        }
    }

    (stack, branches)
}

fn carve_secondary_fork_corridor(
    maze: &mut [[GeneratorCell; MAZE_COLUMNS]; MAZE_ROWS],
    protected: &mut [[bool; MAZE_COLUMNS]; MAZE_ROWS],
    first_fork: (usize, usize),
    first_direction: Direction,
    rng: &mut rand::rngs::ThreadRng,
    stack: &mut Vec<(usize, usize)>,
) -> Option<(usize, usize)> {
    let mut current = first_fork;
    let mut previous_direction = first_direction;
    let mut path = Vec::new();
    let mut reserved = vec![first_fork];

    for step in 0..SECONDARY_FORK_CORRIDOR_LENGTH {
        let mut directions = [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ];
        directions.shuffle(rng);
        let next_step = if step == 0 {
            secondary_corridor_step(maze, current, first_direction, &reserved)
        } else {
            directions
                .into_iter()
                .filter(|direction| *direction != previous_direction.opposite())
                .find_map(|direction| secondary_corridor_step(maze, current, direction, &reserved))
        }?;

        previous_direction = next_step.0;
        path.push((current, next_step.0, next_step.1));
        current = next_step.1;
        reserved.push(current);
    }

    let mut remaining_exits: Vec<_> = generator_neighbors(current)
        .into_iter()
        .filter(|(_, next)| !maze[next.1][next.0].visited && !reserved.contains(next))
        .collect();
    remaining_exits.shuffle(rng);
    if remaining_exits.len() < 2 {
        return None;
    }

    for (index, (from, direction, to)) in path.into_iter().enumerate() {
        carve_generator_edge(maze, from, direction, to);
        maze[to.1][to.0].visited = true;
        if index + 1 < SECONDARY_FORK_CORRIDOR_LENGTH {
            protected[to.1][to.0] = true;
        }
    }

    for (direction, next) in remaining_exits.into_iter().take(2) {
        carve_generator_edge(maze, current, direction, next);
        maze[next.1][next.0].visited = true;
        stack.push(next);
    }
    stack.push(current);

    Some(current)
}

fn secondary_corridor_step(
    maze: &[[GeneratorCell; MAZE_COLUMNS]; MAZE_ROWS],
    cell: (usize, usize),
    direction: Direction,
    reserved: &[(usize, usize)],
) -> Option<(Direction, (usize, usize))> {
    let next = neighbor_cell(cell, direction)?;
    (!maze[next.1][next.0].visited && !reserved.contains(&next)).then_some((direction, next))
}

fn path_to_cell(
    start: (usize, usize),
    goal: (usize, usize),
    rng: &mut rand::rngs::ThreadRng,
) -> Vec<(usize, usize)> {
    let mut path = vec![start];
    let mut current = start;

    while current != goal {
        let mut candidates = Vec::new();
        if current.0 < goal.0 {
            candidates.push((current.0 + 1, current.1));
        }
        if current.1 < goal.1 {
            candidates.push((current.0, current.1 + 1));
        }
        if current.0 > goal.0 {
            candidates.push((current.0 - 1, current.1));
        }
        if current.1 > goal.1 {
            candidates.push((current.0, current.1 - 1));
        }
        candidates.shuffle(rng);
        current = candidates[0];
        path.push(current);
    }

    path
}

fn direction_between_cells(from: (usize, usize), to: (usize, usize)) -> Option<Direction> {
    match (
        to.0 as isize - from.0 as isize,
        to.1 as isize - from.1 as isize,
    ) {
        (0, 1) => Some(Direction::North),
        (1, 0) => Some(Direction::East),
        (0, -1) => Some(Direction::South),
        (-1, 0) => Some(Direction::West),
        _ => None,
    }
}

fn carve_seeded_fork_branches(
    maze: &mut [[GeneratorCell; MAZE_COLUMNS]; MAZE_ROWS],
    cell: (usize, usize),
    stack: &mut Vec<(usize, usize)>,
    rng: &mut rand::rngs::ThreadRng,
) -> bool {
    let needed = 3_usize.saturating_sub(open_edge_count(&maze[cell.1][cell.0]));
    if needed == 0 {
        return false;
    }

    let mut candidates = unvisited_generator_neighbors(maze, cell);
    candidates.shuffle(rng);
    let mut carved = false;

    for (direction, next) in candidates.into_iter().take(needed) {
        carve_generator_edge(maze, cell, direction, next);
        maze[next.1][next.0].visited = true;
        stack.push(next);
        carved = true;
    }

    carved
}

fn move_preserves_fork_seed_capacity(
    maze: &[[GeneratorCell; MAZE_COLUMNS]; MAZE_ROWS],
    fork_seeds: &[[bool; MAZE_COLUMNS]; MAZE_ROWS],
    next: (usize, usize),
) -> bool {
    for direction in [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ] {
        let Some(seed) = neighbor_cell(next, direction) else {
            continue;
        };
        if !fork_seeds[seed.1][seed.0] || maze[seed.1][seed.0].visited {
            continue;
        }

        let remaining_unvisited = generator_neighbors(seed)
            .into_iter()
            .filter(|(_, neighbor)| *neighbor != next && !maze[neighbor.1][neighbor.0].visited)
            .count();
        if remaining_unvisited < 2 {
            return false;
        }
    }

    true
}

fn connect_remaining_unvisited_cells(
    maze: &mut [[GeneratorCell; MAZE_COLUMNS]; MAZE_ROWS],
    protected: &[[bool; MAZE_COLUMNS]; MAZE_ROWS],
    rng: &mut rand::rngs::ThreadRng,
) {
    while let Some((cell, neighbor, direction_to_neighbor)) =
        unvisited_cell_next_to_tree(maze, protected)
    {
        carve_generator_edge(maze, neighbor, direction_to_neighbor.opposite(), cell);
        maze[cell.1][cell.0].visited = true;
        fill_unvisited_from(maze, cell, rng);
    }
}

fn unvisited_cell_next_to_tree(
    maze: &[[GeneratorCell; MAZE_COLUMNS]; MAZE_ROWS],
    protected: &[[bool; MAZE_COLUMNS]; MAZE_ROWS],
) -> Option<((usize, usize), (usize, usize), Direction)> {
    for z in 0..MAZE_ROWS {
        for x in 0..MAZE_COLUMNS {
            let cell = (x, z);
            if maze[z][x].visited {
                continue;
            }
            if let Some((direction, neighbor)) =
                generator_neighbors(cell).into_iter().find(|(_, neighbor)| {
                    maze[neighbor.1][neighbor.0].visited && !protected[neighbor.1][neighbor.0]
                })
            {
                return Some((cell, neighbor, direction));
            }
        }
    }

    None
}

fn fill_unvisited_from(
    maze: &mut [[GeneratorCell; MAZE_COLUMNS]; MAZE_ROWS],
    start: (usize, usize),
    rng: &mut rand::rngs::ThreadRng,
) {
    let mut stack = vec![start];
    while let Some(&cell) = stack.last() {
        let mut candidates = unvisited_generator_neighbors(maze, cell);
        candidates.shuffle(rng);

        if let Some((direction, next)) = candidates.first().copied() {
            carve_generator_edge(maze, cell, direction, next);
            maze[next.1][next.0].visited = true;
            stack.push(next);
        } else {
            stack.pop();
        }
    }
}

fn unvisited_generator_neighbors(
    maze: &[[GeneratorCell; MAZE_COLUMNS]; MAZE_ROWS],
    cell: (usize, usize),
) -> Vec<(Direction, (usize, usize))> {
    generator_neighbors(cell)
        .into_iter()
        .filter(|(_, next)| !maze[next.1][next.0].visited)
        .collect()
}

fn generator_neighbors(cell: (usize, usize)) -> Vec<(Direction, (usize, usize))> {
    [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ]
    .into_iter()
    .filter_map(|direction| neighbor_cell(cell, direction).map(|next| (direction, next)))
    .collect()
}

fn carve_generator_edge(
    maze: &mut [[GeneratorCell; MAZE_COLUMNS]; MAZE_ROWS],
    cell: (usize, usize),
    direction: Direction,
    next: (usize, usize),
) {
    maze[cell.1][cell.0].walls[direction.index()] = false;
    maze[next.1][next.0].walls[direction.opposite().index()] = false;
}

fn open_edge_count(cell: &GeneratorCell) -> usize {
    cell.walls.iter().filter(|wall| !**wall).count()
}

fn manhattan_distance(a: (usize, usize), b: (usize, usize)) -> usize {
    a.0.abs_diff(b.0) + a.1.abs_diff(b.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_generation_keeps_minimum_fork_count() {
        for _ in 0..20 {
            let (mut maze, start_cell) = generate_connected_maze();
            seal_extra_exit_passages(&mut maze, start_cell);
            repair_maze_connectivity(&mut maze, start_cell);

            assert!(
                fork_cell_count(&maze) >= MIN_FORK_CELLS,
                "expected at least {MIN_FORK_CELLS} fork cells"
            );
            assert_eq!(
                reachable_cell_count(&maze, start_cell),
                MAZE_COLUMNS * MAZE_ROWS
            );
        }
    }

    #[test]
    fn first_fork_is_randomized_and_branches_to_more_forks() {
        for _ in 0..20 {
            let (maze, start_cell) = generate_connected_maze();
            let (first_fork, incoming_cell) = first_fork_from_spawn(&maze, start_cell);
            let branch_count = open_neighbors(&maze, first_fork)
                .into_iter()
                .filter(|cell| *cell != incoming_cell)
                .filter(|cell| branch_reaches_another_fork(&maze, first_fork, *cell))
                .count();

            assert!(wall_cell_not_corner(start_cell));
            assert!(
                manhattan_distance(start_cell, (MAZE_COLUMNS - 1, MAZE_ROWS - 1))
                    >= MIN_WALL_SPAWN_EXIT_DISTANCE
            );
            assert!(manhattan_distance(start_cell, first_fork) >= MIN_FIRST_FORK_DISTANCE);
            assert_eq!(branch_count, 2);
        }
    }

    fn wall_cell_not_corner(cell: (usize, usize)) -> bool {
        let on_wall =
            cell.0 == 0 || cell.1 == 0 || cell.0 == MAZE_COLUMNS - 1 || cell.1 == MAZE_ROWS - 1;
        let in_corner =
            (cell.0 == 0 || cell.0 == MAZE_COLUMNS - 1) && (cell.1 == 0 || cell.1 == MAZE_ROWS - 1);
        on_wall && !in_corner
    }

    fn first_fork_from_spawn(
        maze: &[[MazeCell; MAZE_COLUMNS]; MAZE_ROWS],
        start_cell: (usize, usize),
    ) -> ((usize, usize), (usize, usize)) {
        let mut previous = None;
        let mut current = start_cell;

        loop {
            let neighbors = open_neighbors(maze, current);
            if neighbors.len() >= 3 {
                return (
                    current,
                    previous.expect("spawn cell should not be first fork"),
                );
            }

            let next_options: Vec<_> = neighbors
                .into_iter()
                .filter(|cell| Some(*cell) != previous)
                .collect();
            assert_eq!(
                next_options.len(),
                1,
                "spawn route should be a single corridor before first fork"
            );
            previous = Some(current);
            current = next_options[0];
        }
    }

    fn branch_reaches_another_fork(
        maze: &[[MazeCell; MAZE_COLUMNS]; MAZE_ROWS],
        first_fork: (usize, usize),
        first_step: (usize, usize),
    ) -> bool {
        let mut previous = first_fork;
        let mut current = first_step;

        for _ in 0..MAZE_COLUMNS * MAZE_ROWS {
            let neighbors = open_neighbors(maze, current);
            if neighbors.len() >= 3 {
                return true;
            }
            let next_options: Vec<_> = neighbors
                .into_iter()
                .filter(|cell| *cell != previous)
                .collect();
            if next_options.len() != 1 {
                return false;
            }
            previous = current;
            current = next_options[0];
        }

        false
    }

    fn open_neighbors(
        maze: &[[MazeCell; MAZE_COLUMNS]; MAZE_ROWS],
        cell: (usize, usize),
    ) -> Vec<(usize, usize)> {
        [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ]
        .into_iter()
        .filter_map(|direction| {
            if maze[cell.1][cell.0].walls[direction.index()] {
                return None;
            }
            let next = neighbor_cell(cell, direction)?;
            (!maze[next.1][next.0].walls[direction.opposite().index()]).then_some(next)
        })
        .collect()
    }

    fn reachable_cell_count(
        maze: &[[MazeCell; MAZE_COLUMNS]; MAZE_ROWS],
        start_cell: (usize, usize),
    ) -> usize {
        let mut visited = [[false; MAZE_COLUMNS]; MAZE_ROWS];
        let mut stack = vec![start_cell];
        let mut count = 0;

        while let Some(cell) = stack.pop() {
            if visited[cell.1][cell.0] {
                continue;
            }
            visited[cell.1][cell.0] = true;
            count += 1;

            for direction in [
                Direction::North,
                Direction::East,
                Direction::South,
                Direction::West,
            ] {
                if maze[cell.1][cell.0].walls[direction.index()] {
                    continue;
                }
                let Some(next) = neighbor_cell(cell, direction) else {
                    continue;
                };
                if !maze[next.1][next.0].walls[direction.opposite().index()] {
                    stack.push(next);
                }
            }
        }

        count
    }
}
