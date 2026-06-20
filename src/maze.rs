use bevy::prelude::*;
use rand::seq::SliceRandom;

pub const MAZE_COLUMNS: usize = 12;
pub const MAZE_ROWS: usize = 12;
pub const CELL_SIZE: f32 = 120.0;
pub const WALL_HEIGHT: f32 = 90.0;
pub const WALL_THICKNESS: f32 = 10.0;
pub const FLOOR_THICKNESS: f32 = 2.5;
const FOG_HEIGHT: f32 = WALL_HEIGHT - 5.0;
const VISUAL_WALL_LENGTH: f32 = CELL_SIZE - WALL_THICKNESS;

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
    let mut maze = generate_connected_maze();
    seal_extra_exit_passages(&mut maze);
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
    let ore_deposit_edges = choose_ore_deposit_edges(&maze);
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
                    if hard { 3 } else { 1 },
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
                    if hard { 3 } else { 1 },
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

    let start = cell_center(0, 0);
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
) -> Vec<((usize, usize), Direction)> {
    let mut candidates = Vec::new();
    let start = (0_usize, 0_usize);

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
    candidates.truncate(7);
    candidates
}

fn seal_extra_exit_passages(maze: &mut [[MazeCell; MAZE_COLUMNS]; MAZE_ROWS]) {
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
        .find(|(_, neighbor)| connects_to_start_without_exit(maze, *neighbor, exit))
        .or_else(|| open_edges.first().copied())
    else {
        return;
    };

    for (direction, neighbor) in open_edges.into_iter().skip(1) {
        maze[exit.1][exit.0].walls[direction.index()] = true;
        maze[neighbor.1][neighbor.0].walls[direction.opposite().index()] = true;
    }

    info!(
        "Exit chamber has one locked-door passage facing {:?}",
        keep_direction.index()
    );
}

fn connects_to_start_without_exit(
    maze: &[[MazeCell; MAZE_COLUMNS]; MAZE_ROWS],
    from: (usize, usize),
    exit: (usize, usize),
) -> bool {
    let start = (0, 0);
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

fn generate_connected_maze() -> [[MazeCell; MAZE_COLUMNS]; MAZE_ROWS] {
    let mut maze = [[GeneratorCell::default(); MAZE_COLUMNS]; MAZE_ROWS];
    let mut stack = vec![(0usize, 0usize)];
    let mut rng = rand::thread_rng();

    maze[0][0].visited = true;

    while let Some(&(x, z)) = stack.last() {
        let mut directions = [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ];
        directions.shuffle(&mut rng);

        let next = directions.into_iter().find_map(|direction| {
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
            (!maze[next_z][next_x].visited).then_some((direction, next_x, next_z))
        });

        if let Some((direction, next_x, next_z)) = next {
            maze[z][x].walls[direction.index()] = false;
            maze[next_z][next_x].walls[direction.opposite().index()] = false;
            maze[next_z][next_x].visited = true;
            stack.push((next_x, next_z));
        } else {
            stack.pop();
        }
    }

    let mut cells = [[MazeCell { walls: [true; 4] }; MAZE_COLUMNS]; MAZE_ROWS];
    for z in 0..MAZE_ROWS {
        for x in 0..MAZE_COLUMNS {
            cells[z][x] = MazeCell {
                walls: maze[z][x].walls,
            };
        }
    }

    cells
}
