use anyhow::{anyhow, Context, Result};

pub struct WgpuPresenter {
    device: wgpu::Device,
    queue: wgpu::Queue,
    texture: wgpu::Texture,
    width: u32,
    height: u32,
}

impl WgpuPresenter {
    pub fn new(width: u32, height: u32) -> Result<Self> {
        let (device, queue) = pollster::block_on(async {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    compatible_surface: None,
                })
                .await
                .context("failed to acquire wgpu adapter")?;

            adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("vcon-wgpu-device"),
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::downlevel_defaults(),
                    },
                    None,
                )
                .await
                .context("failed to create wgpu device")
        })?;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vcon-frame-rgba"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        Ok(Self {
            device,
            queue,
            texture,
            width,
            height,
        })
    }

    pub fn upload_rgba(&mut self, rgba: &[u8]) -> Result<()> {
        let expected = frame_len(self.width, self.height)?;
        if rgba.len() != expected {
            return Err(anyhow!(
                "invalid RGBA buffer length: expected {expected}, got {}",
                rgba.len()
            ));
        }

        let bytes_per_row = self
            .width
            .checked_mul(4)
            .ok_or_else(|| anyhow!("invalid bytes_per_row for width {}", self.width))?;
        let rows_per_image = self.height;

        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(rows_per_image),
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        self.device.poll(wgpu::Maintain::Poll);
        Ok(())
    }
}

pub fn probe_wgpu_support() -> std::result::Result<(), String> {
    WgpuPresenter::new(2, 2)
        .map(|_| ())
        .map_err(|err| err.to_string())
}

fn frame_len(width: u32, height: u32) -> Result<usize> {
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| anyhow!("frame dimensions overflow: {width}x{height}"))?;
    let bytes = pixels
        .checked_mul(4)
        .ok_or_else(|| anyhow!("frame byte count overflow: {width}x{height}"))?;
    usize::try_from(bytes).context("frame byte count exceeds usize")
}
