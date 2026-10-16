//! Read-only probe (D194): prints one gfx material's graph structure (boxes with type,
//! subtype, parameter words, animation names; wires with ports and swizzles) so a route
//! refusal can be traced to the graph shape. Structure only, no texture or asset bytes.
//! Usage: material_graph_dump SNAPSHOT GAME-DIRS GFX-GUID-HEX
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use std::path::Path;

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("material_graph_dump SNAPSHOT GAME-DIRS GFX-GUID-HEX".into());
    }
    let guid = u32::from_str_radix(args[3].trim_start_matches("0x"), 16)
        .map_err(|_| "GFX-GUID-HEX: a hex number")?;
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let key = lbp_preview::resolve(&catalog, Some(guid), None)?;
    let mut bytes = Vec::with_capacity(key.size as usize);
    while bytes.len() < key.size as usize {
        let count = (key.size as usize - bytes.len()).min(4096);
        let page = source
            .read_hex_checked(&key, bytes.len() as u64, count, &control)?
            .bytes;
        if page.len() != count {
            return Err("short page".into());
        }
        bytes.extend(page);
    }
    let g = lbp_resource::material::inspect_graph(&bytes, || false)
        .map_err(|e| e.to_string())?
        .graph;
    println!(
        "material {guid:08x}: {} boxes, {} wires",
        g.boxes.len(),
        g.wires.len()
    );
    for (i, b) in g.boxes.iter().enumerate() {
        let anims: Vec<String> = b
            .animations
            .iter()
            .filter(|a| a.component_value_count != 0 || a.name_hex.bytes().any(|c| c != b'0'))
            .map(|a| {
                format!(
                    "{}x{} mask {:#x} keys {} base {:08x?}",
                    a.name_hex,
                    a.component_value_count,
                    a.components_raw,
                    a.derived_key_count,
                    a.base_value_bits
                )
            })
            .collect();
        println!(
            "  box {i}: type {} subtype {} params {:?} slot {:?} anim [{}]",
            b.type_raw,
            b.subtype_raw,
            &b.parameters_raw[..],
            b.texture_slot_raw,
            anims.join("; ")
        );
    }
    for w in &g.wires {
        println!(
            "  wire {}.{} -> {}.{} swizzle {:?}",
            w.box_from, w.port_from_raw, w.box_to, w.port_to_raw, w.swizzle_raw
        );
    }
    // D331: the diffuse route's verdict with animation base on (the level-scene default).
    match lbp_preview::select_diffuse_with_animation_base(&g, true) {
        Ok(s) => println!(
            "diffuse: slot {} UV{} {} second {:?}",
            s.slot,
            s.uv_channel,
            s.uv_transform.label(),
            s.second_sample.map(|x| x.uv_transform.label())
        ),
        Err(e) => println!("diffuse: {e}"),
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
