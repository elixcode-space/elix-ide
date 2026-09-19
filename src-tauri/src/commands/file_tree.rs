use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;
use walkdir::{DirEntry, WalkDir};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct FileTreeNode {
    pub path: String,
    pub name: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub size: Option<u64>,
    pub modified: Option<u64>,
    pub children: Option<Vec<FileTreeNode>>,
    pub expanded: Option<bool>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct FileTreeOptions {
    pub root: String,
    pub max_depth: Option<usize>,
    pub include_hidden: Option<bool>,
    pub include_patterns: Option<Vec<String>>,
    pub exclude_patterns: Option<Vec<String>>,
}

#[tauri::command]
pub async fn get_file_tree(
    options: FileTreeOptions,
) -> Result<FileTreeNode, String> {
    let root_path = PathBuf::from(&options.root);

    if !root_path.exists() {
        return Err(format!("Root path does not exist: {}", options.root));
    }

    let max_depth = options.max_depth.unwrap_or(10);
    let include_hidden = options.include_hidden.unwrap_or(false);
    let include_patterns = options.include_patterns.unwrap_or_default();
    let exclude_patterns = options.exclude_patterns.unwrap_or_else(|| {
        vec![
            "node_modules".to_string(),
            ".git".to_string(),
            "target".to_string(),
            "dist".to_string(),
            "build".to_string(),
            ".next".to_string(),
            ".cache".to_string(),
            "*.log".to_string(),
        ]
    });

    let mut root_node = FileTreeNode {
        path: root_path.to_string_lossy().to_string(),
        name: root_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| options.root.clone()),
        node_type: "directory".to_string(),
        size: None,
        modified: None,
        children: Some(Vec::new()),
        expanded: Some(true),
    };

    let mut entries_map: HashMap<PathBuf, Vec<DirEntry>> = HashMap::new();

    for entry in WalkDir::new(&root_path)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        let relative = path.strip_prefix(&root_path).unwrap_or(path);
        let depth = relative.components().count();

        if depth == 0 {
            continue;
        }

        if depth > max_depth {
            continue;
        }

        let file_name = path.file_name().unwrap().to_string_lossy().to_string();

        if !include_hidden && file_name.starts_with('.') && file_name != ".gitignore" {
            continue;
        }

        let excluded = exclude_patterns.iter().any(|pattern| {
            if pattern.starts_with("*.") {
                file_name.ends_with(&pattern[1..])
            } else {
                file_name == *pattern || relative.to_string_lossy().contains(pattern)
            }
        });

        if excluded {
            continue;
        }

        if !include_patterns.is_empty() {
            let included = include_patterns.iter().any(|pattern| {
                if pattern.starts_with("*.") {
                    file_name.ends_with(&pattern[1..])
                } else {
                    file_name.contains(pattern)
                }
            });
            if !included && entry.file_type().is_dir() {
                continue;
            }
        }

        let parent = path.parent().unwrap_or(&root_path);
        entries_map.entry(parent.to_path_buf()).or_default().push(entry.to_owned());
    }

    fn build_node(
        path: &Path,
        root: &Path,
        entries_map: &HashMap<PathBuf, Vec<DirEntry>>,
        max_depth: usize,
    ) -> FileTreeNode {
        let relative = path.strip_prefix(root).unwrap_or(path);
        let depth = relative.components().count();

        let file_name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let metadata = fs::metadata(path).ok();
        let size = if metadata.as_ref().map(|m| m.is_file()).unwrap_or(false) {
            metadata.as_ref().map(|m| m.len())
        } else {
            None
        };
        let modified = metadata.as_ref().and_then(|m| m.modified().ok())
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64);

        let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let node_type = if is_dir { "directory" } else { "file" };

        let mut children = None;
        if is_dir && depth < max_depth {
            if let Some(entries) = entries_map.get(path) {
                let mut child_nodes: Vec<FileTreeNode> = entries.iter()
                    .map(|e| build_node(e.path(), root, entries_map, max_depth))
                    .collect();
                child_nodes.sort_by(|a, b| {
                    match (a.node_type.as_str(), b.node_type.as_str()) {
                        ("directory", "file") => std::cmp::Ordering::Less,
                        ("file", "directory") => std::cmp::Ordering::Greater,
                        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                    }
                });
                children = Some(child_nodes);
            }
        }

        FileTreeNode {
            path: path.to_string_lossy().to_string(),
            name: file_name,
            node_type: node_type.to_string(),
            size,
            modified,
            children,
            expanded: if is_dir { Some(false) } else { None },
        }
    }

    root_node.children = Some({
        let mut children = Vec::new();
        if let Some(entries) = entries_map.get(&root_path) {
            for entry in entries {
                children.push(build_node(entry.path(), &root_path, &entries_map, max_depth));
            }
        }
        children.sort_by(|a, b| {
            match (a.node_type.as_str(), b.node_type.as_str()) {
                ("directory", "file") => std::cmp::Ordering::Less,
                ("file", "directory") => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });
        children
    });

    Ok(root_node)
}

#[tauri::command]
pub async fn create_file(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::File::create(&path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn create_folder(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_path(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    if path.is_dir() {
        fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
    } else {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn rename_path(old_path: String, new_path: String) -> Result<(), String> {
    let old_path = PathBuf::from(old_path);
    let new_path = PathBuf::from(new_path);
    if let Some(parent) = new_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::rename(&old_path, &new_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_file_info(path: String) -> Result<FileTreeNode, String> {
    let path = PathBuf::from(path);
    let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;

    let name = path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let size = if metadata.is_file() {
        Some(metadata.len())
    } else {
        None
    };

    let modified = metadata.modified().ok()
        .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64);

    Ok(FileTreeNode {
        path: path.to_string_lossy().to_string(),
        name,
        node_type: if metadata.is_dir() { "directory" } else { "file" }.to_string(),
        size,
        modified,
        children: None,
        expanded: None,
    })
}