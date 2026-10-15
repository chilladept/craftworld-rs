//! Native metadata/hex viewer. Original-file I/O stays in the shared read-only library.
mod app;
use std::{path::PathBuf, process::ExitCode, time::Instant};
#[derive(Default)]
pub struct Options {
    pub snapshot: Option<PathBuf>,
    pub demo: bool,
    pub check: bool,
    pub query: String,
    pub sort: String,
    pub entry_sort: String,
    pub folder: String,
    pub mesh_view: String,
    pub select_guid: Option<String>,
    pub tab: String,
    pub capture: Option<PathBuf>,
    pub material_json: Option<PathBuf>,
    pub texture_json: Option<PathBuf>,
    pub mesh_json: Option<PathBuf>,
    pub animation_json: Option<PathBuf>,
    pub plan_json: Option<PathBuf>,
    pub game_dir: Option<PathBuf>,
    pub select_entry: Option<(usize, usize)>,
    pub entries: bool,
    pub channel: usize,
    pub navigation_qa: bool,
    /// Show the window without activating it (always on for `--qa-capture`).
    pub no_activate: bool,
    pub census: lbp_catalog::census_summary::CorpusCounts,
}
fn options() -> Result<Option<Options>, String> {
    let mut out = Options::default();
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let flag = arg.to_str().ok_or("Flag must be UTF-8")?;
        if flag == "--help" || flag == "-h" {
            println!("craftworld-rs D40 - metadata and bounded hex\nUsage: lbp-viewer [--snapshot FILE | --demo] [--game-dir EXTRACTED_DIR]\n       lbp-viewer --check-snapshot FILE [--game-dir EXTRACTED_DIR]\nOptions: --mesh-view wireframe|solid|textured, --folder MAP_PREFIX, --query TEXT, --sort map|path|guid|hash, --entry-sort table|offset|size|hash, --select-guid HEX, --entries, --select-entry ARCHIVE_INDEX:ENTRY_INDEX\nSearch: path/text, guid:8-digit-hex, hash:40-digit-hex.\nGame files are read only after source validation; protected resources stop at the boundary.\nPlain 2D GTF/TEX BC1/BC3 and TEX/DDS BGRA8 top-mip previews; ordinary MSH wireframe/neutral solid/direct diffuse and modern ANMb/PLNb metadata; other layouts remain unsupported.\nQA only: --qa-capture NEW_PNG_UNDER_LOCAL_ANALYSIS --qa-tab metadata|dependencies|referenced-by|raw|hex|texture|mesh|material|material-graph|material-tail|animation|plan|plan-inventory; --qa-channel rgba|r|g|b|a; --qa-material-json NEW_JSON_UNDER_LOCAL_ANALYSIS (with material capture); --qa-texture-json NEW_JSON_UNDER_LOCAL_ANALYSIS (with texture capture); --qa-mesh-json NEW_JSON_UNDER_LOCAL_ANALYSIS (with mesh capture); --qa-animation-json NEW_JSON_UNDER_LOCAL_ANALYSIS (with animation capture); --qa-plan-json NEW_JSON_UNDER_LOCAL_ANALYSIS (with plan capture)");
            println!(
                "Corpus counts: {}",
                lbp_catalog::census_summary::CENSUS_FLAGS_HELP
            );
            println!("QA navigation: --demo --qa-navigation-roundtrip --qa-capture NEW_PNG_UNDER_LOCAL_ANALYSIS (fixed synthetic route/tab checks)");
            println!(
                "--no-activate: show the window without taking focus (always on with --qa-capture)"
            );
            return Ok(None);
        }
        if flag == "--demo" {
            out.demo = true;
            continue;
        }
        if flag == "--entries" {
            out.entries = true;
            continue;
        }
        if flag == "--qa-navigation-roundtrip" {
            out.navigation_qa = true;
            continue;
        }
        if flag == "--no-activate" {
            out.no_activate = true;
            continue;
        }
        if let Some(role) = lbp_catalog::census_summary::census_flag(flag) {
            let path = args.next().ok_or_else(|| format!("{flag} needs a FILE"))?;
            out.census.load(role, std::path::Path::new(&path))?;
            continue;
        }
        let value = args.next().ok_or_else(|| format!("{flag} needs a value"))?;
        match flag {
            "--snapshot" => out.snapshot = Some(value.into()),
            "--game-dir" => out.game_dir = Some(value.into()),
            "--select-entry" => {
                let text = value.into_string().map_err(|_| "Entry must be UTF-8")?;
                let (archive, entry) = text
                    .split_once(':')
                    .ok_or("Entry needs ARCHIVE_INDEX:ENTRY_INDEX")?;
                out.select_entry = Some((
                    archive.parse().map_err(|_| "Invalid archive index")?,
                    entry.parse().map_err(|_| "Invalid entry index")?,
                ));
                out.entries = true;
            }
            "--check-snapshot" => {
                out.snapshot = Some(value.into());
                out.check = true;
            }
            "--query" => out.query = value.into_string().map_err(|_| "Query must be UTF-8")?,
            "--mesh-view" => {
                out.mesh_view = value.into_string().map_err(|_| "Mesh view must be UTF-8")?;
                if !matches!(out.mesh_view.as_str(), "wireframe" | "solid" | "textured") {
                    return Err("Mesh view needs wireframe/solid/textured".into());
                }
            }
            "--folder" => {
                out.folder =
                    app::folder_prefix(&value.into_string().map_err(|_| "Folder must be UTF-8")?)?;
            }
            "--sort" => {
                out.sort = value.into_string().map_err(|_| "Sort must be UTF-8")?;
                if !matches!(out.sort.as_str(), "map" | "path" | "guid" | "hash") {
                    return Err("Sort needs map/path/guid/hash".into());
                }
            }
            "--entry-sort" => {
                out.entry_sort = value
                    .into_string()
                    .map_err(|_| "Entry sort must be UTF-8")?;
                if !matches!(
                    out.entry_sort.as_str(),
                    "table" | "offset" | "size" | "hash"
                ) {
                    return Err("Entry sort needs table/offset/size/hash".into());
                }
            }
            "--select-guid" => {
                out.select_guid = Some(value.into_string().map_err(|_| "GUID must be UTF-8")?)
            }
            "--qa-tab" => out.tab = value.into_string().map_err(|_| "Tab must be UTF-8")?,
            "--qa-channel" => {
                out.channel = match value.to_str() {
                    Some("rgba") => 0,
                    Some("r") => 1,
                    Some("g") => 2,
                    Some("b") => 3,
                    Some("a") => 4,
                    _ => return Err("Channel needs rgba/r/g/b/a".into()),
                }
            }
            "--qa-capture" => out.capture = Some(app::capture_path(&PathBuf::from(value))?),
            "--qa-material-json" => out.material_json = Some(value.into()),
            "--qa-texture-json" => out.texture_json = Some(value.into()),
            "--qa-mesh-json" => out.mesh_json = Some(value.into()),
            "--qa-animation-json" => out.animation_json = Some(value.into()),
            "--qa-plan-json" => out.plan_json = Some(value.into()),
            _ => return Err(format!("Unknown argument {flag}; use --help")),
        }
    }
    if out.demo && out.snapshot.is_some() {
        return Err("Choose --demo or --snapshot".into());
    }
    if out.check && !out.folder.is_empty() {
        return Err("--folder is a viewer browsing option, not a snapshot check filter".into());
    }
    if out.navigation_qa && (!out.demo || out.capture.is_none()) {
        return Err("Navigation QA requires --demo and --qa-capture".into());
    }
    if out.select_guid.is_some() && out.select_entry.is_some() {
        return Err("Choose a GUID or archive-entry selection".into());
    }
    if out.capture.is_some() && !out.demo && out.snapshot.is_none() {
        return Err("QA capture needs a snapshot or demo".into());
    }
    if out.material_json.is_some()
        && (out.capture.is_none()
            || out.game_dir.is_none()
            || out.snapshot.is_none()
            || !matches!(
                out.tab.as_str(),
                "material" | "material-graph" | "material-tail"
            ))
    {
        return Err(
            "QA material JSON requires capture, snapshot, game directory and a material tab".into(),
        );
    }
    if out.texture_json.is_some()
        && (out.capture.is_none()
            || out.game_dir.is_none()
            || out.snapshot.is_none()
            || out.tab != "texture")
    {
        return Err(
            "QA texture JSON requires capture, snapshot, game directory and texture tab".into(),
        );
    }
    if out.mesh_json.is_some()
        && (out.capture.is_none()
            || out.game_dir.is_none()
            || out.snapshot.is_none()
            || out.tab != "mesh")
    {
        return Err("QA mesh JSON requires capture, snapshot, game directory and mesh tab".into());
    }
    if out.animation_json.is_some()
        && (out.capture.is_none()
            || out.game_dir.is_none()
            || out.snapshot.is_none()
            || out.tab != "animation")
    {
        return Err(
            "QA animation JSON requires capture, snapshot, game directory and animation tab".into(),
        );
    }
    if out.plan_json.is_some()
        && (out.capture.is_none()
            || out.game_dir.is_none()
            || out.snapshot.is_none()
            || !matches!(out.tab.as_str(), "plan" | "plan-inventory"))
    {
        return Err("QA plan JSON requires capture, snapshot, game directory and plan tab".into());
    }
    if !matches!(
        out.tab.as_str(),
        "" | "metadata"
            | "dependencies"
            | "referenced-by"
            | "raw"
            | "hex"
            | "texture"
            | "mesh"
            | "material"
            | "material-graph"
            | "material-tail"
            | "animation"
            | "plan"
            | "plan-inventory"
    ) {
        return Err("Unknown QA tab".into());
    }
    Ok(Some(out))
}
fn run() -> Result<(), String> {
    let Some(options) = options()? else {
        return Ok(());
    };
    if options.check {
        let start = Instant::now();
        let catalog = lbp_catalog::load(
            options.snapshot.as_ref().ok_or("Snapshot required")?,
            &lbp_catalog::LoadControl::default(),
        )?;
        let filter = lbp_catalog::Filter {
            query: options.query,
            ..Default::default()
        };
        println!(
            "{}",
            serde_json::json!({"schema_version":catalog.snapshot.schema_version,"map_rows":catalog.snapshot.resources.len(),
            "archives":catalog.snapshot.archives.len(),"inspected_locations":catalog.snapshot.resource_inspections.len(),
            "dependency_links":catalog.snapshot.dependency_links.len(),"filtered_rows":catalog.filtered(&filter).len(),
            "load_seconds":start.elapsed().as_secs_f64()})
        );
        if let Some(source) = options.game_dir {
            let attached = lbp_catalog::attachment::AttachedSource::attach(
                &source,
                &catalog,
                &lbp_catalog::LoadControl::default(),
            )?;
            let mut report = serde_json::json!({"source_attached":true,"archives":attached.archive_count,"entries":attached.entry_count,"validation_bytes":attached.validation_bytes});
            if let Some((a, e)) = options.select_entry {
                let archive = catalog
                    .snapshot
                    .archives
                    .get(a)
                    .ok_or("Archive index absent")?;
                let entry = archive.entries.get(e).ok_or("Entry index absent")?;
                let key = lbp_catalog::attachment::LocationKey {
                    path: archive.path.clone(),
                    offset: entry.offset,
                    size: u64::from(entry.size),
                };
                let page = attached.read_hex(&key, 0, lbp_catalog::attachment::HEX_PAGE_BYTES)?;
                report["hex_bytes_read"] = page.bytes.len().into();
                report["hex_absolute_offset"] = page.absolute_offset.into();
            }
            println!("{report}");
        }
        return Ok(());
    }
    let native = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1320.0, 850.0])
            .with_min_inner_size([980.0, 640.0])
            // Never steal focus during QA captures: winit shows an inactive window with
            // SW_SHOWNOACTIVATE on Windows.
            .with_active(options.capture.is_none() && !options.no_activate),
        ..Default::default()
    };
    eframe::run_native(
        "craftworld-rs",
        native,
        Box::new(|cc| Ok(Box::new(app::Explorer::new(cc, options)))),
    )
    .map_err(|e| format!("Native viewer: {e}"))
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("lbp-viewer: {e}");
            ExitCode::FAILURE
        }
    }
}
