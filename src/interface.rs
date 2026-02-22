use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use crate::emulator;
use crate::emulator::*;
use crate::instruction_set::*;
use crate::primitives::*;
use bevy::window::PresentMode;
use bevy::window::WindowResolution;
use bevy::{
    input::{keyboard::KeyboardInput, ButtonState},
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};
use iyes_perf_ui::{entries::PerfUiBundle, PerfUiPlugin};

pub fn start(emulator: Emulator) {
    App::default()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Emu".into(),
                        resizable: false,
                        // resolution: WindowResolution::new(1920., 1080.),
                        present_mode: PresentMode::Immediate,
                        ..Default::default()
                    }),

                    ..Default::default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins((
            bevy::diagnostic::EntityCountDiagnosticsPlugin,
            bevy::diagnostic::SystemInformationDiagnosticsPlugin,
            bevy::diagnostic::FrameTimeDiagnosticsPlugin,
            PerfUiPlugin,
        ))
        .insert_resource(Emu(emulator))
        .insert_resource(Profiler::default())
        .add_systems(Startup, startup)
        .add_systems(Update, (close_on_escape, update_buttons, exit).chain())
        .add_systems(
            PostUpdate,
            (
                display_registers,
                display_instructions.run_if(resource_exists::<StepTimer>),
                display_console,
                display_screen,
                display_buttons,
            ),
        )
        .add_systems(FixedUpdate, (update_tick, step).chain())
        .insert_resource(Time::<Fixed>::from_seconds(1. / 60.))
        // .insert_resource(StepTimer(Timer::from_seconds(0.05, TimerMode::Repeating)))
        .run();
}

fn update_tick(mut emulator: ResMut<Emu>) {
    emulator.0.tick();
}

#[derive(Resource)]
struct Emu(Emulator);

fn startup(
    mut commands: Commands,
    window: Query<&Window>,
    mut textures: ResMut<Assets<Image>>,
    emulator: Res<Emu>,
) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn(PerfUiBundle::default());

    let window = window.single();

    // Registers
    commands
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            for i in 0..32 {
                parent.spawn((
                    TextBundle::from_section(
                        format!("x{}", i),
                        TextStyle {
                            font_size: (window.resolution.physical_height() / 10) as f32,
                            ..default()
                        },
                    ),
                    Register(Reg::new(i)),
                ));
            }
        });

    // Instructions
    commands
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::End,
                position_type: PositionType::Absolute,
                right: Val::Percent(40.),
                top: Val::Percent(5.),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            for _ in 0..5 {
                parent.spawn((
                    TextBundle::from_section(
                        "",
                        TextStyle {
                            font_size: (window.resolution.physical_height() / 10) as f32,
                            ..default()
                        },
                    ),
                    Instruction(Instr::Ecall),
                ));
            }
        });

    // Buttons
    commands
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::End,
                position_type: PositionType::Absolute,
                left: Val::Percent(90.),
                bottom: Val::Percent(5.),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            for i in 0..4 {
                parent.spawn((
                    TextBundle::from_section(
                        format!("B{}", i),
                        TextStyle {
                            font_size: (window.resolution.physical_height() / 20) as f32,
                            ..default()
                        },
                    ),
                    Button(crate::io::Button::new(i)),
                ));
            }
        });

    // Console
    commands.spawn((
        TextBundle::from_section(
            "",
            TextStyle {
                font_size: (window.resolution.physical_height() / 32) as f32,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Percent(30.),
            bottom: Val::Percent(0.),
            ..Default::default()
        }),
        Console,
    ));

    // Screen
    let im = Image::new(
        Extent3d {
            width: 320,
            height: 200,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        emu_screen_to_texture_data(&emulator),
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::all(),
    );
    let texture = textures.add(im);
    commands.spawn((
        Screen(texture.clone()),
        SpriteBundle {
            texture,
            transform: Transform::from_scale(Vec3::splat(3.)),
            ..Default::default()
        },
    ));
}

#[derive(Resource)]
struct StepTimer(Timer);

#[derive(Resource)]
struct Profiler {
    frame: Duration,
    bevy: SystemTime,
}

impl Default for Profiler {
    fn default() -> Self {
        Self {
            frame: Duration::default(),
            bevy: SystemTime::now(),
        }
    }
}

fn step(
    mut emulator: ResMut<Emu>,
    mut writer: EventWriter<AppExit>,
    mut timer: Option<ResMut<StepTimer>>,
    // mut prof: ResMut<Profiler>,
    // mut window: Query<&mut Window>,
    time: Res<Time>,
) {
    if let Some(timer) = &mut timer {
        timer.0.tick(time.delta());

        if timer.0.just_finished() {
            emulator.0.run_next();
        }

        if emulator.0.finished() {
            writer.send(AppExit::Success);
        }
    } else {
        // let bevy_time = SystemTime::now()
        //     .duration_since(prof.bevy)
        //     .unwrap_or_default();
        // let start = SystemTime::now();
        while !emulator.0.should_render() {
            emulator.0.run_next();
            if emulator.0.finished() {
                writer.send(AppExit::Success);
                break;
            }
        }
        // prof.frame = SystemTime::now().duration_since(start).unwrap();
        // prof.bevy = SystemTime::now();
        //
        // let mut window = window.single_mut();
        // window.title = format!(
        //     "Emu - {}ms - {}ms",
        //     prof.frame.as_millis(),
        //     bevy_time.as_millis()
        // );
    }
}

#[derive(Component)]
struct Register(Reg);

fn display_registers(
    mut regs: Query<(&mut Text, &Register)>,
    window: Query<&Window>,
    emulator: Res<Emu>,
) {
    let window = window.single();

    for (mut text, reg) in regs.iter_mut() {
        let padding = if reg.0.reg_index() < 10 { " " } else { "" };
        text.sections[0].style.font_size = (window.resolution.physical_height() / 64) as f32;
        text.sections[0].value = format!(
            "x{}    {}{:#018x}",
            reg.0.reg_index(),
            padding,
            emulator.0.reg(reg.0)
        );
    }
}

#[derive(Component)]
struct Instruction(Instr);

fn display_instructions(
    mut instrs: Query<(&mut Text, &mut Instruction)>,
    window: Query<&Window>,
    emulator: Res<Emu>,
    timer: ResMut<StepTimer>,
) {
    if timer.0.just_finished() {
        let window = window.single();

        let mut tmp1 = *emulator.0.current_instruction();
        let mut tmp2;
        for (mut text, mut instr) in instrs.iter_mut() {
            text.sections[0].style.font_size = (window.resolution.physical_height() / 32) as f32;
            tmp2 = instr.0;
            instr.0 = tmp1;
            tmp1 = tmp2;
            text.sections[0].value = format!("{:?}", instr.0,);
        }
    }
}

#[derive(Component)]
struct Button(crate::io::Button);

fn update_buttons(mut reader: EventReader<KeyboardInput>, mut emulator: ResMut<Emu>) {
    for input in reader.read() {
        match input.state {
            ButtonState::Pressed => match input.key_code {
                KeyCode::KeyW => emulator.0.press_button(crate::io::Button::Zero),
                KeyCode::KeyA => emulator.0.press_button(crate::io::Button::One),
                KeyCode::KeyS => emulator.0.press_button(crate::io::Button::Two),
                KeyCode::KeyD => emulator.0.press_button(crate::io::Button::Three),
                _ => {}
            },
            ButtonState::Released => match input.key_code {
                KeyCode::KeyW => emulator.0.release_button(crate::io::Button::Zero),
                KeyCode::KeyA => emulator.0.release_button(crate::io::Button::One),
                KeyCode::KeyS => emulator.0.release_button(crate::io::Button::Two),
                KeyCode::KeyD => emulator.0.release_button(crate::io::Button::Three),
                _ => {}
            },
        }
    }
}

fn display_buttons(
    mut instrs: Query<(&mut Text, &Button)>,
    window: Query<&Window>,
    emulator: Res<Emu>,
) {
    let window = window.single();

    for (mut text, button) in instrs.iter_mut() {
        if emulator.0.button(button.0) {
            text.sections[0].style.color = Color::xyz(1., 1., 1.);
        } else {
            text.sections[0].style.color = Color::xyz(0.2, 0.2, 0.2);
        }
    }
}

#[derive(Component)]
struct Console;

fn display_console(mut console: Query<&mut Text, With<Console>>, emulator: Res<Emu>) {
    let mut console = console.single_mut();
    console.sections[0].value = String::from_utf8_lossy(emulator.0.console()).into();
}

#[derive(Component)]
struct Screen(Handle<Image>);

fn display_screen(screen: Query<&Screen>, mut images: ResMut<Assets<Image>>, emulator: Res<Emu>) {
    let screen = screen.single();
    let image = images.get_mut(&screen.0).unwrap();
    image.data = emu_screen_to_texture_data(&emulator);
}

fn emu_screen_to_texture_data(emulator: &Emu) -> Vec<u8> {
    emulator
        .0
        .memory(SCREEN_OFFSET, SCREEN_SIZE / 8)
        .iter()
        .map(|byte| {
            let mut bit_field = [0; 8];
            for i in 0..8 {
                bit_field[i] = (byte >> (7 - i)) & 1;
            }

            let mut pixels = [0; 32];
            let mut i = 0;
            for bit in bit_field.iter() {
                if *bit > 0 {
                    pixels[i + 0] = 0xFF;
                    pixels[i + 1] = 0xFF;
                    pixels[i + 2] = 0xFF;
                    pixels[i + 3] = 0xFF;
                } else {
                    pixels[i + 0] = 0x0;
                    pixels[i + 1] = 0x0;
                    pixels[i + 2] = 0x0;
                    pixels[i + 3] = 0xFF;
                }

                i += 4;
            }

            pixels
        })
        .flatten()
        .collect()
}

fn close_on_escape(mut reader: EventReader<KeyboardInput>, mut writer: EventWriter<AppExit>) {
    for input in reader.read() {
        if input.key_code == KeyCode::Escape && input.state == ButtonState::Pressed {
            writer.send(AppExit::Success);
        }
    }
}

fn exit(mut reader: EventReader<AppExit>, emulator: Res<Emu>) {
    if reader.read().next().is_some() {
        crate::emulator::print_emulator(&emulator.0);
    }
}
