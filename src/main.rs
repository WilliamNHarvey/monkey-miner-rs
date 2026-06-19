use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy_sprite3d::prelude::*;
use rand::seq::SliceRandom;
mod maze;
use maze::{
    Direction, FogCell, MAZE_COLUMNS, MAZE_ROWS, MazeMap, WallCollider, WallSegment, cell_center,
    create_maze, world_to_cell,
};

#[derive(Component)]
struct Player;

#[derive(Component)]
struct FollowCamera;

#[derive(Component)]
struct PlayerLight;

#[derive(Component)]
struct Exit;

#[derive(Component)]
struct Rat {
    cell: (usize, usize),
    target_cell: (usize, usize),
    last_direction: Option<Direction>,
}

#[derive(Component)]
struct TrailMarker;

#[derive(Component)]
struct Treasure {
    value: u32,
}

#[derive(Component)]
struct Ore {
    value: u32,
}

#[derive(Component)]
struct KeyPickup;

#[derive(Component)]
struct LockedDoor;

#[derive(Component)]
struct Chest;

#[derive(Component)]
struct HudText;

#[derive(Resource, Default)]
struct RunState {
    won: bool,
    treasure: u32,
    total_treasure: u32,
    ore: u32,
    mined_walls: u32,
    keys: u32,
    doors_unlocked: u32,
    chests_opened: u32,
}

#[derive(Resource)]
struct TrailMarkers {
    marked: [[bool; MAZE_COLUMNS]; MAZE_ROWS],
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

#[derive(Resource)]
struct MiningAssets {
    ore_mesh: Handle<Mesh>,
    ore_material: Handle<StandardMaterial>,
}

#[derive(Resource)]
struct CameraOrbit {
    yaw: f32,
    pitch: f32,
    distance: f32,
}

impl Default for CameraOrbit {
    fn default() -> Self {
        Self {
            yaw: std::f32::consts::PI,
            pitch: 0.02,
            distance: 115.0,
        }
    }
}

#[derive(Resource)]
struct FogMemory {
    seen: [[bool; MAZE_COLUMNS]; MAZE_ROWS],
}

impl Default for FogMemory {
    fn default() -> Self {
        Self {
            seen: [[false; MAZE_COLUMNS]; MAZE_ROWS],
        }
    }
}

// Add a resource to track assets loading
#[derive(Resource, Default)]
struct GameAssets {
    monkey: Handle<Image>,
    wall: Handle<Image>,
    floor: Handle<Image>,
    roof: Handle<Image>,
    smiley_exit: Handle<Image>,
    rat: Handle<Image>,
    loaded: bool,
}

// Define game states
#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
enum GameState {
    #[default]
    Loading,
    Ready,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(Sprite3dPlugin)
        .init_resource::<CameraOrbit>()
        .init_resource::<FogMemory>()
        .init_resource::<GameAssets>()
        .init_resource::<RunState>()
        .init_state::<GameState>()
        .add_systems(Startup, load_assets)
        .add_systems(
            Update,
            check_assets_ready.run_if(in_state(GameState::Loading)),
        )
        .add_systems(OnEnter(GameState::Ready), setup)
        .add_systems(
            Update,
            (
                player_movement,
                drop_trail_marker,
                mine_wall,
                rat_movement,
                camera_drag,
                camera_follow,
                update_fog,
                collect_treasure,
                collect_ore,
                collect_key,
                unlock_door,
                open_chest,
                update_hud,
                check_exit_reached,
            )
                .run_if(in_state(GameState::Ready)),
        )
        .run();
}

fn load_assets(mut game_assets: ResMut<GameAssets>, asset_server: Res<AssetServer>) {
    info!("Loading assets...");
    game_assets.monkey = asset_server.load("images/monkey.png");
    game_assets.wall = asset_server.load("images/wall.png");
    game_assets.floor = asset_server.load("images/floor.png");
    game_assets.roof = asset_server.load("images/roof.png");
    game_assets.smiley_exit = asset_server.load("images/smiley_exit.png");
    game_assets.rat = asset_server.load("images/rat.png");
}

fn check_assets_ready(
    mut game_assets: ResMut<GameAssets>,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    // Check if all assets are loaded
    let monkey_loaded = asset_server
        .get_load_state(&game_assets.monkey)
        .is_some_and(|s| s.is_loaded());
    let wall_loaded = asset_server
        .get_load_state(&game_assets.wall)
        .is_some_and(|s| s.is_loaded());
    let floor_loaded = asset_server
        .get_load_state(&game_assets.floor)
        .is_some_and(|s| s.is_loaded());
    let roof_loaded = asset_server
        .get_load_state(&game_assets.roof)
        .is_some_and(|s| s.is_loaded());
    let smiley_loaded = asset_server
        .get_load_state(&game_assets.smiley_exit)
        .is_some_and(|s| s.is_loaded());
    let rat_loaded = asset_server
        .get_load_state(&game_assets.rat)
        .is_some_and(|s| s.is_loaded());

    if monkey_loaded && wall_loaded && floor_loaded && roof_loaded && smiley_loaded && rat_loaded {
        info!("All assets loaded successfully!");
        game_assets.loaded = true;
        next_state.set(GameState::Ready);
    } else {
        // Log which assets are still loading
        if !monkey_loaded {
            info!("Monkey texture still loading...");
        }
        if !wall_loaded {
            info!("Wall texture still loading...");
        }
        if !floor_loaded {
            info!("Floor texture still loading...");
        }
        if !roof_loaded {
            info!("Roof texture still loading...");
        }
        if !smiley_loaded {
            info!("Smiley exit texture still loading...");
        }
        if !rat_loaded {
            info!("Rat texture still loading...");
        }
    }
}

fn setup(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    mut run_state: ResMut<RunState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Setting up game...");

    let (start_position, maze_map) = create_maze(
        &mut commands,
        game_assets.wall.clone(),
        game_assets.floor.clone(),
        game_assets.roof.clone(),
        &mut meshes,
        &mut materials,
    );

    info!("Monkey starting position: {:?}", start_position);

    run_state.treasure = 0;
    run_state.ore = 0;
    run_state.mined_walls = 0;
    run_state.keys = 0;
    run_state.doors_unlocked = 0;
    run_state.chests_opened = 0;
    run_state.won = false;

    commands.insert_resource(TrailMarkers {
        marked: [[false; MAZE_COLUMNS]; MAZE_ROWS],
        mesh: meshes.add(Cuboid::new(22.0, 2.0, 22.0)),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.05, 0.85, 1.0),
            emissive: LinearRgba::rgb(0.0, 0.45, 0.75),
            perceptual_roughness: 0.65,
            ..default()
        }),
    });

    commands.insert_resource(MiningAssets {
        ore_mesh: meshes.add(Cuboid::new(14.0, 14.0, 14.0)),
        ore_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.38, 0.08),
            emissive: LinearRgba::rgb(0.5, 0.12, 0.0),
            perceptual_roughness: 0.55,
            ..default()
        }),
    });

    let treasure_mesh = meshes.add(Cuboid::new(16.0, 16.0, 16.0));
    let treasure_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.78, 0.05),
        emissive: LinearRgba::rgb(0.55, 0.32, 0.0),
        perceptual_roughness: 0.45,
        ..default()
    });
    let total_treasure = spawn_treasures(
        &mut commands,
        &maze_map,
        treasure_mesh,
        treasure_material,
        start_position,
    );
    run_state.total_treasure = total_treasure;
    spawn_key_door_and_chests(
        &mut commands,
        &maze_map,
        start_position,
        &mut meshes,
        &mut materials,
    );

    commands.spawn((
        Text::new(hud_text(&run_state)),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.95, 0.75)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            top: Val::Px(14.0),
            ..default()
        },
        HudText,
    ));

    let monkey_sprite = Sprite3d {
        pixels_per_metre: 12.0,
        alpha_mode: AlphaMode::Mask(0.5),
        unlit: true,
        double_sided: true,
        pivot: Some(Vec2::new(0.5, 0.07)),
        ..Default::default()
    };

    commands.spawn((
        Sprite::from_image(game_assets.monkey.clone()),
        monkey_sprite,
        Transform::from_translation(start_position),
        Player,
    ));

    let exit_cell = (MAZE_COLUMNS - 1, MAZE_ROWS - 1);
    let exit_center = cell_center(exit_cell.0, exit_cell.1);
    let exit_position = Vec3::new(exit_center.x, 34.0, exit_center.y);
    commands.spawn((
        Sprite::from_image(game_assets.smiley_exit.clone()),
        Sprite3d {
            pixels_per_metre: 5.0,
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            double_sided: true,
            ..Default::default()
        },
        Transform::from_translation(exit_position),
        Exit,
    ));

    let rat_cell = (MAZE_COLUMNS / 2, MAZE_ROWS / 2);
    let rat_center = cell_center(rat_cell.0, rat_cell.1);
    commands.spawn((
        Sprite::from_image(game_assets.rat.clone()),
        Sprite3d {
            pixels_per_metre: 4.0,
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            double_sided: true,
            pivot: Some(Vec2::new(0.5, 0.12)),
            ..Default::default()
        },
        Transform::from_translation(Vec3::new(rat_center.x, 0.0, rat_center.y)),
        Rat {
            cell: rat_cell,
            target_cell: rat_cell,
            last_direction: None,
        },
    ));

    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 700.0,
        ..default()
    });

    commands.spawn((
        PointLight {
            color: Color::WHITE,
            intensity: 9000.0,
            range: 260.0,
            ..default()
        },
        Transform::from_translation(start_position + Vec3::Y * 38.0),
        PlayerLight,
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(start_position + Vec3::new(0.0, 80.0, -180.0))
            .looking_at(start_position + Vec3::Y * 24.0, Vec3::Y),
        FollowCamera,
    ));
}

fn spawn_treasures(
    commands: &mut Commands,
    maze: &MazeMap,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    start_position: Vec3,
) -> u32 {
    let start_cell = world_to_cell(start_position).unwrap_or((0, 0));
    let exit_cell = (MAZE_COLUMNS - 1, MAZE_ROWS - 1);
    let mut treasure_cells = Vec::new();
    let mut spawned = 0;

    if let Some(starter_cell) = open_neighbors(start_cell, maze)
        .into_iter()
        .find(|cell| *cell != exit_cell)
    {
        treasure_cells.push(starter_cell);
        info!("Starter treasure at cell {:?}", starter_cell);
    }

    for z in 0..MAZE_ROWS {
        for x in 0..MAZE_COLUMNS {
            if (x, z) == start_cell || (x, z) == exit_cell {
                continue;
            }
            if treasure_cells.contains(&(x, z)) {
                continue;
            }

            let open_edges = maze.cells[z][x].walls.iter().filter(|wall| !**wall).count();
            if open_edges != 1 {
                continue;
            }

            treasure_cells.push((x, z));
        }
    }

    for cell in treasure_cells {
        let center = cell_center(cell.0, cell.1);
        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(center.x, 12.0, center.y),
            Treasure { value: 1 },
        ));
        spawned += 1;
    }

    info!("Spawned {spawned} treasures");
    spawned
}

fn open_neighbors(cell: (usize, usize), maze: &MazeMap) -> Vec<(usize, usize)> {
    cardinal_directions()
        .into_iter()
        .filter_map(|(direction, dx, dz)| {
            if maze.cells[cell.1][cell.0].walls[direction.index()] {
                return None;
            }

            let nx = cell.0 as isize + dx;
            let nz = cell.1 as isize + dz;
            if nx < 0 || nz < 0 || nx >= MAZE_COLUMNS as isize || nz >= MAZE_ROWS as isize {
                return None;
            }

            Some((nx as usize, nz as usize))
        })
        .collect()
}

fn spawn_key_door_and_chests(
    commands: &mut Commands,
    maze: &MazeMap,
    start_position: Vec3,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let start_cell = world_to_cell(start_position).unwrap_or((0, 0));
    let exit_cell = (MAZE_COLUMNS - 1, MAZE_ROWS - 1);
    let key_cell = open_neighbors(start_cell, maze)
        .into_iter()
        .find(|cell| *cell != exit_cell)
        .unwrap_or(start_cell);
    let key_center = cell_center(key_cell.0, key_cell.1);
    let key_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.92, 0.1),
        emissive: LinearRgba::rgb(0.45, 0.32, 0.0),
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(18.0, 6.0, 10.0))),
        MeshMaterial3d(key_material),
        Transform::from_xyz(key_center.x + 18.0, 8.0, key_center.y),
        KeyPickup,
    ));
    info!("Key spawned at cell {:?}", key_cell);

    if let Some(door_neighbor) = open_neighbors(exit_cell, maze).first().copied() {
        if let Some(door_direction) = direction_between(door_neighbor, exit_cell) {
            let door_center = edge_center(door_neighbor, door_direction);
            let vertical = matches!(door_direction, Direction::East | Direction::West);
            let door_mesh = if vertical {
                meshes.add(Cuboid::new(8.0, 62.0, 56.0))
            } else {
                meshes.add(Cuboid::new(56.0, 62.0, 8.0))
            };
            let half_extents = if vertical {
                Vec2::new(4.0, 28.0)
            } else {
                Vec2::new(28.0, 4.0)
            };
            let door_material = materials.add(StandardMaterial {
                base_color: Color::srgb(0.05, 0.22, 0.95),
                emissive: LinearRgba::rgb(0.0, 0.04, 0.35),
                perceptual_roughness: 0.35,
                ..default()
            });
            commands.spawn((
                Mesh3d(door_mesh),
                MeshMaterial3d(door_material),
                Transform::from_xyz(door_center.x, 31.0, door_center.y),
                WallCollider {
                    center: door_center,
                    half_extents,
                },
                LockedDoor,
            ));
            info!("Locked door spawned before exit at cell {:?}", exit_cell);
        }
    }

    let chest_mesh = meshes.add(Cuboid::new(22.0, 18.0, 18.0));
    let chest_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.48, 0.22, 0.08),
        emissive: LinearRgba::rgb(0.12, 0.05, 0.01),
        perceptual_roughness: 0.7,
        ..default()
    });
    let mut spawned_chests = 0;
    for z in 0..MAZE_ROWS {
        for x in 0..MAZE_COLUMNS {
            if spawned_chests >= 3
                || (x, z) == start_cell
                || (x, z) == exit_cell
                || (x, z) == key_cell
            {
                continue;
            }
            let open_edges = maze.cells[z][x].walls.iter().filter(|wall| !**wall).count();
            if open_edges != 1 {
                continue;
            }

            let center = cell_center(x, z);
            commands.spawn((
                Mesh3d(chest_mesh.clone()),
                MeshMaterial3d(chest_material.clone()),
                Transform::from_xyz(center.x, 10.0, center.y),
                Chest,
            ));
            spawned_chests += 1;
        }
    }
    info!("Spawned {spawned_chests} chests");
}

fn collect_treasure(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    treasure_query: Query<(Entity, &Transform, &Treasure)>,
    mut run_state: ResMut<RunState>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_xz = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.z,
    );

    for (entity, transform, treasure) in treasure_query.iter() {
        let treasure_xz = Vec2::new(transform.translation.x, transform.translation.z);
        if player_xz.distance(treasure_xz) < 28.0 {
            run_state.treasure += treasure.value;
            commands.entity(entity).despawn();
            info!(
                "Collected treasure {}/{}",
                run_state.treasure, run_state.total_treasure
            );
        }
    }
}

fn collect_ore(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    ore_query: Query<(Entity, &Transform, &Ore)>,
    mut run_state: ResMut<RunState>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_xz = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.z,
    );

    for (entity, transform, ore) in ore_query.iter() {
        let ore_xz = Vec2::new(transform.translation.x, transform.translation.z);
        if player_xz.distance(ore_xz) < 28.0 {
            run_state.ore += ore.value;
            commands.entity(entity).despawn();
            info!("Collected ore {}", run_state.ore);
        }
    }
}

fn collect_key(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    key_query: Query<(Entity, &Transform), With<KeyPickup>>,
    mut run_state: ResMut<RunState>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_xz = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.z,
    );
    for (entity, transform) in key_query.iter() {
        let key_xz = Vec2::new(transform.translation.x, transform.translation.z);
        if player_xz.distance(key_xz) < 30.0 {
            run_state.keys += 1;
            commands.entity(entity).despawn();
            info!("Collected key; keys={}", run_state.keys);
        }
    }
}

fn unlock_door(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    door_query: Query<(Entity, &WallCollider), With<LockedDoor>>,
    mut run_state: ResMut<RunState>,
) {
    if run_state.keys == 0 {
        return;
    }

    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_xz = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.z,
    );
    for (entity, collider) in door_query.iter() {
        if player_xz.distance(collider.center) < 58.0 {
            run_state.keys -= 1;
            run_state.doors_unlocked += 1;
            commands.entity(entity).despawn();
            info!("Unlocked exit door");
        }
    }
}

fn open_chest(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    chest_query: Query<(Entity, &Transform), With<Chest>>,
    mut run_state: ResMut<RunState>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_xz = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.z,
    );
    for (entity, transform) in chest_query.iter() {
        let chest_xz = Vec2::new(transform.translation.x, transform.translation.z);
        if player_xz.distance(chest_xz) < 32.0 {
            run_state.treasure += 2;
            run_state.ore += 1;
            run_state.chests_opened += 1;
            commands.entity(entity).despawn();
            info!(
                "Opened chest {}; +2 treasure, +1 ore",
                run_state.chests_opened
            );
        }
    }
}

fn update_hud(run_state: Res<RunState>, mut hud_query: Query<&mut Text, With<HudText>>) {
    if !run_state.is_changed() {
        return;
    }

    let Ok(mut text) = hud_query.single_mut() else {
        return;
    };
    text.0 = hud_text(&run_state);
}

fn hud_text(run_state: &RunState) -> String {
    let objective = if run_state.won {
        "Escaped!"
    } else {
        "Find the smiley exit"
    };
    format!(
        "Treasure: {}/{}  Ore: {}  Keys: {}  Mined: {}\nQ/E: turn  T: marker  F: mine\n{}",
        run_state.treasure,
        run_state.total_treasure,
        run_state.ore,
        run_state.keys,
        run_state.mined_walls,
        objective
    )
}

fn mine_wall(
    keyboard: Res<ButtonInput<KeyCode>>,
    player_query: Query<&Transform, With<Player>>,
    orbit: Res<CameraOrbit>,
    mut maze: ResMut<MazeMap>,
    wall_query: Query<(Entity, &WallSegment, &WallCollider)>,
    mining_assets: Res<MiningAssets>,
    mut run_state: ResMut<RunState>,
    mut commands: Commands,
) {
    if !keyboard.just_pressed(KeyCode::KeyF) {
        return;
    }

    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let Some(cell) = world_to_cell(player_transform.translation) else {
        return;
    };
    let direction = facing_direction(orbit.yaw);
    if !maze.cells[cell.1][cell.0].walls[direction.index()] {
        info!(
            "No wall to mine at cell {:?} facing {:?}",
            cell,
            direction.index()
        );
        return;
    }

    let Some((entity, segment, collider)) = wall_query
        .iter()
        .find(|(_, segment, _)| same_wall_edge(segment.cell, segment.direction, cell, direction))
    else {
        info!(
            "No wall entity found at cell {:?} facing {:?}",
            cell,
            direction.index()
        );
        return;
    };

    if !segment.mineable {
        info!("Boundary wall cannot be mined at cell {:?}", cell);
        return;
    }

    commands.entity(entity).despawn();
    maze.cells[cell.1][cell.0].walls[direction.index()] = false;
    if let Some(neighbor) = neighbor_cell(cell, direction) {
        maze.cells[neighbor.1][neighbor.0].walls[direction.opposite().index()] = false;
    }
    commands.spawn((
        Mesh3d(mining_assets.ore_mesh.clone()),
        MeshMaterial3d(mining_assets.ore_material.clone()),
        Transform::from_xyz(collider.center.x, 12.0, collider.center.y),
        Ore { value: 1 },
    ));
    run_state.mined_walls += 1;
    info!(
        "Mined wall at cell {:?} facing {:?}",
        cell,
        direction.index()
    );
}

fn facing_direction(yaw: f32) -> Direction {
    let forward = Vec2::new(-yaw.sin(), -yaw.cos());
    if forward.x.abs() > forward.y.abs() {
        if forward.x > 0.0 {
            Direction::East
        } else {
            Direction::West
        }
    } else if forward.y > 0.0 {
        Direction::North
    } else {
        Direction::South
    }
}

fn same_wall_edge(
    segment_cell: (usize, usize),
    segment_direction: Direction,
    cell: (usize, usize),
    direction: Direction,
) -> bool {
    if segment_cell == cell && segment_direction == direction {
        return true;
    }

    neighbor_cell(cell, direction).is_some_and(|neighbor| {
        segment_cell == neighbor && segment_direction == direction.opposite()
    })
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

fn edge_center(cell: (usize, usize), direction: Direction) -> Vec2 {
    let center = cell_center(cell.0, cell.1);
    let (dx, dz) = direction.delta();
    center + Vec2::new(dx as f32, dz as f32) * (maze::CELL_SIZE * 0.5)
}

fn drop_trail_marker(
    keyboard: Res<ButtonInput<KeyCode>>,
    player_query: Query<&Transform, With<Player>>,
    mut markers: ResMut<TrailMarkers>,
    mut commands: Commands,
) {
    if !keyboard.just_pressed(KeyCode::KeyT) {
        return;
    }

    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let Some(cell) = world_to_cell(player_transform.translation) else {
        return;
    };
    if markers.marked[cell.1][cell.0] {
        info!("Trail marker already exists at cell {:?}", cell);
        return;
    }

    markers.marked[cell.1][cell.0] = true;
    let center = cell_center(cell.0, cell.1);
    commands.spawn((
        Mesh3d(markers.mesh.clone()),
        MeshMaterial3d(markers.material.clone()),
        Transform::from_xyz(center.x, 1.4, center.y),
        TrailMarker,
    ));
    info!("Dropped trail marker at cell {:?}", cell);
}

fn check_exit_reached(
    player_query: Query<&Transform, With<Player>>,
    exit_query: Query<&Transform, With<Exit>>,
    mut run_state: ResMut<RunState>,
) {
    if run_state.won {
        return;
    }

    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let Ok(exit_transform) = exit_query.single() else {
        return;
    };

    let player_xz = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.z,
    );
    let exit_xz = Vec2::new(exit_transform.translation.x, exit_transform.translation.z);
    if player_xz.distance(exit_xz) < 32.0 {
        run_state.won = true;
        info!("You found the smiley face exit!");
    }
}

fn rat_movement(
    time: Res<Time>,
    maze: Res<MazeMap>,
    mut rat_query: Query<(&mut Transform, &mut Rat)>,
) {
    for (mut transform, mut rat) in rat_query.iter_mut() {
        if rat.cell == rat.target_cell {
            rat.target_cell = choose_rat_target(rat.cell, rat.last_direction, &maze);
        }

        let target_center = cell_center(rat.target_cell.0, rat.target_cell.1);
        let target = Vec3::new(target_center.x, transform.translation.y, target_center.y);
        let to_target = target - transform.translation;
        let distance = to_target.length();

        if distance <= 2.0 {
            if rat.cell != rat.target_cell {
                rat.last_direction = direction_between(rat.cell, rat.target_cell);
                rat.cell = rat.target_cell;
            }
            rat.target_cell = choose_rat_target(rat.cell, rat.last_direction, &maze);
            continue;
        }

        let step = 68.0 * time.delta_secs();
        let movement = to_target.normalize() * step.min(distance);
        transform.translation += movement;
        transform.rotation = Quat::from_rotation_y(movement.x.atan2(movement.z));
    }
}

fn choose_rat_target(
    cell: (usize, usize),
    last_direction: Option<Direction>,
    maze: &MazeMap,
) -> (usize, usize) {
    let mut options = rat_neighbors(cell, maze);
    if let Some(last_direction) = last_direction {
        let reverse = last_direction.opposite();
        if options.len() > 1 {
            options.retain(|(direction, _)| *direction != reverse);
        }
    }

    let mut rng = rand::thread_rng();
    options
        .choose(&mut rng)
        .map(|(_, cell)| *cell)
        .unwrap_or(cell)
}

fn rat_neighbors(cell: (usize, usize), maze: &MazeMap) -> Vec<(Direction, (usize, usize))> {
    cardinal_directions()
        .into_iter()
        .filter_map(|(direction, dx, dz)| {
            if maze.cells[cell.1][cell.0].walls[direction.index()] {
                return None;
            }

            let nx = cell.0 as isize + dx;
            let nz = cell.1 as isize + dz;
            if nx < 0 || nz < 0 || nx >= MAZE_COLUMNS as isize || nz >= MAZE_ROWS as isize {
                return None;
            }

            let next = (nx as usize, nz as usize);
            if maze.cells[next.1][next.0].walls[direction.opposite().index()] {
                return None;
            }

            Some((direction, next))
        })
        .collect()
}

fn direction_between(from: (usize, usize), to: (usize, usize)) -> Option<Direction> {
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

fn camera_drag(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    time: Res<Time>,
    mut orbit: ResMut<CameraOrbit>,
) {
    let keyboard_rotation = 2.4 * time.delta_secs();
    if keyboard.pressed(KeyCode::KeyQ) {
        orbit.yaw = (orbit.yaw - keyboard_rotation).rem_euclid(std::f32::consts::TAU);
    }
    if keyboard.pressed(KeyCode::KeyE) {
        orbit.yaw = (orbit.yaw + keyboard_rotation).rem_euclid(std::f32::consts::TAU);
    }

    let mut total_motion = Vec2::ZERO;
    for event in mouse_motion.read() {
        total_motion += event.delta;
    }

    let dragging =
        mouse_button.pressed(MouseButton::Left) || mouse_button.pressed(MouseButton::Right);
    if !dragging || total_motion.length_squared() < 0.1 {
        return;
    }

    let sensitivity = 0.006;
    orbit.yaw = (orbit.yaw - total_motion.x * sensitivity).rem_euclid(std::f32::consts::TAU);
    orbit.pitch = (orbit.pitch + total_motion.y * sensitivity).clamp(-0.65, 0.75);
}

fn camera_follow(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<FollowCamera>, Without<Player>)>,
    mut light_query: Query<
        &mut Transform,
        (With<PlayerLight>, Without<Player>, Without<FollowCamera>),
    >,
    orbit: Res<CameraOrbit>,
    wall_query: Query<&WallCollider>,
) {
    if let Ok(player_transform) = player_query.single() {
        let target = player_transform.translation + Vec3::Y * 28.0;
        let horizontal_distance = orbit.distance * orbit.pitch.cos();
        let offset = Vec3::new(
            orbit.yaw.sin() * horizontal_distance,
            orbit.distance * orbit.pitch.sin() + 20.0,
            orbit.yaw.cos() * horizontal_distance,
        );
        let desired_position = target + offset;
        let camera_position = constrain_camera_to_maze(target, desired_position, &wall_query);

        if let Ok(mut camera_transform) = camera_query.single_mut() {
            camera_transform.translation = camera_position;
            camera_transform.look_at(target, Vec3::Y);
        }

        if let Ok(mut light_transform) = light_query.single_mut() {
            light_transform.translation = player_transform.translation + Vec3::Y * 38.0;
        }
    }
}

fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    orbit: Res<CameraOrbit>,
    mut player_query: Query<&mut Transform, With<Player>>,
    wall_query: Query<&WallCollider>,
) {
    let forward = Vec3::new(-orbit.yaw.sin(), 0.0, -orbit.yaw.cos()).normalize();
    let right = Vec3::new(-forward.z, 0.0, forward.x).normalize();
    let mut direction = Vec3::ZERO;

    let Ok(mut transform) = player_query.single_mut() else {
        return;
    };

    transform.rotation = Quat::from_rotation_y(orbit.yaw - std::f32::consts::PI);

    if keyboard.pressed(KeyCode::KeyW) {
        direction += forward;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction -= forward;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction -= right;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction += right;
    }

    if direction.length() <= 0.0 {
        return;
    }

    direction = direction.normalize();

    let speed = 105.0;
    let delta = direction * speed * time.delta_secs();
    let radius = 15.0;

    let mut candidate = transform.translation;
    candidate.x += delta.x;
    if !collides(candidate, radius, &wall_query) {
        transform.translation.x = candidate.x;
    }

    candidate = transform.translation;
    candidate.z += delta.z;
    if !collides(candidate, radius, &wall_query) {
        transform.translation.z = candidate.z;
    }
}

fn collides(position: Vec3, radius: f32, walls: &Query<&WallCollider>) -> bool {
    let player = Vec2::new(position.x, position.z);
    walls
        .iter()
        .any(|wall| circle_intersects_aabb(player, radius, wall.center, wall.half_extents))
}

fn circle_intersects_aabb(point: Vec2, radius: f32, center: Vec2, half_extents: Vec2) -> bool {
    let min = center - half_extents;
    let max = center + half_extents;
    let closest = Vec2::new(point.x.clamp(min.x, max.x), point.y.clamp(min.y, max.y));
    point.distance_squared(closest) < radius * radius
}

fn constrain_camera_to_maze(target: Vec3, desired: Vec3, walls: &Query<&WallCollider>) -> Vec3 {
    let start = Vec2::new(target.x, target.z);
    let end = Vec2::new(desired.x, desired.z);
    let segment = end - start;
    let segment_length = segment.length();
    if segment_length <= 0.1 {
        return desired;
    }

    let mut allowed_t: f32 = 1.0;
    let camera_radius = 10.0;
    for wall in walls.iter() {
        let half_extents = wall.half_extents + Vec2::splat(camera_radius);
        if let Some(t) = segment_aabb_intersection(start, end, wall.center, half_extents) {
            if t > 0.03 {
                allowed_t = allowed_t.min(t);
            }
        }
    }

    if allowed_t >= 1.0 {
        return Vec3::new(desired.x, desired.y.clamp(14.0, 62.0), desired.z);
    }

    let backed_off = (allowed_t * segment_length - 12.0).max(36.0);
    let camera_xz = start + segment.normalize() * backed_off;
    Vec3::new(camera_xz.x, desired.y.clamp(14.0, 62.0), camera_xz.y)
}

fn segment_aabb_intersection(
    start: Vec2,
    end: Vec2,
    center: Vec2,
    half_extents: Vec2,
) -> Option<f32> {
    let min = center - half_extents;
    let max = center + half_extents;
    let direction = end - start;
    let mut t_min: f32 = 0.0;
    let mut t_max: f32 = 1.0;

    for axis in 0..2 {
        let origin = if axis == 0 { start.x } else { start.y };
        let delta = if axis == 0 { direction.x } else { direction.y };
        let min_axis = if axis == 0 { min.x } else { min.y };
        let max_axis = if axis == 0 { max.x } else { max.y };

        if delta.abs() < f32::EPSILON {
            if origin < min_axis || origin > max_axis {
                return None;
            }
            continue;
        }

        let inverse_delta = 1.0 / delta;
        let mut t1 = (min_axis - origin) * inverse_delta;
        let mut t2 = (max_axis - origin) * inverse_delta;
        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
        }
        t_min = t_min.max(t1);
        t_max = t_max.min(t2);
        if t_min > t_max {
            return None;
        }
    }

    Some(t_min)
}

fn update_fog(
    player_query: Query<&Transform, With<Player>>,
    maze: Res<MazeMap>,
    mut fog_memory: ResMut<FogMemory>,
    fog_query: Query<&FogCell>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let Some(origin) = world_to_cell(player_transform.translation) else {
        return;
    };

    let mut alpha = [[0.92_f32; MAZE_COLUMNS]; MAZE_ROWS];
    let mut visible = [[false; MAZE_COLUMNS]; MAZE_ROWS];
    reveal_cell(origin, &mut visible, &mut alpha, &mut fog_memory);

    for (direction, dx, dz) in cardinal_directions() {
        let mut current = origin;
        loop {
            if maze.cells[current.1][current.0].walls[direction.index()] {
                break;
            }

            let nx = current.0 as isize + dx;
            let nz = current.1 as isize + dz;
            if nx < 0 || nz < 0 || nx >= MAZE_COLUMNS as isize || nz >= MAZE_ROWS as isize {
                break;
            }

            let next = (nx as usize, nz as usize);
            if maze.cells[next.1][next.0].walls[direction.opposite().index()] {
                break;
            }

            reveal_cell(next, &mut visible, &mut alpha, &mut fog_memory);
            current = next;
        }
    }

    for z in 0..MAZE_ROWS {
        for x in 0..MAZE_COLUMNS {
            if fog_memory.seen[z][x] && !visible[z][x] {
                alpha[z][x] = 0.72;
            }
        }
    }

    for z in 0..MAZE_ROWS {
        for x in 0..MAZE_COLUMNS {
            if !visible[z][x] {
                continue;
            }
            for (direction, dx, dz) in cardinal_directions() {
                if maze.cells[z][x].walls[direction.index()] {
                    continue;
                }

                let nx = x as isize + dx;
                let nz = z as isize + dz;
                if nx < 0 || nz < 0 || nx >= MAZE_COLUMNS as isize || nz >= MAZE_ROWS as isize {
                    continue;
                }
                let nx = nx as usize;
                let nz = nz as usize;
                if maze.cells[nz][nx].walls[direction.opposite().index()] {
                    continue;
                }

                if !visible[nz][nx] {
                    alpha[nz][nx] = alpha[nz][nx].min(0.55);
                }
            }
        }
    }

    for fog in fog_query.iter() {
        if let Some(material) = materials.get_mut(&fog.material) {
            material.base_color = Color::srgba(0.0, 0.0, 0.0, alpha[fog.z][fog.x]);
        }
    }
}

fn reveal_cell(
    cell: (usize, usize),
    visible: &mut [[bool; MAZE_COLUMNS]; MAZE_ROWS],
    alpha: &mut [[f32; MAZE_COLUMNS]; MAZE_ROWS],
    fog_memory: &mut FogMemory,
) {
    visible[cell.1][cell.0] = true;
    fog_memory.seen[cell.1][cell.0] = true;
    alpha[cell.1][cell.0] = 0.02;
}

fn cardinal_directions() -> [(Direction, isize, isize); 4] {
    [
        (Direction::North, 0, 1),
        (Direction::East, 1, 0),
        (Direction::South, 0, -1),
        (Direction::West, -1, 0),
    ]
}
