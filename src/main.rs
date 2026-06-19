use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy_sprite3d::prelude::*;
use rand::seq::SliceRandom;
use std::collections::VecDeque;
mod maze;
use maze::{
    Direction, Floor, FogCell, MAZE_COLUMNS, MAZE_ROWS, MazeMap, Roof, Wall, WallCollider,
    WallSegment, cell_center, create_maze, world_to_cell,
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
    hunting: bool,
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

#[derive(Component)]
struct Billboard;

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
    caught_by_rat: bool,
    energy: u32,
    max_energy: u32,
    pickaxe_level: u32,
    elapsed_seconds: f32,
    score: u32,
    best_score: u32,
    score_finalized: bool,
}

#[derive(Resource)]
struct TrailMarkers {
    marked: [[bool; MAZE_COLUMNS]; MAZE_ROWS],
    image: Handle<Image>,
}

#[derive(Resource)]
struct MiningAssets {
    ore_image: Handle<Image>,
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
    treasure: Handle<Image>,
    ore: Handle<Image>,
    key: Handle<Image>,
    locked_door: Handle<Image>,
    chest: Handle<Image>,
    trail_marker: Handle<Image>,
    pickup_sound: Handle<AudioSource>,
    mine_sound: Handle<AudioSource>,
    unlock_sound: Handle<AudioSource>,
    chest_sound: Handle<AudioSource>,
    upgrade_sound: Handle<AudioSource>,
    marker_sound: Handle<AudioSource>,
    catch_sound: Handle<AudioSource>,
    win_sound: Handle<AudioSource>,
    loaded: bool,
}

// Define game states
#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
enum GameState {
    #[default]
    Loading,
    Ready,
    Restarting,
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: asset_file_path(),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
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
        .add_systems(OnEnter(GameState::Restarting), cleanup_run)
        .add_systems(
            Update,
            (
                update_run_timer,
                player_movement,
                drop_trail_marker,
                mine_wall,
                rat_movement,
                camera_drag,
                camera_follow,
                billboard_to_camera,
                update_fog,
                collect_treasure,
                collect_ore,
                collect_key,
                unlock_door,
                open_chest,
                upgrade_pickaxe,
                update_score,
                finalize_score,
                restart_run,
                update_hud,
                check_exit_reached,
            )
                .run_if(in_state(GameState::Ready)),
        )
        .run();
}

fn asset_file_path() -> String {
    if let Some(packaged_assets) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("assets")))
        .filter(|assets| assets.join("images/monkey.png").is_file())
    {
        return packaged_assets.to_string_lossy().into_owned();
    }

    if let Some(source_assets) = std::env::current_dir()
        .ok()
        .map(|dir| dir.join("assets"))
        .filter(|assets| assets.join("images/monkey.png").is_file())
    {
        return source_assets.to_string_lossy().into_owned();
    }

    "assets".to_string()
}

fn load_assets(mut game_assets: ResMut<GameAssets>, asset_server: Res<AssetServer>) {
    info!("Loading assets...");
    game_assets.monkey = asset_server.load("images/monkey.png");
    game_assets.wall = asset_server.load("images/wall.png");
    game_assets.floor = asset_server.load("images/floor.png");
    game_assets.roof = asset_server.load("images/roof.png");
    game_assets.smiley_exit = asset_server.load("images/smiley_exit.png");
    game_assets.rat = asset_server.load("images/rat.png");
    game_assets.treasure = asset_server.load("images/treasure.png");
    game_assets.ore = asset_server.load("images/ore.png");
    game_assets.key = asset_server.load("images/key.png");
    game_assets.locked_door = asset_server.load("images/locked_door.png");
    game_assets.chest = asset_server.load("images/chest.png");
    game_assets.trail_marker = asset_server.load("images/trail_marker.png");
    game_assets.pickup_sound = asset_server.load("audio/pickup.ogg");
    game_assets.mine_sound = asset_server.load("audio/mine.ogg");
    game_assets.unlock_sound = asset_server.load("audio/unlock.ogg");
    game_assets.chest_sound = asset_server.load("audio/chest.ogg");
    game_assets.upgrade_sound = asset_server.load("audio/upgrade.ogg");
    game_assets.marker_sound = asset_server.load("audio/marker.ogg");
    game_assets.catch_sound = asset_server.load("audio/catch.ogg");
    game_assets.win_sound = asset_server.load("audio/win.ogg");
}

fn asset_loaded<T: Asset>(asset_server: &AssetServer, handle: &Handle<T>, label: &str) -> bool {
    let loaded = asset_server
        .get_load_state(handle)
        .is_some_and(|state| state.is_loaded());
    if !loaded {
        info!("{label} still loading...");
    }
    loaded
}

fn check_assets_ready(
    mut game_assets: ResMut<GameAssets>,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let images_loaded = [
        ("Monkey texture", &game_assets.monkey),
        ("Wall texture", &game_assets.wall),
        ("Floor texture", &game_assets.floor),
        ("Roof texture", &game_assets.roof),
        ("Smiley exit texture", &game_assets.smiley_exit),
        ("Rat texture", &game_assets.rat),
        ("Treasure texture", &game_assets.treasure),
        ("Ore texture", &game_assets.ore),
        ("Key texture", &game_assets.key),
        ("Locked door texture", &game_assets.locked_door),
        ("Chest texture", &game_assets.chest),
        ("Trail marker texture", &game_assets.trail_marker),
    ]
    .into_iter()
    .all(|(label, handle)| asset_loaded(&asset_server, handle, label));
    let sounds_loaded = [
        ("Pickup sound", &game_assets.pickup_sound),
        ("Mine sound", &game_assets.mine_sound),
        ("Unlock sound", &game_assets.unlock_sound),
        ("Chest sound", &game_assets.chest_sound),
        ("Upgrade sound", &game_assets.upgrade_sound),
        ("Marker sound", &game_assets.marker_sound),
        ("Catch sound", &game_assets.catch_sound),
        ("Win sound", &game_assets.win_sound),
    ]
    .into_iter()
    .all(|(label, handle)| asset_loaded(&asset_server, handle, label));

    if images_loaded && sounds_loaded {
        info!("All assets loaded successfully!");
        game_assets.loaded = true;
        next_state.set(GameState::Ready);
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
    run_state.caught_by_rat = false;
    run_state.max_energy = 3;
    run_state.energy = run_state.max_energy;
    run_state.pickaxe_level = 1;
    run_state.elapsed_seconds = 0.0;
    run_state.score = 0;
    run_state.score_finalized = false;
    run_state.won = false;

    commands.insert_resource(TrailMarkers {
        marked: [[false; MAZE_COLUMNS]; MAZE_ROWS],
        image: game_assets.trail_marker.clone(),
    });

    commands.insert_resource(MiningAssets {
        ore_image: game_assets.ore.clone(),
    });

    let total_treasure = spawn_treasures(
        &mut commands,
        &maze_map,
        game_assets.treasure.clone(),
        start_position,
    );
    run_state.total_treasure = total_treasure;
    spawn_key_door_and_chests(
        &mut commands,
        &maze_map,
        start_position,
        &game_assets,
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
        Billboard,
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
            hunting: false,
        },
    ));
    info!("Rat enemy spawned at cell {:?}", rat_cell);

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

fn cleanup_run(
    mut commands: Commands,
    run_entities: Query<
        Entity,
        Or<(
            With<Player>,
            With<FollowCamera>,
            With<PlayerLight>,
            With<Exit>,
            With<Rat>,
            With<TrailMarker>,
            With<Treasure>,
            With<Ore>,
            With<KeyPickup>,
            With<LockedDoor>,
            With<Chest>,
            With<HudText>,
        )>,
    >,
    maze_entities: Query<Entity, Or<(With<Wall>, With<Floor>, With<Roof>, With<FogCell>)>>,
    mut fog_memory: ResMut<FogMemory>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for entity in run_entities.iter().chain(maze_entities.iter()) {
        commands.entity(entity).despawn();
    }
    fog_memory.seen = [[false; MAZE_COLUMNS]; MAZE_ROWS];
    next_state.set(GameState::Ready);
    info!("Restarting run");
}

fn update_run_timer(time: Res<Time>, mut run_state: ResMut<RunState>) {
    if run_state.won || run_state.caught_by_rat {
        return;
    }
    run_state.elapsed_seconds += time.delta_secs();
}

fn score_for(run_state: &RunState) -> u32 {
    let base = run_state.treasure * 100
        + run_state.ore * 25
        + run_state.keys * 20
        + run_state.doors_unlocked * 150
        + run_state.chests_opened * 75
        + run_state.mined_walls * 10;
    let win_bonus = if run_state.won { 1_000 } else { 0 };
    let time_penalty = run_state.elapsed_seconds as u32;
    (base + win_bonus).saturating_sub(time_penalty)
}

fn update_score(mut run_state: ResMut<RunState>) {
    if run_state.score_finalized {
        return;
    }
    run_state.score = score_for(&run_state);
}

fn finalize_score(mut run_state: ResMut<RunState>) {
    if run_state.score_finalized || (!run_state.won && !run_state.caught_by_rat) {
        return;
    }

    run_state.score = score_for(&run_state);
    run_state.best_score = run_state.best_score.max(run_state.score);
    run_state.score_finalized = true;
    info!(
        "Final score {}; best {}",
        run_state.score, run_state.best_score
    );
}

fn restart_run(
    keyboard: Res<ButtonInput<KeyCode>>,
    run_state: Res<RunState>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::KeyR) && (run_state.won || run_state.caught_by_rat) {
        next_state.set(GameState::Restarting);
    }
}

fn spawn_treasures(
    commands: &mut Commands,
    maze: &MazeMap,
    image: Handle<Image>,
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
            Sprite::from_image(image.clone()),
            item_sprite(3.2),
            Transform::from_xyz(center.x, 2.0, center.y),
            Treasure { value: 1 },
            Billboard,
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
    game_assets: &GameAssets,
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
    commands.spawn((
        Sprite::from_image(game_assets.key.clone()),
        item_sprite(2.6),
        Transform::from_xyz(key_center.x + 18.0, 2.0, key_center.y),
        KeyPickup,
        Billboard,
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
                base_color_texture: Some(game_assets.locked_door.clone()),
                base_color: Color::WHITE,
                emissive: LinearRgba::rgb(0.0, 0.02, 0.12),
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
                Sprite::from_image(game_assets.chest.clone()),
                item_sprite(2.6),
                Transform::from_xyz(center.x, 2.0, center.y),
                Chest,
                Billboard,
            ));
            spawned_chests += 1;
        }
    }
    info!("Spawned {spawned_chests} chests");
}

fn item_sprite(pixels_per_metre: f32) -> Sprite3d {
    Sprite3d {
        pixels_per_metre,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        double_sided: true,
        pivot: Some(Vec2::new(0.5, 0.08)),
        ..Default::default()
    }
}

fn play_sound(commands: &mut Commands, sound: Handle<AudioSource>) {
    commands.spawn((AudioPlayer(sound), PlaybackSettings::DESPAWN));
}

fn collect_treasure(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    treasure_query: Query<(Entity, &Transform, &Treasure)>,
    game_assets: Res<GameAssets>,
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
            play_sound(&mut commands, game_assets.pickup_sound.clone());
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
    game_assets: Res<GameAssets>,
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
            play_sound(&mut commands, game_assets.pickup_sound.clone());
            info!("Collected ore {}", run_state.ore);
        }
    }
}

fn collect_key(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    key_query: Query<(Entity, &Transform), With<KeyPickup>>,
    game_assets: Res<GameAssets>,
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
            play_sound(&mut commands, game_assets.pickup_sound.clone());
            info!("Collected key; keys={}", run_state.keys);
        }
    }
}

fn unlock_door(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    door_query: Query<(Entity, &WallCollider), With<LockedDoor>>,
    game_assets: Res<GameAssets>,
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
            play_sound(&mut commands, game_assets.unlock_sound.clone());
            info!("Unlocked exit door");
        }
    }
}

fn open_chest(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    chest_query: Query<(Entity, &Transform), With<Chest>>,
    game_assets: Res<GameAssets>,
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
            play_sound(&mut commands, game_assets.chest_sound.clone());
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
    } else if run_state.caught_by_rat {
        "Caught by the rat!"
    } else {
        "Find the smiley exit"
    };
    format!(
        "Treasure: {}/{}  Ore: {}  Keys: {}  Energy: {}/{}\nPickaxe: {}  Mined: {}  Time: {:.0}s\nScore: {}  Best: {}\nQ/E: turn  T: marker  F: mine  U: upgrade{}\n{}",
        run_state.treasure,
        run_state.total_treasure,
        run_state.ore,
        run_state.keys,
        run_state.energy,
        run_state.max_energy,
        run_state.pickaxe_level,
        run_state.mined_walls,
        run_state.elapsed_seconds,
        run_state.score,
        run_state.best_score,
        if run_state.won || run_state.caught_by_rat {
            "  R: restart"
        } else {
            ""
        },
        objective
    )
}

fn upgrade_pickaxe(
    keyboard: Res<ButtonInput<KeyCode>>,
    game_assets: Res<GameAssets>,
    mut run_state: ResMut<RunState>,
    mut commands: Commands,
) {
    if !keyboard.just_pressed(KeyCode::KeyU) || run_state.won || run_state.caught_by_rat {
        return;
    }

    let cost = run_state.pickaxe_level;
    if run_state.ore < cost {
        info!("Need {cost} ore to upgrade pickaxe");
        return;
    }

    run_state.ore -= cost;
    run_state.pickaxe_level += 1;
    run_state.max_energy += 1;
    run_state.energy = run_state.max_energy;
    play_sound(&mut commands, game_assets.upgrade_sound.clone());
    info!(
        "Upgraded pickaxe to level {}; max energy {}",
        run_state.pickaxe_level, run_state.max_energy
    );
}

fn mine_wall(
    keyboard: Res<ButtonInput<KeyCode>>,
    player_query: Query<&Transform, With<Player>>,
    orbit: Res<CameraOrbit>,
    mut maze: ResMut<MazeMap>,
    wall_query: Query<(Entity, &WallSegment, &WallCollider)>,
    mining_assets: Res<MiningAssets>,
    game_assets: Res<GameAssets>,
    mut run_state: ResMut<RunState>,
    mut commands: Commands,
) {
    if !keyboard.just_pressed(KeyCode::KeyF) {
        return;
    }
    if run_state.energy == 0 {
        info!("Out of mining energy; upgrade your pickaxe with ore");
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
        Sprite::from_image(mining_assets.ore_image.clone()),
        item_sprite(2.4),
        Transform::from_xyz(collider.center.x, 2.0, collider.center.y),
        Ore {
            value: run_state.pickaxe_level,
        },
        Billboard,
    ));
    run_state.energy -= 1;
    run_state.mined_walls += 1;
    play_sound(&mut commands, game_assets.mine_sound.clone());
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
    game_assets: Res<GameAssets>,
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
        Sprite::from_image(markers.image.clone()),
        item_sprite(3.0),
        Transform::from_xyz(center.x, 1.0, center.y),
        TrailMarker,
        Billboard,
    ));
    play_sound(&mut commands, game_assets.marker_sound.clone());
    info!("Dropped trail marker at cell {:?}", cell);
}

fn check_exit_reached(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    exit_query: Query<&Transform, With<Exit>>,
    game_assets: Res<GameAssets>,
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
        play_sound(&mut commands, game_assets.win_sound.clone());
        info!("You found the smiley face exit!");
    }
}

fn rat_movement(
    mut commands: Commands,
    time: Res<Time>,
    maze: Res<MazeMap>,
    player_query: Query<&Transform, (With<Player>, Without<Rat>)>,
    mut rat_query: Query<(&mut Transform, &mut Rat)>,
    game_assets: Res<GameAssets>,
    mut run_state: ResMut<RunState>,
) {
    if run_state.won || run_state.caught_by_rat {
        return;
    }

    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_cell = world_to_cell(player_transform.translation);
    let player_xz = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.z,
    );

    for (mut transform, mut rat) in rat_query.iter_mut() {
        let rat_xz = Vec2::new(transform.translation.x, transform.translation.z);
        if rat_xz.distance(player_xz) < 24.0 {
            run_state.caught_by_rat = true;
            play_sound(&mut commands, game_assets.catch_sound.clone());
            info!("Caught by the rat at cell {:?}", rat.cell);
            return;
        }

        if rat.cell == rat.target_cell {
            if let Some(player_cell) = player_cell {
                if !rat.hunting {
                    rat.hunting = true;
                    info!("Rat picked up the trail");
                }
                rat.target_cell = next_step_toward(rat.cell, player_cell, &maze)
                    .unwrap_or_else(|| choose_rat_target(rat.cell, rat.last_direction, &maze));
            } else {
                rat.target_cell = choose_rat_target(rat.cell, rat.last_direction, &maze);
            }
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
            rat.target_cell = rat.cell;
            continue;
        }

        let step = 92.0 * time.delta_secs();
        let movement = to_target.normalize() * step.min(distance);
        transform.translation += movement;
        transform.rotation = Quat::from_rotation_y(movement.x.atan2(movement.z));
    }
}

fn next_step_toward(
    start: (usize, usize),
    goal: (usize, usize),
    maze: &MazeMap,
) -> Option<(usize, usize)> {
    if start == goal {
        return Some(goal);
    }

    let mut visited = [[false; MAZE_COLUMNS]; MAZE_ROWS];
    let mut previous = [[None; MAZE_COLUMNS]; MAZE_ROWS];
    let mut queue = VecDeque::new();
    visited[start.1][start.0] = true;
    queue.push_back(start);

    while let Some(cell) = queue.pop_front() {
        for next in open_neighbors(cell, maze) {
            if visited[next.1][next.0] {
                continue;
            }
            visited[next.1][next.0] = true;
            previous[next.1][next.0] = Some(cell);
            if next == goal {
                let mut step = next;
                while let Some(prev) = previous[step.1][step.0] {
                    if prev == start {
                        return Some(step);
                    }
                    step = prev;
                }
            }
            queue.push_back(next);
        }
    }

    None
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
        orbit.yaw = (orbit.yaw + keyboard_rotation).rem_euclid(std::f32::consts::TAU);
    }
    if keyboard.pressed(KeyCode::KeyE) {
        orbit.yaw = (orbit.yaw - keyboard_rotation).rem_euclid(std::f32::consts::TAU);
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

fn billboard_to_camera(
    camera_query: Query<&Transform, (With<FollowCamera>, Without<Billboard>)>,
    mut billboard_query: Query<&mut Transform, (With<Billboard>, Without<FollowCamera>)>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    for mut transform in billboard_query.iter_mut() {
        let to_camera = camera_transform.translation - transform.translation;
        if to_camera.xz().length_squared() < 0.01 {
            continue;
        }
        transform.rotation = Quat::from_rotation_y(to_camera.x.atan2(to_camera.z));
    }
}

fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    orbit: Res<CameraOrbit>,
    run_state: Res<RunState>,
    mut player_query: Query<&mut Transform, With<Player>>,
    wall_query: Query<&WallCollider>,
) {
    if run_state.won || run_state.caught_by_rat {
        return;
    }

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
