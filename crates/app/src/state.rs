#![cfg(windows)]

use winsp_core::engine::Engine;

#[derive(Debug)]
pub struct AppState {
    index: Engine,
}

impl AppState {
    pub fn new(index: Engine) -> Self {
        Self { index }
    }

    pub(crate) fn update_index(&mut self, index: Engine) {
        self.index = index;
    }

    pub(crate) fn engine(&self) -> &Engine {
        &self.index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_index_replaces_the_engine() {
        let mut state = AppState::new(Engine::new());
        state.update_index(Engine::new());
        assert!(state.engine().search("", 1).is_empty());
    }
}
