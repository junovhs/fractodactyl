// Shared deterministic triangular noise; preserves legacy output exactly.
fn tpdf(key: u32) -> f32 {
    var x = key + 0x9e3779b9u;
    x ^= x >> 16u;
    x *= 0x7feb352du;
    x ^= x >> 15u;
    x *= 0x846ca68bu;
    x ^= x >> 16u;
    return f32((x & 65535u) + (x >> 16u)) / 65535.0 - 1.0;
}
