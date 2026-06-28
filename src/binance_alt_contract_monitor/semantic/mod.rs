pub mod exposure_gate;
pub mod lock;
pub mod sanitizer;
pub mod severity;

pub use exposure_gate::{evaluate_exposure_gate, AltContractExposureDecision};
pub use lock::{apply_semantic_boundary, seed_semantic_view};
