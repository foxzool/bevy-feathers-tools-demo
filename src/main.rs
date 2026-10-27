//! 用 Feathers 给 3D 场景做一块调试面板（Bevy 0.20.0）。
//! 运行：cargo run
//! 截图：DEMO_SCREENSHOT=out.png cargo run（渲染若干帧后保存并退出）

use bevy::{
    feathers::{
        controls::*,
        dark_theme::create_dark_theme,
        display::{caption, label},
        theme::{ThemeBackgroundColor, UiTheme},
        tokens, FeathersPlugins,
    },
    input_focus::tab_navigation::TabGroup,
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot},
    ui::Checked,
    ui_widgets::{slider_self_update, Activate, SliderPrecision, SliderStep, SliderValue, ValueChange},
};

/// 面板要调的参数，全部放在一个 Resource 里。
#[derive(Resource)]
struct DebugSettings {
    spin: bool,
    speed: f32,
    light_lux: f32,
    color: Color,
    shape: usize,
}

impl Default for DebugSettings {
    fn default() -> Self {
        Self {
            spin: true,
            speed: 1.0,
            light_lux: 2000.0,
            color: Color::srgb(0.85, 0.45, 0.1),
            shape: 0,
        }
    }
}

#[derive(Component)]
struct Target;

#[derive(Component)]
struct KeyLight;

#[derive(Resource)]
struct Meshes3([Handle<Mesh>; 3]);

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .insert_resource(UiTheme(create_dark_theme()))
        .init_resource::<DebugSettings>()
        .add_systems(Startup, (setup_world, panel.spawn()))
        .add_systems(Update, (apply_settings, spin_target, auto_screenshot))
        .run();
}

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<DebugSettings>,
) {
    let shapes = [
        meshes.add(Cuboid::new(1.5, 1.5, 1.5)),
        meshes.add(Sphere::new(1.0)),
        meshes.add(Torus::new(0.5, 1.0)),
    ];
    commands.spawn((
        Mesh3d(shapes[0].clone()),
        MeshMaterial3d(materials.add(settings.color)),
        Transform::from_xyz(1.2, 0.0, 0.0),
        Target,
    ));
    commands.insert_resource(Meshes3(shapes));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(12.0, 12.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.25, 0.27, 0.3))),
        Transform::from_xyz(0.0, -1.2, 0.0),
    ));
    commands.spawn((
        DirectionalLight { illuminance: settings.light_lux, shadow_maps_enabled: true, ..default() },
        Transform::from_xyz(3.0, 6.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        KeyLight,
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-1.0, 2.5, 6.0).looking_at(Vec3::new(0.6, 0.0, 0.0), Vec3::Y),
    ));
}

fn panel() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            top: px(12),
            width: px(260),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            padding: px(12),
        }
        TabGroup
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children [
            @label("Debug Panel")
            --
            @FeathersCheckbox { @caption: bsn! { @caption("Spin") } }
            Checked
            on(|change: On<ValueChange<bool>>, mut s: ResMut<DebugSettings>, mut commands: Commands| {
                s.spin = change.value;
                let mut checkbox = commands.entity(change.source);
                if change.value { checkbox.insert(Checked); } else { checkbox.remove::<Checked>(); }
            })
            --
            @label("Speed")
            --
            @FeathersSlider { @max: 5.0 }
            SliderValue(1.0)
            SliderStep(0.1)
            SliderPrecision(1)
            on(slider_self_update)
            on(|change: On<ValueChange<f32>>, mut s: ResMut<DebugSettings>| {
                s.speed = change.value;
            })
            --
            @label("Light (lux)")
            --
            @FeathersNumberInput
            NumberInputValue::F32(2000.0)
            on(|change: On<ValueChange<f32>>, mut s: ResMut<DebugSettings>, mut commands: Commands| {
                s.light_lux = change.value.max(0.0);
                commands.entity(change.event_target())
                    .insert(NumberInputValue::F32(change.value));
            })
            --
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
            }
            Children [
                @label("Color")
                --
                @FeathersColorInput
                ColorInputValue(Color::srgb(0.85, 0.45, 0.1))
                on(color_input_self_update)
                on(|change: On<ValueChange<Color>>, mut s: ResMut<DebugSettings>| {
                    s.color = change.value;
                })
            ]
            --
            @label("Shape")
            --
            @FeathersSelect {
                @options: list_rows_from_strings(["Cube", "Sphere", "Torus"], Some(0)),
            }
            on(|change: On<ValueChange<Entity>>, q: Query<&OptionIndex>, mut s: ResMut<DebugSettings>| {
                if let Ok(option) = q.get(change.value) {
                    s.shape = option.0;
                }
            })
            --
            @FeathersButton
            on(|_: On<Activate>, mut q: Query<&mut Transform, With<Target>>| {
                for mut t in &mut q { t.rotation = Quat::IDENTITY; }
            })
            Children [ @caption("Reset rotation") ]
        ]
    }
}

fn apply_settings(
    s: Res<DebugSettings>,
    shapes: Res<Meshes3>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut target: Query<(&mut Mesh3d, &MeshMaterial3d<StandardMaterial>), With<Target>>,
    mut light: Query<&mut DirectionalLight, With<KeyLight>>,
) {
    if !s.is_changed() {
        return;
    }
    for (mut mesh, mat) in &mut target {
        mesh.0 = shapes.0[s.shape.min(2)].clone();
        if let Some(mut m) = materials.get_mut(&mat.0) {
            m.base_color = s.color;
        }
    }
    for mut l in &mut light {
        l.illuminance = s.light_lux;
    }
}

fn spin_target(time: Res<Time>, s: Res<DebugSettings>, mut q: Query<&mut Transform, With<Target>>) {
    if !s.spin {
        return;
    }
    for mut t in &mut q {
        t.rotate_y(s.speed * time.delta_secs());
    }
}

/// 设置 DEMO_SCREENSHOT=路径 时截图后退出。
/// DEMO_SHOT_FRAME / DEMO_EXIT_FRAME 可改帧号（默认 120 / 180）。
fn auto_screenshot(mut frame: Local<u32>, mut commands: Commands, mut exit: MessageWriter<AppExit>) {
    let Ok(path) = std::env::var("DEMO_SCREENSHOT") else { return };
    *frame += 1;
    let shot_at: u32 = std::env::var("DEMO_SHOT_FRAME").ok().and_then(|v| v.parse().ok()).unwrap_or(120);
    if *frame == shot_at {
        info!("saving screenshot to {path}");
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
    }
    let exit_at: u32 = std::env::var("DEMO_EXIT_FRAME").ok().and_then(|v| v.parse().ok()).unwrap_or(180);
    if *frame == exit_at {
        exit.write(AppExit::Success);
    }
}
