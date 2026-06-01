use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::window::CursorGrabMode;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::image::{ImageSampler, ImageSamplerDescriptor, ImageAddressMode, ImageFilterMode};

#[derive(Component)]
struct Player;

#[derive(Component)]
struct PlayerVelocity { vertical: f32, knockback: Vec3 }

#[derive(Component)]
struct PlayerCamera { pitch: f32, bob_timer: f32 }

#[derive(Component)]
struct Sword { swinging: bool, timer: f32, hit_registered: bool }

#[derive(Component)]
struct LightningOrb;

#[derive(Component)]
struct LightningBolt { bolt_idx: u32, seg_idx: u32 }

#[derive(Component, PartialEq, Clone)]
enum SkeletonState { Patrol, Chase, Attack, Dead }

#[derive(Component)]
struct Skeleton {
    health: f32,
    state: SkeletonState,
    attack_timer: f32,
    patrol_timer: f32,
    patrol_dir: Vec3,
    damage_flash: f32,
    knockback_vel: Vec3,
}

#[derive(Component)]
struct HeartNode { index: u32 }

#[derive(Component)]
struct DamageVignette;

#[derive(Resource)]
struct PlayerHealth { hearts: i32, hurt_timer: f32 }


#[derive(Resource)]
struct SkeletonMaterials { bone: Handle<StandardMaterial>, flash: Handle<StandardMaterial> }

#[derive(Component, PartialEq, Clone)]
enum DragonState { Idle, Active, Dead }

#[derive(Component)]
struct Dragon { health: f32, fireball_timer: f32, damage_flash: f32, state: DragonState }

#[derive(Component)]
struct Fireball { velocity: Vec3, life: f32 }

#[derive(Resource)]
struct DragonAssets {
    scale_mat: Handle<StandardMaterial>,
    flash_mat: Handle<StandardMaterial>,
    fb_mat:    Handle<StandardMaterial>,
    fb_mesh:   Handle<Mesh>,
}

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
        .insert_resource(PlayerHealth { hearts: 5, hurt_timer: 0.0 })
        .add_systems(Startup, (setup, spawn_castle, spawn_skeletons, setup_hud, spawn_dragon))
        .add_systems(Update, (player_movement, camera_look, head_bob, cursor_grab,
                               sword_swing, animate_lightning, lightning_bolts,
                               skeleton_ai, skeleton_flash, lightning_damage,
                               dragon_ai, dragon_flash, move_fireballs,
                               update_hearts, update_vignette))
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

    // ── Materials ─────────────────────────────────────────────
    let blade_mat  = materials.add(StandardMaterial { base_color: Color::srgb(0.68, 0.80, 0.98), emissive: LinearRgba::new(0.10, 0.16, 0.32, 1.0), perceptual_roughness: 1.0, ..default() });
    let blade_edge = materials.add(StandardMaterial { base_color: Color::srgb(0.92, 0.96, 1.00), emissive: LinearRgba::new(0.35, 0.40, 0.55, 1.0), perceptual_roughness: 1.0, ..default() });
    let gold_mat   = materials.add(StandardMaterial { base_color: Color::srgb(0.92, 0.75, 0.12), emissive: LinearRgba::new(0.30, 0.22, 0.02, 1.0), perceptual_roughness: 1.0, ..default() });
    let grip_mat   = materials.add(StandardMaterial { base_color: Color::srgb(0.55, 0.08, 0.08), perceptual_roughness: 1.0, ..default() });
    let gauntlet_m = materials.add(StandardMaterial { base_color: Color::srgb(0.18, 0.16, 0.20), perceptual_roughness: 1.0, ..default() });
    let orb_mat    = materials.add(StandardMaterial { base_color: Color::srgb(0.45, 0.70, 1.0),  emissive: LinearRgba::new(1.8, 3.5, 7.0, 1.0),  unlit: true, ..default() });
    let spark_mat  = materials.add(StandardMaterial { base_color: Color::WHITE, emissive: LinearRgba::new(4.0, 6.5, 11.0, 1.0), unlit: true, ..default() });
    let bolt_mat   = materials.add(StandardMaterial { base_color: Color::srgb(0.5, 0.75, 1.0),   emissive: LinearRgba::new(3.0, 6.0, 12.0, 1.0), unlit: true, ..default() });
    let bolt2_mat  = materials.add(StandardMaterial { base_color: Color::WHITE, emissive: LinearRgba::new(5.0, 8.0, 16.0, 1.0), unlit: true, ..default() });

    // ── Player + camera ───────────────────────────────────────
    let player_e = commands.spawn((
        Player, Transform::from_xyz(0.0, 0.0, 10.0),
        GlobalTransform::default(), Visibility::default(),
        PlayerVelocity { vertical: 0.0, knockback: Vec3::ZERO },
    )).id();
    let camera_e = commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.7, 0.0),
        PlayerCamera { pitch: 0.0, bob_timer: 0.0 },
    )).set_parent(player_e).id();

    // ── Sword (right hand) — smaller, proper sword proportions ─
    let idle_rot = Quat::from_euler(EulerRot::XYZ,
        (-24f32).to_radians(), (6f32).to_radians(), (16f32).to_radians());
    let sword_root = commands.spawn((
        Transform::from_xyz(0.26, -0.30, -0.42).with_rotation(idle_rot),
        GlobalTransform::default(), Visibility::default(),
        Sword { swinging: false, timer: 0.0, hit_registered: false },
    )).set_parent(camera_e).id();

    // Blade — narrow and very thin (real sword profile)
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.036, 0.44, 0.007))),
        MeshMaterial3d(blade_mat),
        Transform::from_xyz(0.0, 0.22, 0.0), Visibility::default(),
    )).set_parent(sword_root);
    // Bright edge strip
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.005, 0.44, 0.009))),
        MeshMaterial3d(blade_edge),
        Transform::from_xyz(-0.021, 0.22, 0.0), Visibility::default(),
    )).set_parent(sword_root);
    // Crossguard
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.22, 0.046, 0.050))),
        MeshMaterial3d(gold_mat.clone()),
        Transform::from_xyz(0.0, -0.024, 0.0), Visibility::default(),
    )).set_parent(sword_root);
    // Grip
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.046, 0.16, 0.046))),
        MeshMaterial3d(grip_mat),
        Transform::from_xyz(0.0, -0.128, 0.0), Visibility::default(),
    )).set_parent(sword_root);
    // Pommel
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.068, 0.062, 0.062))),
        MeshMaterial3d(gold_mat),
        Transform::from_xyz(0.0, -0.237, 0.0), Visibility::default(),
    )).set_parent(sword_root);

    // ── Left hand: gauntlet + orb + lightning bolts ───────────
    // Moved higher (-0.18 Y) so it's visible on screen
    let hand_root = commands.spawn((
        Transform::from_xyz(-0.23, -0.18, -0.40)
            .with_rotation(Quat::from_euler(EulerRot::XYZ,
                (-8f32).to_radians(), 0.0, (-10f32).to_radians())),
        GlobalTransform::default(), Visibility::default(),
        LightningOrb,
    )).set_parent(camera_e).id();

    // Gauntlet
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.13, 0.042, 0.11))),
        MeshMaterial3d(gauntlet_m.clone()),
        Transform::from_xyz(0.0, 0.0, 0.02), Visibility::default(),
    )).set_parent(hand_root);
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.13, 0.055, 0.036))),
        MeshMaterial3d(gauntlet_m.clone()),
        Transform::from_xyz(0.0, 0.028, -0.048), Visibility::default(),
    )).set_parent(hand_root);
    for fi in 0..3i32 {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.030, 0.022, 0.060))),
            MeshMaterial3d(gauntlet_m.clone()),
            Transform::from_xyz((fi-1) as f32 * 0.042, 0.018, -0.096), Visibility::default(),
        )).set_parent(hand_root);
    }
    // Orb
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.060))),
        MeshMaterial3d(orb_mat.clone()),
        Transform::from_xyz(0.0, 0.01, -0.18), Visibility::default(),
    )).set_parent(hand_root);
    // Glow shell
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.085))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.3, 0.6, 1.0, 0.0),
            emissive: LinearRgba::new(0.5, 1.0, 2.5, 1.0),
            unlit: true, alpha_mode: AlphaMode::Add, ..default()
        })),
        Transform::from_xyz(0.0, 0.01, -0.18), Visibility::default(),
    )).set_parent(hand_root);
    // Static sparks around orb
    for i in 0..4u32 {
        let a = (i as f32 / 4.0) * std::f32::consts::TAU;
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.014, 0.052, 0.014))),
            MeshMaterial3d(spark_mat.clone()),
            Transform::from_xyz(a.cos()*0.10, a.sin()*0.09, -0.18), Visibility::default(),
        )).set_parent(hand_root);
    }

    // ── Lightning bolt segments (hidden until RMB held) ────────
    // 7 bolts × 5 segments = 35 thin cuboid entities
    let bolt_mesh  = meshes.add(Cuboid::new(0.016, 0.016, 0.22));
    let bolt2_mesh = meshes.add(Cuboid::new(0.009, 0.009, 0.16));
    for bolt_idx in 0..7u32 {
        for seg_idx in 0..5u32 {
            let m = if seg_idx < 3 { bolt_mat.clone() } else { bolt2_mat.clone() };
            let msh = if seg_idx < 3 { bolt_mesh.clone() } else { bolt2_mesh.clone() };
            commands.spawn((
                Mesh3d(msh), MeshMaterial3d(m),
                Transform::from_xyz(0.0, 0.01, -0.18),
                Visibility::Hidden,
                LightningBolt { bolt_idx, seg_idx },
            )).set_parent(hand_root);
        }
    }
}

fn sword_swing(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    time: Res<Time>,
    mut sword_q: Query<(&mut Transform, &mut Sword)>,
    camera_q: Query<&GlobalTransform, With<PlayerCamera>>,
    mut skeleton_q: Query<(Entity, &GlobalTransform, &mut Skeleton)>,
    mut dragon_q: Query<(Entity, &GlobalTransform, &mut Dragon)>,
    mut commands: Commands,
) {
    let window = windows.single();
    let (mut t, mut sword) = sword_q.single_mut();

    let idle_rot = Quat::from_euler(EulerRot::XYZ,
        (-24f32).to_radians(), (6f32).to_radians(), (16f32).to_radians());
    let idle_pos = Vec3::new(0.26, -0.30, -0.42);

    if mouse.just_pressed(MouseButton::Left)
        && window.cursor_options.grab_mode == CursorGrabMode::Locked
        && !sword.swinging
    {
        sword.swinging = true;
        sword.timer = 0.0;
        sword.hit_registered = false;
    }

    if sword.swinging {
        sword.timer += time.delta_secs();
        let progress = (sword.timer / 0.38).min(1.0);
        let arc = (progress * std::f32::consts::PI).sin();
        let swing_rot = Quat::from_euler(EulerRot::XYZ,
            (-55.0 * arc).to_radians(), (-35.0 * arc).to_radians(), 0.0f32.to_radians());
        t.rotation    = idle_rot * swing_rot;
        t.translation = idle_pos + Vec3::new(-0.12 * arc, 0.06 * arc, -0.08 * arc);

        // Hit check at swing peak
        if !sword.hit_registered && sword.timer > 0.14 {
            sword.hit_registered = true;
            let cam_gt = camera_q.single();
            let (_, rot, cam_pos) = cam_gt.to_scale_rotation_translation();
            let fwd = rot * Vec3::NEG_Z;
            for (entity, skel_gt, mut skel) in skeleton_q.iter_mut() {
                if skel.state == SkeletonState::Dead { continue; }
                let to = skel_gt.translation() - cam_pos;
                let dist = to.length();
                if dist < 3.5 && dist > 0.1 && fwd.dot(to / dist) > 0.3 {
                    skel.health -= 1.0;
                    skel.damage_flash = 0.25;
                    let knock = Vec3::new(to.x, 0.0, to.z).normalize_or_zero();
                    skel.knockback_vel = knock * 7.0;
                    if skel.health <= 0.0 {
                        skel.state = SkeletonState::Dead;
                        commands.entity(entity).despawn_recursive();
                    }
                }
            }
            // Dragon hit
            for (entity, dgt, mut drag) in dragon_q.iter_mut() {
                if drag.state == DragonState::Dead { continue; }
                let to = dgt.translation() - cam_pos;
                let dist = to.length();
                if dist < 5.5 && dist > 0.1 && fwd.dot(to / dist) > 0.2 {
                    drag.health -= 1.0;
                    drag.damage_flash = 0.25;
                    if drag.health <= 0.0 {
                        drag.state = DragonState::Dead;
                        commands.entity(entity).despawn_recursive();
                    }
                }
            }
        }

        if sword.timer >= 0.38 {
            sword.swinging = false;
            t.rotation    = idle_rot;
            t.translation = idle_pos;
        }
    }
}

fn animate_lightning(
    time: Res<Time>,
    mut orb_q: Query<&mut Transform, With<LightningOrb>>,
) {
    // Pulse the hand_root scale so the orb + sparks breathe
    for mut t in orb_q.iter_mut() {
        let pulse = 1.0 + (time.elapsed_secs() * 6.0).sin() * 0.07;
        t.scale = Vec3::splat(pulse);
    }
}

fn lightning_bolts(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    time: Res<Time>,
    mut bolt_q: Query<(&mut Transform, &mut Visibility, &LightningBolt)>,
) {
    let window = windows.single();
    let active = mouse.pressed(MouseButton::Right)
        && window.cursor_options.grab_mode == CursorGrabMode::Locked;

    // Snap time to ~22 Hz so bolts flicker rather than animate smoothly
    let t = (time.elapsed_secs() * 22.0).floor() / 22.0;

    // Spread directions per bolt (x, y per unit of z-depth)
    let dirs: &[(f32, f32)] = &[
        ( 0.00,  0.00),   // center
        ( 0.20,  0.12),   // upper-right
        (-0.20,  0.12),   // upper-left
        ( 0.20, -0.10),   // lower-right
        (-0.20, -0.10),   // lower-left
        ( 0.00,  0.28),   // straight up
        ( 0.00, -0.16),   // slightly down
    ];

    for (mut tr, mut vis, bolt) in bolt_q.iter_mut() {
        if !active { *vis = Visibility::Hidden; continue; }
        *vis = Visibility::Visible;

        let (dx, dy) = dirs[bolt.bolt_idx as usize % dirs.len()];
        let depth   = 0.22 + bolt.seg_idx as f32 * 0.28;
        let spread  = 1.0  + bolt.seg_idx as f32 * 0.45;

        // Hash-derived jitter — different per bolt, segment, and time snapshot
        let s   = bolt.bolt_idx as f32 * 11.3 + bolt.seg_idx as f32 * 7.9 + t * 43.0;
        let jx  = (s * 127.1).sin()         * 0.038 * (bolt.seg_idx + 1) as f32;
        let jy  = (s * 311.7 + 1.4).cos()   * 0.038 * (bolt.seg_idx + 1) as f32;

        tr.translation = Vec3::new(
            dx * depth * spread + jx,
            0.01 + dy * depth * spread + jy,
            -0.18 - depth,            // extends forward from palm
        );

        // Slight pitch/yaw makes each segment look jagged
        let rx = (s * 73.1  + 2.7).sin() * 0.40;
        let ry = (s * 189.3 + 0.9).cos() * 0.40;
        tr.rotation = Quat::from_euler(EulerRot::XYZ, rx, ry, 0.0);
    }
}

// ── Skeletons ─────────────────────────────────────────────────────────────────
fn spawn_skeletons(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let bone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.80, 0.76, 0.64),
        perceptual_roughness: 0.9, ..default()
    });
    let flash = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.08, 0.08),
        emissive: LinearRgba::new(2.0, 0.1, 0.1, 1.0),
        ..default()
    });
    commands.insert_resource(SkeletonMaterials { bone: bone.clone(), flash });
    // Pre-build mesh handles (shared across all skeletons)
    let head    = meshes.add(Cuboid::new(0.22, 0.22, 0.22));
    let torso   = meshes.add(Cuboid::new(0.30, 0.52, 0.13));
    let pelvis  = meshes.add(Cuboid::new(0.26, 0.17, 0.11));
    let thigh   = meshes.add(Cuboid::new(0.10, 0.34, 0.10));
    let shin    = meshes.add(Cuboid::new(0.09, 0.34, 0.09));
    let uarm    = meshes.add(Cuboid::new(0.09, 0.28, 0.09));
    let farm    = meshes.add(Cuboid::new(0.08, 0.26, 0.08));
    let spear   = meshes.add(Cuboid::new(0.038, 1.6, 0.038));

    let positions = [
        Vec3::new( 5.0, 0.0, -22.0),
        Vec3::new(-8.0, 0.0, -30.0),
        Vec3::new(12.0, 0.0, -36.0),
        Vec3::new(-5.0, 0.0, -26.0),
        Vec3::new( 8.0, 0.0, -52.0),
        Vec3::new(-14.0,0.0, -55.0),
        Vec3::new( 6.0, 0.0, -78.0),
        Vec3::new(-12.0,0.0, -88.0),
    ];

    for (i, pos) in positions.iter().enumerate() {
        let angle = i as f32 * 0.9 + 0.5;
        let patrol_dir = Vec3::new(angle.cos(), 0.0, angle.sin());
        let b = bone.clone();
        commands.spawn((
            Transform::from_translation(*pos),
            GlobalTransform::default(),
            Visibility::default(),
            Skeleton { health: 5.0, state: SkeletonState::Patrol,
                attack_timer: 1.5, patrol_timer: 2.0 + i as f32 * 0.4, patrol_dir,
                damage_flash: 0.0, knockback_vel: Vec3::ZERO },
        )).with_children(|p| {
            // head
            p.spawn((Mesh3d(head.clone()),   MeshMaterial3d(b.clone()), Transform::from_xyz( 0.00, 1.72, 0.0)));
            // torso
            p.spawn((Mesh3d(torso.clone()),  MeshMaterial3d(b.clone()), Transform::from_xyz( 0.00, 1.14, 0.0)));
            // pelvis
            p.spawn((Mesh3d(pelvis.clone()), MeshMaterial3d(b.clone()), Transform::from_xyz( 0.00, 0.77, 0.0)));
            // thighs
            p.spawn((Mesh3d(thigh.clone()),  MeshMaterial3d(b.clone()), Transform::from_xyz(-0.10, 0.52, 0.0)));
            p.spawn((Mesh3d(thigh.clone()),  MeshMaterial3d(b.clone()), Transform::from_xyz( 0.10, 0.52, 0.0)));
            // shins
            p.spawn((Mesh3d(shin.clone()),   MeshMaterial3d(b.clone()), Transform::from_xyz(-0.10, 0.17, 0.0)));
            p.spawn((Mesh3d(shin.clone()),   MeshMaterial3d(b.clone()), Transform::from_xyz( 0.10, 0.17, 0.0)));
            // upper arms
            p.spawn((Mesh3d(uarm.clone()),   MeshMaterial3d(b.clone()), Transform::from_xyz(-0.23, 1.28, 0.0)));
            p.spawn((Mesh3d(uarm.clone()),   MeshMaterial3d(b.clone()), Transform::from_xyz( 0.23, 1.28, 0.0)));
            // forearms
            p.spawn((Mesh3d(farm.clone()),   MeshMaterial3d(b.clone()), Transform::from_xyz(-0.25, 0.97, 0.0)));
            p.spawn((Mesh3d(farm.clone()),   MeshMaterial3d(b.clone()), Transform::from_xyz( 0.25, 0.97, 0.0)));
            // spear
            p.spawn((Mesh3d(spear.clone()),  MeshMaterial3d(b.clone()), Transform::from_xyz( 0.40, 0.82, 0.0)));
        });
    }
}

fn skeleton_ai(
    time: Res<Time>,
    mut commands: Commands,
    mut skel_q: Query<(Entity, &mut Transform, &mut Skeleton)>,
    player_q: Query<&Transform, (With<Player>, Without<Skeleton>)>,
    mut player_vel_q: Query<&mut PlayerVelocity, With<Player>>,
    mut health: ResMut<PlayerHealth>,
) {
    if health.hurt_timer > 0.0 { health.hurt_timer -= time.delta_secs(); }
    let pt = player_q.single();
    let pp = pt.translation;

    for (entity, mut t, mut sk) in skel_q.iter_mut() {
        if sk.state == SkeletonState::Dead { continue; }

        // Apply and decay knockback
        if sk.knockback_vel.length_squared() > 0.01 {
            t.translation += sk.knockback_vel * time.delta_secs();
            sk.knockback_vel *= (1.0 - 8.0 * time.delta_secs()).max(0.0);
            t.translation.y = 0.0;
        }

        let flat = Vec3::new(pp.x - t.translation.x, 0.0, pp.z - t.translation.z);
        let dist = flat.length();
        sk.state = if dist < 2.2 { SkeletonState::Attack }
                   else if dist < 20.0 { SkeletonState::Chase }
                   else { SkeletonState::Patrol };

        match sk.state.clone() {
            SkeletonState::Patrol => {
                sk.patrol_timer -= time.delta_secs();
                if sk.patrol_timer <= 0.0 {
                    let s = t.translation.x * 7.3 + t.translation.z * 3.1 + time.elapsed_secs();
                    sk.patrol_dir = Vec3::new(s.sin(), 0.0, s.cos()).normalize();
                    sk.patrol_timer = 2.5 + (s.abs() % 1.8);
                }
                t.translation += sk.patrol_dir * 1.5 * time.delta_secs();
                t.translation.y = 0.0;
                let look = t.translation + sk.patrol_dir;
                if (look - t.translation).length_squared() > 0.01 { t.look_at(look, Vec3::Y); }
            }
            SkeletonState::Chase => {
                if dist > 0.1 {
                    let dir = flat.normalize();
                    t.translation += dir * 3.0 * time.delta_secs();
                    t.translation.y = 0.0;
                    let ty = t.translation.y; t.look_at(Vec3::new(pp.x, ty, pp.z), Vec3::Y);
                }
            }
            SkeletonState::Attack => {
                let ty = t.translation.y; t.look_at(Vec3::new(pp.x, ty, pp.z), Vec3::Y);
                sk.attack_timer -= time.delta_secs();
                if sk.attack_timer <= 0.0 && health.hurt_timer <= 0.0 {
                    health.hearts = (health.hearts - 1).max(0);
                    health.hurt_timer = 0.9;
                    sk.attack_timer = 1.5;
                    // Knock player away from skeleton
                    let mut pvel = player_vel_q.single_mut();
                    pvel.knockback = Vec3::new(flat.x, 0.0, flat.z).normalize_or_zero() * 5.5;
                }
            }
            SkeletonState::Dead => { commands.entity(entity).despawn_recursive(); }
        }
    }
}

fn lightning_damage(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    time: Res<Time>,
    camera_q: Query<&GlobalTransform, With<PlayerCamera>>,
    mut skel_q: Query<(Entity, &GlobalTransform, &mut Skeleton)>,
    mut dragon_q: Query<(Entity, &GlobalTransform, &mut Dragon)>,
    mut commands: Commands,
) {
    let window = windows.single();
    if !mouse.pressed(MouseButton::Right) { return; }
    if window.cursor_options.grab_mode != CursorGrabMode::Locked { return; }
    let cam = camera_q.single();
    let (_, rot, pos) = cam.to_scale_rotation_translation();
    let fwd = rot * Vec3::NEG_Z;
    let dt = time.delta_secs();
    for (entity, sgt, mut sk) in skel_q.iter_mut() {
        if sk.state == SkeletonState::Dead { continue; }
        let to = sgt.translation() + Vec3::Y - pos;
        let dist = to.length();
        if dist < 0.5 { continue; }
        if dist < 9.0 && fwd.dot(to / dist) > 0.5 {
            sk.health -= 2.0 * dt;
            if sk.health <= 0.0 { sk.state = SkeletonState::Dead; commands.entity(entity).despawn_recursive(); }
        }
    }
    for (entity, dgt, mut drag) in dragon_q.iter_mut() {
        if drag.state == DragonState::Dead { continue; }
        let to = dgt.translation() + Vec3::Y * 2.0 - pos;
        let dist = to.length();
        if dist < 0.5 { continue; }
        if dist < 12.0 && fwd.dot(to / dist) > 0.4 {
            drag.health -= 2.0 * dt;
            drag.damage_flash = 0.12;
            if drag.health <= 0.0 { drag.state = DragonState::Dead; commands.entity(entity).despawn_recursive(); }
        }
    }
}

// ── HUD ───────────────────────────────────────────────────────────────────────
fn setup_hud(mut commands: Commands) {
    // Hearts row (top-left) — Unicode ♥ characters, colored by health state
    commands.spawn(Node {
        position_type: PositionType::Absolute,
        left: Val::Px(16.0), top: Val::Px(16.0),
        flex_direction: FlexDirection::Row, column_gap: Val::Px(4.0),
        ..default()
    }).with_children(|p| {
        for i in 0..5u32 {
            p.spawn((
                Text::new("♥"),
                TextFont { font_size: 40.0, ..default() },
                TextColor(Color::srgb(0.9, 0.1, 0.1)),
                HeartNode { index: i },
            ));
        }
    });

    // 4 corner damage vignettes
    for (left, top, right, bottom) in [
        (Val::Px(0.), Val::Px(0.), Val::Auto,  Val::Auto ),
        (Val::Auto,  Val::Px(0.), Val::Px(0.), Val::Auto ),
        (Val::Px(0.), Val::Auto,  Val::Auto,   Val::Px(0.)),
        (Val::Auto,  Val::Auto,  Val::Px(0.), Val::Px(0.)),
    ] {
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(45.0), height: Val::Percent(45.0),
                left, top, right, bottom,
                ..default()
            },
            BackgroundColor(Color::srgba(0.95, 0.0, 0.0, 0.0)),
            DamageVignette,
        ));
    }
}

fn update_hearts(
    health: Res<PlayerHealth>,
    mut hearts: Query<(&mut TextColor, &HeartNode)>,
) {
    for (mut color, h) in hearts.iter_mut() {
        color.0 = if (h.index as i32) < health.hearts {
            Color::srgb(0.90, 0.12, 0.12)
        } else {
            Color::srgb(0.22, 0.05, 0.05)
        };
    }
}

fn update_vignette(
    health: Res<PlayerHealth>,
    mut vignette_q: Query<&mut BackgroundColor, With<DamageVignette>>,
) {
    let alpha = if health.hurt_timer > 0.0 { (health.hurt_timer / 0.9 * 0.60).min(0.60) } else { 0.0 };
    for mut bg in vignette_q.iter_mut() {
        bg.0 = Color::srgba(0.95, 0.0, 0.0, alpha);
    }
}

fn skeleton_flash(
    time: Res<Time>,
    mats: Option<Res<SkeletonMaterials>>,
    mut skel_q: Query<(&mut Skeleton, &Children)>,
    mut mat_q: Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    let Some(m) = mats else { return; };
    for (mut sk, children) in skel_q.iter_mut() {
        if sk.state == SkeletonState::Dead { continue; }
        sk.damage_flash = (sk.damage_flash - time.delta_secs()).max(0.0);
        let handle = if sk.damage_flash > 0.0 { m.flash.clone() } else { m.bone.clone() };
        for &child in children.iter() {
            if let Ok(mut mat) = mat_q.get_mut(child) { mat.0 = handle.clone(); }
        }
    }
}

fn spawn_dragon(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let scale = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.28, 0.10),
        perceptual_roughness: 0.85, ..default()
    });
    let flash = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.1, 0.1),
        emissive: LinearRgba::new(2.0, 0.1, 0.1, 1.0), ..default()
    });
    let eye = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.0, 0.0),
        emissive: LinearRgba::new(6.0, 0.0, 0.0, 1.0), unlit: true, ..default()
    });
    let fb_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.45, 0.0),
        emissive: LinearRgba::new(4.0, 1.5, 0.0, 1.0), unlit: true, ..default()
    });
    let fb_mesh = meshes.add(Sphere::new(0.28));
    commands.insert_resource(DragonAssets {
        scale_mat: scale.clone(), flash_mat: flash,
        fb_mat, fb_mesh,
    });

    let sc = scale;
    // Dragon faces +Z (toward gate at z=-46), starting at z=-115
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, -115.0)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        GlobalTransform::default(), Visibility::default(),
        Dragon { health: 20.0, fireball_timer: 4.0, damage_flash: 0.0, state: DragonState::Idle },
    )).with_children(|p| {
        // Body
        p.spawn((Mesh3d(meshes.add(Cuboid::new(2.8, 2.0, 4.5))), MeshMaterial3d(sc.clone()), Transform::from_xyz(0.0, 2.2, 0.0)));
        // Neck
        p.spawn((Mesh3d(meshes.add(Cuboid::new(1.0, 1.4, 1.8))), MeshMaterial3d(sc.clone()), Transform::from_xyz(0.0, 3.2, -1.8)));
        // Head
        p.spawn((Mesh3d(meshes.add(Cuboid::new(2.0, 1.4, 2.2))), MeshMaterial3d(sc.clone()), Transform::from_xyz(0.0, 3.6, -3.6)));
        // Snout
        p.spawn((Mesh3d(meshes.add(Cuboid::new(0.9, 0.5, 1.2))), MeshMaterial3d(sc.clone()), Transform::from_xyz(0.0, 3.2, -4.9)));
        // Eyes
        for ex in [-0.65f32, 0.65] {
            p.spawn((Mesh3d(meshes.add(Cuboid::new(0.28, 0.22, 0.15))), MeshMaterial3d(eye.clone()), Transform::from_xyz(ex, 3.9, -4.0)));
        }
        // Wings
        for (wx, rz) in [(-3.8f32, 0.38f32), (3.8, -0.38)] {
            p.spawn((Mesh3d(meshes.add(Cuboid::new(4.2, 0.18, 3.0))), MeshMaterial3d(sc.clone()),
                Transform::from_xyz(wx, 2.8, 0.0).with_rotation(Quat::from_rotation_z(rz))));
        }
        // 4 legs
        for (lx, lz) in [(-0.9f32,-1.5f32),(0.9,-1.5),(-0.9,1.5),(0.9,1.5)] {
            p.spawn((Mesh3d(meshes.add(Cuboid::new(0.5, 1.2, 0.5))), MeshMaterial3d(sc.clone()), Transform::from_xyz(lx, 0.6, lz)));
        }
        // Tail
        for &(tw,th,tl,tz,ty) in &[(1.2f32,0.9f32,1.8f32,2.8f32,1.6f32),(0.8,0.6,1.5,4.5,1.3),(0.5,0.4,1.1,6.2,1.0)] {
            p.spawn((Mesh3d(meshes.add(Cuboid::new(tw,th,tl))), MeshMaterial3d(sc.clone()), Transform::from_xyz(0.0,ty,tz)));
        }
    });
}

fn dragon_ai(
    time: Res<Time>,
    mut commands: Commands,
    mut dragon_q: Query<(Entity, &mut Transform, &mut Dragon)>,
    player_q: Query<&Transform, (With<Player>, Without<Dragon>)>,
    assets: Option<Res<DragonAssets>>,
) {
    let Some(assets) = assets else { return; };
    let pt = player_q.single();
    let pp = pt.translation + Vec3::Y * 1.5;

    for (entity, mut t, mut dragon) in dragon_q.iter_mut() {
        if dragon.state == DragonState::Dead { continue; }
        let to = pp - (t.translation + Vec3::Y * 3.0);
        let dist = to.length();

        if dist < 85.0 { dragon.state = DragonState::Active; }
        if dragon.state != DragonState::Active { continue; }

        // Face player (horizontal)
        let ty = t.translation.y;
        let look = Vec3::new(pp.x, ty, pp.z);
        if (look - Vec3::new(t.translation.x, ty, t.translation.z)).length_squared() > 0.1 {
            t.look_at(look, Vec3::Y);
        }

        dragon.fireball_timer -= time.delta_secs();
        if dragon.fireball_timer <= 0.0 {
            dragon.fireball_timer = 3.5;
            let fwd = t.rotation * Vec3::NEG_Z;
            let fire_pos = t.translation + Vec3::Y * 3.8 + fwd * 2.8;
            let vel = to.normalize() * 13.0;
            commands.spawn((
                Mesh3d(assets.fb_mesh.clone()),
                MeshMaterial3d(assets.fb_mat.clone()),
                Transform::from_translation(fire_pos),
                Fireball { velocity: vel, life: 8.0 },
                PointLight { color: Color::srgb(1.0, 0.4, 0.0), intensity: 90_000.0,
                    range: 14.0, shadows_enabled: false, ..default() },
            ));
        }

        if dragon.health <= 0.0 {
            dragon.state = DragonState::Dead;
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn dragon_flash(
    time: Res<Time>,
    assets: Option<Res<DragonAssets>>,
    mut dragon_q: Query<(&mut Dragon, &Children)>,
    mut mat_q: Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    let Some(a) = assets else { return; };
    for (mut d, children) in dragon_q.iter_mut() {
        if d.state == DragonState::Dead { continue; }
        d.damage_flash = (d.damage_flash - time.delta_secs()).max(0.0);
        let handle = if d.damage_flash > 0.0 { a.flash_mat.clone() } else { a.scale_mat.clone() };
        for &child in children.iter() {
            if let Ok(mut m) = mat_q.get_mut(child) { m.0 = handle.clone(); }
        }
    }
}

fn move_fireballs(
    time: Res<Time>,
    mut commands: Commands,
    mut fb_q: Query<(Entity, &mut Transform, &mut Fireball)>,
    player_q: Query<&Transform, (With<Player>, Without<Fireball>)>,
    mut player_vel: Query<&mut PlayerVelocity, With<Player>>,
    mut health: ResMut<PlayerHealth>,
) {
    let pt = player_q.single();
    let pp = pt.translation + Vec3::Y * 1.0;
    for (entity, mut t, mut fb) in fb_q.iter_mut() {
        t.translation += fb.velocity * time.delta_secs();
        fb.life -= time.delta_secs();
        let dist = (pp - t.translation).length();
        if dist < 1.4 && health.hurt_timer <= 0.0 {
            health.hearts = (health.hearts - 2).max(0);
            health.hurt_timer = 0.9;
            let mut pvel = player_vel.single_mut();
            let dir = (pp - t.translation).normalize_or_zero();
            pvel.knockback = Vec3::new(dir.x, 0.0, dir.z) * 9.0;
            commands.entity(entity).despawn();
        } else if fb.life <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
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

    // Apply knockback (from enemy hits / fireballs)
    if velocity.knockback.length_squared() > 0.01 {
        transform.translation += velocity.knockback * dt;
        velocity.knockback *= (1.0 - 9.0 * dt).max(0.0);
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
