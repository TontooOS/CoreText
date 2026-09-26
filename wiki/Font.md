# Font

Font loading and management (`src/font.rs`): files, system scan,
family/weight/style resolution, fallback chains and runtime
registration.

## CTFontWeight

```rust
pub struct CTFontWeight(pub f32);
```

| Value | Weight |
|---|---|
| `CTFontWeight::ULTRALIGHT` | `100.0` |
| `CTFontWeight::THIN` | `200.0` |
| `CTFontWeight::LIGHT` | `300.0` |
| `CTFontWeight::REGULAR` | `400.0` |
| `CTFontWeight::MEDIUM` | `500.0` |
| `CTFontWeight::SEMIBOLD` | `600.0` |
| `CTFontWeight::BOLD` | `700.0` |
| `CTFontWeight::HEAVY` | `800.0` |
| `CTFontWeight::BLACK` | `900.0` |

```rust
pub fn new(value: f32) -> Self
pub fn sf_pro_stem(self) -> &'static str
```

- Values clamp to `1.0..=1000.0`.
- `sf_pro_stem` maps the weight to the closest SF Pro file stem
  (`Light`, `Regular`, `Medium`, `Semibold`, `Bold`).

## CTFontStyle

```rust
pub enum CTFontStyle {
  Normal,
  Italic,
  Oblique,
}
```

## CTFontDescriptor

```rust
pub struct CTFontDescriptor {
  pub family: String,
  pub weight: CTFontWeight,
  pub style: CTFontStyle,
  pub size: f32,
}
```

```rust
pub fn new(family: impl Into<String>, size: f32) -> Self
pub fn weight(mut self, weight: impl Into<CTFontWeight>) -> Self
pub fn style(mut self, style: CTFontStyle) -> Self
pub fn size(mut self, size: f32) -> Self
pub fn system(size: f32) -> Self
```

- `system` builds an SF Pro descriptor (`SF_PRO_FAMILY`).
- Sizes below `1.0` clamp to `1.0`.

## CTFont

```rust
pub struct CTFont {
  pub descriptor: CTFontDescriptor,
  pub file: Option<PathBuf>,
}
```

```rust
pub fn new(descriptor: CTFontDescriptor) -> Self
pub fn system(size: f32) -> Self
pub fn family(&self) -> &str
pub fn size(&self) -> f32
pub fn weight(&self) -> CTFontWeight
pub fn italic(&self) -> bool
```

- `file` holds the resolved font file when the registry found one;
  `None` means the system loader resolves the family.

## System Directories

```rust
pub fn system_font_dirs() -> Vec<PathBuf>
pub fn sf_pro_files() -> Vec<&'static str>
```

- Scans `/usr/share/fonts/OTF`, `/usr/share/fonts/TTF`,
  `/usr/share/fonts`, `/Library/Fonts`, `/System/Library/Fonts`,
  `~/.fonts`, `~/.local/share/fonts` and `assets/fonts`.
- `sf_pro_files` lists the BaseOS SF Pro files installed to
  `/usr/share/fonts/OTF` + `/usr/share/fonts/TTF`.

## FontRegistry

```rust
pub struct FontRegistry {
  // custom files, raw blobs, disabled families, extra dirs
}
```

```rust
pub fn new() -> Self
pub fn add_directory(&mut self, dir: impl Into<PathBuf>)
pub fn register_font_file(&mut self, family: impl Into<String>, path: impl Into<PathBuf>) -> std::io::Result<()>
pub fn register_font_data(&mut self, family: impl Into<String>, data: Vec<u8>)
pub fn activate(&mut self, family: &str)
pub fn deactivate(&mut self, family: &str)
pub fn is_active(&self, family: &str) -> bool
pub fn families(&self) -> Vec<String>
pub fn resolve(&self, descriptor: &CTFontDescriptor) -> CTFont
pub fn fallback_chain(&self, family: &str) -> Vec<String>
```

- `register_font_file` returns `Err` when the file does not exist.
- `families` lists custom families first, then scanned file stems;
  SF Pro is always advertised (Parley resolves it via fontconfig).
- `resolve` prefers custom files, then scanned family files with a
  weight/style name match, then SF Pro, then an unresolved font.
- `fallback_chain` starts with the family itself (when active),
  then SF Pro variants, `Inter`, `Cantarell`, `Noto Sans`,
  `DejaVu Sans` and `Noto Color Emoji` for missing glyphs.

## Usage / Example

```rust
use coretext::{CTFontDescriptor, CTFontStyle, FontRegistry};

let mut registry = FontRegistry::new();
registry.register_font_file("MyApp", "/usr/share/fonts/MyApp.ttf")?;
registry.deactivate("Legacy");
let font = registry.resolve(&CTFontDescriptor::system(17.0));
let chain = registry.fallback_chain("MyApp");
```

## Cross References

- [Typeset.md](Typeset.md) – framesetter resolution and fallback
- [Render.md](Render.md) – crisp draws with resolved fonts
