use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::window::CursorGrabMode;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::image::{ImageSampler, ImageSamplerDescriptor, ImageAddressMode, ImageFilterMode};

#[derive(Component)]
struct Player;

#[derive(Component)]
struct PlayerVelocity { vertical: f32 }

#[derive(Component)]
struct PlayerCamera { pitch: f32, bob_timer: f32 }

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Dark Souls Lite".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.06, 0.08, 0.22)))
        .add_systems(Startup, (setup, spawn_castle))
        .add_systems(Update, (player_movement, camera_look, head_bob, cursor_grab))
        .run();
}

fn make_grass_texture(images: &mut Assets<Image>) -> Handle<Image> {
    // 6-shade retro grass palette: dark → bright greens + yellow-green
    let pal: &[[u8; 3]] = &[
        [15, 48, 12],  // 0 very dark
        [22, 65, 16],  // 1 dark green
        [32, 85, 22],  // 2 medium green
        [42, 105, 28], // 3 light green
        [52, 92, 18],  // 4 yellow-green
        [28, 75, 20],  // 5 mid-dark
    ];
    // 8x8 hand-placed pattern — no two same neighbours, organic feel
    #[rustfmt::skip]
    let pat: &[u8] = &[
        1,0,3,5,2,4,0,3,
        4,2,1,0,5,1,3,2,
        0,5,4,2,1,3,5,1,
        3,1,0,5,4,0,2,4,
        5,3,2,1,0,5,1,0,
        2,0,5,4,3,2,4,3,
        4,3,1,0,5,1,0,5,
        0,5,4,3,2,4,3,2,
    ];
    let size = 8u32;
    let mut data: Vec<u8> = Vec::with_capacity((size * size * 4) as usize);
    for &p in pat {
        let c = pal[p as usize];
        data.extend_from_slice(&[c[0], c[1], c[2], 255]);
    }
    let mut image = Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        ..default()
    });
    images.add(image)
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // Ground
    let grass = make_grass_texture(&mut images);
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(400.0, 400.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(grass),
            uv_transform: bevy::math::Affine2::from_scale(Vec2::new(100.0, 100.0)),
            perceptual_roughness: 1.0,
            ..default()
        })),
        Transform::default(),
    ));

    // Moonlight
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.65, 0.75, 1.0),
            illuminance: 2200.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(-30.0, 50.0, -20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Bright ambient so the scene is readable
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.30, 0.35, 0.55),
        brightness: 320.0,
    });

    // Moon — large bright sphere, high in the sky
    let moon_dir = Vec3::new(-0.6, 1.4, -1.0).normalize();
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(11.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.96, 0.85),
            emissive: LinearRgba::new(5.0, 5.0, 4.0, 1.0),
            unlit: true,
            ..default()
        })),
        Transform::from_translation(moon_dir * 350.0),
    ));

    // Stars — small emissive cubes, upper hemisphere
    let star_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::new(6.0, 6.0, 6.0, 1.0),
        unlit: true,
        ..default()
    });
    let star_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    for i in 0..200u32 {
        let t = i as f32;
        let phi = (t * 47.0 % 360.0).to_radians();
        let el  = ((t * 23.0 % 70.0) + 10.0).to_radians();
        let r   = 360.0 + (i % 25) as f32;
        let x = r * el.cos() * phi.cos();
        let y = r * el.sin();
        let z = r * el.cos() * phi.sin();
        let size = 0.5 + (i % 4) as f32 * 0.3;
        commands.spawn((
            Mesh3d(star_mesh.clone()),
            MeshMaterial3d(star_mat.clone()),
            Transform::from_xyz(x, y, z).with_scale(Vec3::splat(size)),
        ));
    }

    // --- Trees (low-poly pine: trunk + two stacked cones) ---
    let trunk_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.20, 0.09),
        perceptual_roughness: 1.0,
        ..default()
    });
    let leaf_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.09, 0.30, 0.11),
        perceptual_roughness: 1.0,
        ..default()
    });
    // .resolution(6) = 6-sided faceted cone — angular low-poly PS1 look
    let trunk_mesh = meshes.add(Cuboid::new(0.5, 4.5, 0.5));
    let cone_lo    = meshes.add(Cone { radius: 3.5, height: 6.5 }.mesh().resolution(6));
    let cone_mid   = meshes.add(Cone { radius: 2.2, height: 4.5 }.mesh().resolution(6));
    let cone_hi    = meshes.add(Cone { radius: 1.2, height: 3.0 }.mesh().resolution(6));

    for i in 0..80u32 {
        let t     = i as f32;
        let angle = t * 137.508_f32.to_radians();
        let dist  = (18.0 + t * 1.5_f32).min(175.0);
        let x     = dist * angle.cos();
        let z     = dist * angle.sin();

        if x.abs() < 14.0 && z > -6.0 && z < 22.0 { continue; }   // player spawn
        if x.abs() < 48.0 && z < -38.0 && z > -142.0 { continue; } // castle zone

        let base = Vec3::new(x, 0.0, z);

        // Trunk: height 4.5, center at y=2.25
        commands.spawn((
            Mesh3d(trunk_mesh.clone()),
            MeshMaterial3d(trunk_mat.clone()),
            Transform::from_translation(base + Vec3::Y * 2.25),
        ));
        // Bottom cone: h=6.5, center at y=7.5 → base at 4.25, tip at 10.75
        commands.spawn((
            Mesh3d(cone_lo.clone()),
            MeshMaterial3d(leaf_mat.clone()),
            Transform::from_translation(base + Vec3::Y * 7.5),
        ));
        // Mid cone: h=4.5, center at y=9.5 → base at 7.25, tip at 11.75
        commands.spawn((
            Mesh3d(cone_mid.clone()),
            MeshMaterial3d(leaf_mat.clone()),
            Transform::from_translation(base + Vec3::Y * 9.5),
        ));
        // Top cone: h=3.0, center at y=11.25 → base at 9.75, tip at 12.75
        commands.spawn((
            Mesh3d(cone_hi.clone()),
            MeshMaterial3d(leaf_mat.clone()),
            Transform::from_translation(base + Vec3::Y * 11.25),
        ));
    }

    // Player + first-person camera
    commands
        .spawn((
            Player,
            Transform::from_xyz(0.0, 0.0, 10.0),
            GlobalTransform::default(),
            Visibility::default(),
            PlayerVelocity { vertical: 0.0 },
        ))
        .with_children(|parent| {
            parent.spawn((
                Camera3d::default(),
                Transform::from_xyz(0.0, 1.7, 0.0),
                PlayerCamera { pitch: 0.0, bob_timer: 0.0 },
            ));
        });
}

fn spawn_castle(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.36, 0.40),
        perceptual_roughness: 0.95, ..default()
    });
    let dark_stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.23, 0.28),
        perceptual_roughness: 0.95, ..default()
    });
    let wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.26, 0.12),
        perceptual_roughness: 1.0, ..default()
    });
    let bone_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.74, 0.70, 0.62),
        perceptual_roughness: 0.8, ..default()
    });
    let flame_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.6, 0.05),
        emissive: LinearRgba::new(5.0, 2.2, 0.1, 1.0),
        unlit: true, ..default()
    });

    // Castle layout — center (0, 0, -90), 88×88 outer footprint
    let cz    = -90.0f32;
    let hw    =  44.0f32;  // half-width  (x: -44 to 44)
    let hd    =  44.0f32;  // half-depth  (z: -46 to -134)
    let wh    =  18.0f32;  // wall height
    let wt    =   3.5f32;  // wall thickness
    let gate  =   7.0f32;  // gate half-width

    let front_z = cz + hd; // -46
    let back_z  = cz - hd; // -134
    let side_w  = hw - gate; // 37

    // ── Perimeter walls ──────────────────────────────────────
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(side_w, wh, wt))),
        MeshMaterial3d(stone.clone()),
        Transform::from_xyz(-gate - side_w*0.5, wh*0.5, front_z)));
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(side_w, wh, wt))),
        MeshMaterial3d(stone.clone()),
        Transform::from_xyz( gate + side_w*0.5, wh*0.5, front_z)));
    let lintel_h = wh - 13.0;
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(gate*2.0, lintel_h, wt))),
        MeshMaterial3d(dark_stone.clone()),
        Transform::from_xyz(0.0, 13.0 + lintel_h*0.5, front_z)));
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(hw*2.0, wh, wt))),
        MeshMaterial3d(stone.clone()),
        Transform::from_xyz(0.0, wh*0.5, back_z)));
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(wt, wh, hd*2.0))),
        MeshMaterial3d(stone.clone()),
        Transform::from_xyz(-hw, wh*0.5, cz)));
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(wt, wh, hd*2.0))),
        MeshMaterial3d(stone.clone()),
        Transform::from_xyz( hw, wh*0.5, cz)));

    // ── Corner towers ─────────────────────────────────────────
    let tw = 10.0f32; let th = 26.0f32;
    for (tx, tz) in [(-hw, front_z),(hw, front_z),(-hw, back_z),(hw, back_z)] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(tw, th, tw))),
            MeshMaterial3d(dark_stone.clone()),
            Transform::from_xyz(tx, th*0.5, tz)));
    }

    // ── Keep ──────────────────────────────────────────────────
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(20.0, 34.0, 20.0))),
        MeshMaterial3d(dark_stone.clone()),
        Transform::from_xyz(0.0, 17.0, cz - 12.0)));

    // ── Battlements (all four walls) ──────────────────────────
    let bw = 2.5f32; let bh = 4.0f32; let bs = 5.5f32;
    // front left
    let mut x = -hw + 2.0;
    while x < -gate - bw { commands.spawn((Mesh3d(meshes.add(Cuboid::new(bw,bh,wt))),MeshMaterial3d(stone.clone()),Transform::from_xyz(x,wh+bh*0.5,front_z))); x+=bs; }
    // front right
    let mut x = gate + 2.0;
    while x < hw { commands.spawn((Mesh3d(meshes.add(Cuboid::new(bw,bh,wt))),MeshMaterial3d(stone.clone()),Transform::from_xyz(x,wh+bh*0.5,front_z))); x+=bs; }
    // back
    let mut x = -hw + 2.0;
    while x < hw { commands.spawn((Mesh3d(meshes.add(Cuboid::new(bw,bh,wt))),MeshMaterial3d(stone.clone()),Transform::from_xyz(x,wh+bh*0.5,back_z))); x+=bs; }
    // sides
    for wx in [-hw, hw] {
        let mut z = back_z + 2.0;
        while z < front_z { commands.spawn((Mesh3d(meshes.add(Cuboid::new(wt,bh,bw))),MeshMaterial3d(stone.clone()),Transform::from_xyz(wx,wh+bh*0.5,z))); z+=bs; }
    }

    // ── Interior pillars ──────────────────────────────────────
    let pillar = meshes.add(Cuboid::new(3.5, 17.0, 3.5));
    for (px, pz) in [(-20.0f32,-72.0f32),(20.0,-72.0),(-20.0,-108.0),(20.0,-108.0)] {
        commands.spawn((Mesh3d(pillar.clone()), MeshMaterial3d(dark_stone.clone()),
            Transform::from_xyz(px, 8.5, pz)));
    }

    // ── Altar ─────────────────────────────────────────────────
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(24.0, 1.8, 12.0))),
        MeshMaterial3d(dark_stone.clone()),
        Transform::from_xyz(0.0, 0.9, cz - 30.0)));
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(24.0, 0.7, 3.0))),
        MeshMaterial3d(stone.clone()),
        Transform::from_xyz(0.0, 0.35, cz - 25.5)));

    // ── Barrels ───────────────────────────────────────────────
    let barrel = meshes.add(Cylinder { radius: 0.55, half_height: 0.65 });
    for (bx, bz) in [(-40.0f32,-60.0f32),(-38.0,-62.5),(39.0,-61.0),(-39.0,-118.0),(38.0,-117.0)] {
        commands.spawn((Mesh3d(barrel.clone()), MeshMaterial3d(wood.clone()),
            Transform::from_xyz(bx, 0.65, bz)));
    }

    // ── Crates ────────────────────────────────────────────────
    let crate_m = meshes.add(Cuboid::new(1.2, 1.1, 1.2));
    for (cx2,cz2) in [(-41.0f32,-75.0f32),(-40.5,-76.5),(40.0,-92.0),(40.5,-93.5),(-41.0,-105.0)] {
        commands.spawn((Mesh3d(crate_m.clone()), MeshMaterial3d(wood.clone()),
            Transform::from_xyz(cx2, 0.55, cz2)));
    }

    // ── Bone piles ────────────────────────────────────────────
    let bone = meshes.add(Cuboid::new(0.9, 0.14, 0.45));
    for (bx,bz) in [(-22.0f32,-68.0f32),(21.0,-74.0),(-16.0,-112.0),(24.0,-98.0),(3.0,-132.0)] {
        commands.spawn((Mesh3d(bone.clone()), MeshMaterial3d(bone_mat.clone()),
            Transform::from_xyz(bx, 0.07, bz)));
    }

    // ── Torches (handle + flame + point light) ────────────────
    let t_handle = meshes.add(Cuboid::new(0.16, 0.9, 0.16));
    let t_flame  = meshes.add(Cuboid::new(0.24, 0.30, 0.24));
    let torch_spots = [
        Vec3::new(-10.0, 4.5, front_z + 2.0),  // gate left
        Vec3::new( 10.0, 4.5, front_z + 2.0),  // gate right
        Vec3::new(-hw + 2.5, 5.0, -72.0),      // left wall front
        Vec3::new(-hw + 2.5, 5.0, -108.0),     // left wall back
        Vec3::new( hw - 2.5, 5.0, -72.0),      // right wall front
        Vec3::new( hw - 2.5, 5.0, -108.0),     // right wall back
        Vec3::new(-10.0, 5.0, cz - 26.0),      // keep entrance left
        Vec3::new( 10.0, 5.0, cz - 26.0),      // keep entrance right
    ];
    for p in torch_spots {
        commands.spawn((Mesh3d(t_handle.clone()), MeshMaterial3d(wood.clone()),
            Transform::from_translation(p)));
        commands.spawn((Mesh3d(t_flame.clone()), MeshMaterial3d(flame_mat.clone()),
            Transform::from_translation(p + Vec3::Y * 0.62)));
        commands.spawn((
            PointLight {
                color: Color::srgb(1.0, 0.55, 0.15),
                intensity: 120_000.0,
                range: 22.0,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(p + Vec3::Y * 0.75),
        ));
    }
}

fn cursor_grab(
    mut windows: Query<&mut Window>,
    mouse: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
) {
    let mut window = windows.single_mut();
    if mouse.just_pressed(MouseButton::Left) {
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        window.cursor_options.visible = false;
    }
    if key.just_pressed(KeyCode::Escape) {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
    }
}

fn player_movement(
    key: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut player_q: Query<(&mut Transform, &mut PlayerVelocity), With<Player>>,
) {
    let (mut transform, mut velocity) = player_q.single_mut();
    let dt = time.delta_secs();

    let sprinting = key.pressed(KeyCode::ShiftLeft) || key.pressed(KeyCode::ShiftRight);
    let speed = if sprinting { 12.0 } else { 6.0 };

    let fwd = *transform.forward();
    let right = *transform.right();
    let forward    = Vec3::new(fwd.x,   0.0, fwd.z  ).normalize_or_zero();
    let right_flat = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

    let mut dir = Vec3::ZERO;
    if key.pressed(KeyCode::KeyW) { dir += forward; }
    if key.pressed(KeyCode::KeyS) { dir -= forward; }
    if key.pressed(KeyCode::KeyA) { dir -= right_flat; }
    if key.pressed(KeyCode::KeyD) { dir += right_flat; }
    if dir.length_squared() > 0.0 {
        transform.translation += dir.normalize() * speed * dt;
    }

    let on_ground = transform.translation.y <= 0.0;
    if key.just_pressed(KeyCode::Space) && on_ground {
        velocity.vertical = 8.5;
    }
    velocity.vertical -= 22.0 * dt;
    transform.translation.y += velocity.vertical * dt;
    if transform.translation.y < 0.0 {
        transform.translation.y = 0.0;
        velocity.vertical = 0.0;
    }
}

fn camera_look(
    mut mouse_motion: EventReader<MouseMotion>,
    mut player_q: Query<&mut Transform, (With<Player>, Without<PlayerCamera>)>,
    mut camera_q: Query<(&mut Transform, &mut PlayerCamera), Without<Player>>,
) {
    let sensitivity = 0.003;
    let (mut cam_t, mut ctrl) = camera_q.single_mut();
    let mut player_t = player_q.single_mut();
    for ev in mouse_motion.read() {
        player_t.rotate_y(-ev.delta.x * sensitivity);
        ctrl.pitch = (ctrl.pitch - ev.delta.y * sensitivity).clamp(-1.4, 1.4);
        cam_t.rotation = Quat::from_rotation_x(ctrl.pitch);
    }
}

fn head_bob(
    key: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut camera_q: Query<(&mut Transform, &mut PlayerCamera)>,
) {
    let (mut t, mut ctrl) = camera_q.single_mut();
    let moving = key.pressed(KeyCode::KeyW) || key.pressed(KeyCode::KeyS)
              || key.pressed(KeyCode::KeyA) || key.pressed(KeyCode::KeyD);
    if moving { ctrl.bob_timer += time.delta_secs() * 9.0; }
    t.translation.y = 1.7 + if moving { ctrl.bob_timer.sin() * 0.045 } else { 0.0 };
}
