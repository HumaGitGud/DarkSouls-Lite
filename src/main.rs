use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::window::CursorGrabMode;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::image::{ImageSampler, ImageSamplerDescriptor, ImageAddressMode, ImageFilterMode};
use bevy::pbr::{DistanceFog, FogFalloff, NotShadowCaster};
use bevy::app::AppExit;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct PlayerVelocity { vertical: f32, knockback: Vec3, roll_timer: f32, roll_dir: Vec3 }

#[derive(Component)]
struct PlayerCamera { pitch: f32, bob_timer: f32 }

#[derive(Component)]
struct Sword { swinging: bool, timer: f32, hit_registered: bool }
#[derive(Component)] struct SteelVis;   // the held steel-sword meshes (hidden when ruined)
#[derive(Component)] struct RuinedVis;  // the held Ruined-Blade meshes (shown when claimed)

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
struct HealthBar;       // red current-health fill
#[derive(Component)]
struct GoldenBar;       // golden temporary-shield fill (overlaid)

#[derive(Component)]
struct DamageVignette;

#[derive(Component)]
struct StaminaBar;

#[derive(Component)]
struct ManaBar;

/// Continuous health (100 max). `golden` is temporary shield health granted by
/// the Paladin artifact; it absorbs damage first and decays after `golden_timer`.
/// `blocking` is set each frame while the player holds a sword block (RMB).
#[derive(Resource)]
struct PlayerHealth {
    hp: f32,
    max_hp: f32,
    golden: f32,
    golden_timer: f32,
    hurt_timer: f32,
    iframes: f32,
    blocking: bool,
}

impl PlayerHealth {
    /// Apply incoming damage, honoring i-frames, hurt cooldown, sword block
    /// (-50%), and the golden shield (absorbs first). `hurt` sets the cooldown.
    fn take(&mut self, dmg: f32, hurt: f32) {
        if self.hurt_timer > 0.0 || self.iframes > 0.0 { return; }
        let mut d = if self.blocking { dmg * 0.5 } else { dmg };
        if self.golden > 0.0 {
            let absorbed = d.min(self.golden);
            self.golden -= absorbed;
            d -= absorbed;
        }
        self.hp = (self.hp - d).max(0.0);
        self.hurt_timer = hurt;
    }
}



#[derive(Component, PartialEq, Clone, Copy)]
enum DragonState { Idle, Meteor, Ground, Roar, Takeoff, Fly, Breath, Dead }

// ── Medusa boss (castle) ──
#[derive(Component, PartialEq, Clone, Copy)]
enum MedusaState { Idle, Chase, Enrage, Gaze, Dead }
#[derive(Component)]
struct Medusa { health: f32, max_health: f32, state: MedusaState, damage_flash: f32,
                timer: f32, enraged: bool, attack_timer: f32, gaze_timer: f32,
                dash_cd: f32, windup: f32, charge_t: f32, charge_dir: Vec3 }

/// Handle to Medusa's eye material so her eyes can glow yellow → red on a charge.
#[derive(Resource)]
struct MedusaEye { mat: Handle<StandardMaterial> }

// ── Shadow succubus (rare roaming demon). Carries `Enemy` for HP/flash/weapons. ──
#[derive(Component)]
struct Succubus { attack: f32, swoop: f32 }
#[derive(Component)]
struct SuccubusWing { side: f32 }
#[derive(Resource)]
struct SuccubusSpawn { timer: f32 }

// ── Sauron, the Dark Lord (arena boss). Also carries `Enemy` for HP. ──
#[derive(Component)]
struct Sauron { phase: u8, max: f32, slam: f32, fire: f32, nova: f32, meteor: f32, enraged: bool }
#[derive(Component)]
struct SauronPortal;
#[derive(Component)]
struct SauronArena;            // marker for arena scenery (so it can be cleared)
#[derive(Component)]
struct PortalSwirl;            // a spinning swirl of white motes on the Sauron well
#[derive(Component)]
struct ReturnPortal;           // appears when Sauron falls — steps back to the spire
#[derive(Component)]
struct SauronHpBar;            // healthbar frame (UI)
#[derive(Component)]
struct SauronHpFill;           // healthbar fill (UI)
#[derive(Resource)]
struct SauronFight { active: bool, spawned: bool, defeated: bool, engaged: bool, origin: Vec3 }
#[derive(Component)]
struct MedusaBolt { vel: Vec3, life: f32 }
#[derive(Component)]
struct StoneWave { radius: f32, hit: bool, dir: Vec3, origin: Vec3 }
#[derive(Component)]
struct MedusaBarRoot;
#[derive(Component)]
struct MedusaBarFill;

/// While >0 the player is turned to stone — frozen in place.
#[derive(Resource)]
struct Petrify { timer: f32 }
#[derive(Component)]
struct PetrifyOverlay;

/// Drives the dragon's delayed meteor arrival after Medusa falls.
#[derive(Resource)]
struct DragonArrival { counting: bool, countdown: f32, spawn_now: bool, spawned: bool, pos: Vec3, target: Vec3 }
#[derive(Component)]
struct CountdownText;
#[derive(Component)]
struct MeteorShock { radius: f32, hit: bool, origin: Vec3, max: f32 }

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
    dodge_timer: f32,   // >0 while mid sideways-dodge (ground phase)
    dodge_dir: Vec3,    // direction of the current dodge
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

/// A walkable surface the player can stand on: an XZ footprint with a top height.
/// Lets stairs and platforms raise the player's effective floor (see player_movement).
#[derive(Component)]
struct Walkable { half: Vec2, top: f32 }

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

/// A purely-visual ring/shell that expands and fades, then despawns.
#[derive(Component)]
struct Expand { rate: f32, life: f32 }

#[derive(Component)]
struct Pickup { kind: ItemKind }

#[derive(Component)]
struct HeldVisual { kind: ItemKind }

#[derive(Clone, Copy, PartialEq, Eq)]
enum ItemKind { Sword, Glock, Rocket, Bow, HealthPotion, ManaPotion }

#[derive(Resource)]
struct Stamina { current: f32, max: f32 }

#[derive(Resource)]
struct Mana { current: f32, max: f32 }

#[derive(Resource)]
struct Drinking { timer: f32 }

#[derive(Resource)]
struct GunRecoil { climb: f32 }

/// A flying arrow shot from the bow.
#[derive(Component)]
struct Arrow { vel: Vec3, life: f32 }

/// The string+arrow group on the held bow; pulled back as the bow is drawn.
#[derive(Component)]
struct BowNock;

/// Bow draw charge: hold LMB to draw (windup); release at full draw to loose.
#[derive(Resource)]
struct BowState { draw: f32, drawing: bool }

/// Held-sword blade materials, so the deflect FX can make the blade glow red.
#[derive(Resource)]
struct SwordAssets { blade: Handle<StandardMaterial>, edge: Handle<StandardMaterial> }

/// >0 while the sword is actively deflecting the eye beam (drives the red glow).
#[derive(Resource)]
struct SwordGlow { timer: f32 }
/// Red point light parented to the sword; intensity is driven by SwordGlow.
#[derive(Component)]
struct SwordGlowLight;

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
    has_bow: bool,
    has_ruined_blade: bool,   // the sword slot becomes the Blade of the Ruined King
}

impl Inventory {
    /// Items currently available to cycle through (sword always first).
    fn available(&self) -> Vec<ItemKind> {
        let mut v = vec![ItemKind::Sword];
        if self.has_glock { v.push(ItemKind::Glock); }
        if self.has_rocket { v.push(ItemKind::Rocket); }
        if self.has_bow { v.push(ItemKind::Bow); }
        if self.health_potions > 0 { v.push(ItemKind::HealthPotion); }
        if self.mana_potions > 0 { v.push(ItemKind::ManaPotion); }
        v
    }
    fn owns(&self, kind: ItemKind) -> bool {
        match kind {
            ItemKind::Sword => true,
            ItemKind::Glock => self.has_glock,
            ItemKind::Rocket => self.has_rocket,
            ItemKind::Bow => self.has_bow,
            ItemKind::HealthPotion => self.health_potions > 0,
            ItemKind::ManaPotion => self.mana_potions > 0,
        }
    }
}

/// Off-hand magic artifacts, cycled with E. Lightning is the starter; the rest
/// are found in the world. The equipped one is fired with Q (block stays on RMB).
#[derive(Clone, Copy, PartialEq, Eq)]
enum ArtifactKind { Lightning, Flame, Paladin, Trident, Telekinesis, BlackHole }

#[derive(Resource)]
struct Artifacts {
    selected: ArtifactKind,
    has_flame: bool,
    has_paladin: bool,
    has_trident: bool,
    has_telekinesis: bool,
    has_blackhole: bool,
    trident_armed: f32,   // >0 while the trident's rain/storm mode is active (seconds)
    trident_in_hand: bool, // false briefly after a throw, until it reforms
    throw_cooldown: f32,
    tk_cooldown: f32,      // telekinesis shockwave cooldown
}
impl Artifacts {
    fn unlocked(&self) -> Vec<ArtifactKind> {
        let mut v = vec![ArtifactKind::Lightning];
        if self.has_flame   { v.push(ArtifactKind::Flame); }
        if self.has_paladin { v.push(ArtifactKind::Paladin); }
        if self.has_trident { v.push(ArtifactKind::Trident); }
        if self.has_telekinesis { v.push(ArtifactKind::Telekinesis); }
        if self.has_blackhole { v.push(ArtifactKind::BlackHole); }
        v
    }
    fn owns(&self, k: ArtifactKind) -> bool {
        match k {
            ArtifactKind::Lightning => true,
            ArtifactKind::Flame   => self.has_flame,
            ArtifactKind::Paladin => self.has_paladin,
            ArtifactKind::Trident => self.has_trident,
            ArtifactKind::Telekinesis => self.has_telekinesis,
            ArtifactKind::BlackHole => self.has_blackhole,
        }
    }
}

/// Marks each left-hand artifact model group so only the equipped one shows.
#[derive(Component)]
struct ArtifactVisual { kind: ArtifactKind }

/// A relic resting in the world that unlocks its artifact when the player reaches it.
#[derive(Component)]
struct ArtifactPickup { kind: ArtifactKind, base_y: f32 }

/// Tags for animated artifact effects.
#[derive(Component)]
struct GoldenAura { life: f32 }        // paladin ground ring
#[derive(Component)]
struct TridentProjectile { vel: Vec3, life: f32 } // thrown trident
#[derive(Component)]
struct Geyser { radius: f32, life: f32, hit: bool } // water eruption
#[derive(Component)]
struct Launched { vel: Vec3 }          // enemy flung airborne (e.g. by a geyser)
#[derive(Component)]
struct FlameJet { grow: f32 }          // flamethrower puff that swells as it travels

// ── Sky ──
#[derive(Component)]
struct SkyStar { phase: f32, base: f32 }   // twinkling star
#[derive(Component)]
struct AuroraBand { phase: f32, yaw: f32 } // shimmering aurora curtain
#[derive(Component)]
struct ShootingStar { vel: Vec3, life: f32 }
#[derive(Resource)]
struct SkyTimer { shoot: f32 }

// ── Bonfire kindling + heal feedback ──
#[derive(Component)]
struct BonfireKindle { timer: f32, spark: f32, mat: Handle<StandardMaterial>, mesh: Handle<Mesh> }
#[derive(Resource)]
struct HealFx { timer: f32, spawn_t: f32, color: Color }
#[derive(Component)]
struct HealCrest { life: f32, max: f32, bottom0: f32, rise: f32, color: Color }

// ── Void portal left behind when the dragon is slain ──
#[derive(Component)]
struct VoidPortal { spawn_t: f32, center: Vec3, mat: Handle<StandardMaterial>, mesh: Handle<Mesh> }
#[derive(Component)]
struct VoidParticle { center: Vec3, angle: f32, radius: f32, speed: f32, y: f32 }
#[derive(Component)]
struct RainDrop { vel: f32 }           // falling rain streak
#[derive(Component)]
struct RainCloud;                      // sky cloud slab

/// Drives periodic enemy respawns so the world stays populated.
#[derive(Resource)]
struct Spawner { timer: f32 }

/// Which map the player is in. Entering the dragon-death void portal sends them
/// to a heavenly floating-island parkour realm high in the sky.
#[derive(Resource)]
struct Realm { in_sky: bool, spawned: bool, start: Vec3 }

/// Ambient sky-realm wildlife: 0 = butterfly, 1 = bird.
#[derive(Component)]
struct SkyCritter { kind: u8, base: Vec3, phase: f32, speed: f32, radius: f32 }

/// The princess awaiting at the sky shrine; press R near her to end the game.
#[derive(Component)]
struct Princess;
/// Gentle idle bob for the princess.
#[derive(Component)]
struct PrincessIdle { base_y: f32 }
/// Tally of foes slain, shown in the end credits.
#[derive(Resource, Default)]
struct KillStats { skeletons: u32, beasts: u32, medusa: u32, dragon: u32 }

/// End-game sequence: stage 0 = none, 1 = dialogue, 2 = ascend + credits roll.
#[derive(Resource)]
struct Ending { stage: u8, timer: f32 }
#[derive(Component)]
struct DialogueRoot;
#[derive(Component)]
struct DialogueLine;
#[derive(Component)]
struct CreditsText;

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
#[allow(dead_code)] // kept as the "dragon has arrived" gate for dragon_ai
struct DragonAssets {
    fb_mat:    Handle<StandardMaterial>,
    fb_mesh:   Handle<Mesh>,
}

// ════════════════════════════════════════════════════════════════════════════
//  THE MYSTIC'S PILLS · the Hut of Revelation & Shadow Isles
// ════════════════════════════════════════════════════════════════════════════
#[derive(Component)] struct NpcIdle { base_y: f32, phase: f32 }
#[derive(Component)] struct InteractHint;
#[derive(Component)] struct Mystic;
#[derive(Component)] struct MysticProp;        // anything spawned during her offer
#[derive(Component)] struct MysticLine;        // her dialogue text
#[derive(Component)] struct HutWitch;
#[derive(Component)] struct HutLine;           // witch dialogue text
#[derive(Component)] struct RuinedBlade;
#[derive(Component)] struct FadeOverlay;
#[derive(Component)] struct AreaTitle { life: f32 }
#[derive(Component)] struct ShadowRain { vel: f32 }
#[derive(Component)] struct ShadowProp;        // shadow-isles scenery (despawned on reset)
#[derive(Component)] struct HutProp;           // hut-realm scenery
#[derive(Component)] struct MistZombie { life: f32, attack: f32 }   // a converted foe fighting for you
#[derive(Component)] struct BladeRiddlePanel;
#[derive(Component)] struct BlackHoleProj { vel: Vec3, life: f32 }
#[derive(Component)] struct BlackHoleCore { radius: f32, age: f32 }
#[derive(Component)] struct BlackHoleVis;                              // the hole's own meshes
#[derive(Component)] struct BlackHoleParticle { ang: f32, dist: f32, y: f32 }
#[derive(Component)] struct BlackHoleRing;                            // white-bordered ground ring around the base
#[derive(Component)] struct BlackHoleDebris { ang: f32, dist: f32, spin: f32 }   // violently spinning rubble
#[derive(Component)] struct BlackHoleBlast { radius: f32, max: f32 }  // the final map-wide shockwave
#[derive(Component)] struct VoidScar;                                 // the permanent crater left by the blast
#[derive(Resource)] struct BladeRiddle { active: bool }

// The blade's riddle — the correct answer is option 2 (Footsteps).
const BLADE_RIDDLE: &str = "A voice of green mist rises from the steel:\n\n\"Mortal. Lay no hand on me until you answer — or be unmade.\n\nThe more you take of me, the more you leave behind.\nI dog every traveler to the grave, yet no soul has\never looked upon my face. Name me.\"\n\n   [1]  Shadow        [2]  Footsteps        [3]  Regret";

#[derive(Resource)] struct MysticTalk { stage: u8, timer: f32, line: usize }   // 0 idle,1 intro,2 pills
#[derive(Resource)] struct Warp { stage: u8, timer: f32, dest: u8 }            // dest 1 shadow,2 hut,3 home
#[derive(Resource)] struct HutTalk { line: usize, active: bool }
#[derive(Resource)] struct Areas {
    hut_built: bool, shadow_built: bool, in_shadow: bool, in_hut: bool, bolt_timer: f32,
}

const HUT_ORIGIN: Vec3 = Vec3::new(-6000.0, 0.0, 0.0);
const SHADOW_ORIGIN: Vec3 = Vec3::new(6000.0, 0.0, 0.0);

const MYSTIC_LINES: [&str; 4] = [
    "Oh... another dreamer wanders into my hour.",
    "I am Nyx — keeper of the thresholds between what is and what could be.",
    "I have waited for you since before you were born. And after.",
    "Now choose: one pill ends the dream... the other begins a far stranger one.",
];
const HUT_LINES: [&str; 7] = [
    "...You came. They always come, in the end. Do not look at the candle too long.",
    "I have no name you could pronounce without bleeding. Call me the Hollow Mother.",
    "Listen. The Eye on its black spire is not a god. It is a wound in the world — and the world is leaking.",
    "Everything you have killed still breathes, somewhere behind the sky. Everything you love is already ash, somewhere ahead of it.",
    "There is only one mercy left to a place this rotten: to unmake it. To let the dark finish what the light began.",
    "So I give you a piece of the end itself — a hunger that eats even light. Point it at the Eye. Point it at anything.",
    "Take it. And when you cast it... do not stand too close to what you love.",
];

// ── Startup: just the HUD bits. Nyx only appears once Sauron is defeated. ──
fn spawn_npcs(
    mut commands: Commands,
) {
    // Fullscreen fade overlay (hidden until a warp)
    commands.spawn((
        Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)), Visibility::Hidden, FadeOverlay,
    ));
    // "[ R ]  Interact" hint, bottom-centre, hidden until near someone
    commands.spawn((
        Node { position_type: PositionType::Absolute, bottom: Val::Px(90.0), left: Val::Percent(50.0),
               margin: UiRect::left(Val::Px(-90.0)), width: Val::Px(180.0),
               justify_content: JustifyContent::Center, ..default() },
        Visibility::Hidden, InteractHint,
    )).with_children(|p| {
        p.spawn((Text::new("[ R ]  Interact"), TextFont { font_size: 22.0, ..default() },
            TextColor(Color::srgb(1.0, 0.95, 0.7))));
    });
}

// ── Nyx, the Threshold Witch: an ornate, ominous seer (princess-tier detail) ──
// Layered star-trimmed robe, draped mantle, deep hood with burning eyes, a horned
// crescent diadem, and a scrying crystal cupped in her hands. Front faces +Z.
fn build_mystic(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>, base: Vec3) {
    let robe  = materials.add(StandardMaterial { base_color: Color::srgb(0.09, 0.05, 0.17), perceptual_roughness: 0.7, ..default() });
    let robe2 = materials.add(StandardMaterial { base_color: Color::srgb(0.14, 0.07, 0.24), perceptual_roughness: 0.6, ..default() });
    let mantle= materials.add(StandardMaterial { base_color: Color::srgb(0.05, 0.02, 0.10), perceptual_roughness: 0.8, ..default() });
    let trim  = materials.add(StandardMaterial { base_color: Color::srgb(0.6, 0.3, 1.0), emissive: LinearRgba::new(3.0, 0.8, 5.0, 1.0), unlit: true, ..default() });
    let skin  = materials.add(StandardMaterial { base_color: Color::srgb(0.66, 0.64, 0.76), perceptual_roughness: 0.55, ..default() });
    let eye   = materials.add(StandardMaterial { base_color: Color::srgb(0.95, 0.5, 1.0), emissive: LinearRgba::new(7.0, 2.0, 9.0, 1.0), unlit: true, ..default() });
    let gold  = materials.add(StandardMaterial { base_color: Color::srgb(0.92, 0.78, 0.4), emissive: LinearRgba::new(2.2, 1.5, 0.5, 1.0), metallic: 0.9, perceptual_roughness: 0.25, ..default() });
    let crystal = materials.add(StandardMaterial { base_color: Color::srgb(0.7, 0.45, 1.0), emissive: LinearRgba::new(3.0, 1.0, 6.0, 1.0), unlit: true, ..default() });

    let root = commands.spawn((Transform::from_translation(base), GlobalTransform::default(), Visibility::default(),
        Mystic, NpcIdle { base_y: base.y, phase: 0.0 })).id();

    // ── Layered flowing gown (3 tapering skirts) + glowing star-hem ──
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.62, height: 1.5 }.mesh().resolution(18))), MeshMaterial3d(robe.clone()),
        Transform::from_xyz(0.0, 0.75, 0.0).with_rotation(Quat::from_rotation_x(std::f32::consts::PI)))).set_parent(root);
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.5, height: 1.2 }.mesh().resolution(18))), MeshMaterial3d(robe2.clone()),
        Transform::from_xyz(0.0, 1.25, 0.04).with_rotation(Quat::from_rotation_x(std::f32::consts::PI)))).set_parent(root);
    commands.spawn((Mesh3d(meshes.add(Annulus::new(0.58, 0.66))), MeshMaterial3d(trim.clone()),
        Transform::from_xyz(0.0, 0.06, 0.0).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)))).set_parent(root);
    // little glowing stars scattered on the gown
    for k in 0..7u32 {
        let a = k as f32 / 7.0 * std::f32::consts::TAU;
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.04))), MeshMaterial3d(trim.clone()),
            Transform::from_xyz(a.cos() * 0.4, 0.6 + (k % 3) as f32 * 0.35, -0.42 + a.sin() * 0.1))).set_parent(root);
    }
    // ── Mantle draped over the shoulders ──
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.5, height: 0.5 }.mesh().resolution(16))), MeshMaterial3d(mantle.clone()),
        Transform::from_xyz(0.0, 1.62, 0.0))).set_parent(root);
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.16, 0.4, 0.16))), MeshMaterial3d(trim.clone()),
        Transform::from_xyz(0.0, 1.7, 0.18))).set_parent(root);   // clasp
    // ── Neck + pale spectral face ──
    commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.08, half_height: 0.1 })), MeshMaterial3d(skin.clone()),
        Transform::from_xyz(0.0, 1.86, 0.0))).set_parent(root);
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.24))), MeshMaterial3d(skin.clone()),
        Transform::from_xyz(0.0, 2.08, 0.0))).set_parent(root);
    // deep hood shell + peaked cowl framing the face
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.33))), MeshMaterial3d(robe.clone()),
        Transform::from_xyz(0.0, 2.12, -0.06).with_scale(Vec3::new(1.1, 1.15, 1.05)))).set_parent(root);
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.36, height: 0.7 }.mesh().resolution(12))), MeshMaterial3d(robe.clone()),
        Transform::from_xyz(0.0, 2.45, -0.04))).set_parent(root);
    // burning eyes + faint smile
    for s in [-1.0f32, 1.0] {
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.045))), MeshMaterial3d(eye.clone()),
            Transform::from_xyz(s * 0.09, 2.10, 0.20).with_scale(Vec3::new(1.4, 0.7, 0.6)))).set_parent(root);
    }
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.10, 0.02, 0.03))), MeshMaterial3d(trim.clone()),
        Transform::from_xyz(0.0, 1.99, 0.21))).set_parent(root);
    // ── Horned crescent diadem ──
    commands.spawn((Mesh3d(meshes.add(Annulus::new(0.26, 0.30))), MeshMaterial3d(gold.clone()),
        Transform::from_xyz(0.0, 2.28, 0.16).with_rotation(Quat::from_rotation_x(-0.5)))).set_parent(root);
    for s in [-1.0f32, 1.0] {
        commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.05, height: 0.4 }.mesh().resolution(4))), MeshMaterial3d(gold.clone()),
            Transform::from_xyz(s * 0.24, 2.5, 0.0).with_rotation(Quat::from_rotation_z(s * 0.7)))).set_parent(root);  // crescent horns
    }
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.05))), MeshMaterial3d(crystal.clone()),
        Transform::from_xyz(0.0, 2.42, 0.18))).set_parent(root);   // brow gem
    // ── Long draped sleeves reaching forward, cupped hands ──
    for s in [-1.0f32, 1.0] {
        commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.13, height: 0.8 }.mesh().resolution(8))), MeshMaterial3d(robe2.clone()),
            Transform::from_xyz(s * 0.34, 1.5, 0.1).with_rotation(Quat::from_rotation_x(-1.1) * Quat::from_rotation_z(s * 0.2)))).set_parent(root);
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.08))), MeshMaterial3d(skin.clone()),
            Transform::from_xyz(s * 0.26, 1.2, 0.5))).set_parent(root);
    }
    // ── Scrying crystal hovering in her cupped hands ──
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.15))), MeshMaterial3d(crystal.clone()),
        Transform::from_xyz(0.0, 1.34, 0.52))).set_parent(root);
    for k in 0..4u32 {
        let a = k as f32 / 4.0 * std::f32::consts::TAU;
        commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.03, height: 0.12 }.mesh().resolution(4))), MeshMaterial3d(crystal.clone()),
            Transform::from_xyz(a.cos() * 0.22, 1.34, 0.52 + a.sin() * 0.22).with_rotation(Quat::from_rotation_z(a)))).set_parent(root);
    }
    // ── A faint orbiting wreath of star-motes + her glow ──
    for k in 0..10u32 {
        let a = k as f32 / 10.0 * std::f32::consts::TAU;
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.03))), MeshMaterial3d(trim.clone()),
            Transform::from_xyz(a.cos() * 0.85, 1.7 + (a * 2.0).sin() * 0.4, a.sin() * 0.85))).set_parent(root);
    }
    commands.spawn((PointLight { color: Color::srgb(0.7, 0.4, 1.0), intensity: 220_000.0, range: 14.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(0.0, 1.6, 0.4))).set_parent(root);
}

// Gentle idle sway for the robed figures (Nyx, the hut witch).
fn npc_idle(time: Res<Time>, mut q: Query<(&mut Transform, &NpcIdle)>) {
    let t = time.elapsed_secs();
    for (mut tr, n) in q.iter_mut() {
        tr.translation.y = n.base_y + (t * 1.1 + n.phase).sin() * 0.06;
        tr.rotation = Quat::from_rotation_y((t * 0.25 + n.phase).sin() * 0.14);
    }
}

// Show the "[R] Interact" hint whenever the player is near an interactable.
fn npc_proximity(
    warp: Res<Warp>,
    talk: Res<MysticTalk>,
    player_q: Query<&Transform, With<Player>>,
    targets: Query<&GlobalTransform, Or<(With<Mystic>, With<HutWitch>, With<RuinedBlade>)>>,
    mut hint_q: Query<&mut Visibility, With<InteractHint>>,
) {
    let Ok(pt) = player_q.get_single() else { return; };
    let busy = warp.stage != 0 || talk.stage != 0;
    let near = !busy && targets.iter().any(|g| g.translation().distance(pt.translation) < 4.5);
    for mut v in hint_q.iter_mut() { *v = if near { Visibility::Visible } else { Visibility::Hidden }; }
}

// Press R near Nyx to begin her offer.
fn mystic_interact(
    key: Res<ButtonInput<KeyCode>>,
    mut talk: ResMut<MysticTalk>,
    warp: Res<Warp>,
    player_q: Query<&Transform, With<Player>>,
    mystic_q: Query<&GlobalTransform, With<Mystic>>,
    mut commands: Commands,
) {
    if warp.stage != 0 || talk.stage != 0 || !key.just_pressed(KeyCode::KeyR) { return; }
    let pp = player_q.single().translation;
    if !mystic_q.iter().any(|g| g.translation().distance(pp) < 4.5) { return; }
    talk.stage = 1; talk.timer = 0.0; talk.line = 0;
    commands.spawn((
        Node { position_type: PositionType::Absolute, left: Val::Percent(50.0), bottom: Val::Px(150.0),
               margin: UiRect::left(Val::Px(-360.0)), width: Val::Px(720.0), padding: UiRect::all(Val::Px(22.0)),
               flex_direction: FlexDirection::Column, row_gap: Val::Px(10.0), ..default() },
        BackgroundColor(Color::srgba(0.06, 0.02, 0.1, 0.88)), BorderColor(Color::srgb(0.7, 0.4, 1.0)),
        MysticProp,
    )).with_children(|p| {
        p.spawn((Text::new("Nyx, the Threshold Witch"), TextFont { font_size: 24.0, ..default() }, TextColor(Color::srgb(0.85, 0.5, 1.0))));
        p.spawn((Text::new(MYSTIC_LINES[0]), TextFont { font_size: 23.0, ..default() }, TextColor(Color::srgb(0.95, 0.92, 1.0)), MysticLine));
    });
}

// Drive Nyx's monologue → present the two pills → branch on the player's choice.
fn mystic_talk(
    time: Res<Time>,
    key: Res<ButtonInput<KeyCode>>,
    mut talk: ResMut<MysticTalk>,
    mut warp: ResMut<Warp>,
    mut line_q: Query<&mut Text, With<MysticLine>>,
    prop_q: Query<Entity, With<MysticProp>>,
    mystic_q: Query<&GlobalTransform, With<Mystic>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if talk.stage == 0 { return; }
    talk.timer += time.delta_secs();
    match talk.stage {
        1 => {
            let idx = (talk.timer / 2.8) as usize;
            if idx != talk.line && idx < MYSTIC_LINES.len() {
                talk.line = idx;
                if let Ok(mut t) = line_q.get_single_mut() { t.0 = MYSTIC_LINES[idx].to_string(); }
            }
            if talk.timer > MYSTIC_LINES.len() as f32 * 2.8 {
                talk.stage = 2;
                // Two glowing pills floating over Nyx's outstretched hands
                if let Ok(g) = mystic_q.get_single() {
                    let base = g.translation();
                    let red = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.1, 0.1), emissive: LinearRgba::new(8.0, 0.4, 0.4, 1.0), unlit: true, ..default() });
                    let blue= materials.add(StandardMaterial { base_color: Color::srgb(0.2, 0.4, 1.0), emissive: LinearRgba::new(0.6, 1.6, 9.0, 1.0), unlit: true, ..default() });
                    let pill = meshes.add(Sphere::new(0.12));
                    commands.spawn((Mesh3d(pill.clone()), MeshMaterial3d(red), Transform::from_xyz(base.x - 0.45, 1.3, base.z + 0.5).with_scale(Vec3::new(1.0, 0.6, 1.0)), MysticProp));
                    commands.spawn((Mesh3d(pill), MeshMaterial3d(blue), Transform::from_xyz(base.x + 0.45, 1.3, base.z + 0.5).with_scale(Vec3::new(1.0, 0.6, 1.0)), MysticProp));
                }
                if let Ok(mut t) = line_q.get_single_mut() {
                    t.0 = "[1]  the RED pill        [2]  the BLUE pill".to_string();
                }
            }
        }
        _ => {
            let choose_red  = key.just_pressed(KeyCode::Digit1);
            let choose_blue = key.just_pressed(KeyCode::Digit2);
            if choose_red || choose_blue {
                for e in prop_q.iter() { commands.entity(e).despawn_recursive(); }
                talk.stage = 0; talk.timer = 0.0;
                warp.stage = 1; warp.timer = 0.0;
                warp.dest = if choose_blue { 1 } else { 2 };   // blue→Shadow Isles, red→Hut
            }
        }
    }
}

// The reality-warp: fade to black, build/teleport, fade back in, announce the area.
fn warp_system(
    time: Res<Time>,
    mut warp: ResMut<Warp>,
    mut areas: ResMut<Areas>,
    mut hut_talk: ResMut<HutTalk>,
    mut clear: ResMut<ClearColor>,
    mut ambient: ResMut<AmbientLight>,
    mut sun_q: Query<&mut DirectionalLight>,
    mut fog_q: Query<&mut DistanceFog, With<PlayerCamera>>,
    mut fade_q: Query<(&mut BackgroundColor, &mut Visibility), With<FadeOverlay>>,
    mut player_q: Query<(&mut Transform, &mut PlayerVelocity), With<Player>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if warp.stage == 0 { return; }
    warp.timer += time.delta_secs();
    let set_fade = |fade_q: &mut Query<(&mut BackgroundColor, &mut Visibility), With<FadeOverlay>>, a: f32| {
        for (mut bg, mut v) in fade_q.iter_mut() {
            bg.0 = Color::srgba(0.0, 0.0, 0.0, a.clamp(0.0, 1.0));
            *v = if a > 0.001 { Visibility::Visible } else { Visibility::Hidden };
        }
    };
    match warp.stage {
        1 => {
            set_fade(&mut fade_q, warp.timer / 1.2);
            if warp.timer >= 1.2 {
                // ── Arrive ──
                let (mut pt, mut pv) = player_q.single_mut();
                pv.vertical = 0.0; pv.knockback = Vec3::ZERO; pt.rotation = Quat::IDENTITY;
                match warp.dest {
                    1 => {
                        if !areas.shadow_built { build_shadow_isles(&mut commands, &mut meshes, &mut materials); areas.shadow_built = true; }
                        areas.in_shadow = true; areas.in_hut = false;
                        pt.translation = SHADOW_ORIGIN + Vec3::new(0.0, 0.0, 120.0);
                        // Lit gloom (20% dimmer): the mist glows rather than swallows.
                        clear.0 = Color::srgb(0.04, 0.09, 0.07);
                        ambient.color = Color::srgb(0.34, 0.5, 0.42); ambient.brightness = 480.0;
                        if let Ok(mut s) = sun_q.get_single_mut() { s.color = Color::srgb(0.55, 0.8, 0.65); s.illuminance = 3360.0; }
                        if let Ok(mut f) = fog_q.get_single_mut() { f.color = Color::srgb(0.08, 0.18, 0.15); f.falloff = FogFalloff::Linear { start: 40.0, end: 520.0 }; }
                        // dramatic area title (handled by area_title_tick)
                        commands.spawn((
                            Node { position_type: PositionType::Absolute, top: Val::Percent(42.0), left: Val::Percent(50.0),
                                   margin: UiRect::left(Val::Px(-300.0)), width: Val::Px(600.0),
                                   justify_content: JustifyContent::Center, ..default() },
                            AreaTitle { life: 2.0 },
                        )).with_children(|p| {
                            p.spawn((Text::new("SHADOW ISLES"), TextFont { font_size: 64.0, ..default() },
                                TextColor(Color::srgb(0.5, 1.0, 0.7)), TextLayout::new_with_justify(JustifyText::Center)));
                        });
                    }
                    2 => {
                        if !areas.hut_built { build_hut_realm(&mut commands, &mut meshes, &mut materials); areas.hut_built = true; }
                        areas.in_hut = true; areas.in_shadow = false;
                        pt.translation = HUT_ORIGIN + Vec3::new(0.0, 0.0, 40.0);
                        clear.0 = Color::srgb(0.0, 0.0, 0.0);
                        ambient.color = Color::srgb(0.03, 0.03, 0.05); ambient.brightness = 25.0;
                        if let Ok(mut s) = sun_q.get_single_mut() { s.color = Color::srgb(0.1, 0.1, 0.15); s.illuminance = 0.0; }
                        if let Ok(mut f) = fog_q.get_single_mut() { f.color = Color::srgb(0.0, 0.0, 0.0); f.falloff = FogFalloff::Linear { start: 6.0, end: 42.0 }; }
                        // open the witch's dialogue
                        hut_talk.line = 0; hut_talk.active = true;
                        commands.spawn((
                            Node { position_type: PositionType::Absolute, left: Val::Percent(50.0), bottom: Val::Px(150.0),
                                   margin: UiRect::left(Val::Px(-360.0)), width: Val::Px(720.0), padding: UiRect::all(Val::Px(22.0)),
                                   flex_direction: FlexDirection::Column, row_gap: Val::Px(10.0), ..default() },
                            BackgroundColor(Color::srgba(0.05, 0.03, 0.02, 0.9)), BorderColor(Color::srgb(0.9, 0.6, 0.2)), HutProp,
                        )).with_children(|p| {
                            p.spawn((Text::new("The Hut Witch"), TextFont { font_size: 24.0, ..default() }, TextColor(Color::srgb(1.0, 0.7, 0.3))));
                            p.spawn((Text::new(HUT_LINES[0]), TextFont { font_size: 23.0, ..default() }, TextColor(Color::srgb(0.97, 0.92, 0.85)), HutLine));
                            p.spawn((Text::new("[ R ] continue"), TextFont { font_size: 17.0, ..default() }, TextColor(Color::srgb(0.6, 0.6, 0.6))));
                        });
                    }
                    _ => {
                        areas.in_shadow = false; areas.in_hut = false;
                        pt.translation = Vec3::new(0.0, 0.0, 18.0);
                        clear.0 = Color::srgb(0.018, 0.025, 0.06);
                        ambient.color = Color::srgb(0.32, 0.37, 0.58); ambient.brightness = 416.0;
                        if let Ok(mut s) = sun_q.get_single_mut() { s.color = Color::srgb(0.62, 0.72, 1.0); s.illuminance = 2860.0; }
                        if let Ok(mut f) = fog_q.get_single_mut() { f.color = Color::srgb(0.04, 0.05, 0.11); f.falloff = FogFalloff::Linear { start: 110.0, end: 1150.0 }; }
                    }
                }
                warp.stage = 2; warp.timer = 0.0;
            }
        }
        _ => {
            set_fade(&mut fade_q, 1.0 - warp.timer / 1.2);
            if warp.timer >= 1.2 { warp.stage = 0; warp.timer = 0.0; set_fade(&mut fade_q, 0.0); }
        }
    }
}

// Hold the area title for 2s, then fade & remove it.
fn area_title_tick(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut AreaTitle, &Children)>,
    mut text_q: Query<&mut TextColor>,
) {
    for (e, mut a, children) in q.iter_mut() {
        a.life -= time.delta_secs();
        let alpha = (a.life / 0.6).clamp(0.0, 1.0);
        for &c in children.iter() {
            if let Ok(mut tc) = text_q.get_mut(c) {
                let col = tc.0.to_srgba();
                tc.0 = Color::srgba(col.red, col.green, col.blue, alpha);
            }
        }
        if a.life <= 0.0 { commands.entity(e).despawn_recursive(); }
    }
}

// Advance the witch's revelation; the final line sends the player home.
fn hut_witch_talk(
    key: Res<ButtonInput<KeyCode>>,
    areas: Res<Areas>,
    mut hut_talk: ResMut<HutTalk>,
    mut warp: ResMut<Warp>,
    mut arts: ResMut<Artifacts>,
    mut line_q: Query<&mut Text, With<HutLine>>,
    panel_q: Query<Entity, With<HutProp>>,
    mut commands: Commands,
) {
    if !areas.in_hut || !hut_talk.active || warp.stage != 0 || !key.just_pressed(KeyCode::KeyR) { return; }
    hut_talk.line += 1;
    if hut_talk.line < HUT_LINES.len() {
        if let Ok(mut t) = line_q.get_single_mut() { t.0 = HUT_LINES[hut_talk.line].to_string(); }
    } else {
        // revelation over → she bestows the Black Hole, then sends you back
        hut_talk.active = false;
        arts.has_blackhole = true;
        arts.selected = ArtifactKind::BlackHole;
        for e in panel_q.iter() { commands.entity(e).despawn_recursive(); }
        warp.stage = 1; warp.timer = 0.0; warp.dest = 3;
    }
}

// Heavy Shadow-Isles downpour (≈4× the trident rain), falling around the player.
fn shadow_weather(
    time: Res<Time>,
    areas: Res<Areas>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_q: Query<&Transform, With<Player>>,
    mut drops: Query<(Entity, &mut Transform, &ShadowRain), Without<Player>>,
) {
    let hash = |n: f32| (n.sin() * 43758.5).fract().abs();
    if !areas.in_shadow {
        if !drops.is_empty() { for (e, _, _) in drops.iter() { commands.entity(e).despawn_recursive(); } }
        return;
    }
    let pp = if let Ok(p) = player_q.get_single() { p.translation } else { return; };
    if drops.is_empty() {
        // Small, gray rain streaks (much finer than the trident storm)
        let drop_mat = materials.add(StandardMaterial { base_color: Color::srgba(0.62, 0.66, 0.68, 0.5),
            emissive: LinearRgba::new(0.25, 0.27, 0.30, 1.0), unlit: true, alpha_mode: AlphaMode::Blend, ..default() });
        let drop_mesh = meshes.add(Cuboid::new(0.012, 0.5, 0.012));
        for i in 0..4800u32 {
            let a = hash(i as f32 * 1.3) * std::f32::consts::TAU;
            let d = hash(i as f32 * 5.9) * 75.0;
            let y = hash(i as f32 * 7.7) * 70.0;
            commands.spawn((
                Mesh3d(drop_mesh.clone()), MeshMaterial3d(drop_mat.clone()),
                Transform::from_xyz(pp.x + a.cos() * d, y, pp.z + a.sin() * d),
                ShadowRain { vel: 75.0 + hash(i as f32 * 9.1) * 35.0 }, NotShadowCaster,
            ));
        }
    }
    let dt = time.delta_secs();
    for (_e, mut t, d) in drops.iter_mut() {
        t.translation.y -= d.vel * dt;
        if t.translation.y < 0.0 || (t.translation.x - pp.x).abs() > 85.0 || (t.translation.z - pp.z).abs() > 85.0 {
            let a = hash(t.translation.x * 12.9 + t.translation.z * 78.2) * std::f32::consts::TAU;
            let dd = hash(t.translation.z * 3.3 + 1.7) * 75.0;
            t.translation = Vec3::new(pp.x + a.cos() * dd, 60.0 + hash(t.translation.x) * 12.0, pp.z + a.sin() * dd);
        }
    }
}

// Giant white lightning strikes every 5–10s across the Shadow Isles.
fn shadow_lightning(
    time: Res<Time>,
    mut areas: ResMut<Areas>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_q: Query<&Transform, With<Player>>,
) {
    if !areas.in_shadow { return; }
    areas.bolt_timer -= time.delta_secs();
    if areas.bolt_timer > 0.0 { return; }
    let et = time.elapsed_secs();
    areas.bolt_timer = 5.0 + (et * 1.7).fract() * 5.0;
    let pp = if let Ok(p) = player_q.get_single() { p.translation } else { return; };
    let a = (et * 3.1).fract() * std::f32::consts::TAU;
    let r = 12.0 + (et * 5.3).fract() * 38.0;
    let ground = Vec3::new(pp.x + a.cos() * r, 0.0, pp.z + a.sin() * r);

    let bolt = materials.add(StandardMaterial { base_color: Color::WHITE, emissive: LinearRgba::new(40.0, 42.0, 60.0, 1.0), unlit: true, ..default() });
    let seg = meshes.add(Cuboid::new(0.6, 1.0, 0.6));
    let root = commands.spawn((Transform::default(), GlobalTransform::default(), Visibility::default(), Transient { life: 0.35 })).id();
    // jagged descending bolt
    let top = 150.0f32;
    let n = 10u32;
    let mut prev = Vec3::new(ground.x, top, ground.z);
    for k in 1..=n {
        let f = k as f32 / n as f32;
        let jitter = (1.0 - f) * 6.0;
        let h = (et * 9.0 + k as f32 * 3.7).sin() * jitter;
        let h2 = (et * 7.0 + k as f32 * 2.3).cos() * jitter;
        let next = Vec3::new(ground.x + h, top * (1.0 - f), ground.z + h2);
        let mid = (prev + next) * 0.5;
        let d = next - prev;
        let len = d.length().max(0.01);
        commands.spawn((Mesh3d(seg.clone()), MeshMaterial3d(bolt.clone()),
            Transform::from_translation(mid).with_rotation(Quat::from_rotation_arc(Vec3::Y, d / len)).with_scale(Vec3::new(1.0, len, 1.0)))).set_parent(root);
        prev = next;
    }
    // blinding flash + ground burst
    commands.spawn((PointLight { color: Color::srgb(0.85, 0.95, 1.0), intensity: 40_000_000.0, range: 400.0, shadows_enabled: false, ..default() },
        Transform::from_translation(ground + Vec3::Y * 30.0))).set_parent(root);
    commands.spawn((Mesh3d(meshes.add(Annulus::new(0.5, 4.0))), MeshMaterial3d(bolt.clone()),
        Transform::from_translation(ground + Vec3::Y * 0.1).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)))).set_parent(root);
}

// Press R at the pedestal to claim the Blade of the Ruined King → sent home.
// Approach the blade + R → it poses its riddle. Answer 2 to claim it; answer
// wrong and it unmakes you on the spot.
fn blade_riddle(
    key: Res<ButtonInput<KeyCode>>,
    areas: Res<Areas>,
    mut warp: ResMut<Warp>,
    mut riddle: ResMut<BladeRiddle>,
    mut inv: ResMut<Inventory>,
    mut health: ResMut<PlayerHealth>,
    player_q: Query<&Transform, With<Player>>,
    blade_q: Query<(Entity, &GlobalTransform), With<RuinedBlade>>,
    panel_q: Query<Entity, With<BladeRiddlePanel>>,
    mut commands: Commands,
) {
    if warp.stage != 0 { return; }
    if !riddle.active {
        if !areas.in_shadow || !key.just_pressed(KeyCode::KeyR) { return; }
        let pp = player_q.single().translation;
        if !blade_q.iter().any(|(_, g)| g.translation().distance(pp) < 4.5) { return; }
        riddle.active = true;
        commands.spawn((
            Node { position_type: PositionType::Absolute, left: Val::Percent(50.0), bottom: Val::Px(120.0),
                   margin: UiRect::left(Val::Px(-380.0)), width: Val::Px(760.0), padding: UiRect::all(Val::Px(24.0)),
                   flex_direction: FlexDirection::Column, ..default() },
            BackgroundColor(Color::srgba(0.02, 0.08, 0.06, 0.92)), BorderColor(Color::srgb(0.4, 1.0, 0.7)),
            BladeRiddlePanel,
        )).with_children(|p| {
            p.spawn((Text::new(BLADE_RIDDLE), TextFont { font_size: 22.0, ..default() }, TextColor(Color::srgb(0.7, 1.0, 0.85))));
        });
        return;
    }
    // awaiting the answer
    let ans = if key.just_pressed(KeyCode::Digit1) { 1 }
              else if key.just_pressed(KeyCode::Digit2) { 2 }
              else if key.just_pressed(KeyCode::Digit3) { 3 } else { 0 };
    if ans == 0 { return; }
    riddle.active = false;
    for e in panel_q.iter() { commands.entity(e).despawn_recursive(); }
    if ans == 2 {
        // correct → the blade is yours; it supplants the steel sword
        inv.has_ruined_blade = true;
        inv.selected = ItemKind::Sword;
        for (e, _) in blade_q.iter() { commands.entity(e).despawn_recursive(); }
        commands.spawn((
            Node { position_type: PositionType::Absolute, top: Val::Percent(40.0), left: Val::Percent(50.0),
                   margin: UiRect::left(Val::Px(-340.0)), width: Val::Px(680.0), justify_content: JustifyContent::Center, ..default() },
            AreaTitle { life: 3.0 },
        )).with_children(|p| {
            p.spawn((Text::new("The Blade of the Ruined King is yours"), TextFont { font_size: 34.0, ..default() },
                TextColor(Color::srgb(0.5, 1.0, 0.8)), TextLayout::new_with_justify(JustifyText::Center)));
        });
        warp.stage = 1; warp.timer = 0.0; warp.dest = 3;
    } else {
        // wrong → unmade where you stand
        health.hp = 0.0;
    }
}

// Build a translucent, glowing-green mist zombie at `pos` that fights for the player.
fn spawn_mist_zombie(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>, pos: Vec3) {
    let mist = materials.add(StandardMaterial { base_color: Color::srgba(0.4, 1.0, 0.6, 0.65),
        emissive: LinearRgba::new(0.5, 3.0, 1.4, 1.0), unlit: true, alpha_mode: AlphaMode::Add, ..default() });
    let root = commands.spawn((Transform::from_xyz(pos.x, 0.0, pos.z), GlobalTransform::default(), Visibility::default(),
        MistZombie { life: 20.0, attack: 0.0 })).id();
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.4, height: 1.6 }.mesh().resolution(8))), MeshMaterial3d(mist.clone()),
        Transform::from_xyz(0.0, 0.8, 0.0).with_rotation(Quat::from_rotation_x(std::f32::consts::PI)))).set_parent(root);
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.28))), MeshMaterial3d(mist.clone()), Transform::from_xyz(0.0, 1.7, 0.0))).set_parent(root);
    for s in [-1.0f32, 1.0] {
        commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.09, half_height: 0.5 })), MeshMaterial3d(mist.clone()),
            Transform::from_xyz(s * 0.4, 1.1, 0.1).with_rotation(Quat::from_rotation_x(-0.6)))).set_parent(root);
    }
    commands.spawn((PointLight { color: Color::srgb(0.4, 1.0, 0.5), intensity: 70_000.0, range: 7.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(0.0, 1.3, 0.0))).set_parent(root);
}

// Mist zombies seek the nearest living foe, claw at it, and dissolve after 20s.
fn mist_zombie_ai(
    time: Res<Time>,
    mut commands: Commands,
    mut kills: ResMut<KillStats>,
    mut zombies: Query<(Entity, &mut Transform, &mut MistZombie)>,
    mut skel_q: Query<(Entity, &GlobalTransform, &mut Skeleton), Without<MistZombie>>,
    mut enemy_q: Query<(Entity, &GlobalTransform, &mut Enemy), (Without<MistZombie>, Without<Skeleton>)>,
) {
    let dt = time.delta_secs();
    let et = time.elapsed_secs();
    for (ze, mut zt, mut z) in zombies.iter_mut() {
        z.life -= dt;
        z.attack -= dt;
        if z.life <= 0.0 { commands.entity(ze).despawn_recursive(); continue; }
        // find nearest living target among skeletons + beasts
        let mut best: Option<(f32, Vec3)> = None;
        for (_, g, sk) in skel_q.iter() {
            if sk.state == SkeletonState::Dead { continue; }
            let d = g.translation().distance(zt.translation);
            if best.map_or(true, |(bd, _)| d < bd) { best = Some((d, g.translation())); }
        }
        for (_, g, _) in enemy_q.iter() {
            let d = g.translation().distance(zt.translation);
            if best.map_or(true, |(bd, _)| d < bd) { best = Some((d, g.translation())); }
        }
        let Some((dist, tpos)) = best else {
            // nothing to fight — drift idly
            zt.translation.y = (et * 3.0).sin() * 0.1;
            continue;
        };
        let dir = Vec3::new(tpos.x - zt.translation.x, 0.0, tpos.z - zt.translation.z).normalize_or_zero();
        if dist > 1.6 { zt.translation += dir * 8.0 * dt; }
        zt.translation.y = (et * 6.0).sin() * 0.12;
        if dist < 0.01 { continue; }
        zt.look_to(-dir, Vec3::Y);
        if z.attack <= 0.0 && dist < 2.4 {
            z.attack = 0.6;
            // strike the nearest skeleton, else beast
            let mut hit = false;
            for (e, g, mut sk) in skel_q.iter_mut() {
                if sk.state != SkeletonState::Dead && g.translation().distance(zt.translation) < 2.4 {
                    sk.health -= 3.0; sk.damage_flash = 0.2;
                    if sk.health <= 0.0 { sk.state = SkeletonState::Dead; kills.skeletons += 1; commands.entity(e).despawn_recursive(); }
                    hit = true; break;
                }
            }
            if !hit {
                for (e, g, mut en) in enemy_q.iter_mut() {
                    if g.translation().distance(zt.translation) < 2.4 {
                        en.health -= 3.0; en.damage_flash = 0.2;
                        if en.health <= 0.0 { kills.beasts += 1; commands.entity(e).despawn_recursive(); }
                        break;
                    }
                }
            }
        }
    }
}

// ── Black Hole artifact: Q drains ALL mana to hurl a slow void that erases reality ──
fn blackhole_cast(
    key: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    arts: Res<Artifacts>,
    mut mana: ResMut<Mana>,
    camera_q: Query<&GlobalTransform, With<PlayerCamera>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let window = windows.single();
    if !(key.just_pressed(KeyCode::KeyQ) && arts.selected == ArtifactKind::BlackHole
        && window.cursor_options.grab_mode == CursorGrabMode::Locked && mana.current > 5.0) { return; }
    mana.current = 0.0;                          // consumes ALL mana
    let cam = camera_q.single();
    let (_, rot, pos) = cam.to_scale_rotation_translation();
    let fwd = rot * Vec3::NEG_Z;
    let black = materials.add(StandardMaterial { base_color: Color::BLACK, unlit: true, ..default() });
    let proj = commands.spawn((
        Transform::from_translation(pos + fwd * 1.5), GlobalTransform::default(), Visibility::default(),
        BlackHoleProj { vel: fwd * 14.0, life: 2.2 },
    )).id();
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.45))), MeshMaterial3d(black), Transform::default(), BlackHoleVis)).set_parent(proj);
}

// The thrown void glides forward, then collapses into a growing black hole that
// births its inward-streaming particles + a permanent void scar on the ground.
fn blackhole_proj_update(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(Entity, &mut Transform, &mut BlackHoleProj)>,
) {
    let dt = time.delta_secs();
    for (e, mut t, mut p) in q.iter_mut() {
        t.translation += p.vel * dt;
        p.life -= dt;
        if p.life <= 0.0 {
            let pos = t.translation;
            commands.entity(e).despawn_recursive();
            let black = materials.add(StandardMaterial { base_color: Color::BLACK, unlit: true, ..default() });
            let white = materials.add(StandardMaterial { base_color: Color::WHITE, emissive: LinearRgba::new(8.0, 8.0, 9.0, 1.0), unlit: true, ..default() });
            let rubble = materials.add(StandardMaterial { base_color: Color::srgb(0.05, 0.05, 0.06), perceptual_roughness: 1.0, ..default() });
            // the hole proper
            let cy = pos.y.max(3.0);
            let hole = commands.spawn((
                Transform::from_translation(Vec3::new(pos.x, cy, pos.z)),
                GlobalTransform::default(), Visibility::default(),
                BlackHoleCore { radius: 1.0, age: 0.0 },
            )).id();
            commands.spawn((Mesh3d(meshes.add(Sphere::new(1.0))), MeshMaterial3d(black.clone()), Transform::default(), BlackHoleVis)).set_parent(hole);
            // a white-bordered ring on the ground around the base (spins via blackhole_fx)
            commands.spawn((Mesh3d(meshes.add(Annulus::new(0.9, 1.0))), MeshMaterial3d(white.clone()),
                Transform::from_xyz(pos.x, 0.25, pos.z).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
                BlackHoleRing));
            // inward-streaming black particles (the "suction" effect)
            let pmesh = meshes.add(Cuboid::new(0.4, 0.4, 0.4));
            for k in 0..60u32 {
                let ang = (k as f32 * 2.39966).rem_euclid(std::f32::consts::TAU);
                let dist = 6.0 + (k % 7) as f32 * 2.0;
                let y = ((k * 13 % 9) as f32 - 4.0) * 1.2;
                commands.spawn((Mesh3d(pmesh.clone()), MeshMaterial3d(black.clone()),
                    Transform::from_translation(Vec3::new(pos.x + ang.cos() * dist, cy + y, pos.z + ang.sin() * dist)),
                    BlackHoleParticle { ang, dist, y }));
            }
            // violently spinning ground debris around the base
            let dmesh = meshes.add(Cuboid::new(0.6, 0.4, 0.6));
            for k in 0..20u32 {
                let ang = k as f32 / 20.0 * std::f32::consts::TAU;
                let dist = 3.0 + (k % 5) as f32 * 1.4;
                commands.spawn((Mesh3d(dmesh.clone()), MeshMaterial3d(rubble.clone()),
                    Transform::from_xyz(pos.x + ang.cos() * dist, 0.5, pos.z + ang.sin() * dist),
                    BlackHoleDebris { ang, dist, spin: 6.0 + (k % 4) as f32 * 2.0 }));
            }
        }
    }
}

// The black hole grows, drags in + kills nearby foes, then at the peak of its
// expansion it DETONATES: a map-wide shockwave that slays every monster, leaving
// a crater behind (it no longer erases the world geometry).
fn blackhole_core_update(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cores: Query<(Entity, &mut Transform, &mut BlackHoleCore)>,
    mut skel_q: Query<(Entity, &mut Transform, &mut Skeleton), (Without<BlackHoleCore>, Without<Player>, Without<Enemy>)>,
    mut enemy_q: Query<(Entity, &mut Transform, &mut Enemy), (Without<BlackHoleCore>, Without<Player>, Without<Skeleton>)>,
    mut player_q: Query<(&mut Transform, &mut PlayerVelocity), (With<Player>, Without<BlackHoleCore>, Without<Skeleton>, Without<Enemy>)>,
    mut health: ResMut<PlayerHealth>,
) {
    let dt = time.delta_secs();
    for (e, mut t, mut c) in cores.iter_mut() {
        c.age += dt;
        c.radius = (c.age * 4.0).min(40.0);
        t.scale = Vec3::splat(c.radius);
        t.rotation = Quat::from_rotation_y(c.age * 1.5);
        let center = t.translation;
        let pull = c.radius * 2.4;
        // suck in + crush skeletons
        for (se, mut st, _) in skel_q.iter_mut() {
            let d = st.translation.distance(center);
            if d < pull {
                let dir = (center - st.translation).normalize_or_zero();
                st.translation += dir * (pull - d) * 1.3 * dt;
                if d < c.radius * 1.1 { commands.entity(se).despawn_recursive(); }
            }
        }
        // suck in + crush beasts (Enemy covers succubus/Sauron too)
        for (ee, mut et2, _) in enemy_q.iter_mut() {
            let d = et2.translation.distance(center);
            if d < pull {
                let dir = (center - et2.translation).normalize_or_zero();
                et2.translation += dir * (pull - d) * 1.3 * dt;
                if d < c.radius * 1.1 { commands.entity(ee).despawn_recursive(); }
            }
        }
        // tug the player + flay them if they stand too close
        if let Ok((pt, mut pv)) = player_q.get_single_mut() {
            let d = pt.translation.distance(center);
            if d < pull {
                let dir = (center - pt.translation).normalize_or_zero();
                pv.knockback = dir * (pull - d) * 0.7;
                if d < c.radius * 0.8 && health.hurt_timer <= 0.0 && health.iframes <= 0.0 {
                    health.take(45.0, 0.4);
                }
            }
        }
        // ── At the peak: DETONATE ──
        if c.age > 11.0 {
            commands.entity(e).despawn_recursive();
            // the biggest shockwave — an expanding sphere + ground ring across the map
            let shell = materials.add(StandardMaterial { base_color: Color::srgba(0.8, 0.85, 1.0, 0.35),
                emissive: LinearRgba::new(5.0, 5.0, 7.0, 1.0), unlit: true, alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default() });
            let blast = commands.spawn((
                Transform::from_translation(Vec3::new(center.x, 2.0, center.z)).with_scale(Vec3::splat(1.0)),
                GlobalTransform::default(), Visibility::default(),
                BlackHoleBlast { radius: 1.0, max: 1200.0 },
            )).id();
            commands.spawn((Mesh3d(meshes.add(Sphere::new(1.0))), MeshMaterial3d(shell.clone()), Transform::default(), BlackHoleVis)).set_parent(blast);
            commands.spawn((PointLight { color: Color::srgb(0.8, 0.9, 1.0), intensity: 60_000_000.0, range: 500.0, shadows_enabled: false, ..default() },
                Transform::default(), BlackHoleVis)).set_parent(blast);
            // a permanent crater where the hole stood
            let black = materials.add(StandardMaterial { base_color: Color::BLACK, unlit: true, ..default() });
            commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 14.0, half_height: 0.2 })), MeshMaterial3d(black),
                Transform::from_xyz(center.x, 0.18, center.z), VoidScar));
        }
    }
}

// Stream the black particles inward; spin the white ring + debris; clean up once
// the hole is gone.
fn blackhole_fx(
    time: Res<Time>,
    mut commands: Commands,
    cores: Query<(&Transform, &BlackHoleCore), (Without<BlackHoleParticle>, Without<BlackHoleRing>, Without<BlackHoleDebris>)>,
    mut parts: Query<(Entity, &mut Transform), (With<BlackHoleParticle>, Without<BlackHoleRing>, Without<BlackHoleDebris>)>,
    mut part_data: Query<&mut BlackHoleParticle>,
    mut rings: Query<(Entity, &mut Transform), (With<BlackHoleRing>, Without<BlackHoleParticle>, Without<BlackHoleDebris>)>,
    mut debris: Query<(Entity, &mut Transform, &BlackHoleDebris), (Without<BlackHoleParticle>, Without<BlackHoleRing>)>,
) {
    let dt = time.delta_secs();
    let et = time.elapsed_secs();
    let Some((ct, c)) = cores.iter().next() else {
        for (e, _) in parts.iter() { commands.entity(e).despawn_recursive(); }
        for (e, _) in rings.iter() { commands.entity(e).despawn_recursive(); }
        for (e, _, _) in debris.iter() { commands.entity(e).despawn_recursive(); }
        return;
    };
    let center = ct.translation;
    for (e, mut t) in parts.iter_mut() {
        let Ok(mut p) = part_data.get_mut(e) else { continue; };
        p.dist -= (8.0 + p.dist * 0.6) * dt;
        p.ang += 2.5 * dt;
        p.y *= 1.0 - 0.5 * dt;
        if p.dist < 0.6 {
            p.dist = c.radius.max(4.0) + 2.0;
            p.ang += 1.7;
            p.y = (t.translation.x * 12.9).sin() * c.radius.max(3.0) * 0.5;
        }
        t.translation = center + Vec3::new(p.ang.cos() * p.dist, p.y, p.ang.sin() * p.dist);
        t.scale = Vec3::splat((p.dist * 0.05).clamp(0.15, 0.6));
    }
    // white ground ring sits at the base, spins, and widens with the hole
    for (_, mut t) in rings.iter_mut() {
        let r = (c.radius * 1.4).max(2.0);
        t.translation = Vec3::new(center.x, 0.25, center.z);
        t.rotation = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2) * Quat::from_rotation_z(et * 2.5);
        t.scale = Vec3::splat(r);
    }
    // debris orbits the base, spinning violently
    for (_, mut t, d) in debris.iter_mut() {
        let ang = d.ang + et * d.spin * 0.4;
        t.translation = Vec3::new(center.x + ang.cos() * d.dist, 0.5 + (et * d.spin).sin() * 0.4, center.z + ang.sin() * d.dist);
        t.rotation = Quat::from_euler(EulerRot::XYZ, et * d.spin, et * d.spin * 0.7, et * d.spin * 1.3);
    }
}

// The detonation shockwave races across the map, slaying every monster it passes.
fn blackhole_blast_update(
    time: Res<Time>,
    mut commands: Commands,
    mut blasts: Query<(Entity, &mut Transform, &mut BlackHoleBlast)>,
    skel_q: Query<(Entity, &GlobalTransform), (With<Skeleton>, Without<BlackHoleBlast>)>,
    enemy_q: Query<(Entity, &GlobalTransform), (With<Enemy>, Without<BlackHoleBlast>)>,
    boss_q: Query<(Entity, &GlobalTransform), (Or<(With<Dragon>, With<Medusa>)>, Without<BlackHoleBlast>)>,
) {
    let dt = time.delta_secs();
    for (e, mut t, mut b) in blasts.iter_mut() {
        b.radius += 600.0 * dt;                 // tears across the map fast
        t.scale = Vec3::splat(b.radius);        // expand the spherical shell visual
        let center = t.translation;
        let r = b.radius;
        for (se, g) in skel_q.iter() { if g.translation().distance(center) < r { commands.entity(se).despawn_recursive(); } }
        for (ee, g) in enemy_q.iter() { if g.translation().distance(center) < r { commands.entity(ee).despawn_recursive(); } }
        for (be, g) in boss_q.iter() { if g.translation().distance(center) < r { commands.entity(be).despawn_recursive(); } }
        if b.radius >= b.max { commands.entity(e).despawn_recursive(); }
    }
}

// ── Build the Hut of Revelation: a rope bridge over the void to a glowing cabin ──
fn build_hut_realm(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) {
    let o = HUT_ORIGIN;
    let wood = materials.add(StandardMaterial { base_color: Color::srgb(0.30, 0.18, 0.09), perceptual_roughness: 0.9, ..default() });
    let rope = materials.add(StandardMaterial { base_color: Color::srgb(0.45, 0.36, 0.2), perceptual_roughness: 1.0, ..default() });
    let wall = materials.add(StandardMaterial { base_color: Color::srgb(0.35, 0.22, 0.12), perceptual_roughness: 0.95, ..default() });
    let warm = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.75, 0.4), emissive: LinearRgba::new(4.0, 2.2, 0.7, 1.0), unlit: true, ..default() });

    // Rope bridge: planks from z=+38 down to the hut platform at z=+6
    for i in 0..32u32 {
        let z = o.z + 38.0 - i as f32;
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(1.6, 0.08, 0.7))), MeshMaterial3d(wood.clone()),
            Transform::from_xyz(o.x, 0.02, z), HutProp));
    }
    // rope rails + posts
    for s in [-1.0f32, 1.0] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.05, 0.05, 32.0))), MeshMaterial3d(rope.clone()),
            Transform::from_xyz(o.x + s * 0.85, 0.55, o.z + 22.0), HutProp));
        for i in 0..9u32 {
            commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.06, 0.6, 0.06))), MeshMaterial3d(wood.clone()),
                Transform::from_xyz(o.x + s * 0.85, 0.3, o.z + 38.0 - i as f32 * 4.0), HutProp));
        }
    }
    // Hut platform (walkable) + little cabin
    commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 6.0, half_height: 0.3 })), MeshMaterial3d(wood.clone()),
        Transform::from_xyz(o.x, -0.1, o.z - 1.0), Walkable { half: Vec2::new(6.0, 6.0), top: 0.1 }, HutProp));
    // cabin walls (open front, -Z)
    for (dx, dz, w, d) in [(-2.6f32, -1.0f32, 0.3f32, 5.0f32), (2.6, -1.0, 0.3, 5.0), (0.0, -3.5, 5.2, 0.3)] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(w, 3.2, d))), MeshMaterial3d(wall.clone()),
            Transform::from_xyz(o.x + dx, 1.6, o.z + dz), HutProp));
    }
    // pitched roof
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(6.0, 0.25, 6.0))), MeshMaterial3d(wood.clone()),
        Transform::from_xyz(o.x, 3.3, o.z - 1.0).with_rotation(Quat::from_rotation_z(0.12)), HutProp));
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 4.6, height: 2.4 }.mesh().resolution(4))), MeshMaterial3d(wall.clone()),
        Transform::from_xyz(o.x, 4.2, o.z - 1.0).with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_4)), HutProp));
    // a warm hearth glow + light filling the hut
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.4))), MeshMaterial3d(warm.clone()),
        Transform::from_xyz(o.x + 1.6, 0.8, o.z - 2.4), HutProp));
    commands.spawn((PointLight { color: Color::srgb(1.0, 0.78, 0.45), intensity: 900_000.0, range: 26.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(o.x, 2.2, o.z - 1.0), HutProp));

    // The shadow figure, waiting in the hut, facing the doorway (+Z)
    build_shadow_figure(commands, meshes, materials, Vec3::new(o.x, 0.0, o.z - 2.6));
    commands.spawn((Transform::from_xyz(o.x, 0.0, o.z - 2.6), GlobalTransform::default(), Visibility::default(), HutWitch, HutProp));
}

// A featureless black misty shadow figure — a silhouette of darkness with two cold eyes.
fn build_shadow_figure(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>, pos: Vec3) {
    let shadow = materials.add(StandardMaterial { base_color: Color::srgba(0.01, 0.01, 0.02, 0.9),
        perceptual_roughness: 1.0, alpha_mode: AlphaMode::Blend, ..default() });
    let wisp = materials.add(StandardMaterial { base_color: Color::srgba(0.03, 0.03, 0.05, 0.4),
        unlit: true, alpha_mode: AlphaMode::Blend, cull_mode: None, double_sided: true, ..default() });
    let eye = materials.add(StandardMaterial { base_color: Color::srgb(0.8, 0.85, 1.0),
        emissive: LinearRgba::new(4.0, 4.5, 6.0, 1.0), unlit: true, ..default() });

    let root = commands.spawn((Transform::from_translation(pos), GlobalTransform::default(), Visibility::default(),
        NpcIdle { base_y: pos.y, phase: 0.9 }, HutProp)).id();
    // tall tapering shadow body (a hooded silhouette) + rounded head
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.5, height: 2.2 }.mesh().resolution(14))), MeshMaterial3d(shadow.clone()),
        Transform::from_xyz(0.0, 1.0, 0.0).with_rotation(Quat::from_rotation_x(std::f32::consts::PI)))).set_parent(root);
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.3))), MeshMaterial3d(shadow.clone()),
        Transform::from_xyz(0.0, 2.05, 0.0))).set_parent(root);
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.34, height: 0.6 }.mesh().resolution(10))), MeshMaterial3d(shadow.clone()),
        Transform::from_xyz(0.0, 2.4, -0.02))).set_parent(root);   // wispy hood peak
    // two cold glowing eyes
    for s in [-1.0f32, 1.0] {
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.05))), MeshMaterial3d(eye.clone()),
            Transform::from_xyz(s * 0.1, 2.06, 0.24).with_scale(Vec3::new(1.3, 0.7, 0.6)))).set_parent(root);
    }
    // drifting wisps of darkness around it
    for k in 0..6u32 {
        let a = k as f32 / 6.0 * std::f32::consts::TAU;
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.5, 1.8, 0.05))), MeshMaterial3d(wisp.clone()),
            Transform::from_xyz(a.cos() * 0.5, 1.1, a.sin() * 0.5).with_rotation(Quat::from_rotation_y(a)))).set_parent(root);
    }
}

// ── Build the Shadow Isles: a broken island, dead trees, a dark cathedral with a
//    long hallway, and the Blade of the Ruined King on its pedestal ──
fn build_shadow_isles(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) {
    let o = SHADOW_ORIGIN;
    let rock = materials.add(StandardMaterial { base_color: Color::srgb(0.14, 0.18, 0.16), perceptual_roughness: 1.0, ..default() });
    let stone= materials.add(StandardMaterial { base_color: Color::srgb(0.22, 0.26, 0.25), perceptual_roughness: 0.9, ..default() });
    let pale = materials.add(StandardMaterial { base_color: Color::srgb(0.40, 0.44, 0.43), perceptual_roughness: 0.7, ..default() });  // statues / lighter stone
    let dead = materials.add(StandardMaterial { base_color: Color::srgb(0.08, 0.09, 0.08), perceptual_roughness: 1.0, ..default() });
    let mist = materials.add(StandardMaterial { base_color: Color::srgba(0.55, 0.72, 0.62, 0.06),
        emissive: LinearRgba::new(0.3, 0.6, 0.45, 1.0), unlit: true, alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default() });
    let glow = materials.add(StandardMaterial { base_color: Color::srgb(0.4, 1.0, 0.7), emissive: LinearRgba::new(0.8, 7.0, 4.0, 1.0), unlit: true, ..default() });
    let brazier = materials.add(StandardMaterial { base_color: Color::srgb(0.5, 1.0, 0.7), emissive: LinearRgba::new(1.0, 9.0, 5.0, 1.0), unlit: true, ..default() });

    // ── Broken island top (walkable disc, 5× larger) + jagged underside ──
    commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 600.0, half_height: 6.0 })), MeshMaterial3d(rock.clone()),
        Transform::from_xyz(o.x, -6.0, o.z), Walkable { half: Vec2::new(600.0, 600.0), top: 0.0 }, ShadowProp));
    for i in 0..70u32 {
        let a = i as f32 / 70.0 * std::f32::consts::TAU;
        let d = 420.0 + (a * 3.0).sin() * 120.0;
        commands.spawn((Mesh3d(meshes.add(Cone { radius: 45.0, height: 140.0 + (i % 5) as f32 * 50.0 }.mesh().resolution(5))), MeshMaterial3d(rock.clone()),
            Transform::from_xyz(o.x + a.cos() * d, -70.0, o.z + a.sin() * d).with_rotation(Quat::from_rotation_x(std::f32::consts::PI)), ShadowProp));
    }
    // ── Ring of green braziers lighting the whole isle despite the mist ──
    for i in 0..16u32 {
        let a = i as f32 / 16.0 * std::f32::consts::TAU;
        let d = 120.0 + (i % 3) as f32 * 130.0;
        let px = o.x + a.cos() * d; let pz = o.z + a.sin() * d;
        commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.6, half_height: 3.0 })), MeshMaterial3d(stone.clone()),
            Transform::from_xyz(px, 3.0, pz), ShadowProp));
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.9))), MeshMaterial3d(brazier.clone()),
            Transform::from_xyz(px, 6.4, pz), ShadowProp));
        commands.spawn((PointLight { color: Color::srgb(0.6, 1.0, 0.75), intensity: 9_000_000.0, range: 260.0, shadows_enabled: false, ..default() },
            Transform::from_xyz(px, 10.0, pz), ShadowProp));
    }
    // Keep the cathedral footprint (and its hallway) clear of scenery.
    let in_cathedral = |px: f32, pz: f32| (px - o.x).abs() < 38.0 && pz > o.z - 175.0 && pz < o.z - 50.0;
    // ── A scatter of dead trees + low drifting mist across the isle ──
    for i in 0..90u32 {
        let h = (i as f32 * 12.9).sin().abs();
        let a = (i as f32 * 2.3).sin() * std::f32::consts::TAU;
        let d = 30.0 + h * 520.0;
        let px = o.x + a.cos() * d; let pz = o.z + a.sin() * d;
        if in_cathedral(px, pz) { continue; }
        commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.4, half_height: 3.5 + h * 2.5 })), MeshMaterial3d(dead.clone()),
            Transform::from_xyz(px, 3.5, pz), ShadowProp));
        for b in 0..4u32 {
            let ba = b as f32 * 1.9 + h;
            commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.14, half_height: 1.8 })), MeshMaterial3d(dead.clone()),
                Transform::from_xyz(px + ba.cos() * 0.9, 6.5 + b as f32 * 0.7, pz + ba.sin() * 0.9)
                    .with_rotation(Quat::from_rotation_z(ba.cos() * 0.9) * Quat::from_rotation_x(ba.sin() * 0.9)), ShadowProp));
        }
        if i % 2 == 0 {
            commands.spawn((Mesh3d(meshes.add(Cuboid::new(14.0, 4.0, 14.0))), MeshMaterial3d(mist.clone()),
                Transform::from_xyz(px, 1.8, pz), ShadowProp, NotShadowCaster));
        }
    }
    // ── A graveyard of leaning tombstones near the approach ──
    for r in 0..6u32 { for c in 0..8u32 {
        let px = o.x - 70.0 + c as f32 * 16.0;
        let pz = o.z + 150.0 + r as f32 * 18.0;
        let lean = ((r * 8 + c) as f32 * 1.7).sin() * 0.25;
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(1.6, 2.4, 0.4))), MeshMaterial3d(pale.clone()),
            Transform::from_xyz(px, 1.2, pz).with_rotation(Quat::from_rotation_z(lean)), ShadowProp));
    }}
    // ── Two great winged angel statues on plinths flanking the path (ref art) ──
    for s in [-1.0f32, 1.0] {
        let px = o.x + s * 40.0; let pz = o.z + 60.0;
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(6.0, 10.0, 6.0))), MeshMaterial3d(stone.clone()),
            Transform::from_xyz(px, 5.0, pz), ShadowProp));                                   // plinth
        commands.spawn((Mesh3d(meshes.add(Cone { radius: 1.8, height: 7.0 }.mesh().resolution(8))), MeshMaterial3d(pale.clone()),
            Transform::from_xyz(px, 13.0, pz).with_rotation(Quat::from_rotation_x(std::f32::consts::PI)), ShadowProp));   // robed body
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.9))), MeshMaterial3d(pale.clone()),
            Transform::from_xyz(px, 17.5, pz), ShadowProp));                                  // head
        for w in [-1.0f32, 1.0] {
            commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.4, 7.0, 2.6))), MeshMaterial3d(pale.clone()),
                Transform::from_xyz(px + w * 1.6, 16.0, pz).with_rotation(Quat::from_rotation_z(w * 0.5)), ShadowProp));  // wings
        }
    }
    // ── Ruined free-standing arches dotting the grounds ──
    for i in 0..7u32 {
        let a = (i as f32 * 1.3).sin() * std::f32::consts::TAU;
        let d = 180.0 + (i as f32 * 37.0).sin().abs() * 260.0;
        let px = o.x + a.cos() * d; let pz = o.z + a.sin() * d;
        if in_cathedral(px, pz) { continue; }
        for s in [-1.0f32, 1.0] {
            commands.spawn((Mesh3d(meshes.add(Cuboid::new(2.0, 14.0, 2.0))), MeshMaterial3d(stone.clone()),
                Transform::from_xyz(px + s * 4.0, 7.0, pz), ShadowProp));
        }
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(11.0, 2.0, 2.0))), MeshMaterial3d(stone.clone()),
            Transform::from_xyz(px, 14.5, pz).with_rotation(Quat::from_rotation_z(0.05)), ShadowProp));
    }

    // ── The grand dark cathedral with a real entrance + vaulted hall ──
    let cz = o.z - 110.0;
    build_cathedral(commands, meshes, materials, Vec3::new(o.x, 0.0, cz));

    // ── The Blade of the Ruined King — on the altar deep in the apse ──
    let bz = cz - 42.0;
    commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 1.6, half_height: 0.7 })), MeshMaterial3d(pale.clone()),
        Transform::from_xyz(o.x, 0.7, bz), ShadowProp));
    let blade = commands.spawn((Transform::from_xyz(o.x, 3.0, bz), GlobalTransform::default(), Visibility::default(),
        RuinedBlade, ShadowProp)).id();
    build_ruined_blade(commands, meshes, materials, blade, 1.0);
    // green mist pouring off the blade + an eerie light
    for k in 0..6u32 {
        let ka = k as f32 / 6.0 * std::f32::consts::TAU;
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(1.6, 3.0, 0.05))), MeshMaterial3d(mist.clone()),
            Transform::from_xyz(o.x + ka.cos() * 0.3, 2.8, bz + ka.sin() * 0.3).with_rotation(Quat::from_rotation_y(ka)),
            ShadowProp, NotShadowCaster));
    }
    commands.spawn((PointLight { color: Color::srgb(0.5, 1.0, 0.8), intensity: 4_000_000.0, range: 60.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(o.x, 3.5, bz), ShadowProp));
    let _ = glow;
}

// A grand gothic cathedral: hollow nave you enter through a front doorway, with a
// colonnade, glowing windows, twin entrance towers, a rose window, and an apse.
fn build_cathedral(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>, c: Vec3) {
    let stone = materials.add(StandardMaterial { base_color: Color::srgb(0.20, 0.24, 0.23), perceptual_roughness: 0.9, ..default() });
    let trim  = materials.add(StandardMaterial { base_color: Color::srgb(0.30, 0.34, 0.33), perceptual_roughness: 0.8, ..default() });
    let glass = materials.add(StandardMaterial { base_color: Color::srgb(0.4, 1.0, 0.7), emissive: LinearRgba::new(1.2, 8.0, 5.0, 1.0), unlit: true, ..default() });
    let floor = materials.add(StandardMaterial { base_color: Color::srgb(0.16, 0.19, 0.19), perceptual_roughness: 0.5, metallic: 0.15, ..default() });

    let hw = 15.0f32;   // half-width of the nave
    let hl = 45.0f32;   // half-length
    // nave floor (walkable) running the length of the hall
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(hw * 2.0, 0.5, hl * 2.0))), MeshMaterial3d(floor.clone()),
        Transform::from_xyz(c.x, 0.25, c.z), Walkable { half: Vec2::new(hw, hl), top: 0.5 }, ShadowProp));
    // side walls with regular window slots
    for s in [-1.0f32, 1.0] {
        for i in 0..9u32 {
            let pz = c.z - hl + 6.0 + i as f32 * 10.0;
            commands.spawn((Mesh3d(meshes.add(Cuboid::new(1.4, 26.0, 6.0))), MeshMaterial3d(stone.clone()),
                Transform::from_xyz(c.x + s * hw, 13.0, pz), ShadowProp));                  // pier
            commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.5, 12.0, 3.0))), MeshMaterial3d(glass.clone()),
                Transform::from_xyz(c.x + s * (hw - 0.2), 13.0, pz + 5.0), ShadowProp));     // tall window
            // flying buttress outside
            commands.spawn((Mesh3d(meshes.add(Cuboid::new(4.0, 1.4, 1.4))), MeshMaterial3d(trim.clone()),
                Transform::from_xyz(c.x + s * (hw + 2.5), 16.0, pz).with_rotation(Quat::from_rotation_z(s * 0.6)), ShadowProp));
        }
        // interior colonnade
        for i in 0..7u32 {
            let pz = c.z - hl + 10.0 + i as f32 * 12.0;
            commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 1.0, half_height: 12.0 })), MeshMaterial3d(trim.clone()),
                Transform::from_xyz(c.x + s * (hw - 4.0), 12.0, pz), ShadowProp));
        }
    }
    // apse (solid back wall behind the altar, -Z)
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(hw * 2.0 + 4.0, 30.0, 2.0))), MeshMaterial3d(stone.clone()),
        Transform::from_xyz(c.x, 15.0, c.z - hl), ShadowProp));
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(8.0, 16.0, 0.6))), MeshMaterial3d(glass.clone()),
        Transform::from_xyz(c.x, 16.0, c.z - hl + 1.1), ShadowProp));   // great apse window
    // ── Front facade with a TALL CENTRAL DOORWAY (two flanking wall blocks + lintel) ──
    for s in [-1.0f32, 1.0] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(10.0, 30.0, 2.0))), MeshMaterial3d(stone.clone()),
            Transform::from_xyz(c.x + s * 10.0, 15.0, c.z + hl), ShadowProp));     // facade either side of the door
    }
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(12.0, 8.0, 2.0))), MeshMaterial3d(stone.clone()),
        Transform::from_xyz(c.x, 26.0, c.z + hl), ShadowProp));                    // lintel above the doorway
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 5.0, height: 6.0 }.mesh().resolution(3))), MeshMaterial3d(trim.clone()),
        Transform::from_xyz(c.x, 31.0, c.z + hl), ShadowProp));                    // pointed gable
    // rose window above the entrance
    commands.spawn((Mesh3d(meshes.add(Annulus::new(2.2, 3.2))), MeshMaterial3d(glass.clone()),
        Transform::from_xyz(c.x, 24.0, c.z + hl + 1.1), ShadowProp));
    commands.spawn((PointLight { color: Color::srgb(0.5, 1.0, 0.75), intensity: 3_000_000.0, range: 70.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(c.x, 22.0, c.z + hl + 6.0), ShadowProp));
    // ── Twin entrance towers + the central crossing spire ──
    for s in [-1.0f32, 1.0] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(7.0, 50.0, 7.0))), MeshMaterial3d(stone.clone()),
            Transform::from_xyz(c.x + s * 18.0, 25.0, c.z + hl), ShadowProp));
        commands.spawn((Mesh3d(meshes.add(Cone { radius: 5.0, height: 16.0 }.mesh().resolution(4))), MeshMaterial3d(trim.clone()),
            Transform::from_xyz(c.x + s * 18.0, 58.0, c.z + hl), ShadowProp));
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.7))), MeshMaterial3d(glass.clone()),
            Transform::from_xyz(c.x + s * 18.0, 67.0, c.z + hl), ShadowProp));
    }
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(10.0, 60.0, 10.0))), MeshMaterial3d(stone.clone()),
        Transform::from_xyz(c.x, 30.0, c.z), ShadowProp));
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 7.0, height: 22.0 }.mesh().resolution(4))), MeshMaterial3d(trim.clone()),
        Transform::from_xyz(c.x, 71.0, c.z), ShadowProp));
    // vaulted roof slabs
    for s in [-1.0f32, 1.0] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.6, 22.0, hl * 2.0))), MeshMaterial3d(trim.clone()),
            Transform::from_xyz(c.x + s * 7.5, 30.0, c.z).with_rotation(Quat::from_rotation_z(s * 0.7)), ShadowProp));
    }
    // warm interior lights down the nave so the hall reads clearly
    for i in 0..5u32 {
        let pz = c.z - hl + 12.0 + i as f32 * 18.0;
        commands.spawn((PointLight { color: Color::srgb(0.6, 1.0, 0.8), intensity: 3_500_000.0, range: 60.0, shadows_enabled: false, ..default() },
            Transform::from_xyz(c.x, 16.0, pz), ShadowProp));
    }
}

// The Blade of the Ruined King: a teal spectral greatsword with an ornate
// cross-shaped guard (authentic to the ref). `scale` sizes it for world vs hand.
fn build_ruined_blade(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>, parent: Entity, scale: f32) {
    let edge = materials.add(StandardMaterial { base_color: Color::srgb(0.6, 1.0, 0.85), emissive: LinearRgba::new(1.5, 7.0, 5.0, 1.0), unlit: true, ..default() });
    let core = materials.add(StandardMaterial { base_color: Color::srgb(0.2, 0.7, 0.55), emissive: LinearRgba::new(0.4, 3.0, 2.0, 1.0), unlit: true, ..default() });
    let s = scale;
    // long tapering blade (point up) + bright central fuller
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.16 * s, 2.6 * s, 0.04 * s))), MeshMaterial3d(edge.clone()),
        Transform::from_xyz(0.0, 0.9 * s, 0.0))).set_parent(parent);
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.05 * s, 2.4 * s, 0.06 * s))), MeshMaterial3d(core.clone()),
        Transform::from_xyz(0.0, 0.9 * s, 0.0))).set_parent(parent);
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.14 * s, height: 0.5 * s }.mesh().resolution(4))), MeshMaterial3d(edge.clone()),
        Transform::from_xyz(0.0, 2.3 * s, 0.0))).set_parent(parent);
    // ornate cross guard: a winged/anchor crossbar with up-curled tips
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(1.1 * s, 0.16 * s, 0.14 * s))), MeshMaterial3d(core.clone()),
        Transform::from_xyz(0.0, -0.45 * s, 0.0))).set_parent(parent);
    for d in [-1.0f32, 1.0] {
        commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.13 * s, height: 0.5 * s }.mesh().resolution(4))), MeshMaterial3d(edge.clone()),
            Transform::from_xyz(d * 0.55 * s, -0.34 * s, 0.0).with_rotation(Quat::from_rotation_z(d * 0.9)))).set_parent(parent);
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.1 * s))), MeshMaterial3d(edge.clone()),
            Transform::from_xyz(d * 0.7 * s, -0.18 * s, 0.0))).set_parent(parent);
    }
    // a downward fin under the guard + grip + flared pommel
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.13 * s, height: 0.5 * s }.mesh().resolution(4))), MeshMaterial3d(edge.clone()),
        Transform::from_xyz(0.0, -0.7 * s, 0.0).with_rotation(Quat::from_rotation_x(std::f32::consts::PI)))).set_parent(parent);
    commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.06 * s, half_height: 0.34 * s })), MeshMaterial3d(core.clone()),
        Transform::from_xyz(0.0, -1.0 * s, 0.0))).set_parent(parent);
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.11 * s))), MeshMaterial3d(edge.clone()),
        Transform::from_xyz(0.0, -1.4 * s, 0.0))).set_parent(parent);
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
        .insert_resource(ClearColor(Color::srgb(0.018, 0.025, 0.06)))
        .insert_resource(PlayerHealth { hp: 100.0, max_hp: 100.0, golden: 0.0, golden_timer: 0.0, hurt_timer: 0.0, iframes: 0.0, blocking: false })
        .insert_resource(Stamina { current: 100.0, max: 100.0 })
        .insert_resource(Mana { current: 100.0, max: 100.0 })
        .insert_resource(Drinking { timer: 0.0 })
        .insert_resource(GunRecoil { climb: 0.0 })
        .insert_resource(MoveSlow { timer: 0.0 })
        .insert_resource(Inventory { selected: ItemKind::Sword, health_potions: 0, mana_potions: 0, has_glock: false, has_rocket: false, has_bow: false, has_ruined_blade: false })
        .insert_resource(Artifacts { selected: ArtifactKind::Lightning, has_flame: false, has_paladin: false, has_trident: false, has_telekinesis: false, has_blackhole: false, trident_armed: 0.0, trident_in_hand: true, throw_cooldown: 0.0, tk_cooldown: 0.0 })
        .insert_resource(Spawner { timer: 5.0 })
        .insert_resource(Realm { in_sky: false, spawned: false, start: Vec3::new(0.0, 431.0, 0.0) })
        .insert_resource(BowState { draw: 0.0, drawing: false })
        .insert_resource(SwordGlow { timer: 0.0 })
        .insert_resource(Ending { stage: 0, timer: 0.0 })
        .insert_resource(KillStats::default())
        .insert_resource(SuccubusSpawn { timer: 25.0 })
        .insert_resource(SauronFight { active: false, spawned: false, defeated: false, engaged: false, origin: Vec3::new(4000.0, 0.0, 0.0) })
        .insert_resource(MysticTalk { stage: 0, timer: 0.0, line: 0 })
        .insert_resource(Warp { stage: 0, timer: 0.0, dest: 0 })
        .insert_resource(HutTalk { line: 0, active: false })
        .insert_resource(Areas { hut_built: false, shadow_built: false, in_shadow: false, in_hut: false, bolt_timer: 3.0 })
        .insert_resource(BladeRiddle { active: false })
        .insert_resource(Petrify { timer: 0.0 })
        .insert_resource(DragonArrival { counting: false, countdown: 0.0, spawn_now: false, spawned: false, pos: Vec3::ZERO, target: Vec3::ZERO })
        .insert_resource(SkyTimer { shoot: 4.0 })
        .insert_resource(HealFx { timer: 0.0, spawn_t: 0.0, color: Color::srgb(0.35, 1.0, 0.5) })
        .init_state::<AppState>()
        .add_systems(Startup, (setup, setup_flash_mats, spawn_castle, spawn_skeletons, setup_hud, spawn_medusa,
                               spawn_spire, spawn_mountains, spawn_enemies, spawn_items, spawn_props,
                               spawn_distant_structures, spawn_mountain_keep, spawn_artifacts, spawn_portal,
                               spawn_hills, spawn_vegetation, spawn_sauron_portal, spawn_npcs))
        // Always-on systems
        .add_systems(Update, (animate_sky, shooting_stars, heal_ui, animate_heal_crests, sword_glow_anim,
                               animate_critters, update_blade_skin))
        .add_systems(Update, (update_health_bar, update_vignette, animate_lightning, animate_orb,
                               animate_eye, check_death, update_bars, update_hotbar, drink_anim,
                               tag_body_parts, flash_skeletons, flash_enemies, flash_dragon,
                               update_boss_bar, soapstone_msg, flash_medusa, update_medusa_bar))
        // Gameplay systems — only while alive
        .add_systems(Update, (player_movement, resolve_collisions, camera_look, head_bob, cursor_grab,
                               sword_swing, lightning_bolts,
                               skeleton_ai, skeleton_attack_anim, skeleton_walk_anim, lightning_damage,
                               dragon_ai, move_fireballs,
                               eye_beam, enemy_ai, medusa_ai)
                               .chain()
                               .run_if(in_state(AppState::Playing)))
        .add_systems(Update, (witch_cast, move_magic_missiles, regen_resources,
                               inventory_scroll, update_held, use_item, glock_fire,
                               move_rockets, pickup_system, enemy_limb_anim, orc_combat, move_debris,
                               tick_transient, animate_mushroom, gun_recoil_anim,
                               cycle_artifact, update_artifact_visual, bow_fire, move_arrows, bow_draw_anim)
                               .run_if(in_state(AppState::Playing)))
        .add_systems(Update, (dragon_breath, dragon_wing_flap, update_shockwaves,
                               bonfire_rest, animate_props, update_fire_patches,
                               update_sound_waves, spin_pickups, update_launched, bonfire_kindle,
                               animate_void_portal, update_expand, void_portal_enter, sky_fall_respawn,
                               princess_idle, princess_interact, ending_sequence,
                               move_medusa_bolts, update_stone_waves, petrify_system)
                               .run_if(in_state(AppState::Playing)))
        // Artifact abilities (flame / paladin / trident) + weather + boss arrival
        .add_systems(Update, (flame_breath, paladin_cast, update_golden_aura,
                               trident_cast, move_trident, update_geyser,
                               decay_artifact_timers, weather_system, artifact_pickup_system,
                               enemy_respawn, animate_flame, telekinesis_cast,
                               dragon_arrival_timer, update_meteor_shock, spawn_dragon)
                               .run_if(in_state(AppState::Playing)))
        // Shadow succubus + Sauron arena boss
        .add_systems(Update, (succubus_spawn, succubus_ai, animate_succubus_wings,
                               sauron_portal_anim, sauron_enter, sauron_ai,
                               sauron_hp_bar, sauron_victory, return_portal_enter)
                               .run_if(in_state(AppState::Playing)))
        // The Mystic's offer + the hidden realms + the Ruined Blade
        .add_systems(Update, (npc_idle, npc_proximity,
                               mystic_interact, mystic_talk, hut_witch_talk, blade_riddle, mist_zombie_ai)
                               .run_if(in_state(AppState::Playing)))
        .add_systems(Update, (warp_system, area_title_tick, shadow_weather, shadow_lightning,
                               blackhole_cast, blackhole_proj_update, blackhole_core_update,
                               blackhole_fx, blackhole_blast_update)
                               .run_if(in_state(AppState::Playing)))
        // Death screen
        .add_systems(OnEnter(AppState::Dead), spawn_death_screen)
        .add_systems(OnExit(AppState::Dead),
                     (despawn_death_screen, reset_game, spawn_skeletons, spawn_medusa, spawn_enemies, spawn_items).chain())
        .add_systems(Update, death_button.run_if(in_state(AppState::Dead)))
        // Pause (Esc toggles)
        .add_systems(Update, toggle_pause)
        .add_systems(OnEnter(AppState::Paused), spawn_pause_screen)
        .add_systems(OnExit(AppState::Paused), despawn_pause_screen)
        .run();
}

fn make_grass_texture(images: &mut Assets<Image>) -> Handle<Image> {
    // High-res, smoothly-shaded grass: layered value-noise between green shades,
    // with faint vertical blade streaks. Linear filtering keeps it un-pixelated.
    let size = 256u32;
    let hash = |x: i32, y: i32| -> f32 {
        let mut n = (x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263))) as u32;
        n = (n ^ (n >> 13)).wrapping_mul(1274126177);
        ((n ^ (n >> 16)) as f32) / (u32::MAX as f32)
    };
    // Smooth value noise (bilinear over an integer lattice) so there are no harsh pixels.
    let vnoise = |fx: f32, fy: f32, cell: f32| -> f32 {
        let gx = fx / cell; let gy = fy / cell;
        let x0 = gx.floor() as i32; let y0 = gy.floor() as i32;
        let tx = gx - gx.floor(); let ty = gy - gy.floor();
        let sx = tx * tx * (3.0 - 2.0 * tx);
        let sy = ty * ty * (3.0 - 2.0 * ty);
        let n00 = hash(x0, y0); let n10 = hash(x0 + 1, y0);
        let n01 = hash(x0, y0 + 1); let n11 = hash(x0 + 1, y0 + 1);
        let a = n00 + (n10 - n00) * sx;
        let b = n01 + (n11 - n01) * sx;
        a + (b - a) * sy
    };
    // Toroidal sampling so the texture tiles seamlessly: blend wrapped noise.
    let dark   = [14.0f32, 46.0, 16.0];
    let base   = [30.0f32, 82.0, 28.0];
    let bright = [64.0f32, 128.0, 46.0];
    let mut data: Vec<u8> = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32; let fy = y as f32;
            // multi-octave smooth noise (patchiness) + fine high-frequency grain
            let n = vnoise(fx, fy, 48.0) * 0.45
                  + vnoise(fx, fy, 18.0) * 0.30
                  + vnoise(fx, fy, 7.0) * 0.17
                  + hash(x as i32, y as i32) * 0.08;
            // fine vertical blade striations at a few frequencies
            let blade = ((fx * 1.7).sin() * 0.5 + 0.5) * 0.16
                      + ((fx * 4.3 + fy * 0.3).sin() * 0.5 + 0.5) * 0.10
                      + ((fx * 9.1 + (fy * 0.5).sin() * 3.0).sin() * 0.5 + 0.5) * 0.06;
            let mut t = (n * 0.74 + blade).clamp(0.0, 1.0);
            // occasional bright highlight tips on blades
            if hash(x as i32 / 2, y as i32 / 2) > 0.93 { t = (t + 0.25).min(1.0); }
            let lerp = |a: f32, b: f32, u: f32| a + (b - a) * u;
            let (mut r, mut g, mut b) = if t < 0.5 {
                let u = t * 2.0;
                (lerp(dark[0], base[0], u), lerp(dark[1], base[1], u), lerp(dark[2], base[2], u))
            } else {
                let u = (t - 0.5) * 2.0;
                (lerp(base[0], bright[0], u), lerp(base[1], bright[1], u), lerp(base[2], bright[2], u))
            };
            // sparse dry-earth speckles for realism
            if hash(x as i32 + 7, y as i32 * 3 + 1) > 0.985 {
                r = 58.0; g = 46.0; b = 28.0;
            }
            data.extend_from_slice(&[r as u8, g as u8, b as u8, 255]);
        }
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
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
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

    let count = 24u32;
    for i in 0..count {
        let a = (i as f32 / count as f32) * std::f32::consts::TAU + hash(i as f32) * 0.15;
        let dist = 1080.0 + hash(i as f32 * 2.3) * 90.0 - 45.0;
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
    for k in 0..40u32 {
        let a = hash(k as f32 * 1.7) * std::f32::consts::TAU;
        let dist = 820.0 + hash(k as f32 * 4.2) * 200.0;
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

// Ruined castle silhouettes brooding on the far plain — pure backdrop that gives
// the world scale and the feeling of a fallen kingdom beyond the fog.
fn spawn_distant_structures(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.20, 0.25), perceptual_roughness: 0.97, ..default()
    });
    let roof = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.13, 0.18), perceptual_roughness: 0.95, ..default()
    });
    let tower_mesh = meshes.add(Cuboid::new(14.0, 1.0, 14.0));
    let roof_mesh  = meshes.add(Cone { radius: 11.0, height: 16.0 }.mesh().resolution(6));

    // (x, z, scale) — scattered far out, clear of spire (320,240) & mountain keep
    let sites = [
        (560.0, -560.0, 1.3), (-620.0, 480.0, 1.6), (640.0, 520.0, 1.1),
        (-700.0, -260.0, 1.4), (180.0, 700.0, 1.0), (-520.0, -640.0, 1.2),
    ];
    for (cx, cz, s) in sites {
        let wh = 34.0 * s; // wall/tower height
        let half = 26.0 * s;
        // Four corner towers with conical roofs
        for (dx, dz) in [(-half, -half), (half, -half), (-half, half), (half, half)] {
            commands.spawn((
                Mesh3d(tower_mesh.clone()), MeshMaterial3d(stone.clone()),
                Transform::from_xyz(cx + dx, wh * 0.5, cz + dz).with_scale(Vec3::new(s, wh, s)),
                Collider { half: Vec2::new(7.0 * s, 7.0 * s) },
            ));
            commands.spawn((
                Mesh3d(roof_mesh.clone()), MeshMaterial3d(roof.clone()),
                Transform::from_xyz(cx + dx, wh + 8.0 * s, cz + dz).with_scale(Vec3::splat(s)),
            ));
        }
        // Connecting curtain walls (north & south) + a taller central keep
        for dz in [-half, half] {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(half * 2.0, wh * 0.6, 4.0 * s))),
                MeshMaterial3d(stone.clone()),
                Transform::from_xyz(cx, wh * 0.3, cz + dz),
                Collider { half: Vec2::new(half, 2.0 * s) },
            ));
        }
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(18.0 * s, wh * 1.5, 18.0 * s))),
            MeshMaterial3d(stone.clone()),
            Transform::from_xyz(cx, wh * 0.75, cz),
            Collider { half: Vec2::new(9.0 * s, 9.0 * s) },
        ));
        commands.spawn((
            Mesh3d(roof_mesh.clone()), MeshMaterial3d(roof.clone()),
            Transform::from_xyz(cx, wh * 1.5 + 11.0 * s, cz).with_scale(Vec3::splat(s * 1.3)),
        ));

        // East & west curtain walls — fully enclose the silhouette
        for dx in [-half, half] {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(4.0 * s, wh * 0.6, half * 2.0))), MeshMaterial3d(stone.clone()),
                Transform::from_xyz(cx + dx, wh * 0.3, cz), Collider { half: Vec2::new(2.0 * s, half) },
            ));
        }
        // Battlement merlons ringing the wall tops (broken silhouette)
        let merlon = meshes.add(Cuboid::new(2.4 * s, 3.0 * s, 2.4 * s));
        let mut m = -half + 3.0 * s;
        while m < half {
            for (mx, mz) in [(m, -half), (m, half), (-half, m), (half, m)] {
                commands.spawn((Mesh3d(merlon.clone()), MeshMaterial3d(stone.clone()),
                    Transform::from_xyz(cx + mx, wh * 0.6 + 1.5 * s, cz + mz)));
            }
            m += 7.0 * s;
        }
        // Keep spire crowning the central tower + a warm landmark glow seen through fog
        commands.spawn((Mesh3d(meshes.add(Cone { radius: 2.2 * s, height: 20.0 * s }.mesh().resolution(6))), MeshMaterial3d(roof.clone()),
            Transform::from_xyz(cx, wh * 1.5 + 26.0 * s, cz)));
        commands.spawn((PointLight { color: Color::srgb(1.0, 0.5, 0.18), intensity: 3_000_000.0, range: 130.0, shadows_enabled: false, ..default() },
            Transform::from_xyz(cx, wh * 1.2, cz)));
        // A collapsed, leaning outer tower — evidence of a fallen hold
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(8.0 * s, wh * 0.9, 8.0 * s))), MeshMaterial3d(stone.clone()),
            Transform::from_xyz(cx + half * 0.7, wh * 0.4, cz - half * 1.25).with_rotation(Quat::from_rotation_x(0.22))));
    }
}

// A colossal mountain with a switchback stairway you can actually climb to a
// wind-bitten summit fortress crowned by a glowing beacon. The verticality piece.
fn spawn_mountain_keep(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let rock_tex = make_rock_texture(&mut images);
    let rock = materials.add(StandardMaterial {
        base_color_texture: Some(rock_tex),
        uv_transform: bevy::math::Affine2::from_scale(Vec2::new(10.0, 10.0)),
        perceptual_roughness: 1.0, ..default()
    });
    let snow = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.94, 1.0), perceptual_roughness: 0.7, ..default()
    });
    let stair = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.29, 0.34), perceptual_roughness: 0.95, ..default()
    });
    let keep_stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.21, 0.27), perceptual_roughness: 0.95, ..default()
    });
    let beacon_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.85, 1.0),
        emissive: LinearRgba::new(2.0, 5.0, 12.0, 1.0), unlit: true, ..default()
    });
    let torch_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.6, 0.1),
        emissive: LinearRgba::new(6.0, 2.6, 0.2, 1.0), unlit: true, ..default()
    });

    let base = Vec3::new(-360.0, 0.0, -380.0);

    // The mountain itself — far taller than the surrounding range
    let mh = 320.0; let mr = 155.0;
    commands.spawn((Mesh3d(meshes.add(Cone { radius: mr, height: mh })), MeshMaterial3d(rock.clone()),
        Transform::from_xyz(base.x, mh * 0.5, base.z)));
    let cap = mh * 0.32;
    commands.spawn((Mesh3d(meshes.add(Cone { radius: mr * 0.4, height: cap })), MeshMaterial3d(snow.clone()),
        Transform::from_xyz(base.x, mh - cap * 0.5, base.z)));

    // Switchback stairway climbing the east face
    let step_mesh = meshes.add(Cuboid::new(7.0, 1.0, 1.4));
    let landing_mesh = meshes.add(Cuboid::new(9.0, 1.0, 9.0));
    let post_mesh = meshes.add(Cuboid::new(0.5, 3.0, 0.5));
    let rise = 0.72f32; let run = 1.35f32;
    let flights = 9; let steps_per = 27;
    let mut x = base.x + 46.0;
    let mut z = base.z;
    let mut y = 0.0f32;
    let mut zdir = 1.0f32;
    let (mut kx, mut kz, mut ky) = (x, z, 0.0f32);
    for _f in 0..flights {
        for _ in 0..steps_per {
            y += rise; z += zdir * run;
            commands.spawn((Mesh3d(step_mesh.clone()), MeshMaterial3d(stair.clone()),
                Transform::from_xyz(x, y - 0.5, z),
                Walkable { half: Vec2::new(3.5, 0.72), top: y }));
        }
        // Landing platform + a torch post to light the climb
        commands.spawn((Mesh3d(landing_mesh.clone()), MeshMaterial3d(stair.clone()),
            Transform::from_xyz(x, y - 0.5, z),
            Walkable { half: Vec2::new(4.5, 4.5), top: y }));
        commands.spawn((Mesh3d(post_mesh.clone()), MeshMaterial3d(keep_stone.clone()),
            Transform::from_xyz(x + 3.5, y + 1.5, z)));
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.55))), MeshMaterial3d(torch_mat.clone()),
            Transform::from_xyz(x + 3.5, y + 3.2, z)));
        commands.spawn((PointLight { color: Color::srgb(1.0, 0.6, 0.2), intensity: 420_000.0,
            range: 34.0, shadows_enabled: false, ..default() },
            Transform::from_xyz(x + 3.5, y + 3.4, z)));
        kx = x; kz = z; ky = y;
        zdir = -zdir;
        x -= 8.0;
    }

    // ── Summit fortress ──────────────────────────────────────
    let px = kx - 16.0; let pz = kz;
    // Flat keep platform you step onto from the last landing
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(36.0, 2.0, 36.0))), MeshMaterial3d(keep_stone.clone()),
        Transform::from_xyz(px, ky - 1.0, pz),
        Walkable { half: Vec2::new(18.0, 18.0), top: ky }));
    // Four corner battlement towers
    for (dx, dz) in [(-15.0, -15.0), (15.0, -15.0), (-15.0, 15.0), (15.0, 15.0)] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(6.0, 18.0, 6.0))), MeshMaterial3d(keep_stone.clone()),
            Transform::from_xyz(px + dx, ky + 9.0, pz + dz),
            Collider { half: Vec2::new(3.0, 3.0) }));
        commands.spawn((Mesh3d(meshes.add(Cone { radius: 4.5, height: 6.0 }.mesh().resolution(6))),
            MeshMaterial3d(beacon_mat.clone()),
            Transform::from_xyz(px + dx, ky + 21.0, pz + dz)));
    }
    // Central spire crowned with a glowing beacon (a far-off landmark)
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(9.0, 34.0, 9.0))), MeshMaterial3d(keep_stone.clone()),
        Transform::from_xyz(px, ky + 17.0, pz),
        Collider { half: Vec2::new(4.5, 4.5) }));
    commands.spawn((Mesh3d(meshes.add(Sphere::new(2.4))), MeshMaterial3d(beacon_mat.clone()),
        Transform::from_xyz(px, ky + 37.0, pz)));
    commands.spawn((PointLight { color: Color::srgb(0.6, 0.85, 1.0), intensity: 4_000_000.0,
        range: 220.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(px, ky + 40.0, pz)));
}

// Lush natural detail strewn across the field: bushes, boulders, grass tufts,
// wildflowers, fallen logs, mushroom rings and a couple of moonlit ponds.
fn spawn_vegetation(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let hash = |n: f32| (n.sin() * 43758.547).fract().abs();
    // Skip the castle footprint, spawn portal and the spawn bonfire.
    let clear = |x: f32, z: f32| -> bool {
        if x.abs() < 66.0 && z < -25.0 && z > -160.0 { return false; }     // castle
        if Vec2::new(x, z).distance(Vec2::new(0.0, 18.0)) < 10.0 { return false; } // portal
        if Vec2::new(x, z).distance(Vec2::new(7.0, 9.0)) < 6.0 { return false; }   // bonfire
        true
    };
    let golden = |i: u32, range: f32| -> (f32, f32) {
        let a = i as f32 * 2.399963;
        let d = (i as f32 / 1.0).sqrt() * 0.0 + (hash(i as f32 * 1.3)).sqrt() * range;
        (a.cos() * d, a.sin() * d)
    };

    let leaf_dark = materials.add(StandardMaterial { base_color: Color::srgb(0.10, 0.30, 0.12), perceptual_roughness: 1.0, ..default() });
    let leaf_lt   = materials.add(StandardMaterial { base_color: Color::srgb(0.18, 0.42, 0.16), perceptual_roughness: 1.0, ..default() });
    let rock_tex  = make_rock_texture(&mut images);
    let rock_mat  = materials.add(StandardMaterial { base_color: Color::srgb(0.46, 0.47, 0.52), base_color_texture: Some(rock_tex),
        uv_transform: bevy::math::Affine2::from_scale(Vec2::new(2.0, 2.0)), perceptual_roughness: 1.0, ..default() });
    let bark      = materials.add(StandardMaterial { base_color: Color::srgb(0.32, 0.21, 0.11), perceptual_roughness: 1.0, ..default() });
    let stem      = materials.add(StandardMaterial { base_color: Color::srgb(0.20, 0.38, 0.14), perceptual_roughness: 1.0, ..default() });
    let mush_cap  = materials.add(StandardMaterial { base_color: Color::srgb(0.7, 0.12, 0.12), perceptual_roughness: 0.8, ..default() });
    let mush_stem = materials.add(StandardMaterial { base_color: Color::srgb(0.85, 0.82, 0.72), perceptual_roughness: 0.9, ..default() });
    let water_mat = materials.add(StandardMaterial { base_color: Color::srgba(0.10, 0.22, 0.34, 0.7),
        emissive: LinearRgba::new(0.02, 0.06, 0.12, 1.0), metallic: 0.7, perceptual_roughness: 0.12,
        alpha_mode: AlphaMode::Blend, ..default() });
    // Wildflower glow materials (soft emissive so they catch the eye at night)
    let flowers: Vec<Handle<StandardMaterial>> = [
        (1.0f32, 0.85, 0.3), (0.9, 0.4, 0.9), (0.5, 0.7, 1.0), (1.0, 0.5, 0.5),
    ].iter().map(|&(r, g, b)| materials.add(StandardMaterial {
        base_color: Color::srgb(r, g, b), emissive: LinearRgba::new(r * 0.8, g * 0.8, b * 0.8, 1.0), ..default()
    })).collect();

    // ── Bushes: clusters of overlapping flattened leaf-spheres ──
    let bush_lo = meshes.add(Sphere::new(1.0));
    for i in 0..120u32 {
        let (x, z) = golden(i * 3 + 1, 500.0);
        if !clear(x, z) || (x * x + z * z) < 200.0 { continue; }
        let base = Vec3::new(x, 0.0, z);
        let s = 0.8 + hash(i as f32 * 5.1) * 1.0;
        for k in 0..3u32 {
            let o = Vec3::new((hash((i * 7 + k) as f32) - 0.5) * 1.4, 0.0, (hash((i * 9 + k) as f32) - 0.5) * 1.4);
            let m = if k % 2 == 0 { leaf_dark.clone() } else { leaf_lt.clone() };
            commands.spawn((Mesh3d(bush_lo.clone()), MeshMaterial3d(m),
                Transform::from_translation(base + o + Vec3::Y * (0.6 * s))
                    .with_scale(Vec3::new(s, s * 0.8, s))));
        }
    }

    // ── Boulders & rocks ──
    let rock_mesh = meshes.add(Sphere::new(1.0));
    for i in 0..70u32 {
        let (x, z) = golden(i * 5 + 2, 520.0);
        if !clear(x, z) || (x * x + z * z) < 120.0 { continue; }
        let s = 0.8 + hash(i as f32 * 2.7) * 2.6;
        commands.spawn((Mesh3d(rock_mesh.clone()), MeshMaterial3d(rock_mat.clone()),
            Transform::from_xyz(x, s * 0.45, z)
                .with_rotation(Quat::from_euler(EulerRot::XYZ, hash(i as f32) * 1.0, hash(i as f32 * 1.7) * 6.28, hash(i as f32 * 2.3) * 1.0))
                .with_scale(Vec3::new(s, s * 0.7, s * 0.9))));
    }

    // ── Grass tufts: little fans of green blades ──
    let blade = meshes.add(Cone { radius: 0.05, height: 0.55 }.mesh().resolution(4));
    for i in 0..260u32 {
        let (x, z) = golden(i * 2 + 3, 480.0);
        if !clear(x, z) || (x * x + z * z) < 60.0 { continue; }
        for k in 0..4u32 {
            let o = Vec3::new((hash((i * 3 + k) as f32) - 0.5) * 0.5, 0.0, (hash((i * 5 + k) as f32) - 0.5) * 0.5);
            let tilt = (hash((i + k) as f32) - 0.5) * 0.4;
            commands.spawn((Mesh3d(blade.clone()), MeshMaterial3d(stem.clone()),
                Transform::from_translation(Vec3::new(x, 0.28, z) + o)
                    .with_rotation(Quat::from_rotation_z(tilt) * Quat::from_rotation_x(tilt))));
        }
    }

    // ── Wildflowers: a stem + a glowing bloom ──
    let f_stem = meshes.add(Cylinder { radius: 0.02, half_height: 0.25 });
    let bloom = meshes.add(Sphere::new(0.12));
    for i in 0..90u32 {
        let (x, z) = golden(i * 4 + 5, 470.0);
        if !clear(x, z) || (x * x + z * z) < 40.0 { continue; }
        commands.spawn((Mesh3d(f_stem.clone()), MeshMaterial3d(stem.clone()), Transform::from_xyz(x, 0.25, z)));
        commands.spawn((Mesh3d(bloom.clone()), MeshMaterial3d(flowers[(i as usize) % flowers.len()].clone()),
            Transform::from_xyz(x, 0.52, z)));
    }

    // ── Fallen logs ──
    let log = meshes.add(Cylinder { radius: 0.4, half_height: 2.2 });
    for i in 0..22u32 {
        let (x, z) = golden(i * 11 + 7, 500.0);
        if !clear(x, z) || (x * x + z * z) < 90.0 { continue; }
        commands.spawn((Mesh3d(log.clone()), MeshMaterial3d(bark.clone()),
            Transform::from_xyz(x, 0.4, z)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2) * Quat::from_rotation_x(hash(i as f32) * 6.28))));
    }

    // ── Mushroom rings ──
    let cap = meshes.add(Sphere::new(0.18));
    let mstem = meshes.add(Cylinder { radius: 0.05, half_height: 0.12 });
    for i in 0..34u32 {
        let (x, z) = golden(i * 13 + 9, 460.0);
        if !clear(x, z) || (x * x + z * z) < 50.0 { continue; }
        let n = 3 + (hash(i as f32) * 3.0) as u32;
        for k in 0..n {
            let a = k as f32 / n as f32 * std::f32::consts::TAU;
            let mx = x + a.cos() * 0.6; let mz = z + a.sin() * 0.6;
            commands.spawn((Mesh3d(mstem.clone()), MeshMaterial3d(mush_stem.clone()), Transform::from_xyz(mx, 0.12, mz)));
            commands.spawn((Mesh3d(cap.clone()), MeshMaterial3d(mush_cap.clone()),
                Transform::from_xyz(mx, 0.26, mz).with_scale(Vec3::new(1.0, 0.7, 1.0))));
        }
    }

    // ── Moonlit ponds with reed fringes ──
    for (px, pz, pr) in [(-130.0f32, 120.0f32, 16.0f32), (170.0, -40.0, 12.0)] {
        commands.spawn((Mesh3d(meshes.add(Cylinder { radius: pr, half_height: 0.05 })), MeshMaterial3d(water_mat.clone()),
            Transform::from_xyz(px, 0.08, pz)));
        for k in 0..24u32 {
            let a = k as f32 / 24.0 * std::f32::consts::TAU;
            let rr = pr + 0.5 + hash(k as f32) * 1.5;
            commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.06, height: 1.3 }.mesh().resolution(4))), MeshMaterial3d(stem.clone()),
                Transform::from_xyz(px + a.cos() * rr, 0.65, pz + a.sin() * rr)
                    .with_rotation(Quat::from_rotation_z((hash(k as f32) - 0.5) * 0.3))));
        }
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

    // ── "The Last Stand" — a battlefield memorial that tells a story ──
    // A ring of fallen knights' swords planted blade-down around a great
    // broken greatsword crowned with a helm: where the kingdom's defenders died.
    let ls = Vec3::new(72.0, 0.0, 48.0);
    // Central monument: a huge broken blade thrust into the earth + crossguard + helm
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.5, 5.0, 0.14))), MeshMaterial3d(steel.clone()),
        Transform::from_xyz(ls.x, 2.4, ls.z).with_rotation(Quat::from_rotation_z(0.08))));
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(1.6, 0.3, 0.3))), MeshMaterial3d(steel.clone()),
        Transform::from_xyz(ls.x, 4.4, ls.z)));
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.45))), MeshMaterial3d(steel.clone()),
        Transform::from_xyz(ls.x, 4.9, ls.z).with_scale(Vec3::new(1.0, 1.1, 1.0))));
    // A solitary mourning flame so the memorial draws the eye in the dark
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.4))), MeshMaterial3d(flame_m.clone()),
        Transform::from_xyz(ls.x + 1.8, 0.6, ls.z)));
    commands.spawn((PointLight { color: Color::srgb(1.0, 0.55, 0.2), intensity: 200_000.0, range: 22.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(ls.x + 1.8, 1.2, ls.z)));
    // Ring of planted swords + scattered bones of the fallen
    for s in 0..10u32 {
        let a = s as f32 / 10.0 * std::f32::consts::TAU;
        let rr = 3.4 + hash(s as f32 * 7.3) * 1.6;
        let sx = ls.x + a.cos() * rr;
        let sz = ls.z + a.sin() * rr;
        let tilt = (hash(s as f32) - 0.5) * 0.5;
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.16, 2.2, 0.05))), MeshMaterial3d(steel.clone()),
            Transform::from_xyz(sx, 1.0, sz).with_rotation(Quat::from_rotation_y(a) * Quat::from_rotation_x(tilt))));
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.5, 0.12, 0.12))), MeshMaterial3d(steel.clone()),
            Transform::from_xyz(sx, 2.0, sz).with_rotation(Quat::from_rotation_y(a))));
        // a bone or two at the base
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.08, 0.5, 0.08))), MeshMaterial3d(bone.clone()),
            Transform::from_xyz(sx + 0.4, 0.12, sz - 0.3).with_rotation(Quat::from_rotation_z(1.4))));
    }
    // A soapstone at the memorial's foot to deliver the lore line
    commands.spawn((Mesh3d(meshes.add(Annulus::new(0.4, 0.6))), MeshMaterial3d(rune_m.clone()),
        Transform::from_xyz(ls.x, 0.06, ls.z + 2.2).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        Soapstone { idx: 4 })); // "here lies the last shieldbearer"
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
    mut heal: ResMut<HealFx>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !key.just_pressed(KeyCode::KeyR) { return; }
    let pp = player_q.single().translation;
    for bf in &bonfire_q {
        if bf.translation().distance(pp) < 5.0 {
            health.hp = health.max_hp;
            health.hurt_timer = 0.0;
            mana.current = mana.max;
            stamina.current = stamina.max;
            inv.health_potions = inv.health_potions.max(3);
            inv.mana_potions = inv.mana_potions.max(3);
            heal.timer = 1.1; heal.color = Color::srgb(0.35, 1.0, 0.5); // green heal UI
            // Kindle: a 30s gold glow + showering sparks while the fire roars up
            let spark_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.75, 0.3),
                emissive: LinearRgba::new(9.0, 4.0, 0.6, 1.0), unlit: true, ..default() });
            // Tiny needle-shaped sparks (long thin slivers, not balls)
            let spark_mesh = meshes.add(Cuboid::new(0.012, 0.32, 0.012));
            commands.spawn((
                PointLight { color: Color::srgb(1.0, 0.72, 0.28), intensity: 1_000_000.0,
                    range: 48.0, shadows_enabled: false, ..default() },
                Transform::from_translation(bf.translation() + Vec3::Y * 1.5),
                GlobalTransform::default(), Visibility::default(),
                BonfireKindle { timer: 30.0, spark: 0.0, mat: spark_mat, mesh: spark_mesh },
            ));
        }
    }
}

// While a bonfire is kindled: flicker its glow and shower sparks for 30s.
fn bonfire_kindle(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &GlobalTransform, &mut BonfireKindle, &mut PointLight)>,
) {
    let dt = time.delta_secs();
    let et = time.elapsed_secs();
    let h = |n: f32| (n.sin() * 43758.547).fract().abs();
    for (e, g, mut k, mut light) in q.iter_mut() {
        k.timer -= dt;
        let fade = (k.timer / 4.0).clamp(0.0, 1.0);     // ease out over the last 4s
        light.intensity = (850_000.0 + (et * 17.0).sin() * 220_000.0) * fade.max(0.0);
        k.spark -= dt;
        if k.spark <= 0.0 {
            k.spark = 0.04;
            let base = g.translation();
            for j in 0..2u32 {
                let s = et * 57.0 + j as f32 * 11.3;
                let vel = Vec3::new((h(s) - 0.5) * 3.5, 5.5 + h(s + 1.0) * 4.5, (h(s + 2.0) - 0.5) * 3.5);
                commands.spawn((
                    Mesh3d(k.mesh.clone()), MeshMaterial3d(k.mat.clone()),
                    Transform::from_translation(base - Vec3::Y * 1.3 + Vec3::new((h(s+3.0)-0.5)*0.6, 0.0, (h(s+4.0)-0.5)*0.6))
                        .with_rotation(Quat::from_rotation_arc(Vec3::Y, vel.normalize_or_zero())),
                    Debris { vel, life: 0.85 },
                ));
            }
        }
        if k.timer <= 0.0 { commands.entity(e).despawn_recursive(); }
    }
}

// Spawn rising green heal "crests" + beams in the camera UI while a heal is active.
fn heal_ui(
    time: Res<Time>,
    mut fx: ResMut<HealFx>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    if fx.timer <= 0.0 { return; }
    fx.timer -= dt;
    fx.spawn_t -= dt;
    if fx.spawn_t > 0.0 { return; }
    fx.spawn_t = 0.05;
    let et = time.elapsed_secs();
    let h = |n: f32| (n.sin() * 43758.547).fract().abs();
    let col = fx.color;
    // Several elements per tick spread across the WHOLE width: rising beams + crests
    for j in 0..5u32 {
        let s = et * 31.0 + j as f32 * 7.7;
        let x = 2.0 + h(s) * 96.0;                  // full-width: 2%..98%
        let beam = h(s + 1.0) > 0.5;
        let (w, hgt) = if beam { (6.0, 46.0) } else { (16.0, 16.0) };
        let life = 0.9 + h(s + 2.0) * 0.3;
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(x), bottom: Val::Px(60.0),
                width: Val::Px(w), height: Val::Px(hgt), ..default()
            },
            BackgroundColor(col.with_alpha(0.0)),
            BorderRadius::all(Val::Px(if beam { 3.0 } else { 8.0 })),
            HealCrest { life, max: life, bottom0: 60.0, rise: 200.0 + h(s + 3.0) * 160.0, color: col },
        ));
    }
}

// Rise + fade the green heal crests, then despawn them.
fn animate_heal_crests(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Node, &mut BackgroundColor, &mut HealCrest)>,
) {
    let dt = time.delta_secs();
    for (e, mut node, mut bg, mut c) in q.iter_mut() {
        c.life -= dt;
        let frac = (c.life / c.max).clamp(0.0, 1.0);
        let p = 1.0 - frac;
        node.bottom = Val::Px(c.bottom0 + p * c.rise);
        let alpha = (frac * (1.0 - frac) * 4.0).clamp(0.0, 1.0); // bell-curve fade
        bg.0 = c.color.with_alpha(alpha * 0.85);
        if c.life <= 0.0 { commands.entity(e).despawn_recursive(); }
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
        Mesh3d(meshes.add(Plane3d::default().mesh().size(2600.0, 2600.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(grass),
            uv_transform: bevy::math::Affine2::from_scale(Vec2::new(340.0, 340.0)),
            perceptual_roughness: 1.0,
            ..default()
        })),
        Transform::default(),
    ));

    // Moonlight — bright & cold. (The "dark" mood comes from the sky + fog, not dim light.)
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.62, 0.72, 1.0),
            illuminance: 2860.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(-30.0, 50.0, -20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Bright ambient so the world reads clearly (30% above the original)
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.32, 0.37, 0.58),
        brightness: 416.0,
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

    // ════════════════════════════════════════════════════════════════════
    //  Gorgeous night sky: dense tinted star field, a Milky Way band with
    //  nebula haze, shimmering aurora curtains, and (occasional) shooting stars.
    // ════════════════════════════════════════════════════════════════════
    let hsky = |n: f32| (n.sin() * 43758.547).fract().abs();
    let star_mesh = meshes.add(Sphere::new(1.0));
    // Pure, intensely bright white stars (huge emissive so fog can't dim them)
    let star_mats: Vec<Handle<StandardMaterial>> = [
        45.0f32, 60.0, 80.0, 55.0,
    ].iter().map(|&b| materials.add(StandardMaterial {
        base_color: Color::WHITE, emissive: LinearRgba::new(b, b, b, 1.0), unlit: true, ..default()
    })).collect();

    // Scattered background stars over the whole dome
    for i in 0..1000u32 {
        let t = i as f32;
        let phi = hsky(t * 0.7) * std::f32::consts::TAU;
        let el  = (hsky(t * 1.9) * 80.0 + 6.0).to_radians();
        let r   = 780.0;                       // inside the fog so they stay bright
        let x = r * el.cos() * phi.cos();
        let y = r * el.sin();
        let z = r * el.cos() * phi.sin();
        let size = 0.8 + hsky(t * 3.3) * 1.3;
        commands.spawn((
            Mesh3d(star_mesh.clone()),
            MeshMaterial3d(star_mats[(i as usize) % star_mats.len()].clone()),
            Transform::from_xyz(x, y, z).with_scale(Vec3::splat(size)),
            SkyStar { phase: hsky(t * 5.1) * 6.28, base: size },
        ));
    }

    // ── Milky Way band: a dense, tilted ribbon of stars + soft nebula glow ──
    let n_axis = Vec3::new(0.35, 1.0, 0.25).normalize();           // galactic pole
    let u = n_axis.cross(Vec3::Y).normalize();
    let v = n_axis.cross(u).normalize();
    for i in 0..900u32 {
        let t = i as f32;
        let ang = hsky(t * 0.9) * std::f32::consts::TAU;
        // concentrate near the band centre (sum of hashes ≈ gaussian)
        let spread = (hsky(t * 2.1) + hsky(t * 4.7) + hsky(t * 8.3) - 1.5) * 0.16;
        let dir = ((u * ang.cos() + v * ang.sin()) + n_axis * spread).normalize();
        if dir.y < 0.06 { continue; }
        let p = dir * 760.0;
        let size = 0.6 + hsky(t * 6.6) * 1.1;
        let tint = if hsky(t * 7.7) > 0.6 { 1 } else { 3 }; // bluish / cool
        commands.spawn((
            Mesh3d(star_mesh.clone()),
            MeshMaterial3d(star_mats[tint].clone()),
            Transform::from_xyz(p.x, p.y, p.z).with_scale(Vec3::splat(size)),
        ));
    }
    // Soft nebula haze blobs strung along the band
    let neb_mesh = meshes.add(Sphere::new(70.0));
    for (k, tint) in [(0u32, (1.2f32, 0.5, 2.2)), (1, (0.5, 1.0, 2.4)), (2, (2.0, 0.6, 1.8)),
                      (3, (0.6, 1.4, 1.6)), (4, (1.4, 0.6, 2.0))].into_iter().enumerate() {
        let ang = k as f32 / 5.0 * std::f32::consts::TAU + 0.4;
        let dir = ((u * ang.cos() + v * ang.sin()) + n_axis * (hsky(k as f32) - 0.5) * 0.1).normalize();
        if dir.y < 0.05 { continue; }
        let p = dir * 1300.0;
        let neb = materials.add(StandardMaterial {
            base_color: Color::srgba(0.5, 0.5, 0.7, 0.5),
            emissive: LinearRgba::new(tint.1.0, tint.1.1, tint.1.2, 1.0),
            unlit: true, alpha_mode: AlphaMode::Add, ..default() });
        commands.spawn((
            Mesh3d(neb_mesh.clone()), MeshMaterial3d(neb),
            Transform::from_xyz(p.x, p.y, p.z).with_scale(Vec3::new(2.0 + hsky(k as f32) * 1.5, 1.0, 2.0)),
        ));
    }

    // ── Auroras: soft, smooth glowing ribbons (green/purple/blue) high in the sky ──
    // Each band is a wavy chain of big, faint, additive glow-blobs that overlap into
    // a soft curtain (no hard edges). animate_sky gently sways each band.
    let aur_cols: [LinearRgba; 3] = [
        LinearRgba::new(0.2, 1.1, 0.5, 1.0),   // green
        LinearRgba::new(0.7, 0.3, 1.1, 1.0),   // purple
        LinearRgba::new(0.3, 0.6, 1.2, 1.0),   // blue
    ];
    let glow_mesh = meshes.add(Sphere::new(22.0));
    for k in 0..8u32 {
        let t = k as f32;
        let ang = -1.0 + t / 8.0 * 2.0;                  // arc across the north
        let dist = 800.0;
        let cx = ang.sin() * dist;
        let cz = -ang.cos() * dist;
        let root = commands.spawn((
            Transform::from_xyz(cx, 300.0, cz).with_rotation(Quat::from_rotation_y(ang)),
            GlobalTransform::default(), Visibility::default(),
            AuroraBand { phase: t * 0.8, yaw: ang },
        )).id();
        for j in 0..11u32 {
            let col = aur_cols[(k as usize + j as usize) % 3];
            let mat = materials.add(StandardMaterial {
                base_color: Color::srgba(0.4, 0.7, 0.6, 0.14),
                emissive: LinearRgba::new(col.red * 0.5, col.green * 0.5, col.blue * 0.5, 1.0),
                unlit: true, alpha_mode: AlphaMode::Add, ..default() });
            let yy = (j as f32 - 5.0) * 16.0;
            let xx = (j as f32 * 0.7).sin() * 32.0;      // gentle wave
            commands.spawn((
                Mesh3d(glow_mesh.clone()), MeshMaterial3d(mat),
                Transform::from_xyz(xx, yy, 0.0).with_scale(Vec3::new(2.4, 1.1, 0.35)),
                NotShadowCaster,
            )).set_parent(root);
        }
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

    for i in 0..520u32 {
        let t     = i as f32;
        let angle = t * 137.508_f32.to_radians();
        let dist  = (18.0 + t * 1.7_f32).min(920.0);
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
    commands.insert_resource(SwordAssets { blade: blade_mat.clone(), edge: blade_edge.clone() });
    let gold_mat   = materials.add(StandardMaterial { base_color: Color::srgb(0.92, 0.75, 0.12), emissive: LinearRgba::new(0.30, 0.22, 0.02, 1.0), perceptual_roughness: 1.0, ..default() });
    let grip_mat   = materials.add(StandardMaterial { base_color: Color::srgb(0.55, 0.08, 0.08), perceptual_roughness: 1.0, ..default() });
    let gauntlet_m = materials.add(StandardMaterial { base_color: Color::srgb(0.18, 0.16, 0.20), perceptual_roughness: 1.0, ..default() });
    let bolt_mat   = materials.add(StandardMaterial { base_color: Color::srgb(0.5, 0.75, 1.0),   emissive: LinearRgba::new(3.0, 6.0, 12.0, 1.0), unlit: true, ..default() });
    let bolt2_mat  = materials.add(StandardMaterial { base_color: Color::WHITE, emissive: LinearRgba::new(5.0, 8.0, 16.0, 1.0), unlit: true, ..default() });

    // ── Player + camera ───────────────────────────────────────
    let player_e = commands.spawn((
        Player, Transform::from_xyz(0.0, 0.0, 18.0),
        GlobalTransform::default(), Visibility::default(),
        PlayerVelocity { vertical: 0.0, knockback: Vec3::ZERO, roll_timer: 0.0, roll_dir: Vec3::ZERO },
    )).id();
    let camera_e = commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.7, 0.0),
        PlayerCamera { pitch: 0.0, bob_timer: 0.0 },
        // Cold moonlit haze that swallows the far distance — keeps the dark mood
        DistanceFog {
            color: Color::srgb(0.04, 0.05, 0.11),
            falloff: FogFalloff::Linear { start: 110.0, end: 1150.0 },
            ..default()
        },
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

    // Red glow light along the blade — off until the sword deflects the eye beam
    commands.spawn((
        PointLight { color: Color::srgb(1.0, 0.12, 0.05), intensity: 0.0, range: 8.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(0.0, 0.25, 0.0),
        SwordGlowLight,
    )).set_parent(sword_root);

    // All the steel-sword meshes live under one node so the Ruined Blade can replace them.
    let steel_root = commands.spawn((Transform::default(), GlobalTransform::default(), Visibility::default(), SteelVis)).set_parent(sword_root).id();

    // ── Blade — thin, tapered, with fuller + bright edges + pointed tip (shortened) ──
    // Lower blade (wider section)
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.034, 0.24, 0.006))),
        MeshMaterial3d(blade_mat.clone()),
        Transform::from_xyz(0.0, 0.16, 0.0), Visibility::default(),
    )).set_parent(steel_root);
    // Upper blade (tapered narrower)
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.022, 0.10, 0.005))),
        MeshMaterial3d(blade_mat.clone()),
        Transform::from_xyz(0.0, 0.33, 0.0), Visibility::default(),
    )).set_parent(steel_root);
    // Pointed tip — flattened 4-sided cone (pyramid)
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.013, height: 0.07 }.mesh().resolution(4))),
        MeshMaterial3d(blade_mat.clone()),
        Transform::from_xyz(0.0, 0.415, 0.0).with_scale(Vec3::new(1.0, 1.0, 0.45)),
        Visibility::default(),
    )).set_parent(steel_root);
    // Central fuller — bright groove down the blade face
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.006, 0.34, 0.008))),
        MeshMaterial3d(blade_edge.clone()),
        Transform::from_xyz(0.0, 0.19, 0.0), Visibility::default(),
    )).set_parent(steel_root);
    // Bright cutting edges (left + right)
    for ex in [-0.016f32, 0.016] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.004, 0.24, 0.0075))),
            MeshMaterial3d(blade_edge.clone()),
            Transform::from_xyz(ex, 0.16, 0.0), Visibility::default(),
        )).set_parent(steel_root);
    }

    // ── Crossguard — slim bar with upswept quillon tips ──
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.19, 0.026, 0.042))),
        MeshMaterial3d(gold_mat.clone()),
        Transform::from_xyz(0.0, -0.012, 0.0), Visibility::default(),
    )).set_parent(steel_root);
    for (qx, qrz) in [(-0.092f32, 0.5f32), (0.092, -0.5)] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.05, 0.022, 0.040))),
            MeshMaterial3d(gold_mat.clone()),
            Transform::from_xyz(qx, 0.004, 0.0).with_rotation(Quat::from_rotation_z(qrz)),
            Visibility::default(),
        )).set_parent(steel_root);
    }

    // ── Grip — wrapped leather cylinder with gold ferrules ──
    commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.017, half_height: 0.075 })),
        MeshMaterial3d(grip_mat),
        Transform::from_xyz(0.0, -0.10, 0.0), Visibility::default(),
    )).set_parent(steel_root);
    for ry in [-0.055f32, -0.105, -0.145] {
        commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.019, half_height: 0.006 })),
            MeshMaterial3d(gold_mat.clone()),
            Transform::from_xyz(0.0, ry, 0.0), Visibility::default(),
        )).set_parent(steel_root);
    }
    // ── Pommel — round gold knob ──
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.032))),
        MeshMaterial3d(gold_mat),
        Transform::from_xyz(0.0, -0.19, 0.0), Visibility::default(),
    )).set_parent(steel_root);

    // ── The held Blade of the Ruined King (hidden until claimed) — its own model ──
    let ruined_root = commands.spawn((
        Transform::from_xyz(0.0, 0.05, 0.0).with_scale(Vec3::splat(0.14)),
        GlobalTransform::default(), Visibility::Hidden, RuinedVis,
    )).set_parent(sword_root).id();
    build_ruined_blade(&mut commands, &mut meshes, &mut materials, ruined_root, 1.0);

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
        ArtifactSpin { dir: 1.0 }, ArtifactVisual { kind: ArtifactKind::Lightning },
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
        ArtifactSpin { dir: -1.0 }, ArtifactVisual { kind: ArtifactKind::Lightning },
    )).set_parent(hand_root).id();
    commands.spawn((Mesh3d(meshes.add(Annulus::new(0.118, 0.126))), MeshMaterial3d(ring_mat.clone()),
        Transform::from_xyz(0.0, 0.0, 0.004))).set_parent(orbit_root);
    for k in 0..3u32 {
        let a = k as f32 / 3.0 * std::f32::consts::TAU + std::f32::consts::FRAC_PI_2;
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.02))), MeshMaterial3d(ball_mat.clone()),
            Transform::from_xyz(a.cos() * 0.122, a.sin() * 0.122, 0.004))).set_parent(orbit_root);
    }

    // ── Flame artifact: a smoldering ember core in a spinning ring ──
    let flame_core = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.45, 0.05),
        emissive: LinearRgba::new(8.0, 2.6, 0.2, 1.0), unlit: true, ..default() });
    let flame_ring = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.6, 0.1, 0.7),
        emissive: LinearRgba::new(5.0, 1.6, 0.1, 1.0), unlit: true, alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default() });
    let flame_root = commands.spawn((
        Transform::from_translation(art_pos), GlobalTransform::default(), Visibility::Hidden,
        ArtifactSpin { dir: 1.0 }, ArtifactVisual { kind: ArtifactKind::Flame },
    )).set_parent(hand_root).id();
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.05))), MeshMaterial3d(flame_core.clone()),
        Transform::default())).set_parent(flame_root);
    commands.spawn((Mesh3d(meshes.add(Annulus::new(0.08, 0.10))), MeshMaterial3d(flame_ring.clone()),
        Transform::default())).set_parent(flame_root);
    for k in 0..6u32 {
        let a = k as f32 / 6.0 * std::f32::consts::TAU;
        commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.018, height: 0.05 }.mesh().resolution(5))), MeshMaterial3d(flame_core.clone()),
            Transform::from_xyz(a.cos() * 0.10, a.sin() * 0.10, 0.0)
                .with_rotation(Quat::from_rotation_z(a - std::f32::consts::FRAC_PI_2)))).set_parent(flame_root);
    }

    // ── Paladin artifact: a proper kite shield — steel face, gold trim, cross & gem, halo ──
    let pal_gold = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.84, 0.3),
        emissive: LinearRgba::new(4.0, 2.8, 0.6, 1.0), metallic: 0.9, perceptual_roughness: 0.2, ..default() });
    let pal_steel = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.66, 0.74), metallic: 0.9, perceptual_roughness: 0.3, ..default() });
    let pal_gem = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.85, 1.0), emissive: LinearRgba::new(1.0, 3.0, 6.0, 1.0), unlit: true, ..default() });
    let pal_halo = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.9, 0.5, 0.6),
        emissive: LinearRgba::new(3.0, 2.2, 0.6, 1.0), unlit: true, alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default() });
    // Shield doesn't spin (a turning shield looks odd); just a gentle static emblem with a halo.
    let pal_root = commands.spawn((
        Transform::from_translation(art_pos), GlobalTransform::default(), Visibility::Hidden,
        ArtifactVisual { kind: ArtifactKind::Paladin },
    )).set_parent(hand_root).id();
    // Halo behind
    let halo = commands.spawn((
        Transform::default(), GlobalTransform::default(), Visibility::default(), ArtifactSpin { dir: 0.6 },
    )).set_parent(pal_root).id();
    commands.spawn((Mesh3d(meshes.add(Annulus::new(0.11, 0.125))), MeshMaterial3d(pal_halo.clone()),
        Transform::default())).set_parent(halo);
    // Gold backing (slightly larger = trim), steel face on top, pointed kite tip below
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.105, 0.13, 0.018))), MeshMaterial3d(pal_gold.clone()),
        Transform::from_xyz(0.0, 0.01, 0.0))).set_parent(pal_root);
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.085, 0.11, 0.024))), MeshMaterial3d(pal_steel.clone()),
        Transform::from_xyz(0.0, 0.012, 0.006))).set_parent(pal_root);
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.06, height: 0.07 }.mesh().resolution(3))), MeshMaterial3d(pal_gold.clone()),
        Transform::from_xyz(0.0, -0.085, 0.0).with_rotation(Quat::from_rotation_z(std::f32::consts::PI)))).set_parent(pal_root);
    // Cross
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.016, 0.085, 0.03))), MeshMaterial3d(pal_gold.clone()),
        Transform::from_xyz(0.0, 0.015, 0.014))).set_parent(pal_root);
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.06, 0.016, 0.03))), MeshMaterial3d(pal_gold.clone()),
        Transform::from_xyz(0.0, 0.03, 0.014))).set_parent(pal_root);
    // Center gem
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.018))), MeshMaterial3d(pal_gem.clone()),
        Transform::from_xyz(0.0, 0.03, 0.02))).set_parent(pal_root);

    // ── Trident artifact: a small glowing aqua trident head ──
    let tri_aqua = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.7, 1.0),
        emissive: LinearRgba::new(0.8, 3.0, 5.0, 1.0), metallic: 0.6, perceptual_roughness: 0.25, ..default() });
    let tri_glow = materials.add(StandardMaterial {
        base_color: Color::srgba(0.4, 0.8, 1.0, 0.6),
        emissive: LinearRgba::new(0.6, 2.5, 4.0, 1.0), unlit: true, alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default() });
    let trident_root = commands.spawn((
        Transform::from_translation(art_pos), GlobalTransform::default(), Visibility::Hidden,
        ArtifactSpin { dir: 1.0 }, ArtifactVisual { kind: ArtifactKind::Trident },
    )).set_parent(hand_root).id();
    commands.spawn((Mesh3d(meshes.add(Annulus::new(0.085, 0.10))), MeshMaterial3d(tri_glow.clone()),
        Transform::default())).set_parent(trident_root);
    // shaft
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.014, 0.16, 0.014))), MeshMaterial3d(tri_aqua.clone()),
        Transform::from_xyz(0.0, -0.02, 0.0))).set_parent(trident_root);
    // three prongs
    for px in [-0.03f32, 0.0, 0.03] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.01, 0.06, 0.01))), MeshMaterial3d(tri_aqua.clone()),
            Transform::from_xyz(px, 0.09, 0.0))).set_parent(trident_root);
    }
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.02, height: 0.04 }.mesh().resolution(5))), MeshMaterial3d(tri_aqua.clone()),
        Transform::from_xyz(0.0, 0.13, 0.0))).set_parent(trident_root);

    // ── Telekinesis artifact: a translucent white orb with two counter-spinning rings ──
    let tk_ball = materials.add(StandardMaterial {
        base_color: Color::srgba(0.95, 0.97, 1.0, 0.35),
        emissive: LinearRgba::new(2.2, 2.4, 3.0, 1.0), unlit: true, alpha_mode: AlphaMode::Blend, ..default() });
    let tk_ring = materials.add(StandardMaterial {
        base_color: Color::srgba(0.8, 0.9, 1.0, 0.8),
        emissive: LinearRgba::new(1.6, 2.0, 3.0, 1.0), unlit: true, alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default() });
    // The orb itself doesn't spin; the two rings do (opposite directions).
    let tk_root = commands.spawn((
        Transform::from_translation(art_pos), GlobalTransform::default(), Visibility::Hidden,
        ArtifactVisual { kind: ArtifactKind::Telekinesis },
    )).set_parent(hand_root).id();
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.06))), MeshMaterial3d(tk_ball.clone()),
        Transform::default())).set_parent(tk_root);
    let tk_o1 = commands.spawn((
        Transform::default(), GlobalTransform::default(), Visibility::default(), ArtifactSpin { dir: 1.0 },
    )).set_parent(tk_root).id();
    commands.spawn((Mesh3d(meshes.add(Annulus::new(0.10, 0.115))), MeshMaterial3d(tk_ring.clone()),
        Transform::default())).set_parent(tk_o1);
    let tk_o2 = commands.spawn((
        Transform::default(), GlobalTransform::default(), Visibility::default(), ArtifactSpin { dir: -1.0 },
    )).set_parent(tk_root).id();
    commands.spawn((Mesh3d(meshes.add(Annulus::new(0.13, 0.145))), MeshMaterial3d(tk_ring.clone()),
        Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2 * 0.6)))).set_parent(tk_o2);

    // ── Black Hole artifact: a pure-black orb ringed by a glowing accretion disc ──
    let bh_core = materials.add(StandardMaterial { base_color: Color::BLACK, unlit: true, ..default() });
    let bh_disc = materials.add(StandardMaterial { base_color: Color::srgb(0.7, 0.3, 1.0),
        emissive: LinearRgba::new(4.0, 1.0, 6.0, 1.0), unlit: true, alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default() });
    let bh_root = commands.spawn((
        Transform::from_translation(art_pos), GlobalTransform::default(), Visibility::Hidden,
        ArtifactVisual { kind: ArtifactKind::BlackHole },
    )).set_parent(hand_root).id();
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.07))), MeshMaterial3d(bh_core.clone()), Transform::default())).set_parent(bh_root);
    let bh_o1 = commands.spawn((Transform::default(), GlobalTransform::default(), Visibility::default(), ArtifactSpin { dir: 1.4 })).set_parent(bh_root).id();
    commands.spawn((Mesh3d(meshes.add(Annulus::new(0.10, 0.16))), MeshMaterial3d(bh_disc.clone()),
        Transform::from_rotation(Quat::from_rotation_x(1.2)))).set_parent(bh_o1);
    let bh_o2 = commands.spawn((Transform::default(), GlobalTransform::default(), Visibility::default(), ArtifactSpin { dir: -2.0 })).set_parent(bh_root).id();
    commands.spawn((Mesh3d(meshes.add(Annulus::new(0.17, 0.2))), MeshMaterial3d(bh_disc.clone()),
        Transform::from_rotation(Quat::from_rotation_x(1.2)))).set_parent(bh_o2);

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

    // ── Bow (held) — a smoothly arced longbow with an animatable drawn nock ──
    let bow_w = materials.add(StandardMaterial { base_color: Color::srgb(0.40, 0.26, 0.12), perceptual_roughness: 0.9, ..default() });
    let bow_s = materials.add(StandardMaterial { base_color: Color::srgb(0.88, 0.85, 0.72), perceptual_roughness: 0.7, ..default() });
    let arrow_m = materials.add(StandardMaterial { base_color: Color::srgb(0.5, 0.35, 0.2), perceptual_roughness: 0.9, ..default() });
    let bow = commands.spawn((
        Transform::from_xyz(0.26, -0.14, -0.5).with_rotation(Quat::from_rotation_y(0.22)),
        GlobalTransform::default(), Visibility::Hidden, HeldVisual { kind: ItemKind::Bow },
    )).set_parent(camera_e).id();
    // Smoothly-curved limb: short segments along a vertical arc that bows forward (-Z)
    let bow_pts: Vec<Vec3> = (0..=12).map(|k| {
        let u = k as f32 / 12.0;
        let y = -0.42 + u * 0.84;
        let bow_out = (1.0 - (2.0 * u - 1.0).powi(2)) * 0.16; // belly bulges forward
        Vec3::new(0.0, y, -bow_out)
    }).collect();
    for w in bow_pts.windows(2) {
        let (p0, p1) = (w[0], w[1]);
        let mid = (p0 + p1) * 0.5;
        let d = p1 - p0; let len = d.length().max(0.001);
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.026, len * 1.1, 0.026))), MeshMaterial3d(bow_w.clone()),
            Transform::from_translation(mid).with_rotation(Quat::from_rotation_arc(Vec3::Y, d / len)))).set_parent(bow);
    }
    // Grip wrap at the belly
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.04, 0.12, 0.05))), MeshMaterial3d(arrow_m.clone()),
        Transform::from_xyz(0.0, 0.0, -0.16))).set_parent(bow);
    // ── Nock group (string + arrow) — pulled back as the bow is drawn (bow_draw_anim) ──
    let nock = commands.spawn((
        Transform::default(), GlobalTransform::default(), Visibility::default(), BowNock,
    )).set_parent(bow).id();
    // string drawn into a shallow V toward the nock point (at z = +0.02 resting)
    for s in [-1.0f32, 1.0] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.006, 0.43, 0.006))), MeshMaterial3d(bow_s.clone()),
            Transform::from_xyz(0.0, s * 0.21, 0.0).with_rotation(Quat::from_rotation_x(s * 0.05)))).set_parent(nock);
    }
    // arrow shaft + head pointing forward (-Z)
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.012, 0.012, 0.7))), MeshMaterial3d(arrow_m.clone()),
        Transform::from_xyz(0.0, 0.0, -0.32))).set_parent(nock);
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.028, height: 0.08 }.mesh().resolution(4))), MeshMaterial3d(bow_s.clone()),
        Transform::from_xyz(0.0, 0.0, -0.69).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)))).set_parent(nock);

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
    mut health: ResMut<PlayerHealth>,
    mut sword_q: Query<(&mut Transform, &mut Sword)>,
    camera_q: Query<&GlobalTransform, With<PlayerCamera>>,
    mut skeleton_q: Query<(Entity, &GlobalTransform, &mut Skeleton)>,
    mut dragon_q: Query<(Entity, &GlobalTransform, &mut Dragon)>,
    mut enemy_q: Query<(Entity, &GlobalTransform, &mut Enemy)>,
    mut medusa_q: Query<(&GlobalTransform, &mut Medusa)>,
    mut kills: ResMut<KillStats>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let window = windows.single();
    let (mut t, mut sword) = sword_q.single_mut();

    let idle_rot = Quat::from_euler(EulerRot::XYZ,
        (-24f32).to_radians(), (6f32).to_radians(), (16f32).to_radians());
    let idle_pos = Vec3::new(0.26, -0.18, -0.40);

    // ── Right mouse: steel sword blocks; the Ruined Blade summons mist zombies instead ──
    let rmb_ready = window.cursor_options.grab_mode == CursorGrabMode::Locked
        && inv.selected == ItemKind::Sword && !sword.swinging;
    if inv.has_ruined_blade && rmb_ready && mouse.just_pressed(MouseButton::Right) {
        let c = camera_q.single().translation();
        for k in 0..10u32 {
            let a = k as f32 / 10.0 * std::f32::consts::TAU;
            let r = 3.0 + (k % 3) as f32 * 1.2;
            spawn_mist_zombie(&mut commands, &mut meshes, &mut materials, Vec3::new(c.x + a.cos() * r, 0.0, c.z + a.sin() * r));
        }
    }
    let blocking = !inv.has_ruined_blade && rmb_ready && mouse.pressed(MouseButton::Right);
    health.blocking = blocking;

    if mouse.just_pressed(MouseButton::Left)
        && window.cursor_options.grab_mode == CursorGrabMode::Locked
        && inv.selected == ItemKind::Sword
        && !sword.swinging && !blocking
    {
        sword.swinging = true;
        sword.timer = 0.0;
        sword.hit_registered = false;
    }

    // Hold the raised-guard pose while blocking (sword brought up across the body)
    if blocking {
        let block_rot = Quat::from_euler(EulerRot::XYZ,
            (12f32).to_radians(), (-72f32).to_radians(), (82f32).to_radians());
        t.rotation = block_rot;
        t.translation = Vec3::new(0.08, -0.06, -0.40);
    } else if !sword.swinging {
        t.rotation = idle_rot;
        t.translation = idle_pos;
    }

    if sword.swinging {
        sword.timer += time.delta_secs();
        if inv.has_ruined_blade {
            // ── Ruined Blade: a heavy, quick forward THRUST that holds ~0.5s ──
            let tt = sword.timer;
            let ext = if tt < 0.13 { tt / 0.13 }                         // snap forward
                      else if tt < 0.63 { 1.0 }                          // hold (weight)
                      else { (1.0 - (tt - 0.63) / 0.30).max(0.0) };      // retract
            t.rotation    = Quat::from_euler(EulerRot::XYZ, -1.48, 0.12, 0.18);  // point blade forward (-Z)
            t.translation = Vec3::new(0.18, -0.12, -0.40) + Vec3::new(0.0, 0.0, -0.62 * ext);
        } else {
            let progress = (sword.timer / 0.38).min(1.0);
            let arc = (progress * std::f32::consts::PI).sin();
            let swing_rot = Quat::from_euler(EulerRot::XYZ,
                (-55.0 * arc).to_radians(), (-35.0 * arc).to_radians(), 0.0f32.to_radians());
            t.rotation    = idle_rot * swing_rot;
            t.translation = idle_pos + Vec3::new(-0.12 * arc, 0.06 * arc, -0.08 * arc);
        }

        // Hit check at swing peak — all enemies flash red and are knocked back
        if !sword.hit_registered && sword.timer > 0.13 {
            sword.hit_registered = true;
            let mut struck = false;
            let cam_gt = camera_q.single();
            let (_, rot, cam_pos) = cam_gt.to_scale_rotation_translation();
            let fwd = rot * Vec3::NEG_Z;
            for (entity, skel_gt, mut skel) in skeleton_q.iter_mut() {
                if skel.state == SkeletonState::Dead { continue; }
                let to = skel_gt.translation() + Vec3::Y - cam_pos;
                let dist = to.length();
                if dist < 3.6 && dist > 0.1 && fwd.dot(to / dist) > 0.3 {
                    // Blade of the Ruined King: the struck foe rises as a mist zombie ally
                    if inv.has_ruined_blade {
                        spawn_mist_zombie(&mut commands, &mut meshes, &mut materials, skel_gt.translation());
                        commands.entity(entity).despawn_recursive();
                        struck = true;
                        continue;
                    }
                    skel.health -= 1.0;
                    skel.damage_flash = 0.25;
                    struck = true;
                    skel.knockback_vel = Vec3::new(to.x, 0.0, to.z).normalize_or_zero() * 11.0;
                    if skel.health <= 0.0 {
                        skel.state = SkeletonState::Dead;
                        kills.skeletons += 1;
                        death_burst(&mut commands, &mut meshes, &mut materials, skel_gt.translation(), Color::srgb(0.8, 0.85, 1.0));
                        commands.entity(entity).despawn_recursive();
                    }
                }
            }
            for (_entity, dgt, mut drag) in dragon_q.iter_mut() {
                if drag.state == DragonState::Dead { continue; }
                let hit = dragon_points(dgt).iter().any(|&bp| {
                    let to = bp - cam_pos; let d = to.length();
                    d > 0.1 && d < 8.0 && fwd.dot(to / d) > 0.2
                });
                if hit {
                    drag.health -= 1.0;
                    drag.damage_flash = 0.25;
                    struck = true;
                    // Death (and the midair fireworks) is handled in dragon_ai.
                }
            }
            for (entity, egt, mut en) in enemy_q.iter_mut() {
                let to = egt.translation() + Vec3::Y - cam_pos;
                let dist = to.length();
                if dist < 3.6 && dist > 0.1 && fwd.dot(to / dist) > 0.3 {
                    if inv.has_ruined_blade {
                        spawn_mist_zombie(&mut commands, &mut meshes, &mut materials, egt.translation());
                        commands.entity(entity).despawn_recursive();
                        struck = true;
                        continue;
                    }
                    en.health -= 1.0;
                    en.damage_flash = 0.25;
                    struck = true;
                    en.knockback_vel = Vec3::new(to.x, 0.0, to.z).normalize_or_zero() * 11.0;
                    if en.health <= 0.0 {
                        kills.beasts += 1;
                        death_burst(&mut commands, &mut meshes, &mut materials, egt.translation(), Color::srgb(1.0, 0.4, 0.3));
                        commands.entity(entity).despawn_recursive();
                    }
                }
            }
            for (mgt, mut m) in medusa_q.iter_mut() {
                if m.state == MedusaState::Dead { continue; }
                let to = mgt.translation() + Vec3::Y * 3.0 - cam_pos;
                let dist = to.length();
                if dist < 6.5 && dist > 0.1 && fwd.dot(to / dist) > 0.2 {
                    m.health -= 2.0; m.damage_flash = 0.25; struck = true;
                }
            }
            let _ = struck;
        }

        let swing_end = if inv.has_ruined_blade { 0.95 } else { 0.38 };
        if sword.timer >= swing_end {
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
    key: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    time: Res<Time>,
    arts: Res<Artifacts>,
    mut mana: ResMut<Mana>,
    mut bolt_q: Query<(&mut Transform, &mut Visibility, &LightningBolt)>,
    mut light_q: Query<&mut PointLight, With<LightningLight>>,
) {
    let window = windows.single();
    // Lightning is the off-hand spell — fired with Q while the Lightning artifact is equipped
    let active = key.pressed(KeyCode::KeyQ)
        && arts.selected == ArtifactKind::Lightning
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
    mut enemy_q: Query<(&mut Transform, &mut Enemy, Option<&OrcBrute>, Option<&Launched>), (Without<Succubus>, Without<Sauron>)>,
    player_q: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut player_vel_q: Query<&mut PlayerVelocity, With<Player>>,
    mut health: ResMut<PlayerHealth>,
) {
    let pt = player_q.single();
    let pp = pt.translation;
    let dt = time.delta_secs();

    for (mut t, mut e, orc, launched) in enemy_q.iter_mut() {
        if launched.is_some() { continue; } // airborne — update_launched owns its motion
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
                        health.take(20.0, 0.9);
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
                health.take(40.0, 0.9);
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
        build_skeleton(&mut commands, &mut meshes, &mut materials, *pos, i as f32);
    }
}

// Builds one fully-rigged skeleton at `pos`. Shared by the initial spawn and the
// respawner so they stay identical.
fn build_skeleton(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
    i: f32,
) {
    let bone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.80, 0.76, 0.64),
        perceptual_roughness: 0.9, ..default()
    });
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

    let angle = i * 0.9 + 0.5;
    let patrol_dir = Vec3::new(angle.cos(), 0.0, angle.sin());
    let b = bone.clone();
    commands.spawn((
        Transform::from_translation(pos),
        GlobalTransform::default(),
        Visibility::default(),
        Skeleton { health: 5.0, state: SkeletonState::Patrol,
            attack_timer: 1.5, patrol_timer: 2.0 + i * 0.4, patrol_dir,
            damage_flash: 0.0, knockback_vel: Vec3::ZERO, anim_phase: i },
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

// Periodically repopulate the map with skeletons scattered around the player,
// up to a cap, so the world keeps roaming foes.
fn enemy_respawn(
    time: Res<Time>,
    mut spawner: ResMut<Spawner>,
    realm: Res<Realm>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_q: Query<&Transform, With<Player>>,
    skels: Query<(), With<Skeleton>>,
) {
    if realm.in_sky { return; } // no foes in the heavenly parkour realm
    spawner.timer -= time.delta_secs();
    if spawner.timer > 0.0 { return; }
    spawner.timer = 5.0;
    if skels.iter().count() >= 28 { return; }
    let pp = player_q.single().translation;
    let et = time.elapsed_secs();
    let hash = |n: f32| (n.sin() * 43758.5).fract().abs();
    for k in 0..3u32 {
        let a = hash(et * 3.7 + k as f32 * 11.3) * std::f32::consts::TAU;
        let d = 60.0 + hash(et * 1.9 + k as f32 * 5.1) * 340.0; // scattered, off-screen-ish
        let pos = Vec3::new(pp.x + a.cos() * d, 0.0, pp.z + a.sin() * d);
        build_skeleton(&mut commands, &mut meshes, &mut materials, pos, et + k as f32);
    }
}

fn skeleton_ai(
    time: Res<Time>,
    mut commands: Commands,
    mut skel_q: Query<(Entity, &mut Transform, &mut Skeleton, Option<&Launched>)>,
    player_q: Query<&Transform, (With<Player>, Without<Skeleton>)>,
    mut player_vel_q: Query<&mut PlayerVelocity, With<Player>>,
    mut health: ResMut<PlayerHealth>,
) {
    if health.hurt_timer > 0.0 { health.hurt_timer -= time.delta_secs(); }
    let pt = player_q.single();
    let pp = pt.translation;

    for (entity, mut t, mut sk, launched) in skel_q.iter_mut() {
        if sk.state == SkeletonState::Dead { continue; }
        if launched.is_some() { continue; } // airborne — update_launched owns its motion

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
                    health.take(20.0, 0.9);
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
    key: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    time: Res<Time>,
    arts: Res<Artifacts>,
    mana: Res<Mana>,
    camera_q: Query<&GlobalTransform, With<PlayerCamera>>,
    mut skel_q: Query<(Entity, &GlobalTransform, &mut Skeleton)>,
    mut dragon_q: Query<(Entity, &GlobalTransform, &mut Dragon)>,
    mut enemy_q: Query<(Entity, &GlobalTransform, &mut Enemy)>,
    mut medusa_q: Query<(Entity, &GlobalTransform, &mut Medusa)>,
    mut shock_q: Query<&mut Shock>,
    mut kills: ResMut<KillStats>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let window = windows.single();
    if !key.pressed(KeyCode::KeyQ) || arts.selected != ArtifactKind::Lightning { return; }
    if window.cursor_options.grab_mode != CursorGrabMode::Locked { return; }
    if mana.current <= 0.0 { return; } // no mana, no lightning
    let cam = camera_q.single();
    let (_, rot, pos) = cam.to_scale_rotation_translation();
    let fwd = rot * Vec3::NEG_Z;
    let dt = time.delta_secs();

    // ── Chain lightning: arc from directly-struck foes to nearby ones ──
    let mut pts: Vec<(Entity, Vec3)> = Vec::new();
    for (e, g, sk) in skel_q.iter() { if sk.state != SkeletonState::Dead { pts.push((e, g.translation() + Vec3::Y)); } }
    for (e, g, _) in enemy_q.iter() { pts.push((e, g.translation() + Vec3::Y)); }
    let primaries: Vec<Vec3> = pts.iter().filter(|(_, p)| {
        let to = *p - pos; let d = to.length(); d > 0.5 && d < 9.0 && fwd.dot(to / d) > 0.5
    }).map(|(_, p)| *p).collect();
    let mut chained: Vec<Entity> = Vec::new();
    let mut affected: Vec<Vec3> = primaries.clone();
    for &sp in &primaries {
        for &(e, p) in &pts {
            let d = (p - sp).length();
            if d > 0.3 && d < 10.0 {
                let is_primary = primaries.iter().any(|&q| (q - p).length() < 0.25);
                if !is_primary && !chained.contains(&e) {
                    chained.push(e);
                    affected.push(p);
                }
            }
        }
    }
    // Up to 3 strands between nearby affected foes (one bolt per link)
    let mut strands: Vec<(Vec3, Vec3)> = Vec::new();
    'outer: for i in 0..affected.len() {
        for j in (i + 1)..affected.len() {
            if (affected[i] - affected[j]).length() < 11.0 {
                strands.push((affected[i], affected[j]));
                if strands.len() >= 3 { break 'outer; }
            }
        }
    }

    for (entity, sgt, mut sk) in skel_q.iter_mut() {
        if sk.state == SkeletonState::Dead { continue; }
        let to = sgt.translation() + Vec3::Y - pos;
        let dist = to.length();
        if dist < 0.5 { continue; }
        if (dist < 9.0 && fwd.dot(to / dist) > 0.5) || chained.contains(&entity) {
            sk.health -= 2.0 * dt;
            if let Ok(mut s) = shock_q.get_mut(entity) { s.timer = 0.12; }
            if sk.health <= 0.0 { sk.state = SkeletonState::Dead; kills.skeletons += 1;
                death_burst(&mut commands, &mut meshes, &mut materials, sgt.translation(), Color::srgb(0.8, 0.85, 1.0));
                commands.entity(entity).despawn_recursive(); }
        }
    }
    for (entity, dgt, mut drag) in dragon_q.iter_mut() {
        if drag.state == DragonState::Dead { continue; }
        // Long reach so the airborne dragon can be shocked out of the sky.
        let hit = dragon_points(dgt).iter().any(|&bp| {
            let to = bp - pos; let d = to.length();
            d > 0.5 && d < 70.0 && fwd.dot(to / d) > 0.6
        });
        if hit {
            drag.health -= 2.0 * dt;
            if let Ok(mut s) = shock_q.get_mut(entity) { s.timer = 0.12; }
            // Dragon death + fireworks handled in dragon_ai.
        }
    }
    // Witch / Knight / Bat
    for (entity, egt, mut en) in enemy_q.iter_mut() {
        let to = egt.translation() + Vec3::Y - pos;
        let dist = to.length();
        if dist < 0.5 { continue; }
        if (dist < 9.0 && fwd.dot(to / dist) > 0.5) || chained.contains(&entity) {
            en.health -= 2.0 * dt;
            if let Ok(mut s) = shock_q.get_mut(entity) { s.timer = 0.12; }
            if en.health <= 0.0 { kills.beasts += 1;
                death_burst(&mut commands, &mut meshes, &mut materials, egt.translation(), Color::srgb(0.7, 0.8, 1.0));
                commands.entity(entity).despawn_recursive(); }
        }
    }
    // Jagged crackling strands between chained foes — one bolt per link (max 3),
    // re-spawned each frame to flicker.
    if !strands.is_empty() {
        let strand_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.92, 0.96, 1.0), emissive: LinearRgba::new(12.0, 14.0, 18.0, 1.0), unlit: true, ..default() });
        let seg = meshes.add(Cuboid::new(0.025, 0.025, 1.0));
        let tt2 = (time.elapsed_secs() * 30.0).floor();
        let hh = |n: f32| (n.sin() * 43758.547).fract();
        for (a, b) in strands {
            let axis = b - a;
            let total = axis.length().max(0.01);
            let dirn = axis / total;
            let mut perp = dirn.cross(Vec3::Y);
            if perp.length_squared() < 1e-4 { perp = dirn.cross(Vec3::X); }
            perp = perp.normalize();
            let perp2 = dirn.cross(perp).normalize();
            // A soft white glow riding the midpoint of the arc
            commands.spawn((
                PointLight { color: Color::srgb(0.9, 0.95, 1.0), intensity: 120_000.0, range: 9.0, shadows_enabled: false, ..default() },
                Transform::from_translation((a + b) * 0.5),
                Transient { life: 0.06 },
            ));
            // A single jagged 5-segment bolt per link (3 links max → 3 strands total)
            for strand in 0..1u32 {
                let segs = 5u32;
                let point = |k: u32| -> Vec3 {
                    let f = k as f32 / segs as f32;
                    let base = a.lerp(b, f);
                    if k == 0 || k == segs { return base; }
                    let s = k as f32 * 9.3 + tt2 * 1.7 + strand as f32 * 5.0 + (a.x + b.z) * 0.5;
                    let amp = 0.55;
                    base + perp * (hh(s) * amp) + perp2 * (hh(s + 2.0) * amp)
                };
                for k in 0..segs {
                    let p0 = point(k);
                    let p1 = point(k + 1);
                    let m = (p0 + p1) * 0.5;
                    let dd = p1 - p0;
                    let l = dd.length().max(0.01);
                    commands.spawn((
                        Mesh3d(seg.clone()), MeshMaterial3d(strand_mat.clone()),
                        Transform::from_translation(m).with_rotation(Quat::from_rotation_arc(Vec3::Z, dd / l)).with_scale(Vec3::new(1.0, 1.0, l)),
                        Transient { life: 0.06 },
                    ));
                }
            }
        }
    }
    for (entity, mgt, mut m) in medusa_q.iter_mut() {
        if m.state == MedusaState::Dead { continue; }
        let to = mgt.translation() + Vec3::Y * 3.0 - pos;
        let dist = to.length();
        if dist > 0.5 && dist < 11.0 && fwd.dot(to / dist) > 0.4 {
            m.health -= 4.0 * dt;
            if let Ok(mut s) = shock_q.get_mut(entity) { s.timer = 0.12; }
        }
    }
}

// ── HUD ───────────────────────────────────────────────────────────────────────
fn setup_hud(mut commands: Commands) {
    // Health bar (red) at top-left, with a golden temporary-shield overlay on top
    commands.spawn((
        Node { position_type: PositionType::Absolute, left: Val::Px(16.0), top: Val::Px(18.0),
               width: Val::Px(300.0), height: Val::Px(22.0), border: UiRect::all(Val::Px(2.0)), ..default() },
        BorderColor(Color::srgb(0.12, 0.04, 0.04)),
        BackgroundColor(Color::srgb(0.16, 0.04, 0.04)),
    )).with_children(|p| {
        // Red current-health fill — full (100%) when at max HP
        p.spawn((
            Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0),
                   width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
            BackgroundColor(Color::srgb(0.82, 0.10, 0.10)),
            HealthBar,
        ));
    });
    // Golden paladin shield — a SEPARATE bar that grows to the right of the full
    // health bar (it adds extra health on top; it never eats into the red bar).
    commands.spawn((
        Node { position_type: PositionType::Absolute, left: Val::Px(319.0), top: Val::Px(18.0),
               width: Val::Px(0.0), height: Val::Px(22.0), ..default() },
        BackgroundColor(Color::srgb(1.0, 0.82, 0.25)),
        GoldenBar,
    ));

    // Stamina bar (green) and Mana bar (blue) under the health bar
    for (top, fill_color, is_stamina) in [
        (46.0f32, Color::srgb(0.2, 0.8, 0.25), true),
        (66.0f32, Color::srgb(0.25, 0.5, 1.0), false),
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
        (ItemKind::Bow,          Color::srgb(0.55, 0.38, 0.18)),
        (ItemKind::HealthPotion, Color::srgb(0.85, 0.15, 0.15)),
        (ItemKind::ManaPotion,   Color::srgb(0.22, 0.42, 0.95)),
    ];
    commands.spawn(Node {
        position_type: PositionType::Absolute,
        bottom: Val::Px(18.0),
        left: Val::Percent(50.0),
        margin: UiRect::left(Val::Px(-183.0)), // centre the 366px-wide row (6 slots)
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

    // ── Medusa boss health bar (hidden until she engages) ──
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
        MedusaBarRoot,
    )).with_children(|p| {
        p.spawn((
            Text::new("Medusa"),
            TextFont { font_size: 22.0, ..default() },
            TextColor(Color::srgb(0.7, 0.9, 0.65)),
        ));
        p.spawn((
            Node { width: Val::Px(560.0), height: Val::Px(16.0), border: UiRect::all(Val::Px(2.0)), ..default() },
            BorderColor(Color::srgb(0.10, 0.15, 0.1)),
            BackgroundColor(Color::srgb(0.06, 0.10, 0.06)),
        )).with_children(|b| {
            b.spawn((
                Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                BackgroundColor(Color::srgb(0.35, 0.7, 0.3)),
                MedusaBarFill,
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

fn update_health_bar(
    time: Res<Time>,
    mut health: ResMut<PlayerHealth>,
    mut red_q: Query<&mut Node, (With<HealthBar>, Without<GoldenBar>)>,
    mut gold_q: Query<&mut Node, (With<GoldenBar>, Without<HealthBar>)>,
) {
    // Decay the golden shield's lifetime; drop any remaining shield when it ends
    if health.golden_timer > 0.0 {
        health.golden_timer -= time.delta_secs();
        if health.golden_timer <= 0.0 { health.golden = 0.0; }
    }
    // Red bar = current HP over max (FULL at spawn). Gold = a separate bar that
    // grows to the right of it (extra shield on top; never reserves red space).
    let max = health.max_hp.max(1.0);
    if let Ok(mut n) = red_q.get_single_mut() {
        n.width = Val::Percent((health.hp / max * 100.0).clamp(0.0, 100.0));
    }
    if let Ok(mut n) = gold_q.get_single_mut() {
        // up to ~150px of gold when at the full +50% shield
        n.width = Val::Px((health.golden / max * 300.0).clamp(0.0, 300.0));
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
    roots: Query<Entity, Or<(With<Skeleton>, With<Enemy>, With<Medusa>)>>,
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

fn flash_medusa(
    time: Res<Time>,
    flash: Option<Res<FlashMats>>,
    mut medusa_q: Query<(Entity, &mut Medusa, &mut Shock)>,
    children_q: Query<&Children>,
    mut mat_q: Query<(&mut MeshMaterial3d<StandardMaterial>, &BodyPart)>,
) {
    let Some(f) = flash else { return; };
    let white = (time.elapsed_secs() * 18.0) as i64 % 2 == 0;
    for (e, mut m, mut shock) in medusa_q.iter_mut() {
        if m.state == MedusaState::Dead { continue; }
        m.damage_flash = (m.damage_flash - time.delta_secs()).max(0.0);
        shock.timer = (shock.timer - time.delta_secs()).max(0.0);
        let over = flash_override(shock.timer, m.damage_flash, white, &f);
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
            health.take(20.0, 0.9);
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
    mut medusa: Query<(&GlobalTransform, &mut Medusa)>,
    mut kills: ResMut<KillStats>,
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
                   || dragon.iter().any(|(_, g, d)| d.state != DragonState::Dead && dragon_points(g).iter().any(|&bp| bp.distance(pos) < 4.0))
                   || medusa.iter().any(|(g, m)| m.state != MedusaState::Dead && (g.translation() + Vec3::Y * 3.0).distance(pos) < 4.0);
        }
        if explode {
            let r = 8.0; // blast radius
            for (e, g, mut s) in skel.iter_mut() {
                if s.state != SkeletonState::Dead && g.translation().distance(pos) < r {
                    s.health -= 20.0; s.state = SkeletonState::Dead; kills.skeletons += 1;
                    death_burst(&mut commands, &mut meshes, &mut materials, g.translation(), Color::srgb(1.0, 0.6, 0.2));
                    commands.entity(e).despawn_recursive();
                }
            }
            for (e, g, mut en) in enemy.iter_mut() {
                if g.translation().distance(pos) < r {
                    en.health -= 20.0;
                    if en.health <= 0.0 { kills.beasts += 1;
                        death_burst(&mut commands, &mut meshes, &mut materials, g.translation(), Color::srgb(1.0, 0.6, 0.2));
                        commands.entity(e).despawn_recursive(); }
                }
            }
            for (_e, g, mut d) in dragon.iter_mut() {
                if d.state != DragonState::Dead && dragon_points(g).iter().any(|&bp| bp.distance(pos) < r) {
                    d.health -= 6.0; d.damage_flash = 0.25;
                    // Dragon death + fireworks handled in dragon_ai.
                }
            }
            for (g, mut m) in medusa.iter_mut() {
                if m.state != MedusaState::Dead && (g.translation() + Vec3::Y * 3.0).distance(pos) < r {
                    m.health -= 8.0; m.damage_flash = 0.25;
                }
            }
            spawn_mushroom(&mut commands, &mut meshes, &mut materials, Vec3::new(pos.x, 0.0, pos.z));
            commands.entity(re).despawn_recursive();
        }
    }
}

// A big, layered nuke blast: blinding flash, expanding shockwave + dust rings,
// a billowing fiery mushroom cloud (grows via animate_mushroom) and flying embers.
fn spawn_mushroom(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    base: Vec3,
) {
    let core_m = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.95, 0.7), emissive: LinearRgba::new(14.0, 10.0, 4.0, 1.0), unlit: true, ..default() });
    let fire = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.55, 0.1), emissive: LinearRgba::new(8.0, 2.8, 0.3, 1.0), unlit: true, ..default() });
    let smoke = materials.add(StandardMaterial {
        base_color: Color::srgb(0.34, 0.27, 0.22), emissive: LinearRgba::new(0.9, 0.55, 0.3, 1.0), unlit: true, ..default() });
    let smoke_dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.15, 0.13), unlit: true, ..default() });
    let flash = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.98, 0.85), emissive: LinearRgba::new(16.0, 14.0, 9.0, 1.0), unlit: true, ..default() });
    let ring_m = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.6, 0.2, 0.7), emissive: LinearRgba::new(7.0, 2.4, 0.4, 1.0),
        unlit: true, alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default() });
    let dust_m = materials.add(StandardMaterial {
        base_color: Color::srgba(0.55, 0.45, 0.35, 0.5), unlit: true, alpha_mode: AlphaMode::Blend, cull_mode: None, double_sided: true, ..default() });

    // 1) Blinding flash + a fierce (fading) light
    commands.spawn((Mesh3d(meshes.add(Sphere::new(5.0))), MeshMaterial3d(flash),
        Transform::from_translation(base + Vec3::Y * 1.0), Transient { life: 0.18 }));
    commands.spawn((
        PointLight { color: Color::srgb(1.0, 0.7, 0.35), intensity: 6_000_000.0, range: 90.0, shadows_enabled: false, ..default() },
        Transform::from_translation(base + Vec3::Y * 3.0), Transient { life: 0.9 },
    ));

    // 2) Expanding ground shockwave ring + a low dust ring
    commands.spawn((
        Mesh3d(meshes.add(Annulus::new(1.2, 2.2))), MeshMaterial3d(ring_m),
        Transform::from_translation(base + Vec3::Y * 0.15).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        Expand { rate: 26.0, life: 0.7 }, NotShadowCaster,
    ));
    commands.spawn((
        Mesh3d(meshes.add(Annulus::new(2.0, 5.0))), MeshMaterial3d(dust_m),
        Transform::from_translation(base + Vec3::Y * 0.5).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        Expand { rate: 14.0, life: 1.2 }, NotShadowCaster,
    ));

    // 3) The billowing mushroom cloud (grows via animate_mushroom)
    commands.spawn((
        Transform::from_translation(base), GlobalTransform::default(), Visibility::default(),
        Mushroom { age: 0.0 }, Transient { life: 2.4 },
        PointLight { color: Color::srgb(1.0, 0.5, 0.15), intensity: 900_000.0, range: 55.0, shadows_enabled: false, ..default() },
    )).with_children(|m| {
        // white-hot core + fireball
        m.spawn((Mesh3d(meshes.add(Sphere::new(2.4))), MeshMaterial3d(core_m.clone()), Transform::from_xyz(0.0, 8.5, 0.0)));
        m.spawn((Mesh3d(meshes.add(Sphere::new(3.6))), MeshMaterial3d(fire.clone()), Transform::from_xyz(0.0, 8.6, 0.0)));
        // rising stem (tapered: wide base, thinner neck)
        m.spawn((Mesh3d(meshes.add(Cylinder { radius: 1.9, half_height: 4.5 })), MeshMaterial3d(smoke.clone()), Transform::from_xyz(0.0, 4.0, 0.0)));
        m.spawn((Mesh3d(meshes.add(Cylinder { radius: 1.2, half_height: 2.0 })), MeshMaterial3d(smoke_dark.clone()), Transform::from_xyz(0.0, 7.0, 0.0)));
        // flat underside of the cap
        m.spawn((Mesh3d(meshes.add(Cylinder { radius: 5.2, half_height: 0.4 })), MeshMaterial3d(smoke_dark.clone()), Transform::from_xyz(0.0, 9.6, 0.0)));
        // billowing cap — a ring of smoke lobes + a glowing rim + a dark dome on top
        for i in 0..8u32 {
            let a = i as f32 / 8.0 * std::f32::consts::TAU;
            m.spawn((Mesh3d(meshes.add(Sphere::new(2.4))), MeshMaterial3d(smoke.clone()),
                Transform::from_xyz(a.cos() * 4.2, 10.6, a.sin() * 4.2)));
            m.spawn((Mesh3d(meshes.add(Sphere::new(1.4))), MeshMaterial3d(fire.clone()),
                Transform::from_xyz(a.cos() * 4.6, 10.0, a.sin() * 4.6)));
        }
        m.spawn((Mesh3d(meshes.add(Sphere::new(4.4))), MeshMaterial3d(smoke_dark.clone()), Transform::from_xyz(0.0, 12.0, 0.0)));
        m.spawn((Mesh3d(meshes.add(Sphere::new(3.0))), MeshMaterial3d(fire.clone()), Transform::from_xyz(0.0, 10.8, 0.0)));
    });

    // 4) Flying embers + rubble flung out of the blast
    let ember = meshes.add(Cuboid::new(0.18, 0.18, 0.18));
    let h = |n: f32| (n.sin() * 43758.547).fract().abs();
    for i in 0..28u32 {
        let a = i as f32 * 2.39996;
        let up = 6.0 + h(i as f32) * 12.0;
        let out = 8.0 + h(i as f32 * 1.7) * 10.0;
        let m = if i % 2 == 0 { fire.clone() } else { smoke.clone() };
        commands.spawn((
            Mesh3d(ember.clone()), MeshMaterial3d(m),
            Transform::from_translation(base + Vec3::Y * 1.0),
            Debris { vel: Vec3::new(a.cos() * out, up, a.sin() * out), life: 1.4 },
        ));
    }
}

// Expand + fade a purely-visual ring/shell, then remove it.
fn update_expand(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Transform, &mut Expand)>,
) {
    let dt = time.delta_secs();
    for (e, mut t, mut x) in q.iter_mut() {
        x.life -= dt;
        let s = t.scale.x + x.rate * dt;
        t.scale = Vec3::new(s, s, s);
        if x.life <= 0.0 { commands.entity(e).despawn_recursive(); }
    }
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
                ItemKind::Bow    => inv.has_bow = true,
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

    // Longbow on a small rack next to the glock
    let bow_wood = materials.add(StandardMaterial { base_color: Color::srgb(0.40, 0.26, 0.12), perceptual_roughness: 0.9, ..default() });
    let bow_string = materials.add(StandardMaterial { base_color: Color::srgb(0.85, 0.82, 0.7), perceptual_roughness: 0.8, ..default() });
    commands.spawn((
        Transform::from_xyz(20.0, 1.0, -78.0).with_rotation(Quat::from_rotation_y(0.4)),
        GlobalTransform::default(), Visibility::default(),
        Pickup { kind: ItemKind::Bow },
    )).with_children(|c| {
        // Two curved limbs (angled) + a riser + string
        for s in [-1.0f32, 1.0] {
            c.spawn((Mesh3d(meshes.add(Cuboid::new(0.06, 0.7, 0.06))), MeshMaterial3d(bow_wood.clone()),
                Transform::from_xyz(0.0, 0.45 * s + 0.5, 0.0).with_rotation(Quat::from_rotation_z(s * 0.5))));
        }
        c.spawn((Mesh3d(meshes.add(Cuboid::new(0.08, 0.4, 0.08))), MeshMaterial3d(bow_wood.clone()), Transform::from_xyz(0.0, 0.5, 0.0)));
        c.spawn((Mesh3d(meshes.add(Cuboid::new(0.012, 1.5, 0.012))), MeshMaterial3d(bow_string.clone()), Transform::from_xyz(-0.18, 0.5, 0.0)));
        c.spawn((PointLight { color: Color::srgb(0.7, 0.9, 0.5), intensity: 180_000.0, range: 14.0, radius: 0.3, shadows_enabled: false, ..default() },
            Transform::from_xyz(0.0, 0.7, 0.0)));
    });
}

// Gentle rolling hills the player can walk up. Each is a smooth grass dome with
// a few invisible terraced walkable steps beneath so you ascend it smoothly-ish.
fn spawn_hills(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let grass = make_grass_texture(&mut images);
    let hill_mat = materials.add(StandardMaterial {
        base_color_texture: Some(grass),
        uv_transform: bevy::math::Affine2::from_scale(Vec2::new(8.0, 8.0)),
        perceptual_roughness: 1.0, ..default()
    });

    // (x, z, sphere radius r, hill height h) — kept clear of structures
    let hills = [
        (90.0, 70.0, 14.0, 4.0), (-120.0, -40.0, 16.0, 4.5), (140.0, -120.0, 15.0, 4.0),
        (-80.0, 120.0, 14.0, 3.5), (40.0, 185.0, 16.0, 5.0), (-185.0, 60.0, 15.0, 4.0),
        (210.0, 40.0, 15.0, 4.2), (120.0, -210.0, 14.0, 3.8),
    ];
    for (hx, hz, r, h) in hills {
        // Smooth dome: a big sphere sunk so only the top `h` pokes above ground
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(r))),
            MeshMaterial3d(hill_mat.clone()),
            Transform::from_xyz(hx, h - r, hz),
        ));
        // Four invisible terraced walkable steps so the player can climb it
        for i in 1..=4u32 {
            let top = h * i as f32 / 4.0;
            // radius on the dome at this height
            let inner = top - h + r;
            let rho = (r * r - inner * inner).max(0.0).sqrt() * 0.82;
            commands.spawn((
                Transform::from_xyz(hx, top - 0.5, hz), GlobalTransform::default(),
                Walkable { half: Vec2::new(rho, rho), top },
            ));
        }
    }
}

// The stone summoning portal the player arrives through, near the map centre.
fn spawn_portal(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let c = Vec3::new(0.0, 0.0, 18.0);
    let stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.27, 0.32), perceptual_roughness: 0.95, ..default() });
    let runes = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.7, 1.0), emissive: LinearRgba::new(1.5, 3.0, 6.0, 1.0),
        unlit: true, alpha_mode: AlphaMode::Add, ..default() });
    let portal = materials.add(StandardMaterial {
        base_color: Color::srgba(0.4, 0.55, 1.0, 0.5),
        emissive: LinearRgba::new(1.2, 2.2, 5.0, 1.0), unlit: true,
        alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default() });

    // Circular dais
    commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 4.0, half_height: 0.15 })), MeshMaterial3d(stone.clone()),
        Transform::from_xyz(c.x, 0.15, c.z)));
    // Glowing rune ring on the dais
    commands.spawn((Mesh3d(meshes.add(Annulus::new(2.6, 3.2))), MeshMaterial3d(runes.clone()),
        Transform::from_xyz(c.x, 0.32, c.z).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))));

    // Two side pillars + a top lintel forming an archway behind the spawn (toward +Z)
    let arch_z = c.z + 2.6;
    for sx in [-2.4f32, 2.4] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.9, 7.0, 0.9))), MeshMaterial3d(stone.clone()),
            Transform::from_xyz(c.x + sx, 3.5, arch_z),
            Collider { half: Vec2::new(0.45, 0.45) }));
    }
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(6.4, 1.0, 1.0))), MeshMaterial3d(stone.clone()),
        Transform::from_xyz(c.x, 7.2, arch_z)));
    // The shimmering portal surface within the arch
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(4.2, 6.4, 0.12))), MeshMaterial3d(portal),
        Transform::from_xyz(c.x, 3.6, arch_z + 0.05)));
    // Cold portal glow
    commands.spawn((PointLight { color: Color::srgb(0.5, 0.7, 1.0), intensity: 900_000.0, range: 40.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(c.x, 4.0, arch_z)));

    let _ = runes;
}

// Three magic artifacts resting on stone pedestals in distant structures.
// Reaching one unlocks it (cycle with E, fire with Q).
fn spawn_artifacts(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let pedestal = materials.add(StandardMaterial {
        base_color: Color::srgb(0.26, 0.25, 0.30), perceptual_roughness: 0.95, ..default() });

    // (kind, world position of the relic, light colour)
    // TESTING: clustered right by the spawn portal so all three are grabbable immediately.
    let spots = [
        (ArtifactKind::Flame,       Vec3::new(-9.0, 1.4, 13.0), Color::srgb(1.0, 0.5, 0.1)),
        (ArtifactKind::Paladin,     Vec3::new(-3.0, 1.4, 10.0), Color::srgb(1.0, 0.85, 0.3)),
        (ArtifactKind::Trident,     Vec3::new( 3.0, 1.4, 10.0), Color::srgb(0.3, 0.7, 1.0)),
        (ArtifactKind::Telekinesis, Vec3::new( 9.0, 1.4, 13.0), Color::srgb(0.8, 0.9, 1.0)),
    ];

    for (kind, pos, lcol) in spots {
        // Stone pedestal beneath the relic
        commands.spawn((
            Mesh3d(meshes.add(Cylinder { radius: 0.7, half_height: (pos.y - 0.4).max(0.4) * 0.5 })),
            MeshMaterial3d(pedestal.clone()),
            Transform::from_xyz(pos.x, (pos.y - 0.4).max(0.4) * 0.5, pos.z),
            Collider { half: Vec2::new(0.7, 0.7) },
        ));

        // The floating, spinning relic (root carries ArtifactPickup)
        let root = commands.spawn((
            Transform::from_translation(pos), GlobalTransform::default(), Visibility::default(),
            ArtifactPickup { kind, base_y: pos.y },
        )).id();
        commands.spawn((PointLight { color: lcol, intensity: 600_000.0, range: 30.0, shadows_enabled: false, ..default() },
            Transform::from_xyz(0.0, 0.3, 0.0))).set_parent(root);

        match kind {
            ArtifactKind::Flame => {
                let m = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.45, 0.05),
                    emissive: LinearRgba::new(8.0, 2.6, 0.2, 1.0), unlit: true, ..default() });
                commands.spawn((Mesh3d(meshes.add(Sphere::new(0.28))), MeshMaterial3d(m.clone()), Transform::default())).set_parent(root);
                for k in 0..6u32 {
                    let a = k as f32 / 6.0 * std::f32::consts::TAU;
                    commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.08, height: 0.28 }.mesh().resolution(5))), MeshMaterial3d(m.clone()),
                        Transform::from_xyz(a.cos() * 0.34, 0.0, a.sin() * 0.34)
                            .with_rotation(Quat::from_rotation_z(0.0) * Quat::from_rotation_y(-a)))).set_parent(root);
                }
            }
            ArtifactKind::Paladin => {
                let m = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.84, 0.3),
                    emissive: LinearRgba::new(4.0, 2.8, 0.6, 1.0), metallic: 0.9, perceptual_roughness: 0.2, ..default() });
                let steel = materials.add(StandardMaterial { base_color: Color::srgb(0.62, 0.66, 0.74), metallic: 0.9, perceptual_roughness: 0.3, ..default() });
                // Kite shield: gold trim + steel face + pointed tip + cross
                commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.46, 0.58, 0.07))), MeshMaterial3d(m.clone()), Transform::from_xyz(0.0, 0.05, 0.0))).set_parent(root);
                commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.38, 0.50, 0.10))), MeshMaterial3d(steel.clone()), Transform::from_xyz(0.0, 0.06, 0.02))).set_parent(root);
                commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.26, height: 0.30 }.mesh().resolution(3))), MeshMaterial3d(m.clone()),
                    Transform::from_xyz(0.0, -0.34, 0.0).with_rotation(Quat::from_rotation_z(std::f32::consts::PI)))).set_parent(root);
                commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.07, 0.40, 0.12))), MeshMaterial3d(m.clone()), Transform::from_xyz(0.0, 0.08, 0.06))).set_parent(root);
                commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.28, 0.07, 0.12))), MeshMaterial3d(m.clone()), Transform::from_xyz(0.0, 0.14, 0.06))).set_parent(root);
            }
            ArtifactKind::Trident => {
                let m = materials.add(StandardMaterial { base_color: Color::srgb(0.3, 0.7, 1.0),
                    emissive: LinearRgba::new(0.8, 3.0, 5.0, 1.0), metallic: 0.6, perceptual_roughness: 0.25, ..default() });
                commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.06, 0.7, 0.06))), MeshMaterial3d(m.clone()), Transform::from_xyz(0.0, -0.1, 0.0))).set_parent(root);
                for px in [-0.14f32, 0.0, 0.14] {
                    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.045, 0.28, 0.045))), MeshMaterial3d(m.clone()),
                        Transform::from_xyz(px, 0.34, 0.0))).set_parent(root);
                }
                commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.09, height: 0.18 }.mesh().resolution(5))), MeshMaterial3d(m.clone()),
                    Transform::from_xyz(0.0, 0.52, 0.0))).set_parent(root);
            }
            ArtifactKind::Telekinesis => {
                let ball = materials.add(StandardMaterial { base_color: Color::srgba(0.95, 0.97, 1.0, 0.4),
                    emissive: LinearRgba::new(2.2, 2.4, 3.0, 1.0), unlit: true, alpha_mode: AlphaMode::Blend, ..default() });
                let ring = materials.add(StandardMaterial { base_color: Color::srgba(0.8, 0.9, 1.0, 0.8),
                    emissive: LinearRgba::new(1.6, 2.0, 3.0, 1.0), unlit: true, alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default() });
                commands.spawn((Mesh3d(meshes.add(Sphere::new(0.28))), MeshMaterial3d(ball), Transform::default())).set_parent(root);
                commands.spawn((Mesh3d(meshes.add(Annulus::new(0.42, 0.48))), MeshMaterial3d(ring.clone()), Transform::default())).set_parent(root);
                commands.spawn((Mesh3d(meshes.add(Annulus::new(0.54, 0.60))), MeshMaterial3d(ring),
                    Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2 * 0.6)))).set_parent(root);
            }
            ArtifactKind::Lightning | ArtifactKind::BlackHole => {}
        }
    }
}

// Float/spin world artifacts; unlock + auto-equip one when the player reaches it.
fn artifact_pickup_system(
    time: Res<Time>,
    mut commands: Commands,
    mut arts: ResMut<Artifacts>,
    player_q: Query<&Transform, With<Player>>,
    mut pickups: Query<(Entity, &GlobalTransform, &mut Transform, &ArtifactPickup), Without<Player>>,
) {
    let pp = player_q.single().translation;
    let t = time.elapsed_secs();
    for (e, g, mut tr, ap) in pickups.iter_mut() {
        tr.rotation = Quat::from_rotation_y(t * 1.2);
        tr.translation.y = ap.base_y + (t * 2.0).sin() * 0.18;
        if g.translation().distance(pp) < 3.0 {
            match ap.kind {
                ArtifactKind::Flame   => arts.has_flame = true,
                ArtifactKind::Paladin => arts.has_paladin = true,
                ArtifactKind::Trident => arts.has_trident = true,
                ArtifactKind::Telekinesis => arts.has_telekinesis = true,
                ArtifactKind::Lightning | ArtifactKind::BlackHole => {}
            }
            arts.selected = ap.kind;
            commands.entity(e).despawn_recursive();
        }
    }
}

// ── Medusa: the castle boss — a coiled stone gorgon ──
fn spawn_medusa(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Pale, cold-skinned gorgon with a deep-blue scaled serpent body (see ref art).
    let skin = materials.add(StandardMaterial { base_color: Color::srgb(0.80, 0.84, 0.88), perceptual_roughness: 0.5, ..default() });
    let dark = materials.add(StandardMaterial { base_color: Color::srgb(0.06, 0.08, 0.14), perceptual_roughness: 0.8, ..default() });  // tattoos / sockets
    let scl  = materials.add(StandardMaterial { base_color: Color::srgb(0.14, 0.22, 0.46), perceptual_roughness: 0.45, metallic: 0.35, ..default() });
    let scl2 = materials.add(StandardMaterial { base_color: Color::srgb(0.08, 0.13, 0.30), perceptual_roughness: 0.5, metallic: 0.3, ..default() });
    let belly= materials.add(StandardMaterial { base_color: Color::srgb(0.62, 0.70, 0.80), perceptual_roughness: 0.7, ..default() });
    let hair = materials.add(StandardMaterial { base_color: Color::srgb(0.10, 0.14, 0.32), perceptual_roughness: 0.5, ..default() });
    let hair2= materials.add(StandardMaterial { base_color: Color::srgb(0.16, 0.22, 0.44), perceptual_roughness: 0.5, ..default() });
    // Eyes: yellow glow (medusa_ai turns them red while she charges)
    let eye  = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.95, 0.2), emissive: LinearRgba::new(6.0, 5.0, 0.4, 1.0), unlit: true, ..default() });
    commands.insert_resource(MedusaEye { mat: eye.clone() });

    // FRONT of Medusa is local -Z (medusa_ai look_at points -Z at the player).
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, -100.0).with_scale(Vec3::splat(1.4)),
        GlobalTransform::default(), Visibility::default(),
        Medusa { health: 24.0, max_health: 24.0, state: MedusaState::Idle, damage_flash: 0.0,
                 timer: 0.0, enraged: false, attack_timer: 0.0, gaze_timer: 4.0,
                 dash_cd: 4.0, windup: 0.0, charge_t: 0.0, charge_dir: Vec3::ZERO },
        Shock { timer: 0.0 },
    )).with_children(|p| {
        // Slim coiled serpent tail (a shrinking spiral) — deep-blue scales, two tones
        for i in 0..16u32 {
            let a = i as f32 * 0.62;
            let rad = 2.2 - i as f32 * 0.12;
            let y = 0.45 + i as f32 * 0.1;
            let m = if i % 2 == 0 { scl.clone() } else { scl2.clone() };
            p.spawn((Mesh3d(meshes.add(Sphere::new(0.7))), MeshMaterial3d(m),
                Transform::from_xyz(a.cos() * rad, y, a.sin() * rad).with_scale(Vec3::new(1.0, 0.7, 1.0))));
        }
        // pale belly scutes running up the FRONT (-Z) of the coil
        for i in 0..6u32 {
            p.spawn((Mesh3d(meshes.add(Cuboid::new(0.55, 0.16, 0.42))), MeshMaterial3d(belly.clone()),
                Transform::from_xyz(0.0, 0.9 + i as f32 * 0.32, -(1.7 - i as f32 * 0.2))));
        }
        // tail tip flicking out behind
        p.spawn((Mesh3d(meshes.add(Cone { radius: 0.28, height: 2.2 }.mesh().resolution(6))), MeshMaterial3d(scl.clone()),
            Transform::from_xyz(2.2, 0.45, 1.3).with_rotation(Quat::from_rotation_z(1.2))));
        // ── Slim, alluring humanoid upper body ──
        p.spawn((Mesh3d(meshes.add(Cone { radius: 0.6, height: 1.6 }.mesh().resolution(16))), MeshMaterial3d(skin.clone()),
            Transform::from_xyz(0.0, 2.35, 0.0).with_rotation(Quat::from_rotation_x(std::f32::consts::PI)))); // hips→waist taper
        p.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.26, half_height: 0.18 })), MeshMaterial3d(scl.clone()),
            Transform::from_xyz(0.0, 2.95, 0.0)));                                  // cinched waist
        p.spawn((Mesh3d(meshes.add(Cone { radius: 0.5, height: 1.0 }.mesh().resolution(16))), MeshMaterial3d(skin.clone()),
            Transform::from_xyz(0.0, 3.55, 0.0)));                                  // bust/shoulders
        // ── Tribal tattoo bands across the torso & shoulders (front, -Z) ──
        for (ty, tw, tilt) in [(3.7f32, 0.5f32, 0.0f32), (3.35, 0.42, 0.3), (3.05, 0.3, -0.25)] {
            p.spawn((Mesh3d(meshes.add(Cuboid::new(tw, 0.05, 0.04))), MeshMaterial3d(dark.clone()),
                Transform::from_xyz(0.0, ty, -0.5).with_rotation(Quat::from_rotation_z(tilt))));
        }
        // slender arms reaching slightly forward (-Z), with hands
        for s in [-1.0f32, 1.0] {
            // tattoo coil down each upper arm
            p.spawn((Mesh3d(meshes.add(Cuboid::new(0.04, 0.5, 0.04))), MeshMaterial3d(dark.clone()),
                Transform::from_xyz(s * 0.52, 3.5, -0.12).with_rotation(Quat::from_rotation_z(s * 0.5) * Quat::from_rotation_x(-0.4))));
            p.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.11, half_height: 0.55 })), MeshMaterial3d(skin.clone()),
                Transform::from_xyz(s * 0.52, 3.5, -0.05).with_rotation(Quat::from_rotation_z(s * 0.5) * Quat::from_rotation_x(-0.4))));   // upper arm
            p.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.09, half_height: 0.5 })), MeshMaterial3d(skin.clone()),
                Transform::from_xyz(s * 0.78, 3.0, -0.5).with_rotation(Quat::from_rotation_x(-0.9))));                                     // forearm
            p.spawn((Mesh3d(meshes.add(Sphere::new(0.12))), MeshMaterial3d(skin.clone()),
                Transform::from_xyz(s * 0.86, 2.7, -0.95)));                        // hand
        }
        // Neck + head
        p.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.14, half_height: 0.22 })), MeshMaterial3d(skin.clone()),
            Transform::from_xyz(0.0, 4.0, 0.0)));
        p.spawn((Mesh3d(meshes.add(Sphere::new(0.38))), MeshMaterial3d(skin.clone()),
            Transform::from_xyz(0.0, 4.4, 0.0).with_scale(Vec3::new(0.92, 1.05, 1.0))));
        // ── Face (on the FRONT, -Z) ──
        for s in [-1.0f32, 1.0] {
            p.spawn((Mesh3d(meshes.add(Sphere::new(0.12))), MeshMaterial3d(dark.clone()),
                Transform::from_xyz(s * 0.15, 4.46, -0.30)));                       // socket
            p.spawn((Mesh3d(meshes.add(Sphere::new(0.085))), MeshMaterial3d(eye.clone()),
                Transform::from_xyz(s * 0.15, 4.46, -0.36)));                       // glowing eye
            p.spawn((Mesh3d(meshes.add(Cuboid::new(0.17, 0.045, 0.05))), MeshMaterial3d(dark.clone()),
                Transform::from_xyz(s * 0.16, 4.60, -0.32).with_rotation(Quat::from_rotation_z(s * 0.4))));  // brow
        }
        // nose
        p.spawn((Mesh3d(meshes.add(Cone { radius: 0.06, height: 0.16 }.mesh().resolution(4))), MeshMaterial3d(skin.clone()),
            Transform::from_xyz(0.0, 4.36, -0.38).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))));
        // lips + fangs
        p.spawn((Mesh3d(meshes.add(Cuboid::new(0.2, 0.06, 0.05))), MeshMaterial3d(dark.clone()),
            Transform::from_xyz(0.0, 4.18, -0.34)));
        for s in [-1.0f32, 1.0] {
            p.spawn((Mesh3d(meshes.add(Cone { radius: 0.028, height: 0.09 }.mesh().resolution(4))), MeshMaterial3d(belly.clone()),
                Transform::from_xyz(s * 0.06, 4.13, -0.36).with_rotation(Quat::from_rotation_x(std::f32::consts::PI))));
        }
        // ── A full crown of writhing snakes — each a 3-segment body + a little head ──
        for i in 0..22u32 {
            let a = i as f32 / 22.0 * std::f32::consts::TAU;
            let ring = 0.30 + (i % 3) as f32 * 0.08;
            let tilt = 0.5 + (i % 4) as f32 * 0.18;
            let hm = if i % 2 == 0 { hair.clone() } else { hair2.clone() };
            let cx = a.cos() * ring; let cz = a.sin() * ring;
            // outward/upward base direction for this snake
            let out = Vec3::new(a.cos(), 0.9, a.sin()).normalize();
            for seg in 0..3u32 {
                let t = seg as f32;
                // bend the snake as it rises (writhe)
                let bend = (a * 1.7 + t).sin() * 0.18;
                let pos = Vec3::new(cx, 4.62, cz) + out * (0.18 + t * 0.26) + Vec3::new(bend, 0.0, -bend);
                let r = 0.085 - seg as f32 * 0.018;
                p.spawn((Mesh3d(meshes.add(Sphere::new(r.max(0.04)))), MeshMaterial3d(hm.clone()),
                    Transform::from_translation(pos).with_scale(Vec3::new(1.0, 1.4, 1.0))));
            }
            // little snake head at the tip, facing outward
            let head = Vec3::new(cx, 4.62, cz) + out * 0.95;
            p.spawn((Mesh3d(meshes.add(Sphere::new(0.07))), MeshMaterial3d(hm.clone()),
                Transform::from_translation(head).with_scale(Vec3::new(1.1, 0.8, 1.4))
                    .with_rotation(Quat::from_rotation_z(-a.cos() * tilt) * Quat::from_rotation_x(a.sin() * tilt))));
            // tiny glowing snake eyes
            p.spawn((Mesh3d(meshes.add(Sphere::new(0.02))), MeshMaterial3d(eye.clone()),
                Transform::from_translation(head + out * 0.05 + Vec3::Y * 0.02)));
        }
    });
}

// Medusa AI: slither + melee + stone bolts; enrage at 50% → phase 2 with a
// petrifying stone-gaze shockwave. On death, start the dragon's arrival countdown.
fn medusa_ai(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut medusa_q: Query<(Entity, &mut Transform, &mut Medusa)>,
    player_q: Query<&Transform, (With<Player>, Without<Medusa>)>,
    mut player_vel: Query<&mut PlayerVelocity, With<Player>>,
    mut health: ResMut<PlayerHealth>,
    mut arrival: ResMut<DragonArrival>,
    mut kills: ResMut<KillStats>,
    eye_assets: Option<Res<MedusaEye>>,
) {
    let Ok(pt) = player_q.get_single() else { return; };
    let pp = pt.translation;
    let dt = time.delta_secs();
    for (e, mut t, mut m) in medusa_q.iter_mut() {
        if m.state == MedusaState::Dead { continue; }
        let flat = Vec3::new(pp.x - t.translation.x, 0.0, pp.z - t.translation.z);
        let dist = flat.length();
        if m.state == MedusaState::Idle {
            if dist < 80.0 { m.state = MedusaState::Chase; } else { continue; }
        }
        // Enrage at half health → brief rear-up, then phase 2 (Gaze)
        if !m.enraged && m.health <= m.max_health * 0.5 {
            m.enraged = true;
            m.state = MedusaState::Enrage;
            m.timer = 1.8;
            let aura = materials.add(StandardMaterial { base_color: Color::srgba(0.4, 1.0, 0.3, 0.16),
                emissive: LinearRgba::new(0.4, 1.4, 0.3, 1.0), unlit: true, alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default() });
            commands.entity(e).with_children(|c| {
                c.spawn((Mesh3d(meshes.add(Sphere::new(4.0))), MeshMaterial3d(aura), Transform::from_xyz(0.0, 2.5, 0.0)));
            });
        }
        // Face the player
        let ty = t.translation.y;
        t.look_at(Vec3::new(pp.x, ty + 1.0, pp.z), Vec3::Y);

        let bolt_mat = materials.add(StandardMaterial { base_color: Color::srgb(0.45, 0.95, 0.2),
            emissive: LinearRgba::new(1.2, 5.0, 0.4, 1.0), unlit: true, ..default() });
        let head = t.translation + Vec3::Y * 6.0;

        match m.state {
            MedusaState::Chase | MedusaState::Gaze => {
                let charging = m.windup > 0.0 || m.charge_t > 0.0;
                m.dash_cd -= dt;
                if m.windup > 0.0 {
                    // Stop, rear back (arms coiled) for ~1s, eyes blazing — telegraph
                    m.windup -= dt;
                    if m.windup <= 0.0 {
                        m.charge_t = 0.7;
                        m.charge_dir = (flat / dist.max(0.01)).normalize_or_zero();
                    }
                } else if m.charge_t > 0.0 {
                    // RAM forward fast (dodgeable — fixed direction)
                    m.charge_t -= dt;
                    t.translation += m.charge_dir * 18.0 * dt;
                    t.translation.y = 0.0;
                    if dist < 3.2 {
                        health.take(22.0, 0.9);
                        if let Ok(mut v) = player_vel.get_single_mut() { v.knockback = m.charge_dir * 13.0; }
                        m.charge_t = 0.0;
                    }
                    if m.charge_t <= 0.0 {
                        m.dash_cd = 3.0 + (time.elapsed_secs() * 7.3).fract() * 3.0;
                    }
                } else {
                    // Normal slither toward the player
                    let speed = if m.enraged { 6.0 } else { 4.2 };
                    if dist > 4.5 { t.translation += (flat / dist) * speed * dt; }
                    t.translation.y = 0.0;
                    // Begin a dash windup every few seconds
                    if m.dash_cd <= 0.0 && dist > 4.0 { m.windup = 1.0; }
                    // Melee swipe
                    m.attack_timer -= dt;
                    if dist < 5.0 && m.attack_timer <= 0.0 {
                        m.attack_timer = 1.3;
                        health.take(if m.enraged { 24.0 } else { 16.0 }, 0.9);
                        if let Ok(mut v) = player_vel.get_single_mut() {
                            v.knockback = (flat / dist.max(0.01)) * 7.0;
                        }
                    }
                    // Spit a glob of corrosive acid in a lobbed arc — it pools on the ground
                    m.timer -= dt;
                    if m.timer <= 0.0 {
                        m.timer = if m.enraged { 1.6 } else { 2.4 };
                        let flat_dir = Vec3::new(pp.x - head.x, 0.0, pp.z - head.z);
                        let fd = flat_dir.length().max(0.01);
                        // ballistic lob: forward speed scaled to range + a fixed upward kick
                        let dir = (flat_dir / fd) * (fd * 0.9).clamp(10.0, 20.0) + Vec3::Y * 7.0;
                        commands.spawn((
                            Mesh3d(meshes.add(Sphere::new(0.32))), MeshMaterial3d(bolt_mat.clone()),
                            Transform::from_translation(head).with_scale(Vec3::new(1.0, 1.3, 1.0)),
                            MedusaBolt { vel: dir, life: 5.0 },
                            PointLight { color: Color::srgb(0.5, 1.0, 0.2), intensity: 60_000.0, range: 10.0, shadows_enabled: false, ..default() },
                        ));
                    }
                    // Phase 2: a petrifying stone-gaze CONE projected from her face
                    if m.state == MedusaState::Gaze {
                        m.gaze_timer -= dt;
                        if m.gaze_timer <= 0.0 {
                            m.gaze_timer = 6.0;
                            let gaze_dir = Vec3::new(flat.x, 0.0, flat.z).normalize_or_zero();
                            let face = t.translation + Vec3::Y * 5.0;
                            // Very translucent, white-glowing gaze cone
                            let wave = materials.add(StandardMaterial { base_color: Color::srgba(0.92, 0.96, 0.85, 0.16),
                                emissive: LinearRgba::new(3.2, 3.4, 2.4, 1.0), unlit: true, alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default() });
                            commands.spawn((
                                Mesh3d(meshes.add(Cone { radius: 4.0, height: 26.0 }.mesh().resolution(14))), MeshMaterial3d(wave),
                                Transform::from_translation(face + gaze_dir * 13.0)
                                    .with_rotation(Quat::from_rotation_arc(Vec3::Y, -gaze_dir)),
                                StoneWave { radius: 0.0, hit: false, dir: gaze_dir, origin: face },
                                PointLight { color: Color::srgb(0.9, 1.0, 0.85), intensity: 400_000.0, range: 36.0, shadows_enabled: false, ..default() },
                            ));
                        }
                    }
                }
                // Eyes blaze red while charging, calm yellow otherwise
                if let Some(ea) = &eye_assets {
                    if let Some(em) = materials.get_mut(&ea.mat) {
                        if charging {
                            em.emissive = LinearRgba::new(11.0, 0.4, 0.2, 1.0);
                            em.base_color = Color::srgb(1.0, 0.12, 0.08);
                        } else {
                            em.emissive = LinearRgba::new(6.0, 5.0, 0.4, 1.0);
                            em.base_color = Color::srgb(1.0, 0.95, 0.2);
                        }
                    }
                }
            }
            MedusaState::Enrage => {
                m.timer -= dt;
                if m.timer <= 0.0 { m.state = MedusaState::Gaze; m.gaze_timer = 2.5; }
            }
            _ => {}
        }

        if m.health <= 0.0 {
            m.state = MedusaState::Dead;
            // Stone shatter burst
            let shard = materials.add(StandardMaterial { base_color: Color::srgb(0.45, 0.5, 0.42), perceptual_roughness: 1.0, ..default() });
            let cube = meshes.add(Cuboid::new(0.4, 0.4, 0.4));
            for i in 0..40u32 {
                let a = i as f32 * 2.39996;
                let dir = Vec3::new(a.cos(), 0.6 + (i % 4) as f32 * 0.3, a.sin()).normalize();
                commands.spawn((Mesh3d(cube.clone()), MeshMaterial3d(shard.clone()),
                    Transform::from_translation(t.translation + Vec3::Y * 3.0),
                    Debris { vel: dir * (6.0 + (i % 5) as f32 * 2.0), life: 1.6 }));
            }
            // Begin the dragon's arrival countdown
            kills.medusa = 1;
            arrival.counting = true;
            arrival.countdown = 60.0;
            commands.entity(e).despawn_recursive();
        }
    }
}

// Medusa stone bolts: fly toward the strike point, damage the player on contact.
fn move_medusa_bolts(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut bolts: Query<(Entity, &mut Transform, &mut MedusaBolt), Without<Player>>,
    player_q: Query<&Transform, With<Player>>,
    mut health: ResMut<PlayerHealth>,
) {
    let dt = time.delta_secs();
    let pp = player_q.single().translation + Vec3::Y * 1.0;
    for (e, mut t, mut b) in bolts.iter_mut() {
        b.vel.y -= 22.0 * dt;                       // gravity → arcing lob
        t.translation += b.vel * dt;
        b.life -= dt;
        let splash = t.translation.y <= 0.1 || b.life <= 0.0;
        let direct = pp.distance(t.translation) < 1.4;
        if direct { health.take(12.0, 0.6); }
        if splash || direct {
            // Leave a lingering, corrosive acid pool where it lands
            let acid = materials.add(StandardMaterial {
                base_color: Color::srgba(0.45, 0.95, 0.2, 0.85), emissive: LinearRgba::new(0.8, 3.0, 0.3, 1.0),
                unlit: true, alpha_mode: AlphaMode::Add, ..default() });
            commands.spawn((
                Mesh3d(meshes.add(Cylinder { radius: 2.0, half_height: 0.08 })), MeshMaterial3d(acid),
                Transform::from_xyz(t.translation.x, 0.1, t.translation.z),
                FirePatch { life: 4.5 },
                PointLight { color: Color::srgb(0.5, 1.0, 0.2), intensity: 80_000.0, range: 9.0, shadows_enabled: false, ..default() },
            ));
            commands.entity(e).despawn_recursive();
        }
    }
}

// Medusa's stone-gaze CONE: if the player stands within the forward cone from her
// face, they're petrified for 3s. The cone lingers briefly, then fades.
fn update_stone_waves(
    time: Res<Time>,
    mut commands: Commands,
    mut waves: Query<(Entity, &mut Transform, &mut StoneWave), Without<Player>>,
    player_q: Query<&Transform, With<Player>>,
    mut petrify: ResMut<Petrify>,
) {
    let pt = player_q.single();
    for (e, mut t, mut w) in waves.iter_mut() {
        w.radius += time.delta_secs();              // doubles as the cone's lifetime
        // a subtle pulse while it lingers
        let s = 1.0 + (w.radius * 8.0).sin() * 0.05;
        t.scale = Vec3::splat(s);
        if !w.hit {
            let to = pt.translation + Vec3::Y * 1.0 - w.origin;
            let d = to.length();
            if d > 0.5 && d < 28.0 && w.dir.dot(to / d) > 0.82 {
                w.hit = true;
                if petrify.timer <= 0.0 { petrify.timer = 3.0; }
            }
        }
        if w.radius > 0.8 { commands.entity(e).despawn_recursive(); }
    }
}

// Tick the petrify timer and show/hide a grey "turned to stone" overlay.
fn petrify_system(
    time: Res<Time>,
    mut petrify: ResMut<Petrify>,
    mut commands: Commands,
    overlay_q: Query<Entity, With<PetrifyOverlay>>,
) {
    if petrify.timer > 0.0 {
        petrify.timer -= time.delta_secs();
        if overlay_q.is_empty() {
            commands.spawn((
                Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                BackgroundColor(Color::srgba(0.55, 0.55, 0.58, 0.42)),
                PetrifyOverlay,
            ));
        }
    } else if !overlay_q.is_empty() {
        for e in overlay_q.iter() { commands.entity(e).despawn_recursive(); }
    }
}

// Count down to the dragon's arrival after Medusa dies; show a banner; then spawn.
fn dragon_arrival_timer(
    time: Res<Time>,
    mut arrival: ResMut<DragonArrival>,
    player_q: Query<&Transform, With<Player>>,
    mut commands: Commands,
    mut text_q: Query<(Entity, &mut Text), With<CountdownText>>,
) {
    if !arrival.counting {
        for (e, _) in text_q.iter() { commands.entity(e).despawn_recursive(); }
        return;
    }
    arrival.countdown -= time.delta_secs();
    let secs = arrival.countdown.max(0.0).ceil() as i32;
    if let Ok((_, mut txt)) = text_q.get_single_mut() {
        *txt = Text::new(format!("The Great Dragon descends in {}...", secs));
    } else {
        commands.spawn((
            Node { position_type: PositionType::Absolute, top: Val::Px(70.0), left: Val::Percent(50.0),
                   margin: UiRect::left(Val::Px(-260.0)), width: Val::Px(520.0),
                   justify_content: JustifyContent::Center, ..default() },
            Text::new(format!("The Great Dragon descends in {}...", secs)),
            TextFont { font_size: 28.0, ..default() },
            TextColor(Color::srgb(1.0, 0.5, 0.3)),
            CountdownText,
        ));
    }
    if arrival.countdown <= 0.0 {
        arrival.counting = false;
        arrival.spawn_now = true;
        let pt = if let Ok(p) = player_q.get_single() { *p } else { Transform::IDENTITY };
        let fwd = *pt.forward();
        let facing = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero();
        // Impact is LOCKED to where the player stands now (so they can run away).
        // The dragon spawns ~1 km out in the facing direction and ~700 m up, then
        // descends the fixed hypotenuse to that point — never re-aiming.
        let impact = Vec3::new(pt.translation.x, 0.0, pt.translation.z);
        arrival.target = impact;
        arrival.pos = impact + facing * 1000.0 + Vec3::Y * 700.0;
        for (e, _) in text_q.iter() { commands.entity(e).despawn_recursive(); }
    }
}

// The dragon's meteor impact shockwave: expands fast, knocking back & lightly
// damaging everything it sweeps (monsters take little; the player is hurled).
fn update_meteor_shock(
    time: Res<Time>,
    mut commands: Commands,
    mut shocks: Query<(Entity, &mut Transform, &mut MeteorShock), Without<Player>>,
    mut skel_q: Query<(&GlobalTransform, &mut Skeleton), Without<MeteorShock>>,
    mut enemy_q: Query<(&GlobalTransform, &mut Enemy), Without<MeteorShock>>,
    player_q: Query<&Transform, With<Player>>,
    mut player_vel: Query<&mut PlayerVelocity, With<Player>>,
    mut health: ResMut<PlayerHealth>,
) {
    for (e, mut t, mut s) in shocks.iter_mut() {
        s.radius += time.delta_secs() * 90.0;        // shell expands fast
        t.scale = Vec3::splat(s.radius.max(0.1));
        if !s.hit {
            s.hit = true; // one big sweep, limited to its effect radius `max`
            for (g, mut sk) in skel_q.iter_mut() {
                if sk.state == SkeletonState::Dead { continue; }
                if g.translation().distance(s.origin) < s.max {
                    sk.health -= 2.0;                // non-lethal-ish chip
                    sk.knockback_vel = (g.translation() - s.origin).normalize_or_zero() * 18.0;
                }
            }
            for (g, mut en) in enemy_q.iter_mut() {
                if g.translation().distance(s.origin) < s.max {
                    en.health -= 2.0;
                    en.knockback_vel = (g.translation() - s.origin).normalize_or_zero() * 18.0;
                }
            }
            if let Ok(pp) = player_q.get_single() {
                if pp.translation.distance(s.origin) < s.max {
                    let dir = (pp.translation - s.origin).normalize_or_zero();
                    if let Ok(mut v) = player_vel.get_single_mut() {
                        v.knockback = Vec3::new(dir.x, 0.0, dir.z) * 16.0;
                        v.vertical = 6.0;
                    }
                    health.take(15.0, 0.6);
                }
            }
        }
        if s.radius > s.max.min(70.0) { commands.entity(e).despawn_recursive(); }
    }
}

// Spawn/update the Medusa boss health bar while she's engaged.
fn update_medusa_bar(
    medusa_q: Query<&Medusa>,
    mut root_q: Query<&mut Visibility, With<MedusaBarRoot>>,
    mut fill_q: Query<&mut Node, With<MedusaBarFill>>,
) {
    let active = medusa_q.iter().find(|m| m.state != MedusaState::Idle && m.state != MedusaState::Dead);
    if let Ok(mut vis) = root_q.get_single_mut() {
        *vis = if active.is_some() { Visibility::Inherited } else { Visibility::Hidden };
    }
    if let (Some(m), Ok(mut n)) = (active, fill_q.get_single_mut()) {
        n.width = Val::Percent((m.health / m.max_health * 100.0).clamp(0.0, 100.0));
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  SHADOW SUCCUBUS — a rare roaming demon that swoops and rakes with claws
// ════════════════════════════════════════════════════════════════════════════
fn build_succubus(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>, pos: Vec3) {
    // Evelynn-inspired shadow demoness (see ref art): pale lavender skin, dark
    // bodysuit with magenta filigree, white hair crowned with pink flame, and great
    // black wings tipped in glowing magenta blades.
    let suit  = materials.add(StandardMaterial { base_color: Color::srgb(0.08, 0.03, 0.12), perceptual_roughness: 0.5, ..default() });
    let skin  = materials.add(StandardMaterial { base_color: Color::srgb(0.74, 0.62, 0.74), perceptual_roughness: 0.45, ..default() });
    let hair  = materials.add(StandardMaterial { base_color: Color::srgb(0.92, 0.90, 0.96), perceptual_roughness: 0.4, ..default() });
    let wing  = materials.add(StandardMaterial { base_color: Color::srgba(0.05, 0.01, 0.08, 0.92),
        perceptual_roughness: 0.7, cull_mode: None, double_sided: true, ..default() });
    let magenta = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.1, 0.85),
        emissive: LinearRgba::new(8.0, 0.4, 6.0, 1.0), unlit: true, ..default() });
    let flame = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.3, 0.8),
        emissive: LinearRgba::new(9.0, 1.0, 6.0, 1.0), unlit: true, ..default() });
    let eye   = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.85, 0.2),
        emissive: LinearRgba::new(8.0, 5.5, 0.4, 1.0), unlit: true, ..default() });
    let claw  = materials.add(StandardMaterial { base_color: Color::srgb(0.9, 0.85, 0.9), metallic: 0.6, perceptual_roughness: 0.25, ..default() });

    commands.spawn((
        Transform::from_translation(pos), GlobalTransform::default(), Visibility::default(),
        Enemy { health: 9.0, speed: 7.0, flying: true, base_y: 3.2, attack_timer: 1.2, knockback_vel: Vec3::ZERO,
                bob_phase: 0.0, anim_phase: 0.0, attack_anim: 0.0, moving: false, damage_flash: 0.0 },
        Succubus { attack: 1.2, swoop: 0.0 },
        Shock { timer: 0.0 },
    )).with_children(|p| {
        // ── Slim hourglass body: hips → cinched waist → bust (front is -Z) ──
        p.spawn((Mesh3d(meshes.add(Cone { radius: 0.4, height: 0.9 }.mesh().resolution(16))), MeshMaterial3d(suit.clone()),
            Transform::from_xyz(0.0, 0.05, 0.0).with_rotation(Quat::from_rotation_x(std::f32::consts::PI))));   // hips/skirt
        p.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.16, half_height: 0.16 })), MeshMaterial3d(suit.clone()),
            Transform::from_xyz(0.0, 0.5, 0.0)));                                  // waist
        p.spawn((Mesh3d(meshes.add(Sphere::new(0.3))), MeshMaterial3d(suit.clone()),
            Transform::from_xyz(0.0, 0.78, 0.0).with_scale(Vec3::new(1.0, 0.85, 0.8))));   // bust/chest
        // magenta filigree accents glowing on the bodice
        for s in [-1.0f32, 1.0] {
            p.spawn((Mesh3d(meshes.add(Cuboid::new(0.04, 0.5, 0.04))), MeshMaterial3d(magenta.clone()),
                Transform::from_xyz(s * 0.12, 0.55, -0.22).with_rotation(Quat::from_rotation_z(s * 0.3))));
        }
        // shoulders + slender pale neck
        p.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.06, half_height: 0.1 })), MeshMaterial3d(skin.clone()),
            Transform::from_xyz(0.0, 1.05, 0.0)));
        // ── Head: pale face, glowing yellow eyes, pink lips ──
        p.spawn((Mesh3d(meshes.add(Sphere::new(0.24))), MeshMaterial3d(skin.clone()), Transform::from_xyz(0.0, 1.28, 0.0)));
        for s in [-1.0f32, 1.0] {
            p.spawn((Mesh3d(meshes.add(Sphere::new(0.05))), MeshMaterial3d(eye.clone()),
                Transform::from_xyz(s * 0.1, 1.3, -0.2).with_scale(Vec3::new(1.3, 0.7, 0.6))));   // sharp yellow eye
        }
        p.spawn((Mesh3d(meshes.add(Cuboid::new(0.1, 0.03, 0.04))), MeshMaterial3d(flame.clone()),
            Transform::from_xyz(0.0, 1.18, -0.22)));                                // pink lips
        // ── White hair framing the face + crown of PINK FLAME ──
        p.spawn((Mesh3d(meshes.add(Sphere::new(0.27))), MeshMaterial3d(hair.clone()),
            Transform::from_xyz(0.0, 1.34, 0.06).with_scale(Vec3::new(1.05, 1.0, 1.0))));
        for s in [-1.0f32, 1.0] {
            p.spawn((Mesh3d(meshes.add(Cone { radius: 0.08, height: 0.7 }.mesh().resolution(5))), MeshMaterial3d(hair.clone()),
                Transform::from_xyz(s * 0.22, 1.05, 0.06).with_rotation(Quat::from_rotation_x(std::f32::consts::PI) * Quat::from_rotation_z(s * 0.15))));  // side locks
        }
        for k in 0..4u32 {
            let f = k as f32 / 3.0 - 0.5;
            p.spawn((Mesh3d(meshes.add(Cone { radius: 0.07, height: 0.45 + (0.25 - f.abs() * 0.3) }.mesh().resolution(4))), MeshMaterial3d(flame.clone()),
                Transform::from_xyz(f * 0.26, 1.6, 0.0).with_rotation(Quat::from_rotation_z(-f * 0.5))));   // pink flame crown
        }
        p.spawn((PointLight { color: Color::srgb(1.0, 0.2, 0.7), intensity: 120_000.0, range: 9.0, shadows_enabled: false, ..default() },
            Transform::from_xyz(0.0, 1.7, 0.0)));
        // ── Long gloved arms ending in magenta-lit claws (reach forward, -Z) ──
        for s in [-1.0f32, 1.0] {
            p.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.06, half_height: 0.5 })), MeshMaterial3d(suit.clone()),
                Transform::from_xyz(s * 0.36, 0.62, -0.22).with_rotation(Quat::from_rotation_x(-0.8))));
            for f in -1..2 {
                p.spawn((Mesh3d(meshes.add(Cone { radius: 0.025, height: 0.3 }.mesh().resolution(4))), MeshMaterial3d(claw.clone()),
                    Transform::from_xyz(s * 0.42 + f as f32 * 0.05, 0.22, -0.66).with_rotation(Quat::from_rotation_x(-1.4))));
            }
        }
        // ── Great wings: dark membrane with glowing magenta bladed feathers ──
        for s in [-1.0f32, 1.0] {
            p.spawn((Transform::from_xyz(s * 0.22, 0.85, 0.15), GlobalTransform::default(), Visibility::default(),
                SuccubusWing { side: s }))
            .with_children(|wc| {
                // upper wing bone
                wc.spawn((Mesh3d(meshes.add(Cuboid::new(1.7, 0.06, 0.1))), MeshMaterial3d(suit.clone()),
                    Transform::from_xyz(s * 0.85, 0.0, -0.1)));
                // dark membrane
                wc.spawn((Mesh3d(meshes.add(Cuboid::new(1.6, 0.04, 1.2))), MeshMaterial3d(wing.clone()),
                    Transform::from_xyz(s * 0.85, -0.05, 0.45)));
                // three long magenta blade-feathers sweeping off the wing edge
                for (bi, &(bx, bz, blen, brot)) in [(0.55f32, 0.9f32, 1.4f32, 0.5f32), (0.95, 0.5, 1.7, 0.2), (1.25, 0.0, 1.3, -0.2)].iter().enumerate() {
                    let _ = bi;
                    wc.spawn((Mesh3d(meshes.add(Cone { radius: 0.07, height: blen }.mesh().resolution(4))), MeshMaterial3d(magenta.clone()),
                        Transform::from_xyz(s * bx, 0.0, bz)
                            .with_rotation(Quat::from_rotation_z(s * brot) * Quat::from_rotation_x(-1.2))));
                }
            });
        }
        // sinuous whip tail with a magenta barb
        p.spawn((Mesh3d(meshes.add(Cone { radius: 0.06, height: 1.3 }.mesh().resolution(5))), MeshMaterial3d(suit.clone()),
            Transform::from_xyz(0.0, -0.2, 0.55).with_rotation(Quat::from_rotation_x(1.2))));
        p.spawn((Mesh3d(meshes.add(Cone { radius: 0.07, height: 0.35 }.mesh().resolution(4))), MeshMaterial3d(magenta.clone()),
            Transform::from_xyz(0.0, -0.5, 1.15).with_rotation(Quat::from_rotation_x(1.6))));
    });
}

// Rarely conjure a succubus far from the player.
fn succubus_spawn(
    time: Res<Time>,
    mut sp: ResMut<SuccubusSpawn>,
    realm: Res<Realm>,
    fight: Res<SauronFight>,
    areas: Res<Areas>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_q: Query<&Transform, With<Player>>,
    succ_q: Query<(), With<Succubus>>,
) {
    if realm.in_sky || fight.active || areas.in_shadow || areas.in_hut { return; }
    sp.timer -= time.delta_secs();
    if sp.timer > 0.0 { return; }
    sp.timer = 30.0 + (time.elapsed_secs() * 3.7).fract() * 30.0;  // 30–60s
    if succ_q.iter().count() >= 2 { return; }
    // rare: only ~40% of the time
    if (time.elapsed_secs() * 1.3).fract() > 0.4 { return; }
    let pp = player_q.single().translation;
    let a = (time.elapsed_secs() * 5.1).fract() * std::f32::consts::TAU;
    let pos = Vec3::new(pp.x + a.cos() * 55.0, 3.2, pp.z + a.sin() * 55.0);
    build_succubus(&mut commands, &mut meshes, &mut materials, pos);
}

// Succubus AI: hover-chase the player, then swoop in and rake with her claws.
fn succubus_ai(
    time: Res<Time>,
    mut succ_q: Query<(&mut Transform, &mut Enemy, &mut Succubus)>,
    player_q: Query<&Transform, (With<Player>, Without<Succubus>)>,
    mut player_vel: Query<&mut PlayerVelocity, With<Player>>,
    mut health: ResMut<PlayerHealth>,
) {
    let Ok(pt) = player_q.get_single() else { return; };
    let pp = pt.translation;
    let dt = time.delta_secs();
    let et = time.elapsed_secs();
    for (mut t, mut e, mut s) in succ_q.iter_mut() {
        // knockback decay
        if e.knockback_vel.length_squared() > 0.01 {
            t.translation += e.knockback_vel * dt;
            e.knockback_vel *= (1.0 - 6.0 * dt).max(0.0);
        }
        let flat = Vec3::new(pp.x - t.translation.x, 0.0, pp.z - t.translation.z);
        let d = flat.length().max(0.01);
        let dir = flat / d;
        let ty = t.translation.y;
        t.look_at(Vec3::new(pp.x, ty, pp.z), Vec3::Y);

        s.attack -= dt;
        s.swoop = (s.swoop - dt).max(0.0);
        if d > 4.5 {
            // hover-chase, weaving height
            t.translation += dir * e.speed * dt;
            t.translation.y = e.base_y + (et * 3.0 + e.base_y).sin() * 0.5;
        } else {
            // close: swoop down and claw
            if s.attack <= 0.0 {
                s.attack = 1.4; s.swoop = 0.35;
                health.take(14.0, 0.7);
                if let Ok(mut v) = player_vel.get_single_mut() { v.knockback = dir * 6.0; }
            }
            let dive = if s.swoop > 0.0 { 1.2 } else { 0.0 };
            t.translation.y = (e.base_y - dive).max(1.4) + (et * 6.0).sin() * 0.2;
        }
    }
}

// Flap the succubus wings (and a quick downstroke during a swoop).
fn animate_succubus_wings(
    time: Res<Time>,
    children_q: Query<&Children>,
    succ_root: Query<(Entity, &Succubus)>,
    mut wing_q: Query<(&mut Transform, &SuccubusWing)>,
) {
    let t = time.elapsed_secs();
    for (root, s) in &succ_root {
        let flap = (t * 9.0).sin() * 0.7 - if s.swoop > 0.0 { 0.5 } else { 0.0 };
        if let Ok(children) = children_q.get(root) {
            for &c in children.iter() {
                if let Ok((mut tr, w)) = wing_q.get_mut(c) {
                    tr.rotation = Quat::from_rotation_z(w.side * (0.2 + flap));
                }
            }
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  SAURON — the Dark Lord. A dark-portal well at the spire base leads to his arena.
// ════════════════════════════════════════════════════════════════════════════
fn spawn_sauron_portal(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // At the foot of the spire (320, 240)
    let c = Vec3::new(320.0, 0.0, 256.0);
    let dark = materials.add(StandardMaterial { base_color: Color::srgb(0.02, 0.0, 0.04), unlit: true, ..default() });
    let rim = materials.add(StandardMaterial { base_color: Color::srgb(0.5, 0.5, 0.6),
        emissive: LinearRgba::new(2.0, 2.0, 3.0, 1.0), unlit: true, ..default() });
    let mote = materials.add(StandardMaterial { base_color: Color::WHITE, emissive: LinearRgba::new(6.0, 6.0, 7.0, 1.0), unlit: true, ..default() });
    // ground well
    commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 4.0, half_height: 0.1 })), MeshMaterial3d(dark.clone()),
        Transform::from_xyz(c.x, 0.12, c.z), SauronPortal));
    commands.spawn((Mesh3d(meshes.add(Annulus::new(4.0, 4.6))), MeshMaterial3d(rim.clone()),
        Transform::from_xyz(c.x, 0.14, c.z).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))));
    commands.spawn((PointLight { color: Color::srgb(0.7, 0.7, 1.0), intensity: 600_000.0, range: 30.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(c.x, 2.0, c.z)));
    // swirling white motes (spun by sauron_portal_anim)
    let swirl = commands.spawn((Transform::from_xyz(c.x, 0.6, c.z), GlobalTransform::default(), Visibility::default(), PortalSwirl)).id();
    for i in 0..16u32 {
        let a = i as f32 / 16.0 * std::f32::consts::TAU;
        let r = 1.2 + (i % 4) as f32 * 0.7;
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.13))), MeshMaterial3d(mote.clone()),
            Transform::from_xyz(a.cos() * r, (i % 3) as f32 * 0.3, a.sin() * r))).set_parent(swirl);
    }
}

fn sauron_portal_anim(time: Res<Time>, mut q: Query<&mut Transform, With<PortalSwirl>>) {
    for mut t in q.iter_mut() { t.rotation = Quat::from_rotation_y(time.elapsed_secs() * 1.6); }
}

// Step into the well → transported to Sauron's dark arena (built once).
fn sauron_enter(
    mut fight: ResMut<SauronFight>,
    portal_q: Query<&GlobalTransform, With<SauronPortal>>,
    mut player_q: Query<(&mut Transform, &mut PlayerVelocity), With<Player>>,
    mut clear: ResMut<ClearColor>,
    mut ambient: ResMut<AmbientLight>,
    mut sun_q: Query<&mut DirectionalLight>,
    mut fog_q: Query<&mut DistanceFog, With<PlayerCamera>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if fight.active || fight.defeated { return; }
    let (mut pt, mut pv) = player_q.single_mut();
    let pp = pt.translation;
    let mut enter = false;
    for g in &portal_q {
        if Vec3::new(g.translation().x - pp.x, 0.0, g.translation().z - pp.z).length() < 4.5 { enter = true; }
    }
    if !enter { return; }
    let o = fight.origin;
    if !fight.spawned {
        build_sauron_arena(&mut commands, &mut meshes, &mut materials, o);
        build_sauron(&mut commands, &mut meshes, &mut materials, o);
        fight.spawned = true;
    }
    fight.active = true;
    pt.translation = Vec3::new(o.x, 0.0, o.z + 22.0);
    pv.vertical = 0.0; pv.knockback = Vec3::ZERO;
    // Oppressive dark-red atmosphere
    clear.0 = Color::srgb(0.04, 0.01, 0.02);
    ambient.color = Color::srgb(0.4, 0.18, 0.18);
    ambient.brightness = 180.0;
    if let Ok(mut sun) = sun_q.get_single_mut() { sun.color = Color::srgb(1.0, 0.3, 0.15); sun.illuminance = 1500.0; }
    if let Ok(mut fog) = fog_q.get_single_mut() { fog.color = Color::srgb(0.06, 0.01, 0.02); fog.falloff = FogFalloff::Linear { start: 30.0, end: 240.0 }; }

    // ── Boss healthbar across the top of the screen ──
    commands.spawn((
        Node { position_type: PositionType::Absolute, top: Val::Px(28.0),
               left: Val::Percent(50.0), margin: UiRect::left(Val::Px(-340.0)),
               width: Val::Px(680.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0),
               align_items: AlignItems::Center, ..default() },
        SauronHpBar,
    )).with_children(|p| {
        p.spawn((Text::new("SAURON, THE DARK LORD"),
            TextFont { font_size: 22.0, ..default() }, TextColor(Color::srgb(1.0, 0.4, 0.1))));
        // bar frame
        p.spawn((
            Node { width: Val::Px(680.0), height: Val::Px(20.0), border: UiRect::all(Val::Px(2.0)), ..default() },
            BorderColor(Color::srgb(0.8, 0.3, 0.1)),
            BackgroundColor(Color::srgb(0.12, 0.02, 0.02)),
        )).with_children(|f| {
            f.spawn((
                Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                BackgroundColor(Color::srgb(0.85, 0.12, 0.05)),
                SauronHpFill,
            ));
        });
    });
}

// Drive the boss healthbar fill from Sauron's remaining health.
fn sauron_hp_bar(
    sauron_q: Query<(&Enemy, &Sauron)>,
    mut fill_q: Query<&mut Node, With<SauronHpFill>>,
) {
    let Ok((e, s)) = sauron_q.get_single() else { return; };
    let frac = (e.health / s.max).clamp(0.0, 1.0);
    for mut n in fill_q.iter_mut() { n.width = Val::Percent(frac * 100.0); }
}

// When Sauron falls, open a portal home and quell the Eye's beam for good.
fn sauron_victory(
    mut fight: ResMut<SauronFight>,
    sauron_q: Query<(), With<Sauron>>,
    bar_q: Query<Entity, With<SauronHpBar>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Active fight, boss was engaged and is now gone (slain) → victory.
    if !fight.active || fight.defeated || !fight.engaged || !sauron_q.is_empty() { return; }
    fight.defeated = true;
    for e in bar_q.iter() { commands.entity(e).despawn_recursive(); }
    // A radiant white return-portal at the centre of the arena
    let o = fight.origin;
    let glow = materials.add(StandardMaterial { base_color: Color::WHITE, emissive: LinearRgba::new(6.0, 6.0, 7.0, 1.0), unlit: true, ..default() });
    let dark = materials.add(StandardMaterial { base_color: Color::srgb(0.85, 0.9, 1.0), emissive: LinearRgba::new(2.5, 3.0, 4.0, 1.0), unlit: true, ..default() });
    commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 2.4, half_height: 3.0 })), MeshMaterial3d(dark.clone()),
        Transform::from_xyz(o.x, 3.0, o.z), ReturnPortal, SauronArena));
    commands.spawn((Mesh3d(meshes.add(Sphere::new(1.4))), MeshMaterial3d(glow.clone()),
        Transform::from_xyz(o.x, 3.0, o.z), SauronArena));
    commands.spawn((PointLight { color: Color::srgb(0.8, 0.9, 1.0), intensity: 3_000_000.0, range: 50.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(o.x, 4.0, o.z), SauronArena));
    // Nyx is waiting in the arena to meet the one who unmade the Dark Lord
    build_mystic(&mut commands, &mut meshes, &mut materials, Vec3::new(o.x - 6.0, 0.0, o.z + 5.0));
    // Victory banner
    commands.spawn((
        Node { position_type: PositionType::Absolute, top: Val::Px(40.0), left: Val::Percent(50.0),
               margin: UiRect::left(Val::Px(-260.0)), width: Val::Px(520.0),
               justify_content: JustifyContent::Center, ..default() },
        SauronArena,
    )).with_children(|p| {
        p.spawn((Text::new("SAURON IS VANQUISHED\nStep into the light to return"),
            TextFont { font_size: 26.0, ..default() }, TextColor(Color::srgb(0.9, 0.95, 1.0)),
            TextLayout::new_with_justify(JustifyText::Center)));
    });
}

// Step into the radiant portal → back to the spire, world restored, Eye silenced.
fn return_portal_enter(
    mut fight: ResMut<SauronFight>,
    portal_q: Query<&GlobalTransform, With<ReturnPortal>>,
    arena_q: Query<Entity, With<SauronArena>>,
    mut player_q: Query<(&mut Transform, &mut PlayerVelocity), With<Player>>,
    mut clear: ResMut<ClearColor>,
    mut ambient: ResMut<AmbientLight>,
    mut sun_q: Query<&mut DirectionalLight>,
    mut fog_q: Query<&mut DistanceFog, With<PlayerCamera>>,
    mut commands: Commands,
) {
    if !fight.active || !fight.defeated { return; }
    let (mut pt, mut pv) = player_q.single_mut();
    let pp = pt.translation;
    let mut enter = false;
    for g in &portal_q {
        if Vec3::new(g.translation().x - pp.x, 0.0, g.translation().z - pp.z).length() < 3.5 { enter = true; }
    }
    if !enter { return; }
    fight.active = false;
    // tidy the arena + portal away (the boss stays dead — defeated flag persists)
    for e in arena_q.iter() { commands.entity(e).despawn_recursive(); }
    // Send the player back beside the spire portal
    pt.translation = Vec3::new(320.0, 0.0, 268.0);
    pv.vertical = 0.0; pv.knockback = Vec3::ZERO;
    // Restore the moonlit overworld atmosphere
    clear.0 = Color::srgb(0.018, 0.025, 0.06);
    ambient.color = Color::srgb(0.32, 0.37, 0.58);
    ambient.brightness = 416.0;
    if let Ok(mut sun) = sun_q.get_single_mut() { sun.color = Color::srgb(0.62, 0.72, 1.0); sun.illuminance = 2860.0; }
    if let Ok(mut fog) = fog_q.get_single_mut() { fog.color = Color::srgb(0.04, 0.05, 0.11); fog.falloff = FogFalloff::Linear { start: 110.0, end: 1150.0 }; }
}

fn build_sauron_arena(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>, o: Vec3) {
    let floor = materials.add(StandardMaterial { base_color: Color::srgb(0.10, 0.09, 0.11), perceptual_roughness: 0.9, ..default() });
    let pillar = materials.add(StandardMaterial { base_color: Color::srgb(0.06, 0.05, 0.07), perceptual_roughness: 0.95, ..default() });
    let ember = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.3, 0.05), emissive: LinearRgba::new(6.0, 1.4, 0.1, 1.0), unlit: true, ..default() });
    // round obsidian arena floor (walkable)
    commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 60.0, half_height: 1.0 })), MeshMaterial3d(floor.clone()),
        Transform::from_xyz(o.x, -1.0, o.z), Walkable { half: Vec2::new(60.0, 60.0), top: 0.0 }, SauronArena));
    // ring of jagged pillars with ember tops
    for i in 0..18u32 {
        let a = i as f32 / 18.0 * std::f32::consts::TAU;
        let px = o.x + a.cos() * 58.0; let pz = o.z + a.sin() * 58.0;
        let hgt = 22.0 + (i % 3) as f32 * 8.0;
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(4.0, hgt, 4.0))), MeshMaterial3d(pillar.clone()),
            Transform::from_xyz(px, hgt * 0.5, pz), Collider { half: Vec2::new(2.0, 2.0) }, SauronArena));
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.7))), MeshMaterial3d(ember.clone()),
            Transform::from_xyz(px, hgt + 0.4, pz), SauronArena));
        commands.spawn((PointLight { color: Color::srgb(1.0, 0.35, 0.1), intensity: 300_000.0, range: 40.0, shadows_enabled: false, ..default() },
            Transform::from_xyz(px, hgt + 1.0, pz), SauronArena));
    }
}

fn build_sauron(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>, o: Vec3) {
    let plate = materials.add(StandardMaterial { base_color: Color::srgb(0.06, 0.06, 0.08), metallic: 0.85, perceptual_roughness: 0.35, ..default() });
    let trim = materials.add(StandardMaterial { base_color: Color::srgb(0.20, 0.20, 0.24), metallic: 0.9, perceptual_roughness: 0.3, ..default() });
    let eye = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.55, 0.05), emissive: LinearRgba::new(10.0, 3.0, 0.2, 1.0), unlit: true, ..default() });
    let mace = materials.add(StandardMaterial { base_color: Color::srgb(0.08, 0.07, 0.09), metallic: 0.8, perceptual_roughness: 0.4, ..default() });

    let cape = materials.add(StandardMaterial { base_color: Color::srgb(0.03, 0.02, 0.03), perceptual_roughness: 0.95, cull_mode: None, double_sided: true, ..default() });

    commands.spawn((
        Transform::from_xyz(o.x, 0.0, o.z).with_scale(Vec3::splat(1.7)),
        GlobalTransform::default(), Visibility::default(),
        Enemy { health: 70.0, speed: 4.5, flying: false, base_y: 0.0, attack_timer: 2.0, knockback_vel: Vec3::ZERO,
                bob_phase: 0.0, anim_phase: 0.0, attack_anim: 0.0, moving: false, damage_flash: 0.0 },
        Sauron { phase: 1, max: 70.0, slam: 3.0, fire: 4.0, nova: 6.0, meteor: 3.0, enraged: false },
        Shock { timer: 0.0 },
        Collider { half: Vec2::new(1.9, 1.9) },   // solid body hitbox
    )).with_children(|p| {
        // legs
        for s in [-1.0f32, 1.0] {
            p.spawn((Mesh3d(meshes.add(Cuboid::new(0.7, 2.4, 0.8))), MeshMaterial3d(plate.clone()), Transform::from_xyz(s * 0.55, 1.2, 0.0)));
            p.spawn((Mesh3d(meshes.add(Cuboid::new(0.9, 0.4, 1.2))), MeshMaterial3d(trim.clone()), Transform::from_xyz(s * 0.55, 0.2, 0.1))); // greave/foot
        }
        // torso (broad black plate) + skirt
        p.spawn((Mesh3d(meshes.add(Cuboid::new(2.2, 2.4, 1.3))), MeshMaterial3d(plate.clone()), Transform::from_xyz(0.0, 3.4, 0.0)));
        p.spawn((Mesh3d(meshes.add(Cuboid::new(1.9, 1.2, 1.1))), MeshMaterial3d(trim.clone()), Transform::from_xyz(0.0, 2.2, 0.0)));
        // ── Billowing, tattered cape: a back panel + strips streaming to both sides ──
        p.spawn((Mesh3d(meshes.add(Cuboid::new(2.6, 4.8, 0.12))), MeshMaterial3d(cape.clone()),
            Transform::from_xyz(0.0, 2.8, 0.78).with_rotation(Quat::from_rotation_x(-0.05))));
        for s in [-1.0f32, 1.0] {
            // wide upper sweep flaring outward (windblown)
            p.spawn((Mesh3d(meshes.add(Cuboid::new(2.4, 3.4, 0.1))), MeshMaterial3d(cape.clone()),
                Transform::from_xyz(s * 1.7, 3.7, 0.5).with_rotation(Quat::from_rotation_y(s * 0.9) * Quat::from_rotation_z(s * 0.35))));
            // lower tattered streamers
            for k in 0..3u32 {
                let kf = k as f32;
                p.spawn((Mesh3d(meshes.add(Cuboid::new(0.5, 3.0 - kf * 0.5, 0.08))), MeshMaterial3d(cape.clone()),
                    Transform::from_xyz(s * (1.0 + kf * 0.5), 2.0 - kf * 0.3, 0.7)
                        .with_rotation(Quat::from_rotation_z(s * (0.2 + kf * 0.15)) * Quat::from_rotation_x(-0.1))));
            }
        }
        // A faint Eye sigil smouldering on his chest (the bright one is held aloft)
        p.spawn((Mesh3d(meshes.add(Sphere::new(0.32))), MeshMaterial3d(eye.clone()),
            Transform::from_xyz(0.0, 3.6, -0.7).with_scale(Vec3::new(1.0, 1.7, 0.35))));
        // spiked pauldrons
        for s in [-1.0f32, 1.0] {
            p.spawn((Mesh3d(meshes.add(Sphere::new(0.75))), MeshMaterial3d(plate.clone()), Transform::from_xyz(s * 1.45, 4.5, 0.0)));
            for k in 0..3u32 {
                let a = k as f32 * 0.7 - 0.7;
                p.spawn((Mesh3d(meshes.add(Cone { radius: 0.16, height: 0.9 }.mesh().resolution(4))), MeshMaterial3d(trim.clone()),
                    Transform::from_xyz(s * 1.55, 4.85, 0.0).with_rotation(Quat::from_rotation_z(s * (0.6 + a)))));
            }
        }
        // Right arm — hangs down at the side, gripping the mace
        p.spawn((Mesh3d(meshes.add(Cuboid::new(0.55, 2.0, 0.55))), MeshMaterial3d(plate.clone()),
            Transform::from_xyz(1.5, 3.3, 0.2).with_rotation(Quat::from_rotation_z(0.2))));
        // ── Left arm RAISED HIGH, a scaled dragon-claw gauntlet holding the Eye aloft ──
        p.spawn((Mesh3d(meshes.add(Cuboid::new(0.5, 2.2, 0.5))), MeshMaterial3d(plate.clone()),
            Transform::from_xyz(-1.5, 4.6, -0.1).with_rotation(Quat::from_rotation_z(-0.55))));   // upper arm up & out
        p.spawn((Mesh3d(meshes.add(Cuboid::new(0.42, 1.8, 0.42))), MeshMaterial3d(plate.clone()),
            Transform::from_xyz(-2.3, 6.1, -0.3).with_rotation(Quat::from_rotation_z(-0.15))));    // forearm rising
        // clawed gauntlet fingers cupping the orb
        let claw_c = Vec3::new(-2.5, 7.1, -0.4);
        for k in 0..5u32 {
            let a = k as f32 / 5.0 * std::f32::consts::TAU;
            p.spawn((Mesh3d(meshes.add(Cone { radius: 0.08, height: 0.9 }.mesh().resolution(4))), MeshMaterial3d(trim.clone()),
                Transform::from_xyz(claw_c.x + a.cos() * 0.4, claw_c.y, claw_c.z + a.sin() * 0.4)
                    .with_rotation(Quat::from_rotation_x(-0.4) * Quat::from_rotation_z(a.cos() * 0.5))));
        }
        // THE EYE — a fierce burning orb held above the claw
        p.spawn((Mesh3d(meshes.add(Sphere::new(0.55))), MeshMaterial3d(eye.clone()),
            Transform::from_translation(claw_c + Vec3::Y * 0.3).with_scale(Vec3::new(1.0, 1.5, 1.0))));
        p.spawn((PointLight { color: Color::srgb(1.0, 0.5, 0.05), intensity: 2_500_000.0, range: 50.0, shadows_enabled: false, ..default() },
            Transform::from_translation(claw_c + Vec3::Y * 0.5)));
        // ── Imposing horned helm with a tall iconic CROWN of jagged spikes ──
        p.spawn((Mesh3d(meshes.add(Cuboid::new(0.95, 1.1, 0.95))), MeshMaterial3d(plate.clone()), Transform::from_xyz(0.0, 5.3, 0.0)));
        // tall front-facing crown spikes fanning up & out
        for k in 0..7u32 {
            let f = k as f32 / 6.0 - 0.5;                 // -0.5..0.5 across the brow
            let h = 1.6 - f.abs() * 1.4;                  // centre spike tallest
            p.spawn((Mesh3d(meshes.add(Cone { radius: 0.1, height: h }.mesh().resolution(4))), MeshMaterial3d(trim.clone()),
                Transform::from_xyz(f * 0.85, 5.95 + h * 0.4, -0.1)
                    .with_rotation(Quat::from_rotation_z(-f * 0.7) * Quat::from_rotation_x(-0.25))));
        }
        // two great back-swept horns
        for s in [-1.0f32, 1.0] {
            p.spawn((Mesh3d(meshes.add(Cone { radius: 0.16, height: 1.7 }.mesh().resolution(4))), MeshMaterial3d(trim.clone()),
                Transform::from_xyz(s * 0.5, 5.7, 0.25).with_rotation(Quat::from_rotation_z(s * 0.7) * Quat::from_rotation_x(0.6))));
            p.spawn((Mesh3d(meshes.add(Sphere::new(0.12))), MeshMaterial3d(eye.clone()), Transform::from_xyz(s * 0.2, 5.3, -0.5)));   // burning eyes
        }
        p.spawn((Mesh3d(meshes.add(Cuboid::new(0.55, 0.16, 0.1))), MeshMaterial3d(eye.clone()), Transform::from_xyz(0.0, 5.22, -0.5))); // visor slit glow
        // GREAT SPIKED MACE in the right hand
        p.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.16, half_height: 2.4 })), MeshMaterial3d(mace.clone()),
            Transform::from_xyz(1.9, 2.6, -0.6).with_rotation(Quat::from_rotation_x(0.5))));
        p.spawn((Mesh3d(meshes.add(Sphere::new(0.85))), MeshMaterial3d(mace.clone()), Transform::from_xyz(2.3, 4.6, -1.6)));
        for k in 0..6u32 {
            let a = k as f32 / 6.0 * std::f32::consts::TAU;
            p.spawn((Mesh3d(meshes.add(Cone { radius: 0.13, height: 0.55 }.mesh().resolution(4))), MeshMaterial3d(trim.clone()),
                Transform::from_xyz(2.3 + a.cos() * 0.75, 4.6, -1.6 + a.sin() * 0.75).with_rotation(Quat::from_rotation_z(a))));
        }
    });
}

// Sauron, in two distinct phases:
//   Phase I  (HP > 50%): the armoured Dark Lord — stalks you, hammers ground-shock
//                        mace slams, and lobs the occasional aimed fireball.
//   Phase II (HP ≤ 50%): ENRAGED — on entry he erupts a shock and the fight changes:
//                        faster, triple fireball fans, a sweeping fire NOVA you must
//                        leap, and a rain of meteors that crash down around you.
fn sauron_ai(
    time: Res<Time>,
    mut fight: ResMut<SauronFight>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut sauron_q: Query<(&mut Transform, &mut Enemy, &mut Sauron)>,
    player_q: Query<&Transform, (With<Player>, Without<Sauron>)>,
) {
    if !sauron_q.is_empty() { fight.engaged = true; }   // confirm the boss is in play
    let Ok(pt) = player_q.get_single() else { return; };
    let pp = pt.translation;
    let dt = time.delta_secs();
    let et = time.elapsed_secs();
    let fire_mat = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.4, 0.0),
        emissive: LinearRgba::new(6.0, 1.6, 0.0, 1.0), unlit: true, ..default() });
    let fb = meshes.add(Sphere::new(0.7));
    let shell_mat = || StandardMaterial { base_color: Color::srgba(1.0, 0.4, 0.1, 0.5),
        emissive: LinearRgba::new(6.0, 2.0, 0.3, 1.0), unlit: true, alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default() };

    for (mut t, mut e, mut s) in sauron_q.iter_mut() {
        if e.knockback_vel.length_squared() > 0.01 {
            t.translation += e.knockback_vel * dt; e.knockback_vel *= (1.0 - 8.0 * dt).max(0.0);
        }
        let frac = e.health / s.max;
        s.phase = if frac > 0.5 { 1 } else { 2 };
        let flat = Vec3::new(pp.x - t.translation.x, 0.0, pp.z - t.translation.z);
        let d = flat.length().max(0.01);
        let dir = flat / d;
        let ty = t.translation.y;
        t.look_at(Vec3::new(pp.x, ty, pp.z), Vec3::Y);
        let p2 = s.phase == 2;
        let speed = if p2 { 7.5 } else { 4.5 };
        if d > 6.0 { t.translation += dir * speed * dt; }
        t.translation.y = 0.0;

        // ── Phase-change burst: a single mighty shock + reset attack cadence ──
        if p2 && !s.enraged {
            s.enraged = true;
            s.slam = 1.2; s.fire = 1.0; s.nova = 2.5; s.meteor = 1.5;
            let origin = Vec3::new(t.translation.x, 0.2, t.translation.z);
            commands.spawn((
                Mesh3d(meshes.add(Sphere::new(1.0))), MeshMaterial3d(materials.add(shell_mat())),
                Transform::from_translation(origin).with_scale(Vec3::splat(2.0)),
                MeteorShock { radius: 2.0, hit: false, origin, max: 40.0 },
                PointLight { color: Color::srgb(1.0, 0.3, 0.1), intensity: 6_000_000.0, range: 90.0, shadows_enabled: false, ..default() },
            ));
        }

        // ── Mace slam → ground shock (both phases) ──
        s.slam -= dt;
        if s.slam <= 0.0 && d < 14.0 {
            s.slam = if p2 { 2.2 } else { 3.4 };
            let origin = Vec3::new(t.translation.x + dir.x * 4.0, 0.2, t.translation.z + dir.z * 4.0);
            commands.spawn((
                Mesh3d(meshes.add(Sphere::new(1.0))), MeshMaterial3d(materials.add(shell_mat())),
                Transform::from_translation(origin).with_scale(Vec3::splat(2.0)),
                MeteorShock { radius: 2.0, hit: false, origin, max: 18.0 },
                PointLight { color: Color::srgb(1.0, 0.4, 0.1), intensity: 2_500_000.0, range: 60.0, shadows_enabled: false, ..default() },
            ));
        }

        // ── Fireballs: single aimed (P1) → triple fan (P2) ──
        s.fire -= dt;
        if s.fire <= 0.0 {
            s.fire = if p2 { 1.8 } else { 3.0 };
            let head = t.translation + Vec3::Y * 8.0;
            let aim = (pp + Vec3::Y * 1.0 - head).normalize_or_zero();
            let spread = if p2 { 3 } else { 1 };
            for k in 0..spread {
                let off = (k as f32 - (spread as f32 - 1.0) * 0.5) * 0.22;
                let v = (aim + Vec3::new(off, 0.0, 0.0)).normalize_or_zero() * 24.0;
                commands.spawn((
                    Mesh3d(fb.clone()), MeshMaterial3d(fire_mat.clone()),
                    Transform::from_translation(head),
                    Fireball { velocity: v, life: 6.0 },
                    PointLight { color: Color::srgb(1.0, 0.4, 0.0), intensity: 120_000.0, range: 14.0, shadows_enabled: false, ..default() },
                ));
            }
        }

        // ── Phase II only: sweeping fire NOVA + raining meteors ──
        if p2 {
            // Fire nova ring blasting outward from him (leap to avoid)
            s.nova -= dt;
            if s.nova <= 0.0 {
                s.nova = 6.0;
                let head = t.translation + Vec3::Y * 5.0;
                for k in 0..18u32 {
                    let a = k as f32 / 18.0 * std::f32::consts::TAU;
                    let v = Vec3::new(a.cos() * 22.0, 3.0, a.sin() * 22.0);
                    commands.spawn((
                        Mesh3d(fb.clone()), MeshMaterial3d(fire_mat.clone()),
                        Transform::from_translation(head),
                        Fireball { velocity: v, life: 4.0 },
                    ));
                }
            }
            // Meteors crash down around the player
            s.meteor -= dt;
            if s.meteor <= 0.0 {
                s.meteor = 2.5;
                for k in 0..3u32 {
                    let a = (et * 3.3 + k as f32 * 2.1).fract() * std::f32::consts::TAU;
                    let r = 3.0 + (et * 5.7 + k as f32).fract() * 9.0;
                    let origin = Vec3::new(pp.x + a.cos() * r, 0.2, pp.z + a.sin() * r);
                    commands.spawn((
                        Mesh3d(meshes.add(Sphere::new(1.0))), MeshMaterial3d(materials.add(shell_mat())),
                        Transform::from_translation(origin).with_scale(Vec3::splat(1.0)),
                        MeteorShock { radius: 1.0, hit: false, origin, max: 12.0 },
                        PointLight { color: Color::srgb(1.0, 0.5, 0.1), intensity: 800_000.0, range: 24.0, shadows_enabled: false, ..default() },
                    ));
                }
            }
        }
    }
}

// Runs every frame but only builds the dragon when the arrival countdown fires —
// it descends as a meteor from high above the player.
fn spawn_dragon(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut arrival: ResMut<DragonArrival>,
) {
    if !arrival.spawn_now { return; }
    arrival.spawn_now = false;
    arrival.spawned = true;
    let spawn_pos = arrival.pos;
    let landing = arrival.target;
    // Vivid red scaly hide — rocky texture for scale detail, but unmistakably RED
    // (strong red base + a red emissive lift so it reads red even in shadow).
    let scale_tex = make_rock_texture(&mut images);
    let scale = materials.add(StandardMaterial {
        base_color: Color::srgb(0.66, 0.11, 0.09),
        base_color_texture: Some(scale_tex.clone()),
        uv_transform: bevy::math::Affine2::from_scale(Vec2::new(2.5, 2.5)),
        emissive: LinearRgba::new(0.18, 0.02, 0.01, 1.0),
        perceptual_roughness: 0.82, ..default()
    });
    let scale_dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.06, 0.05),
        base_color_texture: Some(scale_tex),
        uv_transform: bevy::math::Affine2::from_scale(Vec2::new(2.5, 2.5)),
        emissive: LinearRgba::new(0.09, 0.01, 0.0, 1.0),
        perceptual_roughness: 0.87, ..default()
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

    // Dragon plummets from on high (Meteor), scaled 1.35×.
    commands.spawn((
        Transform::from_translation(spawn_pos)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI))
            .with_scale(Vec3::splat(1.35)),
        GlobalTransform::default(), Visibility::default(),
        Dragon { health: 40.0, max_health: 40.0, damage_flash: 0.0, state: DragonState::Meteor,
                 enraged: false, timer: 0.0, shock_left: 0, shock_timer: 0.0,
                 fireball_timer: 4.0, fly_angle: 0.0, breath_timer: 6.0,
                 breath_target: landing, fire_timer: 0.0, dodge_timer: 0.0, dodge_dir: Vec3::ZERO },
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

// World-space sample points spanning the dragon's whole body (head→tail), so
// weapons register hits anywhere on it — not just near the belly/origin.
fn dragon_points(t: &GlobalTransform) -> [Vec3; 6] {
    let (s, r, p) = t.to_scale_rotation_translation();
    let sc = s.x;
    [
        Vec3::new(0.0, 5.0, -6.2),  // head
        Vec3::new(0.0, 3.6, -3.0),  // neck
        Vec3::new(0.0, 3.0,  0.5),  // back
        Vec3::new(0.0, 1.6,  0.6),  // belly
        Vec3::new(0.0, 3.0,  4.0),  // tail base
        Vec3::new(0.0, 2.6,  8.0),  // tail
    ].map(|o| p + r * (o * sc))
}

fn dragon_ai(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut dragon_q: Query<(Entity, &mut Transform, &mut Dragon)>,
    player_q: Query<&Transform, (With<Player>, Without<Dragon>)>,
    mut player_vel: Query<&mut PlayerVelocity, With<Player>>,
    mut slow: ResMut<MoveSlow>,
    mut health: ResMut<PlayerHealth>,
    mut kills: ResMut<KillStats>,
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

        // ── Meteor entrance: a long, slow DIAGONAL glide down a FIXED line to the
        //    locked impact point (no chasing — the player can simply step aside). ──
        if dragon.state == DragonState::Meteor {
            let landing = dragon.breath_target;
            let to_land = Vec3::new(landing.x - t.translation.x, -t.translation.y, landing.z - t.translation.z);
            let step = to_land.normalize_or_zero() * 136.5 * dt;  // descent speed (30% faster)
            t.translation += step;
            if step.length_squared() > 1e-4 {
                let ahead = t.translation + step;
                t.look_at(ahead, Vec3::Y);
            }
            // fiery trail
            let trail = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.5, 0.1),
                emissive: LinearRgba::new(8.0, 3.0, 0.4, 1.0), unlit: true, alpha_mode: AlphaMode::Add, ..default() });
            commands.spawn((Mesh3d(meshes.add(Sphere::new(2.4))), MeshMaterial3d(trail),
                Transform::from_translation(t.translation + Vec3::Y * 5.0 - step.normalize_or_zero() * 4.0), Transient { life: 0.4 }));
            if t.translation.y <= 0.5 {
                t.translation.y = 0.0;
                t.rotation = Quat::from_rotation_y(std::f32::consts::PI);
                dragon.state = DragonState::Ground;
                dragon.fire_timer = 0.0;
                let origin = Vec3::new(t.translation.x, 0.2, t.translation.z);
                let shell = materials.add(StandardMaterial { base_color: Color::srgba(1.0, 0.6, 0.2, 0.45),
                    emissive: LinearRgba::new(8.0, 3.0, 0.4, 1.0), unlit: true, alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default() });
                // A spherical shockwave shell that expands outward
                commands.spawn((
                    Mesh3d(meshes.add(Sphere::new(1.0))), MeshMaterial3d(shell),
                    Transform::from_translation(origin).with_scale(Vec3::splat(2.0)),
                    MeteorShock { radius: 2.0, hit: false, origin, max: 70.0 },
                    PointLight { color: Color::srgb(1.0, 0.6, 0.25), intensity: 8_000_000.0, range: 160.0, shadows_enabled: false, ..default() },
                ));
                // blinding flash + flung dust
                commands.spawn((Mesh3d(meshes.add(Sphere::new(6.0))),
                    MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.95, 0.8),
                        emissive: LinearRgba::new(16.0, 12.0, 7.0, 1.0), unlit: true, ..default() })),
                    Transform::from_translation(origin + Vec3::Y * 2.0), Transient { life: 0.25 }));
                let dust = materials.add(StandardMaterial { base_color: Color::srgb(0.4, 0.34, 0.28), perceptual_roughness: 1.0, ..default() });
                let cube = meshes.add(Cuboid::new(0.4, 0.4, 0.4));
                for i in 0..30u32 {
                    let a = i as f32 * 2.39996;
                    commands.spawn((Mesh3d(cube.clone()), MeshMaterial3d(dust.clone()),
                        Transform::from_translation(origin + Vec3::Y * 0.5),
                        Debris { vel: Vec3::new(a.cos() * 14.0, 6.0 + (i % 5) as f32 * 2.0, a.sin() * 14.0), life: 1.4 }));
                }
            }
            continue;
        }

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
            // Enraged: a bright red glow radiating from the dragon (like a bonfire/paladin light)
            commands.entity(entity).with_children(|c| {
                c.spawn((
                    PointLight { color: Color::srgb(1.0, 0.12, 0.06), intensity: 2_000_000.0,
                        range: 70.0, shadows_enabled: false, ..default() },
                    Transform::from_xyz(0.0, 4.0, 0.0), EnrageAura,
                ));
            });
            dragon.fire_timer = 0.0; // reuse as the sound-wave emit throttle during the roar
        }

        match dragon.state {
            DragonState::Ground => {
                let _ = &assets;
                let ty = t.translation.y;
                t.look_at(Vec3::new(pp.x, ty, pp.z), Vec3::Y);
                let flat = Vec3::new(pp.x - t.translation.x, 0.0, pp.z - t.translation.z);
                let d = flat.length().max(0.01);
                let fwd2 = flat / d;
                let mouth = t.translation + t.rotation * (Vec3::new(0.0, 5.0, -6.2) * t.scale.x);
                let mdir = (pp - mouth).normalize_or_zero();
                let h = |n: f32| (n.sin() * 43758.5).fract();

                if dragon.fire_timer > 0.0 {
                    // ── FLAME BOUT: stand and breathe a stream of sharp fire rectangles ──
                    dragon.fire_timer -= dt;
                    let flame_mat = materials.add(StandardMaterial { base_color: Color::srgba(1.0, 0.5, 0.1, 0.9),
                        emissive: LinearRgba::new(8.0, 2.6, 0.3, 1.0), unlit: true, alpha_mode: AlphaMode::Add, ..default() });
                    for j in 0..16u32 {
                        let s = et * 50.0 + j as f32 * 9.3;
                        let spread = Vec3::new(h(s) * 0.4, h(s + 1.0) * 0.4, h(s + 2.0) * 0.4);
                        let vel = (mdir + spread).normalize_or_zero() * (24.0 + h(s + 3.0).abs() * 8.0);
                        commands.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.08, 0.08, 1.0))), MeshMaterial3d(flame_mat.clone()),
                            Transform::from_translation(mouth).with_rotation(Quat::from_rotation_arc(Vec3::Z, vel.normalize_or_zero())).with_scale(Vec3::splat(0.6)),
                            FlameJet { grow: 6.5 }, Debris { vel, life: 0.55 },
                        ));
                    }
                    commands.spawn((PointLight { color: Color::srgb(1.0, 0.5, 0.15), intensity: 600_000.0, range: 28.0, shadows_enabled: false, ..default() },
                        Transform::from_translation(mouth + mdir * 4.0), Transient { life: 0.05 }));
                    let tod = pp - mouth; let dd = tod.length().max(0.01);
                    if dd < 26.0 && mdir.dot(tod / dd) > 0.6 { health.take(9.0, 0.5); }
                } else if dragon.dodge_timer > 0.0 {
                    // ── TAIL SLAM: rear and smash; mid-swing erupts dust + a ground shock ──
                    dragon.dodge_timer -= dt;
                    if dragon.dodge_dir.x == 0.0 && dragon.dodge_timer < 0.45 {
                        dragon.dodge_dir = Vec3::X; // mark slam-fired
                        let tail = t.translation + t.rotation * (Vec3::new(0.0, 1.0, 8.0) * t.scale.x);
                        let origin = Vec3::new(tail.x, 0.2, tail.z);
                        let shell = materials.add(StandardMaterial { base_color: Color::srgba(0.7, 0.55, 0.4, 0.5),
                            emissive: LinearRgba::new(2.0, 1.4, 0.6, 1.0), unlit: true, alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default() });
                        commands.spawn((
                            Mesh3d(meshes.add(Sphere::new(1.0))), MeshMaterial3d(shell),
                            Transform::from_translation(origin).with_scale(Vec3::splat(2.0)),
                            MeteorShock { radius: 2.0, hit: false, origin, max: 18.0 },
                        ));
                        // flung dirt debris
                        let dust = materials.add(StandardMaterial { base_color: Color::srgb(0.4, 0.32, 0.24), perceptual_roughness: 1.0, ..default() });
                        let cube = meshes.add(Cuboid::new(0.4, 0.4, 0.4));
                        for i in 0..22u32 {
                            let a = i as f32 * 1.9;
                            commands.spawn((Mesh3d(cube.clone()), MeshMaterial3d(dust.clone()),
                                Transform::from_translation(origin + Vec3::Y * 0.4),
                                Debris { vel: Vec3::new(a.cos() * 11.0, 5.0 + (i % 4) as f32 * 2.0, a.sin() * 11.0), life: 1.2 }));
                        }
                    }
                } else {
                    // ── APPROACH & BITE ──
                    if d > 6.0 { t.translation += fwd2 * 7.0 * dt; }
                    t.translation.y = 0.0;
                    dragon.timer -= dt;
                    if d < 8.0 && dragon.timer <= 0.0 {
                        dragon.timer = 1.5;
                        if let Ok(mut v) = player_vel.get_single_mut() { v.knockback = fwd2 * 9.0; }
                        health.take(18.0, 0.8);   // bite!
                    }
                    // Cycle a special: occasionally a flame bout, otherwise a tail slam
                    dragon.fireball_timer -= dt;
                    if dragon.fireball_timer <= 0.0 {
                        dragon.fireball_timer = 4.0 + h(et).abs() * 3.0;
                        if (et * 1.7).sin() > 0.0 { dragon.fire_timer = 1.6; }     // breathe
                        else { dragon.dodge_timer = 0.6; dragon.dodge_dir = Vec3::ZERO; } // tail slam
                    }
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
            kills.dragon = 1;
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

            // ── Leave behind an ominous VOID portal where the dragon fell ──
            let vc = Vec3::new(center.x, 11.0, center.z);
            let void_core = materials.add(StandardMaterial {
                base_color: Color::BLACK, emissive: LinearRgba::new(0.02, 0.0, 0.05, 1.0), unlit: true, ..default() });
            let void_ring = materials.add(StandardMaterial {
                base_color: Color::srgba(0.4, 0.1, 0.7, 0.8),
                emissive: LinearRgba::new(2.4, 0.4, 4.0, 1.0), unlit: true,
                alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default() });
            let part_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.6, 0.3, 1.0), emissive: LinearRgba::new(2.0, 0.6, 4.0, 1.0),
                unlit: true, alpha_mode: AlphaMode::Add, ..default() });
            let part_mesh = meshes.add(Sphere::new(0.3));
            let portal = commands.spawn((
                Transform::from_translation(vc), GlobalTransform::default(), Visibility::default(),
                VoidPortal { spawn_t: 0.0, center: vc, mat: part_mat, mesh: part_mesh },
                PointLight { color: Color::srgb(0.5, 0.2, 1.0), intensity: 700_000.0, range: 60.0, shadows_enabled: false, ..default() },
            )).id();
            // black void core
            commands.spawn((Mesh3d(meshes.add(Sphere::new(3.2))), MeshMaterial3d(void_core),
                Transform::default())).set_parent(portal);
            // swirling accretion rings (spun by animate_void_portal)
            for (rin, rout, tilt) in [(3.6f32, 5.2f32, 0.0f32), (5.6, 7.4, 0.5), (7.8, 9.0, 1.0)] {
                commands.spawn((Mesh3d(meshes.add(Annulus::new(rin, rout))), MeshMaterial3d(void_ring.clone()),
                    Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2 + tilt)))).set_parent(portal);
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
        health.take(60.0, 0.4);
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
            health.take(20.0, 0.7);
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
                health.take(40.0, 0.9);
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
            health.take(40.0, 0.9);
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

    // ════════════════════════════════════════════════════════════════════
    //  REVAMPED GREAT HALL INTERIOR — a cohesive throne room (built clean):
    //  carpet, colonnade, banners, candelabra, wall sconces, throne & relic.
    // ════════════════════════════════════════════════════════════════════
    let cloth = materials.add(StandardMaterial { base_color: Color::srgb(0.52, 0.06, 0.08), perceptual_roughness: 1.0, ..default() });
    let gold  = materials.add(StandardMaterial { base_color: Color::srgb(0.88, 0.70, 0.22), emissive: LinearRgba::new(0.25, 0.18, 0.03, 1.0), metallic: 0.9, perceptual_roughness: 0.3, ..default() });
    let iron  = materials.add(StandardMaterial { base_color: Color::srgb(0.22, 0.22, 0.26), metallic: 0.7, perceptual_roughness: 0.5, ..default() });
    let marble = materials.add(StandardMaterial { base_color: Color::srgb(0.60, 0.58, 0.62), perceptual_roughness: 0.6, ..default() });
    let relic_mat = materials.add(StandardMaterial { base_color: Color::srgb(0.6, 0.85, 1.0), emissive: LinearRgba::new(2.0, 3.5, 6.0, 1.0), unlit: true, ..default() });
    let throne_z = back_z + 12.0;

    // ── Royal carpet (gate → throne) with gold trim ──
    let carpet_len = (front_z - throne_z).abs() + 6.0;
    let carpet_cz = (front_z + throne_z) * 0.5;
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(7.0, 0.08, carpet_len))), MeshMaterial3d(cloth.clone()),
        Transform::from_xyz(0.0, 0.06, carpet_cz)));
    for tx in [-3.3f32, 3.3] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.5, 0.10, carpet_len))), MeshMaterial3d(gold.clone()),
            Transform::from_xyz(tx, 0.07, carpet_cz)));
    }

    // ── Colonnade: clean fluted columns down both sides (solid) ──
    let col_shaft = meshes.add(Cylinder { radius: 1.4, half_height: 9.0 });
    let col_cap   = meshes.add(Cuboid::new(3.4, 1.0, 3.4));
    let mut cz_i = front_z - 12.0;
    while cz_i > back_z + 8.0 {
        for cxp in [-34.0f32, 34.0] {
            commands.spawn((Mesh3d(col_shaft.clone()), MeshMaterial3d(marble.clone()),
                Transform::from_xyz(cxp, 9.0, cz_i), Collider { half: Vec2::new(1.4, 1.4) }));
            commands.spawn((Mesh3d(col_cap.clone()), MeshMaterial3d(gold.clone()), Transform::from_xyz(cxp, 18.2, cz_i)));
            commands.spawn((Mesh3d(col_cap.clone()), MeshMaterial3d(gold.clone()), Transform::from_xyz(cxp, 0.5, cz_i)));
            // banner hung between the wall and the column
            let wall_x = if cxp < 0.0 { -hw + 2.5 } else { hw - 2.5 };
            commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.2, 8.0, 3.2))), MeshMaterial3d(cloth.clone()),
                Transform::from_xyz(wall_x, 13.0, cz_i)));
            commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.3, 0.5, 3.6))), MeshMaterial3d(gold.clone()),
                Transform::from_xyz(wall_x, 17.2, cz_i)));
            // wall sconce torch beside the banner
            commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.18, 0.7, 0.18))), MeshMaterial3d(iron.clone()),
                Transform::from_xyz(wall_x, 6.0, cz_i)));
            commands.spawn((Mesh3d(meshes.add(Sphere::new(0.32))), MeshMaterial3d(flame_mat.clone()),
                Transform::from_xyz(wall_x, 6.6, cz_i)));
            commands.spawn((PointLight { color: Color::srgb(1.0, 0.6, 0.22), intensity: 260_000.0, range: 38.0, shadows_enabled: false, ..default() },
                Transform::from_xyz(wall_x, 7.0, cz_i)));
        }
        cz_i -= 17.0;
    }

    // ── Standing gold candelabra lining the carpet ──
    for cz2 in [front_z - 16.0, cz, throne_z + 14.0] {
        for sx in [-5.0f32, 5.0] {
            commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.16, half_height: 1.6 })), MeshMaterial3d(gold.clone()),
                Transform::from_xyz(sx, 1.6, cz2)));
            commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.4, half_height: 0.12 })), MeshMaterial3d(gold.clone()),
                Transform::from_xyz(sx, 3.2, cz2)));
            commands.spawn((Mesh3d(meshes.add(Sphere::new(0.3))), MeshMaterial3d(flame_mat.clone()),
                Transform::from_xyz(sx, 3.5, cz2)));
            commands.spawn((PointLight { color: Color::srgb(1.0, 0.64, 0.24), intensity: 220_000.0, range: 30.0, shadows_enabled: false, ..default() },
                Transform::from_xyz(sx, 3.8, cz2)));
        }
    }

    // ── Grand throne on a tiered dais, flanked by gold braziers ──
    for (i, (w, d)) in [(16.0f32, 11.0f32), (12.0, 8.0), (8.0, 6.0)].into_iter().enumerate() {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(w, 0.9, d))), MeshMaterial3d(marble.clone()),
            Transform::from_xyz(0.0, 0.45 + i as f32 * 0.9, throne_z + 2.0 - i as f32 * 1.0)));
    }
    let seat_y = 2.7;
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(3.4, 1.6, 3.2))), MeshMaterial3d(dark_stone.clone()),
        Transform::from_xyz(0.0, seat_y + 0.8, throne_z)));                          // seat
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(2.8, 0.5, 2.6))), MeshMaterial3d(cloth.clone()),
        Transform::from_xyz(0.0, seat_y + 1.65, throne_z)));                         // cushion
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(3.8, 6.0, 0.7))), MeshMaterial3d(dark_stone.clone()),
        Transform::from_xyz(0.0, seat_y + 3.5, throne_z - 1.5)));                    // tall backrest
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(3.4, 1.4, 0.5))), MeshMaterial3d(gold.clone()),
        Transform::from_xyz(0.0, seat_y + 6.8, throne_z - 1.5)));                    // gold crest
    for sx in [-1.0f32, 1.0] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.5, 1.4, 2.6))), MeshMaterial3d(dark_stone.clone()),
            Transform::from_xyz(sx * 1.95, seat_y + 1.4, throne_z)));                // armrests
        // flanking gold braziers with flame + light
        commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.8, half_height: 2.2 })), MeshMaterial3d(gold.clone()),
            Transform::from_xyz(sx * 7.5, 2.2, throne_z)));
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.7))), MeshMaterial3d(flame_mat.clone()),
            Transform::from_xyz(sx * 7.5, 4.8, throne_z)));
        commands.spawn((PointLight { color: Color::srgb(1.0, 0.6, 0.2), intensity: 500_000.0, range: 40.0, shadows_enabled: false, ..default() },
            Transform::from_xyz(sx * 7.5, 5.2, throne_z)));
    }

    // ── A glowing relic on a pedestal off to one side ──
    commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 1.0, half_height: 1.0 })), MeshMaterial3d(marble.clone()),
        Transform::from_xyz(-22.0, 1.0, cz + 18.0), Collider { half: Vec2::new(1.0, 1.0) }));
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.55))), MeshMaterial3d(relic_mat.clone()),
        Transform::from_xyz(-22.0, 2.6, cz + 18.0)));
    commands.spawn((PointLight { color: Color::srgb(0.5, 0.8, 1.0), intensity: 300_000.0, range: 24.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(-22.0, 3.0, cz + 18.0)));

    // ── A weapon rack against the far wall ──
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(4.0, 0.3, 0.6))), MeshMaterial3d(wood.clone()),
        Transform::from_xyz(22.0, 3.2, cz + 18.0)));
    for k in 0..4u32 {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.12, 3.0, 0.04))), MeshMaterial3d(iron.clone()),
            Transform::from_xyz(20.5 + k as f32 * 1.0, 1.6, cz + 18.0)));
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.5, 0.16, 0.1))), MeshMaterial3d(gold.clone()),
            Transform::from_xyz(20.5 + k as f32 * 1.0, 2.9, cz + 18.0)));
    }

    // ── Two soft overhead lights so the hall reads clearly ──
    for lz in [cz + 26.0, cz - 26.0] {
        commands.spawn((
            PointLight { color: Color::srgb(1.0, 0.72, 0.4), intensity: 1_800_000.0, range: 140.0, shadows_enabled: false, ..default() },
            Transform::from_xyz(0.0, 22.0, lz),
        ));
    }

    // ════════════════════════════════════════════════════════════════════
    //  Dark-Souls silhouette pass: crowned keep, conical tower roofs, a
    //  gatehouse, buttresses, guardian statues, ruin & weathering.
    // ════════════════════════════════════════════════════════════════════
    let roof = materials.add(StandardMaterial { base_color: Color::srgb(0.13, 0.12, 0.16), perceptual_roughness: 0.9, ..default() });
    let ivy  = materials.add(StandardMaterial { base_color: Color::srgb(0.11, 0.21, 0.10), perceptual_roughness: 1.0, ..default() });

    // ── Corner towers: stone drum + tall conical roof + gold finial (tall silhouette) ──
    for (tx, tz) in [(-hw, front_z), (hw, front_z), (-hw, back_z), (hw, back_z)] {
        commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 5.7, half_height: 4.0 })), MeshMaterial3d(dark_stone.clone()),
            Transform::from_xyz(tx, th + 4.0, tz), Collider { half: Vec2::new(5.0, 5.0) }));
        commands.spawn((Mesh3d(meshes.add(Cone { radius: 6.4, height: 12.0 }.mesh().resolution(10))), MeshMaterial3d(roof.clone()),
            Transform::from_xyz(tx, th + 8.0 + 6.0, tz)));
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.5))), MeshMaterial3d(gold.clone()),
            Transform::from_xyz(tx, th + 8.0 + 12.0, tz)));
    }

    // ── Gatehouse: two tall flanking towers + a machicolation block over the gate ──
    for gx in [-(gate + 5.0), gate + 5.0] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(7.0, 32.0, 7.0))), MeshMaterial3d(dark_stone.clone()),
            Transform::from_xyz(gx, 16.0, front_z), Collider { half: Vec2::new(3.5, 3.5) }));
        commands.spawn((Mesh3d(meshes.add(Cone { radius: 5.0, height: 10.0 }.mesh().resolution(8))), MeshMaterial3d(roof.clone()),
            Transform::from_xyz(gx, 32.0 + 5.0, front_z)));
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.4))), MeshMaterial3d(gold.clone()),
            Transform::from_xyz(gx, 32.0 + 10.0, front_z)));
    }
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(gate * 2.0 + 8.0, 6.0, wt + 2.0))), MeshMaterial3d(dark_stone.clone()),
        Transform::from_xyz(0.0, wh + 2.0, front_z)));
    // Huge iron-banded gate doors (scale cue) recessed in the opening
    for dx in [-gate * 0.5, gate * 0.5] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(gate - 0.5, 13.0, 0.5))), MeshMaterial3d(wood.clone()),
            Transform::from_xyz(dx, 6.5, front_z - 0.4)));
    }

    // ── Central keep crown: pyramidal roof, spire, and four corner turrets ──
    let kz = cz - 12.0;
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 15.5, height: 18.0 }.mesh().resolution(4))), MeshMaterial3d(roof.clone()),
        Transform::from_xyz(0.0, 34.0 + 9.0, kz).with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_4))));
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 1.3, height: 11.0 }.mesh().resolution(6))), MeshMaterial3d(dark_stone.clone()),
        Transform::from_xyz(0.0, 34.0 + 18.0 + 5.5, kz)));
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.9))), MeshMaterial3d(gold.clone()),
        Transform::from_xyz(0.0, 34.0 + 18.0 + 11.5, kz)));
    for (dx, dz) in [(-9.0f32, -9.0f32), (9.0, -9.0), (-9.0, 9.0), (9.0, 9.0)] {
        commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 1.9, half_height: 6.0 })), MeshMaterial3d(dark_stone.clone()),
            Transform::from_xyz(dx, 34.0 + 2.0, kz + dz)));
        commands.spawn((Mesh3d(meshes.add(Cone { radius: 2.3, height: 4.5 }.mesh().resolution(7))), MeshMaterial3d(roof.clone()),
            Transform::from_xyz(dx, 34.0 + 8.0 + 2.25, kz + dz)));
    }
    // Tall arched window recesses down the keep's front face
    for wy in [12.0f32, 21.0, 30.0] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(1.6, 4.0, 0.5))), MeshMaterial3d(dark_stone.clone()),
            Transform::from_xyz(-5.0, wy, kz + 10.2)));
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(1.6, 4.0, 0.5))), MeshMaterial3d(dark_stone.clone()),
            Transform::from_xyz( 5.0, wy, kz + 10.2)));
    }

    // ── Buttresses leaning against the long side walls ──
    for wx in [-hw, hw] {
        let s = wx.signum();
        let mut z = back_z + 12.0;
        while z < front_z - 10.0 {
            commands.spawn((Mesh3d(meshes.add(Cuboid::new(3.0, 17.0, 2.2))), MeshMaterial3d(stone.clone()),
                Transform::from_xyz(wx + s * 2.6, 8.0, z).with_rotation(Quat::from_rotation_z(-s * 0.12))));
            z += 19.0;
        }
    }

    // ── Two colossal guardian statues flanking the approach (scale cue) ──
    for sx in [-17.0f32, 17.0] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(4.5, 3.0, 4.5))), MeshMaterial3d(dark_stone.clone()),
            Transform::from_xyz(sx, 1.5, front_z + 13.0), Collider { half: Vec2::new(2.3, 2.3) }));
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(2.4, 8.0, 1.8))), MeshMaterial3d(stone.clone()),
            Transform::from_xyz(sx, 7.0, front_z + 13.0)));
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.95))), MeshMaterial3d(stone.clone()),
            Transform::from_xyz(sx, 11.4, front_z + 13.0)));
        // a worn greatsword planted point-down before the statue
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.45, 8.0, 0.22))), MeshMaterial3d(dark_stone.clone()),
            Transform::from_xyz(sx - sx.signum() * 1.6, 5.0, front_z + 13.8)));
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(1.6, 0.4, 0.3))), MeshMaterial3d(dark_stone.clone()),
            Transform::from_xyz(sx - sx.signum() * 1.6, 8.6, front_z + 13.8)));
    }

    // ── Ruin & weathering: a collapsed wall chunk + rubble + ivy drapes ──
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(9.0, 13.0, 2.5))), MeshMaterial3d(stone.clone()),
        Transform::from_xyz(hw - 7.0, 5.5, front_z + 9.0).with_rotation(Quat::from_rotation_z(0.28))));
    let rubble = meshes.add(Cuboid::new(2.0, 1.5, 2.3));
    for k in 0..8u32 {
        let a = k as f32 * 2.1;
        commands.spawn((Mesh3d(rubble.clone()), MeshMaterial3d(stone.clone()),
            Transform::from_xyz(hw - 10.0 + a.cos() * 4.0, 0.7, front_z + 8.0 + a.sin() * 4.0)
                .with_rotation(Quat::from_euler(EulerRot::XYZ, a * 0.5, a, a * 0.3))));
    }
    for (ix, iz, iw) in [(-30.0f32, front_z, 3.5f32), (34.0, front_z, 3.0), (-hw, -78.0, 2.5), (hw, -112.0, 3.0)] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(iw, 11.0, 0.3))), MeshMaterial3d(ivy.clone()),
            Transform::from_xyz(ix, 8.0, iz + 0.3)));
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
    mut stamina: ResMut<Stamina>,
    mut slow: ResMut<MoveSlow>,
    mut health: ResMut<PlayerHealth>,
    petrify: Res<Petrify>,
    realm: Res<Realm>,
    ending: Res<Ending>,
    warp: Res<Warp>,
    mut player_q: Query<(&mut Transform, &mut PlayerVelocity), With<Player>>,
    walk_q: Query<(&GlobalTransform, &Walkable)>,
) {
    let (mut transform, mut velocity) = player_q.single_mut();
    let dt = time.delta_secs();

    health.iframes = (health.iframes - dt).max(0.0);
    let frozen = petrify.timer > 0.0 || ending.stage != 0 || warp.stage != 0; // stone, ending, or mid-warp
    let jump_v = if realm.in_sky { 10.2 } else { 8.5 }; // +20% hops in the heaven realm

    let moving = !frozen && (key.pressed(KeyCode::KeyW) || key.pressed(KeyCode::KeyS)
              || key.pressed(KeyCode::KeyA) || key.pressed(KeyCode::KeyD));
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
    if !frozen {
        if key.pressed(KeyCode::KeyW) { dir += forward; }
        if key.pressed(KeyCode::KeyS) { dir -= forward; }
        if key.pressed(KeyCode::KeyA) { dir -= right_flat; }
        if key.pressed(KeyCode::KeyD) { dir += right_flat; }
    }

    // ── Dodge roll (Left Ctrl): a quick burst with brief invulnerability ──
    if !frozen && key.just_pressed(KeyCode::ControlLeft) && velocity.roll_timer <= 0.0 && stamina.current >= 17.5 {
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

    // Effective floor: highest walkable surface under the player (within a small
    // step-up reach so stairs raise you), otherwise the y=0 ground.
    let p = transform.translation;
    let mut floor = 0.0f32;
    for (gt, w) in &walk_q {
        let c = gt.translation();
        if (p.x - c.x).abs() < w.half.x && (p.z - c.z).abs() < w.half.y
            && w.top <= p.y + 1.2 && w.top > floor {
            floor = w.top;
        }
    }

    let on_ground = transform.translation.y <= floor + 0.02;
    if !frozen && key.just_pressed(KeyCode::Space) && on_ground {
        velocity.vertical = jump_v;
    }
    velocity.vertical -= 22.0 * dt;
    transform.translation.y += velocity.vertical * dt;
    if transform.translation.y < floor {
        transform.translation.y = floor;
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
    ending: Res<Ending>,
    mut held_q: Query<(&HeldVisual, &mut Visibility)>,
) {
    if ending.stage != 0 { return; }   // the ending cinematic controls visibility
    for (h, mut vis) in held_q.iter_mut() {
        *vis = if inv.owns(h.kind) && h.kind == inv.selected { Visibility::Inherited } else { Visibility::Hidden };
    }
}

// Swap the held sword meshes for the Ruined Blade once it's been claimed.
fn update_blade_skin(
    inv: Res<Inventory>,
    mut steel_q: Query<&mut Visibility, (With<SteelVis>, Without<RuinedVis>)>,
    mut ruined_q: Query<&mut Visibility, (With<RuinedVis>, Without<SteelVis>)>,
) {
    let ruined = inv.has_ruined_blade;
    for mut v in steel_q.iter_mut()  { *v = if ruined { Visibility::Hidden } else { Visibility::Inherited }; }
    for mut v in ruined_q.iter_mut() { *v = if ruined { Visibility::Inherited } else { Visibility::Hidden }; }
}

// E cycles the equipped off-hand artifact through the unlocked ones.
fn cycle_artifact(
    key: Res<ButtonInput<KeyCode>>,
    mut arts: ResMut<Artifacts>,
) {
    if !key.just_pressed(KeyCode::KeyE) { return; }
    let list = arts.unlocked();
    if list.len() <= 1 { return; }
    let cur = list.iter().position(|k| *k == arts.selected).unwrap_or(0);
    arts.selected = list[(cur + 1) % list.len()];
}

// Show only the left-hand artifact model matching the equipped artifact.
fn update_artifact_visual(
    arts: Res<Artifacts>,
    ending: Res<Ending>,
    mut q: Query<(&ArtifactVisual, &mut Visibility)>,
) {
    if ending.stage != 0 { return; }   // the ending cinematic controls visibility
    for (a, mut vis) in q.iter_mut() {
        // The trident model also vanishes from the hand while it's mid-flight.
        let trident_gone = a.kind == ArtifactKind::Trident && !arts.trident_in_hand;
        *vis = if a.kind == arts.selected && arts.owns(a.kind) && !trident_gone {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

// A burst of glowing soul-shards when an enemy dies — reused at every kill site.
fn death_burst(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
    color: Color,
) {
    let lin = color.to_linear();
    let m = materials.add(StandardMaterial {
        base_color: color,
        emissive: LinearRgba::new(lin.red * 6.0, lin.green * 6.0, lin.blue * 6.0, 1.0),
        unlit: true, ..default()
    });
    let mesh = meshes.add(Sphere::new(0.15));
    for i in 0..16u32 {
        let a = i as f32 * 2.399963;
        let dir = Vec3::new(a.cos() * 0.8, 1.1 + (i % 3) as f32 * 0.4, a.sin() * 0.8).normalize();
        let spd = 4.0 + (i % 5) as f32 * 1.3;
        commands.spawn((
            Mesh3d(mesh.clone()), MeshMaterial3d(m.clone()),
            Transform::from_translation(pos + Vec3::Y * 1.0),
            Debris { vel: dir * spd, life: 0.7 + (i % 4) as f32 * 0.12 },
        ));
    }
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.7))), MeshMaterial3d(m),
        Transform::from_translation(pos + Vec3::Y * 1.0), Transient { life: 0.16 },
    ));
}

// ── Flame breath: hold Q with the Flame artifact for a flamethrower cone ──
fn flame_breath(
    key: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    time: Res<Time>,
    arts: Res<Artifacts>,
    mut mana: ResMut<Mana>,
    camera_q: Query<&GlobalTransform, With<PlayerCamera>>,
    mut skel_q: Query<(Entity, &GlobalTransform, &mut Skeleton)>,
    mut dragon_q: Query<(Entity, &GlobalTransform, &mut Dragon)>,
    mut enemy_q: Query<(Entity, &GlobalTransform, &mut Enemy)>,
    mut medusa_q: Query<(&GlobalTransform, &mut Medusa)>,
    mut kills: ResMut<KillStats>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let window = windows.single();
    let active = key.pressed(KeyCode::KeyQ)
        && arts.selected == ArtifactKind::Flame
        && window.cursor_options.grab_mode == CursorGrabMode::Locked
        && mana.current > 0.0;
    if !active { return; }
    mana.current = (mana.current - 26.0 * time.delta_secs()).max(0.0);

    let cam = camera_q.single();
    let (_, rot, pos) = cam.to_scale_rotation_translation();
    let fwd = rot * Vec3::NEG_Z;
    let origin = pos + fwd * 0.7 + rot * Vec3::new(-0.22, -0.18, 0.0);

    let t = time.elapsed_secs();
    let hash = |n: f32| (n.sin() * 43758.5).fract().abs();
    let flame_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.5, 0.1, 0.85),
        emissive: LinearRgba::new(7.0, 2.2, 0.2, 1.0), unlit: true, alpha_mode: AlphaMode::Add, ..default()
    });
    // A tight stream: tiny puffs at the muzzle that swell as they fly (animate_flame),
    // with ~30% more reach than before.
    for i in 0..4u32 {
        let s = t * 47.0 + i as f32 * 11.1;
        let spread = rot * Vec3::new((hash(s) - 0.5) * 0.18, (hash(s + 1.3) - 0.5) * 0.18, 0.0);
        let vel = (fwd + spread).normalize_or_zero() * (20.0 + hash(s + 2.1) * 8.0);
        // Sharp thin rectangle sparks oriented along travel
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.05, 0.05, 0.7))),
            MeshMaterial3d(flame_mat.clone()),
            Transform::from_translation(origin)
                .with_rotation(Quat::from_rotation_arc(Vec3::Z, vel.normalize_or_zero()))
                .with_scale(Vec3::splat(0.5)),
            FlameJet { grow: 5.5 },
            Debris { vel, life: 0.58 },
        ));
    }
    // Warm flickering glow cast by the flame stream (short-lived, re-spawned each frame)
    commands.spawn((
        PointLight { color: Color::srgb(1.0, 0.55, 0.18),
            intensity: 380_000.0 + (t * 40.0).sin() * 120_000.0, range: 22.0, shadows_enabled: false, ..default() },
        Transform::from_translation(origin + fwd * 2.5), Transient { life: 0.05 },
    ));

    // Cone damage in front (continuous tick)
    let dt = time.delta_secs();
    let mut hits: Vec<(Vec3, Color)> = Vec::new();
    for (entity, sgt, mut sk) in skel_q.iter_mut() {
        if sk.state == SkeletonState::Dead { continue; }
        let to = sgt.translation() + Vec3::Y - pos;
        let d = to.length();
        if d < 18.0 && d > 0.1 && fwd.dot(to / d) > 0.7 {
            sk.health -= 7.0 * dt; sk.damage_flash = 0.15;
            if sk.health <= 0.0 { sk.state = SkeletonState::Dead; kills.skeletons += 1;
                hits.push((sgt.translation(), Color::srgb(0.8, 0.85, 1.0)));
                commands.entity(entity).despawn_recursive(); }
        }
    }
    for (entity, egt, mut en) in enemy_q.iter_mut() {
        let to = egt.translation() + Vec3::Y - pos;
        let d = to.length();
        if d < 18.0 && d > 0.1 && fwd.dot(to / d) > 0.7 {
            en.health -= 7.0 * dt; en.damage_flash = 0.15;
            if en.health <= 0.0 { kills.beasts += 1;
                hits.push((egt.translation(), Color::srgb(1.0, 0.5, 0.2)));
                commands.entity(entity).despawn_recursive(); }
        }
    }
    for (_e, dgt, mut dr) in dragon_q.iter_mut() {
        if dr.state == DragonState::Dead { continue; }
        let to = dgt.translation() - pos;
        let d = to.length();
        if d < 21.0 && d > 0.1 && fwd.dot(to / d) > 0.5 {
            dr.health -= 5.0 * dt; dr.damage_flash = 0.15;
        }
    }
    for (mgt, mut m) in medusa_q.iter_mut() {
        if m.state == MedusaState::Dead { continue; }
        let to = mgt.translation() + Vec3::Y * 3.0 - pos;
        let d = to.length();
        if d < 18.0 && d > 0.1 && fwd.dot(to / d) > 0.6 {
            m.health -= 8.0 * dt; m.damage_flash = 0.15;
        }
    }
    for (p, c) in hits { death_burst(&mut commands, &mut meshes, &mut materials, p, c); }
}

// Swell each flame puff as it flies so the jet reads as a stream that fans out.
fn animate_flame(
    time: Res<Time>,
    mut q: Query<(&mut Transform, &FlameJet)>,
) {
    let dt = time.delta_secs();
    for (mut t, f) in q.iter_mut() {
        let s = t.scale.x + f.grow * dt;
        t.scale = Vec3::splat(s);
    }
}

// ── Paladin shield: tap Q with the Paladin artifact to gain a 30s golden shield ──
fn paladin_cast(
    key: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    arts: Res<Artifacts>,
    mut health: ResMut<PlayerHealth>,
    mut mana: ResMut<Mana>,
    player_q: Query<&Transform, With<Player>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let window = windows.single();
    if !(key.just_pressed(KeyCode::KeyQ)
        && arts.selected == ArtifactKind::Paladin
        && window.cursor_options.grab_mode == CursorGrabMode::Locked) { return; }
    // Already shielded? don't refresh-spam. Costs a little mana.
    if health.golden_timer > 0.0 || mana.current < 20.0 { return; }
    mana.current -= 20.0;

    health.golden = health.max_hp * 0.5;   // +50% temporary golden health
    health.golden_timer = 30.0;

    let pp = player_q.single().translation;
    let gold = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.85, 0.3, 0.45),
        emissive: LinearRgba::new(4.0, 3.0, 0.8, 1.0), unlit: true,
        alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default()
    });
    // Aura root follows the player for 30s: a thin gold ground ring (2× larger) +
    // a strong bonfire-style gold light that bathes the surrounding area.
    let root = commands.spawn((
        Transform::from_xyz(pp.x, 0.0, pp.z), GlobalTransform::default(), Visibility::default(),
        GoldenAura { life: 30.0 },
    )).id();
    commands.spawn((
        Mesh3d(meshes.add(Annulus::new(4.6, 5.4))), MeshMaterial3d(gold),
        Transform::from_xyz(0.0, 0.06, 0.0).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    )).set_parent(root);
    commands.spawn((
        PointLight { color: Color::srgb(1.0, 0.82, 0.35), intensity: 1_300_000.0, range: 60.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(0.0, 3.0, 0.0),
    )).set_parent(root);
}

// Keep the paladin aura under the player; gently pulse; fade out at the end.
fn update_golden_aura(
    time: Res<Time>,
    mut commands: Commands,
    player_q: Query<&Transform, (With<Player>, Without<GoldenAura>)>,
    mut q: Query<(Entity, &mut Transform, &mut GoldenAura)>,
    mut light_q: Query<&mut PointLight>,
    children_q: Query<&Children>,
) {
    let pp = if let Ok(p) = player_q.get_single() { p.translation } else { return; };
    let t = time.elapsed_secs();
    for (e, mut tr, mut g) in q.iter_mut() {
        g.life -= time.delta_secs();
        tr.translation.x = pp.x;
        tr.translation.z = pp.z;
        // Flicker the aura light a touch like a bonfire, and dim it as it expires
        if let Ok(children) = children_q.get(e) {
            for &c in children.iter() {
                if let Ok(mut l) = light_q.get_mut(c) {
                    let flicker = 1.0 + (t * 9.0).sin() * 0.06;
                    let fade = (g.life / 2.0).clamp(0.0, 1.0);
                    l.intensity = 1_300_000.0 * flicker * fade.max(0.15);
                }
            }
        }
        if g.life <= 0.0 { commands.entity(e).despawn_recursive(); }
    }
}

// ── Poseidon trident: tap Q to summon the storm, tap again to hurl the trident ──
fn trident_cast(
    key: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    mut arts: ResMut<Artifacts>,
    mut mana: ResMut<Mana>,
    camera_q: Query<&GlobalTransform, With<PlayerCamera>>,
    player_q: Query<&Transform, With<Player>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let window = windows.single();
    if !(key.just_pressed(KeyCode::KeyQ)
        && arts.selected == ArtifactKind::Trident
        && window.cursor_options.grab_mode == CursorGrabMode::Locked) { return; }

    if arts.trident_armed <= 0.0 {
        // ── Activate: go watery + summon the storm for 30s (costs mana) ──
        if mana.current < 18.0 { return; }
        mana.current -= 18.0;
        arts.trident_armed = 30.0;
        arts.trident_in_hand = true;
        let pp = player_q.single().translation;
        let splash = materials.add(StandardMaterial {
            base_color: Color::srgba(0.4, 0.7, 1.0, 0.6),
            emissive: LinearRgba::new(0.6, 2.4, 4.0, 1.0), unlit: true,
            alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default()
        });
        commands.spawn((Mesh3d(meshes.add(Sphere::new(1.4))), MeshMaterial3d(splash.clone()),
            Transform::from_xyz(pp.x, 1.2, pp.z), Transient { life: 0.45 }));
        commands.spawn((Mesh3d(meshes.add(Annulus::new(0.5, 1.6))), MeshMaterial3d(splash),
            Transform::from_xyz(pp.x, 0.06, pp.z).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            Transient { life: 0.5 }));
    } else if arts.trident_in_hand && arts.throw_cooldown <= 0.0 && mana.current >= 8.0 {
        // ── Throw the trident forward (costs a little mana) ──
        mana.current -= 8.0;
        let cam = camera_q.single();
        let (_, rot, pos) = cam.to_scale_rotation_translation();
        let fwd = rot * Vec3::NEG_Z;
        let aqua = materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.7, 1.0),
            emissive: LinearRgba::new(0.8, 3.0, 5.0, 1.0), metallic: 0.6, perceptual_roughness: 0.25, ..default()
        });
        let proj = commands.spawn((
            Transform::from_translation(pos + fwd * 1.0)
                .with_rotation(Quat::from_rotation_arc(Vec3::NEG_Z, fwd)),
            GlobalTransform::default(), Visibility::default(),
            TridentProjectile { vel: fwd * 60.0, life: 2.5 },
        )).id();
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.06, 0.06, 1.2))), MeshMaterial3d(aqua.clone()),
            Transform::default())).set_parent(proj);
        for px in [-0.12f32, 0.0, 0.12] {
            commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.04, 0.04, 0.4))), MeshMaterial3d(aqua.clone()),
                Transform::from_xyz(px, 0.0, -0.7))).set_parent(proj);
        }
        commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.08, height: 0.3 }.mesh().resolution(6))), MeshMaterial3d(aqua),
            Transform::from_xyz(0.0, 0.0, -0.95).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)))).set_parent(proj);

        arts.trident_in_hand = false;
        arts.throw_cooldown = 0.5;
    }
}

// Fly the thrown trident; on hitting an enemy/ground erupt a geyser and reform in hand.
fn move_trident(
    time: Res<Time>,
    mut arts: ResMut<Artifacts>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut proj_q: Query<(Entity, &mut Transform, &mut TridentProjectile)>,
    skel_q: Query<&GlobalTransform, With<Skeleton>>,
    enemy_q: Query<&GlobalTransform, With<Enemy>>,
) {
    let dt = time.delta_secs();
    for (e, mut t, mut p) in proj_q.iter_mut() {
        t.translation += p.vel * dt;
        p.life -= dt;
        let mut erupt = t.translation.y <= 0.3 || p.life <= 0.0;
        if !erupt {
            for g in skel_q.iter().map(|g| g.translation()).chain(enemy_q.iter().map(|g| g.translation())) {
                if (g + Vec3::Y - t.translation).length() < 2.2 { erupt = true; break; }
            }
        }
        if erupt {
            let water = materials.add(StandardMaterial {
                base_color: Color::srgba(0.4, 0.7, 1.0, 0.65),
                emissive: LinearRgba::new(0.5, 2.2, 3.6, 1.0), unlit: true,
                alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default()
            });
            let gp = Vec3::new(t.translation.x, 0.0, t.translation.z);
            // Tall water column + base ring
            commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 1.4, half_height: 6.0 })), MeshMaterial3d(water.clone()),
                Transform::from_xyz(gp.x, 6.0, gp.z), Transient { life: 0.8 }));
            commands.spawn((
                Mesh3d(meshes.add(Annulus::new(0.5, 1.0))), MeshMaterial3d(water),
                Transform::from_xyz(gp.x, 0.08, gp.z).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
                Geyser { radius: 1.0, life: 0.8, hit: false },
            ));
            commands.entity(e).despawn_recursive();
            arts.trident_in_hand = true; // reforms in the hand
        }
    }
}

// Expand the geyser ring; launch & damage enemies it sweeps over.
fn update_geyser(
    time: Res<Time>,
    mut commands: Commands,
    mut geyser_q: Query<(Entity, &mut Transform, &mut Geyser)>,
    mut skel_q: Query<(Entity, &GlobalTransform, &mut Skeleton), Without<Geyser>>,
    mut enemy_q: Query<(Entity, &GlobalTransform, &mut Enemy), Without<Geyser>>,
    mut kills: ResMut<KillStats>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dt = time.delta_secs();
    let mut bursts: Vec<(Vec3, Color)> = Vec::new();
    for (e, mut t, mut g) in geyser_q.iter_mut() {
        g.life -= dt;
        g.radius += dt * 22.0;
        t.scale = Vec3::splat(g.radius);
        let origin = Vec3::new(t.translation.x, 0.0, t.translation.z);
        if !g.hit {
            g.hit = true; // single big hit when it erupts
            for (ent, sgt, mut sk) in skel_q.iter_mut() {
                if sk.state == SkeletonState::Dead { continue; }
                let d = Vec3::new(sgt.translation().x - origin.x, 0.0, sgt.translation().z - origin.z).length();
                if d < 7.0 {
                    sk.health -= 4.0; sk.damage_flash = 0.2;
                    if sk.health <= 0.0 { sk.state = SkeletonState::Dead; kills.skeletons += 1;
                        bursts.push((sgt.translation(), Color::srgb(0.6, 0.85, 1.0)));
                        commands.entity(ent).despawn_recursive();
                    } else {
                        // Flung skyward on the water column
                        let out = (sgt.translation() - origin).normalize_or_zero();
                        commands.entity(ent).insert(Launched { vel: out * 7.0 + Vec3::Y * 15.0 });
                    }
                }
            }
            for (ent, egt, mut en) in enemy_q.iter_mut() {
                let d = Vec3::new(egt.translation().x - origin.x, 0.0, egt.translation().z - origin.z).length();
                if d < 7.0 {
                    en.health -= 4.0; en.damage_flash = 0.2;
                    if en.health <= 0.0 { kills.beasts += 1;
                        bursts.push((egt.translation(), Color::srgb(0.6, 0.85, 1.0)));
                        commands.entity(ent).despawn_recursive();
                    } else {
                        let out = (egt.translation() - origin).normalize_or_zero();
                        commands.entity(ent).insert(Launched { vel: out * 7.0 + Vec3::Y * 15.0 });
                    }
                }
            }
        }
        if g.life <= 0.0 { commands.entity(e).despawn_recursive(); }
    }
    for (p, c) in bursts { death_burst(&mut commands, &mut meshes, &mut materials, p, c); }
}

// Carry an airborne (geyser-launched) enemy through its arc, then drop the tag.
fn update_launched(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Transform, &mut Launched)>,
) {
    let dt = time.delta_secs();
    for (e, mut t, mut l) in q.iter_mut() {
        l.vel.y -= 26.0 * dt;          // gravity
        t.translation += l.vel * dt;
        let drag = (1.0 - 2.0 * dt).max(0.0);
        l.vel.x *= drag; l.vel.z *= drag;
        if t.translation.y <= 0.0 {
            t.translation.y = 0.0;
            commands.entity(e).remove::<Launched>();
        }
    }
}

// ── Telekinesis: tap Q for a fast, wide forward shockwave that violently flings foes ──
fn telekinesis_cast(
    key: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    time: Res<Time>,
    mut arts: ResMut<Artifacts>,
    mut mana: ResMut<Mana>,
    camera_q: Query<&GlobalTransform, With<PlayerCamera>>,
    mut skel_q: Query<(Entity, &GlobalTransform, &mut Skeleton)>,
    mut enemy_q: Query<(Entity, &GlobalTransform, &mut Enemy)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let window = windows.single();
    arts.tk_cooldown = (arts.tk_cooldown - time.delta_secs()).max(0.0);
    if !(key.just_pressed(KeyCode::KeyQ)
        && arts.selected == ArtifactKind::Telekinesis
        && window.cursor_options.grab_mode == CursorGrabMode::Locked
        && arts.tk_cooldown <= 0.0
        && mana.current >= 12.0) { return; }
    arts.tk_cooldown = 0.7;
    mana.current -= 12.0;

    let cam = camera_q.single();
    let (_, rot, pos) = cam.to_scale_rotation_translation();
    let fwd = rot * Vec3::NEG_Z;
    let flat = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero();

    // Violently FLING foes in a TIGHT, short forward cone — a fast arc (Launched)
    for (e, g, mut s) in skel_q.iter_mut() {
        if s.state == SkeletonState::Dead { continue; }
        let to = g.translation() - pos;
        let d = to.length();
        if d > 0.1 && d < 12.0 && fwd.dot(to / d) > 0.6 {
            let dir = Vec3::new(to.x, 0.0, to.z).normalize_or_zero();
            s.damage_flash = 0.12;
            commands.entity(e).insert(Launched { vel: dir * 52.0 + Vec3::Y * 8.0 });
        }
    }
    for (e, g, mut en) in enemy_q.iter_mut() {
        let to = g.translation() - pos;
        let d = to.length();
        if d > 0.1 && d < 12.0 && fwd.dot(to / d) > 0.6 {
            let dir = Vec3::new(to.x, 0.0, to.z).normalize_or_zero();
            en.damage_flash = 0.12;
            commands.entity(e).insert(Launched { vel: dir * 52.0 + Vec3::Y * 8.0 });
        }
    }

    // A small, fast forward force-cone + ground ring — translucent with a white glow
    let wave = materials.add(StandardMaterial {
        base_color: Color::srgba(0.92, 0.96, 1.0, 0.22),
        emissive: LinearRgba::new(3.0, 3.2, 3.6, 1.0), unlit: true,
        alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default() });
    commands.spawn((
        Mesh3d(meshes.add(Cone { radius: 1.3, height: 2.6 }.mesh().resolution(10))),
        MeshMaterial3d(wave.clone()),
        Transform::from_translation(pos + flat * 2.6 + Vec3::Y * 0.1)
            .with_rotation(Quat::from_rotation_arc(Vec3::Y, flat)),
        Expand { rate: 34.0, life: 0.35 }, NotShadowCaster,
    ));
    let ppos = pos + flat * 2.0;
    commands.spawn((
        Mesh3d(meshes.add(Annulus::new(0.6, 1.1))), MeshMaterial3d(wave),
        Transform::from_xyz(ppos.x, 0.12, ppos.z).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        Expand { rate: 40.0, life: 0.35 }, NotShadowCaster,
    ));
}

// Tick down artifact timers (storm duration + throw cooldown).
fn decay_artifact_timers(
    time: Res<Time>,
    mut arts: ResMut<Artifacts>,
) {
    let dt = time.delta_secs();
    if arts.trident_armed > 0.0 {
        arts.trident_armed -= dt;
        if arts.trident_armed <= 0.0 { arts.trident_in_hand = true; }
    }
    if arts.throw_cooldown > 0.0 { arts.throw_cooldown -= dt; }
}

// Cozy rain + clouds while the trident storm is active — spawned/cleared on demand,
// and recycled around the player so it always rains nearby.
fn weather_system(
    time: Res<Time>,
    arts: Res<Artifacts>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_q: Query<&Transform, With<Player>>,
    clouds: Query<Entity, With<RainCloud>>,
    mut drops: Query<(Entity, &mut Transform, &RainDrop), Without<Player>>,
) {
    let raining = arts.trident_armed > 0.0;
    let pp = if let Ok(p) = player_q.get_single() { p.translation } else { return; };
    let hash = |n: f32| (n.sin() * 43758.5).fract().abs();

    if !raining {
        // Storm over: clear clouds and drops if any remain
        if !clouds.is_empty() {
            for c in clouds.iter() { commands.entity(c).despawn_recursive(); }
            for (e, _t, _d) in drops.iter() { commands.entity(e).despawn_recursive(); }
        }
        return;
    }

    if clouds.is_empty() {
        // Cloud canopy overhead
        let cloud_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.10, 0.11, 0.15), perceptual_roughness: 1.0, unlit: true, ..default() });
        let cloud_mesh = meshes.add(Cuboid::new(60.0, 3.0, 60.0));
        for i in 0..18u32 {
            let a = hash(i as f32 * 2.1) * std::f32::consts::TAU;
            let d = hash(i as f32 * 3.7) * 180.0;
            commands.spawn((
                Mesh3d(cloud_mesh.clone()), MeshMaterial3d(cloud_mat.clone()),
                Transform::from_xyz(pp.x + a.cos() * d, 90.0 + hash(i as f32) * 18.0, pp.z + a.sin() * d),
                RainCloud, NotShadowCaster,
            ));
        }
        // Rain streaks
        let drop_mat = materials.add(StandardMaterial {
            base_color: Color::srgba(0.6, 0.75, 1.0, 0.5),
            emissive: LinearRgba::new(0.3, 0.5, 0.9, 1.0), unlit: true, alpha_mode: AlphaMode::Blend, ..default() });
        let drop_mesh = meshes.add(Cuboid::new(0.03, 0.9, 0.03));
        for i in 0..1200u32 {
            let a = hash(i as f32 * 1.3) * std::f32::consts::TAU;
            let d = hash(i as f32 * 5.9) * 70.0;
            let y = hash(i as f32 * 7.7) * 60.0;
            commands.spawn((
                Mesh3d(drop_mesh.clone()), MeshMaterial3d(drop_mat.clone()),
                Transform::from_xyz(pp.x + a.cos() * d, y, pp.z + a.sin() * d),
                RainDrop { vel: 45.0 + hash(i as f32 * 9.1) * 20.0 }, NotShadowCaster,
            ));
        }
    }

    // Animate falling rain; recycle drops that fall below ground or wander too far
    let dt = time.delta_secs();
    for (_e, mut t, d) in drops.iter_mut() {
        t.translation.y -= d.vel * dt;
        if t.translation.y < 0.0 || (t.translation.x - pp.x).abs() > 80.0 || (t.translation.z - pp.z).abs() > 80.0 {
            let a = hash(t.translation.x * 12.9 + t.translation.z * 78.2) * std::f32::consts::TAU;
            let dd = hash(t.translation.z * 3.3 + 1.7) * 70.0;
            t.translation = Vec3::new(pp.x + a.cos() * dd, 55.0 + hash(t.translation.x) * 8.0, pp.z + a.sin() * dd);
        }
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
    mut heal: ResMut<HealFx>,
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
                health.hp = (health.hp + 40.0).min(health.max_hp);
                drinking.timer = 0.7;
                heal.timer = 1.1; heal.color = Color::srgb(0.35, 1.0, 0.5); // green heal UI
                if inv.health_potions == 0 { inv.selected = ItemKind::Sword; }
            }
        }
        ItemKind::ManaPotion => {
            if inv.mana_potions > 0 {
                inv.mana_potions -= 1;
                mana.current = mana.max; // blue potion fully restores mana
                drinking.timer = 0.7;
                heal.timer = 1.1; heal.color = Color::srgb(0.35, 0.6, 1.0); // blue mana UI
                if inv.mana_potions == 0 { inv.selected = ItemKind::Sword; }
            }
        }
        ItemKind::Glock => {} // hitscan, handled by glock_fire
        ItemKind::Bow => {}   // arrows handled by bow_fire
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
    mut medusa_q: Query<(&GlobalTransform, &mut Medusa)>,
    mut kills: ResMut<KillStats>,
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
            if s.health <= 0.0 { s.state = SkeletonState::Dead; kills.skeletons += 1;
                death_burst(&mut commands, &mut meshes, &mut materials, g.translation(), Color::srgb(0.8, 0.85, 1.0));
                commands.entity(e).despawn_recursive(); }
        }
    }
    for (e, g, mut en) in enemy_q.iter_mut() {
        if hit(g.translation() + Vec3::Y) {
            en.health -= 1.0; en.damage_flash = 0.25;
            if en.health <= 0.0 { kills.beasts += 1;
                death_burst(&mut commands, &mut meshes, &mut materials, g.translation(), Color::srgb(1.0, 0.4, 0.3));
                commands.entity(e).despawn_recursive(); }
        }
    }
    // Full body hitbox; death + portal handled by dragon_ai (don't despawn here)
    for (_e, g, mut d) in dragon_q.iter_mut() {
        if d.state != DragonState::Dead && dragon_points(g).iter().any(|&bp| hit(bp)) {
            d.health -= 1.0; d.damage_flash = 0.2;
        }
    }
    for (g, mut m) in medusa_q.iter_mut() {
        if m.state != MedusaState::Dead && hit(g.translation() + Vec3::Y * 3.0) {
            m.health -= 2.0; m.damage_flash = 0.2;
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

// Draw the bow by HOLDING LMB (windup); release at (near) full draw to loose an
// arrow. You can't rapid-fire — every shot needs a fresh full draw.
fn bow_fire(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    time: Res<Time>,
    inv: Res<Inventory>,
    mut bow: ResMut<BowState>,
    camera_q: Query<&GlobalTransform, With<PlayerCamera>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let window = windows.single();
    let locked = window.cursor_options.grab_mode == CursorGrabMode::Locked;
    if inv.selected != ItemKind::Bow || !locked {
        bow.draw = 0.0; bow.drawing = false;
        return;
    }

    // Hold to draw (full draw in ~0.55s)
    if mouse.pressed(MouseButton::Left) {
        bow.drawing = true;
        bow.draw = (bow.draw + time.delta_secs() / 0.55).min(1.0);
    }

    // Release: loose only if fully drawn; otherwise it just relaxes
    if mouse.just_released(MouseButton::Left) {
        let full = bow.draw >= 0.95;
        let charge = bow.draw;
        bow.draw = 0.0; bow.drawing = false;
        if !full { return; }

        let cam = camera_q.single();
        let (_, rot, pos) = cam.to_scale_rotation_translation();
        let fwd = rot * Vec3::NEG_Z;
        let shaft_m = materials.add(StandardMaterial { base_color: Color::srgb(0.5, 0.35, 0.2), perceptual_roughness: 0.9, ..default() });
        let head_m = materials.add(StandardMaterial { base_color: Color::srgb(0.7, 0.72, 0.78), metallic: 0.8, perceptual_roughness: 0.3, ..default() });
        let fletch_m = materials.add(StandardMaterial { base_color: Color::srgb(0.85, 0.2, 0.2), perceptual_roughness: 0.9, ..default() });

        let start = pos + fwd * 1.0 - rot * Vec3::Y * 0.1;
        let arrow = commands.spawn((
            Transform::from_translation(start).with_rotation(Quat::from_rotation_arc(Vec3::NEG_Z, fwd)),
            GlobalTransform::default(), Visibility::default(),
            Arrow { vel: fwd * (70.0 + charge * 30.0), life: 3.0 },
        )).id();
    // shaft (long, along local -Z), steel head, fletching
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.03, 0.03, 1.1))), MeshMaterial3d(shaft_m),
        Transform::from_xyz(0.0, 0.0, 0.0))).set_parent(arrow);
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.06, height: 0.18 }.mesh().resolution(4))), MeshMaterial3d(head_m),
        Transform::from_xyz(0.0, 0.0, -0.62).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)))).set_parent(arrow);
        for a in [0.0f32, std::f32::consts::FRAC_PI_2] {
            commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.012, 0.12, 0.16))), MeshMaterial3d(fletch_m.clone()),
                Transform::from_xyz(0.0, 0.0, 0.5).with_rotation(Quat::from_rotation_z(a)))).set_parent(arrow);
        }
    }
}

// Animate the held bow's draw: pull the nock (string+arrow) back as it charges.
fn bow_draw_anim(
    bow: Res<BowState>,
    mut nock_q: Query<&mut Transform, With<BowNock>>,
) {
    for mut t in nock_q.iter_mut() {
        t.translation.z = bow.draw * 0.26; // string + arrow slide back toward the cheek
    }
}

// Fly arrows (with a little gravity); hit enemies → damage, then despawn/stick.
fn move_arrows(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut arrows: Query<(Entity, &mut Transform, &mut Arrow)>,
    mut skel_q: Query<(Entity, &GlobalTransform, &mut Skeleton), Without<Arrow>>,
    mut enemy_q: Query<(Entity, &GlobalTransform, &mut Enemy), Without<Arrow>>,
    mut dragon_q: Query<(&GlobalTransform, &mut Dragon), Without<Arrow>>,
    mut medusa_q: Query<(&GlobalTransform, &mut Medusa), Without<Arrow>>,
    mut kills: ResMut<KillStats>,
) {
    let dt = time.delta_secs();
    for (ae, mut at, mut ar) in arrows.iter_mut() {
        ar.vel.y -= 9.0 * dt; // gentle arc
        at.translation += ar.vel * dt;
        ar.life -= dt;
        if at.translation.length_squared() > 1e-4 {
            at.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, ar.vel.normalize_or_zero());
        }
        let tip = at.translation + ar.vel.normalize_or_zero() * 0.6;
        let mut consumed = false;
        for (e, g, mut s) in skel_q.iter_mut() {
            if s.state != SkeletonState::Dead && (g.translation() + Vec3::Y - tip).length() < 1.6 {
                s.health -= 3.0; s.damage_flash = 0.25;
                if s.health <= 0.0 { s.state = SkeletonState::Dead; kills.skeletons += 1;
                    death_burst(&mut commands, &mut meshes, &mut materials, g.translation(), Color::srgb(0.8, 0.85, 1.0));
                    commands.entity(e).despawn_recursive(); }
                consumed = true; break;
            }
        }
        if !consumed {
            for (e, g, mut en) in enemy_q.iter_mut() {
                if (g.translation() + Vec3::Y - tip).length() < 1.6 {
                    en.health -= 3.0; en.damage_flash = 0.25;
                    if en.health <= 0.0 { kills.beasts += 1;
                        death_burst(&mut commands, &mut meshes, &mut materials, g.translation(), Color::srgb(1.0, 0.4, 0.3));
                        commands.entity(e).despawn_recursive(); }
                    consumed = true; break;
                }
            }
        }
        if !consumed {
            for (g, mut d) in dragon_q.iter_mut() {
                if d.state != DragonState::Dead && dragon_points(g).iter().any(|&bp| (bp - tip).length() < 2.2) {
                    d.health -= 1.5; d.damage_flash = 0.2; consumed = true; break;
                }
            }
        }
        if !consumed {
            for (g, mut m) in medusa_q.iter_mut() {
                if m.state != MedusaState::Dead && (g.translation() + Vec3::Y * 3.0 - tip).length() < 2.4 {
                    m.health -= 3.0; m.damage_flash = 0.2; consumed = true; break;
                }
            }
        }
        if consumed || ar.life <= 0.0 || at.translation.y <= 0.0 {
            commands.entity(ae).despawn_recursive();
        }
    }
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
    ending: Res<Ending>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut player_q: Query<&mut Transform, (With<Player>, Without<PlayerCamera>)>,
    mut camera_q: Query<(&mut Transform, &mut PlayerCamera), Without<Player>>,
) {
    if ending.stage != 0 { return; } // the knockout cutscene owns the camera
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
    ending: Res<Ending>,
    mut camera_q: Query<(&mut Transform, &mut PlayerCamera)>,
) {
    if ending.stage != 0 { return; } // don't fight the knockout camera
    let (mut t, mut ctrl) = camera_q.single_mut();
    let moving = key.pressed(KeyCode::KeyW) || key.pressed(KeyCode::KeyS)
              || key.pressed(KeyCode::KeyA) || key.pressed(KeyCode::KeyD);
    if moving { ctrl.bob_timer += time.delta_secs() * 9.0; }
    t.translation.y = 1.7 + if moving { ctrl.bob_timer.sin() * 0.045 } else { 0.0 };
}

// While deflecting the eye beam, make the whole blade pulse red-hot and shower
// tiny sparks from along its length; otherwise restore its normal cool glow.
fn sword_glow_anim(
    time: Res<Time>,
    mut glow: ResMut<SwordGlow>,
    assets: Option<Res<SwordAssets>>,
    inv: Res<Inventory>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sword_q: Query<&GlobalTransform, With<Sword>>,
    mut light_q: Query<&mut PointLight, With<SwordGlowLight>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let dt = time.delta_secs();
    let et = time.elapsed_secs();
    glow.timer = (glow.timer - dt).max(0.0);
    let active = glow.timer > 0.0;
    let ruined = inv.has_ruined_blade;   // the sword slot is now the spectral teal blade
    let pulse = 0.5 + (et * 22.0).sin().abs() * 0.5;
    // Glow light: red while deflecting, faint teal at rest if it's the Ruined Blade
    if let Ok(mut pl) = light_q.get_single_mut() {
        pl.color = if active { Color::srgb(1.0, 0.12, 0.05) } else { Color::srgb(0.4, 1.0, 0.7) };
        pl.intensity = if active { 520_000.0 + pulse * 320_000.0 } else if ruined { 120_000.0 } else { 0.0 };
    }
    let Some(a) = assets else { return; };
    if let Some(m) = materials.get_mut(&a.blade) {
        if active {
            m.emissive = LinearRgba::new(14.0 + pulse * 16.0, 0.18, 0.04, 1.0);
            m.base_color = Color::srgb(1.0, 0.06, 0.03);
        } else if ruined {
            m.emissive = LinearRgba::new(0.4, 3.5, 2.2, 1.0);
            m.base_color = Color::srgb(0.3, 0.8, 0.6);
        } else {
            m.emissive = LinearRgba::new(0.10, 0.16, 0.32, 1.0);
            m.base_color = Color::srgb(0.68, 0.80, 0.98);
        }
    }
    if let Some(m) = materials.get_mut(&a.edge) {
        if active {
            m.emissive = LinearRgba::new(20.0 + pulse * 16.0, 0.5, 0.1, 1.0);
            m.base_color = Color::srgb(1.0, 0.12, 0.06);
        } else if ruined {
            m.emissive = LinearRgba::new(1.5, 7.0, 5.0, 1.0);
            m.base_color = Color::srgb(0.6, 1.0, 0.85);
        } else {
            m.emissive = LinearRgba::new(0.35, 0.40, 0.55, 1.0);
            m.base_color = Color::srgb(0.92, 0.96, 1.0);
        }
    }
    if active {
        if let Ok(g) = sword_q.get_single() {
            let h = |n: f32| (n.sin() * 43758.547).fract().abs();
            let up = *g.up();
            let base = g.translation();
            // Real metal-on-metal sparks: hot yellow-white, tiny thin LINE streaks
            let spark_m = materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.85, 0.25), emissive: LinearRgba::new(16.0, 11.0, 1.5, 1.0),
                unlit: true, alpha_mode: AlphaMode::Add, ..default() });
            let spark = meshes.add(Cuboid::new(0.0035, 0.0035, 0.03));   // thin little line
            for j in 0..2u32 {
                let s = et * 73.0 + j as f32 * 5.1;
                let along = h(s);
                let pos = base + up * (along * 0.45 + 0.05);
                let dir = (*g.right() * (h(s + 1.0) - 0.5) + *g.forward() * (h(s + 2.0) - 0.5) + Vec3::Y * 0.4).normalize_or_zero();
                commands.spawn((
                    Mesh3d(spark.clone()), MeshMaterial3d(spark_m.clone()),
                    Transform::from_translation(pos).with_rotation(Quat::from_rotation_arc(Vec3::Z, dir)),
                    Debris { vel: dir * 3.2 + Vec3::Y * 1.0, life: 0.25 },
                ));
            }
        }
    }
}

// Twinkle the stars and shimmer the aurora curtains.
fn animate_sky(
    time: Res<Time>,
    mut stars: Query<(&mut Transform, &SkyStar)>,
    mut auroras: Query<(&mut Transform, &AuroraBand), Without<SkyStar>>,
) {
    let t = time.elapsed_secs();
    for (mut tr, s) in stars.iter_mut() {
        let k = s.base * (0.7 + (t * 2.4 + s.phase).sin().abs() * 0.55);
        tr.scale = Vec3::splat(k);
    }
    for (mut tr, a) in auroras.iter_mut() {
        let s = (t * 0.5 + a.phase).sin();
        tr.rotation = Quat::from_rotation_y(a.yaw + s * 0.06);
        tr.scale = Vec3::new(1.0 + (t * 0.3 + a.phase).sin() * 0.1, 1.0 + s * 0.2, 1.0);
    }
}

// Occasionally streak a shooting star across the sky, then let it fade.
fn shooting_stars(
    time: Res<Time>,
    mut timer: ResMut<SkyTimer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(Entity, &mut Transform, &mut ShootingStar)>,
) {
    let dt = time.delta_secs();
    let et = time.elapsed_secs();
    let h = |n: f32| (n.sin() * 43758.547).fract().abs();
    timer.shoot -= dt;
    if timer.shoot <= 0.0 {
        timer.shoot = 13.0 + h(et) * 16.0;
        let phi = h(et * 1.7) * std::f32::consts::TAU;
        let el = (h(et * 2.9) * 35.0 + 28.0).to_radians();
        let r = 1300.0;
        let start = Vec3::new(r * el.cos() * phi.cos(), r * el.sin(), r * el.cos() * phi.sin());
        let dir = (Vec3::new(h(et * 3.3) - 0.5, -0.35, h(et * 4.1) - 0.5)).normalize();
        let mat = materials.add(StandardMaterial {
            base_color: Color::WHITE, emissive: LinearRgba::new(8.0, 8.5, 11.0, 1.0),
            unlit: true, alpha_mode: AlphaMode::Add, ..default() });
        let e = commands.spawn((
            Transform::from_translation(start).with_rotation(Quat::from_rotation_arc(Vec3::Z, dir)),
            GlobalTransform::default(), Visibility::default(),
            ShootingStar { vel: dir * 950.0, life: 1.5 },
        )).id();
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(1.4, 1.4, 48.0))), MeshMaterial3d(mat.clone()),
            Transform::from_xyz(0.0, 0.0, -22.0))).set_parent(e);
        commands.spawn((Mesh3d(meshes.add(Sphere::new(2.0))), MeshMaterial3d(mat),
            Transform::default())).set_parent(e);
    }
    for (e, mut tr, mut s) in q.iter_mut() {
        tr.translation += s.vel * dt;
        s.life -= dt;
        if s.life <= 0.0 { commands.entity(e).despawn_recursive(); }
    }
}

// Spin the void portal's rings and pull a swirl of particles inward toward the void.
fn animate_void_portal(
    time: Res<Time>,
    mut commands: Commands,
    mut portal_q: Query<(&mut Transform, &mut VoidPortal)>,
    mut part_q: Query<(Entity, &mut Transform, &mut VoidParticle), Without<VoidPortal>>,
) {
    let dt = time.delta_secs();
    let et = time.elapsed_secs();
    let h = |n: f32| (n.sin() * 43758.547).fract().abs();
    for (mut tr, mut p) in portal_q.iter_mut() {
        tr.rotation = Quat::from_rotation_z(et * 1.4); // swirl the accretion rings
        p.spawn_t -= dt;
        if p.spawn_t <= 0.0 {
            p.spawn_t = 0.02;
            for j in 0..2u32 {
                let s = et * 41.0 + j as f32 * 6.1;
                let a = h(s) * std::f32::consts::TAU;
                let r = 10.0 + h(s + 1.0) * 6.0;
                let y = (h(s + 2.0) - 0.5) * 8.0;
                commands.spawn((
                    Mesh3d(p.mesh.clone()), MeshMaterial3d(p.mat.clone()),
                    Transform::from_translation(p.center + Vec3::new(a.cos() * r, y, a.sin() * r)),
                    VoidParticle { center: p.center, angle: a, radius: r, speed: 5.0 + h(s + 3.0) * 5.0, y },
                ));
            }
        }
    }
    for (e, mut tr, mut vp) in part_q.iter_mut() {
        vp.radius -= vp.speed * dt;
        vp.angle += dt * 3.2;          // spiral inward
        vp.y *= (1.0 - dt * 1.6).max(0.0);
        tr.translation = vp.center + Vec3::new(vp.angle.cos() * vp.radius, vp.y, vp.angle.sin() * vp.radius);
        let s = (vp.radius / 14.0).clamp(0.04, 1.0);
        tr.scale = Vec3::splat(s);
        if vp.radius <= 0.5 { commands.entity(e).despawn_recursive(); }
    }
}

// ── Sky Realm: a heavenly floating-island parkour course high above the world ──
fn build_sky_realm(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    grass_tex: Handle<Image>,
) {
    let base_y = 430.0f32;
    let hash = |n: f32| (n.sin() * 43758.547).fract().abs();
    let grass = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.9, 0.5),
        base_color_texture: Some(grass_tex),
        uv_transform: bevy::math::Affine2::from_scale(Vec2::new(3.0, 3.0)),
        perceptual_roughness: 1.0, ..default() });
    let rock  = materials.add(StandardMaterial { base_color: Color::srgb(0.50, 0.42, 0.36), perceptual_roughness: 1.0, ..default() });
    let rock_dk = materials.add(StandardMaterial { base_color: Color::srgb(0.36, 0.30, 0.26), perceptual_roughness: 1.0, ..default() });
    let trunk = materials.add(StandardMaterial { base_color: Color::srgb(0.38, 0.26, 0.14), perceptual_roughness: 1.0, ..default() });
    let leaf  = materials.add(StandardMaterial { base_color: Color::srgb(0.30, 0.66, 0.32), perceptual_roughness: 1.0, ..default() });
    let crystal = materials.add(StandardMaterial { base_color: Color::srgb(0.6, 0.9, 1.0), emissive: LinearRgba::new(1.5, 3.0, 5.0, 1.0), unlit: true, ..default() });
    let cloud = materials.add(StandardMaterial { base_color: Color::srgb(0.96, 0.97, 1.0), perceptual_roughness: 1.0, unlit: true, ..default() });
    let flower_cols: Vec<Handle<StandardMaterial>> = [
        (1.0f32, 0.45, 0.6), (1.0, 0.85, 0.35), (0.7, 0.55, 1.0), (1.0, 0.55, 0.35),
    ].iter().map(|&(r, g, b)| materials.add(StandardMaterial {
        base_color: Color::srgb(r, g, b), emissive: LinearRgba::new(r * 0.6, g * 0.6, b * 0.6, 1.0), perceptual_roughness: 0.7, ..default()
    })).collect();

    // Spawns one solid, detailed floating island (grass cap + layered rock + tufts + flowers).
    // half = r so the whole grass top is firmly walkable (no edge clip-throughs).
    let island = |commands: &mut Commands, meshes: &mut Assets<Mesh>, c: Vec3, r: f32| {
        commands.spawn((
            Mesh3d(meshes.add(Cylinder { radius: r, half_height: 0.8 })), MeshMaterial3d(grass.clone()),
            Transform::from_xyz(c.x, c.y, c.z),
            Walkable { half: Vec2::new(r, r), top: c.y + 0.8 },
        ));
        // rocky rim just under the grass
        commands.spawn((Mesh3d(meshes.add(Cylinder { radius: r * 1.02, half_height: 0.6 })), MeshMaterial3d(rock.clone()),
            Transform::from_xyz(c.x, c.y - 0.7, c.z)));
        // layered inverted rock body
        commands.spawn((Mesh3d(meshes.add(Cone { radius: r * 0.95, height: r * 1.4 }.mesh().resolution(9))), MeshMaterial3d(rock.clone()),
            Transform::from_xyz(c.x, c.y - 1.2 - r * 0.7, c.z).with_rotation(Quat::from_rotation_x(std::f32::consts::PI))));
        commands.spawn((Mesh3d(meshes.add(Cone { radius: r * 0.55, height: r * 1.6 }.mesh().resolution(7))), MeshMaterial3d(rock_dk.clone()),
            Transform::from_xyz(c.x, c.y - 1.6 - r * 1.2, c.z).with_rotation(Quat::from_rotation_x(std::f32::consts::PI))));
        // a couple of grass tufts near the edge (no colliders → never block jumps)
        for k in 0..3u32 {
            let a = hash((c.x + c.z + k as f32) * 1.7) * std::f32::consts::TAU;
            commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.12, height: 0.5 }.mesh().resolution(4))), MeshMaterial3d(leaf.clone()),
                Transform::from_xyz(c.x + a.cos() * r * 0.7, c.y + 1.05, c.z + a.sin() * r * 0.7)));
        }
        // wildflowers dotting the grass
        for k in 0..6u32 {
            let a = hash(c.x * 1.3 + c.z + k as f32 * 2.7) * std::f32::consts::TAU;
            let rr = hash(c.z * 1.7 + k as f32) * r * 0.85;
            let fx = c.x + a.cos() * rr;
            let fz = c.z + a.sin() * rr;
            commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.025, half_height: 0.18 })), MeshMaterial3d(leaf.clone()),
                Transform::from_xyz(fx, c.y + 0.98, fz)));
            commands.spawn((Mesh3d(meshes.add(Sphere::new(0.12))), MeshMaterial3d(flower_cols[(k as usize) % flower_cols.len()].clone()),
                Transform::from_xyz(fx, c.y + 1.2, fz)));
        }
    };

    // Translucent waterfall material (hangs off island edges into the clouds)
    let water = materials.add(StandardMaterial {
        base_color: Color::srgba(0.55, 0.78, 1.0, 0.55),
        emissive: LinearRgba::new(0.3, 0.7, 1.2, 1.0), unlit: true,
        alpha_mode: AlphaMode::Blend, cull_mode: None, double_sided: true, ..default() });

    // ── Phase 1: a steadily-CLIMBING chain of islands (every step gains height,
    //    each ≤ the jump arc, so the path leads continuously up toward the shrine) ──
    let mut p = Vec3::new(0.0, base_y, 0.0);
    let mut ang = 0.4f32;
    let mut centers: Vec<Vec3> = vec![p];
    island(commands, meshes, p, 8.0);
    for i in 1..15u32 {
        let prev_r = if i == 1 { 8.0 } else { 4.5 + hash((i as f32 - 1.0) * 3.1) * 2.2 };
        let r = 4.5 + hash(i as f32 * 3.1) * 2.2;
        ang += (hash(i as f32 * 7.7) - 0.5) * 1.0;
        let gap = prev_r + r + 2.8 + hash(i as f32 * 5.3) * 1.2;
        let dy = 0.7 + hash(i as f32 * 11.1) * 0.9;    // 0.7..1.6 up — always climbing, reachable
        p = Vec3::new(p.x + ang.cos() * gap, p.y + dy, p.z + ang.sin() * gap);
        island(commands, meshes, p, r);
        centers.push(p);
        // decoration toward the edge so it never blocks the path
        if i % 2 == 0 {
            commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.16, half_height: 0.6 })), MeshMaterial3d(trunk.clone()),
                Transform::from_xyz(p.x + r * 0.6, p.y + 1.4, p.z))) ;
            commands.spawn((Mesh3d(meshes.add(Sphere::new(1.1))), MeshMaterial3d(leaf.clone()),
                Transform::from_xyz(p.x + r * 0.6, p.y + 2.4, p.z)));
        } else {
            commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.35, height: 1.4 }.mesh().resolution(5))), MeshMaterial3d(crystal.clone()),
                Transform::from_xyz(p.x - r * 0.6, p.y + 1.5, p.z)));
        }
        // waterfall spilling off the rim of every third island
        if i % 3 == 0 {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(2.2, 26.0, 0.4))), MeshMaterial3d(water.clone()),
                Transform::from_xyz(p.x - r * 0.9, p.y - 12.0, p.z),
                NotShadowCaster,
            ));
            commands.spawn((
                Mesh3d(meshes.add(Cylinder { radius: 1.6, half_height: 0.3 })), MeshMaterial3d(water.clone()),
                Transform::from_xyz(p.x - r * 0.9, p.y + 0.9, p.z), NotShadowCaster,
            ));
        }
    }

    // ── Phase 2: an ascending flight of stepping platforms leading up to the shrine ──
    let mut sp = p;
    let mut sang = ang;
    for i in 0..12u32 {
        sang += (hash(i as f32 * 4.4) - 0.5) * 0.5;
        sp = Vec3::new(sp.x + sang.cos() * 7.0, sp.y + 1.6, sp.z + sang.sin() * 7.0);
        island(commands, meshes, sp, 3.4);
        centers.push(sp);
        if i % 4 == 2 {
            commands.spawn((Mesh3d(meshes.add(Cuboid::new(1.6, 22.0, 0.4))), MeshMaterial3d(water.clone()),
                Transform::from_xyz(sp.x, sp.y - 10.0, sp.z - 3.0), NotShadowCaster));
        }
    }

    // ── The high shrine plateau, just one reachable hop beyond the last step ──
    let plat = Vec3::new(sp.x + sang.cos() * 16.0, sp.y + 1.4, sp.z + sang.sin() * 16.0);
    island(commands, meshes, plat, 18.0);
    build_sky_shrine(commands, meshes, materials, plat + Vec3::Y * 0.8);

    // ── A soft cloud sea beneath everything ──
    let cloud_mesh = meshes.add(Sphere::new(34.0));
    for k in 0..56u32 {
        let a = hash(k as f32 * 2.1) * std::f32::consts::TAU;
        let d = hash(k as f32 * 3.7) * 300.0;
        commands.spawn((
            Mesh3d(cloud_mesh.clone()), MeshMaterial3d(cloud.clone()),
            Transform::from_xyz(a.cos() * d, base_y - 80.0 + hash(k as f32) * 18.0, a.sin() * d)
                .with_scale(Vec3::new(1.3 + hash(k as f32 * 1.7), 0.5, 1.3 + hash(k as f32 * 5.1))),
            NotShadowCaster,
        ));
    }
    // A radiant sun high in the heavenly sky
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(60.0))),
        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.97, 0.85),
            emissive: LinearRgba::new(9.0, 8.0, 5.5, 1.0), unlit: true, ..default() })),
        Transform::from_xyz(600.0, base_y + 360.0, -500.0),
        NotShadowCaster,
    ));

    // ── Ambient wildlife ──
    let wing_cols: Vec<Handle<StandardMaterial>> = [(1.0f32,0.5,0.2),(0.5,0.7,1.0),(1.0,0.85,0.3),(0.9,0.4,0.8)]
        .iter().map(|&(r,g,b)| materials.add(StandardMaterial { base_color: Color::srgb(r,g,b), emissive: LinearRgba::new(r*0.4,g*0.4,b*0.4,1.0), unlit: true, alpha_mode: AlphaMode::Blend, cull_mode: None, double_sided: true, ..default() })).collect();
    let bird_mat = materials.add(StandardMaterial { base_color: Color::srgb(0.18, 0.18, 0.22), perceptual_roughness: 0.9, ..default() });

    // Butterflies fluttering just above the islands
    let wing_mesh = meshes.add(Cuboid::new(0.28, 0.02, 0.2));
    for i in 0..6u32 {
        let c = centers[(i as usize) % centers.len()];
        let off = Vec3::new((hash(i as f32 * 3.1) - 0.5) * 6.0, 1.8 + hash(i as f32) * 2.5, (hash(i as f32 * 5.7) - 0.5) * 6.0);
        let col = wing_cols[(i as usize) % wing_cols.len()].clone();
        let b = commands.spawn((
            Transform::from_translation(c + off), GlobalTransform::default(), Visibility::default(),
            SkyCritter { kind: 0, base: c + off, phase: hash(i as f32 * 2.2) * 6.28, speed: 1.0 + hash(i as f32) * 0.8, radius: 2.0 + hash(i as f32 * 1.3) * 2.0 },
        )).id();
        for s in [-1.0f32, 1.0] {
            commands.spawn((Mesh3d(wing_mesh.clone()), MeshMaterial3d(col.clone()),
                Transform::from_xyz(s * 0.16, 0.0, 0.0).with_rotation(Quat::from_rotation_z(s * 0.5)))).set_parent(b);
        }
    }

    // Birds wheeling overhead
    for i in 0..14u32 {
        let a = hash(i as f32 * 1.7) * std::f32::consts::TAU;
        let d = 30.0 + hash(i as f32 * 2.9) * 90.0;
        let home = Vec3::new(a.cos() * d, base_y + 18.0 + hash(i as f32) * 22.0, a.sin() * d);
        let bird = commands.spawn((
            Transform::from_translation(home), GlobalTransform::default(), Visibility::default(),
            SkyCritter { kind: 1, base: home, phase: hash(i as f32 * 4.4) * 6.28, speed: 0.4 + hash(i as f32) * 0.3, radius: 16.0 + hash(i as f32 * 3.3) * 20.0 },
        )).id();
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.3, 0.12, 0.5))), MeshMaterial3d(bird_mat.clone()), Transform::default())).set_parent(bird);
        for s in [-1.0f32, 1.0] {
            commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.9, 0.04, 0.3))), MeshMaterial3d(bird_mat.clone()),
                Transform::from_xyz(s * 0.55, 0.0, 0.0).with_rotation(Quat::from_rotation_z(s * 0.3)))).set_parent(bird);
        }
    }

}

// A grand marble shrine crowned with a dome, with a cute princess waiting at its heart.
fn build_sky_shrine(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    c: Vec3,
) {
    let marble = materials.add(StandardMaterial { base_color: Color::srgb(0.93, 0.93, 0.97), perceptual_roughness: 0.5, ..default() });
    let gold = materials.add(StandardMaterial { base_color: Color::srgb(0.95, 0.8, 0.3), emissive: LinearRgba::new(2.2, 1.6, 0.4, 1.0), metallic: 0.9, perceptual_roughness: 0.25, ..default() });
    let crystal = materials.add(StandardMaterial { base_color: Color::srgb(0.7, 0.9, 1.0), emissive: LinearRgba::new(2.0, 3.5, 6.0, 1.0), unlit: true, ..default() });

    // Stepped circular base
    for (i, r) in [13.0f32, 11.0, 9.0].into_iter().enumerate() {
        commands.spawn((Mesh3d(meshes.add(Cylinder { radius: r, half_height: 0.5 })), MeshMaterial3d(marble.clone()),
            Transform::from_xyz(c.x, c.y + 0.5 + i as f32 * 1.0, c.z),
            Walkable { half: Vec2::new(r, r), top: c.y + 1.0 + i as f32 * 1.0 }));
    }
    let floor_y = c.y + 3.0;
    // Ring of tall fluted columns
    let n_col = 10u32;
    for k in 0..n_col {
        let a = k as f32 / n_col as f32 * std::f32::consts::TAU;
        let cx = c.x + a.cos() * 9.0;
        let cz = c.z + a.sin() * 9.0;
        commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.7, half_height: 6.0 })), MeshMaterial3d(marble.clone()),
            Transform::from_xyz(cx, floor_y + 6.0, cz)));
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(2.0, 0.6, 2.0))), MeshMaterial3d(gold.clone()),
            Transform::from_xyz(cx, floor_y + 0.4, cz)));   // base
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(2.0, 0.6, 2.0))), MeshMaterial3d(gold.clone()),
            Transform::from_xyz(cx, floor_y + 12.2, cz)));  // capital
    }
    // Golden architrave ring + a great dome on top
    commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 10.0, half_height: 0.6 })), MeshMaterial3d(gold.clone()),
        Transform::from_xyz(c.x, floor_y + 12.8, c.z)));
    commands.spawn((Mesh3d(meshes.add(Sphere::new(10.0))), MeshMaterial3d(marble.clone()),
        Transform::from_xyz(c.x, floor_y + 13.0, c.z).with_scale(Vec3::new(1.0, 0.6, 1.0))));
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 1.0, height: 4.0 }.mesh().resolution(6))), MeshMaterial3d(gold.clone()),
        Transform::from_xyz(c.x, floor_y + 19.0, c.z)));
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.7))), MeshMaterial3d(crystal.clone()),
        Transform::from_xyz(c.x, floor_y + 21.4, c.z)));
    // Soft heavenly light filling the shrine
    commands.spawn((PointLight { color: Color::srgb(1.0, 0.95, 0.8), intensity: 2_500_000.0, range: 80.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(c.x, floor_y + 8.0, c.z)));

    build_princess(commands, meshes, materials, Vec3::new(c.x, floor_y + 0.05, c.z));
}

// A cute, elegant princess: layered gown, slender bodice, soft face, golden hair & tiara.
fn build_princess(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    base: Vec3,
) {
    let gown   = materials.add(StandardMaterial { base_color: Color::srgb(0.96, 0.74, 0.86), perceptual_roughness: 0.6, ..default() });
    let gown_lt= materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.92, 0.97), perceptual_roughness: 0.5, ..default() });
    let sash   = materials.add(StandardMaterial { base_color: Color::srgb(0.55, 0.45, 0.95), emissive: LinearRgba::new(0.3, 0.2, 0.8, 1.0), perceptual_roughness: 0.4, ..default() });
    let skin   = materials.add(StandardMaterial { base_color: Color::srgb(0.98, 0.84, 0.76), perceptual_roughness: 0.6, ..default() });
    let hair   = materials.add(StandardMaterial { base_color: Color::srgb(0.95, 0.82, 0.35), perceptual_roughness: 0.4, metallic: 0.2, ..default() });
    let gold   = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.85, 0.3), emissive: LinearRgba::new(3.0, 2.2, 0.6, 1.0), metallic: 0.9, perceptual_roughness: 0.2, ..default() });
    let gem    = materials.add(StandardMaterial { base_color: Color::srgb(0.7, 0.9, 1.0), emissive: LinearRgba::new(1.6, 3.0, 5.0, 1.0), unlit: true, ..default() });
    let rosy   = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.6, 0.62), perceptual_roughness: 0.7, ..default() });
    let eyem   = materials.add(StandardMaterial { base_color: Color::srgb(0.2, 0.25, 0.4), perceptual_roughness: 0.4, ..default() });

    let root = commands.spawn((
        Transform::from_translation(base), GlobalTransform::default(), Visibility::default(),
        Princess, PrincessIdle { base_y: base.y },
    )).id();
    // ── Cute chibi proportions: tiny body, big head ── (total height ~1.9)
    // Bell skirt + ruffled underskirt (30% wider)
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.81, height: 0.95 }.mesh().resolution(20))), MeshMaterial3d(gown.clone()),
        Transform::from_xyz(0.0, 0.48, 0.0))).set_parent(root);
    commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.94, height: 0.28 }.mesh().resolution(20))), MeshMaterial3d(gown_lt.clone()),
        Transform::from_xyz(0.0, 0.14, 0.0))).set_parent(root);          // frilly hem
    // Little bodice + waist sash
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.26))), MeshMaterial3d(gown_lt.clone()),
        Transform::from_xyz(0.0, 1.02, 0.0).with_scale(Vec3::new(1.0, 0.85, 0.8)))).set_parent(root);  // bodice
    commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.35, half_height: 0.05 })), MeshMaterial3d(sash.clone()),
        Transform::from_xyz(0.0, 0.9, 0.0))).set_parent(root);           // waist sash
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.07))), MeshMaterial3d(sash.clone()),
        Transform::from_xyz(0.0, 0.9, 0.26).with_scale(Vec3::new(1.6, 1.0, 0.6)))).set_parent(root);   // bow
    // Plump little arms reaching slightly forward, with rounded hands
    for s in [-1.0f32, 1.0] {
        commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.075, half_height: 0.26 })), MeshMaterial3d(gown_lt.clone()),
            Transform::from_xyz(s * 0.28, 0.95, 0.06).with_rotation(Quat::from_rotation_z(s * 0.55) * Quat::from_rotation_x(-0.35)))).set_parent(root);  // upper arm
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.085))), MeshMaterial3d(skin.clone()),
            Transform::from_xyz(s * 0.42, 0.78, 0.2))).set_parent(root);    // hand
    }
    // Big cute head
    commands.spawn((Mesh3d(meshes.add(Cylinder { radius: 0.08, half_height: 0.05 })), MeshMaterial3d(skin.clone()),
        Transform::from_xyz(0.0, 1.24, 0.0))).set_parent(root);             // neck
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.42))), MeshMaterial3d(skin.clone()),
        Transform::from_xyz(0.0, 1.62, 0.0))).set_parent(root);             // head
    // Big sparkly eyes + rosy cheeks + tiny smile
    for s in [-1.0f32, 1.0] {
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.085))), MeshMaterial3d(eyem.clone()),
            Transform::from_xyz(s * 0.15, 1.62, 0.36).with_scale(Vec3::new(1.0, 1.3, 0.6)))).set_parent(root);   // big eyes
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.03))), MeshMaterial3d(gown_lt.clone()),
            Transform::from_xyz(s * 0.13, 1.66, 0.42))).set_parent(root);   // eye sparkle
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.07))), MeshMaterial3d(rosy.clone()),
            Transform::from_xyz(s * 0.26, 1.5, 0.32).with_scale(Vec3::new(1.0, 0.7, 0.5)))).set_parent(root);    // rosy cheeks
    }
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.04))), MeshMaterial3d(rosy.clone()),
        Transform::from_xyz(0.0, 1.46, 0.38).with_scale(Vec3::new(2.0, 0.5, 0.5)))).set_parent(root);            // smile
    // Flowing golden hair: rounded cap + bangs + two long twin-tails
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.46))), MeshMaterial3d(hair.clone()),
        Transform::from_xyz(0.0, 1.7, -0.06).with_scale(Vec3::new(1.0, 1.0, 0.92)))).set_parent(root);
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.4))), MeshMaterial3d(hair.clone()),
        Transform::from_xyz(0.0, 1.78, 0.14).with_scale(Vec3::new(1.0, 0.55, 0.6)))).set_parent(root);           // bangs
    for s in [-1.0f32, 1.0] {
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.16))), MeshMaterial3d(hair.clone()),
            Transform::from_xyz(s * 0.42, 1.55, -0.05).with_scale(Vec3::new(0.8, 1.6, 0.8)))).set_parent(root);  // twin-tail upper
        commands.spawn((Mesh3d(meshes.add(Sphere::new(0.12))), MeshMaterial3d(hair.clone()),
            Transform::from_xyz(s * 0.46, 1.2, -0.02).with_scale(Vec3::new(0.7, 1.4, 0.7)))).set_parent(root);   // twin-tail lower
    }
    // Tiara: a little gold band with three gems
    commands.spawn((Mesh3d(meshes.add(Annulus::new(0.32, 0.37))), MeshMaterial3d(gold.clone()),
        Transform::from_xyz(0.0, 1.84, 0.18).with_rotation(Quat::from_rotation_x(-0.5)))).set_parent(root);
    for s in [-1.0f32, 0.0, 1.0] {
        commands.spawn((Mesh3d(meshes.add(Cone { radius: 0.05, height: 0.13 }.mesh().resolution(5))), MeshMaterial3d(gem.clone()),
            Transform::from_xyz(s * 0.18, 1.96, 0.14))).set_parent(root);
    }
    // A soft halo glow behind her
    commands.spawn((Mesh3d(meshes.add(Annulus::new(0.62, 0.7))), MeshMaterial3d(gem.clone()),
        Transform::from_xyz(0.0, 1.62, -0.42))).set_parent(root);
    commands.spawn((PointLight { color: Color::srgb(1.0, 0.85, 0.95), intensity: 400_000.0, range: 22.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(0.0, 1.4, 0.0))).set_parent(root);
}

// When the player steps into the dragon's void portal, whisk them up to the Sky Realm
// and flip the whole world to a bright, heavenly daytime atmosphere.
fn void_portal_enter(
    mut realm: ResMut<Realm>,
    portal_q: Query<&GlobalTransform, With<VoidPortal>>,
    mut player_q: Query<(&mut Transform, &mut PlayerVelocity), With<Player>>,
    mut clear: ResMut<ClearColor>,
    mut ambient: ResMut<AmbientLight>,
    mut sun_q: Query<&mut DirectionalLight>,
    mut fog_q: Query<&mut DistanceFog, With<PlayerCamera>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    if realm.in_sky { return; }
    let (mut pt, mut pv) = player_q.single_mut();
    let pp = pt.translation;
    let mut enter = false;
    for g in &portal_q {
        if Vec3::new(g.translation().x - pp.x, 0.0, g.translation().z - pp.z).length() < 6.0 { enter = true; }
    }
    if !enter { return; }

    if !realm.spawned {
        let grass_tex = make_grass_texture(&mut images);
        build_sky_realm(&mut commands, &mut meshes, &mut materials, grass_tex);
        realm.spawned = true;
    }
    realm.in_sky = true;
    pt.translation = realm.start;
    pv.vertical = 0.0; pv.knockback = Vec3::ZERO;

    // Heavenly daytime: blue sky, warm sun, bright ambient, gentle pale fog
    clear.0 = Color::srgb(0.45, 0.68, 0.96);
    ambient.color = Color::srgb(0.82, 0.86, 0.96);
    ambient.brightness = 650.0;
    if let Ok(mut sun) = sun_q.get_single_mut() {
        sun.color = Color::srgb(1.0, 0.97, 0.85);
        sun.illuminance = 12000.0;
    }
    if let Ok(mut fog) = fog_q.get_single_mut() {
        fog.color = Color::srgb(0.72, 0.83, 0.96);
        fog.falloff = FogFalloff::Linear { start: 140.0, end: 1000.0 };
    }
}

// In the heaven realm the player is invulnerable (a peaceful parkour); falling off
// an island simply pops them back to the start island.
fn sky_fall_respawn(
    realm: Res<Realm>,
    mut health: ResMut<PlayerHealth>,
    mut player_q: Query<(&mut Transform, &mut PlayerVelocity), With<Player>>,
) {
    if !realm.in_sky { return; }
    // Keep i-frames topped up so nothing from the old world can hurt the player here.
    health.iframes = health.iframes.max(1.0);
    health.hp = health.max_hp;
    let (mut t, mut v) = player_q.single_mut();
    if t.translation.y < realm.start.y - 70.0 {
        t.translation = realm.start;
        v.vertical = 0.0;
        v.knockback = Vec3::ZERO;
    }
}

// Animate the sky-realm wildlife: butterflies flutter, birds wheel.
fn animate_critters(
    time: Res<Time>,
    mut q: Query<(&mut Transform, &SkyCritter)>,
) {
    let t = time.elapsed_secs();
    for (mut tr, c) in q.iter_mut() {
        match c.kind {
            0 => { // butterfly: wandering figure-8 with bobbing
                let a = t * c.speed + c.phase;
                tr.translation = c.base + Vec3::new(a.sin() * c.radius, (a * 1.7).sin() * 0.8, (a * 0.5).cos() * c.radius);
                tr.rotation = Quat::from_rotation_y(a) * Quat::from_rotation_z((t * 12.0 + c.phase).sin() * 0.6);
            }
            _ => { // bird: wide circle, banking, gentle altitude bob
                let a = t * c.speed + c.phase;
                let pos = c.base + Vec3::new(a.cos() * c.radius, (a * 1.3).sin() * 4.0, a.sin() * c.radius);
                let tangent = Vec3::new(-a.sin(), 0.0, a.cos());
                tr.translation = pos;
                tr.look_to(tangent, Vec3::Y);
                tr.rotate_local_z((t * 8.0 + c.phase).sin() * 0.25); // flap-bank
            }
        }
    }
}

fn princess_idle(
    time: Res<Time>,
    ending: Res<Ending>,
    mut q: Query<(&mut Transform, &PrincessIdle)>,
) {
    if ending.stage != 0 { return; } // the ending cutscene drives her instead
    let t = time.elapsed_secs();
    for (mut tr, p) in q.iter_mut() {
        tr.translation.y = p.base_y + (t * 1.3).sin() * 0.12;
        tr.rotation = Quat::from_rotation_y((t * 0.3).sin() * 0.35);
    }
}

// Press R near the princess to begin the ending: a line of dialogue, then credits.
fn princess_interact(
    key: Res<ButtonInput<KeyCode>>,
    mut ending: ResMut<Ending>,
    player_q: Query<&Transform, With<Player>>,
    princess_q: Query<&GlobalTransform, With<Princess>>,
    mut windows: Query<&mut Window>,
    mut commands: Commands,
) {
    if ending.stage != 0 || !key.just_pressed(KeyCode::KeyR) { return; }
    let pp = player_q.single().translation;
    let near = princess_q.iter().any(|g| g.translation().distance(pp) < 6.0);
    if !near { return; }

    ending.stage = 1;
    ending.timer = 0.0;
    // free the cursor for the cinematic
    if let Ok(mut w) = windows.get_single_mut() {
        w.cursor_options.grab_mode = CursorGrabMode::None;
        w.cursor_options.visible = true;
    }
    // Dialogue panel
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0), bottom: Val::Px(120.0),
            margin: UiRect::left(Val::Px(-360.0)),
            width: Val::Px(720.0), padding: UiRect::all(Val::Px(24.0)),
            flex_direction: FlexDirection::Column, row_gap: Val::Px(10.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.12, 0.82)),
        BorderColor(Color::srgb(0.9, 0.8, 0.4)),
        DialogueRoot,
    )).with_children(|p| {
        p.spawn((Text::new("Princess Aurelia"),
            TextFont { font_size: 26.0, ..default() }, TextColor(Color::srgb(1.0, 0.85, 0.4))));
        p.spawn((Text::new(ENDING_LINES[0]),
            TextFont { font_size: 24.0, ..default() }, TextColor(Color::srgb(0.95, 0.95, 1.0)), DialogueLine));
    });
}

// The princess's parting words — shown one after another.
const ENDING_LINES: [&str; 4] = [
    "The long night is over at last.",
    "You walked through shadow and dragonfire for us all.",
    "The realm will sing of you for a thousand years.",
    "Rest now, brave one. Your watch has ended.",
];

// Drive the ending: she speaks a few lines, then the view rises into the clouds
// while the credits roll — closing with a note that the game was made with Claude.
fn ending_sequence(
    time: Res<Time>,
    mut ending: ResMut<Ending>,
    mut commands: Commands,
    dialogue_q: Query<Entity, With<DialogueRoot>>,
    mut line_q: Query<&mut Text, With<DialogueLine>>,
    mut credits_q: Query<&mut Node, With<CreditsText>>,
    mut princess_q: Query<&mut Transform, (With<Princess>, Without<Player>, Without<PlayerCamera>)>,
    player_q: Query<&Transform, (With<Player>, Without<Princess>, Without<PlayerCamera>)>,
    mut camera_q: Query<&mut Transform, (With<PlayerCamera>, Without<Princess>, Without<Player>)>,
    mut held_vis: Query<&mut Visibility, (With<HeldVisual>, Without<ArtifactVisual>, Without<LightningOrb>)>,
    mut art_vis: Query<&mut Visibility, (With<ArtifactVisual>, Without<HeldVisual>, Without<LightningOrb>)>,
    mut orb_vis: Query<&mut Visibility, (With<LightningOrb>, Without<HeldVisual>, Without<ArtifactVisual>)>,
    mut exit: EventWriter<AppExit>,
) {
    if ending.stage == 0 { return; }
    let dt = time.delta_secs();
    ending.timer += dt;
    let pp = player_q.get_single().map(|t| t.translation).unwrap_or(Vec3::ZERO);

    match ending.stage {
        // ── Stage 1: she gently faces you and speaks several lines ──
        1 => {
            if let Ok(mut pt) = princess_q.get_single_mut() {
                // Face the player: the princess's front is +Z, so aim +Z at them
                // (look_at aims -Z, which would turn her back to the player).
                let dir = Vec3::new(pp.x - pt.translation.x, 0.0, pp.z - pt.translation.z).normalize_or_zero();
                if dir.length_squared() > 0.0001 {
                    pt.rotation = Quat::from_rotation_arc(Vec3::Z, dir);
                }
            }
            // Advance through the lines every ~3 seconds.
            let idx = ((ending.timer / 3.0) as usize).min(ENDING_LINES.len() - 1);
            if let Ok(mut text) = line_q.get_single_mut() {
                if text.0 != ENDING_LINES[idx] { text.0 = ENDING_LINES[idx].to_string(); }
            }
            if ending.timer > ENDING_LINES.len() as f32 * 3.0 {
                for e in dialogue_q.iter() { commands.entity(e).despawn_recursive(); }
                ending.stage = 2;
                ending.timer = 0.0;
                // Roll the credits — bright, scrolling up over the open sky & clouds
                let body = "\n\n\nA LEGEND CONCLUDES\n\n\n\n\n\nThis game was made entirely\nwith Claude Opus 4.8\n\n\n\n\n\nThe End";
                commands.spawn((
                    Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0),
                           overflow: Overflow::clip(), ..default() },
                    // soft, bright sky-tint so the clouds behind still show through
                    BackgroundColor(Color::srgba(0.62, 0.76, 0.97, 0.30)),
                )).with_children(|p| {
                    p.spawn((
                        Text::new(body),
                        TextFont { font_size: 34.0, ..default() },
                        TextColor(Color::srgb(1.0, 1.0, 1.0)),
                        TextLayout::new_with_justify(JustifyText::Center),
                        Node { position_type: PositionType::Absolute, left: Val::Percent(50.0), bottom: Val::Px(-620.0),
                               margin: UiRect::left(Val::Px(-280.0)), width: Val::Px(560.0), ..default() },
                        CreditsText,
                    ));
                });
            }
        }
        // ── Stage 2: rise into the heavens, credits scroll, then close ──
        _ => {
            // Hide the first-person hands & gear for a clean cinematic
            for mut v in held_vis.iter_mut() { *v = Visibility::Hidden; }
            for mut v in art_vis.iter_mut() { *v = Visibility::Hidden; }
            for mut v in orb_vis.iter_mut() { *v = Visibility::Hidden; }
            // Lift the camera skyward and tilt up toward the clouds.
            if let Ok(mut cam) = camera_q.get_single_mut() {
                cam.translation.y += dt * 4.0;
                let pitch = (ending.timer * 0.18).min(0.9);
                cam.rotation = Quat::from_rotation_x(pitch);
            }
            for mut node in credits_q.iter_mut() {
                let b = match node.bottom { Val::Px(v) => v, _ => -620.0 };
                node.bottom = Val::Px(b + dt * 55.0);
            }
            if ending.timer > 24.0 {
                exit.send(AppExit::Success);
            }
        }
    }
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
    // Layered eye materials: dark blood-red rim → fiery iris → white-hot centre
    let rim_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.05, 0.0), emissive: LinearRgba::new(3.5, 0.3, 0.0, 1.0), unlit: true, ..default() });
    let iris_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.7, 0.1), emissive: LinearRgba::new(11.0, 5.0, 0.5, 1.0), unlit: true, ..default() });

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

    // ── Vertical buttress fins running up the main shaft (gives it ribbed bulk) ──
    for i in 0..8u32 {
        let a = i as f32 / 8.0 * std::f32::consts::TAU;
        let r = 13.0;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(2.2, 128.0, 4.0))), MeshMaterial3d(obsidian.clone()),
            Transform::from_xyz(tx + a.cos() * r, 62.0, tz + a.sin() * r)
                .with_rotation(Quat::from_rotation_y(a)),
        ));
    }

    // ── Two colossal curved horns sweeping up & forward to frame the eye ──
    // The whole spire is turned a quarter-turn clockwise, so the horn sweep is
    // rotated -90° about its vertical axis.
    let spire_spin = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);
    for side in [-1.0f32, 1.0] {
        let pts: Vec<Vec3> = (0..=12).map(|s| {
            let u = s as f32 / 12.0;
            let y = 150.0 + u * 62.0;
            let out = (1.0 - u) * 15.0 + (u * std::f32::consts::PI).sin() * 9.0; // bow outward then back
            let fwd = (u * 1.6).sin() * 12.0;                                    // sweep forward over the eye
            let off = spire_spin * Vec3::new(side * out, 0.0, fwd);
            Vec3::new(tx + off.x, y, tz + off.z)
        }).collect();
        for w in pts.windows(2) {
            let (p0, p1) = (w[0], w[1]);
            let mid = (p0 + p1) * 0.5;
            let d = p1 - p0;
            let len = d.length().max(0.01);
            let taper = (1.0 - (p0.y - 150.0) / 62.0).clamp(0.12, 1.0);
            let wd = 5.2 * taper;
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(wd, wd, len * 1.08))), MeshMaterial3d(obsidian.clone()),
                Transform::from_translation(mid).with_rotation(Quat::from_rotation_arc(Vec3::Z, d / len)),
            ));
        }
        let tip = *pts.last().unwrap();
        commands.spawn((Mesh3d(meshes.add(Cone { radius: 1.5, height: 10.0 }.mesh().resolution(5))), MeshMaterial3d(obsidian.clone()),
            Transform::from_translation(tip + Vec3::Y * 3.5)));
    }

    // ── Jagged crown spikes ringing the horns' base ──
    let crown_spike = meshes.add(Cone { radius: 1.7, height: 15.0 }.mesh().resolution(5));
    for i in 0..10u32 {
        let a = i as f32 / 10.0 * std::f32::consts::TAU;
        let r = 9.0;
        commands.spawn((Mesh3d(crown_spike.clone()), MeshMaterial3d(obsidian.clone()),
            Transform::from_xyz(tx + a.cos() * r, 150.0, tz + a.sin() * r)
                .with_rotation(Quat::from_rotation_z(-a.cos() * 0.5) * Quat::from_rotation_x(a.sin() * 0.5))));
    }

    // ── Eye assembly (child-rotated by animate_eye to scan the horizon) ──
    // Raised well above the splayed spikes so it sits clear in the air
    let eye_root = commands.spawn((
        Transform::from_xyz(tx, 182.0, tz),
        GlobalTransform::default(),
        Visibility::default(),
        Eye { alert: 0.0 },
    )).id();

    // ── Horizontal almond eye (faces +Z): dark-red rim → fiery red eyeball →
    //    black iris → thin molten slit. (No more radiating "sunflower" wreath.) ──
    // Dark blood-red rim, wide almond, furthest back
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(rim_mat.clone()),
        Transform::from_xyz(0.0, 0.0, 5.6).with_scale(Vec3::new(16.0, 6.8, 1.6)),
        Visibility::default(),
    )).set_parent(eye_root);
    // Sharp almond points at the left & right corners
    for s in [-1.0f32, 1.0] {
        commands.spawn((
            Mesh3d(meshes.add(Cone { radius: 3.4, height: 7.0 }.mesh().resolution(4))),
            MeshMaterial3d(rim_mat.clone()),
            Transform::from_xyz(s * 15.0, 0.0, 6.0)
                .with_rotation(Quat::from_rotation_z(-s * std::f32::consts::FRAC_PI_2)),
            Visibility::default(),
        )).set_parent(eye_root);
    }

    // Fiery red eyeball — wide flattened ellipsoid
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(flame.clone()),
        Transform::from_xyz(0.0, 0.0, 7.0).with_scale(Vec3::new(13.5, 5.4, 2.0)),
        Visibility::default(),
    )).set_parent(eye_root);

    // Black iris in the centre (animate_eye narrows it when alert)
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(pupil_mat),
        Transform::from_xyz(0.0, 0.0, 8.6).with_scale(Vec3::new(5.6, 3.4, 1.3)),
        Visibility::default(),
        EyePupil,
    )).set_parent(eye_root);

    // Thin molten vertical slit at the very centre of the black iris
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.8, 5.0, 1.0))),
        MeshMaterial3d(iris_mat),
        Transform::from_xyz(0.0, 0.0, 9.3),
        Visibility::default(),
    )).set_parent(eye_root);

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
    let beam_mesh = meshes.add(Cuboid::new(0.073, 0.073, 1.0));
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

    // The whole spire is turned a quarter-turn clockwise — turn the eye to match.
    let spire_spin = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);
    // Idle sweep when calm, lock onto the player when alert
    let idle = Quat::from_rotation_y((t * 0.4).sin() * 0.7);
    if to.length_squared() > 0.001 {
        let dir = to.normalize();
        // +Z (eye face) points toward player → aim -Z away from player
        let track = Transform::from_translation(eye_pos).looking_to(-dir, Vec3::Y).rotation;
        // Calm rest pose carries the 90° spire turn; when locked on, it tracks the
        // player exactly (no offset) so it truly stares at them.
        tr.rotation = (spire_spin * idle).slerp(track, eye.alert);
    } else {
        tr.rotation = spire_spin * idle;
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

    // Black iris flicker — keeps its flattened almond shape; narrows a touch when alert
    for mut ptr in pupil_q.iter_mut() {
        let p = 1.0 + (t * 7.0).sin() * 0.05 + (t * 2.3).sin() * 0.03;
        ptr.scale = Vec3::new(5.6 * (1.0 - eye.alert * 0.22) * p, 3.4, 1.3);
    }
}

fn check_death(
    health: Res<PlayerHealth>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
) {
    if *state.get() == AppState::Playing && health.hp <= 0.0 {
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
    "be wary of dragon",
    "the king feeds the flame no longer",
    "ascend the cold stair, find the beacon",
    "the Eye remembers every sin",
    "here lies the last shieldbearer",
    "praise the sun \\[T]/",
    "treasure within the keep",
    "the wyrm was once our guardian",
    "no kindling left... only ash",
    "those who climb do not return",
    "the beacon still burns for the lost",
    "turn back, hollow one",
    "amazing chest ahead",
    "try jumping",
    "the fog drinks the brave",
    "a kingdom drowned in moonlight",
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
        BackgroundColor(Color::srgba(0.04, 0.04, 0.06, 0.72)),
        PauseScreen,
    )).with_children(|p| {
        p.spawn((
            Text::new("PAUSED"),
            TextFont { font_size: 90.0, ..default() },
            TextColor(Color::srgb(0.92, 0.92, 0.95)),
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
    mut petrify: ResMut<Petrify>,
    mut arrival: ResMut<DragonArrival>,
    mut kills: ResMut<KillStats>,
    (mut fight, mut areas, mut warp, mut mystic, mut hut_talk, mut riddle):
        (ResMut<SauronFight>, ResMut<Areas>, ResMut<Warp>, ResMut<MysticTalk>, ResMut<HutTalk>, ResMut<BladeRiddle>),
    realm: Res<Realm>,
    (mut clear, mut ambient, mut sun_q, mut fog_q):
        (ResMut<ClearColor>, ResMut<AmbientLight>, Query<&mut DirectionalLight>, Query<&mut DistanceFog, With<PlayerCamera>>),
    mut player_q: Query<(&mut Transform, &mut PlayerVelocity), With<Player>>,
    mut commands: Commands,
    clutter: Query<Entity, Or<(With<Skeleton>, With<Dragon>, With<Fireball>, With<Enemy>,
                               With<Pickup>, With<Rocket>, With<MagicMissile>, With<Debris>, With<Transient>,
                               With<DragonLaser>, With<Shockwave>, With<FirePatch>,
                               Or<(With<Medusa>, With<MedusaBolt>, With<StoneWave>, With<MeteorShock>,
                                   With<Arrow>, With<CountdownText>, With<PetrifyOverlay>,
                                   With<SauronArena>, With<ReturnPortal>, With<SauronHpBar>)>)>>,
    realm_clutter: Query<Entity, Or<(With<ShadowProp>, With<HutProp>, With<ShadowRain>, With<MistZombie>,
                                     With<BlackHoleCore>, With<BlackHoleProj>, With<BladeRiddlePanel>,
                                     With<MysticProp>, With<AreaTitle>, With<VoidScar>, With<BlackHoleParticle>,
                                     With<BlackHoleRing>, With<BlackHoleDebris>, With<BlackHoleBlast>)>>,
) {
    // If the player fell in a special realm (Sauron's arena / Shadow Isles / Hut),
    // abandon it and restore the waking overworld.
    if (fight.active || areas.in_shadow || areas.in_hut) && !realm.in_sky {
        clear.0 = Color::srgb(0.018, 0.025, 0.06);
        ambient.color = Color::srgb(0.32, 0.37, 0.58);
        ambient.brightness = 416.0;
        if let Ok(mut sun) = sun_q.get_single_mut() { sun.color = Color::srgb(0.62, 0.72, 1.0); sun.illuminance = 2860.0; }
        if let Ok(mut fog) = fog_q.get_single_mut() { fog.color = Color::srgb(0.04, 0.05, 0.11); fog.falloff = FogFalloff::Linear { start: 110.0, end: 1150.0 }; }
    }
    if fight.active { fight.active = false; fight.spawned = false; fight.engaged = false; }
    // Reset the realm/warp bookkeeping (the built areas are torn down below)
    areas.in_shadow = false; areas.in_hut = false; areas.shadow_built = false; areas.hut_built = false;
    warp.stage = 0; warp.timer = 0.0; warp.dest = 0;
    mystic.stage = 0; mystic.timer = 0.0; mystic.line = 0;
    hut_talk.active = false; hut_talk.line = 0;
    riddle.active = false;
    for e in realm_clutter.iter() { commands.entity(e).despawn_recursive(); }
    health.hp = health.max_hp;
    health.golden = 0.0;
    health.golden_timer = 0.0;
    health.blocking = false;
    health.hurt_timer = 0.0;
    health.iframes = 0.0;
    stamina.current = stamina.max;
    mana.current = mana.max;
    petrify.timer = 0.0;
    *kills = KillStats::default();
    *arrival = DragonArrival { counting: false, countdown: 0.0, spawn_now: false, spawned: false, pos: Vec3::ZERO, target: Vec3::ZERO };
    *inv = Inventory { selected: ItemKind::Sword, health_potions: 0, mana_potions: 0, has_glock: false, has_rocket: false, has_bow: false, has_ruined_blade: inv.has_ruined_blade };
    let (mut t, mut v) = player_q.single_mut();
    *t = Transform::from_xyz(0.0, 0.0, 18.0);
    v.vertical = 0.0;
    v.knockback = Vec3::ZERO;
    v.roll_timer = 0.0;
    // Clear all enemies, pickups, and live projectiles — fresh start
    for e in clutter.iter() { commands.entity(e).despawn_recursive(); }
}

// Continuous jagged lightning beam from the eye to the player while in range.
fn eye_beam(
    time: Res<Time>,
    fight: Res<SauronFight>,
    eye_q: Query<&GlobalTransform, With<Eye>>,
    player_q: Query<&Transform, (With<Player>, Without<EyeBeamSeg>)>,
    mut seg_q: Query<(&mut Transform, &mut Visibility, &EyeBeamSeg)>,
    mut player_vel: Query<&mut PlayerVelocity, With<Player>>,
    mut health: ResMut<PlayerHealth>,
    mut sword_glow: ResMut<SwordGlow>,
    sword_q: Query<&GlobalTransform, With<Sword>>,
) {
    // Once the Dark Lord is slain (or while inside his arena) the Eye stops firing.
    if fight.defeated || fight.active {
        for (_, mut vis, _) in seg_q.iter_mut() { *vis = Visibility::Hidden; }
        return;
    }
    let Ok(eye_gt) = eye_q.get_single() else { return; };
    let pt = player_q.single();
    let target = pt.translation + Vec3::Y * 1.0;

    let center = eye_gt.translation();
    let aim = (target - center).normalize_or_zero();
    let eye_pos = center + aim * 12.0;            // beam origin just in front of the eye
    let horiz = Vec3::new(target.x - center.x, 0.0, target.z - center.z).length();
    // Stop firing once the player presses in close to the tower (2x the old radius).
    let active = horiz < 220.0 && horiz > 52.0 && (target - eye_pos).length() > 1.0;

    if !active {
        for (_, mut vis, _) in seg_q.iter_mut() { *vis = Visibility::Hidden; }
        return;
    }

    // Are we deflecting? (block while roughly facing the spire.) If so, the beam
    // terminates at the raised SWORD, not the player's body.
    let pfwd = *pt.forward();
    let to_eye = Vec3::new(center.x - target.x, 0.0, center.z - target.z).normalize_or_zero();
    let facing = Vec3::new(pfwd.x, 0.0, pfwd.z).normalize_or_zero().dot(to_eye) > 0.1;
    let deflecting = health.blocking && facing;
    let beam_end = if deflecting {
        sword_q.get_single().map(|g| g.translation() + *g.up() * 0.4).unwrap_or(target)
    } else { target };

    // Perpendicular basis for sideways jitter
    let dir = (beam_end - eye_pos).normalize_or_zero();
    let mut perp1 = dir.cross(Vec3::Y);
    if perp1.length_squared() < 1e-4 { perp1 = dir.cross(Vec3::X); }
    perp1 = perp1.normalize();
    let perp2 = dir.cross(perp1).normalize();

    let n = 14u32;
    let tt = (time.elapsed_secs() * 26.0).floor() / 26.0; // crackle snap
    // One continuous thin jagged path (anchored at both ends)
    let point = |k: u32| -> Vec3 {
        let f = k as f32 / n as f32;
        let along = eye_pos.lerp(beam_end, f);
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

    if deflecting {
        // Beam ends at the sword; the blade flares red & sparks (sword_glow_anim). Unhurt.
        sword_glow.timer = 0.12;
        return;
    }

    // Continuous damage tick while the beam connects
    if health.hurt_timer <= 0.0 && health.iframes <= 0.0 {
        health.take(20.0, 0.9);
        let mut pvel = player_vel.single_mut();
        let knock = Vec3::new(target.x - center.x, 0.0, target.z - center.z).normalize_or_zero();
        pvel.knockback = knock * 7.0;
    }
}
