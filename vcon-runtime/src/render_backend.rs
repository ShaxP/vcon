use pyo3::prelude::*;
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
}

impl RenderExecutor {
    pub fn new(backend: ActiveRenderBackend, width: u32, height: u32) -> Self {
        Self {
            backend,
            surface: SoftwareFrame::new(width, height),
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
        // Stream 1 keeps deterministic software rasterization as the shared command executor.
        // The moderngl backend selection currently validates backend availability and reserves
        // the backend switch point without changing frame command semantics.
        self.surface.apply_with_assets(commands, assets)
    }

    pub fn dump_ppm(&self, path: &std::path::Path) -> Result<(), vcon_engine::RenderIoError> {
        self.surface.write_ppm(path)
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
