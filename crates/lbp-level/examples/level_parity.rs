//! D107: read-only parity of the independent RLevel/Thing reader against the D106
//! Toolkit oracle outputs (pinned d6a2ef80b5a09999f08f23a803b516096055e5c9).
//! Usage: level_parity SNAPSHOT GAME-DIR[;GAME-DIR] ORACLE-ALL-JSON ORACLE-SCENE-JSON NEW-JSON
//! Summary parity (every oracle level parsed_exact at head 0x341..0x3f8, branch 0):
//! thing count, consumed/expanded/trailing bytes, cross-play hash count, reference tag
//! count, world things listed, part histogram, POS/RENDER_MESH/SHAPE/GENERATED_MESH
//! counts, descriptor tallies, bone-count and polygon-size histograms.
//! Scene parity (oracle scene levels): per Thing by reference tag, UID, parent/group/
//! emitter tags, flags, plan GUID, part list, PPos (anim hash, bone-of tag, world m44
//! exact f32), PRenderMesh (mesh, anim, bones, editor colour, visibility), PShape
//! (vertices exact f32, loops, material, thickness, bevel, brightness, colour, flags,
//! behavior) and PGeneratedMesh (gfx material, bevel, plan GUID, uv offset).
//! Output: a NEW JSON file under ignored `local-analysis/`; nothing else is written.
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    LoadControl,
};
use lbp_level::parts::ShapeColor;
use lbp_level::serial::Descriptor;
use lbp_level::thing::ThingRecord;
use lbp_level::{LevelReport, Limits};
use serde_json::{json, Map, Value};
use std::{collections::BTreeMap, fs::OpenOptions, io::Write, path::Path};

const PAGE: usize = 4096;

fn read_all(
    source: &AttachedSource,
    key: &LocationKey,
    control: &LoadControl,
) -> Result<Vec<u8>, String> {
    let size = usize::try_from(key.size).map_err(|_| "size overflow")?;
    if size > lbp_level::level::MAX_INPUT {
        return Err("archived size exceeds level input budget".into());
    }
    let mut bytes = Vec::with_capacity(size);
    while bytes.len() < size {
        let page = source.read_hex_checked(
            key,
            bytes.len() as u64,
            (size - bytes.len()).min(PAGE),
            control,
        )?;
        if page.bytes.is_empty() {
            return Err("truncated source read".into());
        }
        bytes.extend(page.bytes);
    }
    Ok(bytes)
}

fn load(path: &str) -> Result<Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// Oracle descriptor key: g%08x for a GUID, h<sha1> for a hash-only descriptor.
fn key(d: &Option<Descriptor>) -> String {
    match d {
        Some(Descriptor { guid: Some(g), .. }) => format!("g{g:08x}"),
        Some(Descriptor { sha1: Some(h), .. }) => format!("h{}", lbp_level::serial::hex(h)),
        _ => "null".into(),
    }
}

/// Oracle scene descriptor object.
fn desc(d: &Option<Descriptor>) -> Value {
    match d {
        Some(Descriptor { guid, sha1, .. }) if guid.is_some() || sha1.is_some() => json!({
            "guid": guid.map(|g| format!("{g:08x}")),
            "sha1": sha1.map(|h| lbp_level::serial::hex(&h)),
        }),
        _ => Value::Null,
    }
}

fn bucket(n: usize) -> String {
    match n {
        0..=4 => n.to_string(),
        5..=16 => "5-16".into(),
        17..=64 => "17-64".into(),
        65..=256 => "65-256".into(),
        _ => ">256".into(),
    }
}

fn tally(map: &mut BTreeMap<String, u64>, k: String) {
    *map.entry(k).or_default() += 1;
}

fn parsed(t: &ThingRecord, part: &str) -> bool {
    t.read.iter().any(|p| p.part == part && p.state == "parsed")
}

/// Parts as the oracle lists them after Toolkit's load: non-null part objects only
/// (null references and deprecated zero slots leave the Java part slot empty), and
/// Thing.onFinishSave drops a SCRIPT_NAME whose name (bytes before the first NUL) is
/// empty. Toolkit post-load normalization, not a serialized-format difference.
fn oracle_parts(t: &ThingRecord) -> Vec<&'static str> {
    t.read
        .iter()
        .filter(|p| p.state == "parsed" || p.state == "shared")
        .filter(|p| {
            p.part != "SCRIPT_NAME"
                || t.data
                    .script_name
                    .as_ref()
                    .is_none_or(|n| !n.name.is_empty())
        })
        .map(|p| p.part)
        .collect()
}

fn summary(r: &LevelReport) -> Value {
    let mut parts = BTreeMap::<String, u64>::new();
    let (mut rm_d, mut gfx, mut bevel, mut shape_mat, mut plans) = (
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
    );
    let (mut bones, mut sizes) = (BTreeMap::new(), BTreeMap::new());
    let (mut pos, mut rm, mut shape, mut gm, mut rm_shape, mut shape_gm, mut verts) =
        (0, 0, 0, 0, 0, 0, 0);
    for t in &r.things {
        for p in oracle_parts(t) {
            tally(&mut parts, p.to_string());
        }
        let d = &t.data;
        pos += u64::from(d.pos.is_some());
        if let Some(m) = &d.render_mesh {
            rm += 1;
            tally(&mut rm_d, key(&m.mesh));
            tally(&mut bones, m.bone_things.len().to_string());
        }
        if let Some(s) = &d.shape {
            shape += 1;
            verts += s.vertex_count as u64;
            tally(&mut shape_mat, key(&s.material));
            tally(&mut sizes, bucket(s.vertex_count));
        }
        if let Some(g) = &d.generated_mesh {
            gm += 1;
            tally(&mut gfx, key(&g.gfx_material));
            tally(&mut bevel, key(&g.bevel));
        }
        rm_shape += u64::from(d.render_mesh.is_some() && d.shape.is_some());
        shape_gm += u64::from(d.shape.is_some() && d.generated_mesh.is_some());
        if parsed(t, "REF") {
            tally(&mut plans, key(&d.plan_ref));
        }
    }
    json!({
        "thing_count": r.things.len(),
        "consumed_bytes": r.consumed_bytes,
        "expanded_bytes": r.expanded_bytes,
        "trailing_bytes": r.trailing_bytes,
        "cross_play_hash_count": r.cross_play_hashes.unwrap_or(0),
        "reference_tag_count": r.tags,
        "world_things_listed": r.world_things_walked,
        "part_histogram": parts,
        "counts": {"pos": pos, "render_mesh": rm, "shape": shape, "generated_mesh": gm,
            "render_mesh_and_shape": rm_shape, "shape_and_generated_mesh": shape_gm,
            "shape_vertices_total": verts},
        "render_mesh_descriptors": rm_d, "gfx_material_descriptors": gfx,
        "bevel_descriptors": bevel, "shape_material_descriptors": shape_mat,
        "ref_plan_descriptors": plans, "bone_things_length_histogram": bones,
        "shape_vertex_count_histogram": sizes,
    })
}

fn oracle_summary(o: &Value) -> Value {
    let counts = &o["counts"];
    json!({
        "thing_count": o["thing_count"], "consumed_bytes": o["consumed_bytes"],
        "expanded_bytes": o["expanded_bytes"], "trailing_bytes": o["trailing_bytes"],
        "cross_play_hash_count": o["cross_play_hash_count"],
        "reference_tag_count": o["reference_tag_count"],
        "world_things_listed": o["world"]["world_things_listed"],
        "part_histogram": o["part_histogram"],
        "counts": {"pos": counts["pos"], "render_mesh": counts["render_mesh"],
            "shape": counts["shape"], "generated_mesh": counts["generated_mesh"],
            "render_mesh_and_shape": counts["render_mesh_and_shape"],
            "shape_and_generated_mesh": counts["shape_and_generated_mesh"],
            "shape_vertices_total": counts["shape_vertices_total"]},
        "render_mesh_descriptors": o["render_mesh_descriptors"],
        "gfx_material_descriptors": o["gfx_material_descriptors"],
        "bevel_descriptors": o["bevel_descriptors"],
        "shape_material_descriptors": o["shape_material_descriptors"],
        "ref_plan_descriptors": o["ref_plan_descriptors"],
        "bone_things_length_histogram": o["bone_things_length_histogram"],
        "shape_vertex_count_histogram": o["shape_vertex_count_histogram"],
    })
}

fn tag_of(r: &LevelReport, i: Option<usize>) -> Value {
    i.map_or(Value::Null, |i| json!(r.things[i].tag))
}

fn oracle_tag(v: &Value) -> Value {
    v.get("tag").cloned().unwrap_or(Value::Null)
}

/// Compares f32 values exactly: the oracle prints Java float text, parsed back to f32.
fn floats_equal(ours: &[f32], oracle: &Value) -> bool {
    let Some(a) = oracle.as_array() else {
        return false;
    };
    a.len() == ours.len()
        && a.iter().zip(ours).all(|(o, v)| {
            o.as_f64()
                .is_some_and(|o| (o as f32).to_bits() == v.to_bits())
        })
}

fn f32_equal(ours: f32, oracle: &Value) -> bool {
    floats_equal(&[ours], &json!([oracle]))
}

fn scene_thing(r: &LevelReport, t: &ThingRecord, o: &Value, out: &mut BTreeMap<&'static str, u64>) {
    let mut check = |field: &'static str, ok: bool| {
        if !ok {
            *out.entry(field).or_default() += 1;
        }
    };
    check("uid", o["uid"] == json!(t.uid));
    check("parent", oracle_tag(&o["parent"]) == tag_of(r, t.parent));
    check(
        "group_head",
        oracle_tag(&o["group_head"]) == tag_of(r, t.group_head),
    );
    check(
        "old_emitter",
        oracle_tag(&o["old_emitter"]) == tag_of(r, t.old_emitter),
    );
    check("flags", o["flags"] == json!(t.flags));
    check(
        "plan_guid",
        o["plan_guid"] == json!(t.plan_guid.map(|g| format!("{g:08x}"))),
    );
    check("parts", o["parts"] == json!(oracle_parts(t)));
    let d = &t.data;
    let op = &o["pos"];
    check("pos_present", op.is_null() == d.pos.is_none());
    if let (Some(p), false) = (&d.pos, op.is_null()) {
        check("pos_anim_hash", op["anim_hash"] == json!(p.anim_hash));
        check(
            "pos_bone_of",
            oracle_tag(&op["bone_of"]) == tag_of(r, p.bone_of),
        );
        check(
            "pos_world_m44",
            floats_equal(&p.world, &op["world_m44_column_major"]),
        );
    }
    let om = &o["render_mesh"];
    check(
        "render_mesh_present",
        om.is_null() == d.render_mesh.is_none(),
    );
    if let (Some(m), false) = (&d.render_mesh, om.is_null()) {
        check("render_mesh_mesh", om["mesh"] == desc(&m.mesh));
        check("render_mesh_anim", om["anim"] == desc(&m.anim));
        let bones: Vec<Value> = m.bone_things.iter().map(|b| tag_of(r, *b)).collect();
        let obones: Vec<Value> = om["bones"]
            .as_array()
            .map_or(vec![], |a| a.iter().map(oracle_tag).collect());
        check("render_mesh_bones", bones == obones);
        check(
            "render_mesh_editor_color",
            om["editor_color"] == json!(m.editor_color as i32),
        );
        check(
            "render_mesh_visibility",
            om["visibility_flags"] == json!(m.visibility_flags),
        );
    }
    let os = &o["shape"];
    check("shape_present", os.is_null() == d.shape.is_none());
    if let (Some(s), false) = (&d.shape, os.is_null()) {
        check(
            "shape_vertex_count",
            os["vertex_count"] == json!(s.vertex_count),
        );
        check("shape_requires_z", os["requires_z"] == json!(s.requires_z));
        let flat: Vec<f32> = s.vertices.iter().flatten().copied().collect();
        let oflat: Vec<Value> = os["vertices"].as_array().map_or(vec![], |a| {
            a.iter()
                .flat_map(|v| v.as_array().cloned().unwrap_or_default())
                .collect()
        });
        check("shape_vertices", floats_equal(&flat, &Value::Array(oflat)));
        check("shape_loops", os["loops"] == json!(s.loops));
        check("shape_material", os["material"] == desc(&s.material));
        check(
            "shape_old_material",
            os["old_material"] == desc(&s.old_material),
        );
        check("shape_thickness", f32_equal(s.thickness, &os["thickness"]));
        check(
            "shape_bevel_size",
            f32_equal(s.bevel_size, &os["bevel_size"]),
        );
        check(
            "shape_brightness",
            f32_equal(s.brightness, &os["brightness"]),
        );
        let color = match s.color {
            ShapeColor::Packed(c) => json!(c as i32),
            ShapeColor::Vector(_) => Value::Null,
        };
        check("shape_color", os["color"] == color);
        check("shape_flags", os["flags"] == json!(s.flags));
        check("shape_behavior", os["behavior"] == json!(s.behavior as i32));
    }
    let og = &o["generated_mesh"];
    check(
        "generated_mesh_present",
        og.is_null() == d.generated_mesh.is_none(),
    );
    if let (Some(g), false) = (&d.generated_mesh, og.is_null()) {
        check(
            "generated_mesh_gfx_material",
            og["gfx_material"] == desc(&g.gfx_material),
        );
        check("generated_mesh_bevel", og["bevel"] == desc(&g.bevel));
        check(
            "generated_mesh_plan_guid",
            og["plan_guid"] == json!(g.plan_guid.map(|p| format!("{p:08x}"))),
        );
        check(
            "generated_mesh_uv_offset",
            floats_equal(&g.uv_offset, &og["uv_offset"]),
        );
    }
    // D121: only oracle dumps that project PLevelSettings carry the key.
    if let Some(ol) = o.get("level_settings") {
        let absent = ol.as_str() == Some("absent");
        check(
            "level_settings_present",
            absent == d.level_settings.is_none(),
        );
        if let (Some(s), false) = (&d.level_settings, absent) {
            let fields: [(&'static str, &[f32]); 11] = [
                ("level_settings_sun_position", &s.sun_position),
                ("level_settings_sun_position_scale", &[s.sun_position_scale]),
                ("level_settings_sun_color", &s.sun_color),
                ("level_settings_ambient_color", &s.ambient_color),
                ("level_settings_sun_multiplier", &[s.sun_multiplier]),
                ("level_settings_exposure", &[s.exposure]),
                ("level_settings_fog_color", &s.fog_color),
                ("level_settings_fog_near", &[s.fog_near]),
                ("level_settings_fog_far", &[s.fog_far]),
                ("level_settings_rim_color", &s.rim_color),
                ("level_settings_rim_color2", &s.rim_color2),
            ];
            for (field, ours) in fields {
                let value = &ol[&field["level_settings_".len()..]];
                let oracle = match value {
                    Value::Array(_) => value.clone(),
                    v => json!([v]),
                };
                check(field, floats_equal(ours, &oracle));
            }
            check("level_settings_presets", ol["presets"] == json!(s.presets));
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 6 {
        return Err(
            "usage: level_parity SNAPSHOT GAME-DIR ORACLE-ALL ORACLE-SCENE NEW-JSON".into(),
        );
    }
    let output = Path::new(&args[5]);
    let allowed = Path::new("local-analysis")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if output.exists()
        || !output
            .parent()
            .ok_or("parent required")?
            .canonicalize()
            .map_err(|e| e.to_string())?
            .starts_with(allowed)
    {
        return Err("new output under ignored local-analysis required".into());
    }
    let oracle_all = load(&args[3])?;
    let oracle_scene = load(&args[4])?;
    for o in [&oracle_all, &oracle_scene] {
        if o["toolkit_commit"] != "d6a2ef80b5a09999f08f23a803b516096055e5c9" {
            return Err("oracle pin mismatch".into());
        }
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let limits = Limits {
        max_depth: 4096,
        ..Limits::default()
    };
    let read = |sha: &str| -> Result<LevelReport, String> {
        let key = lbp_preview::resolve(&catalog, None, Some(sha))?;
        let bytes = read_all(&source, &key, &control)?;
        lbp_level::read(&bytes, limits, &|| control.check().is_err()).map_err(|e| e.to_string())
    };
    let empty = vec![];
    let mut rows = Vec::new();
    let mut field_mismatch = BTreeMap::<String, u64>::new();
    let (mut compared, mut equal, mut out_of_scope, mut ours_incomplete) = (0, 0, 0, 0);
    for o in oracle_all["levels"].as_array().unwrap_or(&empty) {
        let sha = o["sha1"].as_str().unwrap_or_default();
        let head = o["header"]["head"].as_u64().unwrap_or(0);
        let in_scope = o["status"] == "parsed_exact"
            && (0x341..=0x3f8).contains(&head)
            && o["header"]["branch_id"] == 0;
        if !in_scope {
            out_of_scope += 1;
            continue;
        }
        let r = read(sha)?;
        if r.stop.is_some() {
            ours_incomplete += 1;
            rows.push(json!({"sha1": sha, "status": "ours_stopped", "stop": r.stop}));
            continue;
        }
        compared += 1;
        let ours = summary(&r);
        let theirs = oracle_summary(o);
        let mut diffs = Vec::new();
        if let (Some(a), Some(b)) = (ours.as_object(), theirs.as_object()) {
            for (k, v) in a {
                let ov = b.get(k).cloned().unwrap_or(Value::Null);
                let same = match (v, &ov) {
                    (Value::Object(x), Value::Object(y)) => normalize(x) == normalize(y),
                    _ => *v == ov,
                };
                if !same {
                    diffs.push(k.clone());
                    *field_mismatch.entry(k.clone()).or_default() += 1;
                }
            }
        }
        if diffs.is_empty() {
            equal += 1;
        } else {
            rows.push(json!({"sha1": sha, "status": "mismatch", "fields": diffs, "ours": ours, "oracle": theirs}));
        }
    }
    let mut scene_rows = Vec::new();
    let (mut scene_things, mut scene_things_equal) = (0u64, 0u64);
    let mut scene_mismatch = BTreeMap::<&'static str, u64>::new();
    for lv in oracle_scene["levels"].as_array().unwrap_or(&empty) {
        let sha = lv["sha1"].as_str().unwrap_or_default();
        let r = read(sha)?;
        let by_tag: BTreeMap<u64, &ThingRecord> =
            r.things.iter().map(|t| (u64::from(t.tag), t)).collect();
        let list = lv["scene_things"].as_array().unwrap_or(&empty);
        let mut missing = 0;
        let mut level_mismatch = BTreeMap::<&'static str, u64>::new();
        let mut level_equal = 0;
        for o in list {
            scene_things += 1;
            let Some(t) = o["tag"].as_u64().and_then(|t| by_tag.get(&t)) else {
                missing += 1;
                continue;
            };
            let mut m = BTreeMap::new();
            scene_thing(&r, t, o, &mut m);
            if m.is_empty() {
                level_equal += 1;
            }
            for (k, n) in m {
                *level_mismatch.entry(k).or_default() += n;
                *scene_mismatch.entry(k).or_default() += n;
            }
        }
        scene_things_equal += level_equal;
        scene_rows.push(
            json!({"sha1": sha, "oracle_things": list.len(), "our_things": r.things.len(),
            "missing_tags": missing, "things_all_fields_equal": level_equal,
            "field_mismatches": level_mismatch, "our_stop": r.stop}),
        );
    }
    source.check_stable()?;
    let value = json!({
        "schema_version": 1, "milestone": "D107",
        "toolkit_commit": "d6a2ef80b5a09999f08f23a803b516096055e5c9",
        "scope": "D106 oracle levels parsed_exact at head 0x341..0x3f8 branch 0; scene levels",
        "unit": "distinct level content hash; scene unit is Thing by reference tag",
        "summary": {"oracle_levels": oracle_all["levels"].as_array().map_or(0, Vec::len),
            "out_of_scope": out_of_scope, "ours_stopped": ours_incomplete,
            "compared": compared, "all_fields_equal": equal, "field_mismatch_levels": field_mismatch},
        "scene": {"levels": scene_rows.len(), "things": scene_things,
            "things_all_fields_equal": scene_things_equal, "field_mismatch_things": scene_mismatch},
        "mismatch_rows": rows,
        "scene_rows": scene_rows,
    });
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, &value).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    println!("summary: {}", value["summary"]);
    println!("scene: {}", value["scene"]);
    Ok(())
}

/// Descriptor/histogram maps: drop zero-count entries so absent and 0 compare equal.
fn normalize(m: &Map<String, Value>) -> BTreeMap<String, Value> {
    m.iter()
        .filter(|(_, v)| **v != json!(0))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn main() {
    let worker = std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(run)
        .expect("spawn parity worker");
    if let Err(e) = worker.join().expect("parity worker panicked") {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
