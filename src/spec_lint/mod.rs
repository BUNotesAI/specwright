mod linters;
mod pipeline;

pub use pipeline::LintPipeline;

// Used by tests in linters.rs via crate::spec_lint::cross_check
#[cfg(test)]
pub use pipeline::cross_check;
