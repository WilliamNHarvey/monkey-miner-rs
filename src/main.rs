use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy_sprite3d::prelude::*;
use rand::seq::SliceRandom;
mod maze;
use maze::{
    Direction, FogCell, MAZE_COLUMNS, MAZE_ROWS, MazeMap, WallCollider, cell_center, create_maze,
    world_to_cell,
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

#[derive(Resource, Default)]
struct RunState {
    won: bool,
}

#[derive(Resource)]
struct TrailMarkers {
    marked: [[bool; MAZE_COLUMNS]; MAZE_ROWS],
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
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
                rat_movement,
                camera_drag,
                camera_follow,
                update_fog,
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
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Setting up game...");

    let start_position = create_maze(
        &mut commands,
        game_assets.wall.clone(),
        game_assets.floor.clone(),
        game_assets.roof.clone(),
        &mut meshes,
        &mut materials,
    );

    info!("Monkey starting position: {:?}", start_position);

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

    let player_xz = Vec2::new(player_transform.translation.x, player_transform.translation.z);
    let exit_xz = Vec2::new(exit_transform.translation.x, exit_transform.translation.z);
    if player_xz.distance(exit_xz) < 32.0 {
        run_state.won = true;
        info!("You found the smiley face exit!");
    }
}

fn rat_movement(time: Res<Time>, maze: Res<MazeMap>, mut rat_query: Query<(&mut Transform, &mut Rat)>) {
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
    match (to.0 as isize - from.0 as isize, to.1 as isize - from.1 as isize) {
        (0, 1) => Some(Direction::North),
        (1, 0) => Some(Direction::East),
        (0, -1) => Some(Direction::South),
        (-1, 0) => Some(Direction::West),
        _ => None,
    }
}

fn camera_drag(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut orbit: ResMut<CameraOrbit>,
) {
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
