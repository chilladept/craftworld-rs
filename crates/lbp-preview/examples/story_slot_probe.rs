//! Read-only probe (D330): each story slot's id, group, `location` and planet fields, to
//! see how a badge's place on Earth is stored. Prints numbers only (no game bytes).
//! Usage: story_slot_probe SNAPSHOT GAME-DIRS
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use std::path::Path;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: story_slot_probe SNAPSHOT GAME-DIRS".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let list = lbp_preview::story::read_story_slot_list(&source, &catalog, &control)?;
    for (i, slot) in list.slots.iter().enumerate() {
        let l = slot.location;
        let len3 = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt();
        println!(
            "{i:2} ({},{:3}) group {:?} loc [{:9.4} {:9.4} {:9.4} {:9.4}] |xyz| {len3:8.4} \
             show {:?} deco {} icon {} type {:?}",
            slot.id.slot_type,
            slot.id.number,
            slot.group.map(|g| (g.slot_type, g.number)),
            l[0],
            l[1],
            l[2],
            l[3],
            slot.show_on_planet,
            slot.planet_decorations.is_some(),
            slot.icon.is_some(),
            slot.developer_level_type,
        );
    }
    Ok(())
}
