use std::io::Write;

use anyhow::{bail, Result};
use vcon_engine::{AssetStore, FrameCommandBuffer, RenderStats, SoftwareFrame};

use crate::wgpu_presenter::{probe_wgpu_support, WgpuPresenter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackendRequest {
    Auto,
    Software,
    Moderngl,
    Wgpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveRenderBackend {
    Software,
    Moderngl,
    Wgpu,
}

impl ActiveRenderBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            ActiveRenderBackend::Software => "software",
            ActiveRenderBackend::Moderngl => "moderngl",
            ActiveRenderBackend::Wgpu => "wgpu",
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
    select_render_backend_with_probe(requested, probe_wgpu_support())
}

fn select_render_backend_with_probe(
    requested: RenderBackendRequest,
    wgpu_probe: std::result::Result<(), String>,
) -> RenderBackendSelection {
    match requested {
        RenderBackendRequest::Software => RenderBackendSelection {
            requested,
            active: ActiveRenderBackend::Software,
            fallback_reason: None,
        },
        RenderBackendRequest::Wgpu => match wgpu_probe {
            Ok(()) => RenderBackendSelection {
                requested,
                active: ActiveRenderBackend::Wgpu,
                fallback_reason: None,
            },
            Err(reason) => RenderBackendSelection {
                requested,
                active: ActiveRenderBackend::Software,
                fallback_reason: Some(reason),
            },
        },
        RenderBackendRequest::Moderngl => RenderBackendSelection {
            requested,
            active: ActiveRenderBackend::Software,
            fallback_reason: Some(
                "moderngl backend is deprecated and disabled; use --render-backend wgpu or software"
                    .to_owned(),
            ),
        },
        RenderBackendRequest::Auto => match wgpu_probe {
            Ok(()) => RenderBackendSelection {
                requested,
                active: ActiveRenderBackend::Wgpu,
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

pub struct RenderExecutor {
    backend: ActiveRenderBackend,
    surface: SoftwareFrame,
    gpu_post: Option<WgpuPostProcess>,
}

impl RenderExecutor {
    pub fn new(backend: ActiveRenderBackend, width: u32, height: u32) -> Self {
        let surface = SoftwareFrame::new(width, height);
        let gpu_post = match backend {
            ActiveRenderBackend::Wgpu => match WgpuPostProcess::new(width, height) {
                Ok(state) => Some(state),
                Err(err) => {
                    eprintln!(
                        "Render backend fallback: wgpu runtime unavailable ({err:#}); using software"
                    );
                    None
                }
            },
            ActiveRenderBackend::Moderngl => {
                eprintln!(
                    "Render backend fallback: moderngl backend is deprecated and disabled; using software"
                );
                None
            }
            ActiveRenderBackend::Software => None,
        };

        let active_backend = if backend == ActiveRenderBackend::Wgpu && gpu_post.is_none() {
            ActiveRenderBackend::Software
        } else if backend == ActiveRenderBackend::Moderngl {
            ActiveRenderBackend::Software
        } else {
            backend
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
                    "Render backend fallback: wgpu frame processing failed ({err:#}); using software"
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
        match self.gpu_post.as_ref() {
            Some(state) => state.pixels(),
            None => self.surface.pixels(),
        }
    }

    pub fn width(&self) -> u32 {
        self.surface.width()
    }

    pub fn height(&self) -> u32 {
        self.surface.height()
    }
}

struct WgpuPostProcess {
    presenter: WgpuPresenter,
    pixels: Vec<u8>,
}

impl WgpuPostProcess {
    fn new(width: u32, height: u32) -> Result<Self> {
        let presenter = WgpuPresenter::new(width, height)?;
        let expected_len = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4);
        Ok(Self {
            presenter,
            pixels: vec![0_u8; expected_len],
        })
    }

    fn process(&mut self, rgba_pixels: &[u8]) -> Result<()> {
        if rgba_pixels.len() != self.pixels.len() {
            bail!(
                "wgpu process_rgba input size mismatch: expected {}, got {}",
                self.pixels.len(),
                rgba_pixels.len()
            );
        }
        self.presenter.upload_rgba(rgba_pixels)?;
        self.pixels.copy_from_slice(rgba_pixels);
        Ok(())
    }

    fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

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
    fn auto_or_gpu_requests_always_resolve_to_supported_backend() {
        for request in [
            RenderBackendRequest::Auto,
            RenderBackendRequest::Moderngl,
            RenderBackendRequest::Wgpu,
        ] {
            let selection = select_render_backend(request);
            assert!(
                matches!(
                    selection.active,
                    ActiveRenderBackend::Software | ActiveRenderBackend::Wgpu
                ),
                "selection must always resolve to a runnable backend"
            );
        }
    }

    #[test]
    fn moderngl_request_is_deprecated_and_falls_back_to_software() {
        let selection = select_render_backend_with_probe(RenderBackendRequest::Moderngl, Ok(()));
        assert_eq!(selection.active, ActiveRenderBackend::Software);
        assert_eq!(
            selection.fallback_reason.as_deref(),
            Some("moderngl backend is deprecated and disabled; use --render-backend wgpu or software")
        );
    }

    #[test]
    fn wgpu_request_uses_wgpu_when_probe_succeeds() {
        let selection = select_render_backend_with_probe(RenderBackendRequest::Wgpu, Ok(()));
        assert_eq!(selection.active, ActiveRenderBackend::Wgpu);
        assert!(selection.fallback_reason.is_none());
    }

    #[test]
    fn auto_request_falls_back_to_software_when_wgpu_probe_fails() {
        let selection =
            select_render_backend_with_probe(RenderBackendRequest::Auto, Err("no adapter".into()));
        assert_eq!(selection.active, ActiveRenderBackend::Software);
        assert_eq!(
            selection.fallback_reason.as_deref(),
            Some("auto fallback: no adapter")
        );
    }
}
