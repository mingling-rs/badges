//! Badge image generation for the Mingling project.
//!
//! Draws a shields.io-style "for-the-badge" PNG using the color palette of
//! the landing page (`mingling/index.html`): a dark code-panel key segment
//! plus a gold value segment, set in bold JetBrains Mono. The font is
//! downloaded from a CDN at runtime and cached in `./.cache/`.

use std::{
    env::current_dir,
    fmt, fs,
    io::{Error, ErrorKind::Other},
    path::Path,
    str::FromStr,
};

use ab_glyph::{Font, FontArc, Glyph, PxScale, PxScaleFont, ScaleFont, point};
use image::RgbaImage;
use tiny_skia::{
    Color as SkColor, FillRule, Paint, Path as SkPath, PathBuilder, Pixmap, Rect, Stroke, Transform,
};

/// JetBrains Mono Bold — the code font used on mingling's site.
/// Fetched from the CDN on first use, then cached at `./.cache/`.
const FONT_URL: &str =
    "https://cdn.jsdelivr.net/gh/JetBrains/JetBrainsMono@2.304/fonts/ttf/JetBrainsMono-Bold.ttf";

/// Badge geometry — a shields.io-style badge exported 36px tall with the
/// width following the text content. All measurements derive from the
/// 30px design via [`SCALE`].
const BADGE_HEIGHT: f32 = 36.0;
const DESIGN_HEIGHT: f32 = 30.0;
const SCALE: f32 = BADGE_HEIGHT / DESIGN_HEIGHT;
const FONT_SIZE: f32 = 12.0 * SCALE;
const PAD_X: f32 = 12.0 * SCALE;
const CORNER_RADIUS: f32 = 3.0 * SCALE;

/// A 24-bit RGB color, parsed from hex strings like `"#d4a84b"` or `"d4a84b"`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    fn to_skia(self) -> SkColor {
        SkColor::from_rgba8(self.r, self.g, self.b, 255)
    }
}

impl FromStr for Color {
    type Err = ColorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix('#').unwrap_or(s);
        if s.len() != 6 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(ColorParseError(s.to_string()));
        }
        Ok(Color {
            r: u8::from_str_radix(&s[0..2], 16).unwrap(),
            g: u8::from_str_radix(&s[2..4], 16).unwrap(),
            b: u8::from_str_radix(&s[4..6], 16).unwrap(),
        })
    }
}

/// Convenience for palette constants like `"#241c16".into()`.
/// Panics on invalid input; use [`FromStr`] for fallible parsing.
impl From<&str> for Color {
    fn from(s: &str) -> Self {
        s.parse().unwrap_or_else(|e: ColorParseError| panic!("{e}"))
    }
}

impl From<String> for Color {
    fn from(s: String) -> Self {
        Color::from(s.as_str())
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorParseError(String);

impl fmt::Display for ColorParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid color {:?}: expected a hex color like \"#RRGGBB\" or \"RRGGBB\"",
            self.0
        )
    }
}

impl std::error::Error for ColorParseError {}

/// The badge palette, defaulting to the Mingling theme from `index.html`.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    /// Key segment background — `#241c16` code panel.
    pub key_bg: Color,
    /// Key segment text — `#e8ddd0` primary text.
    pub key_fg: Color,
    /// Value segment background — `#d4a84b` gold accent.
    pub value_bg: Color,
    /// Value segment text — `#1a1410` ink.
    pub value_fg: Color,
    /// Outer hairline — `#3a2e24` border.
    pub border: Color,
}

impl Default for Palette {
    fn default() -> Self {
        Palette {
            key_bg: "#241c16".into(),
            key_fg: "#e8ddd0".into(),
            value_bg: "#d4a84b".into(),
            value_fg: "#1a1410".into(),
            border: "#3a2e24".into(),
        }
    }
}

/// Generates a badge image file with the default Mingling palette.
///
/// # Arguments
///
/// * `key` - The badge key text.
/// * `value` - The badge value text.
/// * `out_file` - The output file path for the generated badge.
///
/// # Returns
///
/// `Ok(())` on success, or an `Error` if the badge generation fails.
pub fn generate(key: &str, value: &str, out_file: &Path) -> Result<(), Error> {
    generate_with_palette(key, value, out_file, &Palette::default())
}

/// Like [`generate`], but with a custom [`Palette`].
pub fn generate_with_palette(
    key: &str,
    value: &str,
    out_file: &Path,
    palette: &Palette,
) -> Result<(), Error> {
    let font = load_font()?;
    let font = FontArc::try_from_vec(font)
        .map_err(|e| Error::new(Other, format!("invalid font data: {e}")))?;

    let scale = PxScale {
        x: FONT_SIZE,
        y: FONT_SIZE,
    };
    let scaled = font.as_scaled(scale);

    // Render both labels exactly as given.
    let key_layout = layout_text(&scaled, key);
    let value_layout = layout_text(&scaled, value);

    let key_w = (key_layout.width + PAD_X * 2.0).ceil();
    let value_w = (value_layout.width + PAD_X * 2.0).ceil();
    let width = key_w + value_w;
    let height = BADGE_HEIGHT;

    let mut pixmap = Pixmap::new(width as u32, height as u32)
        .ok_or_else(|| Error::new(Other, "failed to allocate badge bitmap"))?;

    // Key segment: dark code panel with rounded left corners.
    let key_rect = Rect::from_xywh(0.0, 0.0, key_w, height).expect("key rect");
    fill_rounded(
        &mut pixmap,
        key_rect,
        CORNER_RADIUS,
        true,
        false,
        false,
        true,
        palette.key_bg.to_skia(),
    );

    // Value segment: gold accent with rounded right corners.
    let value_rect = Rect::from_xywh(key_w, 0.0, value_w, height).expect("value rect");
    fill_rounded(
        &mut pixmap,
        value_rect,
        CORNER_RADIUS,
        false,
        true,
        true,
        false,
        palette.value_bg.to_skia(),
    );

    // Hairline border around the whole badge, inset so it stays inside.
    let half = SCALE / 2.0;
    let whole_rect =
        Rect::from_xywh(half, half, width - SCALE, height - SCALE).expect("whole rect");
    let path = rounded_path(whole_rect, CORNER_RADIUS, true, true, true, true);
    let mut stroke = Stroke::default();
    stroke.width = SCALE;
    let mut stroke_paint = Paint::default();
    stroke_paint.set_color(palette.border.to_skia());
    pixmap.stroke_path(&path, &stroke_paint, &stroke, Transform::identity(), None);

    // Vertically center the glyphs on the shared baseline.
    let baseline = (height - (scaled.ascent() - scaled.descent())) / 2.0 + scaled.ascent();

    draw_text(
        &mut pixmap,
        &scaled,
        &key_layout,
        point(PAD_X, baseline),
        palette.key_fg.to_skia(),
    );
    draw_text(
        &mut pixmap,
        &scaled,
        &value_layout,
        point(key_w + PAD_X, baseline),
        palette.value_fg.to_skia(),
    );

    // The pixmap is fully opaque, so premultiplied == straight alpha.
    let img = RgbaImage::from_raw(width as u32, height as u32, pixmap.data().to_vec())
        .ok_or_else(|| Error::new(Other, "failed to build png buffer"))?;
    img.save(out_file)
        .map_err(|e| Error::new(Other, format!("failed to save png: {e}")))
}

/// Loads the badge font, downloading it from the CDN into `./.cache/` on
/// first use and reusing the cached file on later runs.
fn load_font() -> Result<Vec<u8>, Error> {
    let cache_dir = current_dir()
        .map_err(|e| Error::new(Other, format!("cannot resolve current directory: {e}")))?
        .join(".cache");
    let cache_file = cache_dir.join(FONT_URL.rsplit('/').next().unwrap_or("badge-font.ttf"));

    if let Ok(data) = fs::read(&cache_file) {
        if !data.is_empty() {
            return Ok(data);
        }
    }

    let data = download_font()?;
    if let Err(e) = fs::create_dir_all(&cache_dir).and_then(|_| fs::write(&cache_file, &data)) {
        eprintln!(
            "Warning: failed to cache font at {}: {e}",
            cache_file.to_string_lossy()
        );
    }
    Ok(data)
}

fn download_font() -> Result<Vec<u8>, Error> {
    let mut response = ureq::get(FONT_URL)
        .call()
        .map_err(|e| Error::new(Other, format!("failed to download font ({FONT_URL}): {e}")))?;
    response
        .body_mut()
        .read_to_vec()
        .map_err(|e| Error::new(Other, format!("failed to read font download: {e}")))
}

/// Laid-out text: each glyph plus its pen offset from the text origin.
/// `width` is the total advance width in pixels.
struct TextLayout {
    glyphs: Vec<(ab_glyph::GlyphId, f32)>,
    width: f32,
}

fn layout_text(scaled: &PxScaleFont<&FontArc>, text: &str) -> TextLayout {
    let mut glyphs = Vec::new();
    let mut pen = 0.0;
    for ch in text.chars() {
        let id = scaled.glyph_id(ch);
        glyphs.push((id, pen));
        pen += scaled.h_advance(id);
    }
    TextLayout { glyphs, width: pen }
}

/// Blends the layout into the pixmap with anti-aliased coverage.
/// `origin` is the point on the baseline where the text starts.
fn draw_text(
    pixmap: &mut Pixmap,
    scaled: &PxScaleFont<&FontArc>,
    layout: &TextLayout,
    origin: ab_glyph::Point,
    color: SkColor,
) {
    let (pw, ph) = (pixmap.width() as i32, pixmap.height() as i32);
    let (fr, fg, fb) = (color.red(), color.green(), color.blue());

    for (id, dx) in &layout.glyphs {
        let glyph = Glyph {
            id: *id,
            scale: scaled.scale(),
            position: point(origin.x + dx, origin.y),
        };
        let Some(outlined) = scaled.outline_glyph(glyph) else {
            continue; // whitespace and other glyphs without outlines
        };

        let bounds = outlined.px_bounds();
        if bounds.max.x <= 0.0
            || bounds.max.y <= 0.0
            || bounds.min.x >= pw as f32
            || bounds.min.y >= ph as f32
        {
            continue;
        }

        let data = pixmap.data_mut();
        outlined.draw(|gx, gy, cov| {
            let px = bounds.min.x as i32 + gx as i32;
            let py = bounds.min.y as i32 + gy as i32;
            if px < 0 || py < 0 || px >= pw || py >= ph {
                return;
            }
            let i = (py as usize * pw as usize + px as usize) * 4;
            let a = cov as f32;
            // Background is opaque, so straight alpha == premultiplied.
            data[i] = (data[i] as f32 * (1.0 - a) + fr * 255.0 * a) as u8;
            data[i + 1] = (data[i + 1] as f32 * (1.0 - a) + fg * 255.0 * a) as u8;
            data[i + 2] = (data[i + 2] as f32 * (1.0 - a) + fb * 255.0 * a) as u8;
        });
    }
}

/// Fills a rect whose outer corners can be rounded independently, so the
/// key/value segments form one continuous badge with rounded ends.
fn fill_rounded(
    pixmap: &mut Pixmap,
    rect: Rect,
    radius: f32,
    tl: bool,
    tr: bool,
    br: bool,
    bl: bool,
    color: SkColor,
) {
    let path = rounded_path(rect, radius, tl, tr, br, bl);
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    pixmap.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

fn rounded_path(rect: Rect, radius: f32, tl: bool, tr: bool, br: bool, bl: bool) -> SkPath {
    let (x, y, w, h) = (rect.x(), rect.y(), rect.width(), rect.height());
    let r = radius.min(w / 2.0).min(h / 2.0);
    let mut pb = PathBuilder::new();

    pb.move_to(x, y + if tl { r } else { 0.0 });
    if tl {
        pb.quad_to(x, y, x + r, y);
    }
    pb.line_to(x + w - if tr { r } else { 0.0 }, y);
    if tr {
        pb.quad_to(x + w, y, x + w, y + r);
    }
    pb.line_to(x + w, y + h - if br { r } else { 0.0 });
    if br {
        pb.quad_to(x + w, y + h, x + w - r, y + h);
    }
    pb.line_to(x + if bl { r } else { 0.0 }, y + h);
    if bl {
        pb.quad_to(x, y + h, x, y + h - r);
    }
    pb.close();
    pb.finish().expect("valid path")
}
