//! D332: the fog tint's palettes, from the game's constants file (`docs/level-globals.md`).
//!
//! `gamedata/data/game_constants.con` (GUID 52107) is a text file: a `CONt` line, then one
//! `name<TAB>value` line per constant, vectors as comma-separated floats. The game reads
//! `FogColours_0..14` and `PodFogColours_0..6` from it as vec4s (01.33 visitor 0xaa0198,
//! FOUND); `LevelSettings::tinted_fog` blends the level's fog colour over them.

use lbp_catalog::{attachment::AttachedSource, Catalog, LoadControl};

pub const CONSTANTS_PATH: &str = "gamedata/data/game_constants.con";
/// The file read is about 2 KB; this is a cap, not its size.
const CONSTANTS_BYTES: usize = 64 * 1024;
/// GUID of `gamedata/lbp2/pod/levels/pod_planets.bin`: only this level takes the Pod's
/// palette and the 0..1 tint range (01.33 0xd0e80, FOUND).
pub const POD_LEVEL_GUID: u32 = 102363;

/// The two palettes in file order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FogPalettes {
    pub level: Vec<[f32; 4]>,
    pub pod: Vec<[f32; 4]>,
}

impl FogPalettes {
    /// The palette the game uses for a level (`pod`: the level is the Pod's).
    pub fn for_level(&self, pod: bool) -> &[[f32; 4]] {
        if pod {
            &self.pod
        } else {
            &self.level
        }
    }
}

/// `name_i` vec4s for i = 0, 1, .. up to the first one missing or not four floats.
fn list(text: &str, name: &str) -> Vec<[f32; 4]> {
    let value = |key: &str| {
        text.lines()
            .find_map(|l| l.split_once('\t').filter(|(k, _)| k.trim() == key))
            .and_then(|(_, v)| {
                let f: Vec<f32> = v
                    .split(',')
                    .map(|x| x.trim().parse().ok())
                    .collect::<Option<_>>()?;
                <[f32; 4]>::try_from(f).ok()
            })
    };
    (0..).map_while(|i| value(&format!("{name}_{i}"))).collect()
}

/// The palettes of a constants file's text; an error when it has none.
pub fn parse_palettes(text: &str) -> Result<FogPalettes, String> {
    if !text.starts_with("CONt") {
        return Err(format!("{CONSTANTS_PATH}: no CONt header"));
    }
    let p = FogPalettes {
        level: list(text, "FogColours"),
        pod: list(text, "PodFogColours"),
    };
    if p.level.is_empty() && p.pod.is_empty() {
        return Err(format!("{CONSTANTS_PATH}: no fog colours"));
    }
    Ok(p)
}

/// Reads the palettes from the attached game data.
pub fn load_palettes(
    source: &AttachedSource,
    catalog: &Catalog,
    control: &LoadControl,
) -> Result<FogPalettes, String> {
    let row = catalog
        .snapshot
        .resources
        .iter()
        .find(|r| r.map_row.path.eq_ignore_ascii_case(CONSTANTS_PATH))
        .ok_or_else(|| format!("{CONSTANTS_PATH}: no MAP row"))?;
    let key = lbp_preview::resolve(catalog, None, Some(&row.map_row.sha1))?;
    let bytes = lbp_preview::level_scene::read_resource(source, &key, control, CONSTANTS_BYTES)?;
    parse_palettes(&String::from_utf8_lossy(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn d332_palettes_read_in_order_and_stop_at_a_gap() {
        let text = "CONt\nFogColours_0\t0,0,0,1\nFogColours_1\t0.5, 0.5,0.5,1\nFogColours_3\t1,1,1,1\nPodFogColours_0\t0.1,0.7,0.7,1\nOther\t2\n";
        let p = parse_palettes(text).unwrap();
        assert_eq!(p.level, vec![[0., 0., 0., 1.], [0.5, 0.5, 0.5, 1.]]);
        assert_eq!(p.for_level(true), &[[0.1, 0.7, 0.7, 1.]]);
        assert!(parse_palettes("FogColours_0\t0,0,0,1").is_err());
        assert!(parse_palettes("CONt\nOther\t1\n").is_err());
    }
}
