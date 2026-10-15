//! Read-only census (D309): the glow alpha's form (`lbp_resource::material::glow_alpha_form`)
//! of every gmat in the catalog, from its own compiled program. Writes counts and the GUIDs
//! and paths of each form to a new JSON under ignored `local-analysis/`; no program bytes.
//! Usage: glow_alpha_census SNAPSHOT GAME-DIRS local-analysis/NEW.json
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_resource::material::GlowAlphaForm;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("glow_alpha_census SNAPSHOT GAME-DIRS local-analysis/NEW.json".into());
    }
    let output = Path::new(&args[3]);
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
        return Err("new ignored output required".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let mut seen = BTreeSet::new();
    let mut forms: BTreeMap<&str, Vec<(String, String)>> = BTreeMap::new();
    let mut failures: BTreeMap<String, usize> = BTreeMap::new();
    for r in &catalog.snapshot.resources {
        let path = r.map_row.path.replace('\\', "/");
        if !path.to_ascii_lowercase().ends_with(".gmat") {
            continue;
        }
        let guid = u32::from_str_radix(r.map_row.guid.trim(), 16).ok();
        let id = guid.map_or_else(|| r.map_row.sha1.clone(), |g| format!("{g:08x}"));
        if !seen.insert(id.clone()) {
            continue;
        }
        let key = match lbp_preview::resolve(&catalog, guid.filter(|&g| g != 0), None)
            .or_else(|_| lbp_preview::resolve(&catalog, None, Some(&r.map_row.sha1)))
        {
            Ok(k) => k,
            Err(e) => {
                *failures.entry(format!("resolve: {e}")).or_default() += 1;
                continue;
            }
        };
        if key.size > lbp_resource::material::MAX_INPUT as u64 {
            *failures.entry("over input budget".into()).or_default() += 1;
            continue;
        }
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
        let (blob, ends) = match lbp_resource::material::shader_programs(&bytes, || false) {
            Ok(p) => p,
            Err(e) => {
                *failures.entry(format!("prefix: {e}")).or_default() += 1;
                continue;
            }
        };
        let form = match lbp_resource::material::glow_alpha_form(&blob, &ends) {
            GlowAlphaForm::Fogged => "fogged",
            GlowAlphaForm::Unfogged => "unfogged",
            GlowAlphaForm::Unknown => "unknown",
        };
        forms.entry(form).or_default().push((id, path));
    }
    let counts: BTreeMap<_, _> = forms.iter().map(|(k, v)| (*k, v.len())).collect();
    println!("forms {counts:?} failures {failures:?}");
    for (id, path) in forms.get("unfogged").into_iter().flatten() {
        println!("unfogged {id} {path}");
    }
    let listed = |k: &str| {
        forms
            .get(k)
            .map(|v| {
                v.iter()
                    .map(|(id, path)| json!({"id": id, "path": path}))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    let report = json!({
        "schema": "d309-glow-alpha-census/1",
        "variant": lbp_resource::material::GLOW_ALPHA_VARIANT,
        "counts": counts,
        "failures": failures,
        "unfogged": listed("unfogged"),
        "fogged": listed("fogged"),
    });
    std::fs::write(
        output,
        serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
