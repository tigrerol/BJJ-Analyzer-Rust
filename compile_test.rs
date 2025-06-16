// Quick compilation test to check for major issues
use std::path::PathBuf;

// Test if basic imports work
fn main() {
    println!("Compilation test");
}

// Test state management imports
#[cfg(feature = "test")]
mod test {
    use crate::state::{StateManager, VideoProcessingState, ProcessingStage};
    use crate::processing::{BatchProcessor, ProcessingResult};
}