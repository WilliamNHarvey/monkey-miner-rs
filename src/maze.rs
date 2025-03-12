use bevy::prelude::*;
use bevy_sprite3d::prelude::*;

#[derive(Component)]
pub struct Wall;

#[derive(Component)]
pub struct Floor;

#[derive(Component)]
pub struct Roof;

pub fn create_maze(
    commands: &mut Commands,
    wall_texture: Handle<Image>,
    floor_texture: Handle<Image>,
    roof_texture: Handle<Image>,
    sprite_params : &mut Sprite3dParams,
) -> (f32, f32) {
    // Define maze layout (1 = wall, 0 = path, 2 = starting position)
    let maze_layout = [
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        [1, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 1, 1, 0, 1, 1, 1, 0, 1],
        [1, 0, 1, 0, 0, 0, 0, 1, 0, 1],
        [1, 0, 1, 0, 1, 1, 0, 1, 0, 1],
        [1, 0, 0, 0, 2, 0, 0, 0, 0, 1], // 2 marks the starting position
        [1, 0, 1, 0, 1, 0, 1, 1, 0, 1],
        [1, 0, 1, 1, 1, 0, 1, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 1, 0, 0, 1],
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    ];

    let tile_size = 32.0;
    let wall_height = 32.0;
    let maze_width = maze_layout[0].len() as f32;
    let maze_height = maze_layout.len() as f32;
    
    // Center the maze
    let offset_x = -(maze_width * tile_size) / 2.0 + tile_size / 2.0;
    let offset_y = -(maze_height * tile_size) / 2.0 + tile_size / 2.0;

    // Track the starting position
    let mut start_x = 0.0;
    let mut start_y = 0.0;

    // Create floor (under everything)
    commands.spawn((
        Sprite3dBuilder {
            image: floor_texture,
            pixels_per_metre: 16.0,
            double_sided: true,
            ..default()
        }.bundle(sprite_params),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Floor,
    ));

    // Create walls
    for y in 0..maze_layout.len() {
        for x in 0..maze_layout[0].len() {
            let pos_x = x as f32 * tile_size + offset_x;
            let pos_y = y as f32 * tile_size + offset_y;
            
            if maze_layout[y][x] == 1 {
                // North wall (if needed)
                if y == 0 || maze_layout[y-1][x] == 0 {
                    commands.spawn((
                        Sprite3dBuilder {
                            image: wall_texture.clone(),
                            pixels_per_metre: 16.0,
                            double_sided: true,
                            ..default()
                        }.bundle(sprite_params),
                        Transform {
                            translation: Vec3::new(pos_x, pos_y + tile_size/2.0, wall_height/2.0),
                            rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                            ..default()
                        },
                        Wall,
                    ));
                }
                
                // South wall (if needed)
                if y == maze_layout.len() - 1 || maze_layout[y+1][x] == 0 {
                    commands.spawn((
                        Sprite3dBuilder {
                            image: wall_texture.clone(),
                            pixels_per_metre: 16.0,
                            double_sided: true,
                            ..default()
                        }.bundle(sprite_params),
                        Transform {
                            translation: Vec3::new(pos_x, pos_y - tile_size/2.0, wall_height/2.0),
                            rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                            ..default()
                        },
                        Wall,
                    ));
                }
                
                // East wall (if needed)
                if x == maze_layout[0].len() - 1 || maze_layout[y][x+1] == 0 {
                    commands.spawn((
                        Sprite3dBuilder {
                            image: wall_texture.clone(),
                            pixels_per_metre: 16.0,
                            double_sided: true,
                            ..default()
                        }.bundle(sprite_params),
                        Transform {
                            translation: Vec3::new(pos_x + tile_size/2.0, pos_y, wall_height/2.0),
                            rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2) * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                            ..default()
                        },
                        Wall,
                    ));
                }
                
                // West wall (if needed)
                if x == 0 || maze_layout[y][x-1] == 0 {
                    commands.spawn((
                        Sprite3dBuilder {
                            image: wall_texture.clone(),
                            pixels_per_metre: 16.0,
                            double_sided: true,
                            ..default()
                        }.bundle(sprite_params),
                        Transform {
                            translation: Vec3::new(pos_x - tile_size/2.0, pos_y, wall_height/2.0),
                            rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2) * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                            ..default()
                        },
                        Wall,
                    ));
                }
            } else if maze_layout[y][x] == 2 {
                // This is the starting position
                start_x = pos_x;
                start_y = pos_y;
            }
        }
    }
    
    // Create roof (covers the entire maze)
    commands.spawn((
        Sprite3dBuilder {
            image: roof_texture,
            pixels_per_metre: 16.0,
            double_sided: true,
            ..default()
        }.bundle(sprite_params),
        Transform::from_xyz(0.0, 0.0, wall_height), // Place at the top of the walls
        Roof,
    ));

    // Return the starting position
    (start_x, start_y)
} 
