use std::sync::Arc;

use repository::traits::Repository;

pub struct AppState<R: Repository> {
    pub repository: Arc<R>,
}

impl<R: Repository> AppState<R> {
    pub fn new(repository: R) -> Self {
        Self {
            repository: Arc::new(repository),
        }
    }
}

impl<R: Repository> Clone for AppState<R> {
    fn clone(&self) -> Self {
        Self {
            repository: self.repository.clone(),
        }
    }
}
