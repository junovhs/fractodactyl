#[path = "support/mod.rs"]
mod support;
fn main() -> Result<(), String> {
    let edges = support::load_edges(std::path::Path::new("bench/truth/v1/precision-edges.txt"))?;
    println!("versioned_edge_records={}", edges.len());
    let mut case = support::load(std::path::Path::new("bench/truth/v1/filament.tsv"), 8)?;
    let plane = fd_kernel::Plane::new(&case.view, case.params.nx, case.params.ny)?;
    for index in [318usize, 988] {
        let (ux, uy) = plane.unit_offset(index % 32, index / 32);
        println!(
            "coordinate_bits index={index} cr={} ci={} ar={} ai={}",
            plane.c_re.to_bits(),
            plane.c_im.to_bits(),
            (ux * plane.h()).to_bits(),
            (uy * plane.h()).to_bits()
        );
    }
    case.params.tier = Some(fd_kernel::Tier::Fixed);
    for (route, kernel, _, s) in support::run(&case, &[])? {
        println!(
            "route={route} kernel={kernel} class_mismatches={} scalar_failures={} first={:?}",
            s.class_mismatches, s.scalar_failures, s.first_failure
        );
    }
    Ok(())
}
