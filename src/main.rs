mod bindings;
mod camera;
mod differed;
mod editor;
mod gltf;
mod instance;
mod mesh;
mod node;
mod renderer;
mod transform;
mod ui;
mod ui_pipelines;
mod world;

use editor::Editor;
use futures::executor::block_on;
use glam::*;
use renderer::{Frame, Renderer};
use ui::{UiMesh, UiVertex};
use winit::{
    event::{
        ElementState, Event, ModifiersState, MouseButton, StartCause, VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use world::Resources;

#[inline]
fn to_modifiers(modifiers: ModifiersState) -> egui::Modifiers {
    egui::Modifiers {
        alt: modifiers.alt(),
        ctrl: modifiers.ctrl(),
        shift: modifiers.shift(),
        command: modifiers.logo(),
        mac_cmd: modifiers.logo(),
    }
}

#[inline]
fn to_vec(vec: Vec2) -> egui::Vec2 {
    egui::Vec2::new(vec.x, vec.y)
}

#[inline]
fn to_pos(vec: Vec2) -> egui::Pos2 {
    egui::Pos2::new(vec.x, vec.y)
}

fn main() -> anyhow::Result<()> {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .with_module_level("wgpu", log::LevelFilter::Warn)
        .with_module_level("naga", log::LevelFilter::Warn)
        .with_module_level("gfx", log::LevelFilter::Warn)
        .init()
        .unwrap();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Phil Noire")
        .build(&event_loop)
        .expect("failed to create window");

    let size = window.inner_size();
    let (instance, mut swap_chain) =
        block_on(instance::Instance::new(&window, size.width, size.height))?;

    let mut renderer = Renderer::new(&instance, swap_chain.format(), size.width, size.height);

    let mut resources = Resources::new(&&instance);
    resources.load_assets("assets")?;

    let mut editor = Editor::new();
    editor.input.screen_rect = Some(egui::Rect::from_min_size(
        Default::default(),
        egui::Vec2::new(size.width as f32, size.height as f32),
    ));

    let mut world = resources.get_world("assets/office.world").cloned().unwrap();

    let mut aspect = size.width as f32 / size.height as f32;
    let mut cursor_position = Vec2::ZERO;
    let mut egui_texture = None;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::RedrawRequested(_) | Event::NewEvents(StartCause::Poll) => {
                let frame = editor.ctx.begin_frame(editor.input.take());

                if let Some(texture) = &mut egui_texture {
                } else {
                    egui_texture = Some(editor.texture(&instance));
                }

                editor.ui(&mut world);

                let (output, shapes) = editor.ctx.end_frame();
                let clipped_meshes = editor.ctx.tessellate(shapes);

                world.update(&resources);

                let frame = swap_chain
                    .next_frame()
                    .expect("failed to acquire next frame");

                let mut render_frame = Frame::new();

                render_frame.aspect = aspect;

                world.render(&resources, &mut render_frame);

                let mut ui_meshes = Vec::new();

                for egui::ClippedMesh(_, mesh) in clipped_meshes {
                    let mut vertices = Vec::with_capacity(mesh.vertices.len());

                    for vertex in mesh.vertices {
                        let rgba = egui::Rgba::from(vertex.color);
                        let color = Vec4::new(rgba.r(), rgba.g(), rgba.b(), rgba.a());

                        vertices.push(UiVertex {
                            position: Vec2::new(vertex.pos.x, vertex.pos.y),
                            uv: Vec2::new(vertex.uv.x, vertex.uv.y),
                            color,
                        });
                    }

                    ui_meshes.push(UiMesh {
                        vertices,
                        indices: mesh.indices,
                    });
                }

                for mesh in &ui_meshes {
                    render_frame.render_ui_mesh(mesh, egui_texture.as_ref().unwrap());
                }

                renderer.render_frame(&instance, &frame.output.view, render_frame);
            }
            Event::WindowEvent {
                event,
                window_id: _,
            } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(size) => {
                    aspect = size.width as f32 / size.height as f32;

                    swap_chain.resize(&instance, size.width, size.height);
                    renderer.resize(&instance, size.width, size.height);

                    editor.input.screen_rect = Some(egui::Rect::from_min_size(
                        Default::default(),
                        egui::Vec2::new(size.width as f32, size.height as f32),
                    ));
                }
                WindowEvent::ScaleFactorChanged {
                    new_inner_size: size,
                    ..
                } => {
                    aspect = size.width as f32 / size.height as f32;

                    swap_chain.resize(&instance, size.width, size.height);
                    renderer.resize(&instance, size.width, size.height);

                    editor.input.screen_rect = Some(egui::Rect::from_min_size(
                        Default::default(),
                        egui::Vec2::new(size.width as f32, size.height as f32),
                    ));
                }
                WindowEvent::CursorMoved { position, .. } => {
                    cursor_position = Vec2::new(position.x as f32, position.y as f32);

                    editor
                        .input
                        .events
                        .push(egui::Event::PointerMoved(to_pos(cursor_position)));
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    let button = match button {
                        MouseButton::Left => Some(egui::PointerButton::Primary),
                        MouseButton::Right => Some(egui::PointerButton::Secondary),
                        MouseButton::Middle => Some(egui::PointerButton::Middle),
                        _ => None,
                    };

                    if let Some(button) = button {
                        let pressed = state == ElementState::Pressed;

                        editor.input.events.push(egui::Event::PointerButton {
                            pos: to_pos(cursor_position),
                            button,
                            pressed,
                            modifiers: editor.input.modifiers,
                        });
                    }
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(keycode) = input.virtual_keycode {
                        let key = match keycode {
                            VirtualKeyCode::Escape => Some(egui::Key::Escape),
                            VirtualKeyCode::Tab => Some(egui::Key::Tab),
                            VirtualKeyCode::Space => Some(egui::Key::Space),
                            VirtualKeyCode::Insert => Some(egui::Key::Insert),
                            VirtualKeyCode::Delete => Some(egui::Key::Delete),
                            VirtualKeyCode::Home => Some(egui::Key::Home),
                            VirtualKeyCode::End => Some(egui::Key::End),
                            VirtualKeyCode::PageUp => Some(egui::Key::PageUp),
                            VirtualKeyCode::PageDown => Some(egui::Key::PageDown),
                            VirtualKeyCode::Back => Some(egui::Key::Backspace),
                            VirtualKeyCode::Return => Some(egui::Key::Enter),
                            VirtualKeyCode::Left => Some(egui::Key::ArrowLeft),
                            VirtualKeyCode::Right => Some(egui::Key::ArrowRight),
                            VirtualKeyCode::Up => Some(egui::Key::ArrowUp),
                            VirtualKeyCode::Down => Some(egui::Key::ArrowDown),
                            VirtualKeyCode::Key0 => Some(egui::Key::Num0),
                            VirtualKeyCode::Key1 => Some(egui::Key::Num1),
                            VirtualKeyCode::Key2 => Some(egui::Key::Num2),
                            VirtualKeyCode::Key3 => Some(egui::Key::Num3),
                            VirtualKeyCode::Key4 => Some(egui::Key::Num4),
                            VirtualKeyCode::Key5 => Some(egui::Key::Num5),
                            VirtualKeyCode::Key6 => Some(egui::Key::Num6),
                            VirtualKeyCode::Key7 => Some(egui::Key::Num7),
                            VirtualKeyCode::Key8 => Some(egui::Key::Num8),
                            VirtualKeyCode::Key9 => Some(egui::Key::Num9),
                            VirtualKeyCode::A => Some(egui::Key::A),
                            VirtualKeyCode::B => Some(egui::Key::B),
                            VirtualKeyCode::C => Some(egui::Key::C),
                            VirtualKeyCode::D => Some(egui::Key::D),
                            VirtualKeyCode::E => Some(egui::Key::E),
                            VirtualKeyCode::F => Some(egui::Key::F),
                            VirtualKeyCode::G => Some(egui::Key::G),
                            VirtualKeyCode::H => Some(egui::Key::H),
                            VirtualKeyCode::I => Some(egui::Key::I),
                            VirtualKeyCode::J => Some(egui::Key::J),
                            VirtualKeyCode::K => Some(egui::Key::K),
                            VirtualKeyCode::L => Some(egui::Key::L),
                            VirtualKeyCode::M => Some(egui::Key::M),
                            VirtualKeyCode::N => Some(egui::Key::N),
                            VirtualKeyCode::O => Some(egui::Key::O),
                            VirtualKeyCode::P => Some(egui::Key::P),
                            VirtualKeyCode::Q => Some(egui::Key::Q),
                            VirtualKeyCode::R => Some(egui::Key::R),
                            VirtualKeyCode::S => Some(egui::Key::S),
                            VirtualKeyCode::T => Some(egui::Key::T),
                            VirtualKeyCode::U => Some(egui::Key::U),
                            VirtualKeyCode::V => Some(egui::Key::V),
                            VirtualKeyCode::W => Some(egui::Key::W),
                            VirtualKeyCode::X => Some(egui::Key::X),
                            VirtualKeyCode::Y => Some(egui::Key::Y),
                            VirtualKeyCode::Z => Some(egui::Key::Z),
                            _ => None,
                        };

                        if let Some(key) = key {
                            let pressed = input.state == ElementState::Pressed;

                            editor.input.events.push(egui::Event::Key {
                                pressed,
                                key,
                                modifiers: editor.input.modifiers,
                            });
                        }
                    }
                }
                WindowEvent::ModifiersChanged(modifiers) => {
                    editor.input.modifiers = to_modifiers(modifiers);
                }
                WindowEvent::ReceivedCharacter(c) => {
                    if !c.is_control() {
                        editor.input.events.push(egui::Event::Text(String::from(c)));
                    }
                }
                _ => {}
            },
            _ => {}
        }
    });
}
