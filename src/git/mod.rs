use anyhow::Result;

use crate::traits::VcsClient;

pub struct GitClient {
    repo_path: String,
}

impl GitClient {
    pub fn new(repo_path: &str) -> Self {
        Self {
            repo_path: repo_path.to_string(),
        }
    }
}

impl VcsClient for GitClient {
    fn create_branch(&self, name: &str) -> Result<()> {
        todo!()
    }

    fn current_branch(&self) -> Result<String> {
        todo!()
    }

    fn diff_from_main(&self) -> Result<String> {
        todo!()
    }

    fn commit(&self, message: &str) -> Result<()> {
        todo!()
    }
}
