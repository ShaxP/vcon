use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use font8x8::{UnicodeFonts, BASIC_FONTS};

#[derive(Debug, Clone, PartialEq)]
pub enum DrawCommand {
    Clear {
        color: [u8; 4],
    },
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        color: [u8; 4],
        thickness: f64,
    },
    Rect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        color: [u8; 4],
        filled: bool,
        thickness: f64,
    },
    Circle {
        x: f64,
        y: f64,
        r: f64,
        color: [u8; 4],
        filled: bool,
        thickness: f64,
    },
    Sprite {
        asset_id: String,
        x: f64,
        y: f64,
        rotation: f64,
        scale: f64,
        color: [u8; 4],
    },
    Text {
        value: String,
        x: f64,
        y: f64,
        size: f64,
        color: [u8; 4],
    },
}

#[derive(Debug, Default, Clone)]
pub struct FrameCommandBuffer {
    pub commands: Vec<DrawCommand>,
}

impl FrameCommandBuffer {
    pub fn push(&mut self, command: DrawCommand) -> Result<(), RenderValidationError> {
        validate_command(&command)?;
        self.commands.push(command);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RenderStats {
    pub commands_executed: usize,
    pub commands_unsupported: usize,
}

#[derive(Debug, Clone)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Default, Clone)]
pub struct AssetStore {
    textures: HashMap<String, Texture>,
}

impl AssetStore {
    pub fn load_from_dir(dir: &Path) -> Result<Self, AssetLoadError> {
        let mut store = AssetStore::default();

        if !dir.exists() {
            return Ok(store);
        }

        for entry in std::fs::read_dir(dir)
            .map_err(|source| AssetLoadError::ReadDir(dir.to_path_buf(), source))?
        {
            let entry =
                entry.map_err(|source| AssetLoadError::ReadDir(dir.to_path_buf(), source))?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("ppm") {
                continue;
            }

            let bytes = std::fs::read(&path)
                .map_err(|source| AssetLoadError::ReadFile(path.clone(), source))?;
            let texture = parse_ppm_p6(&bytes)
                .map_err(|msg| AssetLoadError::InvalidPpm(path.clone(), msg))?;
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| AssetLoadError::InvalidAssetId(path.clone()))?
                .to_owned();
            store.textures.insert(id, texture);
        }

        Ok(store)
    }

    pub fn texture(&self, id: &str) -> Option<&Texture> {
        self.textures.get(id)
    }
}

#[derive(Debug, Clone)]
struct FontGlyph {
    u: u32,
    v: u32,
    w: u32,
    h: u32,
}

#[derive(Debug, Clone)]
struct FontAtlas {
    width: u32,
    alpha: Vec<u8>,
    glyphs: HashMap<char, FontGlyph>,
}

impl FontAtlas {
    fn builtin() -> Self {
        let cell_w = 8u32;
        let cell_h = 8u32;
        let cols = 16u32;
        let rows = 8u32;
        let width = cols * cell_w;
        let height = rows * cell_h;
        let mut alpha = vec![0u8; (width * height) as usize];
        let mut glyphs = HashMap::new();

        for code in 32u8..=127u8 {
            let ch = code as char;
            let idx = (code - 32) as u32;
            let gx = (idx % cols) * cell_w;
            let gy = (idx / cols) * cell_h;

            if let Some(bitmap) = BASIC_FONTS.get(ch) {
                for (row, bits) in bitmap.iter().enumerate() {
                    for col in 0..8usize {
                        let on = (bits >> col) & 1 == 1;
                        let x = gx + col as u32;
                        let y = gy + row as u32;
                        let off = (y * width + x) as usize;
                        alpha[off] = if on { 255 } else { 0 };
                    }
                }
            }

            glyphs.insert(
                ch,
                FontGlyph {
                    u: gx,
                    v: gy,
                    w: cell_w,
                    h: cell_h,
                },
            );
        }

        Self {
            width,
            alpha,
            glyphs,
        }
    }

    fn glyph(&self, ch: char) -> Option<&FontGlyph> {
        self.glyphs.get(&ch)
    }

    fn alpha_at(&self, x: u32, y: u32) -> u8 {
        let idx = (y * self.width + x) as usize;
        self.alpha[idx]
    }
}

#[derive(Debug, Clone)]
pub struct SoftwareFrame {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    font: FontAtlas,
}

impl SoftwareFrame {
    pub fn new(width: u32, height: u32) -> Self {
        let pixel_count = (width as usize) * (height as usize) * 4;
        Self {
            width,
            height,
            pixels: vec![0; pixel_count],
            font: FontAtlas::builtin(),
        }
    }

    pub fn apply(&mut self, commands: &FrameCommandBuffer) -> RenderStats {
        self.apply_with_assets(commands, None)
    }

    pub fn apply_with_assets(
        &mut self,
        commands: &FrameCommandBuffer,
        assets: Option<&AssetStore>,
    ) -> RenderStats {
        let mut stats = RenderStats::default();

        for command in &commands.commands {
            match command {
                DrawCommand::Clear { color } => {
                    self.clear(*color);
                    stats.commands_executed += 1;
                }
                DrawCommand::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    color,
                    thickness,
                } => {
                    self.draw_line(*x1, *y1, *x2, *y2, *color, *thickness);
                    stats.commands_executed += 1;
                }
                DrawCommand::Rect {
                    x,
                    y,
                    w,
                    h,
                    color,
                    filled,
                    thickness,
                } => {
                    self.draw_rect(*x, *y, *w, *h, *color, *filled, *thickness);
                    stats.commands_executed += 1;
                }
                DrawCommand::Circle {
                    x,
                    y,
                    r,
                    color,
                    filled,
                    thickness,
                } => {
                    self.draw_circle(*x, *y, *r, *color, *filled, *thickness);
                    stats.commands_executed += 1;
                }
                DrawCommand::Sprite {
                    asset_id,
                    x,
                    y,
                    scale,
                    color,
                    ..
                } => {
                    if let Some(texture) = assets.and_then(|a| a.texture(asset_id)) {
                        self.draw_texture(texture, *x, *y, *scale, *color);
                        stats.commands_executed += 1;
                    } else {
                        stats.commands_unsupported += 1;
                    }
                }
                DrawCommand::Text {
                    value,
                    x,
                    y,
                    size,
                    color,
                } => {
                    self.draw_text_atlas(value, *x, *y, *size, *color);
                    stats.commands_executed += 1;
                }
            }
        }

        stats
    }

    pub fn write_ppm(&self, path: &Path) -> Result<(), RenderIoError> {
        let mut file = std::fs::File::create(path)
            .map_err(|source| RenderIoError::Write(path.to_path_buf(), source))?;

        let header = format!("P6\n{} {}\n255\n", self.width, self.height);
        file.write_all(header.as_bytes())
            .map_err(|source| RenderIoError::Write(path.to_path_buf(), source))?;

        let mut rgb = Vec::with_capacity((self.width as usize) * (self.height as usize) * 3);
        for chunk in self.pixels.chunks_exact(4) {
            rgb.push(chunk[0]);
            rgb.push(chunk[1]);
            rgb.push(chunk[2]);
        }

        file.write_all(&rgb)
            .map_err(|source| RenderIoError::Write(path.to_path_buf(), source))?;
        Ok(())
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    fn clear(&mut self, color: [u8; 4]) {
        for px in self.pixels.chunks_exact_mut(4) {
            px.copy_from_slice(&color);
        }
    }

    fn draw_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, color: [u8; 4], thickness: f64) {
        let half = ((thickness.round() as i32).max(1) - 1) / 2;
        let x1 = x1.round() as i32;
        let y1 = y1.round() as i32;
        let x2 = x2.round() as i32;
        let y2 = y2.round() as i32;

        for oy in -half..=half {
            for ox in -half..=half {
                self.draw_line_1px(x1 + ox, y1 + oy, x2 + ox, y2 + oy, color);
            }
        }
    }

    fn draw_line_1px(&mut self, mut x1: i32, mut y1: i32, x2: i32, y2: i32, color: [u8; 4]) {
        let dx = (x2 - x1).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let dy = -(y2 - y1).abs();
        let sy = if y1 < y2 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.put_pixel(x1, y1, color);
            if x1 == x2 && y1 == y2 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x1 += sx;
            }
            if e2 <= dx {
                err += dx;
                y1 += sy;
            }
        }
    }

    fn draw_rect(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        color: [u8; 4],
        filled: bool,
        thickness: f64,
    ) {
        let x0 = x.round() as i32;
        let y0 = y.round() as i32;
        let x1 = (x + w).round() as i32;
        let y1 = (y + h).round() as i32;

        if filled {
            for yy in y0..y1 {
                for xx in x0..x1 {
                    self.put_pixel(xx, yy, color);
                }
            }
        } else {
            let t = (thickness.round() as i32).max(1);
            for i in 0..t {
                self.draw_line_1px(x0, y0 + i, x1, y0 + i, color);
                self.draw_line_1px(x0, y1 - i, x1, y1 - i, color);
                self.draw_line_1px(x0 + i, y0, x0 + i, y1, color);
                self.draw_line_1px(x1 - i, y0, x1 - i, y1, color);
            }
        }
    }

    fn draw_circle(
        &mut self,
        x: f64,
        y: f64,
        r: f64,
        color: [u8; 4],
        filled: bool,
        thickness: f64,
    ) {
        let cx = x.round() as i32;
        let cy = y.round() as i32;
        let radius = r.round() as i32;

        if filled {
            let r2 = radius * radius;
            for yy in (cy - radius)..=(cy + radius) {
                for xx in (cx - radius)..=(cx + radius) {
                    let dx = xx - cx;
                    let dy = yy - cy;
                    if dx * dx + dy * dy <= r2 {
                        self.put_pixel(xx, yy, color);
                    }
                }
            }
        } else {
            let t = (thickness.round() as i32).max(1);
            let outer = radius;
            let inner = (radius - t).max(0);
            let outer2 = outer * outer;
            let inner2 = inner * inner;
            for yy in (cy - outer)..=(cy + outer) {
                for xx in (cx - outer)..=(cx + outer) {
                    let dx = xx - cx;
                    let dy = yy - cy;
                    let d2 = dx * dx + dy * dy;
                    if d2 <= outer2 && d2 >= inner2 {
                        self.put_pixel(xx, yy, color);
                    }
                }
            }
        }
    }

    fn draw_texture(&mut self, texture: &Texture, x: f64, y: f64, scale: f64, tint: [u8; 4]) {
        let s = scale.max(0.01);
        let out_w = ((texture.width as f64) * s).round().max(1.0) as u32;
        let out_h = ((texture.height as f64) * s).round().max(1.0) as u32;
        let x0 = x.round() as i32;
        let y0 = y.round() as i32;

        for oy in 0..out_h {
            for ox in 0..out_w {
                let sx = ((ox as f64 / out_w as f64) * texture.width as f64)
                    .floor()
                    .min((texture.width - 1) as f64) as u32;
                let sy = ((oy as f64 / out_h as f64) * texture.height as f64)
                    .floor()
                    .min((texture.height - 1) as f64) as u32;
                let src_i = ((sy * texture.width + sx) * 4) as usize;

                let src = &texture.pixels[src_i..src_i + 4];
                let color = [
                    ((src[0] as u16 * tint[0] as u16) / 255) as u8,
                    ((src[1] as u16 * tint[1] as u16) / 255) as u8,
                    ((src[2] as u16 * tint[2] as u16) / 255) as u8,
                    ((src[3] as u16 * tint[3] as u16) / 255) as u8,
                ];

                self.put_pixel(x0 + ox as i32, y0 + oy as i32, color);
            }
        }
    }

    fn draw_text_atlas(&mut self, text: &str, x: f64, y: f64, size: f64, color: [u8; 4]) {
        let scale = (size / 8.0).max(1.0);
        let mut pen_x = x;

        for ch in text.chars() {
            if ch == '\n' {
                pen_x = x;
                continue;
            }

            if let Some(glyph) = self.font.glyph(ch).cloned() {
                self.blit_font_glyph(&glyph, pen_x, y, scale, color);
            }
            pen_x += (8.0 * scale) + (1.0 * scale);
        }
    }

    fn blit_font_glyph(&mut self, glyph: &FontGlyph, x: f64, y: f64, scale: f64, color: [u8; 4]) {
        let x0 = x.round() as i32;
        let y0 = y.round() as i32;
        let out_w = ((glyph.w as f64) * scale).round().max(1.0) as u32;
        let out_h = ((glyph.h as f64) * scale).round().max(1.0) as u32;

        for oy in 0..out_h {
            for ox in 0..out_w {
                let gx = ((ox as f64 / out_w as f64) * glyph.w as f64)
                    .floor()
                    .min((glyph.w - 1) as f64) as u32;
                let gy = ((oy as f64 / out_h as f64) * glyph.h as f64)
                    .floor()
                    .min((glyph.h - 1) as f64) as u32;

                let alpha = self.font.alpha_at(glyph.u + gx, glyph.v + gy);
                if alpha == 0 {
                    continue;
                }

                let blended = [
                    ((color[0] as u16 * alpha as u16) / 255) as u8,
                    ((color[1] as u16 * alpha as u16) / 255) as u8,
                    ((color[2] as u16 * alpha as u16) / 255) as u8,
                    ((color[3] as u16 * alpha as u16) / 255) as u8,
                ];
                self.put_pixel(x0 + ox as i32, y0 + oy as i32, blended);
            }
        }
    }

    fn put_pixel(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if x < 0 || y < 0 {
            return;
        }
        let x = x as u32;
        let y = y as u32;
        if x >= self.width || y >= self.height {
            return;
        }

        let idx = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        self.pixels[idx..idx + 4].copy_from_slice(&color);
    }
}

fn validate_command(command: &DrawCommand) -> Result<(), RenderValidationError> {
    match command {
        DrawCommand::Clear { .. } => Ok(()),
        DrawCommand::Line { thickness, .. } => {
            if *thickness <= 0.0 {
                return Err(RenderValidationError::InvalidCommand(
                    "line thickness must be > 0".to_owned(),
                ));
            }
            Ok(())
        }
        DrawCommand::Rect {
            w,
            h,
            filled,
            thickness,
            ..
        } => {
            if *w <= 0.0 || *h <= 0.0 {
                return Err(RenderValidationError::InvalidCommand(
                    "rect width/height must be > 0".to_owned(),
                ));
            }
            if !filled && *thickness <= 0.0 {
                return Err(RenderValidationError::InvalidCommand(
                    "outlined rect thickness must be > 0".to_owned(),
                ));
            }
            Ok(())
        }
        DrawCommand::Circle {
            r,
            filled,
            thickness,
            ..
        } => {
            if *r <= 0.0 {
                return Err(RenderValidationError::InvalidCommand(
                    "circle radius must be > 0".to_owned(),
                ));
            }
            if !filled && *thickness <= 0.0 {
                return Err(RenderValidationError::InvalidCommand(
                    "outlined circle thickness must be > 0".to_owned(),
                ));
            }
            Ok(())
        }
        DrawCommand::Sprite {
            asset_id, scale, ..
        } => {
            if asset_id.trim().is_empty() {
                return Err(RenderValidationError::InvalidCommand(
                    "sprite asset_id must be non-empty".to_owned(),
                ));
            }
            if *scale <= 0.0 {
                return Err(RenderValidationError::InvalidCommand(
                    "sprite scale must be > 0".to_owned(),
                ));
            }
            Ok(())
        }
        DrawCommand::Text { size, .. } => {
            if *size <= 0.0 {
                return Err(RenderValidationError::InvalidCommand(
                    "text size must be > 0".to_owned(),
                ));
            }
            Ok(())
        }
    }
}

fn parse_ppm_p6(bytes: &[u8]) -> Result<Texture, String> {
    let mut i = 0usize;
    let magic = next_token(bytes, &mut i).ok_or("missing magic")?;
    if magic != "P6" {
        return Err("unsupported ppm format (expected P6)".to_owned());
    }

    let width: u32 = next_token(bytes, &mut i)
        .ok_or("missing width")?
        .parse()
        .map_err(|_| "invalid width".to_owned())?;
    let height: u32 = next_token(bytes, &mut i)
        .ok_or("missing height")?
        .parse()
        .map_err(|_| "invalid height".to_owned())?;
    let maxval: u32 = next_token(bytes, &mut i)
        .ok_or("missing maxval")?
        .parse()
        .map_err(|_| "invalid maxval".to_owned())?;
    if maxval != 255 {
        return Err("unsupported maxval (expected 255)".to_owned());
    }

    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }

    let expected = (width as usize) * (height as usize) * 3;
    if bytes.len().saturating_sub(i) < expected {
        return Err("ppm pixel data is truncated".to_owned());
    }

    let rgb = &bytes[i..i + expected];
    let mut rgba = Vec::with_capacity((width as usize) * (height as usize) * 4);
    for chunk in rgb.chunks_exact(3) {
        rgba.push(chunk[0]);
        rgba.push(chunk[1]);
        rgba.push(chunk[2]);
        rgba.push(255);
    }

    Ok(Texture {
        width,
        height,
        pixels: rgba,
    })
}

fn next_token(bytes: &[u8], i: &mut usize) -> Option<String> {
    while *i < bytes.len() {
        let b = bytes[*i];
        if b == b'#' {
            while *i < bytes.len() && bytes[*i] != b'\n' {
                *i += 1;
            }
        } else if b.is_ascii_whitespace() {
            *i += 1;
        } else {
            break;
        }
    }

    if *i >= bytes.len() {
        return None;
    }

    let start = *i;
    while *i < bytes.len() && !bytes[*i].is_ascii_whitespace() {
        *i += 1;
    }

    std::str::from_utf8(&bytes[start..*i])
        .ok()
        .map(ToOwned::to_owned)
}

#[derive(Debug, thiserror::Error)]
pub enum RenderValidationError {
    #[error("invalid draw command: {0}")]
    InvalidCommand(String),
}

#[derive(Debug, thiserror::Error)]
pub enum RenderIoError {
    #[error("failed writing rendered frame to {0}: {1}")]
    Write(PathBuf, std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum AssetLoadError {
    #[error("failed to read asset directory {0}: {1}")]
    ReadDir(PathBuf, std::io::Error),
    #[error("failed to read asset file {0}: {1}")]
    ReadFile(PathBuf, std::io::Error),
    #[error("invalid ppm texture {0}: {1}")]
    InvalidPpm(PathBuf, String),
    #[error("invalid asset id for texture file {0}")]
    InvalidAssetId(PathBuf),
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{AssetStore, DrawCommand, FrameCommandBuffer, SoftwareFrame};

    #[test]
    fn accepts_valid_rect() {
        let mut frame = FrameCommandBuffer::default();
        frame
            .push(DrawCommand::Rect {
                x: 10.0,
                y: 20.0,
                w: 30.0,
                h: 40.0,
                color: [255, 0, 0, 255],
                filled: true,
                thickness: 1.0,
            })
            .expect("valid rect should pass");
        assert_eq!(frame.commands.len(), 1);
    }

    #[test]
    fn rejects_invalid_line_thickness() {
        let mut frame = FrameCommandBuffer::default();
        let err = frame
            .push(DrawCommand::Line {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 10.0,
                color: [255, 255, 255, 255],
                thickness: 0.0,
            })
            .expect_err("thickness 0 must fail");
        assert!(err.to_string().contains("thickness"));
    }

    #[test]
    fn rasterizes_clear_and_rect() {
        let mut commands = FrameCommandBuffer::default();
        commands
            .push(DrawCommand::Clear {
                color: [10, 20, 30, 255],
            })
            .expect("clear should validate");
        commands
            .push(DrawCommand::Rect {
                x: 2.0,
                y: 2.0,
                w: 4.0,
                h: 4.0,
                color: [200, 10, 10, 255],
                filled: true,
                thickness: 1.0,
            })
            .expect("rect should validate");

        let mut frame = SoftwareFrame::new(16, 16);
        let stats = frame.apply(&commands);

        assert_eq!(stats.commands_executed, 2);
        assert_eq!(stats.commands_unsupported, 0);

        let px = frame.pixels();
        let idx_bg = 0;
        assert_eq!(&px[idx_bg..idx_bg + 4], &[10, 20, 30, 255]);

        let idx_rect = ((3usize * 16usize) + 3usize) * 4;
        assert_eq!(&px[idx_rect..idx_rect + 4], &[200, 10, 10, 255]);
    }

    #[test]
    fn rasterizes_text_with_font_atlas() {
        let mut commands = FrameCommandBuffer::default();
        commands
            .push(DrawCommand::Clear {
                color: [0, 0, 0, 255],
            })
            .expect("clear should validate");
        commands
            .push(DrawCommand::Text {
                value: "A".to_owned(),
                x: 1.0,
                y: 1.0,
                size: 8.0,
                color: [0, 255, 0, 255],
            })
            .expect("text should validate");

        let mut frame = SoftwareFrame::new(32, 32);
        let stats = frame.apply(&commands);
        assert_eq!(stats.commands_executed, 2);
        assert_eq!(stats.commands_unsupported, 0);

        let px = frame.pixels();
        assert!(
            px.chunks_exact(4)
                .any(|p| p[0] == 0 && p[1] == 255 && p[2] == 0 && p[3] == 255),
            "text rendering should write green glyph pixels"
        );
    }

    #[test]
    fn loads_texture_asset_and_renders_sprite() {
        let dir = std::env::temp_dir().join("vcon-render-asset-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir should be created");

        let ppm = b"P6\n2 2\n255\n\
\xff\x00\x00\
\x00\xff\x00\
\x00\x00\xff\
\xff\xff\x00";
        fs::write(dir.join("hero.ppm"), ppm).expect("ppm should be written");

        let assets = AssetStore::load_from_dir(&dir).expect("asset store should load");

        let mut commands = FrameCommandBuffer::default();
        commands
            .push(DrawCommand::Clear {
                color: [0, 0, 0, 255],
            })
            .expect("clear should validate");
        commands
            .push(DrawCommand::Sprite {
                asset_id: "hero".to_owned(),
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
                scale: 1.0,
                color: [255, 255, 255, 255],
            })
            .expect("sprite should validate");

        let mut frame = SoftwareFrame::new(4, 4);
        let stats = frame.apply_with_assets(&commands, Some(&assets));
        assert_eq!(stats.commands_executed, 2);
        assert_eq!(stats.commands_unsupported, 0);

        let px = frame.pixels();
        assert_eq!(&px[0..4], &[255, 0, 0, 255]);

        let _ = fs::remove_dir_all(&dir);
    }
}
