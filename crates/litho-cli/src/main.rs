//! Standalone CLI (`litho`) wrapping `litho-core`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

use litho_core::error::LithoError;
use litho_core::generate;
use litho_core::scene::{GenerationOptions, ImageInput};
use litho_core::settings::{
    bundled_presets, list_presets, load_preset, CurvatureDirection, ExportFormat,
    LithophaneSettings, MatchDimensionMode, SizeMode, StyleId,
};
use litho_core::styles::StyleRegistry;

#[derive(Parser)]
#[command(
    name = "litho",
    version,
    about = "Generate watertight 3D-printable lithophanes (flat, curved, and multi-panel)."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Copy, ValueEnum)]
enum CliFormat {
    Stl,
    #[value(name = "3mf")]
    ThreeMf,
}

impl From<CliFormat> for ExportFormat {
    fn from(value: CliFormat) -> Self {
        match value {
            CliFormat::Stl => ExportFormat::Stl,
            CliFormat::ThreeMf => ExportFormat::ThreeMf,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum CliStyle {
    Flat,
    Curved,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a classic flat upright lithophane STL/3MF from one image.
    Make {
        image: PathBuf,
        #[arg(short, long, default_value = "out")]
        out: PathBuf,
        #[arg(long)]
        preset: Option<String>,
        #[arg(long)]
        max: Option<f64>,
        #[arg(long)]
        width: Option<f64>,
        #[arg(long)]
        height: Option<f64>,
        #[arg(long = "min-t")]
        min_t: Option<f64>,
        #[arg(long = "max-t")]
        max_t: Option<f64>,
        #[arg(long)]
        pixel_size: Option<f64>,
        #[arg(long)]
        no_supports: bool,
        #[arg(long, value_enum)]
        format: Option<CliFormat>,
        #[arg(long = "set")]
        overrides: Vec<String>,
    },
    /// Generate a curved lithophane with optional filleted built-in base and radial supports.
    Curved {
        image: PathBuf,
        #[arg(short, long, default_value = "out")]
        out: PathBuf,
        #[arg(long)]
        preset: Option<String>,
        #[arg(long, default_value_t = 120.0)]
        radius: f64,
        #[arg(long)]
        convex: bool,
        #[arg(long)]
        no_base: bool,
        #[arg(long = "base-thickness", default_value_t = 1.6)]
        base_thickness: f64,
        #[arg(long = "base-front", default_value_t = 8.0)]
        base_front: f64,
        #[arg(long = "base-back", default_value_t = 8.0)]
        base_back: f64,
        #[arg(long = "base-fillet", default_value_t = 4.0)]
        base_fillet: f64,
        #[arg(long)]
        supports: bool,
        #[arg(long)]
        max: Option<f64>,
        #[arg(long)]
        width: Option<f64>,
        #[arg(long)]
        height: Option<f64>,
        #[arg(long = "min-t")]
        min_t: Option<f64>,
        #[arg(long = "max-t")]
        max_t: Option<f64>,
        #[arg(long)]
        pixel_size: Option<f64>,
        #[arg(long, value_enum)]
        format: Option<CliFormat>,
        #[arg(long = "set")]
        overrides: Vec<String>,
    },
    /// Generate a multi-lithophane assembly with shared outer supports and bridges.
    Multi {
        #[arg(required = true, num_args = 2..)]
        images: Vec<PathBuf>,
        #[arg(short, long, default_value = "out")]
        out: PathBuf,
        #[arg(long)]
        preset: Option<String>,
        #[arg(short = 'd', long = "common-dim", default_value_t = 150.0)]
        common_dim: f64,
        #[arg(short, long, default_value_t = 12.0)]
        spacing: f64,
        #[arg(short = 'm', long = "item-mode")]
        item_modes: Vec<String>,
        #[arg(long)]
        pixel_size: Option<f64>,
        #[arg(long)]
        no_supports: bool,
        #[arg(long, value_enum)]
        format: Option<CliFormat>,
        #[arg(long = "set")]
        overrides: Vec<String>,
    },
    /// Batch-process multiple images matching a glob pattern.
    Batch {
        pattern: String,
        #[arg(long, value_enum, default_value_t = CliStyle::Flat)]
        style: CliStyle,
        #[arg(short, long, default_value = "out")]
        out: PathBuf,
        #[arg(long)]
        preset: Option<String>,
        #[arg(long)]
        max: Option<f64>,
        #[arg(long, value_enum)]
        format: Option<CliFormat>,
        #[arg(long = "set")]
        overrides: Vec<String>,
    },
    /// List all registered lithophane styles.
    Styles,
    /// List bundled presets.
    Presets,
    /// Print or save full settings JSON for editing.
    PresetDump {
        #[arg(long)]
        preset: Option<String>,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn load_base_settings(preset: Option<&str>) -> Result<LithophaneSettings, LithoError> {
    let Some(p) = preset else {
        return Ok(LithophaneSettings::default());
    };
    let path = Path::new(p);
    if path.exists() {
        let text = std::fs::read_to_string(path)?;
        let s: LithophaneSettings = serde_json::from_str(&text)
            .map_err(|e| LithoError::Settings(format!("invalid JSON preset {p}: {e}")))?;
        s.validate()?;
        Ok(s)
    } else {
        load_preset(p)
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_common_flags(
    mut s: LithophaneSettings,
    max: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
    min_t: Option<f64>,
    max_t: Option<f64>,
    pixel_size: Option<f64>,
    format: Option<CliFormat>,
    overrides: &[String],
) -> Result<LithophaneSettings, LithoError> {
    if let Some(m) = max {
        s.size.mode = SizeMode::MaxDimension;
        s.size.max_dimension_mm = m;
    }
    match (width, height) {
        (Some(w), Some(h)) => {
            s.size.mode = SizeMode::Absolute;
            s.size.width_mm = w;
            s.size.height_mm = h;
        }
        (Some(w), None) => {
            s.size.mode = SizeMode::Width;
            s.size.width_mm = w;
        }
        (None, Some(h)) => {
            s.size.mode = SizeMode::Height;
            s.size.height_mm = h;
        }
        (None, None) => {}
    }
    if let Some(v) = min_t {
        s.thickness.min_thickness_mm = v;
    }
    if let Some(v) = max_t {
        s.thickness.max_thickness_mm = v;
    }
    if let Some(v) = pixel_size {
        s.size.pixel_size_mm = v;
    }
    if let Some(f) = format {
        s.export.format = f.into();
    }
    if !overrides.is_empty() {
        s = s.apply_overrides(overrides)?;
    } else {
        s.validate()?;
    }
    Ok(s)
}

fn parse_match_mode(s: &str) -> Result<MatchDimensionMode, LithoError> {
    match s.trim().to_ascii_lowercase().as_str() {
        "long_side" => Ok(MatchDimensionMode::LongSide),
        "short_side" => Ok(MatchDimensionMode::ShortSide),
        "width" => Ok(MatchDimensionMode::Width),
        "height" => Ok(MatchDimensionMode::Height),
        other => Err(LithoError::Settings(format!(
            "unknown item-mode {other:?}; expected long_side, short_side, width, or height"
        ))),
    }
}

fn run(cli: Cli) -> Result<(), LithoError> {
    match cli.command {
        Commands::Styles => {
            let reg = StyleRegistry::new();
            for d in reg.descriptors() {
                let max_str = d
                    .max_images
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| "+".into());
                println!(
                    "{:<12} {:<8} ({}..{} images) - {}",
                    d.id.as_str(),
                    d.name,
                    d.min_images,
                    max_str,
                    d.tagline
                );
            }
            Ok(())
        }
        Commands::Presets => {
            for p in bundled_presets() {
                println!(
                    "{:<20} [{}] - {}",
                    p.name,
                    p.recommended_style.as_str(),
                    p.description
                );
            }
            let _ = list_presets();
            Ok(())
        }
        Commands::PresetDump { preset, output } => {
            let s = load_base_settings(preset.as_deref())?;
            let json = serde_json::to_string_pretty(&s)
                .map_err(|e| LithoError::Settings(format!("JSON serialize failed: {e}")))?;
            if let Some(path) = output {
                if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, json)?;
                println!("Wrote {}", path.display());
            } else {
                println!("{json}");
            }
            Ok(())
        }
        Commands::Make {
            image,
            out,
            preset,
            max,
            width,
            height,
            min_t,
            max_t,
            pixel_size,
            no_supports,
            format,
            overrides,
        } => {
            let mut s = load_base_settings(preset.as_deref())?;
            if no_supports {
                s.supports.enabled = false;
            }
            s = apply_common_flags(
                s, max, width, height, min_t, max_t, pixel_size, format, &overrides,
            )?;
            let mut prog = |msg: &str, frac: f32| println!("[{:3.0}%] {msg}", frac * 100.0);
            let scene = generate(
                StyleId::Flat,
                &[ImageInput::from_path(image)],
                &s,
                &GenerationOptions::default(),
                &mut prog,
            )?;
            print_saved(&scene.save_to_path(&out)?, &scene.report.warnings)?;
            Ok(())
        }
        Commands::Curved {
            image,
            out,
            preset,
            radius,
            convex,
            no_base,
            base_thickness,
            base_front,
            base_back,
            base_fillet,
            supports,
            max,
            width,
            height,
            min_t,
            max_t,
            pixel_size,
            format,
            overrides,
        } => {
            let mut s = load_base_settings(preset.as_deref())?;
            s.curved.radius_mm = radius;
            if convex {
                s.curved.direction = CurvatureDirection::Convex;
            }
            s.curved.base.enabled = !no_base;
            s.curved.base.thickness_mm = base_thickness;
            s.curved.base.front_projection_mm = base_front;
            s.curved.base.back_projection_mm = base_back;
            s.curved.base.corner_fillet_mm = base_fillet;
            s.curved.supports_enabled = supports;
            s = apply_common_flags(
                s, max, width, height, min_t, max_t, pixel_size, format, &overrides,
            )?;
            let mut prog = |msg: &str, frac: f32| println!("[{:3.0}%] {msg}", frac * 100.0);
            let scene = generate(
                StyleId::Curved,
                &[ImageInput::from_path(image)],
                &s,
                &GenerationOptions::default(),
                &mut prog,
            )?;
            if let Some(ref ci) = scene.report.curved_info {
                println!(
                    "Curved arc: {:.1}° · chord {:.1} mm · depth {:.1} mm",
                    ci.arc_angle_deg, ci.chord_width_mm, ci.arc_depth_mm
                );
            }
            print_saved(&scene.save_to_path(&out)?, &scene.report.warnings)?;
            Ok(())
        }
        Commands::Multi {
            images,
            out,
            preset,
            common_dim,
            spacing,
            item_modes,
            pixel_size,
            no_supports,
            format,
            overrides,
        } => {
            let mut s = load_base_settings(preset.as_deref())?;
            s.multi.common_dimension_mm = common_dim;
            s.multi.panel_spacing_mm = spacing;
            if no_supports {
                s.supports.enabled = false;
            }
            if !item_modes.is_empty() {
                let parsed: Vec<MatchDimensionMode> = item_modes
                    .iter()
                    .map(|m| parse_match_mode(m))
                    .collect::<Result<_, _>>()?;
                s.multi.item_modes = parsed;
            }
            s = apply_common_flags(s, None, None, None, None, None, pixel_size, format, &overrides)?;
            let inputs: Vec<ImageInput> = images.into_iter().map(ImageInput::from_path).collect();
            let mut prog = |msg: &str, frac: f32| println!("[{:3.0}%] {msg}", frac * 100.0);
            let scene = generate(
                StyleId::MultiFlat,
                &inputs,
                &s,
                &GenerationOptions::default(),
                &mut prog,
            )?;
            print_saved(&scene.save_to_path(&out)?, &scene.report.warnings)?;
            Ok(())
        }
        Commands::Batch {
            pattern,
            style,
            out,
            preset,
            max,
            format,
            overrides,
        } => {
            let paths: Vec<PathBuf> = glob::glob(&pattern)
                .map_err(|e| LithoError::Settings(format!("invalid glob pattern: {e}")))?
                .filter_map(Result::ok)
                .collect();
            if paths.is_empty() {
                return Err(LithoError::Image(format!("no files matched pattern {pattern:?}")));
            }
            let s = apply_common_flags(
                load_base_settings(preset.as_deref())?,
                max,
                None,
                None,
                None,
                None,
                None,
                format,
                &overrides,
            )?;
            let style_id = match style {
                CliStyle::Flat => StyleId::Flat,
                CliStyle::Curved => StyleId::Curved,
            };
            for path in paths {
                println!("== {}", path.display());
                let mut prog = |_: &str, _: f32| {};
                let scene = generate(
                    style_id,
                    &[ImageInput::from_path(path)],
                    &s,
                    &GenerationOptions::default(),
                    &mut prog,
                )?;
                print_saved(&scene.save_to_path(&out)?, &scene.report.warnings)?;
            }
            Ok(())
        }
    }
}

fn print_saved(paths: &[PathBuf], warnings: &[String]) -> Result<(), LithoError> {
    for p in paths {
        let mb = std::fs::metadata(p)?.len() as f64 / 1e6;
        println!("Wrote {} ({mb:.2} MB)", p.display());
    }
    for w in warnings {
        eprintln!("Warning: {w}");
    }
    Ok(())
}
