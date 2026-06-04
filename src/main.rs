use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::window::CursorGrabMode;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::image::{ImageSampler, ImageSamplerDescriptor, ImageAddressMode, ImageFilterMode};

#[derive(Component)]
struct Player;

#[derive(Component)]
struct PlayerVelocity { vertical: f32, knockback: Vec3, roll_timer: f32, roll_dir: Vec3 }

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
    anim_phase: f32,
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
    anim_phase: f32,
    attack_anim: f32,
    moving: bool,
    damage_flash: f32,
}

#[derive(Component)]
struct EnemyLimb { is_arm: bool, side: f32 }

#[derive(Component)]
struct OrcBrute { slammed: bool }

#[derive(Component)]
struct Debris { vel: Vec3, life: f32 }

#[derive(Component)]
struct Bonfire;

#[derive(Component)]
struct FogDrift { base: Vec3, phase: f32 }

#[derive(Component)]
struct FloatMote { base: Vec3, phase: f32 }

#[derive(Component)]
struct BonfireFlame;

/// Stores a mesh part's original material so it can be restored after a flash.
#[derive(Component)]
struct BodyPart { base: Handle<StandardMaterial> }

#[derive(Resource)]
struct FlashMats {
    red: Handle<StandardMaterial>,
    white: Handle<StandardMaterial>,
    blue: Handle<StandardMaterial>,
}

#[derive(Component)]
struct HeartNode { index: u32 }

#[derive(Component)]
struct DamageVignette;

#[derive(Component)]
struct StaminaBar;

#[derive(Component)]
struct ManaBar;

#[derive(Resource)]
struct PlayerHealth { hearts: i32, hurt_timer: f32, iframes: f32 }



#[derive(Component, PartialEq, Clone, Copy)]
enum DragonState { Idle, Ground, Roar, Takeoff, Fly, Breath, Dead }

#[derive(Component)]
struct Dragon {
    health: f32,
    max_health: f32,
    damage_flash: f32,
    state: DragonState,
    enraged: bool,
    timer: f32,         // current sub-phase timer
    shock_left: u32,    // shockwaves remaining to emit during the roar
    shock_timer: f32,   // time until next shockwave
    fireball_timer: f32,
    fly_angle: f32,
    breath_timer: f32,  // countdown to the next dragonbreath
    breath_target: Vec3, // fixed aim point of the current breath (so it's dodgeable)
    fire_timer: f32,    // throttle for laying ground-fire during the breath
}

#[derive(Component)]
struct FirePatch { life: f32 }

#[derive(Component)]
struct DragonLaser;          // the dragonbreath beam entity

#[derive(Component)]
struct DragonWing { side: f32 }

#[derive(Component)]
struct Shockwave { radius: f32, origin: Vec3, hit: bool }

#[derive(Component)]
struct EnrageAura;

#[derive(Resource)]
struct MoveSlow { timer: f32 }

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
struct SkeletonSpear;

#[derive(Component)]
struct SkeletonLimb { is_arm: bool, side: f32 }

#[derive(Component)]
struct ArtifactSpin { dir: f32 }

#[derive(Component)]
struct WitchCaster { cast_timer: f32 }

#[derive(Component)]
struct MagicMissile { velocity: Vec3, life: f32 }

#[derive(Component)]
struct Rocket { velocity: Vec3, life: f32 }

/// Short-lived visual (tracer beam, mushroom cloud) that despawns after `life`.
#[derive(Component)]
struct Transient { life: f32 }

/// Growing nuke mushroom cloud.
#[derive(Component)]
struct Mushroom { age: f32 }

#[derive(Component)]
struct Pickup { kind: ItemKind }

#[derive(Component)]
struct HeldVisual { kind: ItemKind }

#[derive(Clone, Copy, PartialEq, Eq)]
enum ItemKind { Sword, Glock, Rocket, HealthPotion, ManaPotion }

#[derive(Resource)]
struct Stamina { current: f32, max: f32 }

#[derive(Resource)]
struct Mana { current: f32, max: f32 }

#[derive(Resource)]
struct Drinking { timer: f32 }

#[derive(Resource)]
struct GunRecoil { climb: f32 }

#[derive(Component)]
struct HotbarSlot { kind: ItemKind }

#[derive(Component)]
struct HotbarIcon { kind: ItemKind, color: Color }

#[derive(Component)]
struct HotbarCount { kind: ItemKind }

#[derive(Resource)]
struct Inventory {
    selected: ItemKind,
    health_potions: u32,
    mana_potions: u32,
    has_glock: bool,
    has_rocket: bool,
}

impl Inventory {
    /// Items currently available to cycle through (sword always first).
    fn available(&self) -> Vec<ItemKind> {
        let mut v = vec![ItemKind::Sword];
        if self.has_glock { v.push(ItemKind::Glock); }
        if self.has_rocket { v.push(ItemKind::Rocket); }
        if self.health_potions > 0 { v.push(ItemKind::HealthPotion); }
        if self.mana_potions > 0 { v.push(ItemKind::ManaPotion); }
        v
    }
    fn owns(&self, kind: ItemKind) -> bool {
        match kind {
            ItemKind::Sword => true,
            ItemKind::Glock => self.has_glock,
            ItemKind::Rocket => self.has_rocket,
            ItemKind::HealthPotion => self.health_potions > 0,
            ItemKind::ManaPotion => self.mana_potions > 0,
        }
    }
}

#[derive(Resource)]
struct EyeAssets { flame: Handle<StandardMaterial> }

#[derive(Component)]
struct DeathScreen;

#[derive(Component)]
struct RespawnButton;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum AppState { #[default] Playing, Dead, Paused }

#[derive(Component)]
struct PauseScreen;

#[derive(Component)]
struct BossBarRoot;
#[derive(Component)]
struct BossBarFill;
#[derive(Component)]
struct Ember { seed: f32 }
#[derive(Component)]
struct Soapstone { idx: usize }
#[derive(Component)]
struct SoapstoneText;
#[derive(Component)]
struct SoundWave;

#[derive(Resource)]
struct DragonAssets {
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
        .insert_resource(PlayerHealth { hearts: 5, hurt_timer: 0.0, iframes: 0.0 })
        .insert_resource(Stamina { current: 100.0, max: 100.0 })
        .insert_resource(Mana { current: 100.0, max: 100.0 })
        .insert_resource(Drinking { timer: 0.0 })
        .insert_resource(GunRecoil { climb: 0.0 })
        .insert_resource(MoveSlow { timer: 0.0 })
        .insert_resource(Inventory { selected: ItemKind::Sword, health_potions: 0, mana_potions: 0, has_glock: false, has_rocket: false })
        .init_state::<AppState>()
        .add_systems(Startup, (setup, setup_flash_mats, spawn_castle, spawn_skeletons, setup_hud, spawn_dragon,
                               spawn_spire, spawn_mountains, spawn_enemies, spawn_items, spawn_props))
        // Always-on systems
        .add_systems(Update, (update_hearts, update_vignette, animate_lightning, animate_orb,
                               animate_eye, check_death, update_bars, update_hotbar, drink_anim,
                               tag_body_parts, flash_skeletons, flash_enemies, flash_dragon,
                               update_boss_bar, soapstone_msg))
        // Gameplay systems — only while alive
        .add_systems(Update, (player_movement, resolve_collisions, camera_look, head_bob, cursor_grab,
                               sword_swing, lightning_bolts,
                               skeleton_ai, skeleton_attack_anim, skeleton_walk_anim, lightning_damage,
                               dragon_ai, move_fireballs,
                               eye_beam, enemy_ai)
                               .chain()
                               .run_if(in_state(AppState::Playing)))
        .add_systems(Update, (witch_cast, move_magic_missiles, regen_resources,
                               inventory_scroll, update_held, use_item, glock_fire,
                               move_rockets, pickup_system, enemy_limb_anim, orc_combat, move_debris,
                               tick_transient, animate_mushroom, gun_recoil_anim)
                               .run_if(in_state(AppState::Playing)))
        .add_systems(Update, (dragon_breath, dragon_wing_flap, update_shockwaves,
                               bonfire_rest, animate_props, update_fire_patches,
                               update_sound_waves, spin_pickups)
                               .run_if(in_state(AppState::Playing)))
        // Death screen
        .add_systems(OnEnter(AppState::Dead), spawn_death_screen)
        .add_systems(OnExit(AppState::Dead),
                     (despawn_death_screen, reset_game, spawn_skeletons, spawn_dragon, spawn_enemies, spawn_items).chain())
        .add_systems(Update, death_button.run_if(in_state(AppState::Dead)))
        // Pause (Esc toggles)
        .add_systems(Update, toggle_pause)
        .add_systems(OnEnter(AppState::Paused), spawn_pause_screen)
        .add_systems(OnExit(AppState::Paused), despawn_pause_screen)
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

fn make_bark_texture(images: &mut Assets<Image>) -> Handle<Image> {
    // 16×16 vertical brown bark streaks
    let w = 16u32; let h = 16u32;
    let mut data: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
    for _y in 0..h {
        for x in 0..w {
            let streak = ((x * 5 + (x / 3) * 7) % 6) as u8;
            let r = 60 + streak * 7;
            let g = 36 + streak * 5;
            let b = 18 + streak * 3;
            data.extend_from_slice(&[r, g, b, 255]);
        }
    }
    let mut image = Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat, address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Nearest, min_filter: ImageFilterMode::Nearest, ..default()
    });
    images.add(image)
}

fn make_leaf_texture(images: &mut Assets<Image>) -> Handle<Image> {
    // 16×16 mottled pine-needle greens
    let w = 16u32; let h = 16u32;
    let mut data: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let n = ((x * 7 + y * 13 + x * y) % 6) as u8;
            let r = 14 + n * 4;
            let g = 52 + n * 9;
            let b = 18 + n * 4;
            data.extend_from_slice(&[r, g, b, 255]);
        }
    }
    let mut image = Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat, address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Nearest, min_filter: ImageFilterMode::Nearest, ..default()
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

    // Natural-looking mountain range: each massif is a main peak flanked by a few
    // smaller jagged sub-peaks (so the silhouette isn't a row of perfect cones),
    // with snow caps up top and scattered foothills filling the gaps.
    let spire = Vec2::new(320.0, 240.0);
    let hash = |n: f32| (n.sin() * 43758.547).fract().abs(); // cheap pseudo-random 0..1

    let count = 18u32;
    for i in 0..count {
        let a = (i as f32 / count as f32) * std::f32::consts::TAU + hash(i as f32) * 0.15;
        let dist = 700.0 + hash(i as f32 * 2.3) * 60.0 - 30.0;
        let bx = dist * a.cos();
        let bz = dist * a.sin();
        if Vec2::new(bx, bz).distance(spire) < 230.0 { continue; } // clear the spire

        let scale = 1.1 + hash(i as f32 * 5.1) * 0.9; // 1.1 .. 2.0
        let r = 120.0 * scale;
        let hgt = 150.0 * scale;

        // Main peak
        commands.spawn((
            Mesh3d(meshes.add(Cone { radius: r, height: hgt })),
            MeshMaterial3d(rock.clone()),
            Transform::from_xyz(bx, hgt * 0.5, bz).with_rotation(Quat::from_rotation_y(hash(i as f32) * 6.28)),
        ));
        let sh = hgt * 0.30;
        commands.spawn((
            Mesh3d(meshes.add(Cone { radius: r * 0.38, height: sh })),
            MeshMaterial3d(snow.clone()),
            Transform::from_xyz(bx, hgt - sh * 0.5, bz),
        ));
        // 2–3 smaller jagged shoulders around the main peak
        let subs = 2 + (hash(i as f32 * 3.7) * 2.0) as u32;
        for j in 0..subs {
            let sa = a + (j as f32 - 1.0) * 0.18 + hash((i * 7 + j) as f32) * 0.1;
            let sd = dist - 60.0 - hash((i + j) as f32) * 40.0;
            let sx = sd * sa.cos();
            let sz = sd * sa.sin();
            let ss = 0.5 + hash((i * 13 + j) as f32) * 0.5;
            let sr = r * ss;
            let shh = hgt * (0.55 + ss * 0.3);
            commands.spawn((
                Mesh3d(meshes.add(Cone { radius: sr, height: shh })),
                MeshMaterial3d(rock.clone()),
                Transform::from_xyz(sx, shh * 0.5, sz),
            ));
            if shh > hgt * 0.7 {
                let ssh = shh * 0.26;
                commands.spawn((
                    Mesh3d(meshes.add(Cone { radius: sr * 0.36, height: ssh })),
                    MeshMaterial3d(snow.clone()),
                    Transform::from_xyz(sx, shh - ssh * 0.5, sz),
                ));
            }
        }
    }

    // Low rolling foothills scattered just inside the range
    for k in 0..26u32 {
        let a = hash(k as f32 * 1.7) * std::f32::consts::TAU;
        let dist = 560.0 + hash(k as f32 * 4.2) * 120.0;
        let x = dist * a.cos();
        let z = dist * a.sin();
        if Vec2::new(x, z).distance(spire) < 210.0 { continue; }
        let r = 45.0 + hash(k as f32 * 9.1) * 45.0;
        let hgt = r * (0.7 + hash(k as f32 * 2.0) * 0.5);
        commands.spawn((
            Mesh3d(meshes.add(Cone { radius: r, height: hgt })),
            MeshMaterial3d(rock.clone()),
            Transform::from_xyz(x, hgt * 0.5, z),
        ));
    }
}

// Dark-Souls-flavoured props that make the field feel lived-in & haunted:
// a bonfire rest-point, gravestones, ruined arches, drifting fog and item motes.
fn spawn_props(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let hash = |n: f32| (n.sin() * 43758.547).fract().abs();
    let stone = materials.add(StandardMaterial { base_color: Color::srgb(0.32, 0.31, 0.34), perceptual_roughness: 1.0, ..default() });
    let moss  = materials.add(StandardMaterial { base_color: Color::srgb(0.20, 0.28, 0.18), perceptual_roughness: 1.0, ..default() });
    let ash   = materials.add(StandardMaterial { base_color: Color::srgb(0.10, 0.09, 0.09), perceptual_roughness: 1.0, ..default() });
    let steel = materials.add(StandardMaterial { base_color: Color::srgb(0.45, 0.46, 0.5), metallic: 0.8, perceptual_roughness: 0.3, ..default() });
    let bone  = materials.add(StandardMaterial { base_color: Color::srgb(0.78, 0.74, 0.64), perceptual_roughness: 0.8, ..default() });
    let flame_m = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.65, 0.15), emissive: LinearRgba::new(7.0, 3.0, 0.4, 1.0), unlit: true, ..default() });
    let fog_m = materials.add(StandardMaterial { base_color: Color::srgba(0.7, 0.72, 0.78, 0.10), unlit: true, alpha_mode: AlphaMode::Blend, ..default() });
    let mote_m = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.85, 0.4), emissive: LinearRgba::new(5.0, 3.5, 1.0, 1.0), unlit: true, ..default() });
    let rune_m = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.6, 0.1), emissive: LinearRgba::new(4.0, 1.5, 0.2, 1.0), unlit: true, alpha_mode: AlphaMode::Add, ..default() });

    // ── The bonfire (rest point) near the spawn ──
    build_bonfire(&mut commands, &mut meshes, Vec3::new(7.0, 0.0, 9.0), &ash, &bone, &steel, &flame_m);

    // ── Drifting embers in the air for atmosphere ──
    for k in 0..60u32 {
        let a = hash(k as f32 * 3.1) * std::f32::consts::TAU;
        let d = 10.0 + hash(k as f32 * 1.9) * 240.0;
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.07))), MeshMaterial3d(mote_m.clone()),
            Transform::from_xyz(a.cos() * d, 1.0, a.sin() * d),
            Ember { seed: hash(k as f32 * 5.5) * 100.0 },
        ));
    }

    // ── Gravestones, ruined arches, fog & motes scattered through the field ──
    for i in 0..40u32 {
        let a = hash(i as f32 * 1.1) * std::f32::consts::TAU;
        let d = 25.0 + hash(i as f32 * 2.7) * 230.0;
        let x = a.cos() * d;
        let z = a.sin() * d;
        // keep clear of the castle footprint and the bonfire
        if x.abs() < 64.0 && z < -25.0 && z > -160.0 { continue; }
        if Vec2::new(x, z).distance(Vec2::new(7.0, 9.0)) < 6.0 { continue; }
        let kind = i % 5;
        match kind {
            0 | 1 => {
                // leaning gravestone with a mossy top
                let tilt = (hash(i as f32) - 0.5) * 0.4;
                commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.9, 1.6, 0.22))), MeshMaterial3d(stone.clone()),
                    Transform::from_xyz(x, 0.8, z).with_rotation(Quat::from_rotation_z(tilt) * Quat::from_rotation_y(a))));
                commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.94, 0.18, 0.26))), MeshMaterial3d(moss.clone()),
                    Transform::from_xyz(x, 1.6, z).with_rotation(Quat::from_rotation_z(tilt) * Quat::from_rotation_y(a))));
            }
            2 => {
                // broken ruined pillar + rubble (solid)
                let h = 3.0 + hash(i as f32 * 5.0) * 4.0;
                commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.7, half_height: h * 0.5 })), MeshMaterial3d(stone.clone()),
                    Transform::from_xyz(x, h * 0.5, z),
                    Collider { half: Vec2::new(0.7, 0.7) }));
                for r in 0..3u32 {
                    let ra = r as f32 * 2.1;
                    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.5, 0.4, 0.5))), MeshMaterial3d(stone.clone()),
                        Transform::from_xyz(x + ra.cos() * 1.3, 0.2, z + ra.sin() * 1.3).with_rotation(Quat::from_rotation_y(ra))));
                }
            }
            3 => {
                // drifting low fog bank
                commands.spawn((Mesh3d(meshes.add(Cuboid::new(14.0, 0.1, 14.0))), MeshMaterial3d(fog_m.clone()),
                    Transform::from_xyz(x, 1.2, z),
                    FogDrift { base: Vec3::new(x, 1.2, z), phase: hash(i as f32) * 6.28 }));
            }
            _ => {
                // glowing item mote + a soapstone rune on the ground
                commands.spawn((Mesh3d(meshes.add(Sphere::new(0.18))), MeshMaterial3d(mote_m.clone()),
                    Transform::from_xyz(x, 1.0, z),
                    FloatMote { base: Vec3::new(x, 1.0, z), phase: hash(i as f32) * 6.28 }));
                commands.spawn((Mesh3d(meshes.add(Annulus::new(0.4, 0.6))), MeshMaterial3d(rune_m.clone()),
                    Transform::from_xyz(x, 0.06, z).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
                    Soapstone { idx: i as usize }));
            }
        }
    }
}

// Builds one bonfire (ash mound, ring of bones, coiled sword, flames, light).
fn build_bonfire(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    pos: Vec3,
    ash: &Handle<StandardMaterial>,
    bone: &Handle<StandardMaterial>,
    steel: &Handle<StandardMaterial>,
    flame_m: &Handle<StandardMaterial>,
) {
    commands.spawn((
        Transform::from_translation(pos), GlobalTransform::default(), Visibility::default(), Bonfire,
        PointLight { color: Color::srgb(1.0, 0.55, 0.2), intensity: 300_000.0, range: 30.0, shadows_enabled: false, ..default() },
    )).with_children(|b| {
        b.spawn((Mesh3d(meshes.add(Cylinder { radius: 1.3, half_height: 0.18 })), MeshMaterial3d(ash.clone()), Transform::from_xyz(0.0, 0.18, 0.0)));
        for i in 0..9u32 {
            let a = i as f32 / 9.0 * std::f32::consts::TAU;
            b.spawn((Mesh3d(meshes.add(Cuboid::new(0.08, 0.5, 0.08))), MeshMaterial3d(bone.clone()),
                Transform::from_xyz(a.cos() * 1.1, 0.35, a.sin() * 1.1).with_rotation(Quat::from_rotation_z(a.cos() * 0.5) * Quat::from_rotation_x(a.sin() * 0.5))));
        }
        b.spawn((Mesh3d(meshes.add(Cuboid::new(0.08, 1.7, 0.03))), MeshMaterial3d(steel.clone()), Transform::from_xyz(0.0, 1.0, 0.0)));
        b.spawn((Mesh3d(meshes.add(Cuboid::new(0.32, 0.07, 0.07))), MeshMaterial3d(steel.clone()), Transform::from_xyz(0.0, 1.5, 0.0)));
        for i in 0..5u32 {
            let a = i as f32 / 5.0 * std::f32::consts::TAU;
            b.spawn((Mesh3d(meshes.add(Cone { radius: 0.22, height: 0.9 }.mesh().resolution(5))), MeshMaterial3d(flame_m.clone()),
                Transform::from_xyz(a.cos() * 0.35, 0.55, a.sin() * 0.35), BonfireFlame));
        }
        b.spawn((Mesh3d(meshes.add(Cone { radius: 0.3, height: 1.3 }.mesh().resolution(6))), MeshMaterial3d(flame_m.clone()),
            Transform::from_xyz(0.0, 0.7, 0.0), BonfireFlame));
    });
}

// Rest at the bonfire (press R nearby) — full heal + refill potions & mana, Dark Souls style.
fn bonfire_rest(
    key: Res<ButtonInput<KeyCode>>,
    player_q: Query<&Transform, With<Player>>,
    bonfire_q: Query<&GlobalTransform, With<Bonfire>>,
    mut health: ResMut<PlayerHealth>,
    mut mana: ResMut<Mana>,
    mut stamina: ResMut<Stamina>,
    mut inv: ResMut<Inventory>,
    mut commands: Commands,
) {
    if !key.just_pressed(KeyCode::KeyR) { return; }
    let pp = player_q.single().translation;
    for bf in &bonfire_q {
        if bf.translation().distance(pp) < 5.0 {
            health.hearts = 5;
            health.hurt_timer = 0.0;
            mana.current = mana.max;
            stamina.current = stamina.max;
            inv.health_potions = inv.health_potions.max(3);
            inv.mana_potions = inv.mana_potions.max(3);
            // Burst of gold light for ~1s to signal the bonfire kindling
            commands.spawn((
                PointLight { color: Color::srgb(1.0, 0.78, 0.25), intensity: 1_500_000.0,
                    range: 45.0, shadows_enabled: false, ..default() },
                Transform::from_translation(bf.translation() + Vec3::Y * 1.5),
                Transient { life: 1.0 },
            ));
        }
    }
}

// Drift the fog banks, bob the item motes, and waft the rising embers.
fn animate_props(
    time: Res<Time>,
    mut fog_q: Query<(&mut Transform, &FogDrift), (Without<FloatMote>, Without<Ember>)>,
    mut mote_q: Query<(&mut Transform, &FloatMote), (Without<FogDrift>, Without<Ember>)>,
    mut ember_q: Query<(&mut Transform, &Ember), (Without<FogDrift>, Without<FloatMote>)>,
) {
    let t = time.elapsed_secs();
    for (mut tr, f) in fog_q.iter_mut() {
        tr.translation = f.base + Vec3::new((t * 0.15 + f.phase).sin() * 6.0, 0.0, (t * 0.12 + f.phase).cos() * 6.0);
    }
    for (mut tr, m) in mote_q.iter_mut() {
        tr.translation = m.base + Vec3::Y * ((t * 1.5 + m.phase).sin() * 0.3);
    }
    for (mut tr, e) in ember_q.iter_mut() {
        // Slowly rise from the ground to ~16 then loop, with a gentle sway
        let rise = ((t * 0.45 + e.seed) % 1.0) * 16.0;
        tr.translation.y = 0.5 + rise;
        tr.translation.x += (t * 1.3 + e.seed).sin() * 0.01;
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
        Mesh3d(meshes.add(Plane3d::default().mesh().size(1560.0, 1560.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(grass),
            uv_transform: bevy::math::Affine2::from_scale(Vec2::new(390.0, 390.0)),
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

    // --- Trees (big, brushy textured pines) ---
    let bark_tex = make_bark_texture(&mut images);
    let leaf_tex = make_leaf_texture(&mut images);
    let trunk_mat = materials.add(StandardMaterial {
        base_color_texture: Some(bark_tex),
        uv_transform: bevy::math::Affine2::from_scale(Vec2::new(2.0, 6.0)),
        perceptual_roughness: 1.0, ..default()
    });
    let leaf_mat = materials.add(StandardMaterial {
        base_color_texture: Some(leaf_tex),
        uv_transform: bevy::math::Affine2::from_scale(Vec2::new(4.0, 4.0)),
        perceptual_roughness: 1.0, ..default()
    });
    // Fuller, rounder foliage (resolution 9) in four overlapping skirts
    let trunk_mesh = meshes.add(Cuboid::new(0.8, 6.5, 0.8));
    let cone_skirt = meshes.add(Cone { radius: 5.6, height: 6.0 }.mesh().resolution(9));
    let cone_lo    = meshes.add(Cone { radius: 4.6, height: 6.0 }.mesh().resolution(9));
    let cone_mid   = meshes.add(Cone { radius: 3.4, height: 5.5 }.mesh().resolution(9));
    let cone_hi    = meshes.add(Cone { radius: 2.0, height: 4.5 }.mesh().resolution(9));

    for i in 0..320u32 {
        let t     = i as f32;
        let angle = t * 137.508_f32.to_radians();
        let dist  = (18.0 + t * 1.8_f32).min(560.0);
        let x     = dist * angle.cos();
        let z     = dist * angle.sin();

        if x.abs() < 14.0 && z > -6.0 && z < 22.0 { continue; }   // player spawn
        if x.abs() < 66.0 && z < -25.0 && z > -160.0 { continue; } // castle zone
        if (x - 320.0).abs() < 30.0 && (z - 240.0).abs() < 30.0 { continue; } // spire zone

        let base = Vec3::new(x, 0.0, z);
        let yaw = Quat::from_rotation_y(t * 1.3); // vary facing so they don't look identical

        // Trunk (taller/thicker) with collider
        commands.spawn((
            Mesh3d(trunk_mesh.clone()), MeshMaterial3d(trunk_mat.clone()),
            Transform::from_translation(base + Vec3::Y * 3.25),
            Collider { half: Vec2::new(0.55, 0.55) },
        ));
        // Four overlapping brushy skirts of foliage
        for (mesh, cy) in [(&cone_skirt, 9.0), (&cone_lo, 11.5), (&cone_mid, 14.0), (&cone_hi, 16.5)] {
            commands.spawn((
                Mesh3d(mesh.clone()), MeshMaterial3d(leaf_mat.clone()),
                Transform::from_translation(base + Vec3::Y * cy).with_rotation(yaw),
            ));
        }
    }

    // ── Materials ─────────────────────────────────────────────
    let blade_mat  = materials.add(StandardMaterial { base_color: Color::srgb(0.68, 0.80, 0.98), emissive: LinearRgba::new(0.10, 0.16, 0.32, 1.0), perceptual_roughness: 1.0, ..default() });
    let blade_edge = materials.add(StandardMaterial { base_color: Color::srgb(0.92, 0.96, 1.00), emissive: LinearRgba::new(0.35, 0.40, 0.55, 1.0), perceptual_roughness: 1.0, ..default() });
    let gold_mat   = materials.add(StandardMaterial { base_color: Color::srgb(0.92, 0.75, 0.12), emissive: LinearRgba::new(0.30, 0.22, 0.02, 1.0), perceptual_roughness: 1.0, ..default() });
    let grip_mat   = materials.add(StandardMaterial { base_color: Color::srgb(0.55, 0.08, 0.08), perceptual_roughness: 1.0, ..default() });
    let gauntlet_m = materials.add(StandardMaterial { base_color: Color::srgb(0.18, 0.16, 0.20), perceptual_roughness: 1.0, ..default() });
    let bolt_mat   = materials.add(StandardMaterial { base_color: Color::srgb(0.5, 0.75, 1.0),   emissive: LinearRgba::new(3.0, 6.0, 12.0, 1.0), unlit: true, ..default() });
    let bolt2_mat  = materials.add(StandardMaterial { base_color: Color::WHITE, emissive: LinearRgba::new(5.0, 8.0, 16.0, 1.0), unlit: true, ..default() });

    // ── Player + camera ───────────────────────────────────────
    let player_e = commands.spawn((
        Player, Transform::from_xyz(0.0, 0.0, 10.0),
        GlobalTransform::default(), Visibility::default(),
        PlayerVelocity { vertical: 0.0, knockback: Vec3::ZERO, roll_timer: 0.0, roll_dir: Vec3::ZERO },
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
        HeldVisual { kind: ItemKind::Sword },
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
    // ── Magical triangle artifact above the palm ──
    let art_pos = Vec3::new(0.0, 0.06, -0.18);
    let tri_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.6, 0.3, 1.0, 0.34),
        emissive: LinearRgba::new(1.4, 0.6, 3.0, 1.0),
        unlit: true, alpha_mode: AlphaMode::Blend, cull_mode: None, double_sided: true, ..default()
    });
    let ring_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.4, 0.9, 1.0, 0.7),
        emissive: LinearRgba::new(0.8, 2.5, 4.0, 1.0),
        unlit: true, alpha_mode: AlphaMode::Blend, cull_mode: None, double_sided: true, ..default()
    });
    let ball_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.95, 1.0),
        emissive: LinearRgba::new(5.0, 7.0, 10.0, 1.0), unlit: true, ..default()
    });

    // Triangle + concentric magic-pattern circles — spins one way
    let tri_root = commands.spawn((
        Transform::from_translation(art_pos), GlobalTransform::default(), Visibility::default(),
        ArtifactSpin { dir: 1.0 },
    )).set_parent(hand_root).id();
    commands.spawn((Mesh3d(meshes.add(RegularPolygon::new(0.12, 3))), MeshMaterial3d(tri_mat.clone()),
        Transform::default())).set_parent(tri_root);
    commands.spawn((Mesh3d(meshes.add(Annulus::new(0.045, 0.058))), MeshMaterial3d(ring_mat.clone()),
        Transform::from_xyz(0.0, 0.0, 0.002))).set_parent(tri_root);
    commands.spawn((Mesh3d(meshes.add(Annulus::new(0.085, 0.095))), MeshMaterial3d(ring_mat.clone()),
        Transform::from_xyz(0.0, 0.0, 0.002))).set_parent(tri_root);

    // Orbit ring + 3 tiny orbs at the triangle's tips — spins the opposite way
    let orbit_root = commands.spawn((
        Transform::from_translation(art_pos), GlobalTransform::default(), Visibility::default(),
        ArtifactSpin { dir: -1.0 },
    )).set_parent(hand_root).id();
    commands.spawn((Mesh3d(meshes.add(Annulus::new(0.118, 0.126))), MeshMaterial3d(ring_mat.clone()),
        Transform::from_xyz(0.0, 0.0, 0.004))).set_parent(orbit_root);
    for k in 0..3u32 {
        let a = k as f32 / 3.0 * std::f32::consts::TAU + std::f32::consts::FRAC_PI_2;
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.02))), MeshMaterial3d(ball_mat.clone()),
            Transform::from_xyz(a.cos() * 0.122, a.sin() * 0.122, 0.004))).set_parent(orbit_root);
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

    // ── Held weapon visuals (hidden until selected via scroll wheel) ──
    let gun_body = materials.add(StandardMaterial { base_color: Color::srgb(0.09, 0.09, 0.11), metallic: 0.6, perceptual_roughness: 0.35, ..default() });
    let gun_dark = materials.add(StandardMaterial { base_color: Color::srgb(0.04, 0.04, 0.05), metallic: 0.4, perceptual_roughness: 0.6, ..default() });
    let gun_accent = materials.add(StandardMaterial { base_color: Color::srgb(0.5, 0.5, 0.55), metallic: 0.9, perceptual_roughness: 0.25, ..default() });
    let warn = materials.add(StandardMaterial { base_color: Color::srgb(0.9, 0.55, 0.05), perceptual_roughness: 0.6, ..default() });
    let metal = materials.add(StandardMaterial { base_color: Color::srgb(0.12, 0.12, 0.14), metallic: 0.7, perceptual_roughness: 0.4, ..default() });
    let red_glass  = materials.add(StandardMaterial { base_color: Color::srgb(0.9, 0.15, 0.15), emissive: LinearRgba::new(1.2, 0.1, 0.1, 1.0), ..default() });
    let blue_glass = materials.add(StandardMaterial { base_color: Color::srgb(0.2, 0.4, 0.95), emissive: LinearRgba::new(0.2, 0.6, 1.6, 1.0), ..default() });

    // ── Detailed Glock pistol (raised & toward centre, scaled down ~0.8) ──
    let glock = commands.spawn((
        Transform { translation: Vec3::new(0.18, -0.12, -0.46), scale: Vec3::splat(0.8), ..default() },
        GlobalTransform::default(),
        Visibility::Hidden, HeldVisual { kind: ItemKind::Glock },
    )).set_parent(camera_e).id();
    // Slide — sleek squared-off top
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.052, 0.06, 0.32))), MeshMaterial3d(gun_body.clone()),
        Transform::from_xyz(0.0, 0.03, -0.05))).set_parent(glock);
    // Slide serrations near the rear (a few thin grooves)
    for gz in [0.07f32, 0.10, 0.13] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.056, 0.05, 0.012))), MeshMaterial3d(gun_dark.clone()),
            Transform::from_xyz(0.0, 0.035, gz))).set_parent(glock);
    }
    // Iron sights
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.018, 0.018, 0.018))), MeshMaterial3d(gun_dark.clone()),
        Transform::from_xyz(0.0, 0.066, 0.15))).set_parent(glock);       // rear sight
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.012, 0.018, 0.016))), MeshMaterial3d(gun_dark.clone()),
        Transform::from_xyz(0.0, 0.066, -0.20))).set_parent(glock);      // front sight
    // Frame under the slide + dust cover
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.046, 0.032, 0.30))), MeshMaterial3d(gun_dark.clone()),
        Transform::from_xyz(0.0, -0.012, -0.04))).set_parent(glock);
    // Trigger guard (front bar) + trigger
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.040, 0.052, 0.026))), MeshMaterial3d(gun_body.clone()),
        Transform::from_xyz(0.0, -0.06, -0.02))).set_parent(glock);
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.014, 0.034, 0.014))), MeshMaterial3d(gun_accent.clone()),
        Transform::from_xyz(0.0, -0.05, 0.012))).set_parent(glock);      // trigger
    // Angled grip with a subtle backstrap + magazine base
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.05, 0.18, 0.07))), MeshMaterial3d(gun_body.clone()),
        Transform::from_xyz(0.0, -0.155, 0.075).with_rotation(Quat::from_rotation_x(0.26)))).set_parent(glock);
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.054, 0.022, 0.078))), MeshMaterial3d(gun_dark.clone()),
        Transform::from_xyz(0.0, -0.245, 0.10))).set_parent(glock);      // mag base
    // Barrel tip (muzzle)
    commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.016, half_height: 0.03 })), MeshMaterial3d(gun_accent.clone()),
        Transform::from_xyz(0.0, 0.03, -0.22).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)))).set_parent(glock);

    // ── Big rocket launcher ──
    let rocket = commands.spawn((
        Transform::from_xyz(0.30, -0.22, -0.52), GlobalTransform::default(),
        Visibility::Hidden, HeldVisual { kind: ItemKind::Rocket },
    )).set_parent(camera_e).id();
    // main tube (long, fat)
    commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.12, half_height: 0.62 })), MeshMaterial3d(gun_body.clone()),
        Transform::from_xyz(0.0, 0.0, 0.0).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)))).set_parent(rocket);
    // flared rear exhaust
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.17, height: 0.22 }.mesh().resolution(12))), MeshMaterial3d(gun_dark.clone()),
        Transform::from_xyz(0.0, 0.0, 0.66).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)))).set_parent(rocket);
    // front muzzle ring
    commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.15, half_height: 0.05 })), MeshMaterial3d(gun_accent.clone()),
        Transform::from_xyz(0.0, 0.0, -0.6).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)))).set_parent(rocket);
    // warning stripe
    commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.125, half_height: 0.05 })), MeshMaterial3d(warn.clone()),
        Transform::from_xyz(0.0, 0.0, -0.35).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)))).set_parent(rocket);
    // grip + foregrip
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.06, 0.16, 0.08))), MeshMaterial3d(gun_dark.clone()),
        Transform::from_xyz(0.0, -0.20, 0.10).with_rotation(Quat::from_rotation_x(0.25)))).set_parent(rocket);
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.05, 0.14, 0.06))), MeshMaterial3d(gun_dark.clone()),
        Transform::from_xyz(0.0, -0.16, -0.28).with_rotation(Quat::from_rotation_x(-0.2)))).set_parent(rocket);
    // top sight rail
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.03, 0.05, 0.3))), MeshMaterial3d(gun_accent.clone()),
        Transform::from_xyz(0.0, 0.14, -0.1))).set_parent(rocket);
    // a loaded rocket tip peeking out the front
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.08, height: 0.18 }.mesh().resolution(3))), MeshMaterial3d(red_glass.clone()),
        Transform::from_xyz(0.0, 0.0, -0.68).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)))).set_parent(rocket);

    // Potions: red (health) + blue (mana) bottles held up
    for (kind, glass) in [(ItemKind::HealthPotion, red_glass.clone()), (ItemKind::ManaPotion, blue_glass.clone())] {
        let potion = commands.spawn((
            Transform::from_xyz(0.24, -0.20, -0.40), GlobalTransform::default(),
            Visibility::Hidden, HeldVisual { kind },
        )).set_parent(camera_e).id();
        commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.06, half_height: 0.09 })), MeshMaterial3d(glass.clone()),
            Transform::from_xyz(0.0, 0.0, 0.0))).set_parent(potion);        // body
        commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.025, half_height: 0.05 })), MeshMaterial3d(glass.clone()),
            Transform::from_xyz(0.0, 0.13, 0.0))).set_parent(potion);       // neck
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.05, 0.04, 0.05))), MeshMaterial3d(metal.clone()),
            Transform::from_xyz(0.0, 0.19, 0.0))).set_parent(potion);       // cork
    }
}

fn sword_swing(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    time: Res<Time>,
    inv: Res<Inventory>,
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
        && inv.selected == ItemKind::Sword
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

        // Hit check at swing peak — all enemies flash red and are knocked back
        if !sword.hit_registered && sword.timer > 0.14 {
            sword.hit_registered = true;
            let cam_gt = camera_q.single();
            let (_, rot, cam_pos) = cam_gt.to_scale_rotation_translation();
            let fwd = rot * Vec3::NEG_Z;
            for (entity, skel_gt, mut skel) in skeleton_q.iter_mut() {
                if skel.state == SkeletonState::Dead { continue; }
                let to = skel_gt.translation() + Vec3::Y - cam_pos;
                let dist = to.length();
                if dist < 3.6 && dist > 0.1 && fwd.dot(to / dist) > 0.3 {
                    skel.health -= 1.0;
                    skel.damage_flash = 0.25;
                    skel.knockback_vel = Vec3::new(to.x, 0.0, to.z).normalize_or_zero() * 11.0;
                    if skel.health <= 0.0 {
                        skel.state = SkeletonState::Dead;
                        commands.entity(entity).despawn_recursive();
                    }
                }
            }
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
            for (entity, egt, mut en) in enemy_q.iter_mut() {
                let to = egt.translation() + Vec3::Y - cam_pos;
                let dist = to.length();
                if dist < 3.6 && dist > 0.1 && fwd.dot(to / dist) > 0.3 {
                    en.health -= 1.0;
                    en.damage_flash = 0.25;
                    en.knockback_vel = Vec3::new(to.x, 0.0, to.z).normalize_or_zero() * 11.0;
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

// Slowly spin the triangle artifact and counter-spin its orbiting orbs.
fn animate_orb(
    time: Res<Time>,
    mut q: Query<(&mut Transform, &ArtifactSpin)>,
) {
    let dt = time.delta_secs();
    for (mut tr, spin) in q.iter_mut() {
        // Spin about the forward (Z) axis so the flat triangle/rings turn in view
        tr.rotate_z(spin.dir * 0.7 * dt);
    }
}

fn lightning_bolts(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    time: Res<Time>,
    inv: Res<Inventory>,
    mut mana: ResMut<Mana>,
    mut bolt_q: Query<(&mut Transform, &mut Visibility, &LightningBolt)>,
    mut light_q: Query<&mut PointLight, With<LightningLight>>,
) {
    let window = windows.single();
    // Lightning is the off-hand spell — available regardless of selected item, but costs mana
    let _ = &inv;
    let active = mouse.pressed(MouseButton::Right)
        && window.cursor_options.grab_mode == CursorGrabMode::Locked
        && mana.current > 0.0;
    if active {
        mana.current = (mana.current - 21.0 * time.delta_secs()).max(0.0); // -30% drain
    }

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
    let accent = materials.add(StandardMaterial { base_color: Color::srgb(0.85, 0.10, 0.55), emissive: LinearRgba::new(0.5, 0.0, 0.3, 1.0), perceptual_roughness: 0.6, ..default() });
    // Orc materials (replaces the old knights)
    let orc_skin = materials.add(StandardMaterial { base_color: Color::srgb(0.30, 0.45, 0.20), perceptual_roughness: 0.9, ..default() });
    let orc_dark = materials.add(StandardMaterial { base_color: Color::srgb(0.20, 0.30, 0.14), perceptual_roughness: 0.95, ..default() });
    let leather  = materials.add(StandardMaterial { base_color: Color::srgb(0.25, 0.16, 0.10), perceptual_roughness: 1.0, ..default() });
    let iron     = materials.add(StandardMaterial { base_color: Color::srgb(0.30, 0.30, 0.34), metallic: 0.7, perceptual_roughness: 0.5, ..default() });
    let tusk     = materials.add(StandardMaterial { base_color: Color::srgb(0.9, 0.87, 0.75), perceptual_roughness: 0.6, ..default() });
    let bat_mat = materials.add(StandardMaterial { base_color: Color::srgb(0.10, 0.08, 0.13), perceptual_roughness: 1.0, ..default() });
    let eye_red = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.0, 0.0), emissive: LinearRgba::new(5.0, 0.0, 0.0, 1.0), unlit: true, ..default() });

    // ── Witches (fast chasers) — refined sorceress ──
    // Extra materials for a prettier look
    let gown_lt  = materials.add(StandardMaterial { base_color: Color::srgb(0.42, 0.12, 0.55), perceptual_roughness: 0.7, ..default() });
    let hair_w   = materials.add(StandardMaterial { base_color: Color::srgb(0.16, 0.05, 0.20), perceptual_roughness: 0.45, metallic: 0.2, ..default() });
    let wood_w   = materials.add(StandardMaterial { base_color: Color::srgb(0.25, 0.15, 0.08), perceptual_roughness: 0.9, ..default() });
    let eyes_w   = materials.add(StandardMaterial { base_color: Color::srgb(0.6, 1.0, 1.0), emissive: LinearRgba::new(2.0, 5.0, 6.0, 1.0), unlit: true, ..default() });
    let crystal  = materials.add(StandardMaterial { base_color: Color::srgb(0.9, 0.3, 1.0), emissive: LinearRgba::new(4.0, 1.0, 6.0, 1.0), unlit: true, ..default() });

    // Smooth meshes (high resolution)
    let w_skirt_lo = meshes.add(Cone { radius: 0.60, height: 1.0 }.mesh().resolution(20));
    let w_skirt_hi = meshes.add(Cone { radius: 0.40, height: 0.7 }.mesh().resolution(20));
    let w_hem      = meshes.add(Cylinder { radius: 0.58, half_height: 0.05 });
    let w_waist    = meshes.add(Cylinder { radius: 0.15, half_height: 0.12 });
    let w_belt     = meshes.add(Cylinder { radius: 0.175, half_height: 0.045 });
    let w_bodice   = meshes.add(Cone { radius: 0.22, height: 0.5 }.mesh().resolution(20)); // tapers up to shoulders
    let w_bust     = meshes.add(Sphere::new(0.085));
    let w_shoulder = meshes.add(Sphere::new(0.07));
    let w_arm      = meshes.add(Cylinder { radius: 0.038, half_height: 0.24 });
    let w_hand     = meshes.add(Sphere::new(0.05));
    let w_neck     = meshes.add(Cylinder { radius: 0.042, half_height: 0.06 });
    let w_head     = meshes.add(Sphere::new(0.145));
    let w_eye      = meshes.add(Cuboid::new(0.035, 0.05, 0.02));
    let w_hairback = meshes.add(Sphere::new(0.16));
    let w_lock     = meshes.add(Cylinder { radius: 0.05, half_height: 0.28 });
    let w_brim     = meshes.add(Cylinder { radius: 0.42, half_height: 0.022 });
    let w_hat      = meshes.add(Cone { radius: 0.27, height: 0.85 }.mesh().resolution(20));
    let w_tip      = meshes.add(Cone { radius: 0.08, height: 0.30 }.mesh().resolution(16));
    let w_band     = meshes.add(Cylinder { radius: 0.285, half_height: 0.05 });
    let w_staff    = meshes.add(Cylinder { radius: 0.022, half_height: 0.72 });
    let w_crystal  = meshes.add(Sphere::new(0.09));
    let w_mouth    = meshes.add(Cuboid::new(0.024, 0.024, 0.02));
    let mouth_mat  = materials.add(StandardMaterial { base_color: Color::srgb(0.04, 0.0, 0.02), ..default() });

    let witch_pos = [
        Vec3::new( 30.0, 0.0, -20.0), Vec3::new(-40.0, 0.0, -60.0),
        Vec3::new( 70.0, 0.0,  40.0), Vec3::new(-90.0, 0.0,  30.0),
        Vec3::new(130.0, 0.0, -50.0), Vec3::new(-130.0,0.0, -20.0),
        Vec3::new( 50.0, 0.0, 120.0), Vec3::new(-60.0, 0.0, 150.0),
        Vec3::new(170.0, 0.0,  90.0), Vec3::new(-180.0,0.0, 110.0),
        Vec3::new( 20.0, 0.0, -130.0),Vec3::new(-30.0, 0.0, 200.0),
    ];
    let hat_tilt = Quat::from_rotation_x(0.16);
    for p in witch_pos {
        commands.spawn((
            Transform::from_translation(p), GlobalTransform::default(), Visibility::default(),
            Enemy { health: 4.0, speed: 3.5, flying: false, base_y: 0.0, attack_timer: 1.3, knockback_vel: Vec3::ZERO, bob_phase: 0.0, anim_phase: 0.0, attack_anim: 0.0, moving: false, damage_flash: 0.0 },
            Shock { timer: 0.0 },
            WitchCaster { cast_timer: 2.0 },
        )).with_children(|c| {
            // Layered flowing gown
            c.spawn((Mesh3d(w_skirt_lo.clone()), MeshMaterial3d(robe.clone()),   Transform::from_xyz(0.0, 0.50, 0.0)));
            c.spawn((Mesh3d(w_skirt_hi.clone()), MeshMaterial3d(gown_lt.clone()),Transform::from_xyz(0.0, 1.0, 0.0)));
            c.spawn((Mesh3d(w_hem.clone()),      MeshMaterial3d(accent.clone()), Transform::from_xyz(0.0, 0.06, 0.0)));
            // Cinched waist + corset belt
            c.spawn((Mesh3d(w_waist.clone()),    MeshMaterial3d(robe.clone()),   Transform::from_xyz(0.0, 1.18, 0.0)));
            c.spawn((Mesh3d(w_belt.clone()),     MeshMaterial3d(accent.clone()), Transform::from_xyz(0.0, 1.18, 0.0)));
            // Bodice tapering to shoulders + subtle chest
            c.spawn((Mesh3d(w_bodice.clone()),   MeshMaterial3d(robe.clone()),   Transform::from_xyz(0.0, 1.34, 0.0)));
            c.spawn((Mesh3d(w_bust.clone()),     MeshMaterial3d(robe.clone()),   Transform::from_xyz(-0.055, 1.48, -0.10)));
            c.spawn((Mesh3d(w_bust.clone()),     MeshMaterial3d(robe.clone()),   Transform::from_xyz( 0.055, 1.48, -0.10)));
            // Shoulders, arms, hands
            c.spawn((Mesh3d(w_shoulder.clone()), MeshMaterial3d(robe.clone()),   Transform::from_xyz(-0.20, 1.62, 0.0)));
            c.spawn((Mesh3d(w_shoulder.clone()), MeshMaterial3d(robe.clone()),   Transform::from_xyz( 0.20, 1.62, 0.0)));
            c.spawn((Mesh3d(w_arm.clone()),      MeshMaterial3d(robe.clone()),   Transform::from_xyz(-0.26, 1.40, 0.04).with_rotation(Quat::from_rotation_z( 0.30))));
            c.spawn((Mesh3d(w_arm.clone()),      MeshMaterial3d(robe.clone()),   Transform::from_xyz( 0.26, 1.40, 0.04).with_rotation(Quat::from_rotation_z(-0.30))));
            c.spawn((Mesh3d(w_hand.clone()),     MeshMaterial3d(skin.clone()),   Transform::from_xyz(-0.31, 1.17, 0.10)));
            c.spawn((Mesh3d(w_hand.clone()),     MeshMaterial3d(skin.clone()),   Transform::from_xyz( 0.31, 1.17, 0.10)));
            // Neck + head + glowing eyes
            c.spawn((Mesh3d(w_neck.clone()),     MeshMaterial3d(skin.clone()),   Transform::from_xyz(0.0, 1.74, 0.0)));
            c.spawn((Mesh3d(w_head.clone()),     MeshMaterial3d(skin.clone()),   Transform::from_xyz(0.0, 1.89, 0.0)));
            c.spawn((Mesh3d(w_eye.clone()),      MeshMaterial3d(eyes_w.clone()), Transform::from_xyz(-0.05, 1.92, -0.13)));
            c.spawn((Mesh3d(w_eye.clone()),      MeshMaterial3d(eyes_w.clone()), Transform::from_xyz( 0.05, 1.92, -0.13)));
            // Ominous upturned smile (no nose) — arc of small dark teeth
            for seg in -2i32..=2 {
                let mx = seg as f32 * 0.028;
                let my = 1.83 + (seg.abs() as f32) * 0.018; // ends curl upward
                c.spawn((Mesh3d(w_mouth.clone()), MeshMaterial3d(mouth_mat.clone()),
                    Transform::from_xyz(mx, my, -0.135)));
            }
            // Flowing hair: rounded back + long side locks
            c.spawn((Mesh3d(w_hairback.clone()), MeshMaterial3d(hair_w.clone()), Transform::from_xyz(0.0, 1.91, 0.07)));
            c.spawn((Mesh3d(w_lock.clone()),     MeshMaterial3d(hair_w.clone()), Transform::from_xyz(-0.13, 1.66, 0.06)));
            c.spawn((Mesh3d(w_lock.clone()),     MeshMaterial3d(hair_w.clone()), Transform::from_xyz( 0.13, 1.66, 0.06)));
            // Elegant hat with curled tip + band
            c.spawn((Mesh3d(w_brim.clone()),     MeshMaterial3d(hat.clone()),    Transform::from_xyz(0.0, 2.03, -0.02).with_rotation(hat_tilt)));
            c.spawn((Mesh3d(w_band.clone()),     MeshMaterial3d(accent.clone()), Transform::from_xyz(0.0, 2.08, -0.02).with_rotation(hat_tilt)));
            c.spawn((Mesh3d(w_hat.clone()),      MeshMaterial3d(hat.clone()),    Transform::from_xyz(0.0, 2.46, -0.06).with_rotation(hat_tilt)));
            c.spawn((Mesh3d(w_tip.clone()),      MeshMaterial3d(hat.clone()),    Transform::from_xyz(0.0, 2.86, -0.18).with_rotation(Quat::from_rotation_x(0.9))));
            // Magic staff with glowing crystal, held in the right hand
            c.spawn((Mesh3d(w_staff.clone()),    MeshMaterial3d(wood_w.clone()), Transform::from_xyz(0.34, 1.05, 0.10).with_rotation(Quat::from_rotation_z(-0.12))));
            c.spawn((Mesh3d(w_crystal.clone()),  MeshMaterial3d(crystal.clone()),Transform::from_xyz(0.30, 1.80, 0.10)));
        });
    }

    // ── Orcs (big, tanky, slow) wielding heavy hammers ──
    let o_torso  = meshes.add(Cuboid::new(0.70, 0.78, 0.42));
    let o_belly  = meshes.add(Sphere::new(0.34));
    let o_pelv   = meshes.add(Cuboid::new(0.52, 0.28, 0.34));
    let o_leg    = meshes.add(Cuboid::new(0.22, 0.62, 0.24));
    let o_foot   = meshes.add(Cuboid::new(0.24, 0.14, 0.34));
    let o_arm    = meshes.add(Cuboid::new(0.20, 0.58, 0.20));
    let o_fist   = meshes.add(Sphere::new(0.15));
    let o_shldr  = meshes.add(Sphere::new(0.20));
    let o_head   = meshes.add(Cuboid::new(0.34, 0.32, 0.34));
    let o_jaw    = meshes.add(Cuboid::new(0.30, 0.14, 0.26));
    let o_tusk   = meshes.add(Cone { radius: 0.05, height: 0.18 }.mesh().resolution(6));
    let o_eye    = meshes.add(Cuboid::new(0.06, 0.05, 0.04));
    // Hammer: handle + big stone head
    let h_shaft  = meshes.add(Cylinder { radius: 0.055, half_height: 0.7 });
    let h_head   = meshes.add(Cuboid::new(0.34, 0.40, 0.40));
    let o_eyemat = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.7, 0.0), emissive: LinearRgba::new(4.0, 2.0, 0.0, 1.0), unlit: true, ..default() });
    for p in [Vec3::new(-20.0,0.0,-40.0), Vec3::new(50.0,0.0,-50.0), Vec3::new(20.0,0.0,60.0), Vec3::new(-60.0,0.0,-90.0),
              Vec3::new(120.0,0.0,40.0), Vec3::new(-140.0,0.0,60.0), Vec3::new(160.0,0.0,-60.0), Vec3::new(-90.0,0.0,170.0)] {
        commands.spawn((
            Transform::from_translation(p), GlobalTransform::default(), Visibility::default(),
            Enemy { health: 10.0, speed: 3.0, flying: false, base_y: 0.0, attack_timer: 1.8, knockback_vel: Vec3::ZERO, bob_phase: 0.0, anim_phase: 0.0, attack_anim: 0.0, moving: false, damage_flash: 0.0 },
            Shock { timer: 0.0 },
            OrcBrute { slammed: false },
        )).with_children(|c| {
            // Hunched muscular torso + gut
            c.spawn((Mesh3d(o_torso.clone()), MeshMaterial3d(orc_skin.clone()), Transform::from_xyz(0.0, 1.55, 0.0).with_rotation(Quat::from_rotation_x(0.12))));
            c.spawn((Mesh3d(o_belly.clone()), MeshMaterial3d(orc_skin.clone()), Transform::from_xyz(0.0, 1.30, 0.10)));
            c.spawn((Mesh3d(o_pelv.clone()),  MeshMaterial3d(leather.clone()),  Transform::from_xyz(0.0, 1.02, 0.0)));
            // Thick legs — pivot at the hip so they stride (leg+foot hang below)
            for sx in [-0.20f32, 0.20] {
                c.spawn((
                    Transform::from_xyz(sx, 1.0, 0.0), GlobalTransform::default(), Visibility::default(),
                    EnemyLimb { is_arm: false, side: sx.signum() },
                )).with_children(|l| {
                    l.spawn((Mesh3d(o_leg.clone()),  MeshMaterial3d(orc_skin.clone()), Transform::from_xyz(0.0, -0.38, 0.0)));
                    l.spawn((Mesh3d(o_foot.clone()), MeshMaterial3d(orc_dark.clone()), Transform::from_xyz(0.0, -0.90, -0.05)));
                });
            }
            // Shoulders (static)
            c.spawn((Mesh3d(o_shldr.clone()), MeshMaterial3d(orc_skin.clone()), Transform::from_xyz(-0.46, 1.78, 0.0)));
            c.spawn((Mesh3d(o_shldr.clone()), MeshMaterial3d(orc_skin.clone()), Transform::from_xyz( 0.46, 1.78, 0.0)));
            // Arms + fists + hammer on ONE pivot at the shoulders, so it can chop overhead
            c.spawn((
                Transform::from_xyz(0.0, 1.78, 0.0), GlobalTransform::default(), Visibility::default(),
                EnemyLimb { is_arm: true, side: 1.0 },
            )).with_children(|a| {
                a.spawn((Mesh3d(o_arm.clone()),   MeshMaterial3d(orc_skin.clone()), Transform::from_xyz(-0.50, -0.38, 0.10).with_rotation(Quat::from_rotation_x(-0.4))));
                a.spawn((Mesh3d(o_arm.clone()),   MeshMaterial3d(orc_skin.clone()), Transform::from_xyz( 0.50, -0.38, 0.10).with_rotation(Quat::from_rotation_x(-0.4))));
                a.spawn((Mesh3d(o_fist.clone()),  MeshMaterial3d(orc_skin.clone()), Transform::from_xyz(-0.52, -0.68, 0.32)));
                a.spawn((Mesh3d(o_fist.clone()),  MeshMaterial3d(orc_skin.clone()), Transform::from_xyz( 0.52, -0.68, 0.32)));
                a.spawn((Mesh3d(h_shaft.clone()), MeshMaterial3d(leather.clone()), Transform::from_xyz(0.52, -0.48, 0.34).with_rotation(Quat::from_rotation_x(0.5))));
                a.spawn((Mesh3d(h_head.clone()),  MeshMaterial3d(iron.clone()),    Transform::from_xyz(0.52, 0.17, 0.66)));
            });
            // Head: brute jaw, tusks, glowing eyes
            c.spawn((Mesh3d(o_head.clone()), MeshMaterial3d(orc_skin.clone()), Transform::from_xyz(0.0, 2.05, 0.04)));
            c.spawn((Mesh3d(o_jaw.clone()),  MeshMaterial3d(orc_dark.clone()), Transform::from_xyz(0.0, 1.92, -0.06)));
            c.spawn((Mesh3d(o_tusk.clone()), MeshMaterial3d(tusk.clone()),     Transform::from_xyz(-0.10, 1.96, -0.16).with_rotation(Quat::from_rotation_x(3.14))));
            c.spawn((Mesh3d(o_tusk.clone()), MeshMaterial3d(tusk.clone()),     Transform::from_xyz( 0.10, 1.96, -0.16).with_rotation(Quat::from_rotation_x(3.14))));
            c.spawn((Mesh3d(o_eye.clone()),  MeshMaterial3d(o_eyemat.clone()), Transform::from_xyz(-0.08, 2.08, -0.18)));
            c.spawn((Mesh3d(o_eye.clone()),  MeshMaterial3d(o_eyemat.clone()), Transform::from_xyz( 0.08, 2.08, -0.18)));
        });
    }

    // ── Black Knights: rare, tanky elites in blackened plate with a greatsword ──
    let plate = materials.add(StandardMaterial { base_color: Color::srgb(0.07, 0.07, 0.09), metallic: 0.85, perceptual_roughness: 0.35, ..default() });
    let trim  = materials.add(StandardMaterial { base_color: Color::srgb(0.30, 0.26, 0.10), metallic: 0.9, perceptual_roughness: 0.3, ..default() });
    let visor = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.5, 0.0), emissive: LinearRgba::new(6.0, 2.0, 0.0, 1.0), unlit: true, ..default() });
    let bk_torso = meshes.add(Cuboid::new(0.5, 0.66, 0.34));
    let bk_pauld = meshes.add(Sphere::new(0.22));
    let bk_leg   = meshes.add(Cuboid::new(0.18, 0.62, 0.20));
    let bk_arm   = meshes.add(Cuboid::new(0.16, 0.5, 0.16));
    let bk_head  = meshes.add(Cuboid::new(0.26, 0.30, 0.28));
    let bk_horn  = meshes.add(Cone { radius: 0.06, height: 0.4 }.mesh().resolution(5));
    let bk_blade = meshes.add(Cuboid::new(0.10, 1.7, 0.04));
    let bk_eye   = meshes.add(Cuboid::new(0.18, 0.05, 0.04));
    for p in [Vec3::new(0.0, 0.0, -120.0), Vec3::new(40.0, 0.0, 30.0), Vec3::new(-46.0, 0.0, -8.0)] {
        commands.spawn((
            Transform::from_translation(p), GlobalTransform::default(), Visibility::default(),
            Enemy { health: 16.0, speed: 4.6, flying: false, base_y: 0.0, attack_timer: 1.4, knockback_vel: Vec3::ZERO, bob_phase: 0.0, anim_phase: 0.0, attack_anim: 0.0, moving: false, damage_flash: 0.0 },
            Shock { timer: 0.0 },
        )).with_children(|c| {
            c.spawn((Mesh3d(bk_torso.clone()), MeshMaterial3d(plate.clone()), Transform::from_xyz(0.0, 1.25, 0.0)));
            c.spawn((Mesh3d(meshes.add(Cuboid::new(0.34, 0.2, 0.24))), MeshMaterial3d(trim.clone()), Transform::from_xyz(0.0, 0.92, 0.0)));
            c.spawn((Mesh3d(bk_pauld.clone()), MeshMaterial3d(plate.clone()), Transform::from_xyz(-0.34, 1.5, 0.0)));
            c.spawn((Mesh3d(bk_pauld.clone()), MeshMaterial3d(plate.clone()), Transform::from_xyz( 0.34, 1.5, 0.0)));
            c.spawn((Mesh3d(bk_arm.clone()),   MeshMaterial3d(plate.clone()), Transform::from_xyz(-0.36, 1.15, 0.0)));
            c.spawn((Mesh3d(bk_arm.clone()),   MeshMaterial3d(plate.clone()), Transform::from_xyz( 0.36, 1.15, 0.0)));
            c.spawn((Mesh3d(bk_leg.clone()),   MeshMaterial3d(plate.clone()), Transform::from_xyz(-0.13, 0.42, 0.0)));
            c.spawn((Mesh3d(bk_leg.clone()),   MeshMaterial3d(plate.clone()), Transform::from_xyz( 0.13, 0.42, 0.0)));
            c.spawn((Mesh3d(bk_head.clone()),  MeshMaterial3d(plate.clone()), Transform::from_xyz(0.0, 1.74, 0.0)));
            c.spawn((Mesh3d(bk_eye.clone()),   MeshMaterial3d(visor.clone()), Transform::from_xyz(0.0, 1.74, -0.15)));
            c.spawn((Mesh3d(bk_horn.clone()),  MeshMaterial3d(trim.clone()),  Transform::from_xyz(-0.10, 1.92, 0.0).with_rotation(Quat::from_rotation_z(0.4))));
            c.spawn((Mesh3d(bk_horn.clone()),  MeshMaterial3d(trim.clone()),  Transform::from_xyz( 0.10, 1.92, 0.0).with_rotation(Quat::from_rotation_z(-0.4))));
            // greatsword held to the side
            c.spawn((Mesh3d(bk_blade.clone()), MeshMaterial3d(trim.clone()),  Transform::from_xyz(0.42, 1.2, 0.18)));
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
                Enemy { health: 2.0, speed: 7.5, flying: true, base_y: p.y, attack_timer: 1.0, knockback_vel: Vec3::ZERO, bob_phase: bi as f32 * 0.7, anim_phase: 0.0, attack_anim: 0.0, moving: false, damage_flash: 0.0 },
                Shock { timer: 0.0 },
            )).with_children(|c| {
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
    mut enemy_q: Query<(&mut Transform, &mut Enemy, Option<&OrcBrute>)>,
    player_q: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut player_vel_q: Query<&mut PlayerVelocity, With<Player>>,
    mut health: ResMut<PlayerHealth>,
) {
    let pt = player_q.single();
    let pp = pt.translation;
    let dt = time.delta_secs();

    for (mut t, mut e, orc) in enemy_q.iter_mut() {
        // Knockback decay
        if e.knockback_vel.length_squared() > 0.01 {
            t.translation += e.knockback_vel * dt;
            e.knockback_vel *= (1.0 - 8.0 * dt).max(0.0);
        }

        let flat = Vec3::new(pp.x - t.translation.x, 0.0, pp.z - t.translation.z);
        let dist = flat.length();
        e.bob_phase += dt * 6.0;
        e.attack_anim = (e.attack_anim - dt).max(0.0);
        e.moving = false;

        if dist < 75.0 && dist > 0.01 {
            let dir = flat / dist;
            let stop = if e.flying { 1.2 } else { 1.8 };
            // Orcs plant their feet while winding up / slamming the hammer
            let rooted = orc.is_some() && e.attack_anim > 0.0;
            if dist > stop && !rooted {
                t.translation += dir * e.speed * dt;
                e.moving = true;
                e.anim_phase += dt * 7.0; // drive walk cycle
            }
            let ty = t.translation.y;
            t.look_at(Vec3::new(pp.x, ty, pp.z), Vec3::Y);

            // Orcs use their own slow hammer-slam (orc_combat); others melee here.
            if orc.is_none() {
                let atk_range = if e.flying { 2.8 } else { 2.2 };
                if dist < atk_range {
                    e.attack_timer -= dt;
                    if e.attack_timer <= 0.0 && health.hurt_timer <= 0.0 && health.iframes <= 0.0 {
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
        }

        // Height: bats hover + bob, ground enemies stay grounded
        if e.flying {
            t.translation.y = e.base_y + e.bob_phase.sin() * 0.4;
        } else {
            t.translation.y = 0.0;
        }
    }
}

// Swing orc legs while walking; raise & slam the hammer-arm on attack.
fn enemy_limb_anim(
    time: Res<Time>,
    enemy_q: Query<(&Enemy, &Children)>,
    mut limb_q: Query<(&mut Transform, &EnemyLimb)>,
) {
    let k = (time.delta_secs() * 12.0).min(1.0);
    for (e, children) in &enemy_q {
        let swing = e.anim_phase.sin();
        for &ch in children.iter() {
            if let Ok((mut tr, limb)) = limb_q.get_mut(ch) {
                let target = if limb.is_arm {
                    if e.attack_anim > 0.0 {
                        // Whole strike stays IN FRONT: raise the hammer up-forward,
                        // then slam it down-forward (positive X keeps the arm ahead).
                        let p = (1.0 - (e.attack_anim / 2.0)).powf(1.6); // ease into a fast slam
                        Quat::from_rotation_x(0.9 + 1.4 * p)
                    } else if e.moving {
                        Quat::from_rotation_x(swing * 0.25 * -limb.side) // gentle sway
                    } else {
                        Quat::from_rotation_x(-0.25) // hammer held ready
                    }
                } else if e.moving {
                    Quat::from_rotation_x(swing * 0.6 * limb.side) // leg stride
                } else {
                    Quat::IDENTITY
                };
                tr.rotation = tr.rotation.slerp(target, k);
            }
        }
    }
}

// Orcs: when in range, start a 2s wind-up; the slam lands near the end,
// dealing 2 hearts + heavy knockback and kicking up dirt debris.
fn orc_combat(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut orcs: Query<(&Transform, &mut Enemy, &mut OrcBrute)>,
    player_q: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut pv: Query<&mut PlayerVelocity, With<Player>>,
    mut health: ResMut<PlayerHealth>,
) {
    let pp = player_q.single().translation;
    for (t, mut e, mut orc) in orcs.iter_mut() {
        let flat = Vec3::new(pp.x - t.translation.x, 0.0, pp.z - t.translation.z);
        let dist = flat.length();
        // Begin a swing when in range and not already swinging
        if dist < 3.2 && e.attack_anim <= 0.0 {
            e.attack_anim = 2.0;
            orc.slammed = false;
        }
        // Slam lands near the end of the wind-up
        if e.attack_anim > 0.0 && e.attack_anim < 0.45 && !orc.slammed {
            orc.slammed = true;
            // Dirt debris kicked up at the orc's feet
            let dirt = materials.add(StandardMaterial {
                base_color: Color::srgb(0.30, 0.22, 0.12), perceptual_roughness: 1.0, ..default() });
            let cube = meshes.add(Cuboid::new(0.18, 0.18, 0.18));
            let foot = t.translation + flat.normalize_or_zero() * 1.0;
            for i in 0..14u32 {
                let a = i as f32 / 14.0 * std::f32::consts::TAU;
                let s = (i as f32 * 12.9).sin().abs();
                // 2x wider spread than before
                let vel = Vec3::new(a.cos() * (4.0 + s * 4.0), 4.5 + s * 3.0, a.sin() * (4.0 + s * 4.0));
                commands.spawn((
                    Mesh3d(cube.clone()), MeshMaterial3d(dirt.clone()),
                    Transform::from_translation(foot + Vec3::Y * 0.2).with_scale(Vec3::splat(0.4 + s * 0.6)),
                    Debris { vel, life: 0.9 },
                ));
            }
            // Damage if the player is still in the blast
            if dist < 4.2 && health.hurt_timer <= 0.0 && health.iframes <= 0.0 {
                health.hearts = (health.hearts - 2).max(0);
                health.hurt_timer = 0.9;
                let mut v = pv.single_mut();
                v.knockback = Vec3::new(flat.x, 0.0, flat.z).normalize_or_zero() * 13.0;
            }
        }
    }
}

fn move_debris(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Transform, &mut Debris)>,
) {
    let dt = time.delta_secs();
    for (e, mut t, mut d) in q.iter_mut() {
        d.vel.y -= 22.0 * dt;
        t.translation += d.vel * dt;
        if t.translation.y < 0.05 { t.translation.y = 0.05; d.vel = Vec3::ZERO; }
        d.life -= dt;
        if d.life <= 0.0 { commands.entity(e).despawn(); }
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
                damage_flash: 0.0, knockback_vel: Vec3::ZERO, anim_phase: i as f32 },
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
            // Legs — each a pivot at the hip so it can swing (thigh+shin+foot hang below)
            for sx in [-0.10f32, 0.10] {
                let side = sx.signum();
                p.spawn((
                    Transform::from_xyz(sx, 0.72, 0.0), GlobalTransform::default(), Visibility::default(),
                    SkeletonLimb { is_arm: false, side },
                )).with_children(|l| {
                    l.spawn((Mesh3d(thigh.clone()), MeshMaterial3d(b.clone()), Transform::from_xyz(0.0, -0.18, 0.0)));
                    l.spawn((Mesh3d(shin.clone()),  MeshMaterial3d(b.clone()), Transform::from_xyz(0.0, -0.54, 0.0)));
                    l.spawn((Mesh3d(foot.clone()),  MeshMaterial3d(b.clone()), Transform::from_xyz(0.0, -0.68, -0.06)));
                });
            }
            // Arms — pivot at the shoulder (upper arm+forearm+hand hang below); shoulder ball is static
            for sx in [-0.24f32, 0.24] {
                let side = sx.signum();
                p.spawn((Mesh3d(shoulder.clone()), MeshMaterial3d(b.clone()), Transform::from_xyz(sx, 1.40, 0.0)));
                p.spawn((
                    Transform::from_xyz(sx, 1.40, 0.0), GlobalTransform::default(), Visibility::default(),
                    SkeletonLimb { is_arm: true, side },
                )).with_children(|a| {
                    a.spawn((Mesh3d(uarm.clone()), MeshMaterial3d(b.clone()), Transform::from_xyz(0.0, -0.18, 0.0)));
                    a.spawn((Mesh3d(farm.clone()), MeshMaterial3d(b.clone()), Transform::from_xyz(side * 0.02, -0.47, 0.0)));
                    a.spawn((Mesh3d(hand.clone()), MeshMaterial3d(b.clone()), Transform::from_xyz(side * 0.03, -0.64, 0.0)));
                });
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
                sk.anim_phase += time.delta_secs() * 6.0;
                let look = t.translation + sk.patrol_dir;
                if (look - t.translation).length_squared() > 0.01 { t.look_at(look, Vec3::Y); }
            }
            SkeletonState::Chase => {
                if dist > 0.1 {
                    let dir = flat.normalize();
                    t.translation += dir * 3.0 * time.delta_secs();
                    t.translation.y = 0.0;
                    sk.anim_phase += time.delta_secs() * 11.0;
                    let ty = t.translation.y; t.look_at(Vec3::new(pp.x, ty, pp.z), Vec3::Y);
                }
            }
            SkeletonState::Attack => {
                let ty = t.translation.y; t.look_at(Vec3::new(pp.x, ty, pp.z), Vec3::Y);
                sk.attack_timer -= time.delta_secs();
                if sk.attack_timer <= 0.0 && health.hurt_timer <= 0.0 && health.iframes <= 0.0 {
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
    mana: Res<Mana>,
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
    if mana.current <= 0.0 { return; } // no mana, no lightning
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
    let red = Color::srgb(0.90, 0.12, 0.12);
    // Hearts row (top-left) — two round lobes + a rounded-bottom body (no rotation = clean)
    commands.spawn(Node {
        position_type: PositionType::Absolute,
        left: Val::Px(16.0), top: Val::Px(16.0),
        flex_direction: FlexDirection::Row, column_gap: Val::Px(8.0),
        ..default()
    }).with_children(|p| {
        for i in 0..5u32 {
            p.spawn(Node { width: Val::Px(38.0), height: Val::Px(34.0), ..default() })
                .with_children(|h| {
                    // left lobe (circle)
                    h.spawn((
                        Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0),
                               width: Val::Px(20.0), height: Val::Px(20.0), ..default() },
                        BorderRadius::all(Val::Px(10.0)),
                        BackgroundColor(red), HeartNode { index: i },
                    ));
                    // right lobe (circle)
                    h.spawn((
                        Node { position_type: PositionType::Absolute, left: Val::Px(18.0), top: Val::Px(0.0),
                               width: Val::Px(20.0), height: Val::Px(20.0), ..default() },
                        BorderRadius::all(Val::Px(10.0)),
                        BackgroundColor(red), HeartNode { index: i },
                    ));
                    // body (rounded bottom forms the heart base)
                    h.spawn((
                        Node { position_type: PositionType::Absolute, left: Val::Px(4.0), top: Val::Px(9.0),
                               width: Val::Px(30.0), height: Val::Px(22.0), ..default() },
                        BorderRadius {
                            top_left: Val::Px(2.0), top_right: Val::Px(2.0),
                            bottom_left: Val::Px(14.0), bottom_right: Val::Px(14.0),
                        },
                        BackgroundColor(red), HeartNode { index: i },
                    ));
                });
        }
    });

    // Stamina bar (green) and Mana bar (blue) under the hearts
    for (top, fill_color, is_stamina) in [
        (66.0f32, Color::srgb(0.2, 0.8, 0.25), true),
        (88.0f32, Color::srgb(0.25, 0.5, 1.0), false),
    ] {
        commands.spawn((
            Node { position_type: PositionType::Absolute, left: Val::Px(16.0), top: Val::Px(top),
                   width: Val::Px(240.0), height: Val::Px(16.0), ..default() },
            BackgroundColor(Color::srgb(0.08, 0.08, 0.10)),
        )).with_children(|p| {
            let mut e = p.spawn((
                Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                BackgroundColor(fill_color),
            ));
            if is_stamina { e.insert(StaminaBar); } else { e.insert(ManaBar); }
        });
    }

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

    // ── Hotbar (Minecraft-style) along the bottom centre ──
    let slots = [
        (ItemKind::Sword,        Color::srgb(0.72, 0.76, 0.82)),
        (ItemKind::Glock,        Color::srgb(0.20, 0.20, 0.23)),
        (ItemKind::Rocket,       Color::srgb(0.90, 0.42, 0.12)),
        (ItemKind::HealthPotion, Color::srgb(0.85, 0.15, 0.15)),
        (ItemKind::ManaPotion,   Color::srgb(0.22, 0.42, 0.95)),
    ];
    commands.spawn(Node {
        position_type: PositionType::Absolute,
        bottom: Val::Px(18.0),
        left: Val::Percent(50.0),
        margin: UiRect::left(Val::Px(-160.0)), // centre the 320px-wide row
        flex_direction: FlexDirection::Row, column_gap: Val::Px(6.0),
        ..default()
    }).with_children(|p| {
        for (kind, color) in slots {
            p.spawn((
                Node {
                    width: Val::Px(56.0), height: Val::Px(56.0),
                    border: UiRect::all(Val::Px(3.0)),
                    justify_content: JustifyContent::Center, align_items: AlignItems::Center,
                    ..default()
                },
                BorderColor(Color::srgb(0.25, 0.25, 0.30)),
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                HotbarSlot { kind },
            )).with_children(|s| {
                s.spawn((
                    Node { width: Val::Px(36.0), height: Val::Px(36.0), ..default() },
                    BorderRadius::all(Val::Px(6.0)),
                    BackgroundColor(color),
                    HotbarIcon { kind, color },
                ));
                // Count label for stackable potions
                if matches!(kind, ItemKind::HealthPotion | ItemKind::ManaPotion) {
                    s.spawn((
                        Node { position_type: PositionType::Absolute, right: Val::Px(3.0), bottom: Val::Px(1.0), ..default() },
                        Text::new("0"),
                        TextFont { font_size: 18.0, ..default() },
                        TextColor(Color::WHITE),
                        HotbarCount { kind },
                    ));
                }
            });
        }
    });

    // ── Boss health bar (hidden until the dragon engages) ──
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(24.0), left: Val::Percent(50.0),
            margin: UiRect::left(Val::Px(-280.0)),
            width: Val::Px(560.0),
            flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(4.0),
            ..default()
        },
        Visibility::Hidden,
        BossBarRoot,
    )).with_children(|p| {
        p.spawn((
            Text::new("Great Dragon"),
            TextFont { font_size: 22.0, ..default() },
            TextColor(Color::srgb(0.85, 0.78, 0.6)),
        ));
        p.spawn((
            Node { width: Val::Px(560.0), height: Val::Px(16.0), border: UiRect::all(Val::Px(2.0)), ..default() },
            BorderColor(Color::srgb(0.15, 0.12, 0.1)),
            BackgroundColor(Color::srgb(0.10, 0.05, 0.05)),
        )).with_children(|b| {
            b.spawn((
                Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                BackgroundColor(Color::srgb(0.7, 0.08, 0.06)),
                BossBarFill,
            ));
        });
    });

    // ── Soapstone message line (bottom-centre, shown when standing on a rune) ──
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(96.0), left: Val::Percent(50.0),
            margin: UiRect::left(Val::Px(-260.0)),
            width: Val::Px(520.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        Text::new(""),
        TextFont { font_size: 26.0, ..default() },
        TextColor(Color::srgb(0.95, 0.85, 0.5)),
        SoapstoneText,
    ));
}

fn update_hearts(
    health: Res<PlayerHealth>,
    mut hearts: Query<(&mut BackgroundColor, &HeartNode)>,
) {
    for (mut bg, h) in hearts.iter_mut() {
        bg.0 = if (h.index as i32) < health.hearts {
            Color::srgb(0.90, 0.12, 0.12)
        } else {
            Color::srgb(0.20, 0.05, 0.05)
        };
    }
}

fn update_bars(
    stamina: Res<Stamina>,
    mana: Res<Mana>,
    mut stam_q: Query<&mut Node, (With<StaminaBar>, Without<ManaBar>)>,
    mut mana_q: Query<&mut Node, (With<ManaBar>, Without<StaminaBar>)>,
) {
    if let Ok(mut n) = stam_q.get_single_mut() {
        n.width = Val::Percent((stamina.current / stamina.max * 100.0).clamp(0.0, 100.0));
    }
    if let Ok(mut n) = mana_q.get_single_mut() {
        n.width = Val::Percent((mana.current / mana.max * 100.0).clamp(0.0, 100.0));
    }
}

// Highlight the selected hotbar slot; dim icons the player doesn't own.
fn update_hotbar(
    inv: Res<Inventory>,
    mut slots: Query<(&mut BorderColor, &HotbarSlot)>,
    mut icons: Query<(&mut BackgroundColor, &HotbarIcon)>,
    mut counts: Query<(&mut Text, &HotbarCount)>,
) {
    for (mut bc, slot) in slots.iter_mut() {
        bc.0 = if slot.kind == inv.selected {
            Color::srgb(1.0, 0.9, 0.2)              // bright = selected
        } else {
            Color::srgb(0.25, 0.25, 0.30)
        };
    }
    for (mut bg, ic) in icons.iter_mut() {
        bg.0 = if inv.owns(ic.kind) { ic.color } else { Color::srgb(0.12, 0.12, 0.14) };
    }
    for (mut text, c) in counts.iter_mut() {
        let n = match c.kind { ItemKind::HealthPotion => inv.health_potions, ItemKind::ManaPotion => inv.mana_potions, _ => 0 };
        *text = Text::new(n.to_string());
    }
}

// Tilt the held potion up to the mouth while drinking.
fn drink_anim(
    time: Res<Time>,
    mut drinking: ResMut<Drinking>,
    mut held_q: Query<(&mut Transform, &HeldVisual)>,
) {
    drinking.timer = (drinking.timer - time.delta_secs()).max(0.0);
    let base_pos = Vec3::new(0.24, -0.20, -0.40);
    // 0 at start/end, 1 at mid-drink
    let tilt = if drinking.timer > 0.0 {
        let s = drinking.timer / 0.7;
        (1.0 - (s - 0.5).abs() * 2.0).max(0.0)
    } else { 0.0 };
    for (mut t, h) in held_q.iter_mut() {
        if matches!(h.kind, ItemKind::HealthPotion | ItemKind::ManaPotion) {
            // Bring the flask up and back toward the player's face, tilting the mouth in
            t.translation = base_pos + Vec3::new(-0.10 * tilt, 0.16 * tilt, 0.18 * tilt);
            t.rotation = Quat::from_rotation_x(1.7 * tilt);
        }
    }
}

fn update_vignette(
    health: Res<PlayerHealth>,
    mut vignette_q: Query<&mut BackgroundColor, With<DamageVignette>>,
) {
    let alpha = if health.hurt_timer > 0.0 { (health.hurt_timer / 0.9 * 0.48).min(0.48) } else { 0.0 };
    for mut bg in vignette_q.iter_mut() {
        bg.0 = Color::srgba(0.95, 0.0, 0.0, alpha);
    }
}

// Shared flash materials (red = sword hit, white/blue = lightning shock).
fn setup_flash_mats(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.insert_resource(FlashMats {
        red:   materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.05, 0.05), emissive: LinearRgba::new(3.0, 0.0, 0.0, 1.0), unlit: true, ..default() }),
        white: materials.add(StandardMaterial { base_color: Color::WHITE, emissive: LinearRgba::new(6.0, 6.0, 6.0, 1.0), unlit: true, ..default() }),
        blue:  materials.add(StandardMaterial { base_color: Color::srgb(0.1, 0.4, 1.0), emissive: LinearRgba::new(0.3, 2.0, 8.0, 1.0), unlit: true, ..default() }),
    });
}

// Record each enemy mesh part's original material so flashes can be undone.
fn tag_body_parts(
    mut commands: Commands,
    roots: Query<Entity, Or<(With<Skeleton>, With<Enemy>)>>,
    children_q: Query<&Children>,
    mat_q: Query<&MeshMaterial3d<StandardMaterial>>,
    tagged: Query<(), With<BodyPart>>,
) {
    for root in &roots {
        let mut stack: Vec<Entity> = vec![root];
        while let Some(e) = stack.pop() {
            if let Ok(ch) = children_q.get(e) { for &c in ch.iter() { stack.push(c); } }
            if e != root && tagged.get(e).is_err() {
                if let Ok(m) = mat_q.get(e) {
                    commands.entity(e).insert(BodyPart { base: m.0.clone() });
                }
            }
        }
    }
}

// Recolour every BodyPart descendant of `root` (or restore its base material).
fn apply_flash(
    root: Entity,
    over: Option<&Handle<StandardMaterial>>,
    children_q: &Query<&Children>,
    mat_q: &mut Query<(&mut MeshMaterial3d<StandardMaterial>, &BodyPart)>,
) {
    let mut stack: Vec<Entity> = vec![root];
    while let Some(e) = stack.pop() {
        if let Ok(ch) = children_q.get(e) { for &c in ch.iter() { stack.push(c); } }
        if let Ok((mut m, part)) = mat_q.get_mut(e) {
            m.0 = match over { Some(h) => h.clone(), None => part.base.clone() };
        }
    }
}

// Pick the flash override for the current timers (shock beats damage).
fn flash_override<'a>(
    shock: f32, dmg: f32, white: bool, f: &'a FlashMats,
) -> Option<&'a Handle<StandardMaterial>> {
    if shock > 0.0 { Some(if white { &f.white } else { &f.blue }) }
    else if dmg > 0.0 { Some(&f.red) }
    else { None }
}

fn flash_skeletons(
    time: Res<Time>,
    flash: Option<Res<FlashMats>>,
    mut skel_q: Query<(Entity, &mut Skeleton, &mut Shock)>,
    children_q: Query<&Children>,
    mut mat_q: Query<(&mut MeshMaterial3d<StandardMaterial>, &BodyPart)>,
) {
    let Some(f) = flash else { return; };
    let white = (time.elapsed_secs() * 18.0) as i64 % 2 == 0;
    for (e, mut sk, mut shock) in skel_q.iter_mut() {
        if sk.state == SkeletonState::Dead { continue; }
        sk.damage_flash = (sk.damage_flash - time.delta_secs()).max(0.0);
        shock.timer = (shock.timer - time.delta_secs()).max(0.0);
        let over = flash_override(shock.timer, sk.damage_flash, white, &f);
        apply_flash(e, over, &children_q, &mut mat_q);
    }
}

fn flash_enemies(
    time: Res<Time>,
    flash: Option<Res<FlashMats>>,
    mut enemy_q: Query<(Entity, &mut Enemy, &mut Shock)>,
    children_q: Query<&Children>,
    mut mat_q: Query<(&mut MeshMaterial3d<StandardMaterial>, &BodyPart)>,
) {
    let Some(f) = flash else { return; };
    let white = (time.elapsed_secs() * 18.0) as i64 % 2 == 0;
    for (e, mut en, mut shock) in enemy_q.iter_mut() {
        en.damage_flash = (en.damage_flash - time.delta_secs()).max(0.0);
        shock.timer = (shock.timer - time.delta_secs()).max(0.0);
        let over = flash_override(shock.timer, en.damage_flash, white, &f);
        apply_flash(e, over, &children_q, &mut mat_q);
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

// Swing skeleton arms & legs in a walk cycle while moving; raise arms when attacking.
fn skeleton_walk_anim(
    time: Res<Time>,
    skel_q: Query<(&Skeleton, &Children)>,
    mut limb_q: Query<(&mut Transform, &SkeletonLimb)>,
) {
    let k = (time.delta_secs() * 12.0).min(1.0);
    for (sk, children) in &skel_q {
        if sk.state == SkeletonState::Dead { continue; }
        // Legs keep striding while patrolling or chasing; arms only swing on patrol —
        // once the skeleton spots you, the arms lock forward to hold/level the spear.
        let legs_move = sk.state == SkeletonState::Patrol || sk.state == SkeletonState::Chase;
        let detected  = sk.state == SkeletonState::Chase || sk.state == SkeletonState::Attack;
        let swing = sk.anim_phase.sin();
        for &ch in children.iter() {
            if let Ok((mut tr, limb)) = limb_q.get_mut(ch) {
                let target = if limb.is_arm {
                    if detected {
                        Quat::from_rotation_x(-1.0)       // arms reach forward, hands stay in view
                    } else if legs_move {
                        Quat::from_rotation_x(swing * 0.5 * -limb.side)
                    } else {
                        Quat::IDENTITY
                    }
                } else if legs_move {
                    Quat::from_rotation_x(swing * 0.7 * limb.side)
                } else {
                    Quat::IDENTITY
                };
                tr.rotation = tr.rotation.slerp(target, k);
            }
        }
    }
}


// Witches fling magic missiles at the player from range.
fn witch_cast(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut witches: Query<(&GlobalTransform, &mut WitchCaster)>,
    player_q: Query<&Transform, With<Player>>,
) {
    let pt = player_q.single();
    let target = pt.translation + Vec3::Y * 1.0;
    for (g, mut w) in witches.iter_mut() {
        let from = g.translation() + Vec3::Y * 1.5;
        let d = target - from;
        let dist = d.length();
        if dist < 55.0 && dist > 3.0 {
            w.cast_timer -= time.delta_secs();
            if w.cast_timer <= 0.0 {
                w.cast_timer = 2.2;
                let m = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.7, 0.2, 1.0),
                    emissive: LinearRgba::new(3.0, 0.5, 6.0, 1.0), unlit: true, ..default() });
                commands.spawn((
                    Mesh3d(meshes.add(Sphere::new(0.22))), MeshMaterial3d(m),
                    Transform::from_translation(from),
                    MagicMissile { velocity: d.normalize() * 24.0, life: 4.0 },
                    PointLight { color: Color::srgb(0.6, 0.2, 1.0), intensity: 40_000.0,
                        range: 8.0, shadows_enabled: false, ..default() },
                ));
            }
        } else {
            w.cast_timer = 1.0;
        }
    }
}

fn move_magic_missiles(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Transform, &mut MagicMissile)>,
    player_q: Query<&Transform, (With<Player>, Without<MagicMissile>)>,
    mut pv: Query<&mut PlayerVelocity, With<Player>>,
    mut health: ResMut<PlayerHealth>,
) {
    let pp = player_q.single().translation + Vec3::Y * 1.0;
    for (e, mut t, mut mm) in q.iter_mut() {
        t.translation += mm.velocity * time.delta_secs();
        mm.life -= time.delta_secs();
        if pp.distance(t.translation) < 1.4 && health.hurt_timer <= 0.0 && health.iframes <= 0.0 {
            health.hearts = (health.hearts - 1).max(0);
            health.hurt_timer = 0.9;
            let mut v = pv.single_mut();
            let dir = (pp - t.translation).normalize_or_zero();
            v.knockback = Vec3::new(dir.x, 0.0, dir.z) * 6.0;
            commands.entity(e).despawn();
        } else if mm.life <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

// Rockets — slow; explode on contact for area damage + a nuke mushroom cloud.
fn move_rockets(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rockets: Query<(Entity, &mut Transform, &mut Rocket)>,
    mut skel: Query<(Entity, &GlobalTransform, &mut Skeleton)>,
    mut enemy: Query<(Entity, &GlobalTransform, &mut Enemy)>,
    mut dragon: Query<(Entity, &GlobalTransform, &mut Dragon)>,
) {
    let dt = time.delta_secs();
    for (re, mut rt, mut rk) in rockets.iter_mut() {
        rt.translation += rk.velocity * dt;
        rk.life -= dt;
        let pos = rt.translation;
        let mut explode = rk.life <= 0.0 || pos.y <= 0.2; // also detonate on the ground
        if !explode {
            explode = skel.iter().any(|(_, g, s)| s.state != SkeletonState::Dead && g.translation().distance(pos) < 2.0)
                   || enemy.iter().any(|(_, g, _)| g.translation().distance(pos) < 2.0)
                   || dragon.iter().any(|(_, g, d)| d.state != DragonState::Dead && g.translation().distance(pos) < 4.0);
        }
        if explode {
            let r = 8.0; // blast radius
            for (e, g, mut s) in skel.iter_mut() {
                if s.state != SkeletonState::Dead && g.translation().distance(pos) < r {
                    s.health -= 20.0; s.state = SkeletonState::Dead; commands.entity(e).despawn_recursive();
                }
            }
            for (e, g, mut en) in enemy.iter_mut() {
                if g.translation().distance(pos) < r {
                    en.health -= 20.0; if en.health <= 0.0 { commands.entity(e).despawn_recursive(); }
                }
            }
            for (e, g, mut d) in dragon.iter_mut() {
                if d.state != DragonState::Dead && g.translation().distance(pos) < r {
                    d.health -= 6.0; d.damage_flash = 0.25;
                    if d.health <= 0.0 { d.state = DragonState::Dead; commands.entity(e).despawn_recursive(); }
                }
            }
            spawn_mushroom(&mut commands, &mut meshes, &mut materials, Vec3::new(pos.x, 0.0, pos.z));
            commands.entity(re).despawn_recursive();
        }
    }
}

// Build a stylized nuke mushroom cloud (stem + cap + ground flash) that grows & fades.
fn spawn_mushroom(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    base: Vec3,
) {
    let fire = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.6, 0.1), emissive: LinearRgba::new(6.0, 2.5, 0.3, 1.0), unlit: true, ..default() });
    let smoke = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.35, 0.2), emissive: LinearRgba::new(1.2, 0.7, 0.3, 1.0), unlit: true, ..default() });
    let flash = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.95, 0.7), emissive: LinearRgba::new(9.0, 7.0, 3.0, 1.0), unlit: true, ..default() });

    commands.spawn((
        Transform::from_translation(base), GlobalTransform::default(), Visibility::default(),
        Mushroom { age: 0.0 }, Transient { life: 1.6 },
        PointLight { color: Color::srgb(1.0, 0.6, 0.2), intensity: 800_000.0, range: 40.0, shadows_enabled: false, ..default() },
    )).with_children(|m| {
        // ground flash
        m.spawn((Mesh3d(meshes.add(Sphere::new(3.0))), MeshMaterial3d(flash),
            Transform::from_xyz(0.0, 0.5, 0.0)));
        // rising stem
        m.spawn((Mesh3d(meshes.add(Cylinder { radius: 1.4, half_height: 4.0 })), MeshMaterial3d(smoke.clone()),
            Transform::from_xyz(0.0, 4.0, 0.0)));
        // billowing cap
        m.spawn((Mesh3d(meshes.add(Sphere::new(3.6))), MeshMaterial3d(fire.clone()),
            Transform::from_xyz(0.0, 8.5, 0.0)));
        m.spawn((Mesh3d(meshes.add(Sphere::new(2.6))), MeshMaterial3d(smoke),
            Transform::from_xyz(0.0, 9.8, 0.0)));
        // fiery core
        m.spawn((Mesh3d(meshes.add(Sphere::new(2.0))), MeshMaterial3d(fire),
            Transform::from_xyz(0.0, 8.5, 0.0)));
    });
}

// Walk over a Pickup to collect it.
fn pickup_system(
    mut commands: Commands,
    mut inv: ResMut<Inventory>,
    player_q: Query<&Transform, With<Player>>,
    pickups: Query<(Entity, &GlobalTransform, &Pickup)>,
) {
    let pp = player_q.single().translation;
    for (e, g, pk) in &pickups {
        if g.translation().distance(pp) < 2.4 {
            match pk.kind {
                ItemKind::HealthPotion => inv.health_potions += 1,
                ItemKind::ManaPotion   => inv.mana_potions += 1,
                ItemKind::Glock  => inv.has_glock = true,
                ItemKind::Rocket => inv.has_rocket = true,
                ItemKind::Sword  => {}
            }
            commands.entity(e).despawn_recursive();
        }
    }
}

// Scatter potions in the field and place the firearms inside the castle.
fn spawn_items(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let red_glass  = materials.add(StandardMaterial { base_color: Color::srgb(0.9, 0.15, 0.15), emissive: LinearRgba::new(1.4, 0.15, 0.15, 1.0), ..default() });
    let blue_glass = materials.add(StandardMaterial { base_color: Color::srgb(0.2, 0.4, 0.95), emissive: LinearRgba::new(0.3, 0.8, 1.8, 1.0), ..default() });
    let cork  = materials.add(StandardMaterial { base_color: Color::srgb(0.4, 0.25, 0.1), ..default() });
    let metal = materials.add(StandardMaterial { base_color: Color::srgb(0.12, 0.12, 0.14), metallic: 0.7, perceptual_roughness: 0.4, ..default() });

    // Potions scattered around the map — red = health, blue = mana
    let bottle = meshes.add(Cylinder { radius: 0.18, half_height: 0.28 });
    let neck   = meshes.add(Cylinder { radius: 0.07, half_height: 0.14 });
    let stopper= meshes.add(Cuboid::new(0.14, 0.10, 0.14));
    let potion_spots = [
        (Vec3::new(12.0, 0.0, -4.0), true), (Vec3::new(-18.0, 0.0, 8.0), false),
        (Vec3::new(40.0, 0.0, -30.0), true), (Vec3::new(-55.0, 0.0, -20.0), false),
        (Vec3::new(0.0, 0.0, -110.0), true), (Vec3::new(80.0, 0.0, 60.0), false),
        (Vec3::new(-90.0, 0.0, 70.0), true), (Vec3::new(20.0, 0.0, -80.0), false),
        (Vec3::new(-30.0, 0.0, -60.0), true), (Vec3::new(60.0, 0.0, 20.0), false),
    ];
    for (p, is_health) in potion_spots {
        let glass = if is_health { red_glass.clone() } else { blue_glass.clone() };
        let kind = if is_health { ItemKind::HealthPotion } else { ItemKind::ManaPotion };
        commands.spawn((
            Transform::from_translation(p + Vec3::Y * 0.3), GlobalTransform::default(), Visibility::default(),
            Pickup { kind },
        )).with_children(|c| {
            c.spawn((Mesh3d(bottle.clone()),  MeshMaterial3d(glass.clone()), Transform::from_xyz(0.0, 0.0, 0.0)));
            c.spawn((Mesh3d(neck.clone()),    MeshMaterial3d(glass.clone()), Transform::from_xyz(0.0, 0.34, 0.0)));
            c.spawn((Mesh3d(stopper.clone()), MeshMaterial3d(cork.clone()),  Transform::from_xyz(0.0, 0.50, 0.0)));
            let light = if is_health { Color::srgb(1.0, 0.18, 0.18) } else { Color::srgb(0.3, 0.55, 1.0) };
            c.spawn((PointLight { color: light, intensity: 200_000.0, range: 15.0, radius: 0.3, shadows_enabled: false, ..default() },
                Transform::from_xyz(0.0, 0.4, 0.0)));
        });
    }

    // Glock + rocket launcher inside the castle courtyard
    commands.spawn((
        Transform::from_xyz(14.0, 0.8, -78.0), GlobalTransform::default(), Visibility::default(),
        Pickup { kind: ItemKind::Glock },
    )).with_children(|c| {
        c.spawn((Mesh3d(meshes.add(Cuboid::new(0.18, 0.26, 0.6))), MeshMaterial3d(metal.clone()), Transform::from_xyz(0.0, 0.0, 0.0)));
        c.spawn((Mesh3d(meshes.add(Cuboid::new(0.16, 0.36, 0.2))), MeshMaterial3d(metal.clone()), Transform::from_xyz(0.0, -0.3, 0.18)));
        c.spawn((PointLight { color: Color::srgb(1.0, 0.85, 0.4), intensity: 200_000.0, range: 15.0, radius: 0.3, shadows_enabled: false, ..default() },
            Transform::from_xyz(0.0, 0.5, 0.0)));
    });
    commands.spawn((
        Transform::from_xyz(-14.0, 1.0, -78.0), GlobalTransform::default(), Visibility::default(),
        Pickup { kind: ItemKind::Rocket },
    )).with_children(|c| {
        c.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.22, half_height: 1.1 })), MeshMaterial3d(metal.clone()),
            Transform::from_xyz(0.0, 0.0, 0.0).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))));
        c.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.30, half_height: 0.22 })), MeshMaterial3d(metal.clone()),
            Transform::from_xyz(0.0, 0.0, 1.0).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))));
        c.spawn((PointLight { color: Color::srgb(1.0, 0.85, 0.4), intensity: 200_000.0, range: 15.0, radius: 0.3, shadows_enabled: false, ..default() },
            Transform::from_xyz(0.0, 0.5, 0.0)));
    });
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
    let fb_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.45, 0.0),
        emissive: LinearRgba::new(4.0, 1.5, 0.0, 1.0), unlit: true, ..default()
    });
    let fb_mesh = meshes.add(Sphere::new(0.56)); // 2× bigger fireballs
    commands.insert_resource(DragonAssets { fb_mat, fb_mesh });

    // Build the dragon as a parts list (mesh, material, transform).
    // Local convention: forward = -Z (head points -Z), tail at +Z.
    let cone = |r: f32, h: f32, res: u32, m: &mut Assets<Mesh>| m.add(Cone { radius: r, height: h }.mesh().resolution(res));
    let q_x = |deg: f32| Quat::from_rotation_x(deg.to_radians());
    let mut parts: Vec<(Handle<Mesh>, Handle<StandardMaterial>, Transform)> = Vec::new();

    // ── Lean, sinewy torso (slimmer than before) ──
    parts.push((meshes.add(Cuboid::new(1.9, 1.8, 2.6)), scale.clone(), Transform::from_xyz(0.0, 2.7, -0.9)));
    parts.push((meshes.add(Cuboid::new(2.1, 2.0, 2.8)), scale.clone(), Transform::from_xyz(0.0, 2.5, 1.4)));
    parts.push((meshes.add(Cuboid::new(1.4, 1.0, 2.0)), scale_dark.clone(), Transform::from_xyz(0.0, 1.55, 0.6))); // chest underside
    // Glowing molten underbelly seam
    parts.push((meshes.add(Cuboid::new(0.55, 0.16, 4.8)), maw.clone(), Transform::from_xyz(0.0, 1.4, 0.3)));

    // ── Long sinuous neck rising to the head ──
    parts.push((meshes.add(Cuboid::new(1.1, 1.25, 1.4)), scale.clone(), Transform::from_xyz(0.0, 3.7, -2.2).with_rotation(q_x(-26.0))));
    parts.push((meshes.add(Cuboid::new(0.95, 1.1, 1.3)), scale.clone(), Transform::from_xyz(0.0, 4.5, -3.2).with_rotation(q_x(-36.0))));
    parts.push((meshes.add(Cuboid::new(0.85, 0.95, 1.2)), scale.clone(), Transform::from_xyz(0.0, 5.1, -4.1).with_rotation(q_x(-18.0))));

    // ── Scary angular head ──
    parts.push((meshes.add(Cuboid::new(1.05, 0.85, 1.5)), scale.clone(), Transform::from_xyz(0.0, 5.3, -5.1)));           // skull
    parts.push((meshes.add(Cuboid::new(0.66, 0.48, 1.2)), scale.clone(), Transform::from_xyz(0.0, 5.12, -6.1)));          // upper snout
    parts.push((meshes.add(Cuboid::new(0.46, 0.36, 0.5)), scale.clone(), Transform::from_xyz(0.0, 5.05, -6.7)));          // snout tip
    parts.push((meshes.add(Cuboid::new(0.78, 0.30, 1.5)), scale_dark.clone(), Transform::from_xyz(0.0, 4.78, -5.85)));    // lower jaw
    parts.push((meshes.add(Cuboid::new(0.55, 0.26, 1.0)), maw.clone(), Transform::from_xyz(0.0, 4.98, -5.95)));           // glowing maw
    // Angular brow ridges (angled wedges over the eyes)
    for sx in [-1.0f32, 1.0] {
        parts.push((meshes.add(Cuboid::new(0.5, 0.16, 0.7)), scale_dark.clone(),
            Transform::from_xyz(sx * 0.42, 5.62, -5.6).with_rotation(Quat::from_rotation_z(sx * -0.4))));
        // glowing slit eye
        parts.push((meshes.add(Cuboid::new(0.30, 0.16, 0.22)), eye.clone(),
            Transform::from_xyz(sx * 0.44, 5.42, -5.78).with_rotation(Quat::from_rotation_z(sx * -0.3))));
        // nostril ember
        parts.push((meshes.add(Cuboid::new(0.09, 0.09, 0.09)), maw.clone(),
            Transform::from_xyz(sx * 0.18, 5.18, -6.7)));
    }
    // ── A crown of horns (scarier: big swept pair + extra spikes) ──
    for sx in [-1.0f32, 1.0] {
        parts.push((cone(0.34, 3.2, 7, &mut meshes), horn.clone(),
            Transform::from_xyz(sx * 0.55, 5.9, -4.5).with_rotation(q_x(62.0) * Quat::from_rotation_z(sx * 0.18))));
        parts.push((cone(0.20, 1.6, 6, &mut meshes), horn.clone(),
            Transform::from_xyz(sx * 0.8, 5.5, -5.0).with_rotation(q_x(38.0) * Quat::from_rotation_z(sx * 0.45))));
        parts.push((cone(0.12, 0.8, 5, &mut meshes), horn.clone(),
            Transform::from_xyz(sx * 0.62, 4.9, -5.6).with_rotation(Quat::from_rotation_z(sx * 1.3)))); // cheek spike
        parts.push((cone(0.09, 0.55, 5, &mut meshes), horn.clone(),
            Transform::from_xyz(sx * 0.30, 5.75, -5.2).with_rotation(q_x(48.0)))); // brow spike
    }
    // ── Fangs (jutting from upper & lower jaw) ──
    for i in 0..7u32 {
        let tx = -0.34 + i as f32 * 0.113;
        parts.push((cone(0.05, 0.34, 4, &mut meshes), teeth.clone(),
            Transform::from_xyz(tx, 4.95, -6.35).with_rotation(q_x(180.0))));
        parts.push((cone(0.045, 0.26, 4, &mut meshes), teeth.clone(),
            Transform::from_xyz(tx, 4.7, -6.1)));
    }

    // ── Lean legs + clawed feet ──
    for (lx, lz) in [(-1.0f32, -0.9f32), (1.0, -0.9), (-1.05, 1.9), (1.05, 1.9)] {
        parts.push((meshes.add(Cuboid::new(0.5, 1.2, 0.5)), scale.clone(), Transform::from_xyz(lx, 1.05, lz)));
        parts.push((meshes.add(Cuboid::new(0.38, 1.0, 0.38)), scale.clone(), Transform::from_xyz(lx, 0.4, lz - 0.12)));
        parts.push((meshes.add(Cuboid::new(0.5, 0.2, 0.85)), scale_dark.clone(), Transform::from_xyz(lx, 0.1, lz - 0.35)));
        for cxo in [-0.15f32, 0.0, 0.15] {
            parts.push((cone(0.05, 0.22, 4, &mut meshes), teeth.clone(),
                Transform::from_xyz(lx + cxo, 0.1, lz - 0.78).with_rotation(q_x(-90.0))));
        }
    }

    // ── Long tapering tail with a bladed tip ──
    let tail: &[(f32, f32, f32, f32, f32)] = &[
        (1.0, 1.0, 1.7, 3.2, 2.2),
        (0.82, 0.82, 1.6, 4.6, 2.0),
        (0.64, 0.64, 1.5, 5.9, 1.8),
        (0.46, 0.46, 1.4, 7.1, 1.6),
        (0.3, 0.3, 1.3, 8.2, 1.5),
    ];
    for &(tw, th, tl, tz, ty) in tail {
        parts.push((meshes.add(Cuboid::new(tw, th, tl)), scale.clone(), Transform::from_xyz(0.0, ty, tz)));
    }
    parts.push((cone(0.30, 1.3, 5, &mut meshes), horn.clone(),
        Transform::from_xyz(0.0, 1.5, 9.2).with_rotation(q_x(-90.0))));

    // ── Tall sharp spine ridge from neck to tail ──
    for i in 0..16u32 {
        let z = -3.8 + i as f32 * 0.82;
        let y = 3.9 - (i as f32 * 0.15);
        parts.push((cone(0.14, 0.7, 5, &mut meshes), scale_dark.clone(),
            Transform::from_xyz(0.0, y.max(1.7), z)));
    }

    // Dragon faces the gate (rotate 180° so local -Z → world +Z), scaled 1.35×.
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, -54.0)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI))
            .with_scale(Vec3::splat(1.35)),
        GlobalTransform::default(), Visibility::default(),
        Dragon { health: 40.0, max_health: 40.0, damage_flash: 0.0, state: DragonState::Idle,
                 enraged: false, timer: 0.0, shock_left: 0, shock_timer: 0.0,
                 fireball_timer: 4.0, fly_angle: 0.0, breath_timer: 6.0,
                 breath_target: Vec3::ZERO, fire_timer: 0.0 },
        Shock { timer: 0.0 },
    )).with_children(|p| {
        for (mesh, mat, tf) in parts {
            p.spawn((
                Mesh3d(mesh), MeshMaterial3d(mat.clone()), tf,
                Visibility::default(), DragonPart { base: mat },
            ));
        }
        // ── Big membrane wings as flap-able pivots (pivot at the shoulder) ──
        for s in [-1.0f32, 1.0] {
            p.spawn((
                Transform::from_xyz(s * 1.0, 3.9, -0.3),
                GlobalTransform::default(), Visibility::default(),
                DragonWing { side: s },
            )).with_children(|w| {
                // leading-edge arm bone
                w.spawn((Mesh3d(meshes.add(Cuboid::new(7.2, 0.24, 0.34))), MeshMaterial3d(scale.clone()),
                    Transform::from_xyz(s * 3.6, 0.25, -0.7), Visibility::default(), DragonPart { base: scale.clone() }));
                // big membrane
                w.spawn((Mesh3d(meshes.add(Cuboid::new(7.0, 0.08, 4.6))), MeshMaterial3d(scale_dark.clone()),
                    Transform::from_xyz(s * 3.5, 0.0, 1.0), Visibility::default(), DragonPart { base: scale_dark.clone() }));
                // wing finger struts
                for fz in [-0.4f32, 1.2, 2.6] {
                    w.spawn((Mesh3d(meshes.add(Cuboid::new(6.0, 0.10, 0.14))), MeshMaterial3d(scale.clone()),
                        Transform::from_xyz(s * 3.0, 0.04, fz).with_rotation(Quat::from_rotation_z(s * 0.10)),
                        Visibility::default(), DragonPart { base: scale.clone() }));
                }
                // claw at the wing tip
                w.spawn((Mesh3d(cone(0.12, 0.7, 5, &mut meshes)), MeshMaterial3d(horn.clone()),
                    Transform::from_xyz(s * 6.9, 0.1, -1.0).with_rotation(Quat::from_rotation_z(s * -1.4)),
                    Visibility::default(), DragonPart { base: horn.clone() }));
            });
        }
    });
}

fn dragon_ai(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut dragon_q: Query<(Entity, &mut Transform, &mut Dragon)>,
    player_q: Query<&Transform, (With<Player>, Without<Dragon>)>,
    mut slow: ResMut<MoveSlow>,
    assets: Option<Res<DragonAssets>>,
) {
    let Some(assets) = assets else { return; };
    let pt = player_q.single();
    let pp = pt.translation + Vec3::Y * 1.5;
    let dt = time.delta_secs();
    let et = time.elapsed_secs();

    for (entity, mut t, mut dragon) in dragon_q.iter_mut() {
        if dragon.state == DragonState::Dead { continue; }
        let to = pp - (t.translation + Vec3::Y * 3.0);
        let dist = to.length();

        if dragon.state == DragonState::Idle {
            if dist < 90.0 { dragon.state = DragonState::Ground; } else { continue; }
        }

        // ── Enrage trigger at 50% HP ──
        if !dragon.enraged && dragon.health <= dragon.max_health * 0.5 {
            dragon.enraged = true;
            dragon.state = DragonState::Roar;
            dragon.timer = 3.2;
            dragon.shock_left = 3;
            dragon.shock_timer = 0.0;
            // Faint red aura (persists)
            let aura = materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.12, 0.06, 0.14),
                emissive: LinearRgba::new(1.4, 0.15, 0.05, 1.0),
                unlit: true, alpha_mode: AlphaMode::Blend, ..default() });
            commands.entity(entity).with_children(|c| {
                c.spawn((Mesh3d(meshes.add(Sphere::new(7.0))), MeshMaterial3d(aura),
                    Transform::from_xyz(0.0, 3.0, 1.0), EnrageAura));
            });
            dragon.fire_timer = 0.0; // reuse as the sound-wave emit throttle during the roar
        }

        match dragon.state {
            DragonState::Ground => {
                // Face the player and lob ONE big fireball
                let ty = t.translation.y;
                t.look_at(Vec3::new(pp.x, ty, pp.z), Vec3::Y);
                dragon.fireball_timer -= dt;
                if dragon.fireball_timer <= 0.0 {
                    dragon.fireball_timer = 3.0;
                    let fwd = t.rotation * Vec3::NEG_Z;
                    let fire_pos = t.translation + Vec3::Y * 4.8 + fwd * 6.0;
                    let dir = to.normalize_or_zero();
                    commands.spawn((
                        Mesh3d(assets.fb_mesh.clone()), MeshMaterial3d(assets.fb_mat.clone()),
                        Transform::from_translation(fire_pos),
                        Fireball { velocity: dir * 16.0, life: 8.0 },
                        PointLight { color: Color::srgb(1.0, 0.4, 0.0), intensity: 120_000.0,
                            range: 16.0, shadows_enabled: false, ..default() },
                    ));
                }
            }
            DragonState::Roar => {
                // Rear up and roar at the sky; slow the player while it lasts
                let ty = t.translation.y;
                t.look_at(Vec3::new(pp.x, ty + 9.0, pp.z), Vec3::Y);
                slow.timer = 0.2;
                // White sound-wave rings billowing out of the mouth
                dragon.fire_timer -= dt;
                if dragon.fire_timer <= 0.0 {
                    dragon.fire_timer = 0.35;
                    let fwd = t.rotation * Vec3::NEG_Z;
                    let mouth = t.translation + t.rotation * (Vec3::new(0.0, 5.0, -6.2) * t.scale.x);
                    let wave_mat = materials.add(StandardMaterial {
                        base_color: Color::srgba(1.0, 1.0, 1.0, 0.6),
                        emissive: LinearRgba::new(2.5, 2.5, 2.8, 1.0),
                        unlit: true, alpha_mode: AlphaMode::Blend, ..default() });
                    commands.spawn((
                        Mesh3d(meshes.add(Annulus::new(0.55, 0.7))), MeshMaterial3d(wave_mat),
                        Transform::from_translation(mouth)
                            .with_rotation(Quat::from_rotation_arc(Vec3::Z, fwd))
                            .with_scale(Vec3::splat(0.6)),
                        SoundWave, Transient { life: 1.1 },
                    ));
                }
                dragon.shock_timer -= dt;
                if dragon.shock_left > 0 && dragon.shock_timer <= 0.0 {
                    dragon.shock_left -= 1;
                    dragon.shock_timer = 1.0;
                    let origin = Vec3::new(t.translation.x, 0.1, t.translation.z);
                    let ring = materials.add(StandardMaterial {
                        base_color: Color::srgb(1.0, 0.25, 0.05),
                        emissive: LinearRgba::new(7.0, 1.6, 0.2, 1.0), unlit: true, ..default() });
                    commands.spawn((
                        Mesh3d(meshes.add(Annulus::new(0.9, 1.0))), MeshMaterial3d(ring),
                        Transform::from_translation(origin)
                            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                            .with_scale(Vec3::splat(2.0)),
                        Shockwave { radius: 2.0, origin, hit: false },
                        PointLight { color: Color::srgb(1.0, 0.35, 0.1), intensity: 600_000.0,
                            range: 40.0, shadows_enabled: false, ..default() },
                    ));
                }
                dragon.timer -= dt;
                if dragon.timer <= 0.0 {
                    dragon.state = DragonState::Takeoff;
                    dragon.timer = 1.4;
                    // Downdraft dust from the wing-beat takeoff
                    let dust = materials.add(StandardMaterial {
                        base_color: Color::srgb(0.35, 0.30, 0.22), perceptual_roughness: 1.0, ..default() });
                    let cube = meshes.add(Cuboid::new(0.25, 0.25, 0.25));
                    for i in 0..18u32 {
                        let a = i as f32 / 18.0 * std::f32::consts::TAU;
                        let s = (i as f32 * 7.1).sin().abs();
                        commands.spawn((
                            Mesh3d(cube.clone()), MeshMaterial3d(dust.clone()),
                            Transform::from_xyz(t.translation.x + a.cos() * 3.0, 0.3, t.translation.z + a.sin() * 3.0),
                            Debris { vel: Vec3::new(a.cos() * (6.0 + s * 4.0), 2.0 + s * 2.0, a.sin() * (6.0 + s * 4.0)), life: 1.0 },
                        ));
                    }
                }
            }
            DragonState::Takeoff => {
                // Beat the wings and rise off the ground like a real dragon
                t.translation.y += dt * 9.0;
                if t.translation.y > 12.0 { t.translation.y = 12.0; }
                let ty = t.translation.y;
                t.look_at(Vec3::new(pp.x, ty, pp.z), Vec3::Y);
                dragon.timer -= dt;
                if dragon.timer <= 0.0 {
                    dragon.state = DragonState::Fly;
                    dragon.breath_timer = 5.0 + et.sin().abs() * 7.0;
                }
            }
            DragonState::Fly => {
                // Soar high and erratically around the player (a touch slower)
                dragon.fly_angle += dt * (0.55 + (et * 0.9).sin() * 0.35);
                let r = 50.0 + (et * 1.3).sin() * 8.0;
                let wob = (et * 1.7).sin() * 7.0;
                let target = Vec3::new(pp.x + dragon.fly_angle.cos() * r, 26.0 + wob, pp.z + dragon.fly_angle.sin() * r);
                let prev = t.translation;
                t.translation = t.translation.lerp(target, dt * 1.0);
                let vel = t.translation - prev;
                if vel.length_squared() > 1e-4 { let ahead = t.translation + vel; t.look_at(ahead, Vec3::Y); }
                dragon.breath_timer -= dt;
                if dragon.breath_timer <= 0.0 {
                    // Lock the breath onto where the player is NOW, then hover there —
                    // the beam sweeps toward this fixed point so you can run out of it.
                    dragon.state = DragonState::Breath;
                    dragon.timer = 3.0;
                    dragon.fire_timer = 0.0;
                    dragon.breath_target = pt.translation;
                }
            }
            DragonState::Breath => {
                // Hover (gently) over the fixed strike point and unleash the laser
                let bt = dragon.breath_target;
                let target = Vec3::new(bt.x, 19.0, bt.z - 8.0);
                t.translation = t.translation.lerp(target, dt * 1.4);
                t.look_at(Vec3::new(bt.x, 0.5, bt.z), Vec3::Y);
                dragon.timer -= dt;
                if dragon.timer <= 0.0 {
                    dragon.state = DragonState::Fly;
                    dragon.breath_timer = 5.0 + (et * 1.7).cos().abs() * 7.0;
                }
            }
            _ => {}
        }

        if dragon.health <= 0.0 {
            dragon.state = DragonState::Dead;
            // Burst apart mid-air like fireworks
            let center = t.translation + Vec3::Y * 4.0 * t.scale.x;
            let colors = [
                Color::srgb(1.0, 0.3, 0.2), Color::srgb(1.0, 0.8, 0.2),
                Color::srgb(0.3, 0.7, 1.0), Color::srgb(0.7, 0.4, 1.0),
                Color::srgb(0.4, 1.0, 0.5), Color::srgb(1.0, 1.0, 1.0),
            ];
            let mats: Vec<Handle<StandardMaterial>> = colors.iter().map(|c| {
                let lin = c.to_linear();
                materials.add(StandardMaterial { base_color: *c,
                    emissive: LinearRgba::new(lin.red * 7.0, lin.green * 7.0, lin.blue * 7.0, 1.0),
                    unlit: true, ..default() })
            }).collect();
            let spark = meshes.add(Sphere::new(0.32));
            for i in 0..60u32 {
                let a = i as f32 * 2.39996;          // golden-angle spray
                let el = (i as f32 * 0.7).sin();
                let dir = Vec3::new(a.cos(), 0.6 + el.abs(), a.sin()).normalize();
                let spd = 9.0 + (i % 5) as f32 * 2.5;
                commands.spawn((
                    Mesh3d(spark.clone()), MeshMaterial3d(mats[i as usize % mats.len()].clone()),
                    Transform::from_translation(center),
                    Debris { vel: dir * spd, life: 1.6 },
                ));
            }
            // Bright flash bursts
            for _ in 0..2 {
                commands.spawn((
                    Mesh3d(meshes.add(Sphere::new(2.2))),
                    MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::WHITE,
                        emissive: LinearRgba::new(9.0, 7.0, 4.0, 1.0), unlit: true, ..default() })),
                    Transform::from_translation(center),
                    Transient { life: 0.35 },
                    PointLight { color: Color::srgb(1.0, 0.8, 0.5), intensity: 1_500_000.0, range: 50.0, shadows_enabled: false, ..default() },
                ));
            }
            commands.entity(entity).despawn_recursive();
        }
    }
}

// Dragonbreath: a continuous red beam with a yellow core that starts pointing
// down and slowly sweeps toward a FIXED strike point (so you can run out of it).
// Standing in the beam is near-instant death, and it sets the ground ablaze.
fn dragon_breath(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut dragon_q: Query<(&Transform, &mut Dragon), Without<DragonLaser>>,
    mut laser_q: Query<(Entity, &mut Transform), (With<DragonLaser>, Without<Dragon>, Without<Player>)>,
    player_q: Query<&Transform, (With<Player>, Without<Dragon>, Without<DragonLaser>)>,
    mut health: ResMut<PlayerHealth>,
) {
    let mut breathing = dragon_q.iter_mut().find(|(_, d)| d.state == DragonState::Breath);
    let Some((dtf, ref mut d)) = breathing else {
        for (e, _) in laser_q.iter() { commands.entity(e).despawn_recursive(); }
        return;
    };
    let head = dtf.translation + dtf.rotation * (Vec3::new(0.0, 5.0, -6.2) * dtf.scale.x);
    // Aim at the fixed strike point on the ground, not the live player
    let aim = d.breath_target + Vec3::Y * 0.5;

    // Sweep: straight down → toward the strike point over the breath
    let prog = ((3.0 - d.timer) / 3.0).clamp(0.0, 1.0);
    let to_aim = (aim - head).normalize_or_zero();
    let dir = Vec3::NEG_Y.lerp(to_aim, prog).normalize_or_zero();
    let up = if dir.y.abs() > 0.95 { Vec3::Z } else { Vec3::Y };

    if laser_q.is_empty() {
        let outer = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.05, 0.0), emissive: LinearRgba::new(10.0, 0.5, 0.0, 1.0), unlit: true, ..default() });
        let core = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 1.0, 0.4), emissive: LinearRgba::new(12.0, 11.0, 3.0, 1.0), unlit: true, ..default() });
        commands.spawn((
            Transform::from_translation(head).looking_to(dir, up),
            GlobalTransform::default(), Visibility::default(), DragonLaser,
            PointLight { color: Color::srgb(1.0, 0.3, 0.1), intensity: 300_000.0, range: 30.0, shadows_enabled: false, ..default() },
        )).with_children(|l| {
            l.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.7, half_height: 30.0 })), MeshMaterial3d(outer),
                Transform::from_xyz(0.0, 0.0, -30.0).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))));
            l.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.32, half_height: 30.0 })), MeshMaterial3d(core),
                Transform::from_xyz(0.0, 0.0, -30.0).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))));
        });
    }
    for (_, mut lt) in laser_q.iter_mut() {
        *lt = Transform::from_translation(head).looking_to(dir, up);
    }

    // Where the beam meets the ground → lay a lingering fire patch there
    if dir.y < -0.05 {
        let hit_t = -head.y / dir.y;
        let ground = head + dir * hit_t;
        d.fire_timer -= time.delta_secs();
        if d.fire_timer <= 0.0 {
            d.fire_timer = 0.18;
            let fire = materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.35, 0.05), emissive: LinearRgba::new(6.0, 1.6, 0.2, 1.0),
                unlit: true, alpha_mode: AlphaMode::Add, ..default() });
            commands.spawn((
                Mesh3d(meshes.add(Cylinder { radius: 2.2, half_height: 0.1 })), MeshMaterial3d(fire),
                Transform::from_xyz(ground.x, 0.12, ground.z),
                FirePatch { life: 5.0 },
                PointLight { color: Color::srgb(1.0, 0.4, 0.1), intensity: 120_000.0, range: 10.0, shadows_enabled: false, ..default() },
            ));
        }
    }

    // Death damage if you're standing in the beam itself
    let pp = player_q.single().translation + Vec3::Y * 0.9;
    let to = pp - head;
    let proj = dir.dot(to);
    let perp = (to - dir * proj).length();
    if proj > 0.0 && proj < 60.0 && perp < 1.7 && health.hurt_timer <= 0.0 && health.iframes <= 0.0 {
        health.hearts = (health.hearts - 3).max(0);
        health.hurt_timer = 0.4;
    }
}

// Ground fire left by the dragonbreath: burns the player for 5s then fades.
fn update_fire_patches(
    time: Res<Time>,
    mut commands: Commands,
    mut patches: Query<(Entity, &mut FirePatch, &GlobalTransform)>,
    player_q: Query<&Transform, With<Player>>,
    mut health: ResMut<PlayerHealth>,
) {
    let pp = player_q.single().translation;
    for (e, mut f, g) in patches.iter_mut() {
        f.life -= time.delta_secs();
        let d = Vec3::new(pp.x - g.translation().x, 0.0, pp.z - g.translation().z).length();
        if d < 2.4 && pp.y < 1.3 && health.hurt_timer <= 0.0 && health.iframes <= 0.0 {
            health.hearts = (health.hearts - 1).max(0);
            health.hurt_timer = 0.7;
        }
        if f.life <= 0.0 { commands.entity(e).despawn_recursive(); }
    }
}

// Flap the dragon's wings while airborne (and hard during takeoff).
fn dragon_wing_flap(
    time: Res<Time>,
    dragon_q: Query<&Dragon>,
    mut wing_q: Query<(&mut Transform, &DragonWing)>,
) {
    let flapping = dragon_q.iter().any(|d| matches!(d.state, DragonState::Takeoff | DragonState::Fly | DragonState::Breath));
    let hard = dragon_q.iter().any(|d| d.state == DragonState::Takeoff);
    let speed = if hard { 9.0 } else { 5.0 };
    let amp = if hard { 1.0 } else if flapping { 0.55 } else { 0.0 };
    let flap = (time.elapsed_secs() * speed).sin() * amp;
    for (mut tr, wing) in wing_q.iter_mut() {
        tr.rotation = Quat::from_rotation_z(wing.side * (-0.15 + flap));
    }
}

// Expanding ground shockwave from the dragon's stomp — jump to avoid it.
fn update_shockwaves(
    time: Res<Time>,
    mut commands: Commands,
    mut waves: Query<(Entity, &mut Transform, &mut Shockwave)>,
    player_q: Query<&Transform, (With<Player>, Without<Shockwave>)>,
    mut pv: Query<&mut PlayerVelocity, With<Player>>,
    mut health: ResMut<PlayerHealth>,
) {
    let pt = player_q.single();
    for (e, mut t, mut w) in waves.iter_mut() {
        w.radius += time.delta_secs() * 24.0;
        t.scale = Vec3::splat(w.radius);
        // Player must be airborne when the ring sweeps past them
        let horiz = Vec3::new(pt.translation.x - w.origin.x, 0.0, pt.translation.z - w.origin.z).length();
        if !w.hit && (horiz - w.radius).abs() < 2.5 {
            w.hit = true;
            if pt.translation.y < 1.3 && health.hurt_timer <= 0.0 && health.iframes <= 0.0 {
                health.hearts = (health.hearts - 2).max(0);
                health.hurt_timer = 0.9;
                if let Ok(mut v) = pv.get_single_mut() {
                    let dir = Vec3::new(pt.translation.x - w.origin.x, 0.0, pt.translation.z - w.origin.z).normalize_or_zero();
                    v.knockback = dir * 12.0;
                    v.vertical = 5.0;
                }
            }
        }
        if w.radius > 68.0 { commands.entity(e).despawn_recursive(); }
    }
}

fn flash_dragon(
    time: Res<Time>,
    flash: Option<Res<FlashMats>>,
    mut dragon_q: Query<(Entity, &mut Dragon, &mut Shock)>,
    children_q: Query<&Children>,
    mut mat_q: Query<(&mut MeshMaterial3d<StandardMaterial>, &DragonPart)>,
) {
    let Some(f) = flash else { return; };
    let white = (time.elapsed_secs() * 18.0) as i64 % 2 == 0;
    for (e, mut d, mut shock) in dragon_q.iter_mut() {
        if d.state == DragonState::Dead { continue; }
        d.damage_flash = (d.damage_flash - time.delta_secs()).max(0.0);
        shock.timer = (shock.timer - time.delta_secs()).max(0.0);
        let over = flash_override(shock.timer, d.damage_flash, white, &f);
        // Recurse all descendants (body + wing sub-entities) and recolour the parts
        let mut stack: Vec<Entity> = vec![e];
        while let Some(cur) = stack.pop() {
            if let Ok(ch) = children_q.get(cur) { for &c in ch.iter() { stack.push(c); } }
            if let Ok((mut m, part)) = mat_q.get_mut(cur) {
                m.0 = match over { Some(h) => h.clone(), None => part.base.clone() };
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
        if dist < 1.4 && health.hurt_timer <= 0.0 && health.iframes <= 0.0 {
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

    // ── Awe-inspiring décor: banners, a throne, a royal carpet & a boss fog gate ──
    let cloth = materials.add(StandardMaterial { base_color: Color::srgb(0.45, 0.05, 0.07), perceptual_roughness: 1.0, ..default() });
    let gold  = materials.add(StandardMaterial { base_color: Color::srgb(0.85, 0.68, 0.20), metallic: 0.9, perceptual_roughness: 0.3, ..default() });
    let fog_gate = materials.add(StandardMaterial {
        base_color: Color::srgba(0.7, 0.6, 1.0, 0.35),
        emissive: LinearRgba::new(0.6, 0.4, 1.4, 1.0),
        unlit: true, alpha_mode: AlphaMode::Blend, ..default() });

    // Hanging banners down both side walls
    for wall_x in [-hw + 2.0, hw - 2.0] {
        for bz in [front_z - 18.0, cz, back_z + 18.0] {
            commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.15, 7.0, 3.0))), MeshMaterial3d(cloth.clone()),
                Transform::from_xyz(wall_x, 12.0, bz)));
            commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.2, 0.4, 3.4))), MeshMaterial3d(gold.clone()),
                Transform::from_xyz(wall_x, 15.7, bz)));
        }
    }

    // Royal carpet from the gate to the throne
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(6.0, 0.06, 90.0))), MeshMaterial3d(cloth.clone()),
        Transform::from_xyz(0.0, 0.05, cz)));

    // Throne at the back, raised on a dais, flanked by gold pillars
    let throne_z = back_z + 10.0;
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(14.0, 1.0, 10.0))), MeshMaterial3d(dark_stone.clone()),
        Transform::from_xyz(0.0, 0.5, throne_z)));                                  // dais
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(3.0, 1.6, 3.0))), MeshMaterial3d(dark_stone.clone()),
        Transform::from_xyz(0.0, 1.8, throne_z)));                                  // seat
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(3.4, 5.0, 0.8))), MeshMaterial3d(dark_stone.clone()),
        Transform::from_xyz(0.0, 4.0, throne_z - 1.4)));                            // tall backrest
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(3.0, 1.4, 0.4))), MeshMaterial3d(gold.clone()),
        Transform::from_xyz(0.0, 6.2, throne_z - 1.5)));                            // gold crest
    for sx in [-1.0f32, 1.0] {
        commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.4, half_height: 4.0 })), MeshMaterial3d(gold.clone()),
            Transform::from_xyz(sx * 4.5, 4.0, throne_z)));
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.6))), MeshMaterial3d(flame_mat.clone()),
            Transform::from_xyz(sx * 4.5, 8.3, throne_z)));                          // flame finials
        commands.spawn((PointLight { color: Color::srgb(1.0, 0.6, 0.2), intensity: 200_000.0, range: 26.0, shadows_enabled: false, ..default() },
            Transform::from_xyz(sx * 4.5, 8.3, throne_z)));
    }

    // Shimmering boss fog gate filling the entrance
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(gate * 2.0, 14.0, 0.3))), MeshMaterial3d(fog_gate),
        Transform::from_xyz(0.0, 7.0, front_z)));
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
    mut stamina: ResMut<Stamina>,
    mut slow: ResMut<MoveSlow>,
    mut health: ResMut<PlayerHealth>,
    mut player_q: Query<(&mut Transform, &mut PlayerVelocity), With<Player>>,
) {
    let (mut transform, mut velocity) = player_q.single_mut();
    let dt = time.delta_secs();

    health.iframes = (health.iframes - dt).max(0.0);

    let moving = key.pressed(KeyCode::KeyW) || key.pressed(KeyCode::KeyS)
              || key.pressed(KeyCode::KeyA) || key.pressed(KeyCode::KeyD);
    let wants_sprint = key.pressed(KeyCode::ShiftLeft) || key.pressed(KeyCode::ShiftRight);
    // Sprint only while there's stamina to burn
    let sprinting = wants_sprint && moving && stamina.current > 0.0;
    if sprinting {
        stamina.current = (stamina.current - 13.3 * dt).max(0.0); // gentle drain
    }
    // Dragon's roar slows the player 30%
    slow.timer = (slow.timer - dt).max(0.0);
    let slow_mul = if slow.timer > 0.0 { 0.7 } else { 1.0 };
    let speed = (if sprinting { 18.0 } else { 6.0 }) * slow_mul;

    let fwd = *transform.forward();
    let right = *transform.right();
    let forward    = Vec3::new(fwd.x,   0.0, fwd.z  ).normalize_or_zero();
    let right_flat = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

    let mut dir = Vec3::ZERO;
    if key.pressed(KeyCode::KeyW) { dir += forward; }
    if key.pressed(KeyCode::KeyS) { dir -= forward; }
    if key.pressed(KeyCode::KeyA) { dir -= right_flat; }
    if key.pressed(KeyCode::KeyD) { dir += right_flat; }

    // ── Dodge roll (Left Ctrl): a quick burst with brief invulnerability ──
    if key.just_pressed(KeyCode::ControlLeft) && velocity.roll_timer <= 0.0 && stamina.current >= 17.5 {
        velocity.roll_dir = if dir.length_squared() > 0.0 { dir.normalize() } else { forward };
        velocity.roll_timer = 0.45;
        health.iframes = 0.35;
        stamina.current -= 17.5;
    }

    if velocity.roll_timer > 0.0 {
        // Rolling overrides normal walking
        velocity.roll_timer -= dt;
        transform.translation += velocity.roll_dir * 16.0 * dt;
    } else if dir.length_squared() > 0.0 {
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

// Regenerate stamina (when not sprinting) and mana (when not casting) over time.
fn regen_resources(
    time: Res<Time>,
    key: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut stamina: ResMut<Stamina>,
    mut mana: ResMut<Mana>,
) {
    let dt = time.delta_secs();
    let sprinting = key.pressed(KeyCode::ShiftLeft) || key.pressed(KeyCode::ShiftRight);
    if !sprinting {
        stamina.current = (stamina.current + 22.0 * dt).min(stamina.max);
    }
    // Mana no longer regenerates — refill it with blue mana potions.
    let _ = (&mut mana, &mouse);
}

// Scroll wheel cycles the selected inventory item.
fn inventory_scroll(
    mut scroll: EventReader<bevy::input::mouse::MouseWheel>,
    mut inv: ResMut<Inventory>,
) {
    let mut dir = 0i32;
    for ev in scroll.read() {
        if ev.y > 0.0 { dir -= 1; } else if ev.y < 0.0 { dir += 1; }
    }
    if dir == 0 { return; }
    let items = inv.available();
    let cur = items.iter().position(|k| *k == inv.selected).unwrap_or(0) as i32;
    let n = items.len() as i32;
    let next = ((cur + dir) % n + n) % n;
    inv.selected = items[next as usize];
}

// Show only the held visual matching the selected (and owned) item.
fn update_held(
    inv: Res<Inventory>,
    mut held_q: Query<(&HeldVisual, &mut Visibility)>,
) {
    for (h, mut vis) in held_q.iter_mut() {
        *vis = if inv.owns(h.kind) && h.kind == inv.selected { Visibility::Inherited } else { Visibility::Hidden };
    }
}

// Left-click action depends on the selected item (sword handled separately).
fn use_item(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut inv: ResMut<Inventory>,
    mut health: ResMut<PlayerHealth>,
    mut mana: ResMut<Mana>,
    mut drinking: ResMut<Drinking>,
    camera_q: Query<&GlobalTransform, With<PlayerCamera>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let window = windows.single();
    if window.cursor_options.grab_mode != CursorGrabMode::Locked { return; }
    if !mouse.just_pressed(MouseButton::Left) { return; }

    let cam = camera_q.single();
    let (_, rot, pos) = cam.to_scale_rotation_translation();
    let fwd = rot * Vec3::NEG_Z;
    let muzzle = pos + fwd * 0.8 - Vec3::Y * 0.1;

    match inv.selected {
        ItemKind::HealthPotion => {
            if inv.health_potions > 0 {
                inv.health_potions -= 1;
                health.hearts = (health.hearts + 2).min(5);
                drinking.timer = 0.7;
                if inv.health_potions == 0 { inv.selected = ItemKind::Sword; }
            }
        }
        ItemKind::ManaPotion => {
            if inv.mana_potions > 0 {
                inv.mana_potions -= 1;
                mana.current = mana.max; // blue potion fully restores mana
                drinking.timer = 0.7;
                if inv.mana_potions == 0 { inv.selected = ItemKind::Sword; }
            }
        }
        ItemKind::Glock => {} // hitscan, handled by glock_fire
        ItemKind::Rocket => {
            // Slow rocket: cylinder body + triangular (cone-3) tip, pointing along travel
            let body = materials.add(StandardMaterial {
                base_color: Color::srgb(0.55, 0.55, 0.6), metallic: 0.6, perceptual_roughness: 0.4, ..default() });
            let tipm = materials.add(StandardMaterial {
                base_color: Color::srgb(0.9, 0.2, 0.1), emissive: LinearRgba::new(2.0, 0.3, 0.1, 1.0), ..default() });
            let aim_rot = Transform::from_translation(muzzle).looking_to(fwd, Vec3::Y).rotation;
            commands.spawn((
                Transform { translation: muzzle, rotation: aim_rot, ..default() },
                GlobalTransform::default(), Visibility::default(),
                Rocket { velocity: fwd * 28.0, life: 6.0 },
                PointLight { color: Color::srgb(1.0, 0.5, 0.1), intensity: 80_000.0,
                    range: 12.0, shadows_enabled: false, ..default() },
            )).with_children(|r| {
                // cylinder body (local Z = travel; rotate cylinder's Y to Z)
                r.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.10, half_height: 0.30 })),
                    MeshMaterial3d(body.clone()),
                    Transform::from_xyz(0.0, 0.0, 0.0).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))));
                // triangular nose cone at the front (-Z)
                r.spawn((Mesh3d(meshes.add(Cone { radius: 0.12, height: 0.26 }.mesh().resolution(3))),
                    MeshMaterial3d(tipm),
                    Transform::from_xyz(0.0, 0.0, -0.42).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))));
                // tail fins
                for a in [0.0f32, std::f32::consts::FRAC_PI_2] {
                    r.spawn((Mesh3d(meshes.add(Cuboid::new(0.26, 0.02, 0.12))),
                        MeshMaterial3d(body.clone()),
                        Transform::from_xyz(0.0, 0.0, 0.28).with_rotation(Quat::from_rotation_z(a))));
                }
            });
        }
        ItemKind::Sword => {} // handled by sword_swing
    }
}

// Glock: instant hitscan beam from the muzzle; pierces enemies along the ray.
fn glock_fire(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    inv: Res<Inventory>,
    camera_q: Query<&GlobalTransform, With<PlayerCamera>>,
    mut recoil: ResMut<GunRecoil>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut skel_q: Query<(Entity, &GlobalTransform, &mut Skeleton)>,
    mut enemy_q: Query<(Entity, &GlobalTransform, &mut Enemy)>,
    mut dragon_q: Query<(Entity, &GlobalTransform, &mut Dragon)>,
) {
    let window = windows.single();
    if window.cursor_options.grab_mode != CursorGrabMode::Locked { return; }
    if !mouse.just_pressed(MouseButton::Left) || inv.selected != ItemKind::Glock { return; }

    let cam = camera_q.single();
    let (_, rot, pos) = cam.to_scale_rotation_translation();
    let fwd = rot * Vec3::NEG_Z;
    let right = rot * Vec3::X;
    let up = rot * Vec3::Y;
    let muzzle = pos + fwd * 0.7 + right * 0.14 - up * 0.06;
    let range = 120.0f32;
    recoil.climb = (recoil.climb + 0.35).min(2.4); // stacks higher the more you shoot

    // Tighter hit radius — aim matters now
    let hit = |center: Vec3| -> bool {
        let to = center - muzzle;
        let proj = fwd.dot(to);
        proj > 0.0 && proj < range && (to - fwd * proj).length() < 1.0
    };
    for (e, g, mut s) in skel_q.iter_mut() {
        if s.state != SkeletonState::Dead && hit(g.translation() + Vec3::Y) {
            s.health -= 1.0; s.damage_flash = 0.25;
            if s.health <= 0.0 { s.state = SkeletonState::Dead; commands.entity(e).despawn_recursive(); }
        }
    }
    for (e, g, mut en) in enemy_q.iter_mut() {
        if hit(g.translation() + Vec3::Y) {
            en.health -= 1.0; en.damage_flash = 0.25;
            if en.health <= 0.0 { commands.entity(e).despawn_recursive(); }
        }
    }
    for (e, g, mut d) in dragon_q.iter_mut() {
        if d.state != DragonState::Dead && hit(g.translation() + Vec3::Y * 2.0) {
            d.health -= 1.0; d.damage_flash = 0.2;
            if d.health <= 0.0 { d.state = DragonState::Dead; commands.entity(e).despawn_recursive(); }
        }
    }

    // Tracer beam from the muzzle
    let beam = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.95, 0.5),
        emissive: LinearRgba::new(8.0, 7.0, 2.0, 1.0), unlit: true, ..default() });
    let len = 60.0f32;
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.04, 0.04, len))),
        MeshMaterial3d(beam),
        Transform { translation: muzzle + fwd * (len * 0.5), rotation: rot, ..default() },
        Transient { life: 0.05 },
    ));
    // Jagged red-orange muzzle flash — a spiky star of thin spikes at the tip
    let flash = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.35, 0.05),
        emissive: LinearRgba::new(11.0, 3.0, 0.4, 1.0), unlit: true, ..default() });
    let spike = meshes.add(Cone { radius: 0.05, height: 0.26 }.mesh().resolution(4));
    let flash_pos = muzzle + fwd * 0.12;
    for i in 0..6u32 {
        let a = i as f32 / 6.0 * std::f32::consts::TAU;
        // point each spike outward around the barrel axis (forward = -Z in cam space)
        let dir_rot = rot * Quat::from_rotation_z(a) * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2 + 0.5);
        commands.spawn((
            Mesh3d(spike.clone()), MeshMaterial3d(flash.clone()),
            Transform { translation: flash_pos, rotation: dir_rot, scale: Vec3::new(1.0, 1.0, 0.6 + (i % 2) as f32 * 0.5), ..default() },
            Transient { life: 0.05 },
        ));
    }
    commands.spawn((
        PointLight { color: Color::srgb(1.0, 0.45, 0.1), intensity: 110_000.0, range: 8.0, shadows_enabled: false, ..default() },
        Transform::from_translation(flash_pos),
        Transient { life: 0.05 },
    ));
}

// Gun recoil: kick the held glock up; rapid fire stacks the climb higher.
fn gun_recoil_anim(
    time: Res<Time>,
    mut recoil: ResMut<GunRecoil>,
    mut held_q: Query<(&mut Transform, &HeldVisual)>,
) {
    recoil.climb = (recoil.climb - time.delta_secs() * 3.0).max(0.0); // settle back down
    let c = recoil.climb;
    for (mut t, h) in held_q.iter_mut() {
        if h.kind == ItemKind::Glock {
            let base = Vec3::new(0.18, -0.12, -0.46);
            t.translation = base + Vec3::new(0.0, 0.03 * c, 0.05 * c); // rises with the stack
            t.rotation = Quat::from_rotation_x(0.35 * c);              // muzzle flips up more each shot
        }
    }
}

// Despawn short-lived visuals (tracers, mushroom clouds) when their life runs out.
fn tick_transient(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Transient)>,
) {
    for (e, mut t) in q.iter_mut() {
        t.life -= time.delta_secs();
        if t.life <= 0.0 { commands.entity(e).despawn_recursive(); }
    }
}

// Grow the nuke mushroom cloud as it ages.
fn animate_mushroom(
    time: Res<Time>,
    mut q: Query<(&mut Transform, &mut Mushroom)>,
) {
    for (mut t, mut m) in q.iter_mut() {
        m.age += time.delta_secs();
        let s = (m.age * 3.5).min(1.0); // grow over ~0.3s then hold
        t.scale = Vec3::splat(0.3 + s * 1.4);
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

    // Stacked tapering tiers (base → crown), 30% taller. (half-width, height, center-y)
    let tiers: &[(f32, f32, f32)] = &[
        (16.0, 39.0, 19.5),
        (12.5, 36.4, 57.2),
        ( 9.5, 33.8, 92.3),
        ( 7.0, 28.6, 123.5),
        ( 9.0,  6.5, 141.0), // flared crown
    ];
    for &(hw, h, cy) in tiers {
        let mut e = commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(hw * 2.0, h, hw * 2.0))),
            MeshMaterial3d(obsidian.clone()),
            Transform::from_xyz(tx, cy, tz),
        ));
        // Only the lower tiers (reachable on foot) need collision
        if cy < 65.0 {
            e.insert(Collider { half: Vec2::new(hw, hw) });
        }
    }

    // Jagged buttress spikes splayed outward to frame the eye (Barad-dûr silhouette)
    let spike_mesh = meshes.add(Cuboid::new(1.6, 28.0, 1.6));
    let splay = 0.42f32; // outward lean
    for i in 0..8u32 {
        let a = (i as f32 / 8.0) * std::f32::consts::TAU;
        let r = 8.0;
        commands.spawn((
            Mesh3d(spike_mesh.clone()),
            MeshMaterial3d(obsidian.clone()),
            Transform::from_xyz(tx + a.cos() * r, 154.0, tz + a.sin() * r)
                .with_rotation(Quat::from_rotation_z(-a.cos() * splay)
                             * Quat::from_rotation_x(a.sin() * splay)),
        ));
    }

    // ── Eye assembly (child-rotated by animate_eye to scan the horizon) ──
    // Raised well above the splayed spikes so it sits clear in the air
    let eye_root = commands.spawn((
        Transform::from_xyz(tx, 182.0, tz),
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
            Text::new("YOU DIED"),
            TextFont { font_size: 110.0, ..default() },
            TextColor(Color::srgb(0.62, 0.03, 0.03)),
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

// Show the boss bar + scale its fill while the dragon is engaged.
fn update_boss_bar(
    dragon_q: Query<&Dragon>,
    mut root_q: Query<&mut Visibility, With<BossBarRoot>>,
    mut fill_q: Query<&mut Node, With<BossBarFill>>,
) {
    // Only appears once the dragon is enraged (phase 2)
    let engaged = dragon_q.iter().find(|d| matches!(d.state,
        DragonState::Roar | DragonState::Takeoff | DragonState::Fly | DragonState::Breath));
    if let Ok(mut vis) = root_q.get_single_mut() {
        *vis = if engaged.is_some() { Visibility::Inherited } else { Visibility::Hidden };
    }
    if let (Some(d), Ok(mut n)) = (engaged, fill_q.get_single_mut()) {
        n.width = Val::Percent((d.health / d.max_health * 100.0).clamp(0.0, 100.0));
    }
}

// Cryptic Dark-Souls-style soapstone messages when you stand on a rune.
const SOAPSTONE_MSGS: &[&str] = &[
    "amazing chest ahead",
    "try jumping",
    "be wary of dragon",
    "didn't expect horror...",
    "visions of treasure ahead",
    "praise the sun \\[T]/",
    "but hole",
    "could this be a trap?",
];
fn soapstone_msg(
    player_q: Query<&Transform, With<Player>>,
    stones: Query<(&GlobalTransform, &Soapstone)>,
    mut text_q: Query<&mut Text, With<SoapstoneText>>,
) {
    let pp = player_q.single().translation;
    let mut msg = "";
    let mut best = 5.0f32;
    for (g, s) in &stones {
        let d = g.translation().distance(pp);
        if d < best { best = d; msg = SOAPSTONE_MSGS[s.idx % SOAPSTONE_MSGS.len()]; }
    }
    if let Ok(mut t) = text_q.get_single_mut() { *t = Text::new(msg); }
}

// Grow & drift the roar sound-wave rings, then fade them out.
fn update_sound_waves(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Transform, &mut Transient), With<SoundWave>>,
) {
    for (e, mut t, mut tr) in q.iter_mut() {
        tr.life -= time.delta_secs();
        let grow = 1.0 + time.delta_secs() * 6.0;
        t.scale *= grow;
        let fwd = t.rotation * Vec3::Z;
        t.translation += fwd * 6.0 * time.delta_secs(); // travel outward from the mouth
        if tr.life <= 0.0 { commands.entity(e).despawn_recursive(); }
    }
}

// Old-school floating loot: ground pickups slowly spin and bob.
fn spin_pickups(
    time: Res<Time>,
    mut q: Query<&mut Transform, With<Pickup>>,
) {
    let t = time.elapsed_secs();
    for mut tr in q.iter_mut() {
        tr.rotation = Quat::from_rotation_y(t * 1.4);
        tr.translation.y = 0.6 + (t * 2.0).sin() * 0.18;
    }
}

// Esc toggles pause (only between Playing and Paused).
fn toggle_pause(
    key: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
) {
    if !key.just_pressed(KeyCode::Escape) { return; }
    match *state.get() {
        AppState::Playing => next.set(AppState::Paused),
        AppState::Paused  => next.set(AppState::Playing),
        _ => {}
    }
}

fn spawn_pause_screen(mut commands: Commands, mut windows: Query<&mut Window>) {
    let mut window = windows.single_mut();
    window.cursor_options.grab_mode = CursorGrabMode::None;
    window.cursor_options.visible = true;
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0), height: Val::Percent(100.0),
            justify_content: JustifyContent::Center, align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.45)),
        PauseScreen,
    )).with_children(|p| {
        p.spawn((
            Text::new("PAUSED"),
            TextFont { font_size: 90.0, ..default() },
            TextColor(Color::srgb(0.1, 0.1, 0.12)),
        ));
    });
}

fn despawn_pause_screen(mut commands: Commands, q: Query<Entity, With<PauseScreen>>) {
    for e in q.iter() { commands.entity(e).despawn_recursive(); }
}

fn reset_game(
    mut health: ResMut<PlayerHealth>,
    mut stamina: ResMut<Stamina>,
    mut mana: ResMut<Mana>,
    mut inv: ResMut<Inventory>,
    mut player_q: Query<(&mut Transform, &mut PlayerVelocity), With<Player>>,
    mut commands: Commands,
    clutter: Query<Entity, Or<(With<Skeleton>, With<Dragon>, With<Fireball>, With<Enemy>,
                               With<Pickup>, With<Rocket>, With<MagicMissile>, With<Debris>, With<Transient>,
                               With<DragonLaser>, With<Shockwave>, With<FirePatch>)>>,
) {
    health.hearts = 5;
    health.hurt_timer = 0.0;
    health.iframes = 0.0;
    stamina.current = stamina.max;
    mana.current = mana.max;
    *inv = Inventory { selected: ItemKind::Sword, health_potions: 0, mana_potions: 0, has_glock: false, has_rocket: false };
    let (mut t, mut v) = player_q.single_mut();
    *t = Transform::from_xyz(0.0, 0.0, 10.0);
    v.vertical = 0.0;
    v.knockback = Vec3::ZERO;
    v.roll_timer = 0.0;
    // Clear all enemies, pickups, and live projectiles — fresh start
    for e in clutter.iter() { commands.entity(e).despawn_recursive(); }
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
    if health.hurt_timer <= 0.0 && health.iframes <= 0.0 {
        health.hearts = (health.hearts - 1).max(0);
        health.hurt_timer = 0.9;
        let mut pvel = player_vel.single_mut();
        let knock = Vec3::new(target.x - center.x, 0.0, target.z - center.z).normalize_or_zero();
        pvel.knockback = knock * 7.0;
    }
}
