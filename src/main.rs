use bevy::audio::{SpatialScale, Volume};
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy_sprite3d::prelude::*;
use rand::Rng;
use rand::seq::SliceRandom;
use std::collections::VecDeque;
mod maze;
use maze::{
    Direction, Floor, FogCell, MAZE_COLUMNS, MAZE_ROWS, MazeMap, Roof, Wall, WallCollider,
    WallSegment, cell_center, create_maze, world_to_cell,
};

const MIN_KEY_SPAWN_DISTANCE: usize = 8;
const MIN_NAV_PICKUP_SPAWN_DISTANCE: usize = 5;
const ORE_WALL_BOUNCE_MARGIN: f32 = 10.0;
const RAT_SPAWN_CELL: (usize, usize) = (MAZE_COLUMNS / 2, MAZE_ROWS / 2);
const STARTING_TRAIL_MARKERS: u32 = 5;
const TRAIL_MARKERS_PER_TREASURE: u32 = 2;

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
    squeak_timer: Timer,
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
struct OreBurst {
    velocity: Vec3,
    timer: Timer,
}

#[derive(Component)]
struct KeyPickup;

#[derive(Component)]
struct CompassPickup;

#[derive(Component)]
struct TreasureMapPickup;

#[derive(Component)]
struct LockedDoor;

#[derive(Component)]
struct Chest;

#[derive(Component)]
struct HudText;

#[derive(Component)]
struct UpgradeMenuText;

#[derive(Component)]
struct CompassText;

#[derive(Component)]
struct TreasureMapText;

#[derive(Component)]
struct CompassArrow;

#[derive(Component)]
struct TreasureMapArrow;

#[derive(Component)]
struct FloatingMessage {
    timer: Timer,
}

#[derive(Component)]
struct StartScreenRoot;

#[derive(Component)]
struct StartPrompt;

#[derive(Component)]
struct WallShake {
    timer: Timer,
    original: Vec3,
}

#[derive(Component)]
struct Billboard;

#[derive(Component)]
struct RatAudioListener;

#[derive(Resource, Default)]
struct RunState {
    won: bool,
    treasure: u32,
    total_treasure: u32,
    ore: u32,
    markers: u32,
    mined_walls: u32,
    keys: u32,
    doors_unlocked: u32,
    chests_opened: u32,
    caught_by_rat: bool,
    energy: u32,
    max_energy: u32,
    pickaxe_level: u32,
    speed_level: u32,
    upgrade_menu_open: bool,
    upgrade_selection: usize,
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
struct HardWallAssets {
    textures: [Handle<Image>; 3],
}

#[derive(Resource, Default)]
struct NavigationState {
    key_cell: Option<(usize, usize)>,
    compass_cell: Option<(usize, usize)>,
    key_collected: bool,
    compass_collected: bool,
    treasure_map_collected: bool,
    door_unlocked: bool,
    show_treasure_map: bool,
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
    ore_wall: Handle<Image>,
    hard_wall: Handle<Image>,
    hard_wall_cracked_1: Handle<Image>,
    hard_wall_cracked_2: Handle<Image>,
    floor: Handle<Image>,
    roof: Handle<Image>,
    cover: Handle<Image>,
    smiley_exit: Handle<Image>,
    rat: Handle<Image>,
    treasure: Handle<Image>,
    treasure_map: Handle<Image>,
    compass: Handle<Image>,
    nav_arrow: Handle<Image>,
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
    rat_squeak_sound: Handle<AudioSource>,
    rat_squeak_echo_sound: Handle<AudioSource>,
    win_sound: Handle<AudioSource>,
    loaded: bool,
}

// Define game states
#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
enum GameState {
    #[default]
    Loading,
    Start,
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
        .init_resource::<NavigationState>()
        .init_state::<GameState>()
        .add_systems(Startup, load_assets)
        .add_systems(
            Update,
            check_assets_ready.run_if(in_state(GameState::Loading)),
        )
        .add_systems(OnEnter(GameState::Start), setup_start_screen)
        .add_systems(OnExit(GameState::Start), cleanup_start_screen)
        .add_systems(OnEnter(GameState::Ready), setup)
        .add_systems(OnEnter(GameState::Restarting), cleanup_run)
        .add_systems(
            Update,
            (start_screen_input, blink_start_prompt).run_if(in_state(GameState::Start)),
        )
        .add_systems(
            Update,
            (
                update_run_timer,
                player_movement,
                drop_trail_marker,
                mine_wall,
                update_ore_bursts,
                update_wall_shake,
                update_floating_messages,
                rat_movement,
                toggle_treasure_map,
                camera_drag,
                update_navigation_ui,
                update_audio_listener,
                rat_squeaks,
                camera_follow,
                billboard_to_camera,
                update_fog,
            )
                .run_if(in_state(GameState::Ready)),
        )
        .add_systems(
            Update,
            (
                collect_treasure,
                collect_ore,
                collect_navigation_pickups,
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
    game_assets.monkey = asset_server.load("images/monkey_red_shirt.png");
    game_assets.wall = asset_server.load("images/wall.png");
    game_assets.ore_wall = asset_server.load("images/ore_wall.png");
    game_assets.hard_wall = asset_server.load("images/hard_wall.png");
    game_assets.hard_wall_cracked_1 = asset_server.load("images/hard_wall_cracked_1.png");
    game_assets.hard_wall_cracked_2 = asset_server.load("images/hard_wall_cracked_2.png");
    game_assets.floor = asset_server.load("images/floor.png");
    game_assets.roof = asset_server.load("images/roof.png");
    game_assets.cover = asset_server.load("images/monkey-miner-cover.png");
    game_assets.smiley_exit = asset_server.load("images/smiley_exit.png");
    game_assets.rat = asset_server.load("images/rat.png");
    game_assets.treasure = asset_server.load("images/treasure.png");
    game_assets.treasure_map = asset_server.load("images/treasure_map.png");
    game_assets.compass = asset_server.load("images/compass.png");
    game_assets.nav_arrow = asset_server.load("images/nav_arrow.png");
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
    game_assets.rat_squeak_sound = asset_server.load("audio/rat_squeak.ogg");
    game_assets.rat_squeak_echo_sound = asset_server.load("audio/rat_squeak_echo.ogg");
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
        ("Ore wall texture", &game_assets.ore_wall),
        ("Hard wall texture", &game_assets.hard_wall),
        (
            "Hard wall crack 1 texture",
            &game_assets.hard_wall_cracked_1,
        ),
        (
            "Hard wall crack 2 texture",
            &game_assets.hard_wall_cracked_2,
        ),
        ("Floor texture", &game_assets.floor),
        ("Roof texture", &game_assets.roof),
        ("Cover texture", &game_assets.cover),
        ("Smiley exit texture", &game_assets.smiley_exit),
        ("Rat texture", &game_assets.rat),
        ("Treasure texture", &game_assets.treasure),
        ("Treasure map texture", &game_assets.treasure_map),
        ("Compass texture", &game_assets.compass),
        ("Navigation arrow texture", &game_assets.nav_arrow),
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
        ("Rat squeak sound", &game_assets.rat_squeak_sound),
        ("Rat squeak echo sound", &game_assets.rat_squeak_echo_sound),
        ("Win sound", &game_assets.win_sound),
    ]
    .into_iter()
    .all(|(label, handle)| asset_loaded(&asset_server, handle, label));

    if images_loaded && sounds_loaded {
        info!("All assets loaded successfully!");
        game_assets.loaded = true;
        next_state.set(GameState::Start);
    }
}

fn setup_start_screen(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.spawn((Camera2d, StartScreenRoot));
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.02, 0.015, 0.01)),
            StartScreenRoot,
        ))
        .with_children(|parent| {
            parent
                .spawn(Node {
                    width: Val::Px(650.0),
                    height: Val::Px(650.0),
                    position_type: PositionType::Relative,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|cover| {
                    cover.spawn((
                        ImageNode::new(game_assets.cover.clone()),
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                    ));
                    cover.spawn((
                        Text::new("Press Enter to mine"),
                        TextFont {
                            font_size: 34.0,
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 0.92, 0.25, 1.0)),
                        Node {
                            position_type: PositionType::Absolute,
                            bottom: Val::Px(30.0),
                            ..default()
                        },
                        StartPrompt,
                    ));
                });
        });
}

fn start_screen_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        next_state.set(GameState::Ready);
    }
}

fn cleanup_start_screen(mut commands: Commands, query: Query<Entity, With<StartScreenRoot>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn blink_start_prompt(time: Res<Time>, mut query: Query<&mut TextColor, With<StartPrompt>>) {
    let alpha = if (time.elapsed_secs() * 2.3).sin() > 0.0 {
        1.0
    } else {
        0.22
    };
    for mut color in query.iter_mut() {
        color.0 = Color::srgba(1.0, 0.92, 0.25, alpha);
    }
}

fn setup(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    mut run_state: ResMut<RunState>,
    mut navigation_state: ResMut<NavigationState>,
    mut orbit: ResMut<CameraOrbit>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Setting up game...");

    let (start_position, maze_map) = create_maze(
        &mut commands,
        game_assets.wall.clone(),
        game_assets.ore_wall.clone(),
        [
            game_assets.hard_wall.clone(),
            game_assets.hard_wall_cracked_1.clone(),
            game_assets.hard_wall_cracked_2.clone(),
        ],
        game_assets.floor.clone(),
        game_assets.roof.clone(),
        &mut meshes,
        &mut materials,
    );

    info!("Monkey starting position: {:?}", start_position);
    let spawn_cell = world_to_cell(start_position).unwrap_or((0, 0));
    let spawn_direction = best_spawn_direction(spawn_cell, &maze_map);
    let spawn_yaw = yaw_for_direction(spawn_direction);
    let default_orbit = CameraOrbit::default();
    orbit.yaw = spawn_yaw;
    orbit.pitch = default_orbit.pitch;
    orbit.distance = default_orbit.distance;
    info!(
        "Spawn facing {:?} toward the more open maze direction",
        spawn_direction.index()
    );

    run_state.treasure = 0;
    run_state.ore = 0;
    run_state.markers = STARTING_TRAIL_MARKERS;
    run_state.mined_walls = 0;
    run_state.keys = 0;
    run_state.doors_unlocked = 0;
    run_state.chests_opened = 0;
    run_state.caught_by_rat = false;
    run_state.max_energy = 3;
    run_state.energy = run_state.max_energy;
    run_state.pickaxe_level = 1;
    run_state.speed_level = 1;
    run_state.upgrade_menu_open = false;
    run_state.upgrade_selection = 0;
    run_state.elapsed_seconds = 0.0;
    run_state.score = 0;
    run_state.score_finalized = false;
    run_state.won = false;
    navigation_state.key_cell = None;
    navigation_state.compass_cell = None;
    navigation_state.key_collected = false;
    navigation_state.compass_collected = false;
    navigation_state.treasure_map_collected = false;
    navigation_state.door_unlocked = false;
    navigation_state.show_treasure_map = false;

    commands.insert_resource(TrailMarkers {
        marked: [[false; MAZE_COLUMNS]; MAZE_ROWS],
        image: game_assets.trail_marker.clone(),
    });

    commands.insert_resource(HardWallAssets {
        textures: [
            game_assets.hard_wall.clone(),
            game_assets.hard_wall_cracked_1.clone(),
            game_assets.hard_wall_cracked_2.clone(),
        ],
    });

    let loose_treasure = spawn_treasures(
        &mut commands,
        &maze_map,
        game_assets.treasure.clone(),
        start_position,
    );
    let (chest_treasure, key_cell) = spawn_key_door_and_chests(
        &mut commands,
        &maze_map,
        start_position,
        &game_assets,
        &mut meshes,
        &mut materials,
    );
    navigation_state.key_cell = Some(key_cell);
    info!("Navigation key target stored at cell {:?}", key_cell);
    let (compass_cell, treasure_map_cell) = spawn_navigation_pickups(
        &mut commands,
        &maze_map,
        start_position,
        key_cell,
        &game_assets,
    );
    navigation_state.compass_cell = Some(compass_cell);
    info!(
        "Navigation pickups spawned: compass {:?}, treasure map {:?}",
        compass_cell, treasure_map_cell
    );
    run_state.total_treasure = loose_treasure + chest_treasure;

    commands.spawn((
        Text::new(hud_text(&run_state, &navigation_state)),
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

    commands.spawn((
        Text::new(upgrade_menu_text(&run_state)),
        TextFont {
            font_size: 22.0,
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.95, 0.75)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(20.0),
            bottom: Val::Px(20.0),
            ..default()
        },
        UpgradeMenuText,
    ));

    commands.spawn((
        Text::new(String::new()),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.86, 0.25)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(20.0),
            top: Val::Px(18.0),
            ..default()
        },
        CompassText,
    ));

    commands.spawn((
        ImageNode {
            image: game_assets.nav_arrow.clone(),
            color: Color::srgba(1.0, 1.0, 1.0, 0.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(48.0),
            top: Val::Px(62.0),
            width: Val::Px(72.0),
            height: Val::Px(72.0),
            ..default()
        },
        UiTransform::IDENTITY,
        CompassArrow,
    ));

    commands.spawn((
        Text::new(String::new()),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::srgb(0.98, 0.80, 0.42)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(20.0),
            top: Val::Px(135.0),
            ..default()
        },
        TreasureMapText,
    ));

    commands.spawn((
        ImageNode {
            image: game_assets.nav_arrow.clone(),
            color: Color::srgba(1.0, 1.0, 1.0, 0.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(48.0),
            top: Val::Px(198.0),
            width: Val::Px(72.0),
            height: Val::Px(72.0),
            ..default()
        },
        UiTransform::IDENTITY,
        TreasureMapArrow,
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
        Transform::from_translation(start_position)
            .with_rotation(Quat::from_rotation_y(spawn_yaw - std::f32::consts::PI)),
        Player,
    ));

    let exit_cell = (MAZE_COLUMNS - 1, MAZE_ROWS - 1);
    let exit_center = cell_center(exit_cell.0, exit_cell.1);
    let exit_position = Vec3::new(exit_center.x, maze::WALL_HEIGHT * 0.47, exit_center.y);
    commands.spawn((
        Sprite::from_image(game_assets.smiley_exit.clone()),
        Sprite3d {
            pixels_per_metre: 4.0,
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            double_sided: true,
            ..Default::default()
        },
        Transform::from_translation(exit_position),
        Exit,
        Billboard,
    ));

    let rat_cell = RAT_SPAWN_CELL;
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
            squeak_timer: Timer::from_seconds(0.25, TimerMode::Once),
        },
        Billboard,
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

    let target = start_position + Vec3::Y * 28.0;
    let horizontal_distance = orbit.distance * orbit.pitch.cos();
    let camera_offset = Vec3::new(
        orbit.yaw.sin() * horizontal_distance,
        orbit.distance * orbit.pitch.sin() + 20.0,
        orbit.yaw.cos() * horizontal_distance,
    );
    commands.spawn((
        Transform::from_translation(start_position + Vec3::Y * 28.0)
            .with_rotation(Quat::from_rotation_y(listener_yaw(orbit.yaw))),
        SpatialListener::new(30.0),
        RatAudioListener,
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(target + camera_offset).looking_at(target, Vec3::Y),
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
            With<UpgradeMenuText>,
        )>,
    >,
    navigation_entities: Query<
        Entity,
        Or<(
            With<CompassPickup>,
            With<TreasureMapPickup>,
            With<CompassText>,
            With<TreasureMapText>,
            With<CompassArrow>,
            With<TreasureMapArrow>,
            With<FloatingMessage>,
            With<RatAudioListener>,
        )>,
    >,
    maze_entities: Query<Entity, Or<(With<Wall>, With<Floor>, With<Roof>, With<FogCell>)>>,
    mut fog_memory: ResMut<FogMemory>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for entity in run_entities
        .iter()
        .chain(navigation_entities.iter())
        .chain(maze_entities.iter())
    {
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

    for z in 0..MAZE_ROWS {
        for x in 0..MAZE_COLUMNS {
            if (x, z) == start_cell || (x, z) == exit_cell {
                continue;
            }
            let spawn_distance = start_cell.0.abs_diff(x) + start_cell.1.abs_diff(z);
            if spawn_distance <= 2 || treasure_cells.contains(&(x, z)) {
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

fn best_spawn_direction(cell: (usize, usize), maze: &MazeMap) -> Direction {
    let mut best_direction = Direction::North;
    let mut best_score = 0;
    let mut found_opening = false;

    for (direction, _, _) in cardinal_directions() {
        if maze.cells[cell.1][cell.0].walls[direction.index()] {
            continue;
        }

        let Some(next) = neighbor_cell(cell, direction) else {
            continue;
        };
        if maze.cells[next.1][next.0].walls[direction.opposite().index()] {
            continue;
        }

        let score = spawn_direction_score(cell, next, maze);
        if !found_opening || score > best_score {
            found_opening = true;
            best_direction = direction;
            best_score = score;
        }
    }

    best_direction
}

fn spawn_direction_score(
    origin: (usize, usize),
    first_cell: (usize, usize),
    maze: &MazeMap,
) -> u32 {
    let mut visited = [[false; MAZE_COLUMNS]; MAZE_ROWS];
    let mut queue = VecDeque::new();
    let mut score = 0;

    visited[origin.1][origin.0] = true;
    visited[first_cell.1][first_cell.0] = true;
    queue.push_back((first_cell, 1_u32));

    while let Some((cell, depth)) = queue.pop_front() {
        score += 7_u32.saturating_sub(depth);
        if depth >= 6 {
            continue;
        }

        for next in open_neighbors(cell, maze) {
            if visited[next.1][next.0] {
                continue;
            }
            visited[next.1][next.0] = true;
            queue.push_back((next, depth + 1));
        }
    }

    score
}

fn route_between(
    start: (usize, usize),
    goal: (usize, usize),
    maze: &MazeMap,
) -> Option<Vec<(usize, usize)>> {
    let mut visited = [[false; MAZE_COLUMNS]; MAZE_ROWS];
    let mut previous = [[None; MAZE_COLUMNS]; MAZE_ROWS];
    let mut queue = VecDeque::new();
    visited[start.1][start.0] = true;
    queue.push_back(start);

    while let Some(cell) = queue.pop_front() {
        if cell == goal {
            let mut route = vec![goal];
            let mut current = goal;
            while current != start {
                current = previous[current.1][current.0]?;
                route.push(current);
            }
            route.reverse();
            return Some(route);
        }

        for next in open_neighbors(cell, maze) {
            if visited[next.1][next.0] {
                continue;
            }
            visited[next.1][next.0] = true;
            previous[next.1][next.0] = Some(cell);
            queue.push_back(next);
        }
    }

    None
}

fn key_cell_on_exit_route(
    start_cell: (usize, usize),
    exit_cell: (usize, usize),
    maze: &MazeMap,
) -> (usize, usize) {
    if let Some(route) = route_between(start_cell, exit_cell, maze) {
        let furthest_index = route.len().saturating_sub(2);
        if furthest_index > 0 {
            let min_index = furthest_index.min(MIN_KEY_SPAWN_DISTANCE);
            let preferred_index = ((route.len() * 3) / 5).clamp(min_index, furthest_index);
            if let Some(cell) = route[preferred_index..=furthest_index]
                .iter()
                .copied()
                .find(|cell| {
                    *cell != RAT_SPAWN_CELL
                        && manhattan_distance(start_cell, *cell) >= MIN_KEY_SPAWN_DISTANCE
                })
            {
                return cell;
            }
            if let Some(cell) = route[min_index..=furthest_index]
                .iter()
                .copied()
                .rev()
                .find(|cell| {
                    *cell != RAT_SPAWN_CELL
                        && manhattan_distance(start_cell, *cell) >= MIN_KEY_SPAWN_DISTANCE
                })
            {
                return cell;
            }
            return route[furthest_index];
        }
    }

    far_spawn_candidate(start_cell, exit_cell, maze, MIN_KEY_SPAWN_DISTANCE).unwrap_or(start_cell)
}

fn manhattan_distance(a: (usize, usize), b: (usize, usize)) -> usize {
    a.0.abs_diff(b.0) + a.1.abs_diff(b.1)
}

fn far_spawn_candidate(
    start_cell: (usize, usize),
    exit_cell: (usize, usize),
    maze: &MazeMap,
    min_distance: usize,
) -> Option<(usize, usize)> {
    let mut best = None;
    let mut best_distance = 0;
    for z in 0..MAZE_ROWS {
        for x in 0..MAZE_COLUMNS {
            let cell = (x, z);
            if cell == start_cell || cell == exit_cell || cell == RAT_SPAWN_CELL {
                continue;
            }
            let distance = manhattan_distance(start_cell, cell);
            if distance < min_distance || distance <= best_distance {
                continue;
            }
            if route_between(start_cell, cell, maze).is_some() {
                best = Some(cell);
                best_distance = distance;
            }
        }
    }
    best
}

fn spawn_key_door_and_chests(
    commands: &mut Commands,
    maze: &MazeMap,
    start_position: Vec3,
    game_assets: &GameAssets,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> (u32, (usize, usize)) {
    let start_cell = world_to_cell(start_position).unwrap_or((0, 0));
    let exit_cell = (MAZE_COLUMNS - 1, MAZE_ROWS - 1);
    let key_cell = key_cell_on_exit_route(start_cell, exit_cell, maze);
    let key_center = cell_center(key_cell.0, key_cell.1);
    commands.spawn((
        Sprite::from_image(game_assets.key.clone()),
        item_sprite(2.6),
        Transform::from_xyz(key_center.x, 2.0, key_center.y),
        KeyPickup,
        Billboard,
    ));
    info!("Key spawned at cell {:?}", key_cell);

    if let Some(door_neighbor) = open_neighbors(exit_cell, maze).first().copied() {
        if let Some(door_direction) = direction_between(door_neighbor, exit_cell) {
            let door_center = edge_center(door_neighbor, door_direction);
            let vertical = matches!(door_direction, Direction::East | Direction::West);
            let door_mesh = if vertical {
                meshes.add(Cuboid::new(
                    maze::WALL_THICKNESS * 0.25,
                    maze::WALL_HEIGHT,
                    maze::CELL_SIZE - maze::WALL_THICKNESS * 2.25,
                ))
            } else {
                meshes.add(Cuboid::new(
                    maze::CELL_SIZE - maze::WALL_THICKNESS * 2.25,
                    maze::WALL_HEIGHT,
                    maze::WALL_THICKNESS * 0.25,
                ))
            };
            let half_extents = if vertical {
                Vec2::new(
                    maze::WALL_THICKNESS * 0.25,
                    (maze::CELL_SIZE - maze::WALL_THICKNESS * 2.25) * 0.5,
                )
            } else {
                Vec2::new(
                    (maze::CELL_SIZE - maze::WALL_THICKNESS * 2.25) * 0.5,
                    maze::WALL_THICKNESS * 0.25,
                )
            };
            let door_material = materials.add(StandardMaterial {
                base_color_texture: Some(game_assets.locked_door.clone()),
                base_color: Color::WHITE,
                perceptual_roughness: 1.0,
                unlit: true,
                ..default()
            });
            commands.spawn((
                Mesh3d(door_mesh),
                MeshMaterial3d(door_material),
                Transform::from_xyz(door_center.x, maze::WALL_HEIGHT * 0.5, door_center.y),
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
                || (x, z) == RAT_SPAWN_CELL
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
    (spawned_chests * 2, key_cell)
}

fn spawn_navigation_pickups(
    commands: &mut Commands,
    maze: &MazeMap,
    start_position: Vec3,
    key_cell: (usize, usize),
    game_assets: &GameAssets,
) -> ((usize, usize), (usize, usize)) {
    let start_cell = world_to_cell(start_position).unwrap_or((0, 0));
    let exit_cell = (MAZE_COLUMNS - 1, MAZE_ROWS - 1);
    let mut candidates = Vec::new();

    for z in 0..MAZE_ROWS {
        for x in 0..MAZE_COLUMNS {
            let cell = (x, z);
            if cell == start_cell || cell == exit_cell || cell == key_cell || cell == RAT_SPAWN_CELL
            {
                continue;
            }
            let spawn_distance = manhattan_distance(start_cell, cell);
            if spawn_distance < MIN_NAV_PICKUP_SPAWN_DISTANCE {
                continue;
            }

            let open_edges = maze.cells[z][x].walls.iter().filter(|wall| !**wall).count();
            if open_edges >= 2 {
                candidates.push(cell);
            }
        }
    }

    let mut rng = rand::thread_rng();
    candidates.shuffle(&mut rng);
    let fallback = far_spawn_candidate(start_cell, exit_cell, maze, MIN_NAV_PICKUP_SPAWN_DISTANCE)
        .unwrap_or(key_cell);
    let compass_cell = candidates.first().copied().unwrap_or(fallback);
    let treasure_map_cell = candidates
        .iter()
        .copied()
        .find(|cell| *cell != compass_cell)
        .unwrap_or(fallback);

    let compass_center = cell_center(compass_cell.0, compass_cell.1);
    commands.spawn((
        Sprite::from_image(game_assets.compass.clone()),
        item_sprite(5.6),
        Transform::from_xyz(compass_center.x, 2.0, compass_center.y),
        CompassPickup,
        Billboard,
    ));

    let map_center = cell_center(treasure_map_cell.0, treasure_map_cell.1);
    commands.spawn((
        Sprite::from_image(game_assets.treasure_map.clone()),
        item_sprite(2.8),
        Transform::from_xyz(map_center.x, 2.0, map_center.y),
        TreasureMapPickup,
        Billboard,
    ));

    (compass_cell, treasure_map_cell)
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
            run_state.markers += TRAIL_MARKERS_PER_TREASURE;
            commands.entity(entity).despawn();
            play_sound(&mut commands, game_assets.pickup_sound.clone());
            info!(
                "Collected treasure {}/{}; markers {}",
                run_state.treasure, run_state.total_treasure, run_state.markers
            );
        }
    }
}

fn collect_ore(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    ore_query: Query<(Entity, &Transform, &Ore), Without<OreBurst>>,
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

fn collect_navigation_pickups(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    compass_query: Query<(Entity, &Transform), With<CompassPickup>>,
    map_query: Query<(Entity, &Transform), With<TreasureMapPickup>>,
    game_assets: Res<GameAssets>,
    mut navigation_state: ResMut<NavigationState>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_xz = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.z,
    );

    for (entity, transform) in compass_query.iter() {
        let pickup_xz = Vec2::new(transform.translation.x, transform.translation.z);
        if player_xz.distance(pickup_xz) < 30.0 {
            navigation_state.compass_collected = true;
            commands.entity(entity).despawn();
            play_sound(&mut commands, game_assets.pickup_sound.clone());
            info!("Collected exit compass");
        }
    }

    for (entity, transform) in map_query.iter() {
        let pickup_xz = Vec2::new(transform.translation.x, transform.translation.z);
        if player_xz.distance(pickup_xz) < 30.0 {
            navigation_state.treasure_map_collected = true;
            navigation_state.show_treasure_map = true;
            commands.entity(entity).despawn();
            play_sound(&mut commands, game_assets.pickup_sound.clone());
            info!("Collected treasure map");
        }
    }
}

fn collect_key(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    key_query: Query<(Entity, &Transform), With<KeyPickup>>,
    game_assets: Res<GameAssets>,
    mut run_state: ResMut<RunState>,
    mut navigation_state: ResMut<NavigationState>,
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
            navigation_state.key_collected = true;
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
    mut navigation_state: ResMut<NavigationState>,
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
            navigation_state.door_unlocked = true;
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

fn update_hud(
    run_state: Res<RunState>,
    navigation_state: Res<NavigationState>,
    mut hud_query: Query<&mut Text, With<HudText>>,
    mut upgrade_menu_query: Query<&mut Text, (With<UpgradeMenuText>, Without<HudText>)>,
) {
    if !run_state.is_changed() && !navigation_state.is_changed() {
        return;
    }

    if let Ok(mut text) = hud_query.single_mut() {
        text.0 = hud_text(&run_state, &navigation_state);
    }
    if let Ok(mut text) = upgrade_menu_query.single_mut() {
        text.0 = upgrade_menu_text(&run_state);
    }
}

fn hud_text(run_state: &RunState, navigation_state: &NavigationState) -> String {
    let objective = if run_state.won {
        "Escaped!"
    } else if run_state.caught_by_rat {
        "Caught by the rat!"
    } else {
        "Find the exit"
    };
    let map_hint = if navigation_state.treasure_map_collected {
        "  M: map"
    } else {
        ""
    };
    let compass_hint = if navigation_state.compass_collected {
        ""
    } else {
        "  Find compass"
    };
    format!(
        "Treasure: {}/{}  Ore: {}  Markers: {}  Keys: {}  Energy: {}/{}\nMining: {}  Speed: {}  Mined: {}  Time: {:.0}s\nScore: {}  Best: {}\nQ/E: turn  T: marker  F: mine  U: upgrades{}{}{}\n{}",
        run_state.treasure,
        run_state.total_treasure,
        run_state.ore,
        run_state.markers,
        run_state.keys,
        run_state.energy,
        run_state.max_energy,
        run_state.pickaxe_level,
        run_state.speed_level,
        run_state.mined_walls,
        run_state.elapsed_seconds,
        run_state.score,
        run_state.best_score,
        map_hint,
        compass_hint,
        if run_state.won || run_state.caught_by_rat {
            "  R: restart"
        } else {
            ""
        },
        objective
    )
}

fn toggle_treasure_map(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut navigation_state: ResMut<NavigationState>,
    run_state: Res<RunState>,
) {
    if run_state.won
        || run_state.caught_by_rat
        || !navigation_state.treasure_map_collected
        || !keyboard.just_pressed(KeyCode::KeyM)
    {
        return;
    }

    navigation_state.show_treasure_map = !navigation_state.show_treasure_map;
}

fn update_navigation_ui(
    player_query: Query<&Transform, With<Player>>,
    orbit: Res<CameraOrbit>,
    navigation_state: Res<NavigationState>,
    mut compass_query: Query<(&mut Text, &mut TextColor), With<CompassText>>,
    mut map_query: Query<
        (&mut Text, &mut TextColor),
        (With<TreasureMapText>, Without<CompassText>),
    >,
    mut compass_arrow_query: Query<
        (&mut ImageNode, &mut UiTransform),
        (With<CompassArrow>, Without<TreasureMapArrow>),
    >,
    mut map_arrow_query: Query<
        (&mut ImageNode, &mut UiTransform),
        (With<TreasureMapArrow>, Without<CompassArrow>),
    >,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let exit_cell = (MAZE_COLUMNS - 1, MAZE_ROWS - 1);
    let exit_center = cell_center(exit_cell.0, exit_cell.1);
    let exit_angle = relative_direction_angle(player_transform.translation, exit_center, orbit.yaw);

    if let Ok((mut text, mut color)) = compass_query.single_mut() {
        if navigation_state.compass_collected {
            text.0 = "EXIT COMPASS".to_string();
            color.0 = if navigation_state.key_collected {
                Color::srgb(0.25, 1.0, 0.25)
            } else {
                Color::srgb(1.0, 0.15, 0.1)
            };
            if let Ok((mut arrow, mut transform)) = compass_arrow_query.single_mut() {
                set_ui_arrow(&mut arrow, &mut transform, exit_angle, color.0);
            }
        } else {
            text.0.clear();
            if let Ok((mut arrow, _)) = compass_arrow_query.single_mut() {
                arrow.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
            }
        }
    }

    let Ok((mut text, mut color)) = map_query.single_mut() else {
        return;
    };
    color.0 = Color::srgb(1.0, 0.84, 0.18);
    if !navigation_state.treasure_map_collected || !navigation_state.show_treasure_map {
        text.0.clear();
        if let Ok((mut arrow, _)) = map_arrow_query.single_mut() {
            arrow.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
        }
        return;
    }

    let map_arrow_angle = if !navigation_state.key_collected {
        if let Some(key_cell) = navigation_state.key_cell {
            let key_center = cell_center(key_cell.0, key_cell.1);
            text.0 = "TREASURE MAP\nKEY".to_string();
            Some(relative_direction_angle(
                player_transform.translation,
                key_center,
                orbit.yaw,
            ))
        } else {
            text.0 = "TREASURE MAP\nKEY LOST".to_string();
            None
        }
    } else if !navigation_state.compass_collected {
        if let Some(compass_cell) = navigation_state.compass_cell {
            let compass_center = cell_center(compass_cell.0, compass_cell.1);
            text.0 = "TREASURE MAP\nCOMPASS".to_string();
            Some(relative_direction_angle(
                player_transform.translation,
                compass_center,
                orbit.yaw,
            ))
        } else {
            text.0 = "TREASURE MAP\nCOMPASS LOST".to_string();
            None
        }
    } else {
        text.0 = "TREASURE FOUND".to_string();
        None
    };

    if let Ok((mut arrow, mut transform)) = map_arrow_query.single_mut() {
        if let Some(angle) = map_arrow_angle {
            set_ui_arrow(
                &mut arrow,
                &mut transform,
                angle,
                Color::srgb(1.0, 0.84, 0.18),
            )
        } else {
            arrow.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
        }
    }
}

fn set_ui_arrow(arrow: &mut ImageNode, transform: &mut UiTransform, angle: f32, color: Color) {
    arrow.color = color;
    transform.rotation = Rot2::radians(ui_arrow_rotation(angle));
}

fn ui_arrow_rotation(direction_angle: f32) -> f32 {
    direction_angle - std::f32::consts::FRAC_PI_2
}

fn relative_direction_angle(from: Vec3, target: Vec2, camera_yaw: f32) -> f32 {
    let delta = target - Vec2::new(from.x, from.z);
    if delta.length_squared() < 4.0 {
        return 0.0;
    }

    let direction = delta.normalize();
    direction_angle_relative_to_camera(direction, camera_yaw)
}

fn direction_angle_relative_to_camera(direction: Vec2, camera_yaw: f32) -> f32 {
    let forward = camera_forward_xz(camera_yaw);
    let right = camera_right_xz(camera_yaw);
    direction.dot(right).atan2(direction.dot(forward))
}

fn camera_forward_xz(camera_yaw: f32) -> Vec2 {
    Vec2::new(-camera_yaw.sin(), -camera_yaw.cos()).normalize()
}

fn camera_right_xz(camera_yaw: f32) -> Vec2 {
    Vec2::new(-camera_yaw.cos(), camera_yaw.sin()).normalize()
}

fn upgrade_menu_text(run_state: &RunState) -> String {
    if !run_state.upgrade_menu_open {
        return String::new();
    }

    let mining_prefix = if run_state.upgrade_selection == 0 {
        ">"
    } else {
        " "
    };
    let speed_prefix = if run_state.upgrade_selection == 1 {
        ">"
    } else {
        " "
    };
    format!(
        "UPGRADES\nOre: {}\n\n{} Mining energy\n  Cost: {} ore\n  Level: {} -> {}\n\n{} Movement speed\n  Cost: {} ore\n  Level: {} -> {}\n\nUp/Down select\nEnter buy\nU/Esc close",
        run_state.ore,
        mining_prefix,
        upgrade_cost(run_state.pickaxe_level),
        run_state.pickaxe_level,
        run_state.pickaxe_level + 1,
        speed_prefix,
        upgrade_cost(run_state.speed_level),
        run_state.speed_level,
        run_state.speed_level + 1,
    )
}

fn upgrade_cost(level: u32) -> u32 {
    level.saturating_mul(2)
}

fn upgrade_pickaxe(
    keyboard: Res<ButtonInput<KeyCode>>,
    game_assets: Res<GameAssets>,
    mut run_state: ResMut<RunState>,
    mut commands: Commands,
) {
    if run_state.won || run_state.caught_by_rat {
        return;
    }

    if keyboard.just_pressed(KeyCode::KeyU) {
        run_state.upgrade_menu_open = !run_state.upgrade_menu_open;
        return;
    }
    if !run_state.upgrade_menu_open {
        return;
    }
    if keyboard.just_pressed(KeyCode::Escape) {
        run_state.upgrade_menu_open = false;
        return;
    }
    if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::ArrowDown) {
        run_state.upgrade_selection = 1 - run_state.upgrade_selection;
        return;
    }
    if !keyboard.just_pressed(KeyCode::Enter) && !keyboard.just_pressed(KeyCode::Space) {
        return;
    }

    if run_state.upgrade_selection == 0 {
        let cost = upgrade_cost(run_state.pickaxe_level);
        if run_state.ore < cost {
            info!("Need {cost} ore to upgrade mining energy");
            return;
        }

        run_state.ore -= cost;
        run_state.pickaxe_level += 1;
        run_state.max_energy += 1;
        run_state.energy = run_state.max_energy;
        play_sound(&mut commands, game_assets.upgrade_sound.clone());
        info!(
            "Upgraded mining to level {}; max energy {}",
            run_state.pickaxe_level, run_state.max_energy
        );
    } else {
        let cost = upgrade_cost(run_state.speed_level);
        if run_state.ore < cost {
            info!("Need {cost} ore to upgrade speed");
            return;
        }

        run_state.ore -= cost;
        run_state.speed_level += 1;
        play_sound(&mut commands, game_assets.upgrade_sound.clone());
        info!("Upgraded speed to level {}", run_state.speed_level);
    }
}

fn mine_wall(
    keyboard: Res<ButtonInput<KeyCode>>,
    player_query: Query<&Transform, With<Player>>,
    orbit: Res<CameraOrbit>,
    mut maze: ResMut<MazeMap>,
    mut wall_query: Query<(
        Entity,
        &mut WallSegment,
        &WallCollider,
        &mut MeshMaterial3d<StandardMaterial>,
        &Transform,
    )>,
    hard_wall_assets: Res<HardWallAssets>,
    game_assets: Res<GameAssets>,
    mut run_state: ResMut<RunState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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

    let Some((entity, mut segment, collider, mut material, transform)) =
        wall_query.iter_mut().find(|(_, segment, _, _, _)| {
            same_wall_edge(segment.cell, segment.direction, cell, direction)
        })
    else {
        info!(
            "No wall entity found at cell {:?} facing {:?}",
            cell,
            direction.index()
        );
        spawn_unbreakable_message(&mut commands);
        return;
    };

    if !segment.mineable {
        info!("Boundary wall cannot be mined at cell {:?}", cell);
        spawn_unbreakable_message(&mut commands);
        return;
    }

    if run_state.energy == 0 {
        info!("Out of mining energy; upgrade your pickaxe with ore");
        return;
    }

    run_state.energy -= 1;
    play_sound(&mut commands, game_assets.mine_sound.clone());

    if segment.hits_remaining > 1 {
        segment.hits_remaining -= 1;
        let crack_index = (segment.max_hits - segment.hits_remaining) as usize;
        let texture = hard_wall_assets.textures[crack_index.min(2)].clone();
        *material = MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(texture),
            base_color: Color::WHITE,
            perceptual_roughness: 0.9,
            ..default()
        }));
        commands.entity(entity).insert(WallShake {
            timer: Timer::from_seconds(0.22, TimerMode::Once),
            original: transform.translation,
        });
        info!(
            "Cracked hard wall at cell {:?}; {} hits remaining",
            cell, segment.hits_remaining
        );
        return;
    }

    let ore_deposit = segment.ore_deposit;
    let ore_spawn_center = collider.center;
    commands.entity(entity).despawn();
    maze.cells[cell.1][cell.0].walls[direction.index()] = false;
    if let Some(neighbor) = neighbor_cell(cell, direction) {
        maze.cells[neighbor.1][neighbor.0].walls[direction.opposite().index()] = false;
    }
    if ore_deposit {
        spawn_ore_burst(
            &mut commands,
            game_assets.ore.clone(),
            ore_spawn_center,
            direction,
        );
    }
    run_state.mined_walls += 1;
    info!(
        "Mined wall at cell {:?} facing {:?}",
        cell,
        direction.index()
    );
}

fn spawn_unbreakable_message(commands: &mut Commands) {
    commands.spawn((
        Text::new("Unbreakable!"),
        TextFont {
            font_size: 34.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 0.9, 0.1, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(44.0),
            ..default()
        },
        UiTransform::from_translation(Val2::px(-105.0, 0.0)),
        FloatingMessage {
            timer: Timer::from_seconds(0.9, TimerMode::Once),
        },
    ));
}

fn update_floating_messages(
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &mut FloatingMessage,
        &mut TextColor,
        &mut UiTransform,
    )>,
    mut commands: Commands,
) {
    for (entity, mut message, mut color, mut transform) in query.iter_mut() {
        message.timer.tick(time.delta());
        let fraction = message.timer.fraction();
        transform.translation = Val2::px(-105.0, -58.0 * fraction);
        color.0 = Color::srgba(1.0, 0.9, 0.1, 1.0 - fraction);
        if message.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn spawn_ore_burst(
    commands: &mut Commands,
    image: Handle<Image>,
    center: Vec2,
    wall_direction: Direction,
) {
    let mut rng = rand::thread_rng();
    let count = rng.gen_range(2..=4);
    let (dx, dz) = wall_direction.delta();
    let outward = Vec3::new(dx as f32, 0.0, dz as f32).normalize_or_zero();

    for _ in 0..count {
        let sideways = Vec3::new(outward.z, 0.0, -outward.x) * rng.gen_range(-35.0..=35.0);
        let velocity = outward * rng.gen_range(70.0..=120.0)
            + sideways
            + Vec3::Y * rng.gen_range(95.0..=145.0);
        commands.spawn((
            Sprite::from_image(image.clone()),
            item_sprite(2.4),
            Transform::from_xyz(center.x, 18.0, center.y),
            Ore { value: 1 },
            OreBurst {
                velocity,
                timer: Timer::from_seconds(0.95, TimerMode::Once),
            },
            Billboard,
        ));
    }
    info!("Ore deposit burst {count} ore nuggets");
}

fn update_ore_bursts(
    time: Res<Time>,
    maze: Res<MazeMap>,
    mut query: Query<(Entity, &mut Transform, &mut OreBurst)>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    for (entity, mut transform, mut burst) in query.iter_mut() {
        burst.timer.tick(time.delta());
        burst.velocity.y -= 360.0 * dt;
        let x_delta = burst.velocity.x * dt;
        let z_delta = burst.velocity.z * dt;
        move_ore_burst_axis(&mut transform, &mut burst, &maze, x_delta, true);
        move_ore_burst_axis(&mut transform, &mut burst, &maze, z_delta, false);
        transform.translation.y += burst.velocity.y * dt;

        if transform.translation.y <= 2.0 {
            transform.translation.y = 2.0;
            if burst.velocity.y < -18.0 && !burst.timer.is_finished() {
                burst.velocity.y = -burst.velocity.y * 0.34;
                burst.velocity.x *= 0.72;
                burst.velocity.z *= 0.72;
            } else {
                commands.entity(entity).remove::<OreBurst>();
            }
        }

        if burst.timer.is_finished() {
            transform.translation.y = 2.0;
            commands.entity(entity).remove::<OreBurst>();
        }
    }
}

fn move_ore_burst_axis(
    transform: &mut Transform,
    burst: &mut OreBurst,
    maze: &MazeMap,
    delta: f32,
    x_axis: bool,
) {
    if delta.abs() <= f32::EPSILON {
        return;
    }

    let old_position = transform.translation;
    let mut candidate = old_position;
    if x_axis {
        candidate.x += delta;
    } else {
        candidate.z += delta;
    }

    if ore_move_blocked(old_position, candidate, maze) {
        let Some(cell) = world_to_cell(old_position) else {
            if x_axis {
                burst.velocity.x *= -0.55;
            } else {
                burst.velocity.z *= -0.55;
            }
            return;
        };
        let center = cell_center(cell.0, cell.1);
        let half_cell = maze::CELL_SIZE * 0.5 - ORE_WALL_BOUNCE_MARGIN;
        if x_axis {
            burst.velocity.x *= -0.55;
            transform.translation.x = if delta > 0.0 {
                center.x + half_cell
            } else {
                center.x - half_cell
            };
        } else {
            burst.velocity.z *= -0.55;
            transform.translation.z = if delta > 0.0 {
                center.y + half_cell
            } else {
                center.y - half_cell
            };
        }
        burst.velocity.y *= 0.9;
    } else if x_axis {
        transform.translation.x = candidate.x;
    } else {
        transform.translation.z = candidate.z;
    }
}

fn ore_move_blocked(old_position: Vec3, candidate: Vec3, maze: &MazeMap) -> bool {
    let Some(old_cell) = world_to_cell(old_position) else {
        return true;
    };
    let Some(new_cell) = world_to_cell(candidate) else {
        return true;
    };
    if old_cell == new_cell {
        return false;
    }

    let Some(direction) = direction_between(old_cell, new_cell) else {
        return true;
    };
    maze.cells[old_cell.1][old_cell.0].walls[direction.index()]
        || maze.cells[new_cell.1][new_cell.0].walls[direction.opposite().index()]
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

fn yaw_for_direction(direction: Direction) -> f32 {
    match direction {
        Direction::North => std::f32::consts::PI,
        Direction::East => std::f32::consts::FRAC_PI_2 * 3.0,
        Direction::South => 0.0,
        Direction::West => std::f32::consts::FRAC_PI_2,
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

fn update_wall_shake(
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut WallShake)>,
    mut commands: Commands,
) {
    for (entity, mut transform, mut shake) in query.iter_mut() {
        shake.timer.tick(time.delta());
        let remaining = 1.0 - shake.timer.fraction();
        let wobble = (shake.timer.elapsed_secs() * 95.0).sin() * 3.0 * remaining;
        transform.translation = shake.original + Vec3::new(wobble, 0.0, 0.0);
        if shake.timer.is_finished() {
            transform.translation = shake.original;
            commands.entity(entity).remove::<WallShake>();
        }
    }
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
    mut run_state: ResMut<RunState>,
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
    if run_state.markers == 0 {
        info!("No trail markers left; collect treasure for more");
        return;
    }

    run_state.markers -= 1;
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
        info!("You found the exit!");
    }
}

fn update_audio_listener(
    player_query: Query<&Transform, (With<Player>, Without<RatAudioListener>)>,
    mut listener_query: Query<&mut Transform, (With<RatAudioListener>, Without<Player>)>,
    orbit: Res<CameraOrbit>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let Ok(mut listener_transform) = listener_query.single_mut() else {
        return;
    };

    listener_transform.translation = player_transform.translation + Vec3::Y * 28.0;
    listener_transform.rotation = Quat::from_rotation_y(listener_yaw(orbit.yaw));
}

fn listener_yaw(camera_yaw: f32) -> f32 {
    camera_yaw + std::f32::consts::PI
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

        let same_player_cell = player_cell.is_some_and(|cell| cell == rat.cell);
        if !same_player_cell && rat.cell == rat.target_cell {
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

        let target = if same_player_cell {
            Vec3::new(player_xz.x, transform.translation.y, player_xz.y)
        } else {
            let target_center = cell_center(rat.target_cell.0, rat.target_cell.1);
            Vec3::new(target_center.x, transform.translation.y, target_center.y)
        };
        let to_target = target - transform.translation;
        let distance = to_target.length();

        if distance <= 2.0 {
            if !same_player_cell && rat.cell != rat.target_cell {
                rat.last_direction = direction_between(rat.cell, rat.target_cell);
                rat.cell = rat.target_cell;
            }
            if !same_player_cell {
                rat.target_cell = rat.cell;
            }
            continue;
        }

        let step = 92.0 * time.delta_secs();
        let movement = to_target.normalize() * step.min(distance);
        transform.translation += movement;
        transform.rotation = Quat::from_rotation_y(movement.x.atan2(movement.z));
    }
}

fn rat_squeaks(
    time: Res<Time>,
    maze: Res<MazeMap>,
    player_query: Query<&Transform, (With<Player>, Without<Rat>)>,
    mut rat_query: Query<(&Transform, &mut Rat)>,
    game_assets: Res<GameAssets>,
    run_state: Res<RunState>,
    mut commands: Commands,
) {
    if run_state.won || run_state.caught_by_rat {
        return;
    }

    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let Some(player_cell) = world_to_cell(player_transform.translation) else {
        return;
    };

    let mut rng = rand::thread_rng();
    for (rat_transform, mut rat) in rat_query.iter_mut() {
        rat.squeak_timer.tick(time.delta());
        if !rat.squeak_timer.is_finished() {
            continue;
        }

        let route = route_between(player_cell, rat.cell, &maze);
        let route_steps = route
            .as_ref()
            .map(|route| route.len().saturating_sub(1) as f32)
            .unwrap_or(18.0);
        let distance_t = (route_steps / 12.0).clamp(0.0, 1.0);
        let next_interval = 0.12 + distance_t.powf(1.1) * 3.9 + rng.gen_range(-0.04..=0.28);
        rat.squeak_timer = Timer::from_seconds(next_interval.max(0.1), TimerMode::Once);

        let clear_line = has_clear_sound_line(player_cell, rat.cell, &maze);
        let use_echo = !clear_line && route_steps >= 2.0;
        let sound = if use_echo {
            game_assets.rat_squeak_echo_sound.clone()
        } else {
            game_assets.rat_squeak_sound.clone()
        };
        let volume = rat_squeak_volume(use_echo, distance_t);
        let speed = if use_echo {
            rng.gen_range(0.82..=0.96)
        } else {
            rng.gen_range(0.96..=1.14)
        };
        let spatial_scale = if use_echo { 0.008 } else { 0.018 };
        let sound_position = if use_echo {
            echo_source_position(player_cell, route.as_deref(), rat_transform.translation)
        } else {
            rat_transform.translation + Vec3::Y * 18.0
        };
        commands.spawn((
            AudioPlayer(sound),
            PlaybackSettings::DESPAWN
                .with_volume(Volume::Linear(volume))
                .with_speed(speed)
                .with_spatial(true)
                .with_spatial_scale(SpatialScale::new(spatial_scale)),
            Transform::from_translation(sound_position),
        ));
    }
}

fn rat_squeak_volume(use_echo: bool, distance_t: f32) -> f32 {
    if use_echo {
        (0.20 - distance_t.clamp(0.0, 1.0) * 0.10).clamp(0.05, 0.22)
    } else {
        (1.25 - distance_t.clamp(0.0, 1.0) * 0.12).clamp(0.85, 1.30)
    }
}

fn has_clear_sound_line(start: (usize, usize), goal: (usize, usize), maze: &MazeMap) -> bool {
    if start == goal {
        return true;
    }

    if start.0 == goal.0 {
        let direction = if goal.1 > start.1 {
            Direction::North
        } else {
            Direction::South
        };
        return has_clear_axis_path(start, goal, direction, maze);
    }

    if start.1 == goal.1 {
        let direction = if goal.0 > start.0 {
            Direction::East
        } else {
            Direction::West
        };
        return has_clear_axis_path(start, goal, direction, maze);
    }

    false
}

fn has_clear_axis_path(
    mut current: (usize, usize),
    goal: (usize, usize),
    direction: Direction,
    maze: &MazeMap,
) -> bool {
    while current != goal {
        if maze.cells[current.1][current.0].walls[direction.index()] {
            return false;
        }
        let Some(next) = neighbor_cell(current, direction) else {
            return false;
        };
        if maze.cells[next.1][next.0].walls[direction.opposite().index()] {
            return false;
        }
        current = next;
    }
    true
}

fn echo_source_position(
    player_cell: (usize, usize),
    route: Option<&[(usize, usize)]>,
    fallback: Vec3,
) -> Vec3 {
    let Some(route) = route else {
        return fallback + Vec3::Y * 18.0;
    };
    let Some(next_cell) = route.get(1).copied() else {
        return fallback + Vec3::Y * 18.0;
    };
    let Some(direction) = direction_between(player_cell, next_cell) else {
        return fallback + Vec3::Y * 18.0;
    };
    let doorway = edge_center(player_cell, direction);
    Vec3::new(doorway.x, 22.0, doorway.y)
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
    let keyboard_rotation = 3.8 * time.delta_secs();
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

    let sensitivity = 0.01;
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

    let speed = 105.0 + (run_state.speed_level.saturating_sub(1) as f32 * 20.0);
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
    reveal_visible_area(origin, &maze, &mut visible, &mut alpha, &mut fog_memory);

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

fn reveal_visible_area(
    origin: (usize, usize),
    maze: &MazeMap,
    visible: &mut [[bool; MAZE_COLUMNS]; MAZE_ROWS],
    alpha: &mut [[f32; MAZE_COLUMNS]; MAZE_ROWS],
    fog_memory: &mut FogMemory,
) {
    let mut visited = [[false; MAZE_COLUMNS]; MAZE_ROWS];
    let mut queue = VecDeque::new();
    visited[origin.1][origin.0] = true;
    queue.push_back((origin, 0usize));

    while let Some((cell, distance)) = queue.pop_front() {
        reveal_cell(cell, visible, alpha, fog_memory);
        if distance >= 3 {
            continue;
        }

        for next in open_neighbors(cell, maze) {
            if visited[next.1][next.0] {
                continue;
            }
            visited[next.1][next.0] = true;
            queue.push_back((next, distance + 1));
        }
    }
}

fn cardinal_directions() -> [(Direction, isize, isize); 4] {
    [
        (Direction::North, 0, 1),
        (Direction::East, 1, 0),
        (Direction::South, 0, -1),
        (Direction::West, -1, 0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_maze() -> MazeMap {
        MazeMap {
            cells: [[maze::MazeCell { walls: [true; 4] }; MAZE_COLUMNS]; MAZE_ROWS],
        }
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn arrow_angles_are_continuous_relative_to_camera() {
        let yaw = std::f32::consts::PI;

        assert_close(
            direction_angle_relative_to_camera(Vec2::new(0.0, 1.0), yaw),
            0.0,
        );
        assert_close(
            direction_angle_relative_to_camera(Vec2::new(1.0, 0.0), yaw),
            std::f32::consts::FRAC_PI_2,
        );
        assert_close(
            direction_angle_relative_to_camera(Vec2::new(-1.0, 0.0), yaw),
            -std::f32::consts::FRAC_PI_2,
        );

        let diagonal = direction_angle_relative_to_camera(Vec2::new(1.0, 1.0).normalize(), yaw);
        assert_close(diagonal, std::f32::consts::FRAC_PI_4);
    }

    #[test]
    fn ui_arrow_rotation_accounts_for_right_facing_texture() {
        assert_close(ui_arrow_rotation(0.0), -std::f32::consts::FRAC_PI_2);
        assert_close(ui_arrow_rotation(std::f32::consts::FRAC_PI_2), 0.0);
        assert_close(
            ui_arrow_rotation(-std::f32::consts::FRAC_PI_2),
            -std::f32::consts::PI,
        );
    }

    #[test]
    fn listener_right_ear_matches_camera_right() {
        for yaw in [
            0.0,
            std::f32::consts::FRAC_PI_2,
            std::f32::consts::PI,
            std::f32::consts::FRAC_PI_2 * 3.0,
        ] {
            let listener_right = Quat::from_rotation_y(listener_yaw(yaw)) * Vec3::X;
            let expected = camera_right_xz(yaw);
            assert_close(listener_right.x, expected.x);
            assert_close(listener_right.z, expected.y);
        }
    }

    #[test]
    fn audio_side_math_tracks_camera_turns() {
        let source_east = Vec2::new(1.0, 0.0);

        assert!(direction_angle_relative_to_camera(source_east, std::f32::consts::PI) > 0.0);
        assert!(direction_angle_relative_to_camera(source_east, 0.0) < 0.0);
    }

    #[test]
    fn echo_squeaks_are_much_quieter_than_direct_squeaks() {
        let near_corner_echo = rat_squeak_volume(true, 2.0 / 12.0);
        let near_direct = rat_squeak_volume(false, 2.0 / 12.0);
        let far_echo = rat_squeak_volume(true, 1.0);

        assert!(near_corner_echo < 0.20);
        assert!(far_echo <= 0.10);
        assert!(near_direct > 1.20);
        assert!(near_direct > near_corner_echo * 6.0);
    }

    #[test]
    fn ore_burst_crossing_respects_maze_walls() {
        let mut maze = test_maze();
        maze.cells[0][0].walls[Direction::East.index()] = false;
        maze.cells[0][1].walls[Direction::West.index()] = false;

        let old_position = Vec3::new(cell_center(0, 0).x, 2.0, cell_center(0, 0).y);
        let open_candidate = Vec3::new(cell_center(1, 0).x, 2.0, cell_center(1, 0).y);
        assert!(!ore_move_blocked(old_position, open_candidate, &maze));

        maze.cells[0][0].walls[Direction::East.index()] = true;
        assert!(ore_move_blocked(old_position, open_candidate, &maze));

        let out_of_bounds = old_position - Vec3::X * maze::CELL_SIZE;
        assert!(ore_move_blocked(old_position, out_of_bounds, &maze));
    }

    #[test]
    fn ore_burst_bounces_back_inside_cell_when_it_hits_wall() {
        let maze = test_maze();
        let center = cell_center(0, 0);
        let mut transform = Transform::from_xyz(center.x, 2.0, center.y);
        let mut burst = OreBurst {
            velocity: Vec3::new(120.0, 20.0, 0.0),
            timer: Timer::from_seconds(0.95, TimerMode::Once),
        };

        move_ore_burst_axis(&mut transform, &mut burst, &maze, maze::CELL_SIZE, true);

        assert_close(
            transform.translation.x,
            center.x + maze::CELL_SIZE * 0.5 - ORE_WALL_BOUNCE_MARGIN,
        );
        assert!(burst.velocity.x < 0.0);
        assert!(world_to_cell(transform.translation).is_some_and(|cell| cell == (0, 0)));
    }
}
