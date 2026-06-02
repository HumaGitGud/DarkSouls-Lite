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

#[derive(Component)]
struct LightningLight;

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
struct Enemy {
    health: f32,
    speed: f32,
    flying: bool,
    base_y: f32,
    attack_timer: f32,
    knockback_vel: Vec3,
    bob_phase: f32,
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
struct DragonPart { base: Handle<StandardMaterial> }

#[derive(Component)]
struct Fireball { velocity: Vec3, life: f32 }

#[derive(Component)]
struct Eye { alert: f32 }

#[derive(Component)]
struct EyePupil;

#[derive(Component)]
struct EyeBeamSeg { idx: u32 }

/// Axis-aligned XZ collider (half-extents) for solid static structures.
#[derive(Component)]
struct Collider { half: Vec2 }

/// Lightning "shocked" state — drives a blue/white aura on a hit enemy.
#[derive(Component)]
struct Shock { timer: f32 }

#[derive(Component)]
struct ShockAura;

#[derive(Component)]
struct SkeletonSpear;

#[derive(Resource)]
struct EyeAssets { flame: Handle<StandardMaterial> }

#[derive(Component)]
struct DeathScreen;

#[derive(Component)]
struct RespawnButton;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum AppState { #[default] Playing, Dead }

#[derive(Resource)]
struct DragonAssets {
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
        .init_state::<AppState>()
        .add_systems(Startup, (setup, spawn_castle, spawn_skeletons, setup_hud, spawn_dragon,
                               spawn_spire, spawn_mountains, spawn_enemies))
        // Always-on systems
        .add_systems(Update, (update_hearts, update_vignette, animate_lightning,
                               animate_eye, check_death))
        // Gameplay systems — only while alive
        .add_systems(Update, (player_movement, resolve_collisions, camera_look, head_bob, cursor_grab,
                               sword_swing, lightning_bolts,
                               skeleton_ai, skeleton_flash, skeleton_attack_anim, lightning_damage,
                               dragon_ai, dragon_flash, move_fireballs,
                               eye_beam, enemy_ai, update_shock)
                               .chain()
                               .run_if(in_state(AppState::Playing)))
        // Death screen
        .add_systems(OnEnter(AppState::Dead), spawn_death_screen)
        .add_systems(OnExit(AppState::Dead),
                     (despawn_death_screen, reset_game, spawn_skeletons, spawn_dragon).chain())
        .add_systems(Update, death_button.run_if(in_state(AppState::Dead)))
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

fn make_tower_texture(images: &mut Assets<Image>) -> Handle<Image> {
    // 16×16 dark obsidian brick pattern with offset rows + faint warm flecks
    let w = 16u32; let h = 16u32;
    let mut data: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        let brick_row = y / 4;
        let offset = if brick_row % 2 == 0 { 0u32 } else { 4 };
        for x in 0..w {
            let h_mortar = y % 4 == 0;
            let v_mortar = (x + offset) % 8 == 0;
            let (r, g, b) = if h_mortar || v_mortar {
                (10u8, 10, 15)                         // dark mortar
            } else {
                let v = ((x * 7 + y * 13) % 5) as u8;  // per-brick variation
                let warm = if (x * 3 + y * 5) % 17 == 0 { 14u8 } else { 0 }; // rare ember fleck
                (34 + v * 3 + warm, 30 + v * 2, 40 + v * 3)
            };
            data.extend_from_slice(&[r, g, b, 255]);
        }
    }
    let mut image = Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb,
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

fn make_rock_texture(images: &mut Assets<Image>) -> Handle<Image> {
    // 16×16 grey-brown rocky noise
    let w = 16u32; let h = 16u32;
    let mut data: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let n = ((x * 13 + y * 7 + x * y) % 7) as u8;
            let s = ((x * 5 + y * 11) % 4) as u8;
            let r = 70 + n * 6 + s * 3;
            let g = 66 + n * 5 + s * 3;
            let b = 60 + n * 5;
            data.extend_from_slice(&[r, g, b, 255]);
        }
    }
    let mut image = Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb,
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

fn spawn_mountains(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let rock_tex = make_rock_texture(&mut images);
    let rock = materials.add(StandardMaterial {
        base_color_texture: Some(rock_tex),
        uv_transform: bevy::math::Affine2::from_scale(Vec2::new(6.0, 6.0)),
        perceptual_roughness: 1.0, ..default()
    });
    let snow = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.94, 1.0),
        perceptual_roughness: 0.7, ..default()
    });

    // Sparse chunky peaks spaced around the map edges (not interconnected).
    let spire = Vec2::new(320.0, 240.0);
    let count = 13u32;
    for i in 0..count {
        let a = (i as f32 / count as f32) * std::f32::consts::TAU;
        let dist = 545.0 + ((i as f32 * 53.7).sin()) * 25.0;
        let x = dist * a.cos();
        let z = dist * a.sin();
        // Keep a wide clearing around the Eye of Sauron tower
        if Vec2::new(x, z).distance(spire) < 200.0 { continue; }

        let scale = 1.0 + ((i * 7) % 5) as f32 * 0.16; // 1.0 .. 1.64
        let r = 110.0 * scale;   // chunky, but spaced so they don't merge
        let hgt = 140.0 * scale;

        commands.spawn((
            Mesh3d(meshes.add(Cone { radius: r, height: hgt })),
            MeshMaterial3d(rock.clone()),
            Transform::from_xyz(x, hgt * 0.5, z),
        ));
        let sh = hgt * 0.32;
        commands.spawn((
            Mesh3d(meshes.add(Cone { radius: r * 0.40, height: sh })),
            MeshMaterial3d(snow.clone()),
            Transform::from_xyz(x, hgt - sh * 0.5, z),
        ));
    }
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
        Mesh3d(meshes.add(Plane3d::default().mesh().size(1200.0, 1200.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(grass),
            uv_transform: bevy::math::Affine2::from_scale(Vec2::new(300.0, 300.0)),
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
        Mesh3d(meshes.add(Sphere::new(32.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.96, 0.85),
            emissive: LinearRgba::new(5.0, 5.0, 4.0, 1.0),
            unlit: true,
            ..default()
        })),
        Transform::from_translation(moon_dir * 920.0),
    ));

    // Stars — small emissive cubes, upper hemisphere
    let star_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::new(6.0, 6.0, 6.0, 1.0),
        unlit: true,
        ..default()
    });
    let star_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    for i in 0..600u32 {
        let t = i as f32;
        let phi = (t * 47.0 % 360.0).to_radians();
        let el  = ((t * 23.0 % 72.0) + 8.0).to_radians();
        let r   = 950.0 + (i % 40) as f32 * 2.0;
        let x = r * el.cos() * phi.cos();
        let y = r * el.sin();
        let z = r * el.cos() * phi.sin();
        // Scale stars up to stay visible at the greater distance
        let size = 1.4 + (i % 4) as f32 * 0.8;
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

    for i in 0..320u32 {
        let t     = i as f32;
        let angle = t * 137.508_f32.to_radians();
        let dist  = (18.0 + t * 1.8_f32).min(560.0);
        let x     = dist * angle.cos();
        let z     = dist * angle.sin();

        if x.abs() < 14.0 && z > -6.0 && z < 22.0 { continue; }   // player spawn
        if x.abs() < 66.0 && z < -25.0 && z > -160.0 { continue; } // castle zone
        // spire zone (moved away to +X/+Z): keep clear around (320, 240)
        if (x - 320.0).abs() < 30.0 && (z - 240.0).abs() < 30.0 { continue; }

        let base = Vec3::new(x, 0.0, z);

        // Trunk: height 4.5, center at y=2.25 (with collider)
        commands.spawn((
            Mesh3d(trunk_mesh.clone()),
            MeshMaterial3d(trunk_mat.clone()),
            Transform::from_translation(base + Vec3::Y * 2.25),
            Collider { half: Vec2::new(0.4, 0.4) },
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
        Transform::from_xyz(0.26, -0.18, -0.40).with_rotation(idle_rot),
        GlobalTransform::default(), Visibility::default(),
        Sword { swinging: false, timer: 0.0, hit_registered: false },
    )).set_parent(camera_e).id();

    // ── Blade — thin, tapered, with fuller + bright edges + pointed tip (shortened) ──
    // Lower blade (wider section)
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.034, 0.24, 0.006))),
        MeshMaterial3d(blade_mat.clone()),
        Transform::from_xyz(0.0, 0.16, 0.0), Visibility::default(),
    )).set_parent(sword_root);
    // Upper blade (tapered narrower)
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.022, 0.10, 0.005))),
        MeshMaterial3d(blade_mat.clone()),
        Transform::from_xyz(0.0, 0.33, 0.0), Visibility::default(),
    )).set_parent(sword_root);
    // Pointed tip — flattened 4-sided cone (pyramid)
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.013, height: 0.07 }.mesh().resolution(4))),
        MeshMaterial3d(blade_mat.clone()),
        Transform::from_xyz(0.0, 0.415, 0.0).with_scale(Vec3::new(1.0, 1.0, 0.45)),
        Visibility::default(),
    )).set_parent(sword_root);
    // Central fuller — bright groove down the blade face
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.006, 0.34, 0.008))),
        MeshMaterial3d(blade_edge.clone()),
        Transform::from_xyz(0.0, 0.19, 0.0), Visibility::default(),
    )).set_parent(sword_root);
    // Bright cutting edges (left + right)
    for ex in [-0.016f32, 0.016] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.004, 0.24, 0.0075))),
            MeshMaterial3d(blade_edge.clone()),
            Transform::from_xyz(ex, 0.16, 0.0), Visibility::default(),
        )).set_parent(sword_root);
    }

    // ── Crossguard — slim bar with upswept quillon tips ──
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.19, 0.026, 0.042))),
        MeshMaterial3d(gold_mat.clone()),
        Transform::from_xyz(0.0, -0.012, 0.0), Visibility::default(),
    )).set_parent(sword_root);
    for (qx, qrz) in [(-0.092f32, 0.5f32), (0.092, -0.5)] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.05, 0.022, 0.040))),
            MeshMaterial3d(gold_mat.clone()),
            Transform::from_xyz(qx, 0.004, 0.0).with_rotation(Quat::from_rotation_z(qrz)),
            Visibility::default(),
        )).set_parent(sword_root);
    }

    // ── Grip — wrapped leather cylinder with gold ferrules ──
    commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.017, half_height: 0.075 })),
        MeshMaterial3d(grip_mat),
        Transform::from_xyz(0.0, -0.10, 0.0), Visibility::default(),
    )).set_parent(sword_root);
    for ry in [-0.055f32, -0.105, -0.145] {
        commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.019, half_height: 0.006 })),
            MeshMaterial3d(gold_mat.clone()),
            Transform::from_xyz(0.0, ry, 0.0), Visibility::default(),
        )).set_parent(sword_root);
    }
    // ── Pommel — round gold knob ──
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.032))),
        MeshMaterial3d(gold_mat),
        Transform::from_xyz(0.0, -0.19, 0.0), Visibility::default(),
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

    // Open palm with fingers — pale skin + armored wrist cuff
    let hand_skin = materials.add(StandardMaterial {
        base_color: Color::srgb(0.80, 0.64, 0.55), perceptual_roughness: 0.9, ..default() });

    // Palm
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.105, 0.115, 0.034))),
        MeshMaterial3d(hand_skin.clone()),
        Transform::from_xyz(0.0, 0.0, 0.0), Visibility::default(),
    )).set_parent(hand_root);
    // Armored wrist cuff (toward camera)
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.12, 0.07, 0.06))),
        MeshMaterial3d(gauntlet_m.clone()),
        Transform::from_xyz(0.0, -0.02, 0.055), Visibility::default(),
    )).set_parent(hand_root);
    // Four fingers — proximal + distal segments, pointing forward, fanned & curled up
    for fi in 0..4i32 {
        let fx  = (fi as f32 - 1.5) * 0.028;
        let fan = (fi as f32 - 1.5) * 0.07;
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.020, 0.022, 0.050))),
            MeshMaterial3d(hand_skin.clone()),
            Transform::from_xyz(fx, 0.058, -0.045)
                .with_rotation(Quat::from_rotation_z(fan) * Quat::from_rotation_x(-0.25)),
            Visibility::default(),
        )).set_parent(hand_root);
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.018, 0.020, 0.044))),
            MeshMaterial3d(hand_skin.clone()),
            Transform::from_xyz(fx + fan * 0.04, 0.078, -0.090)
                .with_rotation(Quat::from_rotation_z(fan) * Quat::from_rotation_x(-0.55)),
            Visibility::default(),
        )).set_parent(hand_root);
    }
    // Thumb — two segments on the inner side, angled across
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.024, 0.024, 0.046))),
        MeshMaterial3d(hand_skin.clone()),
        Transform::from_xyz(0.058, -0.005, -0.02)
            .with_rotation(Quat::from_rotation_z(-0.7) * Quat::from_rotation_y(-0.3)),
        Visibility::default(),
    )).set_parent(hand_root);
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.020, 0.020, 0.040))),
        MeshMaterial3d(hand_skin.clone()),
        Transform::from_xyz(0.085, 0.025, -0.052)
            .with_rotation(Quat::from_rotation_z(-0.7) * Quat::from_rotation_y(-0.3)),
        Visibility::default(),
    )).set_parent(hand_root);
    // Spell tome held above the palm — the artifact that casts the lightning
    let leather = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.10, 0.08), perceptual_roughness: 0.9, ..default() });
    let pages_m = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.80, 0.66), perceptual_roughness: 1.0, ..default() });
    let book_rot = Quat::from_rotation_x(-0.6);
    // Leather cover
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.17, 0.045, 0.22))),
        MeshMaterial3d(leather.clone()),
        Transform::from_xyz(0.0, 0.05, -0.13).with_rotation(book_rot), Visibility::default(),
    )).set_parent(hand_root);
    // Page block (slightly proud of the cover)
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.155, 0.052, 0.205))),
        MeshMaterial3d(pages_m),
        Transform::from_xyz(0.0, 0.052, -0.128).with_rotation(book_rot), Visibility::default(),
    )).set_parent(hand_root);
    // Spine
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.022, 0.05, 0.22))),
        MeshMaterial3d(leather),
        Transform::from_xyz(-0.085, 0.05, -0.13).with_rotation(book_rot), Visibility::default(),
    )).set_parent(hand_root);
    // Glowing rune on the cover (reuses the blue lightning material)
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.05, 0.012, 0.05))),
        MeshMaterial3d(orb_mat.clone()),
        Transform::from_xyz(0.0, 0.076, -0.13).with_rotation(book_rot), Visibility::default(),
    )).set_parent(hand_root);
    // Sparks crackling above the tome
    for i in 0..4u32 {
        let a = (i as f32 / 4.0) * std::f32::consts::TAU;
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.014, 0.05, 0.014))),
            MeshMaterial3d(spark_mat.clone()),
            Transform::from_xyz(a.cos()*0.09, 0.13 + a.sin()*0.04, -0.16), Visibility::default(),
        )).set_parent(hand_root);
    }

    // ── Lightning bolt segments (hidden until RMB held) ────────
    // Connected jagged arcs: 10 bolts × 8 segments. Mesh is unit-length on Z
    // so each segment can be stretched/oriented to link two points of an arc.
    let bolt_core = meshes.add(Cuboid::new(0.012, 0.012, 1.0)); // white core
    let bolt_glow = meshes.add(Cuboid::new(0.028, 0.028, 1.0)); // blue halo
    for bolt_idx in 0..10u32 {
        for seg_idx in 0..8u32 {
            // Two overlaid layers per segment: thick blue glow + thin white core
            let blue = bolt_idx % 2 == 0;
            let (msh, m) = if blue {
                (bolt_glow.clone(), bolt_mat.clone())
            } else {
                (bolt_core.clone(), bolt2_mat.clone())
            };
            commands.spawn((
                Mesh3d(msh), MeshMaterial3d(m),
                Transform::from_xyz(0.0, 0.01, -0.18),
                Visibility::Hidden,
                LightningBolt { bolt_idx, seg_idx },
            )).set_parent(hand_root);
        }
    }

    // White flash light cast by the lightning (torch-style PointLight, off until RMB)
    commands.spawn((
        PointLight {
            color: Color::srgb(0.9, 0.95, 1.0),
            intensity: 0.0,
            range: 45.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, 0.05, -1.2), // out in front of the palm
        LightningLight,
    )).set_parent(hand_root);
}

fn sword_swing(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    time: Res<Time>,
    mut sword_q: Query<(&mut Transform, &mut Sword)>,
    camera_q: Query<&GlobalTransform, With<PlayerCamera>>,
    mut skeleton_q: Query<(Entity, &GlobalTransform, &mut Skeleton)>,
    mut dragon_q: Query<(Entity, &GlobalTransform, &mut Dragon)>,
    mut enemy_q: Query<(Entity, &GlobalTransform, &mut Enemy)>,
    mut commands: Commands,
) {
    let window = windows.single();
    let (mut t, mut sword) = sword_q.single_mut();

    let idle_rot = Quat::from_euler(EulerRot::XYZ,
        (-24f32).to_radians(), (6f32).to_radians(), (16f32).to_radians());
    let idle_pos = Vec3::new(0.26, -0.18, -0.40);

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
            // Witch / Knight / Bat hit
            for (entity, egt, mut en) in enemy_q.iter_mut() {
                let to = egt.translation() + Vec3::Y - cam_pos;
                let dist = to.length();
                if dist < 3.5 && dist > 0.1 && fwd.dot(to / dist) > 0.3 {
                    en.health -= 1.0;
                    en.knockback_vel = Vec3::new(to.x, 0.0, to.z).normalize_or_zero() * 6.0;
                    if en.health <= 0.0 {
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
    mut light_q: Query<&mut PointLight, With<LightningLight>>,
) {
    let window = windows.single();
    let active = mouse.pressed(MouseButton::Right)
        && window.cursor_options.grab_mode == CursorGrabMode::Locked;

    // Snap time to ~26 Hz so the arcs crackle/flicker rather than glide
    let t = (time.elapsed_secs() * 26.0).floor() / 26.0;

    // Drive the white flash light: bright + crackling while firing, off otherwise
    if let Ok(mut light) = light_q.get_single_mut() {
        light.intensity = if active {
            700_000.0 + (t * 60.0).sin() * 250_000.0
        } else { 0.0 };
    }

    let n_bolts = 10u32;
    let orb = Vec3::new(0.0, 0.01, -0.18);   // emit point (palm orb), local space
    let seg_len = 0.42f32;                    // forward step per segment

    // Deterministic jagged point along a bolt's path at segment k.
    // Computed identically wherever referenced so neighbouring segments connect.
    let point = |b: u32, k: u32| -> Vec3 {
        let depth = k as f32 * seg_len;
        // Each bolt fans out in its own direction around the forward axis
        let theta = (b as f32 / n_bolts as f32) * std::f32::consts::TAU
                  + (t * 0.7 + b as f32).sin() * 0.3;
        let fan = depth * 0.20;               // spread grows with distance
        let base = orb + Vec3::new(theta.cos() * fan, theta.sin() * fan, -depth);
        if k == 0 { return base; }            // root stays pinned to the orb
        // Crackle jitter, grows slightly further from the hand
        let s = b as f32 * 11.3 + k as f32 * 7.9 + t * 43.0;
        let amp = 0.045 + k as f32 * 0.012;
        base + Vec3::new(
            (s * 127.1).sin() * amp,
            (s * 311.7 + 1.4).cos() * amp,
            (s * 53.3 + 0.6).sin() * amp * 0.5,
        )
    };

    for (mut tr, mut vis, bolt) in bolt_q.iter_mut() {
        if !active { *vis = Visibility::Hidden; continue; }
        *vis = Visibility::Visible;

        let p0 = point(bolt.bolt_idx, bolt.seg_idx);
        let p1 = point(bolt.bolt_idx, bolt.seg_idx + 1);
        let mid = (p0 + p1) * 0.5;
        let delta = p1 - p0;
        let len = delta.length().max(0.0001);

        tr.translation = mid;
        tr.rotation = Quat::from_rotation_arc(Vec3::Z, delta / len);
        tr.scale = Vec3::new(1.0, 1.0, len); // stretch unit mesh to span the gap
    }
}

// ── Witches / Knights / Bats ────────────────────────────────────────────────
fn spawn_enemies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Materials
    let robe   = materials.add(StandardMaterial { base_color: Color::srgb(0.30, 0.05, 0.38), perceptual_roughness: 0.85, ..default() });
    let hat    = materials.add(StandardMaterial { base_color: Color::srgb(0.05, 0.03, 0.09), perceptual_roughness: 1.0, ..default() });
    let skin   = materials.add(StandardMaterial { base_color: Color::srgb(0.93, 0.76, 0.66), perceptual_roughness: 0.7, ..default() });
    let hair   = materials.add(StandardMaterial { base_color: Color::srgb(0.07, 0.04, 0.10), perceptual_roughness: 0.6, ..default() });
    let accent = materials.add(StandardMaterial { base_color: Color::srgb(0.85, 0.10, 0.55), emissive: LinearRgba::new(0.5, 0.0, 0.3, 1.0), perceptual_roughness: 0.6, ..default() });
    let steel      = materials.add(StandardMaterial { base_color: Color::srgb(0.34, 0.36, 0.42), metallic: 0.6, perceptual_roughness: 0.4, ..default() });
    let steel_dark = materials.add(StandardMaterial { base_color: Color::srgb(0.17, 0.18, 0.23), metallic: 0.6, perceptual_roughness: 0.5, ..default() });
    let bat_mat = materials.add(StandardMaterial { base_color: Color::srgb(0.10, 0.08, 0.13), perceptual_roughness: 1.0, ..default() });
    let eye_red = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.0, 0.0), emissive: LinearRgba::new(5.0, 0.0, 0.0, 1.0), unlit: true, ..default() });
    // Shared shock aura (blue/white additive) for all enemy types
    let aura_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.5, 0.8, 1.0, 0.0),
        emissive: LinearRgba::new(1.5, 3.0, 6.0, 1.0),
        unlit: true, alpha_mode: AlphaMode::Add, ..default()
    });
    let aura_tall  = meshes.add(Sphere::new(0.85)); // witch / knight
    let aura_small = meshes.add(Sphere::new(0.55)); // bat

    // ── Witches (fast chasers) — shapely sorceress figure ──
    let w_skirt  = meshes.add(Cone { radius: 0.58, height: 1.15 }.mesh().resolution(9)); // flared dress
    let w_waist  = meshes.add(Cylinder { radius: 0.17, half_height: 0.13 });
    let w_bodice = meshes.add(Cylinder { radius: 0.215, half_height: 0.24 });
    let w_bust   = meshes.add(Sphere::new(0.155));
    let w_neck   = meshes.add(Cylinder { radius: 0.05, half_height: 0.06 });
    let w_head   = meshes.add(Sphere::new(0.155));
    let w_arm    = meshes.add(Cylinder { radius: 0.045, half_height: 0.26 });
    let w_belt   = meshes.add(Cylinder { radius: 0.20, half_height: 0.035 });
    let w_hairbk = meshes.add(Cuboid::new(0.30, 0.55, 0.13));
    let w_hairsd = meshes.add(Cuboid::new(0.075, 0.42, 0.11));
    let w_brim   = meshes.add(Cylinder { radius: 0.36, half_height: 0.03 });
    let w_hat    = meshes.add(Cone { radius: 0.26, height: 0.8 }.mesh().resolution(9));
    let w_band   = meshes.add(Cylinder { radius: 0.27, half_height: 0.05 });

    let witch_pos = [
        Vec3::new( 30.0, 0.0, -20.0), Vec3::new(-40.0, 0.0, -60.0),
        Vec3::new( 70.0, 0.0,  40.0), Vec3::new(-90.0, 0.0,  30.0),
        Vec3::new(130.0, 0.0, -50.0), Vec3::new(-130.0,0.0, -20.0),
        Vec3::new( 50.0, 0.0, 120.0), Vec3::new(-60.0, 0.0, 150.0),
        Vec3::new(170.0, 0.0,  90.0), Vec3::new(-180.0,0.0, 110.0),
        Vec3::new( 20.0, 0.0, -130.0),Vec3::new(-30.0, 0.0, 200.0),
    ];
    let hat_tilt = Quat::from_rotation_x(0.18);
    for p in witch_pos {
        commands.spawn((
            Transform::from_translation(p), GlobalTransform::default(), Visibility::default(),
            Enemy { health: 4.0, speed: 7.0, flying: false, base_y: 0.0, attack_timer: 1.3, knockback_vel: Vec3::ZERO, bob_phase: 0.0 },
            Shock { timer: 0.0 },
        )).with_children(|c| {
            c.spawn((Mesh3d(aura_tall.clone()), MeshMaterial3d(aura_mat.clone()),
                Transform::from_xyz(0.0, 1.2, 0.0), Visibility::Hidden, ShockAura));
            // Flared dress + cinched waist + fitted bodice (hourglass)
            c.spawn((Mesh3d(w_skirt.clone()),  MeshMaterial3d(robe.clone()),   Transform::from_xyz(0.0, 0.58, 0.0)));
            c.spawn((Mesh3d(w_waist.clone()),  MeshMaterial3d(robe.clone()),   Transform::from_xyz(0.0, 1.16, 0.0)));
            c.spawn((Mesh3d(w_belt.clone()),   MeshMaterial3d(accent.clone()), Transform::from_xyz(0.0, 1.16, 0.0)));
            c.spawn((Mesh3d(w_bodice.clone()), MeshMaterial3d(robe.clone()),   Transform::from_xyz(0.0, 1.48, 0.0)));
            // Subtle curves
            c.spawn((Mesh3d(w_bust.clone()),   MeshMaterial3d(robe.clone()),   Transform::from_xyz(-0.09, 1.54, -0.12)));
            c.spawn((Mesh3d(w_bust.clone()),   MeshMaterial3d(robe.clone()),   Transform::from_xyz( 0.09, 1.54, -0.12)));
            // Arms
            c.spawn((Mesh3d(w_arm.clone()),    MeshMaterial3d(robe.clone()),   Transform::from_xyz(-0.27, 1.40, 0.0).with_rotation(Quat::from_rotation_z( 0.25))));
            c.spawn((Mesh3d(w_arm.clone()),    MeshMaterial3d(robe.clone()),   Transform::from_xyz( 0.27, 1.40, 0.0).with_rotation(Quat::from_rotation_z(-0.25))));
            // Neck + head
            c.spawn((Mesh3d(w_neck.clone()),   MeshMaterial3d(skin.clone()),   Transform::from_xyz(0.0, 1.74, 0.0)));
            c.spawn((Mesh3d(w_head.clone()),   MeshMaterial3d(skin.clone()),   Transform::from_xyz(0.0, 1.90, 0.0)));
            // Long flowing hair
            c.spawn((Mesh3d(w_hairbk.clone()), MeshMaterial3d(hair.clone()),   Transform::from_xyz(0.0, 1.70, 0.10)));
            c.spawn((Mesh3d(w_hairsd.clone()), MeshMaterial3d(hair.clone()),   Transform::from_xyz(-0.15, 1.82, 0.02)));
            c.spawn((Mesh3d(w_hairsd.clone()), MeshMaterial3d(hair.clone()),   Transform::from_xyz( 0.15, 1.82, 0.02)));
            // Witch hat (tilted, with accent band)
            c.spawn((Mesh3d(w_brim.clone()),   MeshMaterial3d(hat.clone()),    Transform::from_xyz(0.0, 2.04, -0.02).with_rotation(hat_tilt)));
            c.spawn((Mesh3d(w_band.clone()),   MeshMaterial3d(accent.clone()), Transform::from_xyz(0.0, 2.10, -0.02).with_rotation(hat_tilt)));
            c.spawn((Mesh3d(w_hat.clone()),    MeshMaterial3d(hat.clone()),    Transform::from_xyz(0.0, 2.45, -0.06).with_rotation(hat_tilt)));
        });
    }

    // ── Knights (tanky, medium speed) ──
    let k_torso = meshes.add(Cuboid::new(0.38, 0.56, 0.24));
    let k_pelv  = meshes.add(Cuboid::new(0.32, 0.18, 0.20));
    let k_leg   = meshes.add(Cuboid::new(0.14, 0.52, 0.14));
    let k_arm   = meshes.add(Cuboid::new(0.14, 0.40, 0.14));
    let k_head  = meshes.add(Cuboid::new(0.24, 0.24, 0.24));
    let k_crest = meshes.add(Cuboid::new(0.07, 0.20, 0.28));
    let k_sword = meshes.add(Cuboid::new(0.06, 1.0, 0.12));
    for p in [Vec3::new(-20.0,0.0,-40.0), Vec3::new(50.0,0.0,-50.0), Vec3::new(20.0,0.0,60.0), Vec3::new(-60.0,0.0,-90.0),
              Vec3::new(120.0,0.0,40.0), Vec3::new(-140.0,0.0,60.0), Vec3::new(160.0,0.0,-60.0), Vec3::new(-90.0,0.0,170.0)] {
        commands.spawn((
            Transform::from_translation(p), GlobalTransform::default(), Visibility::default(),
            Enemy { health: 7.0, speed: 3.2, flying: false, base_y: 0.0, attack_timer: 1.6, knockback_vel: Vec3::ZERO, bob_phase: 0.0 },
            Shock { timer: 0.0 },
        )).with_children(|c| {
            c.spawn((Mesh3d(aura_tall.clone()), MeshMaterial3d(aura_mat.clone()),
                Transform::from_xyz(0.0, 1.1, 0.0), Visibility::Hidden, ShockAura));
            c.spawn((Mesh3d(k_torso.clone()), MeshMaterial3d(steel.clone()),      Transform::from_xyz(0.0, 1.18, 0.0)));
            c.spawn((Mesh3d(k_pelv.clone()),  MeshMaterial3d(steel_dark.clone()), Transform::from_xyz(0.0, 0.80, 0.0)));
            c.spawn((Mesh3d(k_leg.clone()),   MeshMaterial3d(steel_dark.clone()), Transform::from_xyz(-0.11, 0.40, 0.0)));
            c.spawn((Mesh3d(k_leg.clone()),   MeshMaterial3d(steel_dark.clone()), Transform::from_xyz( 0.11, 0.40, 0.0)));
            c.spawn((Mesh3d(k_arm.clone()),   MeshMaterial3d(steel.clone()),      Transform::from_xyz(-0.28, 1.18, 0.0)));
            c.spawn((Mesh3d(k_arm.clone()),   MeshMaterial3d(steel.clone()),      Transform::from_xyz( 0.28, 1.18, 0.0)));
            c.spawn((Mesh3d(k_head.clone()),  MeshMaterial3d(steel.clone()),      Transform::from_xyz(0.0, 1.62, 0.0)));
            c.spawn((Mesh3d(k_crest.clone()), MeshMaterial3d(steel_dark.clone()), Transform::from_xyz(0.0, 1.80, 0.0)));
            c.spawn((Mesh3d(k_sword.clone()), MeshMaterial3d(steel.clone()),      Transform::from_xyz(0.36, 1.05, 0.10)));
        });
    }

    // ── Bats (fast flyers) ──
    let b_body = meshes.add(Cuboid::new(0.30, 0.22, 0.46));
    let b_head = meshes.add(Cuboid::new(0.18, 0.18, 0.18));
    let b_ear  = meshes.add(Cuboid::new(0.06, 0.12, 0.04));
    let b_wing = meshes.add(Cuboid::new(0.75, 0.04, 0.42));
    let b_eye  = meshes.add(Cuboid::new(0.05, 0.05, 0.04));
    // Swarms: 8 group centers scattered across the map, 3 bats per group
    let swarm_centers = [
        Vec3::new( 40.0, 5.0, -40.0), Vec3::new(-70.0, 6.0, -30.0),
        Vec3::new(110.0, 5.0,  60.0), Vec3::new(-120.0, 6.5, 80.0),
        Vec3::new( 180.0, 5.5, -90.0), Vec3::new(-160.0, 5.0, -140.0),
        Vec3::new( 90.0, 6.0, 180.0), Vec3::new(-50.0, 5.5, 200.0),
    ];
    let offsets = [Vec3::new(-3.0, 0.5, -2.0), Vec3::new(3.5, -0.5, 1.5), Vec3::new(0.0, 1.0, 3.0)];
    let mut bi = 0u32;
    for center in swarm_centers {
        for off in offsets {
            let p = center + off;
            bi += 1;
            commands.spawn((
                Transform::from_translation(p), GlobalTransform::default(), Visibility::default(),
                Enemy { health: 2.0, speed: 7.5, flying: true, base_y: p.y, attack_timer: 1.0, knockback_vel: Vec3::ZERO, bob_phase: bi as f32 * 0.7 },
                Shock { timer: 0.0 },
            )).with_children(|c| {
                c.spawn((Mesh3d(aura_small.clone()), MeshMaterial3d(aura_mat.clone()),
                    Transform::from_xyz(0.0, 0.0, 0.0), Visibility::Hidden, ShockAura));
                c.spawn((Mesh3d(b_body.clone()), MeshMaterial3d(bat_mat.clone()), Transform::from_xyz(0.0, 0.0, 0.0)));
                c.spawn((Mesh3d(b_head.clone()), MeshMaterial3d(bat_mat.clone()), Transform::from_xyz(0.0, 0.04, -0.30)));
                c.spawn((Mesh3d(b_ear.clone()),  MeshMaterial3d(bat_mat.clone()), Transform::from_xyz(-0.06, 0.16, -0.30)));
                c.spawn((Mesh3d(b_ear.clone()),  MeshMaterial3d(bat_mat.clone()), Transform::from_xyz( 0.06, 0.16, -0.30)));
                c.spawn((Mesh3d(b_wing.clone()), MeshMaterial3d(bat_mat.clone()), Transform::from_xyz(-0.52, 0.02, 0.0).with_rotation(Quat::from_rotation_z(0.25))));
                c.spawn((Mesh3d(b_wing.clone()), MeshMaterial3d(bat_mat.clone()), Transform::from_xyz( 0.52, 0.02, 0.0).with_rotation(Quat::from_rotation_z(-0.25))));
                c.spawn((Mesh3d(b_eye.clone()),  MeshMaterial3d(eye_red.clone()), Transform::from_xyz(-0.05, 0.05, -0.39)));
                c.spawn((Mesh3d(b_eye.clone()),  MeshMaterial3d(eye_red.clone()), Transform::from_xyz( 0.05, 0.05, -0.39)));
            });
        }
    }
}

fn enemy_ai(
    time: Res<Time>,
    mut enemy_q: Query<(&mut Transform, &mut Enemy)>,
    player_q: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut player_vel_q: Query<&mut PlayerVelocity, With<Player>>,
    mut health: ResMut<PlayerHealth>,
) {
    let pt = player_q.single();
    let pp = pt.translation;
    let dt = time.delta_secs();

    for (mut t, mut e) in enemy_q.iter_mut() {
        // Knockback decay
        if e.knockback_vel.length_squared() > 0.01 {
            t.translation += e.knockback_vel * dt;
            e.knockback_vel *= (1.0 - 8.0 * dt).max(0.0);
        }

        let flat = Vec3::new(pp.x - t.translation.x, 0.0, pp.z - t.translation.z);
        let dist = flat.length();
        e.bob_phase += dt * 6.0;

        if dist < 75.0 && dist > 0.01 {
            let dir = flat / dist;
            let stop = if e.flying { 1.2 } else { 1.8 };
            if dist > stop {
                t.translation += dir * e.speed * dt;
            }
            let ty = t.translation.y;
            t.look_at(Vec3::new(pp.x, ty, pp.z), Vec3::Y);

            let atk_range = if e.flying { 2.8 } else { 2.2 };
            if dist < atk_range {
                e.attack_timer -= dt;
                if e.attack_timer <= 0.0 && health.hurt_timer <= 0.0 {
                    health.hearts = (health.hearts - 1).max(0);
                    health.hurt_timer = 0.9;
                    e.attack_timer = 1.3;
                    let mut pvel = player_vel_q.single_mut();
                    pvel.knockback = Vec3::new(dir.x, 0.0, dir.z) * 6.0;
                }
            } else {
                e.attack_timer = 0.5;
            }
        }

        // Height: bats hover + bob, ground enemies stay grounded
        if e.flying {
            t.translation.y = e.base_y + e.bob_phase.sin() * 0.4;
        } else {
            t.translation.y = 0.0;
        }
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
    // Shock aura (blue/white additive) shown when hit by lightning
    let aura_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.5, 0.8, 1.0, 0.0),
        emissive: LinearRgba::new(1.5, 3.0, 6.0, 1.0),
        unlit: true, alpha_mode: AlphaMode::Add, ..default()
    });
    let aura_mesh = meshes.add(Sphere::new(0.75));
    // Pre-build shared mesh handles for a detailed skeleton
    let skull   = meshes.add(Cuboid::new(0.24, 0.20, 0.24));
    let jaw     = meshes.add(Cuboid::new(0.20, 0.06, 0.18));
    let brow    = meshes.add(Cuboid::new(0.26, 0.05, 0.12));
    let neck    = meshes.add(Cuboid::new(0.08, 0.10, 0.08));
    let collar  = meshes.add(Cuboid::new(0.44, 0.07, 0.11));
    let spine   = meshes.add(Cuboid::new(0.06, 0.46, 0.06));
    let rib1    = meshes.add(Cuboid::new(0.32, 0.05, 0.16));
    let rib2    = meshes.add(Cuboid::new(0.29, 0.05, 0.15));
    let rib3    = meshes.add(Cuboid::new(0.24, 0.05, 0.14));
    let sternum = meshes.add(Cuboid::new(0.05, 0.34, 0.06));
    let pelvis  = meshes.add(Cuboid::new(0.26, 0.16, 0.12));
    let thigh   = meshes.add(Cuboid::new(0.10, 0.36, 0.10));
    let shin    = meshes.add(Cuboid::new(0.09, 0.34, 0.09));
    let foot    = meshes.add(Cuboid::new(0.10, 0.07, 0.22));
    let shoulder= meshes.add(Sphere::new(0.075));
    let uarm    = meshes.add(Cuboid::new(0.09, 0.30, 0.09));
    let farm    = meshes.add(Cuboid::new(0.08, 0.27, 0.08));
    let hand    = meshes.add(Cuboid::new(0.08, 0.09, 0.08));
    let shaft   = meshes.add(Cuboid::new(0.038, 1.9, 0.038));
    let tip     = meshes.add(Cone { radius: 0.06, height: 0.24 }.mesh().resolution(5));

    // Scatter across the much larger map
    let positions = [
        Vec3::new(  8.0, 0.0, -22.0), Vec3::new(-14.0, 0.0, -34.0),
        Vec3::new( 24.0, 0.0, -50.0), Vec3::new(-28.0, 0.0, -62.0),
        Vec3::new( 60.0, 0.0, -18.0), Vec3::new(-72.0, 0.0,  20.0),
        Vec3::new(100.0, 0.0,  70.0), Vec3::new(-110.0,0.0, -40.0),
        Vec3::new(150.0, 0.0, -110.0),Vec3::new(-150.0,0.0, 120.0),
        Vec3::new( 40.0, 0.0, 160.0), Vec3::new(-60.0, 0.0, 190.0),
        Vec3::new(200.0, 0.0,  30.0), Vec3::new(-190.0,0.0, -90.0),
        Vec3::new( 18.0, 0.0, -150.0),Vec3::new(-30.0, 0.0, -170.0),
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
            Shock { timer: 0.0 },
        )).with_children(|p| {
            // Skull + jaw + brow
            p.spawn((Mesh3d(skull.clone()), MeshMaterial3d(b.clone()), Transform::from_xyz(0.0, 1.74, 0.0)));
            p.spawn((Mesh3d(jaw.clone()),   MeshMaterial3d(b.clone()), Transform::from_xyz(0.0, 1.62, 0.02)));
            p.spawn((Mesh3d(brow.clone()),  MeshMaterial3d(b.clone()), Transform::from_xyz(0.0, 1.82, -0.07)));
            // Neck + collar
            p.spawn((Mesh3d(neck.clone()),   MeshMaterial3d(b.clone()), Transform::from_xyz(0.0, 1.52, 0.0)));
            p.spawn((Mesh3d(collar.clone()), MeshMaterial3d(b.clone()), Transform::from_xyz(0.0, 1.44, 0.0)));
            // Spine + ribcage + sternum
            p.spawn((Mesh3d(spine.clone()),   MeshMaterial3d(b.clone()), Transform::from_xyz(0.0, 1.15, 0.04)));
            p.spawn((Mesh3d(rib1.clone()),    MeshMaterial3d(b.clone()), Transform::from_xyz(0.0, 1.30, 0.0)));
            p.spawn((Mesh3d(rib2.clone()),    MeshMaterial3d(b.clone()), Transform::from_xyz(0.0, 1.16, 0.0)));
            p.spawn((Mesh3d(rib3.clone()),    MeshMaterial3d(b.clone()), Transform::from_xyz(0.0, 1.02, 0.0)));
            p.spawn((Mesh3d(sternum.clone()), MeshMaterial3d(b.clone()), Transform::from_xyz(0.0, 1.16, -0.07)));
            // Pelvis
            p.spawn((Mesh3d(pelvis.clone()), MeshMaterial3d(b.clone()), Transform::from_xyz(0.0, 0.80, 0.0)));
            // Legs + feet
            for sx in [-0.10f32, 0.10] {
                p.spawn((Mesh3d(thigh.clone()), MeshMaterial3d(b.clone()), Transform::from_xyz(sx, 0.54, 0.0)));
                p.spawn((Mesh3d(shin.clone()),  MeshMaterial3d(b.clone()), Transform::from_xyz(sx, 0.18, 0.0)));
                p.spawn((Mesh3d(foot.clone()),  MeshMaterial3d(b.clone()), Transform::from_xyz(sx, 0.04, -0.06)));
            }
            // Shoulders + arms + hands
            for sx in [-0.24f32, 0.24] {
                p.spawn((Mesh3d(shoulder.clone()), MeshMaterial3d(b.clone()), Transform::from_xyz(sx, 1.40, 0.0)));
                p.spawn((Mesh3d(uarm.clone()),     MeshMaterial3d(b.clone()), Transform::from_xyz(sx, 1.22, 0.0)));
                p.spawn((Mesh3d(farm.clone()),     MeshMaterial3d(b.clone()), Transform::from_xyz(sx * 1.08, 0.93, 0.0)));
                p.spawn((Mesh3d(hand.clone()),     MeshMaterial3d(b.clone()), Transform::from_xyz(sx * 1.12, 0.76, 0.0)));
            }
            // Spear as an animatable sub-entity (shaft + tip), held in the right hand
            p.spawn((
                Transform::from_xyz(0.30, 0.95, 0.0),
                GlobalTransform::default(), Visibility::default(),
                SkeletonSpear,
            )).with_children(|s| {
                s.spawn((Mesh3d(shaft.clone()), MeshMaterial3d(b.clone()), Transform::from_xyz(0.0, 0.0, 0.0)));
                s.spawn((Mesh3d(tip.clone()),   MeshMaterial3d(b.clone()), Transform::from_xyz(0.0, 1.0, 0.0)));
            });
            // Shock aura (hidden until lightning hits)
            p.spawn((Mesh3d(aura_mesh.clone()), MeshMaterial3d(aura_mat.clone()),
                Transform::from_xyz(0.0, 1.0, 0.0), Visibility::Hidden, ShockAura));
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
    mut enemy_q: Query<(Entity, &GlobalTransform, &mut Enemy)>,
    mut shock_q: Query<&mut Shock>,
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
            if let Ok(mut s) = shock_q.get_mut(entity) { s.timer = 0.12; }
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
            if let Ok(mut s) = shock_q.get_mut(entity) { s.timer = 0.12; }
            if drag.health <= 0.0 { drag.state = DragonState::Dead; commands.entity(entity).despawn_recursive(); }
        }
    }
    // Witch / Knight / Bat
    for (entity, egt, mut en) in enemy_q.iter_mut() {
        let to = egt.translation() + Vec3::Y - pos;
        let dist = to.length();
        if dist < 0.5 { continue; }
        if dist < 9.0 && fwd.dot(to / dist) > 0.5 {
            en.health -= 2.0 * dt;
            if let Ok(mut s) = shock_q.get_mut(entity) { s.timer = 0.12; }
            if en.health <= 0.0 { commands.entity(entity).despawn_recursive(); }
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
    mut mat_q: Query<&mut MeshMaterial3d<StandardMaterial>, Without<ShockAura>>,
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

// Skeletons couch their spear forward when they spot the player, jabbing on attack.
fn skeleton_attack_anim(
    time: Res<Time>,
    skel_q: Query<(&Skeleton, &Children)>,
    mut spear_q: Query<&mut Transform, With<SkeletonSpear>>,
) {
    let k = (time.delta_secs() * 9.0).min(1.0);
    for (sk, children) in &skel_q {
        if sk.state == SkeletonState::Dead { continue; }
        let aggressive = sk.state == SkeletonState::Chase || sk.state == SkeletonState::Attack;
        for &ch in children.iter() {
            if let Ok(mut tr) = spear_q.get_mut(ch) {
                let (tgt_rot, base_pos) = if aggressive {
                    (Quat::from_rotation_x(-1.55), Vec3::new(0.22, 1.15, -0.15)) // couched, horizontal
                } else {
                    (Quat::IDENTITY, Vec3::new(0.30, 0.95, 0.0))                  // shouldered, vertical
                };
                let thrust = if sk.state == SkeletonState::Attack {
                    (time.elapsed_secs() * 7.0).sin().max(0.0) * 0.5
                } else { 0.0 };
                tr.rotation = tr.rotation.slerp(tgt_rot, k);
                tr.translation = tr.translation.lerp(base_pos - Vec3::Z * thrust, k);
            }
        }
    }
}

// Decay shock timers and toggle each shocked enemy's blue/white aura.
fn update_shock(
    time: Res<Time>,
    mut shocked: Query<(&mut Shock, &Children)>,
    mut aura_q: Query<&mut Visibility, With<ShockAura>>,
) {
    for (mut s, children) in shocked.iter_mut() {
        s.timer = (s.timer - time.delta_secs()).max(0.0);
        let show = s.timer > 0.0;
        for &ch in children.iter() {
            if let Ok(mut vis) = aura_q.get_mut(ch) {
                *vis = if show { Visibility::Visible } else { Visibility::Hidden };
            }
        }
    }
}

fn spawn_dragon(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let scale = materials.add(StandardMaterial {
        base_color: Color::srgb(0.09, 0.27, 0.13),
        perceptual_roughness: 0.8, ..default()
    });
    let scale_dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.15, 0.09),
        perceptual_roughness: 0.85, ..default()
    });
    let horn = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.10, 0.10),
        perceptual_roughness: 0.6, ..default()
    });
    let teeth = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.90, 0.82),
        perceptual_roughness: 0.5, ..default()
    });
    let eye = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.55, 0.0),
        emissive: LinearRgba::new(11.0, 3.0, 0.0, 1.0), unlit: true, ..default()
    });
    let maw = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.35, 0.0),
        emissive: LinearRgba::new(6.0, 1.4, 0.0, 1.0), unlit: true, ..default()
    });
    let flash = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.1, 0.1),
        emissive: LinearRgba::new(2.5, 0.1, 0.1, 1.0), ..default()
    });
    let fb_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.45, 0.0),
        emissive: LinearRgba::new(4.0, 1.5, 0.0, 1.0), unlit: true, ..default()
    });
    let fb_mesh = meshes.add(Sphere::new(0.28));
    commands.insert_resource(DragonAssets {
        flash_mat: flash, fb_mat, fb_mesh,
    });

    // Build the whole dragon as a parts list (mesh, material, transform).
    // Local convention: forward = -Z (head points -Z), tail at +Z.
    let cone = |r: f32, h: f32, res: u32, m: &mut Assets<Mesh>| m.add(Cone { radius: r, height: h }.mesh().resolution(res));
    let q_x = |deg: f32| Quat::from_rotation_x(deg.to_radians());
    let mut parts: Vec<(Handle<Mesh>, Handle<StandardMaterial>, Transform)> = Vec::new();

    // Torso (chest + hindquarters)
    parts.push((meshes.add(Cuboid::new(2.6, 2.1, 2.8)), scale.clone(), Transform::from_xyz(0.0, 2.4, -0.7)));
    parts.push((meshes.add(Cuboid::new(3.0, 2.3, 3.0)), scale.clone(), Transform::from_xyz(0.0, 2.3, 1.5)));
    // Belly plates (lighter)
    parts.push((meshes.add(Cuboid::new(1.8, 0.5, 5.2)), scale_dark.clone(), Transform::from_xyz(0.0, 1.25, 0.4)));

    // Curving neck (3 segments rising toward the head)
    parts.push((meshes.add(Cuboid::new(1.35, 1.5, 1.5)), scale.clone(), Transform::from_xyz(0.0, 3.3, -2.0).with_rotation(q_x(-18.0))));
    parts.push((meshes.add(Cuboid::new(1.15, 1.3, 1.4)), scale.clone(), Transform::from_xyz(0.0, 4.0, -3.0).with_rotation(q_x(-28.0))));
    parts.push((meshes.add(Cuboid::new(1.0, 1.15, 1.3)), scale.clone(), Transform::from_xyz(0.0, 4.5, -3.9).with_rotation(q_x(-15.0))));

    // Head: skull, snout, lower jaw
    parts.push((meshes.add(Cuboid::new(1.35, 1.0, 1.7)), scale.clone(), Transform::from_xyz(0.0, 4.7, -5.0)));
    parts.push((meshes.add(Cuboid::new(0.85, 0.6, 1.1)), scale.clone(), Transform::from_xyz(0.0, 4.55, -5.95)));
    parts.push((meshes.add(Cuboid::new(0.9, 0.35, 1.4)), scale_dark.clone(), Transform::from_xyz(0.0, 4.2, -5.6)));
    // Brow ridge
    parts.push((meshes.add(Cuboid::new(1.4, 0.25, 0.7)), scale_dark.clone(), Transform::from_xyz(0.0, 5.25, -4.7)));

    // Glowing maw between the jaws + nostrils
    parts.push((meshes.add(Cuboid::new(0.7, 0.32, 1.0)), maw.clone(),
        Transform::from_xyz(0.0, 4.4, -5.7)));
    for sx in [-1.0f32, 1.0] {
        parts.push((meshes.add(Cuboid::new(0.10, 0.10, 0.10)), maw.clone(),
            Transform::from_xyz(sx * 0.22, 4.62, -6.45)));
    }

    // Horns (large swept-back pair), cheek horns, brow spikes, eyes
    for sx in [-1.0f32, 1.0] {
        parts.push((cone(0.30, 2.2, 6, &mut meshes), horn.clone(),
            Transform::from_xyz(sx * 0.6, 5.5, -4.3).with_rotation(q_x(58.0))));
        parts.push((cone(0.15, 0.9, 5, &mut meshes), horn.clone(),
            Transform::from_xyz(sx * 0.78, 5.0, -4.9).with_rotation(q_x(30.0) * Quat::from_rotation_z(sx * 0.3))));
        // cheek horn
        parts.push((cone(0.10, 0.5, 5, &mut meshes), horn.clone(),
            Transform::from_xyz(sx * 0.72, 4.3, -5.6).with_rotation(Quat::from_rotation_z(sx * 1.2))));
        // big glowing eye
        parts.push((meshes.add(Cuboid::new(0.34, 0.30, 0.20)), eye.clone(),
            Transform::from_xyz(sx * 0.52, 4.9, -5.75)));
    }
    // Teeth row (upper + lower)
    for i in 0..6u32 {
        let tx = -0.36 + i as f32 * 0.145;
        parts.push((cone(0.05, 0.24, 4, &mut meshes), teeth.clone(),
            Transform::from_xyz(tx, 4.28, -6.25).with_rotation(q_x(180.0))));
        parts.push((cone(0.045, 0.18, 4, &mut meshes), teeth.clone(),
            Transform::from_xyz(tx, 4.12, -6.05)));
    }

    // Wings: leading-edge bone + membrane panel + a couple of finger bones
    for s in [-1.0f32, 1.0] {
        let rz = Quat::from_rotation_z(s * 0.35);
        let ry = Quat::from_rotation_y(s * 0.25);
        // membrane
        parts.push((meshes.add(Cuboid::new(4.6, 0.08, 3.2)), scale_dark.clone(),
            Transform::from_xyz(s * 3.2, 3.4, 0.8).with_rotation(rz * ry)));
        // leading bone
        parts.push((meshes.add(Cuboid::new(4.8, 0.22, 0.30)), scale.clone(),
            Transform::from_xyz(s * 3.3, 3.7, -0.7).with_rotation(rz * ry)));
        // wing fingers
        for fz in [0.2f32, 1.6, 2.8] {
            parts.push((meshes.add(Cuboid::new(3.6, 0.10, 0.12)), scale.clone(),
                Transform::from_xyz(s * 2.6, 3.45, fz).with_rotation(Quat::from_rotation_z(s * 0.30))));
        }
    }

    // Four legs: thigh + shin + clawed foot
    for (lx, lz) in [(-1.15f32, -1.0f32), (1.15, -1.0), (-1.2, 1.9), (1.2, 1.9)] {
        parts.push((meshes.add(Cuboid::new(0.6, 1.1, 0.6)), scale.clone(), Transform::from_xyz(lx, 1.0, lz)));
        parts.push((meshes.add(Cuboid::new(0.45, 0.95, 0.45)), scale.clone(), Transform::from_xyz(lx, 0.4, lz - 0.12)));
        parts.push((meshes.add(Cuboid::new(0.6, 0.22, 0.95)), scale_dark.clone(), Transform::from_xyz(lx, 0.11, lz - 0.35)));
        // claws
        for cxo in [-0.16f32, 0.0, 0.16] {
            parts.push((cone(0.05, 0.22, 4, &mut meshes), teeth.clone(),
                Transform::from_xyz(lx + cxo, 0.10, lz - 0.85).with_rotation(q_x(-90.0))));
        }
    }

    // Tapering curved tail
    let tail: &[(f32, f32, f32, f32, f32)] = &[
        (1.1, 1.1, 1.6, 3.1, 2.0),
        (0.9, 0.9, 1.5, 4.4, 1.8),
        (0.7, 0.7, 1.4, 5.6, 1.6),
        (0.5, 0.5, 1.3, 6.7, 1.5),
        (0.35, 0.35, 1.2, 7.7, 1.4),
    ];
    for &(tw, th, tl, tz, ty) in tail {
        parts.push((meshes.add(Cuboid::new(tw, th, tl)), scale.clone(), Transform::from_xyz(0.0, ty, tz)));
    }
    // Tail spike
    parts.push((cone(0.28, 1.1, 6, &mut meshes), horn.clone(),
        Transform::from_xyz(0.0, 1.4, 8.6).with_rotation(q_x(-90.0))));

    // Spine spikes from neck down the back and tail
    for i in 0..14u32 {
        let z = -3.5 + i as f32 * 0.85;
        let y = 3.6 - (i as f32 * 0.16);
        parts.push((cone(0.13, 0.55, 5, &mut meshes), scale_dark.clone(),
            Transform::from_xyz(0.0, y.max(1.6), z)));
    }

    // Dragon faces toward the gate (rotate 180° so local -Z points +Z world).
    // Scaled up 1.35× for an imposing boss.
    // Placed just inside the gate (front wall ≈ z=-37) so the player meets it on entry
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, -54.0)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI))
            .with_scale(Vec3::splat(1.35)),
        GlobalTransform::default(), Visibility::default(),
        Dragon { health: 20.0, fireball_timer: 4.0, damage_flash: 0.0, state: DragonState::Idle },
        Shock { timer: 0.0 },
    )).with_children(|p| {
        for (mesh, mat, tf) in parts {
            p.spawn((
                Mesh3d(mesh), MeshMaterial3d(mat.clone()), tf,
                Visibility::default(), DragonPart { base: mat },
            ));
        }
        // Shock aura enveloping the dragon
        let aura_mat = materials.add(StandardMaterial {
            base_color: Color::srgba(0.5, 0.8, 1.0, 0.0),
            emissive: LinearRgba::new(1.2, 2.5, 5.0, 1.0),
            unlit: true, alpha_mode: AlphaMode::Add, ..default()
        });
        p.spawn((Mesh3d(meshes.add(Sphere::new(5.5))), MeshMaterial3d(aura_mat),
            Transform::from_xyz(0.0, 3.0, 1.5), Visibility::Hidden, ShockAura));
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
            let fire_pos = t.translation + Vec3::Y * 4.3 + fwd * 5.5;
            let base = to.normalize_or_zero();
            // Burst of 3 — fanned ±15° around the aim direction
            for deg in [-15.0f32, 0.0, 15.0] {
                let dir = Quat::from_rotation_y(deg.to_radians()) * base;
                commands.spawn((
                    Mesh3d(assets.fb_mesh.clone()),
                    MeshMaterial3d(assets.fb_mat.clone()),
                    Transform::from_translation(fire_pos),
                    Fireball { velocity: dir * 14.0, life: 8.0 },
                    PointLight { color: Color::srgb(1.0, 0.4, 0.0), intensity: 90_000.0,
                        range: 14.0, shadows_enabled: false, ..default() },
                ));
            }
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
    mut mat_q: Query<(&mut MeshMaterial3d<StandardMaterial>, &DragonPart)>,
) {
    let Some(a) = assets else { return; };
    for (mut d, children) in dragon_q.iter_mut() {
        if d.state == DragonState::Dead { continue; }
        d.damage_flash = (d.damage_flash - time.delta_secs()).max(0.0);
        let flashing = d.damage_flash > 0.0;
        for &child in children.iter() {
            if let Ok((mut m, part)) = mat_q.get_mut(child) {
                // Flash red, else restore the part's own material
                m.0 = if flashing { a.flash_mat.clone() } else { part.base.clone() };
            }
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
    let spin = time.delta_secs() * 9.0;
    for (entity, mut t, mut fb) in fb_q.iter_mut() {
        t.translation += fb.velocity * time.delta_secs();
        fb.life -= time.delta_secs();
        // Animate: tumble + flicker pulse
        t.rotate_local_x(spin);
        t.rotate_local_y(spin * 0.7);
        let pulse = 1.0 + (time.elapsed_secs() * 18.0 + fb.life * 5.0).sin() * 0.18;
        t.scale = Vec3::splat(pulse);
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

    // Castle layout — center (0, 0, -95), 116×116 outer footprint
    let cz    = -95.0f32;
    let hw    =  58.0f32;  // half-width
    let hd    =  58.0f32;  // half-depth
    let wh    =  22.0f32;  // wall height
    let wt    =   4.0f32;  // wall thickness
    let gate  =   9.0f32;  // gate half-width

    let front_z = cz + hd; // -46
    let back_z  = cz - hd; // -134
    let side_w  = hw - gate; // 37

    // ── Perimeter walls (solid colliders; gate left open) ────
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(side_w, wh, wt))),
        MeshMaterial3d(stone.clone()),
        Transform::from_xyz(-gate - side_w*0.5, wh*0.5, front_z),
        Collider { half: Vec2::new(side_w*0.5, wt*0.5) }));
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(side_w, wh, wt))),
        MeshMaterial3d(stone.clone()),
        Transform::from_xyz( gate + side_w*0.5, wh*0.5, front_z),
        Collider { half: Vec2::new(side_w*0.5, wt*0.5) }));
    let lintel_h = wh - 13.0;
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(gate*2.0, lintel_h, wt))),
        MeshMaterial3d(dark_stone.clone()),
        Transform::from_xyz(0.0, 13.0 + lintel_h*0.5, front_z)));
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(hw*2.0, wh, wt))),
        MeshMaterial3d(stone.clone()),
        Transform::from_xyz(0.0, wh*0.5, back_z),
        Collider { half: Vec2::new(hw, wt*0.5) }));
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(wt, wh, hd*2.0))),
        MeshMaterial3d(stone.clone()),
        Transform::from_xyz(-hw, wh*0.5, cz),
        Collider { half: Vec2::new(wt*0.5, hd) }));
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(wt, wh, hd*2.0))),
        MeshMaterial3d(stone.clone()),
        Transform::from_xyz( hw, wh*0.5, cz),
        Collider { half: Vec2::new(wt*0.5, hd) }));

    // ── Corner towers ─────────────────────────────────────────
    let tw = 10.0f32; let th = 26.0f32;
    for (tx, tz) in [(-hw, front_z),(hw, front_z),(-hw, back_z),(hw, back_z)] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(tw, th, tw))),
            MeshMaterial3d(dark_stone.clone()),
            Transform::from_xyz(tx, th*0.5, tz),
            Collider { half: Vec2::new(tw*0.5, tw*0.5) }));
    }

    // ── Keep ──────────────────────────────────────────────────
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(20.0, 34.0, 20.0))),
        MeshMaterial3d(dark_stone.clone()),
        Transform::from_xyz(0.0, 17.0, cz - 12.0),
        Collider { half: Vec2::new(10.0, 10.0) }));

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
            Transform::from_xyz(px, 8.5, pz),
            Collider { half: Vec2::new(1.75, 1.75) }));
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
                color: Color::srgb(1.0, 0.6, 0.2),
                intensity: 260_000.0,
                range: 40.0,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(p + Vec3::Y * 0.75),
        ));
    }

    // Interior braziers — extra warm fill light so the courtyard reads clearly
    let brazier = meshes.add(Cylinder { radius: 0.7, half_height: 0.5 });
    for (bx, bz) in [(-26.0f32, -70.0f32), (26.0, -70.0), (-26.0, -120.0), (26.0, -120.0), (0.0, -95.0)] {
        commands.spawn((Mesh3d(brazier.clone()), MeshMaterial3d(dark_stone.clone()),
            Transform::from_xyz(bx, 1.4, bz)));
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.9, 0.5, 0.9))), MeshMaterial3d(flame_mat.clone()),
            Transform::from_xyz(bx, 2.1, bz)));
        commands.spawn((
            PointLight {
                color: Color::srgb(1.0, 0.62, 0.22),
                intensity: 320_000.0,
                range: 48.0,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_xyz(bx, 3.0, bz),
        ));
    }

    // Two large overhead courtyard lights — flood the whole interior so it's fully lit
    for lz in [cz + 28.0, cz - 28.0] {
        commands.spawn((
            PointLight {
                color: Color::srgb(1.0, 0.7, 0.35),
                intensity: 2_500_000.0,
                range: 150.0,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_xyz(0.0, 20.0, lz),
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

// Push the player out of solid structures (XZ axis-aligned resolution).
fn resolve_collisions(
    mut player_q: Query<&mut Transform, With<Player>>,
    collider_q: Query<(&GlobalTransform, &Collider), Without<Player>>,
) {
    let mut pt = player_q.single_mut();
    let r = 0.45f32; // player radius
    for (gt, col) in &collider_q {
        let c = gt.translation();
        let hx = col.half.x + r;
        let hz = col.half.y + r;
        let dx = pt.translation.x - c.x;
        let dz = pt.translation.z - c.z;
        if dx.abs() < hx && dz.abs() < hz {
            // Eject along the axis of least penetration
            let push_x = hx - dx.abs();
            let push_z = hz - dz.abs();
            if push_x < push_z {
                pt.translation.x = c.x + if dx >= 0.0 { hx } else { -hx };
            } else {
                pt.translation.z = c.z + if dz >= 0.0 { hz } else { -hz };
            }
        }
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

// ── Eye of Sauron spire ─────────────────────────────────────────────────────
fn spawn_spire(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // Dark obsidian tower stone with pixel brick texture
    let tower_tex = make_tower_texture(&mut images);
    let obsidian = materials.add(StandardMaterial {
        base_color_texture: Some(tower_tex),
        uv_transform: bevy::math::Affine2::from_scale(Vec2::new(8.0, 8.0)),
        perceptual_roughness: 0.9, ..default()
    });
    // Glowing eye flame — warm orange baseline (turns red when alert, see animate_eye)
    let flame = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.55, 0.05),
        emissive: LinearRgba::new(8.0, 3.0, 0.2, 1.0),
        unlit: true, ..default()
    });
    commands.insert_resource(EyeAssets { flame: flame.clone() });
    let pupil_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.02, 0.0, 0.0),
        unlit: true, ..default()
    });

    // Tower placed far from the castle, off in the +X/+Z corner of the map
    let tx = 320.0f32;
    let tz = 240.0f32;

    // Stacked tapering tiers (base → crown). (half-width, height, center-y)
    let tiers: &[(f32, f32, f32)] = &[
        (16.0, 30.0, 15.0),
        (12.5, 28.0, 44.0),
        ( 9.5, 26.0, 71.0),
        ( 7.0, 22.0, 95.0),
        ( 9.0,  5.0, 108.5), // flared crown
    ];
    for &(hw, h, cy) in tiers {
        let mut e = commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(hw * 2.0, h, hw * 2.0))),
            MeshMaterial3d(obsidian.clone()),
            Transform::from_xyz(tx, cy, tz),
        ));
        // Only the lower tiers (reachable on foot) need collision
        if cy < 50.0 {
            e.insert(Collider { half: Vec2::new(hw, hw) });
        }
    }

    // Jagged buttress spikes splayed outward to frame the eye (Barad-dûr silhouette)
    let spike_mesh = meshes.add(Cuboid::new(1.6, 22.0, 1.6));
    let splay = 0.42f32; // outward lean
    for i in 0..8u32 {
        let a = (i as f32 / 8.0) * std::f32::consts::TAU;
        let r = 8.0;
        commands.spawn((
            Mesh3d(spike_mesh.clone()),
            MeshMaterial3d(obsidian.clone()),
            Transform::from_xyz(tx + a.cos() * r, 119.0, tz + a.sin() * r)
                .with_rotation(Quat::from_rotation_z(-a.cos() * splay)
                             * Quat::from_rotation_x(a.sin() * splay)),
        ));
    }

    // ── Eye assembly (child-rotated by animate_eye to scan the horizon) ──
    // Raised well above the splayed spikes so it sits clear in the air
    let eye_root = commands.spawn((
        Transform::from_xyz(tx, 140.0, tz),
        GlobalTransform::default(),
        Visibility::default(),
        Eye { alert: 0.0 },
    )).id();

    // Eyeball — large flattened glowing ellipsoid, faces +Z
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(flame.clone()),
        Transform::from_xyz(0.0, 0.0, 7.0).with_scale(Vec3::new(7.5, 12.0, 2.0)),
        Visibility::default(),
    )).set_parent(eye_root);

    // Vertical slit pupil — much bigger iris
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.6, 10.5, 1.0))),
        MeshMaterial3d(pupil_mat),
        Transform::from_xyz(0.0, 0.0, 9.2),
        Visibility::default(),
        EyePupil,
    )).set_parent(eye_root);

    // Flame wreath — spikes radiating around the larger eye
    let wreath = meshes.add(Cuboid::new(1.0, 5.5, 1.0));
    for i in 0..16u32 {
        let a = (i as f32 / 16.0) * std::f32::consts::TAU;
        commands.spawn((
            Mesh3d(wreath.clone()),
            MeshMaterial3d(flame.clone()),
            Transform::from_xyz(a.cos() * 9.0, a.sin() * 13.5, 6.5)
                .with_rotation(Quat::from_rotation_z(a + std::f32::consts::FRAC_PI_2)),
            Visibility::default(),
        )).set_parent(eye_root);
    }

    // Orange beacon light at the eye
    commands.spawn((
        PointLight {
            color: Color::srgb(1.0, 0.45, 0.05),
            intensity: 800_000.0,
            range: 160.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 12.0),
    )).set_parent(eye_root);

    // Pre-spawn the continuous lightning-beam segments (world-space, hidden until firing).
    // Thin glowing red, unit-length on Z so eye_beam can stretch/orient each link.
    let beam_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.2, 0.0),
        emissive: LinearRgba::new(14.0, 2.0, 0.0, 1.0),
        unlit: true, ..default()
    });
    let beam_mesh = meshes.add(Cuboid::new(0.22, 0.22, 1.0));
    for idx in 0..14u32 {
        commands.spawn((
            Mesh3d(beam_mesh.clone()),
            MeshMaterial3d(beam_mat.clone()),
            Transform::from_xyz(0.0, 2.0, 0.0),
            Visibility::Hidden,
            EyeBeamSeg { idx },
        ));
    }
}

fn animate_eye(
    time: Res<Time>,
    eye_assets: Option<Res<EyeAssets>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_q: Query<&Transform, (With<Player>, Without<Eye>, Without<EyePupil>)>,
    mut eye_q: Query<(&mut Transform, &mut Eye), (Without<EyePupil>, Without<Player>)>,
    mut pupil_q: Query<&mut Transform, (With<EyePupil>, Without<Eye>, Without<Player>)>,
) {
    let t = time.elapsed_secs();
    let dt = time.delta_secs();
    let pp = player_q.single().translation + Vec3::Y * 1.0;

    let Ok((mut tr, mut eye)) = eye_q.get_single_mut() else { return; };
    let eye_pos = tr.translation;
    let to = pp - eye_pos;
    let horiz = Vec3::new(to.x, 0.0, to.z).length();
    let detected = horiz < 220.0;

    // Ramp the alert level up when detected, down otherwise
    let target = if detected { 1.0 } else { 0.0 };
    eye.alert = (eye.alert + (target - eye.alert) * (dt * 3.0)).clamp(0.0, 1.0);

    // Idle sweep when calm, lock onto the player when alert
    let idle = Quat::from_rotation_y((t * 0.4).sin() * 0.7);
    if to.length_squared() > 0.001 {
        let dir = to.normalize();
        // +Z (eye face) points toward player → aim -Z away from player
        let track = Transform::from_translation(eye_pos).looking_to(-dir, Vec3::Y).rotation;
        tr.rotation = idle.slerp(track, eye.alert);
    } else {
        tr.rotation = idle;
    }

    // Glow shifts orange → burning red as alert ramps up
    if let Some(ea) = eye_assets {
        if let Some(mat) = materials.get_mut(&ea.flame) {
            let flicker = (t * 11.0).sin() * 0.5 + (t * 3.7).sin() * 0.3;
            let a = eye.alert;
            // Emissive: orange (8,3,0.2) → red (16,0.8,0) + flicker when alert
            let er = 8.0 + a * (8.0 + flicker);
            let eg = 3.0 - a * 2.2;
            mat.emissive = LinearRgba::new(er, eg, 0.15, 1.0);
            // Base color: orange → red
            mat.base_color = Color::srgb(1.0, 0.55 - a * 0.45, 0.05 - a * 0.05);
        }
    }

    // Pupil flicker — dilates harder when alert
    for mut ptr in pupil_q.iter_mut() {
        let flick = 1.0 + (t * 7.0).sin() * 0.12 + (t * 2.3).sin() * 0.08;
        ptr.scale = Vec3::new(flick + eye.alert * 0.4, 1.0, 1.0);
    }
}

fn check_death(
    health: Res<PlayerHealth>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
) {
    if *state.get() == AppState::Playing && health.hearts <= 0 {
        next.set(AppState::Dead);
    }
}

fn spawn_death_screen(mut commands: Commands, mut windows: Query<&mut Window>) {
    // Free the cursor so the player can click the button
    let mut window = windows.single_mut();
    window.cursor_options.grab_mode = CursorGrabMode::None;
    window.cursor_options.visible = true;

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0), height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(36.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.86)),
        DeathScreen,
    )).with_children(|p| {
        p.spawn((
            Text::new("YOU'RE DEAD"),
            TextFont { font_size: 96.0, ..default() },
            TextColor(Color::srgb(0.80, 0.05, 0.05)),
        ));
        p.spawn((
            Button,
            Node {
                width: Val::Px(260.0), height: Val::Px(72.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(3.0)),
                ..default()
            },
            BorderColor(Color::srgb(0.9, 0.3, 0.3)),
            BackgroundColor(Color::srgb(0.45, 0.05, 0.05)),
            RespawnButton,
        )).with_children(|b| {
            b.spawn((
                Text::new("RESPAWN"),
                TextFont { font_size: 36.0, ..default() },
                TextColor(Color::WHITE),
            ));
        });
    });
}

fn despawn_death_screen(mut commands: Commands, q: Query<Entity, With<DeathScreen>>) {
    for e in q.iter() { commands.entity(e).despawn_recursive(); }
}

fn death_button(
    interaction: Query<&Interaction, (Changed<Interaction>, With<RespawnButton>)>,
    mut next: ResMut<NextState<AppState>>,
) {
    for i in interaction.iter() {
        if *i == Interaction::Pressed { next.set(AppState::Playing); }
    }
}

fn reset_game(
    mut health: ResMut<PlayerHealth>,
    mut player_q: Query<(&mut Transform, &mut PlayerVelocity), With<Player>>,
    mut commands: Commands,
    enemies: Query<Entity, Or<(With<Skeleton>, With<Dragon>, With<Fireball>, With<Enemy>)>>,
) {
    health.hearts = 5;
    health.hurt_timer = 0.0;
    let (mut t, mut v) = player_q.single_mut();
    *t = Transform::from_xyz(0.0, 0.0, 10.0);
    v.vertical = 0.0;
    v.knockback = Vec3::ZERO;
    // Clear all enemies and live projectiles — fresh start
    for e in enemies.iter() { commands.entity(e).despawn_recursive(); }
}

// Continuous jagged lightning beam from the eye to the player while in range.
fn eye_beam(
    time: Res<Time>,
    eye_q: Query<&GlobalTransform, With<Eye>>,
    player_q: Query<&Transform, (With<Player>, Without<EyeBeamSeg>)>,
    mut seg_q: Query<(&mut Transform, &mut Visibility, &EyeBeamSeg)>,
    mut player_vel: Query<&mut PlayerVelocity, With<Player>>,
    mut health: ResMut<PlayerHealth>,
) {
    let Ok(eye_gt) = eye_q.get_single() else { return; };
    let pt = player_q.single();
    let target = pt.translation + Vec3::Y * 1.0;

    let center = eye_gt.translation();
    let aim = (target - center).normalize_or_zero();
    let eye_pos = center + aim * 12.0;            // beam origin just in front of the eye
    let horiz = Vec3::new(target.x - center.x, 0.0, target.z - center.z).length();
    let active = horiz < 220.0 && (target - eye_pos).length() > 1.0;

    if !active {
        for (_, mut vis, _) in seg_q.iter_mut() { *vis = Visibility::Hidden; }
        return;
    }

    // Perpendicular basis for sideways jitter
    let dir = (target - eye_pos).normalize_or_zero();
    let mut perp1 = dir.cross(Vec3::Y);
    if perp1.length_squared() < 1e-4 { perp1 = dir.cross(Vec3::X); }
    perp1 = perp1.normalize();
    let perp2 = dir.cross(perp1).normalize();

    let n = 14u32;
    let tt = (time.elapsed_secs() * 26.0).floor() / 26.0; // crackle snap
    // One continuous thin jagged path (anchored at both ends)
    let point = |k: u32| -> Vec3 {
        let f = k as f32 / n as f32;
        let along = eye_pos.lerp(target, f);
        if k == 0 || k == n { return along; }
        let s = k as f32 * 12.9 + tt * 47.0;
        let amp = 1.3;
        along + perp1 * (s.sin() * amp) + perp2 * ((s * 1.7 + 0.6).cos() * amp)
    };

    for (mut tr, mut vis, seg) in seg_q.iter_mut() {
        if seg.idx >= n { *vis = Visibility::Hidden; continue; }
        *vis = Visibility::Visible;
        let p0 = point(seg.idx);
        let p1 = point(seg.idx + 1);
        let mid = (p0 + p1) * 0.5;
        let delta = p1 - p0;
        let len = delta.length().max(0.001);
        tr.translation = mid;
        tr.rotation = Quat::from_rotation_arc(Vec3::Z, delta / len);
        tr.scale = Vec3::new(1.0, 1.0, len);
    }

    // Continuous damage tick while the beam connects
    if health.hurt_timer <= 0.0 {
        health.hearts = (health.hearts - 1).max(0);
        health.hurt_timer = 0.9;
        let mut pvel = player_vel.single_mut();
        let knock = Vec3::new(target.x - center.x, 0.0, target.z - center.z).normalize_or_zero();
        pvel.knockback = knock * 7.0;
    }
}
