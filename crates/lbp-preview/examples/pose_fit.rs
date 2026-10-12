#![allow(clippy::all)]
//! QA probe: fits dumped game vertex positions (RPCS3 CPU-skinned buffer, 16-byte
//! big-endian records, K samples back to back) to every frame of the named clips on one
//! mesh. Prints per sample the best clip/frame and the per-joint residual. No game bytes
//! are printed, only fit numbers.
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_preview::skinned;

struct Grid {
    cell: f32,
    map: std::collections::HashMap<(i32, i32, i32), Vec<[f32; 3]>>,
}
impl Grid {
    fn new(pts: &[[f32; 3]], cell: f32) -> Self {
        let mut map: std::collections::HashMap<(i32, i32, i32), Vec<[f32; 3]>> = Default::default();
        for p in pts {
            map.entry(Self::key(cell, *p)).or_default().push(*p);
        }
        Self { cell, map }
    }
    fn key(cell: f32, p: [f32; 3]) -> (i32, i32, i32) {
        (
            (p[0] / cell).floor() as i32,
            (p[1] / cell).floor() as i32,
            (p[2] / cell).floor() as i32,
        )
    }
    /// nearest stored point within ~2 cells (None past that)
    fn nearest(&self, q: [f32; 3]) -> Option<([f32; 3], f32)> {
        let k = Self::key(self.cell, q);
        let mut best: Option<([f32; 3], f32)> = None;
        for dx in -2..=2 {
            for dy in -2..=2 {
                for dz in -2..=2 {
                    if let Some(v) = self.map.get(&(k.0 + dx, k.1 + dy, k.2 + dz)) {
                        for p in v {
                            let d = ((p[0] - q[0]).powi(2)
                                + (p[1] - q[1]).powi(2)
                                + (p[2] - q[2]).powi(2))
                            .sqrt();
                            if best.map_or(true, |b| d < b.1) {
                                best = Some((*p, d));
                            }
                        }
                    }
                }
            }
        }
        best
    }
}

fn centroid(p: &[[f32; 3]]) -> [f32; 3] {
    let n = p.len() as f32;
    p.iter()
        .fold([0f32; 3], |a, v| [a[0] + v[0], a[1] + v[1], a[2] + v[2]])
        .map(|v| v / n)
}

/// ICP of our posed vertices onto the game's vertex cloud (translation + yaw, order free).
/// Returns (rms of the best 80% nearest distances, per-vertex nearest distance).
fn fit(grid: &Grid, g: &[[f32; 3]], p: &[[f32; 3]]) -> (f32, Vec<f32>) {
    let inits: &[f32] = if std::env::var("MULTI").is_ok() {
        &[0., 1.5707964, 3.1415927, 4.712389]
    } else {
        &[0.]
    };
    let mut best: Option<(f32, Vec<f32>)> = None;
    for &y in inits {
        let r = fit1(grid, g, p, y);
        if best.as_ref().map_or(true, |b| r.0 < b.0) {
            best = Some(r);
        }
    }
    best.unwrap()
}

thread_local! {
    /// Translation of the last `fit1` call (QA print: the visual root offset per sample).
    static LAST_T: std::cell::Cell<[f32; 3]> = const { std::cell::Cell::new([0.; 3]) };
    static LAST_YAW: std::cell::Cell<f32> = const { std::cell::Cell::new(0.) };
}

fn fit1(grid: &Grid, g: &[[f32; 3]], p: &[[f32; 3]], yaw0: f32) -> (f32, Vec<f32>) {
    let cg = centroid(g);
    let cp = centroid(p);
    let (s0, c0) = yaw0.sin_cos();
    let mut t = [
        cg[0] - (c0 * cp[0] - s0 * cp[2]),
        cg[1] - cp[1],
        cg[2] - (s0 * cp[0] + c0 * cp[2]),
    ];
    let mut yaw = yaw0;
    let place = |q: &[f32; 3], t: &[f32; 3], yaw: f32| {
        let (sn, cs) = yaw.sin_cos();
        [
            cs * q[0] - sn * q[2] + t[0],
            q[1] + t[1],
            sn * q[0] + cs * q[2] + t[2],
        ]
    };
    let mut dists = vec![f32::INFINITY; p.len()];
    for _ in 0..6 {
        let mut pairs = Vec::new();
        for (i, q) in p.iter().enumerate() {
            let w = place(q, &t, yaw);
            if let Some((n, d)) = grid.nearest(w) {
                dists[i] = d;
                pairs.push((d, *q, n));
            } else {
                dists[i] = f32::INFINITY;
            }
        }
        if pairs.len() < 50 {
            break;
        }
        pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        pairs.truncate(pairs.len() * 8 / 10);
        let mp = centroid(&pairs.iter().map(|x| x.1).collect::<Vec<_>>());
        let mn = centroid(&pairs.iter().map(|x| x.2).collect::<Vec<_>>());
        let (mut s, mut c) = (0f32, 0f32);
        for (_, q, n) in &pairs {
            let (px, pz) = (q[0] - mp[0], q[2] - mp[2]);
            let (gx, gz) = (n[0] - mn[0], n[2] - mn[2]);
            c += gx * px + gz * pz;
            s += gz * px - gx * pz;
        }
        yaw = s.atan2(c);
        let (sn, cs) = yaw.sin_cos();
        t = [
            mn[0] - (cs * mp[0] - sn * mp[2]),
            mn[1] - mp[1],
            mn[2] - (sn * mp[0] + cs * mp[2]),
        ];
    }
    LAST_T.with(|c| c.set(t));
    LAST_YAW.with(|c| c.set(yaw));
    for (i, q) in p.iter().enumerate() {
        dists[i] = grid
            .nearest(place(q, &t, yaw))
            .map_or(f32::INFINITY, |x| x.1);
    }
    let mut d: Vec<f32> = dists.iter().copied().filter(|v| v.is_finite()).collect();
    d.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let k = (d.len() * 8 / 10).max(1);
    let rms = (d.iter().take(k).map(|v| v * v).sum::<f32>() / k as f32).sqrt();
    (rms, dists)
}

/// Index-ordered Procrustes (translation + yaw) over the first `g.len()` vertices.
fn fit_ordered(g: &[[f32; 3]], p: &[[f32; 3]]) -> f32 {
    let p = &p[..g.len().min(p.len())];
    let g = &g[..p.len()];
    let cg = centroid(g);
    let cp = centroid(p);
    let (mut s, mut c) = (0f32, 0f32);
    for (a, b) in g.iter().zip(p) {
        let (gx, gz) = (a[0] - cg[0], a[2] - cg[2]);
        let (px, pz) = (b[0] - cp[0], b[2] - cp[2]);
        c += gx * px + gz * pz;
        s += gz * px - gx * pz;
    }
    let (sn, cs) = s.atan2(c).sin_cos();
    let mut tot = 0f32;
    for (a, b) in g.iter().zip(p) {
        let (px, pz) = (b[0] - cp[0], b[2] - cp[2]);
        let d = [
            a[0] - cg[0] - (cs * px - sn * pz),
            a[1] - cg[1] - (b[1] - cp[1]),
            a[2] - cg[2] - (sn * px + cs * pz),
        ];
        tot += d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
    }
    (tot / p.len() as f32).sqrt()
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 6 {
        return Err(
            "pose_fit SNAPSHOT GAME-DIR MESH-GUID DUMP.bin CLIP-HEX[,CLIP-HEX...] [flip]".into(),
        );
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(std::path::Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(std::path::Path::new(&args[2]), &catalog, &control)?;
    let guid = |s: &str| u32::from_str_radix(s, 16).map_err(|e| e.to_string());
    let mesh = lbp_preview::resolve(&catalog, Some(guid(&args[3])?), None)?;
    let bytes = std::fs::read(&args[4]).map_err(|e| e.to_string())?;
    let mut clips = Vec::new();
    for g in args[5].split(',') {
        let anim = skinned::resolve_animation(&catalog, guid(g)?)?;
        let c = skinned::load_clip(&source, &mesh, &anim, &control)?;
        clips.push((g.to_string(), c));
    }
    if std::env::var("BIPED").is_ok() {
        // the Sackboy play figure's scale rule (D246) for every pose built below
        for (_, c) in &mut clips {
            c.rig.scale_rule = skinned::ScaleRule::Biped;
        }
    }
    if std::env::var("MORPHS").is_ok() {
        // per clip: morph weight range over the clip, and how many vertices each mesh
        // morph moves by more than 0.05 (with the largest offset)
        for (name, c) in &clips {
            let n = c.frame_pose(0).map_or(0, |p| p.morphs.len());
            for m in 0..n {
                let (mut lo, mut hi) = (f32::MAX, f32::MIN);
                for f in 0..i64::from(c.report.frames.max(1)) {
                    if let Some(w) = c.frame_pose(f).and_then(|p| p.morphs.get(m).copied()) {
                        lo = lo.min(w);
                        hi = hi.max(w);
                    }
                }
                let len = |o: &[f32; 3]| (o[0] * o[0] + o[1] * o[1] + o[2] * o[2]).sqrt();
                let (moved, far) = c.skin.morphs.get(m).map_or((0, 0.), |t| {
                    (
                        t.offsets.iter().filter(|o| len(o) > 0.05).count(),
                        t.offsets.iter().map(len).fold(0f32, f32::max),
                    )
                });
                println!("{name} morph {m}: weight {lo:.3}..{hi:.3}, moves {moved} vertices, largest {far:.1}");
                if let (true, Some(t)) = (hi > 0., c.skin.morphs.get(m)) {
                    // where the moved vertices are (bind x, depth, up), their mean offset and bones
                    let (mut lo3, mut hi3, mut mean) = ([f32::MAX; 3], [f32::MIN; 3], [0f32; 3]);
                    let mut bones = std::collections::BTreeMap::<u8, u32>::new();
                    for ((o, p), j) in t.offsets.iter().zip(&c.bind_positions).zip(&c.skin.joints) {
                        if len(o) > 0.05 {
                            for k in 0..3 {
                                lo3[k] = lo3[k].min(p[k]);
                                hi3[k] = hi3[k].max(p[k]);
                                mean[k] += o[k] / moved as f32;
                            }
                            *bones.entry(j[0]).or_default() += 1;
                        }
                    }
                    println!(
                        "   bind lo {lo3:.1?} hi {hi3:.1?} mean offset {mean:.2?} bones {bones:?}"
                    );
                }
            }
        }
        return Ok(());
    }
    if std::env::var("DEBUG_ROOT").is_ok() {
        for (name, c) in &clips {
            for f in [0i64, 3, 6] {
                if let Some(pose) = c.frame_pose(f) {
                    for i in 0..c.rig.bone_count() {
                        if c.rig.parents()[i] == -1 {
                            if let Some(t) = c.rig.pose_trs(&pose, i) {
                                println!("{name} f{f} root{i} t {:?} r {:?} s {:?}", t.t, t.r, t.s);
                            }
                        }
                    }
                }
            }
        }
        let bp = skinned::BlendPose::bind(&clips[0].1.rig);
        for (i, b) in bp.bones.iter().enumerate() {
            if clips[0].1.rig.parents()[i] == -1 {
                println!("bind root{i} t {:?} r {:?}", b.t, b.r);
            }
        }
    }
    if std::env::var("JOINTS").is_ok() {
        let c = &clips[0].1;
        let mut acc = std::collections::BTreeMap::<u8, ([f32; 3], u32)>::new();
        for (p, j) in c.bind_positions.iter().zip(&c.skin.joints) {
            let e = acc.entry(j[0]).or_default();
            for k in 0..3 {
                e.0[k] += p[k];
            }
            e.1 += 1;
        }
        for (j, (t, n)) in acc {
            println!(
                "joint {j}: n {n} x {:.1} up {:.1} depth {:.1}",
                t[0] / n as f32,
                t[2] / n as f32,
                t[1] / n as f32
            );
        }
    }
    if std::env::var("TREE").is_ok() {
        // bone, parent, bind world position, bind local translation, stand-clip local
        // translation range per axis
        let c = &clips[0].1;
        let parents = c.rig.parents();
        let bind = skinned::BlendPose::bind(&c.rig);
        let w = c.rig.worlds_from_trs(&bind.bones);
        for i in 0..c.rig.bone_count() {
            let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
            for f in 0..i64::from(c.report.frames.max(1)) {
                if let Some(t) = c.frame_pose(f).and_then(|p| c.rig.pose_trs(&p, i)) {
                    for k in 0..3 {
                        lo[k] = lo[k].min(t.t[k]);
                        hi[k] = hi[k].max(t.t[k]);
                    }
                }
            }
            println!(
                "bone {i} parent {} world [{:.1} {:.1} {:.1}] bind t [{:.1} {:.1} {:.1}] clip t lo [{:.1} {:.1} {:.1}] hi [{:.1} {:.1} {:.1}]",
                parents[i], w[i][12], w[i][13], w[i][14],
                bind.bones[i].t[0], bind.bones[i].t[1], bind.bones[i].t[2],
                lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]
            );
        }
        return Ok(());
    }
    if std::env::var("FEET").is_ok() {
        // per clip, a few frames: model-space joint of each leg bone (x, depth, up)
        for (name, c) in &clips {
            let last = i64::from(c.report.frames.max(1)) - 1;
            for f in [0, last / 2, last] {
                let Some(p) = c.frame_pose(f) else { continue };
                let mut pose = skinned::BlendPose::bind(&c.rig);
                pose.lerp(&c.rig, &p, 1.);
                let w = c.rig.worlds_from_trs(&pose.bones);
                for side in ["L", "R"] {
                    for part in ["Thigh", "Calf", "Foot", "Toe0"] {
                        if let Some(i) = c.rig.bone_named(&format!("Bip01 {side} {part}")) {
                            println!(
                                "{name} f{f} {side} {part} (bone {i}): x {:.2} depth {:.2} up {:.2}",
                                w[i][12], w[i][13], w[i][14]
                            );
                        }
                    }
                }
            }
        }
        return Ok(());
    }
    if std::env::var("SCALES").is_ok() {
        // per clip: every bone whose local scale leaves bind by over 0.5% in any frame
        // (bone, parent, child count, weighted vertex count, min..max scale ratio per axis)
        for (name, c) in &clips {
            let parents = c.rig.parents();
            let bind = skinned::BlendPose::bind(&c.rig);
            for i in 0..c.rig.bone_count() {
                let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
                for f in 0..i64::from(c.report.frames.max(1)) {
                    let Some(t) = c.frame_pose(f).and_then(|p| c.rig.pose_trs(&p, i)) else {
                        continue;
                    };
                    for k in 0..3 {
                        let r = t.s[k] / bind.bones[i].s[k];
                        lo[k] = lo[k].min(r);
                        hi[k] = hi[k].max(r);
                    }
                }
                if (0..3).any(|k| lo[k] < 0.995 || hi[k] > 1.005) {
                    println!("  bind local scale {:?}", bind.bones[i].s);
                    let kids = parents.iter().filter(|&&p| p == i as i32).count();
                    let verts = c
                        .skin
                        .joints
                        .iter()
                        .zip(&c.skin.weights)
                        .filter(|(j, w)| (0..4).any(|k| j[k] as usize == i && w[k] > 0.))
                        .count();
                    println!(
                        "{name} bone {i} parent {} kids {kids} verts {verts} scale x {:.3}..{:.3} y {:.3}..{:.3} z {:.3}..{:.3}",
                        parents[i], lo[0], hi[0], lo[1], hi[1], lo[2], hi[2]
                    );
                }
            }
        }
        return Ok(());
    }
    if std::env::var("JUMP_TABLE").is_ok() {
        // clips "small,held": mesh height of lerp(small, held, h) at frames 1..=10 for
        // h = 0.1..=0.6, then of each clip alone, under both scale rules
        for rule in [skinned::ScaleRule::Inherited, skinned::ScaleRule::Biped] {
            println!("{rule:?}");
            let height = |bp: &skinned::BlendPose| {
                let c = &clips[0].1;
                let mut rig = c.rig.clone();
                rig.scale_rule = rule;
                let base = c.morphed_bind_weights(&bp.morphs);
                let m = rig.skin_matrices_from_trs(&bp.bones);
                let p = skinned::skin_positions(&base, &c.skin.joints, &c.skin.weights, &m);
                let lo = p.iter().map(|q| q[2]).fold(f32::MAX, f32::min);
                let hi = p.iter().map(|q| q[2]).fold(f32::MIN, f32::max);
                hi - lo
            };
            let row = |h: Option<f32>, only: usize| -> Vec<String> {
                (1..=10)
                    .map(|f| {
                        let mut bp = skinned::BlendPose::bind(&clips[0].1.rig);
                        let (a, b) = (&clips[0].1, &clips[only].1);
                        match h {
                            Some(h) => {
                                bp.lerp(&a.rig, &a.frame_pose(f).unwrap(), 1.);
                                bp.lerp(&b.rig, &b.frame_pose(f).unwrap(), h);
                            }
                            None => bp.lerp(&b.rig, &b.frame_pose(f).unwrap(), 1.),
                        }
                        bp.hold_root_rotation(&a.rig);
                        format!("{:.1}", height(&bp))
                    })
                    .collect()
            };
            for k in 1..=6 {
                let h = k as f32 / 10.;
                println!("  h {h:.1}: {}", row(Some(h), 1).join(", "));
            }
            for (i, (name, _)) in clips.iter().enumerate() {
                println!("  {name} alone: {}", row(None, i).join(", "));
            }
        }
        return Ok(());
    }
    if let Ok(path) = std::env::var("RULE_DUMP") {
        // every frame of clip 0 over bind at each weight, per scale rule: viewer dump format
        let c = &clips[0].1;
        for (tag, rule) in [
            ("inh", skinned::ScaleRule::Inherited),
            ("bip", skinned::ScaleRule::Biped),
        ] {
            let mut rig = c.rig.clone();
            rig.scale_rule = rule;
            for w in [0.0f32, 0.25, 0.3, 0.35, 0.5, 0.75, 1.0] {
                let mut buf = Vec::new();
                for f in 0..i64::from(c.report.frames.max(1)) {
                    let Some(pose) = c.frame_pose(f) else {
                        continue;
                    };
                    let mut bp = skinned::BlendPose::bind(&c.rig);
                    bp.lerp(&c.rig, &pose, w);
                    bp.hold_root_rotation(&c.rig);
                    let base = c.morphed_bind_weights(&bp.morphs);
                    let m = rig.skin_matrices_from_trs(&bp.bones);
                    for p in skinned::skin_positions(&base, &c.skin.joints, &c.skin.weights, &m) {
                        for v in p {
                            buf.extend_from_slice(&v.to_le_bytes());
                        }
                    }
                }
                std::fs::write(format!("{path}_{tag}_{w}.bin"), buf).map_err(|e| e.to_string())?;
            }
        }
        return Ok(());
    }
    if let Ok(path) = std::env::var("DUMP_SKIN") {
        // per vertex: 4 joint bytes then 4 f32 LE weights
        let mut buf = Vec::new();
        for (j, w) in clips[0].1.skin.joints.iter().zip(&clips[0].1.skin.weights) {
            buf.extend_from_slice(j);
            for v in w {
                buf.extend_from_slice(&v.to_le_bytes());
            }
        }
        std::fs::write(path, buf).map_err(|e| e.to_string())?;
    }
    if let Ok(path) = std::env::var("DUMP_JOINTS") {
        let j: Vec<u8> = clips[0].1.skin.joints.iter().map(|j| j[0]).collect();
        std::fs::write(path, j).map_err(|e| e.to_string())?;
    }
    let n = clips[0].1.bind_positions.len();
    println!("mesh vertices {n}, clips {}", clips.len());
    let gn: usize = std::env::var("GAME_VERTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(n);
    let rec = gn * 16;
    let samples = bytes.len() / rec;
    let be = |i: usize| f32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    // pre-skin all clip frames
    let mut table: Vec<(String, i64, Vec<[f32; 3]>)> = Vec::new();
    for (name, c) in &clips {
        for f in 0..i64::from(c.report.frames.max(1)) {
            if std::env::var("VARIANTS").is_ok() {
                if let Some(pose) = c.frame_pose(f) {
                    for (tag, mir, hold) in [
                        ("hold", false, true),
                        ("mir", true, false),
                        ("mirhold", true, true),
                    ] {
                        let mut bp = skinned::BlendPose::bind(&c.rig);
                        if mir {
                            bp.lerp_mirrored(&c.rig, &pose, 1.0);
                        } else {
                            bp.lerp(&c.rig, &pose, 1.0);
                        }
                        if hold {
                            bp.hold_root_rotation(&c.rig);
                        }
                        let ax: u32 = std::env::var("AXIS")
                            .ok()
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0);
                        let p = c
                            .blended_positions(&bp)
                            .iter()
                            .map(|q| if ax == 1 { [q[0], q[2], -q[1]] } else { *q })
                            .collect();
                        table.push((format!("{name}+{tag}"), f, p));
                    }
                }
            }
            if let Some(p) = c.frame_positions(f) {
                let ax: u32 = std::env::var("AXIS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                let p = p
                    .iter()
                    .map(|q| match ax {
                        1 => [q[0], q[2], -q[1]],
                        2 => [q[0], -q[2], q[1]],
                        3 => [-q[0], q[2], q[1]],
                        4 => [-q[0], -q[2], -q[1]],
                        _ => *q,
                    })
                    .map(|q: [f32; 3]| {
                        if std::env::var("MIRROR").is_ok() {
                            [-q[0], q[1], q[2]]
                        } else {
                            q
                        }
                    })
                    .collect();
                table.push((name.clone(), f, p));
            }
        }
    }
    if std::env::var("BIND_BLEND").is_ok() {
        // lerp(bind pose, clip frame, w): the clip played at partial weight over the bind pose
        let ax: u32 = std::env::var("AXIS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        for (name, c) in &clips {
            for f in 0..i64::from(c.report.frames.max(1)) {
                let Some(pose) = c.frame_pose(f) else {
                    continue;
                };
                for k in 1..=10 {
                    let w = k as f32 / 10.0;
                    let mut bp = skinned::BlendPose::bind(&c.rig);
                    bp.lerp(&c.rig, &pose, w);
                    let p = c
                        .blended_positions(&bp)
                        .iter()
                        .map(|q| if ax == 1 { [q[0], q[2], -q[1]] } else { *q })
                        .collect();
                    table.push((format!("{name}*w{w}"), f, p));
                }
            }
        }
    }
    if let Ok(path) = std::env::var("OURS_DUMP") {
        // viewer dump (LBP_DUMP_VERTS): f32 LE xyz, one pose after another
        let raw = std::fs::read(path).map_err(|e| e.to_string())?;
        if std::env::var("ONLY_DUMP").is_ok() {
            table.clear();
        }
        let ax: u32 = std::env::var("AXIS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        for (i, rec) in raw.chunks_exact(n * 12).enumerate() {
            let p: Vec<[f32; 3]> = rec
                .chunks_exact(12)
                .map(|c| {
                    let f = |k: usize| f32::from_le_bytes([c[k], c[k + 1], c[k + 2], c[k + 3]]);
                    let q = [f(0), f(4), f(8)];
                    match ax {
                        1 => [q[0], q[2], -q[1]],
                        _ => q,
                    }
                })
                .collect();
            table.push(("ours".to_string(), i as i64, p));
        }
    }
    if let Ok(spec) = std::env::var("BLEND") {
        // BLEND=a,b: every frame of clip a with clip b's frames lerped (w .25 .5 .75 1) or
        // added against b's frame 0 (w .5 1)
        let names: Vec<&str> = spec.split(',').collect();
        let find = |n: &str| clips.iter().find(|(g, _)| g == n).map(|(_, c)| c);
        let (Some(a), Some(b)) = (find(names[0]), find(names[1])) else {
            return Err("BLEND clips must be in the clip list".into());
        };
        let ax: u32 = std::env::var("AXIS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let conv = |v: Vec<[f32; 3]>| -> Vec<[f32; 3]> {
            v.iter()
                .map(|q| if ax == 1 { [q[0], q[2], -q[1]] } else { *q })
                .collect()
        };
        for fa in 0..i64::from(a.report.frames) {
            let Some(pa) = a.frame_pose(fa) else { continue };
            for fb in 0..i64::from(b.report.frames) {
                if std::env::var("BLEND_FB")
                    .ok()
                    .and_then(|v| v.parse::<i64>().ok())
                    .is_some_and(|only| only != fb)
                {
                    continue;
                }
                let (Some(pb), Some(p0)) = (b.frame_pose(fb), b.frame_pose(0)) else {
                    continue;
                };
                for w in [0.0f32, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.5, 0.6] {
                    let mut bp = skinned::BlendPose::bind(&a.rig);
                    bp.lerp(&a.rig, &pa, 1.0);
                    bp.lerp(&b.rig, &pb, w);
                    table.push((
                        format!("{}@{fa}+lerp({})@{fb}*{w}", names[0], names[1]),
                        0,
                        conv(a.blended_positions(&bp)),
                    ));
                }
                let pm = b.frame_pose(i64::from(b.report.frames) / 2).unwrap_or(p0);
                for w in [0.5f32, 1.0] {
                    let mut bp = skinned::BlendPose::bind(&a.rig);
                    bp.lerp(&a.rig, &pa, 1.0);
                    bp.add(&b.rig, &pb, &pm, w);
                    table.push((
                        format!("{}@{fa}+add({})@{fb}*{w}", names[0], names[1]),
                        0,
                        conv(a.blended_positions(&bp)),
                    ));
                }
            }
        }
    }
    let flip = args.get(6).is_some();
    println!("samples {samples}, pose table {}", table.len());
    if std::env::var("HEIGHTS").is_ok() {
        // lowest / highest vertex and height of every table pose (no fit)
        for (name, f, p) in &table {
            let lo = p.iter().map(|q| q[1]).fold(f32::MAX, f32::min);
            let hi = p.iter().map(|q| q[1]).fold(f32::MIN, f32::max);
            println!("{name} {f} lo {lo:.1} hi {hi:.1} h {:.1}", hi - lo);
        }
        return Ok(());
    }
    // STEP=n fits every nth sample; SUMMARY=1 prints only the mean of the best fits
    let step: usize = std::env::var("STEP")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let summary = std::env::var("SUMMARY").is_ok();
    let mut sum = (0f32, 0u32, [0f32; 5]);
    for s in (0..samples).step_by(step.max(1)) {
        let g: Vec<[f32; 3]> = (0..gn)
            .map(|v| {
                let o = s * rec + v * 16;
                let mut p = [be(o), be(o + 4), be(o + 8)];
                if flip {
                    p[0] = -p[0];
                    p[2] = -p[2];
                }
                p
            })
            .collect();
        let grid = Grid::new(&g, 6.0);
        let mut best: Vec<(f32, usize)> = table
            .iter()
            .enumerate()
            .map(|(i, (_, _, p))| {
                if std::env::var("ORDERED").is_ok() {
                    (fit_ordered(&g, p), i)
                } else {
                    (fit(&grid, &g, p).0, i)
                }
            })
            .collect();
        best.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let b = best[0];
        if s == 0 {
            let bb = |v: &[[f32; 3]]| {
                let mut lo = [f32::INFINITY; 3];
                let mut hi = [f32::NEG_INFINITY; 3];
                for p in v {
                    for i in 0..3 {
                        lo[i] = lo[i].min(p[i]);
                        hi[i] = hi[i].max(p[i]);
                    }
                }
                [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]]
            };
            println!("bbox game {:?} ours {:?}", bb(&g), bb(&table[b.1].2));
        }
        let (_, err) = fit(&grid, &g, &table[b.1].2);
        // worst joints by mean residual
        let mut per = std::collections::BTreeMap::<u8, (f32, u32)>::new();
        for (e, j) in err.iter().zip(&clips[0].1.skin.joints) {
            if e.is_finite() {
                let k = per.entry(j[0]).or_default();
                k.0 += e;
                k.1 += 1;
            }
        }
        let mut worst: Vec<_> = per
            .into_iter()
            .map(|(j, (t, c))| (t / c as f32, j))
            .collect();
        worst.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        if std::env::var("PGROUPS").is_ok() {
            let grp = |j: u8| match j {
                22 | 23 | 24 | 27 | 28 | 29 | 32 => 0,
                30 | 31 => 1,
                33..=42 => 2,
                5 | 6 | 8 | 10 | 14 | 16 | 25 | 26 => 3,
                _ => 4,
            };
            let mut acc = [(0f32, 0u32); 5];
            for (e, j) in err.iter().zip(&clips[0].1.skin.joints) {
                if e.is_finite() {
                    let a = &mut acc[grp(j[0])];
                    a.0 += e * e;
                    a.1 += 1;
                }
            }
            let g: Vec<String> = ["headx", "arm", "hand", "leg", "body"]
                .iter()
                .zip(acc)
                .map(|(n, (t, c))| format!("{n} {:.1}", (t / c.max(1) as f32).sqrt()))
                .collect();
            for (k, (t, c)) in acc.iter().enumerate() {
                sum.2[k] += (t / (*c).max(1) as f32).sqrt();
            }
            if !summary {
                println!("  groups {}", g.join(" "));
            }
        }
        if std::env::var("SIGNED").is_ok() {
            // mean signed vertical offset game - ours per joint (ours placed by the fit)
            let t = LAST_T.with(|c| c.get());
            let (sn, cs) = LAST_YAW.with(|c| c.get()).sin_cos();
            let mut acc = std::collections::BTreeMap::<u8, (f32, u32)>::new();
            for (q, j) in table[b.1].2.iter().zip(&clips[0].1.skin.joints) {
                let w = [
                    cs * q[0] - sn * q[2] + t[0],
                    q[1] + t[1],
                    sn * q[0] + cs * q[2] + t[2],
                ];
                if let Some((n, d)) = grid.nearest(w) {
                    if d < 12.0 {
                        let e = acc.entry(j[0]).or_default();
                        e.0 += n[1];
                        e.1 += 1;
                    }
                }
            }
            let v: Vec<String> = acc
                .into_iter()
                .filter(|(_, (_, c))| *c > 20)
                .map(|(j, (t, c))| format!("{j}:{:.2}", t / c as f32))
                .collect();
            println!("  dy {}", v.join(" "));
        }
        let w: Vec<String> = worst
            .iter()
            .take(4)
            .map(|(e, j)| format!("j{j}:{e:.1}"))
            .collect();
        let t: Vec<String> = best
            .iter()
            .take(3)
            .map(|(r, i)| format!("{}@{} {:.1}", table[*i].0, table[*i].1, r))
            .collect();
        let ty = LAST_T.with(|c| c.get())[1];
        sum.0 += b.0;
        sum.1 += 1;
        if summary {
            continue;
        }
        println!(
            "sample {s}: {} | worst {} | ty {ty:.1}",
            t.join(" ; "),
            w.join(" ")
        );
    }
    if summary && sum.1 > 0 {
        let n = sum.1 as f32;
        println!(
            "mean best rms {:.2} over {} samples; headx arm hand leg body {:.2?}",
            sum.0 / n,
            sum.1,
            sum.2.map(|v| v / n)
        );
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(2);
    }
}
