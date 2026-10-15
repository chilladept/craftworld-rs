use crate::Options;
mod animation;
mod bytes;
mod diagnostics;
mod entries;
mod entry_order;
use entry_order::EntrySort;
mod material;
mod mesh;
mod navigation;
mod plan;
use navigation::{History, Route, Visit};
mod references;
use references::reference_groups;
mod folders;
mod preview;
mod resource_list;
pub fn folder_prefix(value: &str) -> Result<String, String> {
    folders::prefix(value)
}
use eframe::egui::{self, Color32, RichText};
use lbp_catalog::{Catalog, Filter, LoadControl};
use resource_list::ResourceSort;
use std::{
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
    sync::{atomic::Ordering, mpsc, Arc},
    time::{Duration, Instant},
};
const ACCENT: Color32 = Color32::from_rgb(111, 205, 182);
const ROW: f32 = 25.0;
struct Job {
    control: LoadControl,
    result: mpsc::Receiver<Result<Catalog, String>>,
    started: Instant,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.control.cancel();
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tab {
    Metadata,
    Dependencies,
    Reverse,
    Raw,
    Hex,
    Texture,
    Mesh,
    Material,
    Animation,
    Plan,
}
pub struct Explorer {
    catalog: Option<Arc<Catalog>>,
    job: Option<Job>,
    path: String,
    loaded_path: String,
    message: String,
    filter: Filter,
    last_filter: Option<Filter>,
    resource_sort: ResourceSort,
    last_sort: Option<ResourceSort>,
    folders: folders::Folders,
    initial_folder: Option<String>,
    visible: Vec<usize>,
    selected: Option<usize>,
    inspection: Option<usize>,
    tab: Tab,
    raw: String,
    history: History,
    jump: Option<usize>,
    renderer: String,
    initial_guid: Option<String>,
    capture: Option<PathBuf>,
    material_json: Option<PathBuf>,
    texture_json: Option<PathBuf>,
    mesh_json: Option<PathBuf>,
    animation_json: Option<PathBuf>,
    plan_json: Option<PathBuf>,
    capture_sent: bool,
    ready_frames: usize,
    navigation_qa: bool,
    started: Instant,
    io: bytes::IoState,
    preview: preview::Preview,
    mesh: mesh::MeshView,
    material: material::MaterialView,
    animation: animation::AnimationView,
    plan: plan::PlanView,
    archive_mode: bool,
    entry_selection: Option<(usize, usize)>,
    entry_jump: Option<(usize, usize)>,
    entry_visible: Vec<(usize, usize)>,
    entry_sort: EntrySort,
    last_entry_sort: Option<EntrySort>,
    initial_entry: Option<(usize, usize)>,
    census: lbp_catalog::census_summary::CorpusCounts,
}
// Source attachment/hex jobs are separate from JSON loading and carry cancellation.
pub fn capture_path(path: &Path) -> Result<PathBuf, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("Workspace absent")?;
    let allowed = root
        .join("local-analysis")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let destination = parent.join(path.file_name().ok_or("Capture needs filename")?);
    if !parent.starts_with(&allowed)
        || destination.exists()
        || destination.extension().is_none_or(|s| s != "png")
    {
        return Err("QA capture must be a new PNG inside ignored local-analysis".into());
    }
    Ok(destination)
}
impl Explorer {
    pub fn new(cc: &eframe::CreationContext<'_>, options: Options) -> Self {
        let qa_metadata = options.capture.is_some();
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        cc.egui_ctx.style_mut_of(egui::Theme::Dark, |style| {
            style.spacing.item_spacing = egui::vec2(8.0, 7.0);
            style.visuals.selection.bg_fill = Color32::from_rgb(36, 90, 79);
        });
        let renderer = cc
            .wgpu_render_state
            .as_ref()
            .map(|state| {
                let info = state.adapter.get_info();
                format!("{} · {:?}", info.name, info.backend)
            })
            .unwrap_or_else(|| "wgpu".into());
        println!("Viewer renderer: {renderer}");
        let mut app = Self {
            catalog: None,
            job: None,
            path: options
                .snapshot
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            loaded_path: String::new(),
            message: "Open a schema-2 scanner snapshot or explore the authored demo.".into(),
            filter: Filter {
                query: options.query,
                ..Default::default()
            },
            last_filter: None,
            resource_sort: ResourceSort::from_value(&options.sort),
            last_sort: None,
            folders: folders::Folders::default(),
            initial_folder: (!options.folder.is_empty()).then_some(options.folder),
            visible: Vec::new(),
            selected: None,
            inspection: None,
            tab: match options.tab.as_str() {
                "dependencies" => Tab::Dependencies,
                "referenced-by" => Tab::Reverse,
                "raw" => Tab::Raw,
                "hex" => Tab::Hex,
                "texture" => Tab::Texture,
                "mesh" => Tab::Mesh,
                "material" | "material-graph" | "material-tail" => Tab::Material,
                "animation" => Tab::Animation,
                "plan" | "plan-inventory" => Tab::Plan,
                _ => Tab::Metadata,
            },
            raw: String::new(),
            history: History::default(),
            jump: None,
            renderer,
            initial_guid: options.select_guid,
            capture: options.capture,
            material_json: options.material_json,
            texture_json: options.texture_json,
            mesh_json: options.mesh_json,
            animation_json: options.animation_json,
            plan_json: options.plan_json,
            capture_sent: false,
            ready_frames: 0,
            navigation_qa: options.navigation_qa,
            started: Instant::now(),
            io: bytes::IoState::new(options.game_dir),
            preview: preview::Preview::new(options.channel),
            mesh: mesh::MeshView::new(options.mesh_view == "solid", options.tab == "mesh")
                .with_textured(options.mesh_view == "textured"),
            material: material::MaterialView::new(
                matches!(options.tab.as_str(), "material-graph" | "material-tail"),
                options.tab == "material-tail",
            ),
            animation: animation::AnimationView::new(qa_metadata),
            plan: plan::PlanView::new(qa_metadata).with_inventory(options.tab == "plan-inventory"),
            archive_mode: options.entries,
            entry_selection: None,
            entry_jump: None,
            entry_visible: Vec::new(),
            entry_sort: EntrySort::from_value(&options.entry_sort),
            last_entry_sort: None,
            initial_entry: options.select_entry,
            census: options.census,
        };
        if options.demo {
            app.set_catalog(
                Catalog::build(lbp_catalog::demo_snapshot(), &LoadControl::default())
                    .expect("authored demo valid"),
                "Authored synthetic demo".into(),
            );
        } else if options.snapshot.is_some() {
            app.start_load(&cc.egui_ctx);
        }
        app
    }
    fn start_load(&mut self, ctx: &egui::Context) {
        if self.job.is_some() {
            return;
        }
        let path = PathBuf::from(self.path.trim());
        let control = LoadControl::default();
        let worker_control = control.clone();
        let (sender, result) = mpsc::sync_channel(1);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = lbp_catalog::load(&path, &worker_control);
            let _ = sender.send(result);
            ctx.request_repaint();
        });
        self.message = "Opening snapshot…".into();
        self.job = Some(Job {
            control,
            result,
            started: Instant::now(),
        });
    }
    fn set_catalog(&mut self, catalog: Catalog, name: String) {
        self.io.reset_source();
        self.preview.reset();
        self.mesh.reset();
        self.material.reset();
        self.animation.reset();
        self.plan.reset();
        self.entry_selection = None;
        self.entry_jump = None;
        self.jump = None;
        self.catalog = Some(Arc::new(catalog));
        self.loaded_path = name;
        self.selected = None;
        self.inspection = None;
        self.history.clear();
        self.last_filter = None;
        self.last_sort = None;
        self.last_entry_sort = None;
        self.folders = folders::Folders::default();
        if let Some(prefix) = self.initial_folder.take() {
            self.folders.prefix = prefix;
        }
        self.raw.clear();
        self.ready_frames = 0;
        self.message = "Snapshot loaded. All game data remains untouched.".into();
        if let Some((a, e)) = self.initial_entry.take() {
            if self.catalog.as_ref().is_some_and(|c| {
                c.snapshot
                    .archives
                    .get(a)
                    .and_then(|archive| archive.entries.get(e))
                    .is_some()
            }) {
                self.transition(
                    Visit {
                        route: Route::Entry(a, e),
                        tab: self.tab,
                    },
                    false,
                    false,
                );
            } else {
                self.message = "Requested archive entry is absent".into();
            }
        }
        if let Some(guid) = self.initial_guid.take() {
            let guid = guid.trim_start_matches("0x").to_ascii_lowercase();
            if let Some(index) = self
                .catalog
                .as_ref()
                .and_then(|c| c.by_guid.get(&guid))
                .and_then(|rows| rows.first())
                .copied()
            {
                self.select(index, false);
            } else {
                self.message = format!("GUID {guid} is absent from this snapshot.");
            }
        }
    }
    fn poll(&mut self) {
        let received = self
            .job
            .as_ref()
            .and_then(|job| match job.result.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err("Loader ended without a result".into()))
                }
                Err(mpsc::TryRecvError::Empty) => None,
            });
        if let Some(result) = received {
            let job = self.job.take().unwrap();
            if job.control.cancelled.load(Ordering::Relaxed) {
                self.message = "Load cancelled; previous snapshot retained.".into();
                return;
            }
            match result {
                Ok(catalog) => {
                    let elapsed = job.started.elapsed();
                    self.set_catalog(catalog, self.path.trim().into());
                    println!("Snapshot loaded in {:.3}s", elapsed.as_secs_f64());
                }
                Err(error) => self.message = error,
            }
        }
    }
    fn select(&mut self, index: usize, remember: bool) {
        self.transition(
            Visit {
                route: Route::Map(index),
                tab: self.tab,
            },
            remember,
            false,
        );
    }
    fn navigate(&mut self, index: usize) {
        self.transition(
            Visit {
                route: Route::Map(index),
                tab: Tab::Metadata,
            },
            true,
            true,
        );
    }
    fn cache_raw(&mut self) {
        self.raw=self.catalog.as_ref().and_then(|c|self.selected.map(|index| {
            // Raw diagnostic metadata only; no original resource bytes are exposed.
            let inspection=self.inspection.and_then(|i|c.snapshot.resource_inspections.get(i));
            serde_json::to_string_pretty(&serde_json::json!({"resource":c.snapshot.resources[index],
                "location_header":inspection.map(|l|serde_json::json!({"source_path":l.source_path,"offset":l.offset,
                    "size":l.size,"inspection_status":l.inspection.status,"header":l.inspection.header,
                    "dependency_status":l.inspection.dependency_status,"dependency_count_raw":l.inspection.dependency_count_raw}))})).unwrap_or_default()
        })).unwrap_or_default();
    }
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("craftworld-rs");
            ui.label(RichText::new("METADATA").color(ACCENT));
        });
        ui.horizontal(|ui| {
            ui.add_enabled(
                self.job.is_none(),
                egui::TextEdit::singleline(&mut self.path)
                    .hint_text("Path to scanner inventory JSON")
                    .desired_width(ui.available_width() - 190.0),
            );
            if ui
                .add_enabled(
                    self.job.is_none() && !self.path.trim().is_empty(),
                    egui::Button::new("Open snapshot"),
                )
                .clicked()
            {
                self.start_load(ui.ctx());
            }
            if ui
                .add_enabled(self.job.is_none(), egui::Button::new("Demo"))
                .clicked()
            {
                self.filter = Filter::default();
                self.set_catalog(
                    Catalog::build(lbp_catalog::demo_snapshot(), &LoadControl::default()).unwrap(),
                    "Authored synthetic demo".into(),
                );
            }
        });
        if let Some(job) = &self.job {
            ui.horizontal(|ui| {
                ui.spinner();
                let bytes = job.control.bytes_read.load(Ordering::Relaxed);
                let total = job.control.file_bytes.load(Ordering::Relaxed);
                let label = if job.control.phase.load(Ordering::Relaxed) == 0 {
                    format!(
                        "Reading {:.1} / {:.1} MB",
                        bytes as f64 / 1e6,
                        total as f64 / 1e6
                    )
                } else {
                    "Validating and indexing references…".into()
                };
                ui.label(label);
                if ui.button("Cancel").clicked() {
                    job.control.cancel();
                }
            });
        }
    }
    fn sources(&mut self, ui: &mut egui::Ui, catalog: &Catalog) {
        ui.heading("Sources");
        if ui
            .add(egui::Button::selectable(
                self.filter.archive.is_none(),
                "All MAP resources",
            ))
            .clicked()
        {
            self.filter.archive = None;
        }
        ui.label(format!(
            "{} rows · {} hashes",
            catalog.snapshot.resources.len(),
            catalog
                .snapshot
                .counts
                .get("unique_map_hashes")
                .unwrap_or(&0)
        ));
        ui.separator();
        ui.label(RichText::new("ARCHIVES").color(ACCENT));
        for archive in &catalog.snapshot.archives {
            let name = archive.path.rsplit('/').next().unwrap_or(&archive.path);
            if ui
                .add(egui::Button::selectable(
                    self.filter.archive.as_ref() == Some(&archive.path),
                    name,
                ))
                .on_hover_text(&archive.path)
                .clicked()
            {
                self.filter.archive = Some(archive.path.clone());
            }
            ui.small(format!(
                "{} entries · {:.1} MB",
                archive.entry_count,
                archive.file_size as f64 / 1e6
            ));
            if self.filter.archive.as_ref() == Some(&archive.path) {
                ui.small(format!("Table @ {}", archive.table_offset));
                ui.small(format!(
                    "Duplicate hash rows: {}",
                    archive.duplicate_hash_rows
                ));
            }
        }
        ui.separator();
        ui.label(RichText::new("INSPECTION").color(ACCENT));
        for (status, count) in &catalog.snapshot.metadata_status_counts {
            if ui
                .add(egui::Button::selectable(
                    self.filter.status.as_ref() == Some(status),
                    format!("{}  {count}", short_status(status)),
                ))
                .on_hover_text(status)
                .clicked()
            {
                self.filter.status = if self.filter.status.as_ref() == Some(status) {
                    None
                } else {
                    Some(status.clone())
                };
            }
        }
        if ui.button("Missing locations").clicked() {
            self.filter.status = Some("missing".into());
            self.filter.archive = None;
        }
        if ui.button("Clear filters").clicked() {
            self.filter = Filter::default();
            self.folders.prefix.clear();
        }
        ui.separator();
        ui.small("Archive counts describe entries. Resource lists retain every MAP alias.");
        ui.small("Byte access requires a validated source attachment.");
    }
    fn list(&mut self, ui: &mut egui::Ui, catalog: &Catalog) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(!self.archive_mode, "MAP resources")
                .clicked()
                && self.archive_mode
            {
                self.show_overview(false);
            }
            if ui
                .selectable_label(self.archive_mode, "Archive entries")
                .clicked()
                && !self.archive_mode
            {
                self.show_overview(true);
            }
        });
        if self.archive_mode {
            self.entry_list(ui, catalog);
            return;
        }
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.filter.query)
                    .hint_text("Search path, GUID or hash…")
                    .desired_width(ui.available_width() - 65.0),
            );
            if ui.button("Clear").clicked() {
                self.filter.query.clear();
            }
        });
        ui.small("Exact lookup: guid:00001001 or hash:40-digit-hex");
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("type")
                .selected_text(self.filter.kind.as_deref().unwrap_or("All types"))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.filter.kind, None, "All types");
                    for kind in &catalog.kinds {
                        ui.selectable_value(&mut self.filter.kind, Some(kind.to_owned()), kind);
                    }
                });
            egui::ComboBox::from_id_salt("resource_sort")
                .selected_text(format!("Sort: {}", self.resource_sort.label()))
                .show_ui(ui, |ui| {
                    for order in ResourceSort::ALL {
                        ui.selectable_value(&mut self.resource_sort, order, order.label());
                    }
                });
        });
        self.refresh_visible(catalog);
        self.folder_ui(ui, catalog);
        ui.horizontal(|ui| {
            ui.label(format!(
                "{} / {} rows",
                self.visible.len(),
                catalog.snapshot.resources.len()
            ));
        });
        ui.separator();
        let mut scroll = egui::ScrollArea::vertical()
            .id_salt("resource_list")
            .auto_shrink([false, false]);
        if let Some(index) = self.jump.take() {
            if let Some(position) = self.visible.iter().position(|i| *i == index) {
                scroll = scroll
                    .vertical_scroll_offset(position as f32 * (ROW + ui.spacing().item_spacing.y));
            }
        }
        scroll.show_rows(ui, ROW, self.visible.len(), |ui, range| {
            for visible in range {
                let index = self.visible[visible];
                let resource = &catalog.snapshot.resources[index];
                let label = format!("{}   {}", resource.map_row.guid, resource.map_row.path);
                let response = ui.add_sized(
                    [ui.available_width(), ROW],
                    egui::Button::selectable(
                        self.selected == Some(index),
                        RichText::new(label).monospace().size(12.0),
                    )
                    .truncate(),
                );
                let response = response.on_hover_text(format!(
                    "{}\n{}\n{} · {}",
                    resource.map_row.sha1,
                    resource.display_type(),
                    resource.header_status,
                    resource.resolution
                ));
                if response.clicked() {
                    self.select(index, true);
                }
            }
        });
    }
    fn detail(&mut self, ui: &mut egui::Ui, catalog: &Catalog) {
        if self.navigation_bar(ui) {
            return;
        }
        if let Some((a, e)) = self.entry_selection {
            self.entry_detail(ui, catalog, a, e);
            return;
        }
        let Some(index) = self.selected else {
            egui::ScrollArea::vertical()
                .id_salt("overview")
                .show(ui, |ui| self.overview(ui, catalog));
            return;
        };
        let resource = &catalog.snapshot.resources[index];
        ui.heading(
            resource
                .map_row
                .path
                .rsplit('/')
                .next()
                .unwrap_or(&resource.map_row.path),
        );
        ui.add(egui::Label::new(&resource.map_row.path).wrap());
        ui.horizontal(|ui| {
            ui.monospace(&resource.map_row.guid);
            if ui.small_button("Copy GUID").clicked() {
                ui.ctx().copy_text(resource.map_row.guid.clone());
            }
        });
        ui.horizontal(|ui| {
            ui.monospace(format!("{}…", &resource.map_row.sha1[..12]));
            if ui.small_button("Copy SHA1").clicked() {
                ui.ctx().copy_text(resource.map_row.sha1.clone());
            }
        });
        ui.label(
            RichText::new(short_status(&resource.header_status))
                .color(status_color(&resource.header_status)),
        );
        ui.horizontal_wrapped(|ui| {
            for (tab, label) in [
                (Tab::Metadata, "Metadata"),
                (Tab::Dependencies, "Depends on"),
                (Tab::Reverse, "Referenced by"),
                (Tab::Raw, "Raw metadata"),
                (Tab::Hex, "Hex bytes"),
                (Tab::Texture, "Texture"),
                (Tab::Mesh, "Mesh"),
                (Tab::Material, "Material"),
                (Tab::Animation, "Animation"),
                (Tab::Plan, "Plan"),
            ] {
                if ui
                    .add(egui::Button::selectable(self.tab == tab, label))
                    .clicked()
                {
                    self.tab = tab;
                }
            }
        });
        ui.separator();
        match self.tab {
            Tab::Metadata => {
                egui::ScrollArea::vertical()
                    .id_salt("metadata")
                    .show(ui, |ui| {
                        field(
                            ui,
                            "Confirmed type",
                            resource
                                .confirmed_type
                                .as_deref()
                                .unwrap_or("Not confirmed"),
                        );
                        field(
                            ui,
                            "Extension hint",
                            resource.type_hint.as_ref().map_or("Unknown", |h| &h.name),
                        );
                        field(ui, "Resolution", &resource.resolution);
                        field(ui, "Dependencies", &resource.dependency_status);
                        field(
                            ui,
                            "Declared bytes",
                            &resource.map_row.declared_size.to_string(),
                        );
                        field(ui, "MAP row", &resource.map_row.row_index.to_string());
                        field(
                            ui,
                            "Raw timestamp",
                            &resource.map_row.timestamp_raw.to_string(),
                        );
                        ui.separator();
                        ui.strong("Location candidates");
                        for candidate in &resource.archive_candidates {
                            ui.small(format!(
                                "{} @ {} · {} bytes",
                                candidate.archive, candidate.offset, candidate.size
                            ));
                            if !candidate.declared_size_matches {
                                ui.colored_label(Color32::YELLOW, "Declared size differs");
                            }
                        }
                        for candidate in &resource.loose_candidates {
                            ui.small(format!(
                                "{} · {} bytes · {}",
                                candidate.path, candidate.size, candidate.integrity
                            ));
                        }
                        let indices: Vec<_> = resource.inspection_indices().collect();
                        if indices.len() > 1 {
                            ui.label("Ambiguous candidates — choose an inspection below.");
                        }
                        for &i in &indices {
                            if ui
                                .add(egui::Button::selectable(
                                    self.inspection == Some(i),
                                    format!("Inspection {i}"),
                                ))
                                .clicked()
                            {
                                self.inspection = Some(i);
                                self.cache_raw();
                            }
                        }
                        if let Some(i) = self.inspection {
                            let metadata = &catalog.snapshot.resource_inspections[i].inspection;
                            field(ui, "Magic (hex)", &metadata.magic_hex);
                            field(ui, "Serialization", &metadata.serialization);
                            if let Some(reason) = &metadata.reason {
                                ui.colored_label(Color32::YELLOW, reason);
                            }
                            if let Some(header) = &metadata.header {
                                ui.strong("Binary header");
                                ui.monospace(
                                    serde_json::to_string_pretty(header).unwrap_or_default(),
                                );
                            }
                            field(
                                ui,
                                "Raw dependencies",
                                &metadata.dependencies.len().to_string(),
                            );
                        }
                        ui.separator();
                        ui.small("Metadata only. Indexed SHA1s are not payload integrity checks.");
                    });
            }
            Tab::Dependencies => self.dependencies(ui, catalog, index),
            Tab::Reverse => self.reverse(ui, catalog, index),
            Tab::Raw => {
                egui::ScrollArea::both().id_salt("raw").show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.raw.as_str())
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY),
                    );
                });
            }
            Tab::Hex | Tab::Texture | Tab::Mesh | Tab::Material | Tab::Animation | Tab::Plan => {
                let locations = resource
                    .archive_candidates
                    .iter()
                    .map(|c| lbp_catalog::attachment::LocationKey {
                        path: c.archive.clone(),
                        offset: c.offset,
                        size: u64::from(c.size),
                    })
                    .chain(resource.loose_candidates.iter().map(|c| {
                        lbp_catalog::attachment::LocationKey {
                            path: c.path.clone(),
                            offset: 0,
                            size: c.size,
                        }
                    }))
                    .collect::<Vec<_>>();
                if self.tab == Tab::Texture {
                    self.preview.ui(ui, self.io.source(), &locations);
                } else if self.tab == Tab::Mesh {
                    self.mesh.ui(
                        ui,
                        self.io.source(),
                        &locations,
                        self.catalog.as_ref().unwrap().clone(),
                    );
                    if let Some(target) = self.mesh.take_jump() {
                        self.navigate(target);
                    }
                } else if self.tab == Tab::Material {
                    self.material.ui(ui, self.io.source(), &locations, catalog);
                    if let Some(target) = self.material.take_jump() {
                        self.navigate(target);
                    }
                } else if self.tab == Tab::Animation {
                    self.animation.ui(ui, self.io.source(), &locations);
                } else if self.tab == Tab::Plan {
                    self.plan.ui(ui, self.io.source(), &locations);
                } else {
                    self.io.hex(ui, &locations);
                }
            }
        }
    }
    fn dependencies(&mut self, ui: &mut egui::Ui, catalog: &Catalog, index: usize) {
        let indices: Vec<_> = catalog.snapshot.resources[index]
            .inspection_indices()
            .collect();
        if indices.is_empty() {
            ui.label("No inspected location. Dependencies are not known.");
            return;
        }
        let previous = self.inspection;
        egui::ComboBox::from_id_salt("dep_location")
            .selected_text(format!(
                "Location {}",
                self.inspection.unwrap_or(indices[0])
            ))
            .show_ui(ui, |ui| {
                for i in &indices {
                    ui.selectable_value(&mut self.inspection, Some(*i), format!("Inspection {i}"));
                }
            });
        if previous != self.inspection {
            self.cache_raw();
        }
        let i = self.inspection.unwrap_or(indices[0]);
        let location = &catalog.snapshot.resource_inspections[i];
        ui.label(format!(
            "{} · {} raw entries",
            location.inspection.dependency_status,
            location.inspection.dependencies.len()
        ));
        egui::ScrollArea::vertical()
            .id_salt(("dependencies", i))
            .show_rows(ui, 90.0, catalog.outgoing[i].len(), |ui, range| {
                for row in range {
                    fixed_row(ui, 90.0, |ui| {
                        let link_index = catalog.outgoing[i][row];
                        let link = &catalog.snapshot.dependency_links[link_index];
                        let dep = &location.inspection.dependencies[link.dependency_index];
                        ui.horizontal(|ui| {
                            ui.monospace(format!("#{}  flags {:02x}", dep.index, dep.flags));
                            ui.small(
                                dep.known_type
                                    .as_ref()
                                    .map(|k| k.name.clone())
                                    .unwrap_or_else(|| format!("unknown type {}", dep.type_id)),
                            );
                        });
                        ui.small(format!(
                            "GUID {} · SHA1 {} · {}",
                            dep.guid.as_deref().unwrap_or("null"),
                            dep.sha1.as_deref().map(|h| &h[..12]).unwrap_or("null"),
                            link.status
                        ));
                        egui::ScrollArea::horizontal()
                            .id_salt(("target_aliases", link_index))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    for target in catalog.targets(link_index) {
                                        let r = &catalog.snapshot.resources[target];
                                        if ui
                                            .small_button(format!("Open {}", r.map_row.guid))
                                            .on_hover_text(&r.map_row.path)
                                            .clicked()
                                        {
                                            self.navigate(target);
                                        }
                                    }
                                });
                            });
                        ui.separator();
                    });
                }
            });
    }
    fn reverse(&mut self, ui: &mut egui::Ui, catalog: &Catalog, index: usize) {
        ui.label(format!(
            "{} incoming dependency entries",
            catalog.reverse[index].len()
        ));
        ui.small("References recorded in this snapshot only; aliases and duplicate entries are retained.");
        egui::ScrollArea::vertical()
            .id_salt(("reverse", index))
            .show_rows(ui, 58.0, catalog.reverse[index].len(), |ui, range| {
                for row in range {
                    fixed_row(ui, 58.0, |ui| {
                        let link = &catalog.snapshot.dependency_links[catalog.reverse[index][row]];
                        ui.small(format!(
                            "Inspection {} · dependency #{} · {}",
                            link.inspection_index, link.dependency_index, link.status
                        ));
                        egui::ScrollArea::horizontal()
                            .id_salt((
                                "source_aliases",
                                link.inspection_index,
                                link.dependency_index,
                            ))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    for source in
                                        &catalog.inspection_resources[link.inspection_index]
                                    {
                                        let r = &catalog.snapshot.resources[*source];
                                        if ui
                                            .small_button(format!("From {}", r.map_row.guid))
                                            .on_hover_text(&r.map_row.path)
                                            .clicked()
                                        {
                                            self.navigate(*source);
                                        }
                                    }
                                });
                            });
                        ui.separator();
                    });
                }
            });
    }
    fn overview(&self, ui: &mut egui::Ui, catalog: &Catalog) {
        ui.heading("Snapshot overview");
        field(
            ui,
            "Build",
            catalog
                .snapshot
                .build
                .get("title_id")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown"),
        );
        field(
            ui,
            "APP_VER",
            catalog
                .snapshot
                .build
                .get("app_version")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown"),
        );
        ui.label(format!(
            "{} MAP rows · {} inspected locations",
            catalog.snapshot.resources.len(),
            catalog.snapshot.resource_inspections.len()
        ));
        ui.label(format!(
            "{} dependency references",
            catalog.snapshot.dependency_links.len()
        ));
        if catalog.snapshot.partial_metadata {
            ui.colored_label(
                Color32::YELLOW,
                "Partial scan: inspect diagnostic statuses.",
            );
        }
        ui.separator();
        ui.strong("Choose a resource to inspect");
        ui.label("Browse an archive, filter by type or inspection status, or search a path, GUID or hash.");
        ui.separator();
        ui.scope(|ui| {
            ui.strong("Confirmed types by location");
            for (kind, count) in &catalog.snapshot.confirmed_type_counts {
                field(ui, kind, &count.to_string());
            }
            ui.separator();
            ui.strong("Reference resolution");
            for (status, count) in &catalog.snapshot.dependency_resolution_counts {
                field(ui, status, &count.to_string());
            }
            ui.separator();
            ui.small(&catalog.snapshot.source_scope);
            ui.small(&catalog.snapshot.payload_integrity);
        });
        ui.separator();
        ui.collapsing("Corpus counts (census files)", |ui| {
            use lbp_catalog::census_summary::LineKind;
            for (kind, text) in self.census.lines() {
                match kind {
                    LineKind::Heading => {
                        ui.strong(text);
                    }
                    LineKind::Value => {
                        ui.label(text);
                    }
                    LineKind::Note => {
                        ui.small(text);
                    }
                }
            }
        });
    }
    fn capture(&mut self, ctx: &egui::Context) {
        if self.capture.is_none() {
            return;
        }
        if self.started.elapsed() > Duration::from_secs(90) {
            eprintln!("QA capture timed out: {}", self.message);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if self.catalog.is_some()
            && self.job.is_none()
            && !self.io.busy()
            && !self.preview.busy()
            && !self.mesh.busy()
            && !self.material.busy()
            && !self.animation.busy()
            && !self.plan.busy()
        {
            self.ready_frames += 1;
        } else {
            self.ready_frames = 0;
        }
        if self.ready_frames >= 4 && !self.capture_sent {
            if std::mem::take(&mut self.navigation_qa) {
                if let Err(error) = self.qa_navigation_roundtrip() {
                    eprintln!("Navigation QA failed: {error}");
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                self.ready_frames = 0;
                ctx.request_repaint();
                return;
            }
            if let Some(path) = self.plan_json.take() {
                match self.plan.export_to(&path) {
                    Ok(()) => println!("Cached plan diagnostic JSON saved"),
                    Err(error) => eprintln!("Plan diagnostic JSON failed: {error}"),
                }
                ctx.request_repaint();
                return;
            }
            if let Some(path) = self.animation_json.take() {
                match self.animation.export_to(&path) {
                    Ok(()) => println!("Cached animation diagnostic JSON saved"),
                    Err(error) => eprintln!("Animation diagnostic JSON failed: {error}"),
                }
                ctx.request_repaint();
                return;
            }
            if let Some(path) = self.mesh_json.take() {
                match self.mesh.export_to(&path) {
                    Ok(()) => println!("Cached mesh diagnostic JSON saved"),
                    Err(error) => eprintln!("Mesh diagnostic JSON failed: {error}"),
                }
                ctx.request_repaint();
                return;
            }
            if let Some(path) = self.texture_json.take() {
                match self.preview.export_to(&path) {
                    Ok(()) => println!("Cached texture diagnostic JSON saved"),
                    Err(error) => eprintln!("Texture diagnostic JSON failed: {error}"),
                }
                ctx.request_repaint();
                return;
            }
            if let Some(path) = self.material_json.take() {
                match self.material.export_to(&path) {
                    Ok(()) => println!("Cached material diagnostic JSON saved"),
                    Err(error) => eprintln!("Material diagnostic JSON failed: {error}"),
                }
                ctx.request_repaint();
                return;
            }
            if self.archive_mode {
                if let Some(catalog) = &self.catalog {
                    let rows: Vec<_> = self.entry_visible.iter().take(10).map(|&(a,e)| {
                        let archive = &catalog.snapshot.archives[a];
                        let entry = &archive.entries[e];
                        serde_json::json!({"archive_index":a,"entry_position":e,
                            "archive":archive.path,"entry_index":entry.entry_index,
                            "sha1":entry.sha1,"offset":entry.offset,"size":entry.size,
                            "aliases":catalog.by_hash.get(&entry.sha1.to_ascii_lowercase()).map_or(0,Vec::len)})
                    }).collect();
                    println!(
                        "QA archive list: {}",
                        serde_json::json!({"sort":self.entry_sort.label(),
                        "count":self.entry_visible.len(),"selected":self.entry_selection,"first_rows":rows})
                    );
                }
            } else {
                if let Some(catalog) = &self.catalog {
                    let rows: Vec<_> = self
                        .visible
                        .iter()
                        .take(10)
                        .map(|&index| {
                            let row = &catalog.snapshot.resources[index].map_row;
                            serde_json::json!({"catalog_index":index,"map_row":row.row_index,
                            "path":row.path,"guid":row.guid,"sha1":row.sha1})
                        })
                        .collect();
                    println!(
                        "QA resource list: {}",
                        serde_json::json!({
                        "sort":self.resource_sort.label(),"count":self.visible.len(),
                        "folder_prefix":self.folders.prefix,
                        "child_folder_count":self.folders.children.len(),
                        "first_child_folders":self.folders.children.iter().take(10).collect::<Vec<_>>(),
                        "direct_rows":self.folders.direct,"ungrouped_rows":self.folders.ungrouped,
                        "selected_catalog_index":self.selected,"first_rows":rows})
                    );
                }
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.capture_sent = true;
        }
        let screenshot = ctx.input(|input| {
            input.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(image) = screenshot {
            let path = self.capture.take().unwrap();
            let data: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
            let result = File::options()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|e| e.to_string())
                .and_then(|file| {
                    use image::ImageEncoder;
                    image::codecs::png::PngEncoder::new(BufWriter::new(file))
                        .write_image(
                            &data,
                            image.width() as u32,
                            image.height() as u32,
                            image::ExtendedColorType::Rgba8,
                        )
                        .map_err(|e| e.to_string())
                });
            match result {
                Ok(()) => println!("QA capture saved: {}", path.display()),
                Err(error) => eprintln!("QA capture failed: {error}"),
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(40));
    }
}
impl eframe::App for Explorer {
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.poll();
        self.io.poll(ctx, self.catalog.as_ref());
        self.preview.poll(ctx, self.io.source());
        self.mesh.poll(ctx, self.io.source());
        self.material.poll(ctx, self.io.source());
        self.animation.poll(ctx, self.io.source());
        self.plan.poll(ctx, self.io.source());
        if self.job.is_some()
            || self.io.busy()
            || self.preview.busy()
            || self.mesh.busy()
            || self.material.busy()
            || self.animation.busy()
            || self.plan.busy()
        {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        self.navigation_shortcuts(ui.ctx());
        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));
        egui::Panel::top("source_toolbar")
            .show(ui, |ui| self.io.toolbar(ui, self.catalog.as_ref()));
        egui::Panel::bottom("footer").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.small(&self.message);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.small(&self.renderer);
                });
            });
        });
        if let Some(catalog) = self.catalog.clone() {
            if !self.archive_mode {
                self.refresh_visible(&catalog);
            }
            egui::Panel::left("sources")
                .default_size(220.0)
                .min_size(170.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("sources_scroll")
                        .show(ui, |ui| self.sources(ui, &catalog));
                });
            egui::Panel::right("details")
                .default_size(510.0)
                .min_size(380.0)
                .show(ui, |ui| self.detail(ui, &catalog));
            egui::CentralPanel::default().show(ui, |ui| self.list(ui, &catalog));
        } else {
            egui::CentralPanel::default().show(ui,|ui| {ui.add_space(50.0);ui.heading("Explore your scanner inventory");ui.label("Open the JSON produced by lbp-cli, or use Demo to see authored metadata examples.");ui.label("This first explorer displays resource identities and relationships. Attach your source and select Texture or Mesh for supported previews.");});
        }
        self.capture(ui.ctx());
    }
}
fn fixed_row(ui: &mut egui::Ui, height: f32, contents: impl FnOnce(&mut egui::Ui)) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.set_clip_rect(rect.intersect(ui.clip_rect()));
        ui.spacing_mut().item_spacing.y = 3.0;
        contents(ui);
    });
}
fn field(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(format!("{label}: ")).color(Color32::GRAY));
        ui.label(value);
    });
}
fn short_status(status: &str) -> &str {
    match status {
        "metadata_parsed" => "Parsed metadata",
        "identified_unparsed" => "Identified, payload unparsed",
        "metadata_partial_static_mesh" => "Static mesh, partial header",
        "unknown_magic" => "Unknown signature",
        "blocked_encrypted" => "Protected — stopped",
        "missing" => "Missing location",
        _ => status,
    }
}
fn status_color(status: &str) -> Color32 {
    if status == "metadata_parsed" {
        ACCENT
    } else if matches!(status, "malformed" | "blocked_encrypted" | "missing") {
        Color32::from_rgb(235, 179, 119)
    } else {
        Color32::from_rgb(167, 178, 195)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn demo() -> Explorer {
        Explorer::new(
            &eframe::CreationContext::_new_kittest(egui::Context::default()),
            Options {
                demo: true,
                ..Default::default()
            },
        )
    }
    #[test]
    fn navigation_clears_filters_retains_history_and_updates_raw() {
        let mut app = demo();
        app.select(0, false);
        app.filter.query = "missing".into();
        app.navigate(1);
        assert_eq!(app.selected, Some(1));
        assert_eq!(app.inspection, Some(1));
        assert!(app.history.can_back());
        assert_eq!(app.filter, Filter::default());
        assert!(app.raw.contains("synthetic/checker.tex"));
        assert!(app.travel(true));
        assert_eq!(app.inspection, Some(0));
        app.navigate(3);
        assert_eq!(app.inspection, None);
        assert!(app.raw.contains("\"location_header\": null"));
    }
    #[test]
    fn cancellation_and_failed_load_retain_previous_catalog() {
        let mut app = demo();
        app.select(1, false);
        for cancelled in [true, false] {
            let control = LoadControl::default();
            if cancelled {
                control.cancel();
            }
            let (tx, rx) = mpsc::sync_channel(1);
            app.job = Some(Job {
                control,
                result: rx,
                started: Instant::now(),
            });
            tx.send(Err("Authored malformed snapshot".into())).unwrap();
            app.poll();
            assert!(app.job.is_none());
            assert_eq!(app.catalog.as_ref().unwrap().snapshot.resources.len(), 6);
            assert_eq!(app.selected, Some(1));
            assert!(if cancelled {
                app.message.contains("cancelled")
            } else {
                app.message.contains("malformed")
            });
        }
    }
    #[test]
    fn capture_cannot_overwrite_or_leave_ignored_analysis() {
        assert!(capture_path(Path::new("README.md")).is_err());
        assert!(capture_path(Path::new("docs/new.png")).is_err());
        assert!(capture_path(Path::new("local-analysis/not-a-png.json")).is_err());
    }
    #[test]
    fn every_tab_renders_missing_protected_unknown_and_alias_rows_at_minimum_size() {
        let ctx = egui::Context::default();
        let mut app = Explorer::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Options {
                demo: true,
                ..Default::default()
            },
        );
        let mut frame = eframe::Frame::_new_kittest();
        for row in 0..6 {
            app.select(row, false);
            for tab in [
                Tab::Metadata,
                Tab::Dependencies,
                Tab::Reverse,
                Tab::Raw,
                Tab::Hex,
                Tab::Texture,
                Tab::Mesh,
                Tab::Material,
                Tab::Animation,
                Tab::Plan,
            ] {
                app.tab = tab;
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(980.0, 640.0),
                        )),
                        ..Default::default()
                    },
                    |ui| eframe::App::ui(&mut app, ui, &mut frame),
                );
                assert!(!output.shapes.is_empty());
                // This headless layout test deliberately has no GPU texture consumer.
                output.textures_delta.clear();
            }
        }
    }
    #[test]
    fn archive_mode_lists_orphans_and_navigates_to_alias_without_stale_hex() {
        let ctx = egui::Context::default();
        let mut snapshot = lbp_catalog::demo_snapshot();
        snapshot.archives[0].entries.push(serde_json::from_value(serde_json::json!({"entry_index":4,"sha1":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","offset":390,"size":5})).unwrap());
        snapshot.archives[0].entry_count = 5;
        snapshot.counts.insert("archive_entries".into(), 5);
        let mut app = Explorer::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Options::default(),
        );
        app.set_catalog(
            Catalog::build(snapshot, &LoadControl::default()).unwrap(),
            "synthetic".into(),
        );
        app.select_entry(0, 4);
        let mut frame = eframe::Frame::_new_kittest();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1320.0, 850.0),
                )),
                ..Default::default()
            },
            |ui| eframe::App::ui(&mut app, ui, &mut frame),
        );
        output.textures_delta.clear();
        assert_eq!(
            app.entry_visible,
            vec![(0, 0), (0, 1), (0, 2), (0, 3), (0, 4)]
        );
        assert_eq!(app.entry_selection, Some((0, 4)));
        app.navigate(1);
        assert!(!app.archive_mode);
        assert_eq!(app.entry_selection, None);
        assert_eq!(app.selected, Some(1));
    }
}
