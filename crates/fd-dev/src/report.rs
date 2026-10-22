//! BENC01 report compatibility: same calculations/schema as the retired Python tool.
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::path::Path;
type Result<T> = std::result::Result<T, String>;

fn text(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}
fn field<'a>(value: &'a Value, path: &[&str]) -> Result<&'a Value> {
    let mut at = value;
    for key in path {
        at = at
            .get(key)
            .ok_or_else(|| format!("missing field {}", path.join(".")))?;
    }
    Ok(at)
}
fn val(value: &Value, path: &[&str]) -> Result<Value> {
    field(value, path).cloned()
}
fn num(value: &Value, path: &[&str]) -> Result<f64> {
    field(value, path)?
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("invalid number {}", path.join(".")))
}
fn list<'a>(value: &'a Value, path: &[&str]) -> Result<&'a Vec<Value>> {
    field(value, path)?
        .as_array()
        .ok_or_else(|| format!("invalid array {}", path.join(".")))
}
fn boolean(value: &Value, path: &[&str]) -> Result<bool> {
    field(value, path)?
        .as_bool()
        .ok_or_else(|| format!("invalid boolean {}", path.join(".")))
}
fn string<'a>(value: &'a Value, path: &[&str]) -> Result<&'a str> {
    field(value, path)?
        .as_str()
        .ok_or_else(|| format!("invalid string {}", path.join(".")))
}
fn jsonl(path: &Path) -> Result<Vec<Value>> {
    text(path)?
        .lines()
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .map(|(i, l)| {
            serde_json::from_str(l).map_err(|e| format!("{} line{}: {e}", path.display(), i + 1))
        })
        .collect()
}
fn seconds(frame: &Value) -> Result<f64> {
    num(
        list(frame, &["runs"])?.first().ok_or("frame has no run")?,
        &["seconds"],
    )
}
fn sum(frames: &[Value]) -> Result<f64> {
    frames.iter().map(seconds).sum()
}
fn divide(a: f64, b: f64) -> Result<f64> {
    if b == 0.0 {
        return Err("report division by zero".into());
    }
    let result = a / b;
    if result.is_finite() {
        Ok(result)
    } else {
        Err("nonfinite report result".into())
    }
}
fn gnu_time(path: &Path) -> Result<Value> {
    let text = text(path)?;
    let mut out = Map::new();
    for line in text.lines() {
        for (label, key, scale) in [
            (
                "Maximum resident set size (kbytes):",
                "process_peak_rss_bytes",
                1024u64,
            ),
            ("exit ", "exit", 1),
        ] {
            if let Some((before, after)) = line.split_once(label) {
                if key == "exit" && !before.is_empty() {
                    continue;
                }
                let number: u64 = after
                    .trim()
                    .parse()
                    .map_err(|_| format!("{}: invalid {key}", path.display()))?;
                out.insert(
                    key.into(),
                    json!(number.checked_mul(scale).ok_or("time integer overflow")?),
                );
            }
        }
        for (label, key) in [
            ("User time (seconds):", "user_seconds"),
            ("System time (seconds):", "system_seconds"),
        ] {
            if let Some((_, after)) = line.split_once(label) {
                let n: f64 = after
                    .trim()
                    .parse()
                    .map_err(|_| "invalid process seconds")?;
                if !n.is_finite() {
                    return Err("nonfinite process seconds".into());
                }
                out.insert(key.into(), json!(n));
            }
        }
        if let Some((_, after)) = line.split_once("Elapsed (wall clock) time (h:mm:ss or m:ss):") {
            let mut seconds = 0.0;
            for part in after.trim().split(':') {
                let n: f64 = part.parse().map_err(|_| "invalid elapsed time")?;
                seconds = seconds * 60.0 + n;
            }
            if !seconds.is_finite() {
                return Err("nonfinite elapsed time".into());
            }
            out.insert("wall_seconds".into(), json!(seconds));
        }
    }
    Ok(Value::Object(out))
}
fn compile_log(path: &Path) -> Result<Value> {
    const KEEP:&str="frames atlas.bytes atlas.chunks atlas.bla_share orbits bla_tables reuse.frames_per_orbit.mean reuse.frames_per_orbit.max reuse.frames_per_bla.mean reuse.frames_per_bla.max reuse.frames_without_bla tiles tile_demand reuse.frames_per_tile.mean reuse.frames_per_tile.max reuse.tiles_shared dedup.ratio.chunks dedup.ratio.bytes dedup.logical_bytes budget.target budget.cap budget.headroom budget.within_cap seconds.plan seconds.estimate seconds.build seconds.reference seconds.operator seconds.certification seconds.manifests seconds.verify seconds.wall peak_rss_bytes makespan_seconds workers";
    let mut out = Map::new();
    for line in text(path)?.lines() {
        let words: Vec<_> = line.split_whitespace().collect();
        if words.len() == 2 && KEEP.split_whitespace().any(|k| k == words[0]) {
            let value = if let Ok(v) = words[1].parse::<i64>() {
                json!(v)
            } else if let Ok(v) = words[1].parse::<f64>() {
                if !v.is_finite() {
                    return Err("nonfinite compile value".into());
                }
                json!(v)
            } else {
                json!(words[1])
            };
            out.insert(words[0].into(), value);
        }
    }
    Ok(Value::Object(out))
}
fn arm(records: Vec<Value>, control: bool) -> Result<(Value, Vec<Value>)> {
    let frames: Vec<_> = records
        .iter()
        .filter(|r| r.get("record") == Some(&json!("frame")))
        .cloned()
        .collect();
    let totals = records
        .iter()
        .rev()
        .find(|r| r.get("record") == Some(&json!("totals")))
        .ok_or("missing totals record")?;
    if frames.is_empty() {
        return Err("report arm has no frames".into());
    }
    let oracle: Vec<_> = frames
        .iter()
        .filter(|r| {
            r.get("oracle")
                .is_some_and(|o| !o.is_null() && o != &json!(false))
        })
        .collect();
    let mut oracle_frames = Vec::new();
    let mut failed_frames = Vec::new();
    let mut failures = Map::new();
    let mut passed = 0;
    let mut mismatches = 0u64;
    for r in oracle {
        let f = val(r, &["frame"])?;
        oracle_frames.push(f.clone());
        if boolean(r, &["oracle", "ok"])? {
            passed += 1;
        } else {
            failed_frames.push(f.clone());
            failures.insert(
                f.to_string(),
                json!(list(r, &["oracle", "failures"])?
                    .iter()
                    .take(3)
                    .collect::<Vec<_>>()),
            );
        }
        mismatches += field(r, &["oracle", "class_mismatch"])?
            .as_u64()
            .ok_or("invalid oracle mismatch count")?;
    }
    let total = sum(&frames)?;
    let per_frame = divide(total, frames.len() as f64)?;
    let mut out = json!({"frames":frames.len(),"seconds_total":total,"seconds_per_frame":per_frame,"iterations_executed":val(totals,&["iterations","total"] )?,"iterations_per_pixel":val(totals,&["iterations","per_pixel"] )?,"fallback":val(totals,&["fallback"] )?,"macro_operators_per_pixel":val(totals,&["atlas_work","macro_operators_per_pixel"] )?,"peak_rss_bytes_per_frame_max":val(totals,&["memory","peak_rss_bytes"] )?,"classes":val(totals,&["classes"] )?,"oracle":{"frames":oracle_frames,"passed":passed,"failed_frames":failed_frames,"class_mismatches":mismatches,"failures":failures},"ok":val(totals,&["ok"] )?,"error":val(totals,&["error"] )?});
    if control {
        out["cold_seconds_per_frame"] = json!(per_frame);
        out["warm_seconds_per_frame"] = Value::Null;
        out["reference_seconds_total"] = val(totals, &["reference_seconds", "total"])?;
        out["bytes_read"] = json!(0);
        if let Some(bla) = totals.get("bla") {
            out["bla"] = bla.clone();
            out["operator_seconds_total"] = val(bla, &["operator_seconds", "total"])?;
            out["iterations_equivalent"] = val(bla, &["iterations_equivalent"])?;
        } else {
            out["operator_seconds_total"] = json!(0);
            out["iterations_equivalent"] = val(totals, &["iterations", "total"])?;
        }
    } else {
        for key in [
            "cold_seconds_per_frame",
            "cold_frames",
            "warm_seconds_per_frame",
            "warm_frames",
        ] {
            out[key] = val(totals, &["timing", key])?;
        }
        out["load_seconds"] = val(totals, &["load_seconds"])?;
        out["reference_seconds_total"] = json!(0);
        out["operator_seconds_total"] = json!(0);
        out["bytes_read"] = val(totals, &["bytes", "atlas_bytes_read"])?;
        out["bytes"] = val(totals, &["bytes"])?;
        out["tiles_touched_per_frame"] = val(totals, &["atlas_work", "tiles_touched_per_frame"])?;
        out["bla"] = val(totals, &["bla"])?;
        out["iterations_equivalent"] = val(totals, &["iterations", "equivalent"])?;
    }
    Ok((out, frames))
}
fn round2(value: f64) -> Result<f64> {
    format!("{value:.2}")
        .parse()
        .map_err(|_| "invalid rounded depth".into())
}
fn depth_bins(a: &[Value], b: &[Value], c: &[Value]) -> Result<Vec<Value>> {
    if b.len() != a.len() || c.len() != a.len() {
        return Err("arm frame counts differ".into());
    }
    let mut bins = Vec::new();
    for start in (0..a.len()).step_by(75) {
        let end = (start + 75).min(a.len());
        let n = (end - start) as f64;
        let sa = sum(&a[start..end])?;
        let sb = sum(&b[start..end])?;
        let sc = sum(&c[start..end])?;
        let kernels = a[start..end]
            .iter()
            .map(|r| string(r, &["kernel"]).map(|s| s.split(' ').next().unwrap_or("").to_owned()))
            .collect::<Result<BTreeSet<_>>>()?;
        let used_b = b[start..end]
            .iter()
            .map(|r| string(r, &["bla", "use"]).map(|s| usize::from(s == "used")))
            .sum::<Result<usize>>()?;
        let used_c = c[start..end]
            .iter()
            .map(|r| string(r, &["play", "bla_use"]).map(|s| usize::from(s == "used")))
            .sum::<Result<usize>>()?;
        bins.push(json!({"frames":[start,end],"log10_width":[round2(num(&a[start],&["depth","log10_width"] )?)?,round2(num(&a[end-1],&["depth","log10_width"] )?)?],"kernel_A":kernels,"A_seconds_per_frame":sa/n,"B_seconds_per_frame":sb/n,"C_seconds_per_frame":sc/n,"B_frames_bla_used":used_b,"C_frames_bla_used":used_c,"A_over_C":divide(sa,sc)?,"B_over_C":divide(sb,sc)?}));
    }
    Ok(bins)
}
fn break_even(compile: f64, frames: &[Value], atlas: &[Value]) -> Result<Value> {
    let px = sum(frames)?;
    let pc = sum(atlas)?;
    let gain = px - pc;
    let mut first = None;
    let (mut cx, mut cc) = (0.0, compile);
    for (f, (a, c)) in frames.iter().zip(atlas).enumerate() {
        cx += seconds(a)?;
        cc += seconds(c)?;
        if first.is_none() && cc <= cx {
            first = Some(f + 1);
        }
    }
    Ok(
        json!({"independent_path_seconds":px,"atlas_play_seconds":pc,"saving_per_play_seconds":gain,"compile_seconds":compile,"plays_to_break_even":if gain<=0.0{None}else{Some(compile/gain)},"first_play_break_even_frames":first,"first_play_break_even_fraction":first.map(|f|f as f64/frames.len() as f64)}),
    )
}
fn comparison(path: &Path) -> Result<Value> {
    let records = jsonl(path)?;
    let (totals, frames) = records.split_last().ok_or("empty comparison log")?;
    let mut out = Map::new();
    for key in [
        "frames",
        "errors",
        "frames_class_identical",
        "frames_bytes_identical",
        "frames_width_off",
        "samples",
        "class_mismatches",
        "kinds",
        "nu",
        "px",
        "ok",
    ] {
        out.insert(key.into(), val(totals, &[key])?);
    }
    let mut bad = Vec::new();
    let mut worst = Vec::new();
    for r in frames {
        if r.get("class_mismatches").and_then(Value::as_u64) != Some(0) {
            bad.push(val(r, &["frame"])?);
        }
        if r.get("nu").is_some() {
            worst.push((num(r, &["nu", "px_max"])?, val(r, &["frame"])?));
        }
    }
    worst.sort_by(|a, b| b.0.total_cmp(&a.0));
    out.insert("frames_with_class_mismatch".into(), json!(bad));
    out.insert(
        "worst_nu_px_frames".into(),
        json!(worst
            .into_iter()
            .take(3)
            .map(|(v, f)| json!([f, v]))
            .collect::<Vec<_>>()),
    );
    Ok(Value::Object(out))
}

/// Read a captured BENC01 run and preserve the historical report contract.
pub fn run(dir: &Path) -> Result<Value> {
    let (mut a, fa) = arm(jsonl(&dir.join("A.jsonl"))?, true)?;
    let (mut b, fb) = arm(jsonl(&dir.join("B.jsonl"))?, true)?;
    let (mut c, fc) = arm(jsonl(&dir.join("C.jsonl"))?, false)?;
    for (arm, name) in [(&mut a, "A.time"), (&mut b, "B.time"), (&mut c, "C.time")] {
        arm["process"] = gnu_time(&dir.join(name))?;
    }
    let mut compile = compile_log(&dir.join("C.compile.log"))?;
    compile["process"] = gnu_time(&dir.join("C.compile.time"))?;
    let compile_s = num(&compile, &["process", "wall_seconds"])?;
    let mut cmp = Map::new();
    for pair in ["AB", "AC", "BC"] {
        cmp.insert(
            pair.into(),
            comparison(&dir.join(format!("cmp{pair}.jsonl")))?,
        );
    }
    let mut load = Vec::new();
    for line in text(&dir.join("load.log"))?.lines() {
        let w: Vec<_> = line.split_whitespace().collect();
        if w.len() >= 2 {
            let v: f64 = w[1].parse().map_err(|_| "invalid load sample")?;
            if !v.is_finite() {
                return Err("nonfinite load sample".into());
            }
            load.push(v);
        }
    }
    if load.is_empty() {
        return Err("load.log has no samples".into());
    }
    let loads = json!({"min":load.iter().copied().fold(f64::INFINITY,f64::min),"max":load.iter().copied().fold(f64::NEG_INFINITY,f64::max),"mean":load.iter().sum::<f64>()/load.len() as f64,"samples":load.len()});
    let phases: Vec<_> = text(&dir.join("phases.log"))?
        .trim()
        .lines()
        .map(str::to_owned)
        .collect();
    Ok(
        json!({"schema":"fd-benc01/1","path":"bench/path-atlas-v0.txt","settings":{"size":"960x540","ss":1,"iter":100000,"columns":"nu,de,normal","kernel":"auto","threads":12,"compile_workers":12},"machine":{"cpu":"Ryzen 9 3900X, 12 cores / 24 threads","shared":true,"load_1min":loads,"phases":phases},"arms":{"A_independent_plain":a,"B_independent_per_frame_bla":b,"C_atlas_play":c},"compile":compile,"speedup":{"A_over_C":divide(num(&a,&["seconds_total"] )?,num(&c,&["seconds_total"] )?)?,"B_over_C":divide(num(&b,&["seconds_total"] )?,num(&c,&["seconds_total"] )?)?,"A_over_B":divide(num(&a,&["seconds_total"] )?,num(&b,&["seconds_total"] )?)?},"break_even":{"vs_A":break_even(compile_s,&fa,&fc)?,"vs_B":break_even(compile_s,&fb,&fc)?},"depth_bins":depth_bins(&fa,&fb,&fc)?,"correctness":cmp}),
    )
}
