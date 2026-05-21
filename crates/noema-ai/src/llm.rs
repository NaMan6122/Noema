pub struct LlmEngine {
    loaded: bool,
}

impl LlmEngine {
    pub fn new() -> Self {
        Self { loaded: false }
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }
}
