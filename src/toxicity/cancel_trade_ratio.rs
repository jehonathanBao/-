pub fn compute_cancel_to_trade_ratio(cancel_qty: f64, fill_qty: f64) -> Option<f64> {
    if cancel_qty <= 0.0 || fill_qty <= f64::EPSILON {
        None
    } else {
        Some(cancel_qty / fill_qty)
    }
}

pub fn high_cancel_without_fill(cancel_qty: f64, fill_qty: f64) -> bool {
    cancel_qty > 0.0 && fill_qty <= f64::EPSILON
}
