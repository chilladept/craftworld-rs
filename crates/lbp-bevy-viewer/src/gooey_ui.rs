//! U6: Bevy adapter for the `lbp-gooey` UI model, plus the `--ui-demo` screen.
//!
//! Each frame the active screen rebuilds its `UiTree`; when the tree differs from the last
//! one, the bevy_ui entities are despawned and spawned again (menus are small, and focus is
//! kept by widget id, so a full rebuild is simpler than a keyed diff). Keyboard arrows move
//! focus with `lbp_gooey::navigate` over the laid-out rectangles; Enter/Space activate
//! (Enter only on screens with a text box);
//! mouse hover focuses and click activates. Gamepad input is not wired (bevy_gilrs is not
//! enabled). Text uses the generic sans-serif family resolved through the OS fonts
//! (Bevy `system_font_discovery`; not bundled), so strings with characters outside Bevy's
//! built-in subset font render; game fonts (U2) are shown by the U3 font sheet instead.
//! The demo content is authored and synthetic: no game data, not a copy of game screens.
//! Other screens (U7 `--slot-browser`) plug in through [`run_gooey`]; game images reach the
//! adapter as decoded RGBA keyed by [`ImageRef`].
use bevy::prelude::*;
use lbp_gooey::{
    navigate, settle_focus, Align, Builder, Direction, FrameInput, ImageRef, Layout, PanelStyle,
    Rect as UiRectangle, TextStyle, UiTree, Uid, Widget,
};
use std::collections::HashMap;

#[derive(Component, Clone, Copy)]
struct GooeyId(Uid);

/// Text typed into a screen that takes text (`run_gooey` `text`): characters are appended,
/// Backspace removes the last one. With a text box, Space types instead of activating.
pub type SharedText = std::sync::Arc<std::sync::Mutex<String>>;

#[derive(Resource)]
struct GooeyText(Option<SharedText>);

/// The active screen: builds this frame's tree from the input.
pub type ScreenFn = Box<dyn FnMut(FrameInput) -> UiTree + Send + Sync>;

#[derive(Resource)]
struct GooeyScreen(ScreenFn);

/// Decoded game image for an [`ImageRef`] (RGBA8, sRGB).
pub struct GooeyImage {
    pub image: ImageRef,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Resource, Default)]
struct GooeyImages(HashMap<ImageRef, Handle<Image>>);

/// Loads a game image the first time a screen shows it (None: not loadable, not retried).
pub type ImageLoader = Box<dyn FnMut(ImageRef) -> Option<GooeyImage> + Send + Sync>;

/// Images loaded per frame, so a page of icons appears over a few frames.
const LOADS_PER_FRAME: usize = 8;

#[derive(Resource)]
struct GooeyLoader {
    load: Option<ImageLoader>,
    failed: std::collections::HashSet<ImageRef>,
}

#[derive(Resource, Default)]
struct GooeyHost {
    focused: Option<Uid>,
    activated: Option<Uid>,
    tree: UiTree,
    root: Option<Entity>,
    buttons: HashMap<Uid, Entity>,
    rebuilds: u64,
}

/// Authored demo state (synthetic, Popit-like tabs and an item grid).
#[derive(Default)]
struct DemoState {
    tab: usize,
    picked: Option<String>,
}

const TABS: [&str; 4] = ["Materials", "Objects", "Stickers", "Tools"];
const ROOT: Uid = Uid(0x5549_0000);
const TAB_BASE: u64 = 0x5549_1000;
const ITEM_BASE: u64 = 0x5549_2000;

fn demo_screen(input: FrameInput, state: &mut DemoState) -> UiTree {
    let mut b = Builder::new(ROOT, input);
    b.frame(
        Uid(0x5549_0001),
        Layout::Column,
        Align::Start,
        PanelStyle::Filled,
        16.,
        10.,
        |b| {
            b.text(
                "UI layer demo (authored synthetic content, not game UI)",
                TextStyle::Title,
            );
            b.frame(
                Uid(0x5549_0002),
                Layout::Row,
                Align::Start,
                PanelStyle::Plain,
                0.,
                8.,
                |b| {
                    for (i, name) in TABS.iter().enumerate() {
                        if b.button(Uid(TAB_BASE + i as u64), *name).activated {
                            state.tab = i;
                        }
                    }
                },
            );
            b.frame(
                Uid(0x5549_0003),
                Layout::Wrap,
                Align::Start,
                PanelStyle::Bubble,
                8.,
                8.,
                |b| {
                    for i in 0..12u64 {
                        let shade = (40 + i * 16) as u8;
                        let swatch = match state.tab {
                            0 => [shade, 90, 60, 255],
                            1 => [60, shade, 90, 255],
                            2 => [90, 60, shade, 255],
                            _ => [shade, shade, 60, 255],
                        };
                        let label = format!("{} {}", TABS[state.tab], i + 1);
                        let enabled = i != 11;
                        let uid = Uid(ITEM_BASE + state.tab as u64 * 0x100 + i);
                        if b.button_with(
                            uid,
                            label.clone(),
                            Some(ImageRef::Swatch(swatch)),
                            enabled,
                        )
                        .activated
                        {
                            state.picked = Some(label);
                        }
                    }
                },
            );
            b.text(
                format!(
                    "Picked: {}   |   arrows move, Enter activates, mouse works too",
                    state.picked.as_deref().unwrap_or("nothing")
                ),
                TextStyle::Small,
            );
        },
    );
    b.finish()
}

pub fn run_ui_demo() {
    let mut state = DemoState::default();
    run_gooey(
        "UI layer demo",
        Box::new(move |input| demo_screen(input, &mut state)),
        Vec::new(),
        None,
        None,
    );
}

/// Runs a gooey screen in its own window (second monitor, `--qa-capture` aware). `images`
/// are uploaded at start; `loader` (optional) supplies images the tree shows later.
pub fn run_gooey(
    title: &str,
    screen: ScreenFn,
    images: Vec<GooeyImage>,
    loader: Option<ImageLoader>,
    text: Option<SharedText>,
) {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.025, 0.035, 0.045)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("craftworld-rs - {title}"),
                resolution: (1280, 800).into(),
                visible: false,
                focused: crate::activate_on_show(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(DemoCapture(demo_capture_path()))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        });
    add_gooey(&mut app, screen, images, loader, text);
    app.add_systems(Update, crate::place_on_second_monitor.before(gooey_input))
        .add_systems(Update, demo_capture.after(gooey_paint_focus))
        .run();
}

/// Where the screen's UI is drawn: the window's camera (`camera` None) or a camera that
/// renders to a texture (the Puter screen, `puter.rs`). While `pending`, nothing is built
/// (the texture camera does not exist yet); a change rebuilds the tree on the new target.
#[derive(Resource, Default)]
pub struct GooeyTarget {
    pub camera: Option<Entity>,
    pub pending: bool,
}

/// Adds a gooey screen to an app that already has its plugins and a camera: input, rebuild
/// and focus painting each frame. `images` are uploaded at start; `loader` (optional)
/// supplies images the tree shows later.
pub fn add_gooey(
    app: &mut App,
    screen: ScreenFn,
    mut images: Vec<GooeyImage>,
    loader: Option<ImageLoader>,
    text: Option<SharedText>,
) {
    app.init_resource::<GooeyHost>()
        .init_resource::<GooeyTarget>()
        .insert_resource(GooeyScreen(screen))
        .insert_resource(GooeyText(text))
        .insert_resource(GooeyLoader {
            load: loader,
            failed: Default::default(),
        })
        .add_systems(
            Startup,
            move |mut commands: Commands, mut assets: ResMut<Assets<Image>>| {
                let mut handles = HashMap::new();
                for i in images.drain(..) {
                    if let Some(image) = gpu_image(i.width, i.height, i.rgba) {
                        handles.insert(i.image, assets.add(image));
                    }
                }
                commands.insert_resource(GooeyImages(handles));
            },
        )
        .add_systems(
            Update,
            (gooey_input, gooey_rebuild, gooey_paint_focus).chain(),
        );
}

fn gooey_input(
    keys: Res<ButtonInput<KeyCode>>,
    interactions: Query<(&GooeyId, &Interaction), Changed<Interaction>>,
    laid_out: Query<(&GooeyId, &ComputedNode, &UiGlobalTransform), With<Button>>,
    mut host: ResMut<GooeyHost>,
    mut typed: MessageReader<bevy::input::keyboard::KeyboardInput>,
    text: Res<GooeyText>,
) {
    if let Some(shared) = &text.0 {
        let mut value = shared.lock().unwrap_or_else(|e| e.into_inner());
        for key in typed.read() {
            if !key.state.is_pressed() {
                continue;
            }
            if key.key_code == KeyCode::Backspace {
                value.pop();
            } else if let Some(t) = &key.text {
                value.extend(t.chars().filter(|c| !c.is_control()));
            }
        }
    }
    for (id, interaction) in &interactions {
        match interaction {
            Interaction::Pressed => {
                host.focused = Some(id.0);
                host.activated = Some(id.0);
            }
            Interaction::Hovered => host.focused = Some(id.0),
            Interaction::None => {}
        }
    }
    let direction = [
        (KeyCode::ArrowUp, Direction::Up),
        (KeyCode::ArrowDown, Direction::Down),
        (KeyCode::ArrowLeft, Direction::Left),
        (KeyCode::ArrowRight, Direction::Right),
    ]
    .into_iter()
    .find(|(key, _)| keys.just_pressed(*key))
    .map(|(_, direction)| direction);
    if let Some(direction) = direction {
        let focusable = host.tree.focusable();
        let targets: Vec<(Uid, UiRectangle)> = laid_out
            .iter()
            .filter(|(id, _, _)| focusable.contains(&id.0))
            .map(|(id, node, transform)| {
                let centre = transform.translation;
                (
                    id.0,
                    UiRectangle {
                        x: centre.x - node.size.x / 2.,
                        y: centre.y - node.size.y / 2.,
                        width: node.size.x,
                        height: node.size.y,
                    },
                )
            })
            .collect();
        if let Some(next) = navigate(&targets, host.focused, direction) {
            host.focused = Some(next);
        }
    }
    // With a text box, Space is typed instead of activating.
    let space = text.0.is_none() && keys.just_pressed(KeyCode::Space);
    if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) || space {
        host.activated = host.focused;
    }
}

/// Loads up to `LOADS_PER_FRAME` game images the tree shows and nothing holds yet.
fn load_missing(
    tree: &UiTree,
    images: &mut GooeyImages,
    loader: &mut GooeyLoader,
    assets: &mut Assets<Image>,
) -> bool {
    let GooeyLoader { load, failed } = loader;
    let Some(load) = load else {
        return false;
    };
    let wanted: Vec<ImageRef> = tree
        .nodes
        .iter()
        .filter_map(|n| match &n.widget {
            Widget::Button { image, .. } => *image,
            Widget::Image { image, .. } => Some(*image),
            _ => None,
        })
        .filter(|i| !matches!(i, ImageRef::Swatch(_)))
        .filter(|i| !images.0.contains_key(i) && !failed.contains(i))
        .take(LOADS_PER_FRAME)
        .collect();
    let mut loaded = false;
    for image in wanted {
        match load(image).and_then(|i| gpu_image(i.width, i.height, i.rgba)) {
            Some(gpu) => {
                images.0.insert(image, assets.add(gpu));
                loaded = true;
            }
            None => {
                failed.insert(image);
            }
        }
    }
    loaded
}

/// RGBA8 sRGB image within the viewer's GPU budget (the adapter's 4096 side cap).
fn gpu_image(width: u32, height: u32, rgba: Vec<u8>) -> Option<Image> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let pixels = u64::from(width).checked_mul(u64::from(height))?;
    if width == 0 || height == 0 || width > 4096 || height > 4096 || pixels * 4 != rgba.len() as u64
    {
        return None;
    }
    Some(Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

fn gooey_rebuild(
    mut commands: Commands,
    mut host: ResMut<GooeyHost>,
    mut screen: ResMut<GooeyScreen>,
    images: Option<ResMut<GooeyImages>>,
    mut loader: ResMut<GooeyLoader>,
    mut assets: ResMut<Assets<Image>>,
    target: Res<GooeyTarget>,
) {
    let Some(mut images) = images else {
        return;
    };
    if target.pending {
        return;
    }
    let input = FrameInput {
        focused: host.focused,
        activated: host.activated.take(),
    };
    let tree = (screen.0)(input);
    host.focused = settle_focus(&tree, host.focused);
    let loaded = load_missing(&tree, &mut images, &mut loader, &mut assets);
    if tree == host.tree && host.root.is_some() && !loaded && !target.is_changed() {
        return;
    }
    if !tree.duplicate_ids.is_empty() {
        warn!("UI tree has duplicate widget ids: {:?}", tree.duplicate_ids);
    }
    if let Some(root) = host.root.take() {
        commands.entity(root).despawn();
    }
    host.buttons.clear();
    let mut buttons = HashMap::new();
    let root = spawn_node(&mut commands, &tree, 0, &mut buttons, &images);
    commands.entity(root).insert(Node {
        width: Val::Percent(100.),
        height: Val::Percent(100.),
        padding: UiRect::all(Val::Px(24.)),
        ..default()
    });
    if let Some(camera) = target.camera {
        commands.entity(root).insert(UiTargetCamera(camera));
    }
    host.root = Some(root);
    host.buttons = buttons;
    host.tree = tree;
    host.rebuilds += 1;
    debug!("UI rebuild {}", host.rebuilds);
}

fn font(style: TextStyle) -> TextFont {
    TextFont {
        font: bevy::text::FontSource::SansSerif,
        ..TextFont::from_font_size(match style {
            TextStyle::Title => 28.,
            TextStyle::Body => 18.,
            TextStyle::Small => 14.,
        })
    }
}

/// Draws `image`: a loaded game image, else a swatch / neutral grey placeholder.
fn image_bundle(images: &GooeyImages, image: ImageRef) -> (BackgroundColor, Option<ImageNode>) {
    match images.0.get(&image) {
        Some(handle) => (
            BackgroundColor(Color::NONE),
            Some(ImageNode::new(handle.clone())),
        ),
        None => (BackgroundColor(swatch_color(image)), None),
    }
}

fn swatch_color(image: ImageRef) -> Color {
    match image {
        ImageRef::Swatch([r, g, b, a]) => Color::srgba_u8(r, g, b, a),
        // Game textures are resolved by a later item; draw a labelled-neutral grey.
        ImageRef::Guid(_) | ImageRef::Hash(_) => Color::srgb(0.35, 0.35, 0.35),
    }
}

fn spawn_node(
    commands: &mut Commands,
    tree: &UiTree,
    index: usize,
    buttons: &mut HashMap<Uid, Entity>,
    images: &GooeyImages,
) -> Entity {
    let node = &tree.nodes[index];
    let entity = match &node.widget {
        Widget::Frame {
            layout,
            align,
            style,
            padding,
            gap,
            size,
        } => {
            let justify = match align {
                Align::Start => JustifyContent::FlexStart,
                Align::Center => JustifyContent::Center,
                Align::End => JustifyContent::FlexEnd,
                Align::Full => JustifyContent::SpaceBetween,
            };
            let background = match style {
                PanelStyle::Plain => Color::NONE,
                PanelStyle::Filled => Color::srgba(0.10, 0.13, 0.17, 0.95),
                PanelStyle::Bubble => Color::srgba(0.16, 0.20, 0.26, 0.95),
            };
            commands
                .spawn((
                    Node {
                        flex_direction: match layout {
                            Layout::Column => FlexDirection::Column,
                            Layout::Row | Layout::Wrap => FlexDirection::Row,
                        },
                        flex_wrap: if *layout == Layout::Wrap {
                            FlexWrap::Wrap
                        } else {
                            FlexWrap::NoWrap
                        },
                        justify_content: justify,
                        // Sizes are minimums (PC check of #46: fixed heights hid labels).
                        min_width: size[0].map_or(Val::Auto, |f| Val::Vw(f * 100.)),
                        min_height: size[1].map_or(Val::Auto, |f| Val::Vh(f * 100.)),
                        flex_shrink: if size.iter().any(Option::is_some) {
                            0.
                        } else {
                            1.
                        },
                        padding: UiRect::all(Val::Px(*padding)),
                        row_gap: Val::Px(*gap),
                        column_gap: Val::Px(*gap),
                        border_radius: BorderRadius::all(Val::Px(
                            if *style == PanelStyle::Bubble {
                                14.
                            } else {
                                4.
                            },
                        )),
                        ..default()
                    },
                    BackgroundColor(background),
                    GooeyId(node.uid),
                ))
                .id()
        }
        Widget::Text { text, style } => commands
            .spawn((Text::new(text.clone()), font(*style), GooeyId(node.uid)))
            .id(),
        Widget::Button {
            label,
            image,
            enabled,
        } => {
            let text_colour = if *enabled {
                Color::WHITE
            } else {
                Color::srgb(0.45, 0.45, 0.45)
            };
            let button = commands
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(8.)),
                        row_gap: Val::Px(4.),
                        border: UiRect::all(Val::Px(3.)),
                        border_radius: BorderRadius::all(Val::Px(6.)),
                        min_width: Val::Px(96.),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.18, 0.22, 0.28)),
                    BorderColor::all(Color::NONE),
                    Button,
                    GooeyId(node.uid),
                ))
                .id();
            if let Some(image) = image {
                let (background, picture) = image_bundle(images, *image);
                let swatch = commands
                    .spawn((
                        Node {
                            width: Val::Px(56.),
                            height: Val::Px(56.),
                            border_radius: BorderRadius::all(Val::Px(4.)),
                            ..default()
                        },
                        background,
                    ))
                    .id();
                if let Some(picture) = picture {
                    commands.entity(swatch).insert(picture);
                }
                commands.entity(button).add_child(swatch);
            }
            let text = commands
                .spawn((
                    Text::new(label.clone()),
                    font(TextStyle::Body),
                    TextColor(text_colour),
                ))
                .id();
            commands.entity(button).add_child(text);
            if *enabled {
                buttons.insert(node.uid, button);
            }
            button
        }
        Widget::Image {
            image,
            width,
            height,
        } => {
            let (background, picture) = image_bundle(images, *image);
            let entity = commands
                .spawn((
                    Node {
                        width: Val::Px(*width),
                        height: Val::Px(*height),
                        ..default()
                    },
                    background,
                    GooeyId(node.uid),
                ))
                .id();
            if let Some(picture) = picture {
                commands.entity(entity).insert(picture);
            }
            entity
        }
        Widget::Spacer { width, height } => commands
            .spawn((
                Node {
                    width: Val::Px(*width),
                    height: Val::Px(*height),
                    ..default()
                },
                GooeyId(node.uid),
            ))
            .id(),
    };
    for &child in &node.children {
        let child = spawn_node(commands, tree, child, buttons, images);
        commands.entity(entity).add_child(child);
    }
    entity
}

fn gooey_paint_focus(host: Res<GooeyHost>, mut borders: Query<(&GooeyId, &mut BorderColor)>) {
    for (id, mut border) in &mut borders {
        let colour = if host.focused == Some(id.0) {
            Color::srgb(1.0, 0.78, 0.2)
        } else {
            Color::NONE
        };
        *border = BorderColor::all(colour);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u6_demo_screen_ids_unique_and_tabs_switch_items() {
        let mut state = DemoState::default();
        let tree = demo_screen(FrameInput::default(), &mut state);
        assert!(tree.duplicate_ids.is_empty());
        // 4 tabs + 11 enabled items are focusable.
        assert_eq!(tree.focusable().len(), 15);
        let tab = Uid(TAB_BASE + 2);
        demo_screen(
            FrameInput {
                focused: Some(tab),
                activated: Some(tab),
            },
            &mut state,
        );
        assert_eq!(state.tab, 2);
        let item = Uid(ITEM_BASE + 2 * 0x100 + 3);
        let tree = demo_screen(FrameInput::default(), &mut state);
        assert!(tree.find(item).is_some());
        demo_screen(
            FrameInput {
                focused: Some(item),
                activated: Some(item),
            },
            &mut state,
        );
        assert_eq!(state.picked.as_deref(), Some("Stickers 4"));
        // The disabled last item never activates.
        let locked = Uid(ITEM_BASE + 2 * 0x100 + 11);
        demo_screen(
            FrameInput {
                focused: Some(locked),
                activated: Some(locked),
            },
            &mut state,
        );
        assert_eq!(state.picked.as_deref(), Some("Stickers 4"));
    }

    #[test]
    fn u6_idle_frames_build_equal_trees() {
        // gooey_rebuild respawns only when the tree changes, so a frame with no input must
        // build the same tree as the one before it (no per-frame counters in the tree).
        let mut state = DemoState::default();
        let first = demo_screen(FrameInput::default(), &mut state);
        let focused = FrameInput {
            focused: Some(Uid(TAB_BASE + 1)),
            activated: None,
        };
        assert_eq!(demo_screen(focused, &mut state), first);
        assert_eq!(demo_screen(FrameInput::default(), &mut state), first);
    }
}

/// `--qa-capture NEW_PNG` with `--ui-demo`: screenshot the demo after it settles, then exit.
#[derive(Resource)]
pub(crate) struct DemoCapture(pub(crate) Option<std::path::PathBuf>);

pub(crate) fn demo_capture_path() -> Option<std::path::PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    let value = args
        .iter()
        .position(|a| a == "--qa-capture")
        .and_then(|i| args.get(i + 1))?;
    let base = std::fs::canonicalize("local-analysis").ok()?;
    let path = std::path::PathBuf::from(value);
    let parent = std::fs::canonicalize(path.parent()?).ok()?;
    if !parent.starts_with(base) || path.exists() || path.extension().is_none_or(|e| e != "png") {
        eprintln!("Capture must be a new PNG under local-analysis");
        return None;
    }
    Some(parent.join(path.file_name()?))
}

pub(crate) fn demo_capture(
    mut commands: Commands,
    mut capture: ResMut<DemoCapture>,
    time: Res<Time>,
    mut frames: Local<u32>,
) {
    *frames += 1;
    if *frames > 120 && time.elapsed_secs() > 3. {
        if let Some(path) = capture.0.take() {
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                .observe(bevy::render::view::screenshot::save_to_disk(path))
                .observe(
                    |_: On<bevy::render::view::screenshot::ScreenshotCaptured>,
                     mut exit: MessageWriter<AppExit>| {
                        exit.write(AppExit::Success);
                    },
                );
        }
    }
}
