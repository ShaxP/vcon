use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use vcon_engine::InputFrame;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::window::WindowAttributes;

use crate::python_host::{FrameObserver, InputProvider};

struct WindowRuntimeState {
    event_loop: EventLoop<()>,
    window: Arc<winit::window::Window>,
    presenter: WgpuWindowPresenter,
    keys_down: HashSet<KeyCode>,
    close_requested: bool,
    target_frame_duration: Duration,
    last_present_at: Option<Instant>,
}

impl WindowRuntimeState {
    fn key_down(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    fn pump_events(&mut self) {
        let keys_down = &mut self.keys_down;
        let close_requested = &mut self.close_requested;
        let presenter = &mut self.presenter;
        let window_id = self.window.id();

        #[allow(deprecated)]
        let status = self
            .event_loop
            .pump_events(Some(Duration::ZERO), |event, _event_loop| {
                if let Event::WindowEvent {
                    event,
                    window_id: event_window_id,
                } = event
                {
                    if event_window_id != window_id {
                        return;
                    }

                    match event {
                        WindowEvent::CloseRequested => {
                            *close_requested = true;
                        }
                        WindowEvent::KeyboardInput { event, .. } => {
                            if let PhysicalKey::Code(code) = event.physical_key {
                                match event.state {
                                    ElementState::Pressed => {
                                        keys_down.insert(code);
                                    }
                                    ElementState::Released => {
                                        keys_down.remove(&code);
                                    }
                                }
                            }
                        }
                        WindowEvent::Resized(size) => {
                            if size.width > 0 && size.height > 0 {
                                presenter.resize(size.width, size.height);
                            }
                        }
                        _ => {}
                    }
                }
            });

        if matches!(status, PumpStatus::Exit(_)) {
            self.close_requested = true;
        }
    }

    fn should_continue(&self) -> bool {
        !self.close_requested && !self.key_down(KeyCode::Escape)
    }

    fn throttle(&mut self) {
        if self.target_frame_duration.is_zero() {
            return;
        }

        if let Some(last) = self.last_present_at {
            let elapsed = last.elapsed();
            if elapsed < self.target_frame_duration {
                std::thread::sleep(self.target_frame_duration - elapsed);
            }
        }
        self.last_present_at = Some(Instant::now());
    }
}

pub struct WindowInputProvider {
    state: Rc<RefCell<WindowRuntimeState>>,
}

pub struct WindowFrameObserver {
    state: Rc<RefCell<WindowRuntimeState>>,
}

pub fn create_window_runtime(
    title: &str,
    width: u32,
    height: u32,
    target_fps: u32,
) -> Result<(WindowInputProvider, WindowFrameObserver)> {
    let event_loop = EventLoop::new().context("failed to create winit event loop")?;
    #[allow(deprecated)]
    let window = Arc::new(
        event_loop
            .create_window(
                WindowAttributes::default()
                    .with_title(title)
                    .with_inner_size(LogicalSize::new(width as f64, height as f64))
                    .with_resizable(false),
            )
            .context("failed to create window")?,
    );

    let presenter = WgpuWindowPresenter::new(window.clone(), width, height)
        .context("wgpu window init failed")?;

    let target_frame_duration = if target_fps == 0 {
        Duration::ZERO
    } else {
        Duration::from_secs_f64(1.0 / (target_fps as f64))
    };

    let state = Rc::new(RefCell::new(WindowRuntimeState {
        event_loop,
        window,
        presenter,
        keys_down: HashSet::new(),
        close_requested: false,
        target_frame_duration,
        last_present_at: None,
    }));

    Ok((
        WindowInputProvider {
            state: Rc::clone(&state),
        },
        WindowFrameObserver { state },
    ))
}

impl InputProvider for WindowInputProvider {
    fn next_frame(&mut self, _frame_idx: u32) -> InputFrame {
        let mut state = self.state.borrow_mut();
        state.pump_events();

        let left = state.key_down(KeyCode::KeyA) || state.key_down(KeyCode::ArrowLeft);
        let right = state.key_down(KeyCode::KeyD) || state.key_down(KeyCode::ArrowRight);
        let up = state.key_down(KeyCode::KeyW) || state.key_down(KeyCode::ArrowUp);
        let down = state.key_down(KeyCode::KeyS) || state.key_down(KeyCode::ArrowDown);

        let mut frame = InputFrame::default();
        let move_x = if left == right {
            0.0
        } else if left {
            -1.0
        } else {
            1.0
        };
        let move_y = if up == down {
            0.0
        } else if up {
            -1.0
        } else {
            1.0
        };

        frame.set_axis("move_x", move_x);
        frame.set_axis("move_y", move_y);
        frame.set_action("DPadLeft", left);
        frame.set_action("DPadRight", right);
        frame.set_action("DPadUp", up);
        frame.set_action("DPadDown", down);
        frame.set_action(
            "A",
            state.key_down(KeyCode::Space) || state.key_down(KeyCode::KeyZ),
        );
        frame.set_action("Pause", state.key_down(KeyCode::KeyP));
        frame.set_action(
            "Start",
            state.key_down(KeyCode::Enter) || state.key_down(KeyCode::KeyR),
        );
        frame
    }
}

impl FrameObserver for WindowFrameObserver {
    fn on_frame(
        &mut self,
        _frame_idx: u32,
        frame_rgba: &[u8],
        width: u32,
        height: u32,
    ) -> Result<bool> {
        let mut state = self.state.borrow_mut();
        state.pump_events();
        if !state.should_continue() {
            return Ok(false);
        }

        state.throttle();
        state
            .presenter
            .present_rgba(frame_rgba, width, height)
            .context("failed to present frame to window")?;

        Ok(state.should_continue())
    }
}

struct WgpuWindowPresenter {
    window: Arc<winit::window::Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    frame_texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    frame_width: u32,
    frame_height: u32,
}

impl WgpuWindowPresenter {
    fn new(
        window: Arc<winit::window::Window>,
        frame_width: u32,
        frame_height: u32,
    ) -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window.clone())
            .context("failed to create wgpu surface")?;

        let (adapter, device, queue) = pollster::block_on(async {
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    compatible_surface: Some(&surface),
                })
                .await
                .ok_or_else(|| anyhow!("failed to acquire wgpu adapter for window surface"))?;

            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("vcon-window-device"),
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::downlevel_defaults(),
                    },
                    None,
                )
                .await
                .context("failed to create wgpu device")?;

            Ok::<_, anyhow::Error>((adapter, device, queue))
        })?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .or_else(|| surface_caps.formats.first().copied())
            .ok_or_else(|| anyhow!("surface reports no supported formats"))?;
        let present_mode = surface_caps
            .present_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::PresentMode::Fifo)
            .or_else(|| surface_caps.present_modes.first().copied())
            .ok_or_else(|| anyhow!("surface reports no present modes"))?;
        let alpha_mode = surface_caps
            .alpha_modes
            .first()
            .copied()
            .ok_or_else(|| anyhow!("surface reports no alpha modes"))?;

        let window_size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window_size.width.max(1),
            height: window_size.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let frame_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vcon-window-frame-rgba"),
            size: wgpu::Extent3d {
                width: frame_width,
                height: frame_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let frame_view = frame_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..wgpu::SamplerDescriptor::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vcon-window-bind-group-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vcon-window-bind-group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&frame_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vcon-window-shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
@group(0) @binding(0) var frame_tex: texture_2d<f32>;
@group(0) @binding(1) var frame_sampler: sampler;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(3.0, 1.0)
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 2.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(2.0, 0.0)
    );
    var out: VsOut;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    out.uv = uvs[vertex_index];
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(frame_tex, frame_sampler, in.uv);
}
"#
                .into(),
            ),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vcon-window-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("vcon-window-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            frame_texture,
            bind_group,
            pipeline,
            frame_width,
            frame_height,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
    }

    fn present_rgba(&mut self, rgba: &[u8], width: u32, height: u32) -> Result<()> {
        if width != self.frame_width || height != self.frame_height {
            return Err(anyhow!(
                "frame dimensions changed unexpectedly: expected {}x{}, got {}x{}",
                self.frame_width,
                self.frame_height,
                width,
                height
            ));
        }
        let expected = usize::try_from(width)
            .ok()
            .and_then(|w| usize::try_from(height).ok().map(|h| w.saturating_mul(h)))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| anyhow!("frame dimensions overflow"))?;
        if rgba.len() != expected {
            return Err(anyhow!(
                "invalid RGBA buffer length: expected {expected}, got {}",
                rgba.len()
            ));
        }

        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.frame_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(width.saturating_mul(4)),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                let size = self.window.inner_size();
                self.resize(size.width, size.height);
                self.surface
                    .get_current_texture()
                    .context("failed to acquire surface texture after reconfigure")?
            }
            Err(err) => {
                return Err(anyhow!("failed to acquire surface texture: {err}"));
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("vcon-window-encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vcon-window-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }
}
