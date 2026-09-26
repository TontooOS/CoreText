//! Font loading and management (CTFont / CTFontDescriptor).
//!
//! - Load from file (TTF, OTF; WOFF/WOFF2 resolve through the system
//!   loader when Parley/Fontique supports them).
//! - Scan system dirs (`/usr/share/fonts`, `~/.fonts`, ...).
//! - Resolve family + weight + style, with SF Pro first.
//! - Fallback chains for missing glyphs (emoji, CJK).
//! - Runtime register/activate/deactivate of custom fonts.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// System default family on TontooOS.
pub const SF_PRO_FAMILY: &str = "SF Pro";

/// System font directories scanned on TontooOS (plus user dirs).
pub fn system_font_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/fonts/OTF"),
        PathBuf::from("/usr/share/fonts/TTF"),
        PathBuf::from("/usr/share/fonts"),
        PathBuf::from("/Library/Fonts"),
        PathBuf::from("/System/Library/Fonts"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(home);
        dirs.push(home.join(".fonts"));
        dirs.push(home.join(".local/share/fonts"));
    }
    dirs.push(PathBuf::from("assets/fonts"));
    dirs
}

/// SF Pro files shipped in BaseOS (`BaseOS/fonts/SF-Pro/`), installed
/// to `/usr/share/fonts/OTF` + `/usr/share/fonts/TTF`.
pub fn sf_pro_files() -> Vec<&'static str> {
    vec![
        "SF-Pro-Display-Regular.otf",
        "SF-Pro-Display-Medium.otf",
        "SF-Pro-Display-Semibold.otf",
        "SF-Pro-Display-Bold.otf",
        "SF-Pro-Text-Regular.otf",
        "SF-Pro-Text-Medium.otf",
    ]
}

/// Font weight on the 100..900 scale.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CTFontWeight(pub f32);

impl CTFontWeight {
    pub const ULTRALIGHT: Self = Self(100.0);
    pub const THIN: Self = Self(200.0);
    pub const LIGHT: Self = Self(300.0);
    pub const REGULAR: Self = Self(400.0);
    pub const MEDIUM: Self = Self(500.0);
    pub const SEMIBOLD: Self = Self(600.0);
    pub const BOLD: Self = Self(700.0);
    pub const HEAVY: Self = Self(800.0);
    pub const BLACK: Self = Self(900.0);

    pub fn new(value: f32) -> Self {
        Self(value.clamp(1.0, 1000.0))
    }

    /// Closest named SF Pro file stem for this weight.
    pub fn sf_pro_stem(self) -> &'static str {
        if self.0 < 250.0 {
            "Light"
        } else if self.0 < 450.0 {
            "Regular"
        } else if self.0 < 550.0 {
            "Medium"
        } else if self.0 < 650.0 {
            "Semibold"
        } else {
            "Bold"
        }
    }
}

impl From<f32> for CTFontWeight {
    fn from(value: f32) -> Self {
        Self::new(value)
    }
}

/// Italic / normal style selector.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CTFontStyle {
    #[default]
    Normal,
    Italic,
    Oblique,
}

/// Font descriptor: family + weight + style + size (mirrors
/// `CTFontDescriptorRef`).
#[derive(Clone, Debug, PartialEq)]
pub struct CTFontDescriptor {
    pub family: String,
    pub weight: CTFontWeight,
    pub style: CTFontStyle,
    pub size: f32,
}

impl CTFontDescriptor {
    pub fn new(family: impl Into<String>, size: f32) -> Self {
        Self {
            family: family.into(),
            weight: CTFontWeight::REGULAR,
            style: CTFontStyle::Normal,
            size: size.max(1.0),
        }
    }

    pub fn weight(mut self, weight: impl Into<CTFontWeight>) -> Self {
        self.weight = weight.into();
        self
    }

    pub fn style(mut self, style: CTFontStyle) -> Self {
        self.style = style;
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size.max(1.0);
        self
    }

    /// System UI font (SF Pro) at `size` with regular weight.
    pub fn system(size: f32) -> Self {
        Self::new(SF_PRO_FAMILY, size)
    }
}

/// Resolved font (mirrors `CTFontRef`): descriptor plus the file it
/// resolved to, if any.
#[derive(Clone, Debug)]
pub struct CTFont {
    pub descriptor: CTFontDescriptor,
    /// Resolved font file on disk, if the scan found one.
    pub file: Option<PathBuf>,
}

impl CTFont {
    pub fn new(descriptor: CTFontDescriptor) -> Self {
        Self {
            descriptor,
            file: None,
        }
    }

    pub fn system(size: f32) -> Self {
        Self::new(CTFontDescriptor::system(size))
    }

    pub fn family(&self) -> &str {
        &self.descriptor.family
    }

    pub fn size(&self) -> f32 {
        self.descriptor.size
    }

    pub fn weight(&self) -> CTFontWeight {
        self.descriptor.weight
    }

    pub fn italic(&self) -> bool {
        self.descriptor.style != CTFontStyle::Normal
    }
}

/// Runtime font registry: custom fonts plus a scan cache.
#[derive(Clone, Debug, Default)]
pub struct FontRegistry {
    /// Family name to font files registered at runtime.
    custom: HashMap<String, Vec<PathBuf>>,
    /// Raw font blobs registered at runtime (family hint).
    custom_data: Vec<(String, Vec<u8>)>,
    /// Deactivated families (hidden from resolution).
    disabled: HashSet<String>,
    /// Extra dirs prepended to the system scan.
    extra_dirs: Vec<PathBuf>,
}

impl FontRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a directory to the front of the scan order.
    pub fn add_directory(&mut self, dir: impl Into<PathBuf>) {
        self.extra_dirs.push(dir.into());
    }

    fn search_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = self.extra_dirs.clone();
        dirs.extend(system_font_dirs());
        dirs
    }

    /// Register a font file (TTF/OTF/WOFF/WOFF2) under `family`.
    /// Returns an error when the file does not exist.
    pub fn register_font_file(
        &mut self,
        family: impl Into<String>,
        path: impl Into<PathBuf>,
    ) -> std::io::Result<()> {
        let path = path.into();
        if !path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("font file not found: {}", path.display()),
            ));
        }
        self.custom.entry(family.into()).or_default().push(path);
        Ok(())
    }

    /// Register raw font data (TTF/OTF bytes) under `family`.
    pub fn register_font_data(&mut self, family: impl Into<String>, data: Vec<u8>) {
        self.custom_data.push((family.into(), data));
    }

    /// Activate a previously deactivated family.
    pub fn activate(&mut self, family: &str) {
        self.disabled.remove(family);
    }

    /// Deactivate a family so resolution skips it.
    pub fn deactivate(&mut self, family: &str) {
        self.disabled.insert(family.to_string());
    }

    pub fn is_active(&self, family: &str) -> bool {
        !self.disabled.contains(family)
    }

    /// Families visible to resolution: custom first, then files found
    /// by scanning font dirs (stem-based, without extension).
    pub fn families(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .custom
            .keys()
            .filter(|f| self.is_active(f))
            .cloned()
            .collect();
        for dir in self.search_dirs() {
            let entries = std::fs::read_dir(&dir).ok();
            let Some(entries) = entries else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                if !is_font_file(&path) {
                    continue;
                }
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let family = stem.to_string();
                    if self.is_active(&family) && !out.contains(&family) {
                        out.push(family);
                    }
                }
            }
        }
        // Always advertise SF Pro: Parley resolves it from fontconfig
        // even when no file stem matches exactly.
        if self.is_active(SF_PRO_FAMILY) && !out.contains(&SF_PRO_FAMILY.to_string()) {
            out.push(SF_PRO_FAMILY.to_string());
        }
        out.sort();
        out
    }

    /// Resolve `descriptor` to a font file. Custom registrations win,
    /// then a scan for family/weight/style file names, then any file
    /// of the family, then SF Pro fallback, then `None` (caller uses
    /// the system loader).
    pub fn resolve(&self, descriptor: &CTFontDescriptor) -> CTFont {
        let mut font = CTFont::new(descriptor.clone());
        if !self.is_active(&descriptor.family) {
            return self.fallback(descriptor);
        }
        if let Some(paths) = self.custom.get(&descriptor.family) {
            if let Some(path) = pick_weight_match(paths, descriptor) {
                font.file = Some(path);
                return font;
            }
        }
        let mut candidates = Vec::new();
        for dir in self.search_dirs() {
            let entries = std::fs::read_dir(&dir).ok();
            let Some(entries) = entries else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                if !is_font_file(&path) {
                    continue;
                }
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_lowercase();
                let want = descriptor.family.to_lowercase().replace(' ', "");
                if name.contains(&want) {
                    candidates.push(path);
                }
            }
        }
        if let Some(path) = pick_weight_match(&candidates, descriptor) {
            font.file = Some(path);
            return font;
        }
        self.fallback(descriptor)
    }

    fn fallback(&self, descriptor: &CTFontDescriptor) -> CTFont {
        let font = CTFont::new(descriptor.clone());
        if descriptor.family != SF_PRO_FAMILY && self.is_active(SF_PRO_FAMILY) {
            let probe = CTFontDescriptor {
                family: SF_PRO_FAMILY.to_string(),
                ..descriptor.clone()
            };
            let resolved = self.resolve_sans_recursion(&probe);
            if resolved.file.is_some() {
                return resolved;
            }
        }
        font
    }

    fn resolve_sans_recursion(&self, descriptor: &CTFontDescriptor) -> CTFont {
        let mut font = CTFont::new(descriptor.clone());
        if let Some(paths) = self.custom.get(&descriptor.family) {
            if let Some(path) = pick_weight_match(paths, descriptor) {
                font.file = Some(path);
                return font;
            }
        }
        font
    }

    /// Ordered fallback chain for `family`: itself (when active),
    /// then SF Pro variants, then generic system families. Renderers
    /// walk this chain when a glyph is missing (emoji, CJK).
    pub fn fallback_chain(&self, family: &str) -> Vec<String> {
        let mut chain = Vec::new();
        if self.is_active(family) {
            chain.push(family.to_string());
        }
        for probe in [
            SF_PRO_FAMILY,
            "SF Pro Text",
            "SF Pro Display",
            "Inter",
            "Cantarell",
            "Noto Sans",
            "DejaVu Sans",
            "Noto Color Emoji",
        ] {
            if probe != family && self.is_active(probe) && !chain.contains(&probe.to_string()) {
                chain.push(probe.to_string());
            }
        }
        chain
    }
}

fn is_font_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .as_deref(),
        Some("ttf") | Some("otf") | Some("ttc") | Some("woff") | Some("woff2")
    )
}

/// Pick the file whose name best matches weight/style hints.
fn pick_weight_match(paths: &[PathBuf], descriptor: &CTFontDescriptor) -> Option<PathBuf> {
    if paths.is_empty() {
        return None;
    }
    let stem = descriptor.weight.sf_pro_stem().to_lowercase();
    let italic = descriptor.style != CTFontStyle::Normal;
    let mut best: Option<(i32, &PathBuf)> = None;
    for path in paths {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_lowercase();
        let mut score = 0;
        if name.contains(&stem) {
            score += 2;
        }
        if italic && name.contains("italic") {
            score += 2;
        }
        if !italic && !name.contains("italic") {
            score += 1;
        }
        if score > best.map(|(s, _)| s).unwrap_or(-1) {
            best = Some((score, path));
        }
    }
    best.map(|(_, p)| p.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weight_maps_to_sf_pro_stems() {
        assert_eq!(CTFontWeight::REGULAR.sf_pro_stem(), "Regular");
        assert_eq!(CTFontWeight::SEMIBOLD.sf_pro_stem(), "Semibold");
        assert_eq!(CTFontWeight::BOLD.sf_pro_stem(), "Bold");
    }

    #[test]
    fn registry_registers_and_resolves_custom() {
        let dir = std::env::temp_dir();
        let path = dir.join("coretext-test-font.ttf");
        std::fs::write(&path, b"fake").unwrap();
        let mut registry = FontRegistry::new();
        registry
            .register_font_file("TestFam", &path)
            .expect("register");
        assert!(registry.families().contains(&"TestFam".to_string()));
        let font = registry.resolve(&CTFontDescriptor::new("TestFam", 14.0));
        assert_eq!(font.file, Some(path.clone()));
        registry.deactivate("TestFam");
        assert!(!registry.is_active("TestFam"));
        registry.activate("TestFam");
        assert!(registry.is_active("TestFam"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn fallback_chain_starts_with_family() {
        let registry = FontRegistry::new();
        let chain = registry.fallback_chain("MyApp");
        assert_eq!(chain.first().map(String::as_str), Some("MyApp"));
        assert!(chain.contains(&SF_PRO_FAMILY.to_string()));
    }

    #[test]
    fn missing_file_registration_errors() {
        let mut registry = FontRegistry::new();
        assert!(registry
            .register_font_file("Nope", "/definitely/not/here.ttf")
            .is_err());
    }
}
