use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy_sprite3d::prelude::*;
use bevy::asset::LoadState;
mod maze;
use maze::{Wall, create_maze};

#[derive(Component)]
struct Player;

#[derive(Component)]
struct FollowCamera;

// Add a resource to track camera drag state
#[derive(Resource, Default)]
struct CameraDragState {
    dragging: bool,
    last_position: Option<Vec2>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(Sprite3dPlugin)
        .init_resource::<CameraDragState>() // Initialize the camera drag state
        .add_systems(Startup, setup)
        .add_systems(Update, (player_movement, camera_follow, camera_drag))
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut sprite_params: Sprite3dParams,
) {
    info!("Loading textures...");
    let monkey_texture = asset_server.load("images/monkey.png");
    let wall_texture = asset_server.load("images/wall.png");
    let floor_texture = asset_server.load("images/floor.png");
    let roof_texture = asset_server.load("images/roof.png");

    // Log the loaded textures
    info!("Textures loaded: monkey");
    
    info!("Loading assets...");
    
    // Block until the monkey texture is loaded
    info!("Waiting for monkey texture to load...");
    let mut loaded = false;
    while !loaded {
        let load_state = asset_server.get_load_state(&monkey_texture);
        if load_state.is_some() && load_state.unwrap().is_loaded() {
            loaded = true;
            info!("Monkey texture loaded successfully!");
        } else {
            info!("Monkey texture is still loading...");
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    // Log the loaded textures
    info!("All textures loaded: monkey, wall, floor, roof");
    
    // Create maze
    let (start_x, start_y) = create_maze(&mut commands, wall_texture, floor_texture, roof_texture, &mut sprite_params);

    // Log the starting position of the monkey
    info!("Monkey starting position: ({}, {})", start_x, start_y);

    // Spawn the monkey sprite at the starting position using Sprite3d
    let monkey_sprite = Sprite3dBuilder {
        image: monkey_texture,
        pixels_per_metre: 32.0,
        alpha_mode: AlphaMode::Mask(0.5),
        unlit: false,
        double_sided: true,
        ..Default::default()
    }.bundle(&mut sprite_params);

    commands.spawn((
        monkey_sprite,
        Transform::from_xyz(start_x, 16.0, start_y),
        Player,
    ));

    // Spawn a point light at the monkey's position
    commands.spawn((
        PointLight {
            color: Color::WHITE,
            intensity: 5000.0,
            range: 200.0,
            ..default()
        },
        Transform::from_xyz(start_x, 16.0, start_y),
    ));

    // Spawn a 3D camera that follows the player
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(start_x, 50.0, start_y - 100.0)
            .looking_at(Vec3::new(start_x, 16.0, start_y), Vec3::Y),
        FollowCamera,
    ));
}

// Add a new system for camera dragging
fn camera_drag(
    mut camera_query: Query<&mut Transform, With<FollowCamera>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut drag_state: ResMut<CameraDragState>,
) {
    // Start dragging when the right mouse button is pressed
    if mouse_button.just_pressed(MouseButton::Right) {
        drag_state.dragging = true;
        drag_state.last_position = None;
    }
    
    // Stop dragging when the right mouse button is released
    if mouse_button.just_released(MouseButton::Right) {
        drag_state.dragging = false;
    }
    
    // If we're dragging, move the camera
    if drag_state.dragging {
        let mut camera_transform = if let Ok(transform) = camera_query.get_single_mut() {
            transform
        } else {
            return;
        };
        
        // Calculate the total motion since the last frame
        let mut total_motion = Vec2::ZERO;
        for event in mouse_motion.read() {
            total_motion += event.delta;
        }
        
        // If there's no motion, do nothing
        if total_motion.length_squared() < 0.1 {
            return;
        }
        
        // Move the camera based on the mouse motion
        // Invert the motion to make it feel more natural
        let sensitivity = 0.5;
        let motion_x = -total_motion.x * sensitivity;
        let motion_z = -total_motion.y * sensitivity;
        
        // Move the camera in the XZ plane (keeping Y constant)
        camera_transform.translation.x += motion_x;
        camera_transform.translation.z += motion_z;
    }
}

// Modify the camera_follow system to only work when not dragging
fn camera_follow(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<FollowCamera>, Without<Player>)>,
    drag_state: Res<CameraDragState>,
) {
    // Only follow the player if we're not dragging
    if !drag_state.dragging {
        if let Ok(player_transform) = player_query.get_single() {
            if let Ok(mut camera_transform) = camera_query.get_single_mut() {
                // Position camera above and behind the player
                camera_transform.translation.x = player_transform.translation.x;
                camera_transform.translation.z = player_transform.translation.z - 100.0;
                camera_transform.look_at(player_transform.translation, Vec3::Y);
            }
        }
    }
}

// Keep the existing player_movement system
fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<&mut Transform, With<Player>>,
    wall_query: Query<&Transform, With<Wall>>,
) {
    // Get player direction
    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        direction.z += 1.0; // Move forward
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction.z -= 1.0; // Move backward
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction.x -= 1.0; // Move left
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction.x += 1.0; // Move right
    }

    if direction.length() <= 0.0 {
        return;
    }

    direction = direction.normalize();
    
    // Get player position and calculate new position
    let speed = 2.0;
    
    if let Ok(mut player_transform) = player_query.get_single_mut() {
        let current_pos = player_transform.translation;
        let new_pos = current_pos + direction * speed;
        
        // Simple collision detection
        let mut collision = false;
        for wall_transform in wall_query.iter() {
            let wall_pos = wall_transform.translation;
            let distance = Vec2::new(new_pos.x - wall_pos.x, new_pos.z - wall_pos.z).length();
            
            if distance < 24.0 { // Approximate collision radius
                collision = true;
                break;
            }
        }
        
        if !collision {
            player_transform.translation = new_pos;
        }
    }
}
