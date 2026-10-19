// Shared footprint filtering for legacy studio and graph styling.
// tanh for x >= 0 from exp alone. Backend tanh varies: DX12 WARP's returns NaN once
// e^2x overflows and drifts ~1e-4 near x = 4 (FIX-59); 1 - 2/(e^2x + 1) is
// well-conditioned for x >= 0, and every x >= 10 is 1 in f32.
fn tanh_bounded(x: f32) -> f32 {
    let y = clamp(x, 0.0, 10.0);
    return 1.0 - 2.0 / (exp(2.0 * y) + 1.0);
}


fn band_response(cycles: f32) -> f32 {
    return exp(-4.9348022 * cycles * cycles);
}
