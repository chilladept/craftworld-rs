//! Read-only probe (D331): a story group's planet decorations level, Thing by Thing (parent,
//! parts, translation and scale, mesh GUID), to see what the Story view loads onto Earth.
//! Prints numbers only (no game bytes).
//! Usage: story_group_plan_probe SNAPSHOT GAME-DIRS GROUP-NUMBER
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_preview::story::ResourceKey;
use std::path::Path;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: story_group_plan_probe SNAPSHOT GAME-DIRS GROUP-NUMBER".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    // A GUID in hex ("x18fdb") reads that level instead of a group's decorations.
    let deco = if let Some(hex) = args[3].strip_prefix('x') {
        ResourceKey::Guid(u32::from_str_radix(hex, 16).map_err(|e| format!("guid: {e}"))?)
    } else {
        let number: u32 = args[3].parse().map_err(|e| format!("group number: {e}"))?;
        let list = lbp_preview::story::read_story_slot_list(&source, &catalog, &control)?;
        let slot = list
            .slots
            .iter()
            .find(|s| s.id.slot_type == 4 && s.id.number == number)
            .ok_or("no such group")?;
        let deco = slot
            .planet_decorations
            .as_ref()
            .and_then(ResourceKey::of)
            .ok_or("the group has no planet decorations")?;
        println!("group (4, {number}) decorations {}", deco.label());
        deco
    };
    let key = lbp_preview::story::resolve_key(&catalog, deco)?;
    let bytes = lbp_preview::level_scene::read_resource(
        &source,
        &key,
        &control,
        lbp_level::level::MAX_INPUT,
    )?;
    let magic = bytes.get(..4).unwrap_or_default();
    println!(
        "{} bytes, magic {}",
        bytes.len(),
        String::from_utf8_lossy(magic)
    );
    let plan = lbp_level::read(&bytes, lbp_level::Limits::default(), &|| false)
        .map_err(|e| format!("level: {e}"))?;
    println!("{} Things, stop {:?}", plan.things.len(), plan.stop);
    for t in &plan.things {
        let pos = t.data.pos.as_ref().map(|p| {
            let w = p.world;
            let sx = (w[0] * w[0] + w[1] * w[1] + w[2] * w[2]).sqrt();
            format!("t [{:8.1} {:8.1} {:8.1}] s {sx:.3}", w[12], w[13], w[14])
        });
        let mesh = t
            .data
            .render_mesh
            .as_ref()
            .and_then(|m| m.mesh.as_ref())
            .and_then(ResourceKey::of)
            .map(|k| k.label());
        println!(
            "{:3} parent {:?} {} mesh {} parts {:?}",
            t.index,
            t.parent,
            pos.unwrap_or_else(|| "no pos".into()),
            mesh.unwrap_or_else(|| "-".into()),
            t.parts
        );
        if let (Some(o), Some(mesh)) = (
            &t.data.material_override,
            t.data.render_mesh.as_ref().and_then(|m| m.mesh.as_ref()),
        ) {
            let swaps = lbp_preview::level_scene::region_materials(o);
            let mkey = lbp_preview::resolve(&catalog, mesh.guid, None)?;
            // As the level scene loads it: standalone defaults on, editorColor tint.
            let (mut options, _) = lbp_preview::StandaloneDefaults::ON.apply(Default::default());
            if let Some(rm) = &t.data.render_mesh {
                options.instance_color =
                    Some(lbp_preview::level_scene::editor_color_tint(rm.editor_color));
            }
            let g = lbp_preview::load_mesh_groups_with_overrides(
                &source, &catalog, &mkey, &control, options, &swaps,
            )?;
            for (p, b) in g.geometry.metadata.primitives.iter().zip(&g.bindings) {
                println!(
                    "    primitive region {} gmat {:?} {}",
                    p.region as i32,
                    p.material_guid,
                    b.as_ref().map_or_else(|e| e.clone(), |_| "ok".into())
                );
            }
        }
        if let Some(o) = &t.data.material_override {
            println!("    material override mesh {:?}", o.mesh);
            for r in &o.regions {
                println!(
                    "    region {} plan {:?} gmat {:?} uv {:?}",
                    r.region, r.material_plan, r.material, r.uv_scale
                );
            }
        }
    }
    Ok(())
}
