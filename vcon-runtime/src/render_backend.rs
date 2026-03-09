use std::io::Write;

use anyhow::{Context, Result};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use vcon_engine::{AssetStore, FrameCommandBuffer, RenderStats, SoftwareFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackendRequest {
    Auto,
    Software,
    Moderngl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveRenderBackend {
    Software,
    Moderngl,
}

impl ActiveRenderBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            ActiveRenderBackend::Software => "software",
            ActiveRenderBackend::Moderngl => "moderngl",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderBackendSelection {
    pub requested: RenderBackendRequest,
    pub active: ActiveRenderBackend,
    pub fallback_reason: Option<String>,
}

pub fn select_render_backend(requested: RenderBackendRequest) -> RenderBackendSelection {
    select_render_backend_with_probe(requested, probe_moderngl_support())
}

fn select_render_backend_with_probe(
    requested: RenderBackendRequest,
    probe: Result<(), String>,
) -> RenderBackendSelection {
    match requested {
        RenderBackendRequest::Software => RenderBackendSelection {
            requested,
            active: ActiveRenderBackend::Software,
            fallback_reason: None,
        },
        RenderBackendRequest::Moderngl => match probe {
            Ok(()) => RenderBackendSelection {
                requested,
                active: ActiveRenderBackend::Moderngl,
                fallback_reason: None,
            },
            Err(reason) => RenderBackendSelection {
                requested,
                active: ActiveRenderBackend::Software,
                fallback_reason: Some(reason),
            },
        },
        RenderBackendRequest::Auto => match probe {
            Ok(()) => RenderBackendSelection {
                requested,
                active: ActiveRenderBackend::Moderngl,
                fallback_reason: None,
            },
            Err(reason) => RenderBackendSelection {
                requested,
                active: ActiveRenderBackend::Software,
                fallback_reason: Some(format!("auto fallback: {reason}")),
            },
        },
    }
}

fn probe_moderngl_support() -> Result<(), String> {
    Python::with_gil(|py| {
        let moderngl = py
            .import_bound("moderngl")
            .map_err(|err| format!("failed to import moderngl: {err}"))?;
        moderngl
            .getattr("create_standalone_context")
            .map_err(|err| format!("moderngl missing create_standalone_context: {err}"))?
            .call0()
            .map_err(|err| format!("failed to initialize moderngl context: {err}"))?;
        Ok(())
    })
}

pub struct RenderExecutor {
    backend: ActiveRenderBackend,
    surface: SoftwareFrame,
    gpu_post: Option<ModernglPostProcess>,
}

impl RenderExecutor {
    pub fn new(backend: ActiveRenderBackend, width: u32, height: u32) -> Self {
        let surface = SoftwareFrame::new(width, height);
        let gpu_post = if backend == ActiveRenderBackend::Moderngl {
            match ModernglPostProcess::new(width, height) {
                Ok(state) => Some(state),
                Err(err) => {
                    eprintln!(
                        "Render backend fallback: moderngl runtime unavailable ({err:#}); using software"
                    );
                    None
                }
            }
        } else {
            None
        };

        let active_backend = if gpu_post.is_some() {
            backend
        } else {
            ActiveRenderBackend::Software
        };

        Self {
            backend: active_backend,
            surface,
            gpu_post,
        }
    }

    pub fn backend(&self) -> ActiveRenderBackend {
        self.backend
    }

    pub fn render_frame(
        &mut self,
        commands: &FrameCommandBuffer,
        assets: Option<&AssetStore>,
    ) -> RenderStats {
        let stats = self.surface.apply_with_assets(commands, assets);

        if let Some(gpu) = self.gpu_post.as_mut() {
            if let Err(err) = gpu.process(self.surface.pixels()) {
                eprintln!(
                    "Render backend fallback: moderngl frame processing failed ({err:#}); using software"
                );
                self.gpu_post = None;
                self.backend = ActiveRenderBackend::Software;
            }
        }

        stats
    }

    pub fn dump_ppm(&self, path: &std::path::Path) -> Result<(), vcon_engine::RenderIoError> {
        let mut file = std::fs::File::create(path)
            .map_err(|source| vcon_engine::RenderIoError::Write(path.to_path_buf(), source))?;

        let header = format!("P6\n{} {}\n255\n", self.width(), self.height());
        file.write_all(header.as_bytes())
            .map_err(|source| vcon_engine::RenderIoError::Write(path.to_path_buf(), source))?;

        let mut rgb = Vec::with_capacity((self.width() as usize) * (self.height() as usize) * 3);
        for chunk in self.pixels_rgba().chunks_exact(4) {
            rgb.push(chunk[0]);
            rgb.push(chunk[1]);
            rgb.push(chunk[2]);
        }

        file.write_all(&rgb)
            .map_err(|source| vcon_engine::RenderIoError::Write(path.to_path_buf(), source))?;
        Ok(())
    }

    pub fn pixels_rgba(&self) -> &[u8] {
        if let Some(gpu) = self.gpu_post.as_ref() {
            gpu.pixels()
        } else {
            self.surface.pixels()
        }
    }

    pub fn width(&self) -> u32 {
        self.surface.width()
    }

    pub fn height(&self) -> u32 {
        self.surface.height()
    }
}

struct ModernglPostProcess {
    renderer: Py<PyAny>,
    pixels: Vec<u8>,
}

impl ModernglPostProcess {
    fn new(width: u32, height: u32) -> Result<Self> {
        let renderer = Python::with_gil(|py| -> Result<Py<PyAny>> {
            let module = PyModule::from_code_bound(
                py,
                MODERNGL_POST_PROCESS_PY,
                "vcon_moderngl_post.py",
                "vcon_moderngl_post",
            )
            .context("failed to compile moderngl post-process module")?;
            let class = module
                .getattr("ModernglPostProcess")
                .context("moderngl post-process class missing")?;
            let instance = class
                .call1((width, height))
                .context("failed to initialize moderngl post-process")?;
            Ok(instance.unbind())
        })?;

        let expected_len = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4);
        Ok(Self {
            renderer,
            pixels: vec![0_u8; expected_len],
        })
    }

    fn process(&mut self, rgba_pixels: &[u8]) -> Result<()> {
        Python::with_gil(|py| -> Result<()> {
            let renderer = self.renderer.bind(py);
            let input = PyBytes::new_bound(py, rgba_pixels);
            let output = renderer
                .call_method1("process_rgba", (input,))
                .context("moderngl process_rgba call failed")?;
            let bytes = output
                .downcast_into::<PyBytes>()
                .map_err(|_| anyhow::anyhow!("process_rgba must return bytes"))?;
            let out = bytes.as_bytes();
            if out.len() != self.pixels.len() {
                anyhow::bail!(
                    "moderngl process_rgba output size mismatch: expected {}, got {}",
                    self.pixels.len(),
                    out.len()
                );
            }
            self.pixels.copy_from_slice(out);
            Ok(())
        })
    }

    fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

const MODERNGL_POST_PROCESS_PY: &str = r#"
import struct
import moderngl


class ModernglPostProcess:
    def __init__(self, width, height):
        self.width = int(width)
        self.height = int(height)
        self.expected_size = self.width * self.height * 4

        self.ctx = moderngl.create_standalone_context()
        self.src = self.ctx.texture((self.width, self.height), 4, dtype='f1')
        self.dst = self.ctx.texture((self.width, self.height), 4, dtype='f1')
        self.fbo = self.ctx.framebuffer(color_attachments=[self.dst])

        self.program = self.ctx.program(
            vertex_shader='''
#version 330
in vec2 in_pos;
in vec2 in_uv;
out vec2 v_uv;
void main() {
    v_uv = in_uv;
    gl_Position = vec4(in_pos, 0.0, 1.0);
}
''',
            fragment_shader='''
#version 330
uniform sampler2D src_tex;
in vec2 v_uv;
out vec4 f_color;
void main() {
    f_color = texture(src_tex, v_uv);
}
''',
        )

        vertex_data = (
            -1.0, -1.0, 0.0, 0.0,
             1.0, -1.0, 1.0, 0.0,
            -1.0,  1.0, 0.0, 1.0,
             1.0,  1.0, 1.0, 1.0,
        )
        self.vertex_buffer = self.ctx.buffer(data=struct.pack('16f', *vertex_data))
        self.vao = self.ctx.vertex_array(
            self.program,
            [(self.vertex_buffer, '2f 2f', 'in_pos', 'in_uv')],
        )

        self.src.filter = (moderngl.NEAREST, moderngl.NEAREST)
        self.dst.filter = (moderngl.NEAREST, moderngl.NEAREST)

    def process_rgba(self, rgba):
        if len(rgba) != self.expected_size:
            raise ValueError(
                f'invalid input size: expected {self.expected_size}, got {len(rgba)}'
            )

        self.src.write(rgba)
        self.fbo.use()
        self.ctx.disable(moderngl.BLEND)
        self.ctx.viewport = (0, 0, self.width, self.height)
        self.src.use(location=0)
        self.vao.render(moderngl.TRIANGLE_STRIP)
        return self.dst.read(alignment=1)
"#;

#[cfg(test)]
mod tests {
    use super::{
        select_render_backend, select_render_backend_with_probe, ActiveRenderBackend,
        RenderBackendRequest,
    };

    #[test]
    fn software_request_stays_software() {
        let selection = select_render_backend(RenderBackendRequest::Software);
        assert_eq!(selection.active, ActiveRenderBackend::Software);
        assert!(selection.fallback_reason.is_none());
    }

    #[test]
    fn auto_or_moderngl_always_resolve_to_supported_backend() {
        for request in [RenderBackendRequest::Auto, RenderBackendRequest::Moderngl] {
            let selection = select_render_backend(request);
            assert!(
                matches!(
                    selection.active,
                    ActiveRenderBackend::Software | ActiveRenderBackend::Moderngl
                ),
                "selection must always resolve to a runnable backend"
            );
        }
    }

    #[test]
    fn moderngl_request_falls_back_to_software_when_probe_fails() {
        let selection = select_render_backend_with_probe(
            RenderBackendRequest::Moderngl,
            Err("missing GL".to_owned()),
        );
        assert_eq!(selection.active, ActiveRenderBackend::Software);
        assert_eq!(selection.fallback_reason.as_deref(), Some("missing GL"));
    }

    #[test]
    fn auto_request_falls_back_with_auto_prefix() {
        let selection =
            select_render_backend_with_probe(RenderBackendRequest::Auto, Err("no context".into()));
        assert_eq!(selection.active, ActiveRenderBackend::Software);
        assert_eq!(
            selection.fallback_reason.as_deref(),
            Some("auto fallback: no context")
        );
    }
}
