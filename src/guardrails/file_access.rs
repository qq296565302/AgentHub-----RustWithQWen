use crate::error::{Result, SecurityError};
use regex::Regex;
use lazy_static::lazy_static;
use std::path::{Path, PathBuf};

lazy_static! {
    static ref PATH_TRAVERSAL: Regex = Regex::new(r"\.\.[/\\]").unwrap();
    static ref ABSOLUTE_PATH: Regex = Regex::new(r"^[/\\]").unwrap();
}

pub struct FileAccessGuard {
    #[allow(dead_code)]
    enabled: bool,
    #[allow(dead_code)]
    workspace_dir: PathBuf,
    #[allow(dead_code)]
    sensitive_files: Vec<String>,
    #[allow(dead_code)]
    allow_symlinks: bool,
}

impl FileAccessGuard {
    pub fn new(enabled: bool, workspace_dir: &str, sensitive_files: &[String], allow_symlinks: bool) -> Self {
        let workspace = if workspace_dir.starts_with('~') {
            dirs::home_dir()
                .map(|h| h.join(&workspace_dir[2..]))
                .unwrap_or_else(|| PathBuf::from(workspace_dir))
        } else {
            PathBuf::from(workspace_dir)
        };

        Self {
            enabled,
            workspace_dir: workspace,
            sensitive_files: sensitive_files.to_vec(),
            allow_symlinks,
        }
    }

    #[allow(dead_code)]
    pub fn check_access(&self, path: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        if PATH_TRAVERSAL.is_match(path) {
            return Err(SecurityError::FileAccessBlocked {
                path: path.to_string(),
            }
            .into());
        }

        let path_obj = Path::new(path);
        let file_name = path_obj
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        for sensitive in &self.sensitive_files {
            if file_name.contains(sensitive.trim_end_matches('/'))
                || path.contains(sensitive)
            {
                return Err(SecurityError::FileAccessBlocked {
                    path: path.to_string(),
                }
                .into());
            }
        }

        if !self.allow_symlinks && path_obj.is_symlink() {
            let target = std::fs::read_link(path_obj).ok();
            if let Some(target) = target {
                if !target.starts_with(&self.workspace_dir) {
                    return Err(SecurityError::FileAccessBlocked {
                        path: path.to_string(),
                    }
                    .into());
                }
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn is_within_workspace(&self, path: &str) -> bool {
        let path_obj = Path::new(path).canonicalize().ok();
        if let Some(canonical) = path_obj {
            let workspace = self.workspace_dir.canonicalize().ok();
            if let Some(ws) = workspace {
                return canonical.starts_with(ws);
            }
        }
        false
    }
}
