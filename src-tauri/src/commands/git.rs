use crate::git::service::GitService;
use crate::git::types::*;
use tauri::command;

#[command]
pub async fn git_init(path: String, bare: bool) -> Result<GitRepository, String> {
    let service = GitService::new();
    service.init(&path, bare).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_open(path: String) -> Result<GitRepository, String> {
    let service = GitService::new();
    service.open(&path).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_get_repository_info(path: String) -> Result<GitRepository, String> {
    let service = GitService::new();
    service.get_repository_info(&path).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_status(path: String) -> Result<GitStatus, String> {
    let service = GitService::new();
    service.status(&path).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_diff(path: String, options: Option<GitDiffOptions>) -> Result<GitDiff, String> {
    let service = GitService::new();
    service.diff(&path, options).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_log(path: String, options: GitLogOptions) -> Result<Vec<GitCommit>, String> {
    let service = GitService::new();
    service.log(&path, options).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_blame(path: String, file_path: String) -> Result<GitBlame, String> {
    let service = GitService::new();
    service.blame(&path, &file_path).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_add(path: String, files: Vec<String>) -> Result<(), String> {
    let service = GitService::new();
    service.add(&path, files).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_commit(path: String, options: GitCommitOptions) -> Result<String, String> {
    let service = GitService::new();
    service.commit(&path, options).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_push(path: String, options: GitPushOptions) -> Result<(), String> {
    let service = GitService::new();
    service.push(&path, options).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_pull(path: String, options: GitPullOptions) -> Result<(), String> {
    let service = GitService::new();
    service.pull(&path, options).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_fetch(path: String, options: GitFetchOptions) -> Result<(), String> {
    let service = GitService::new();
    service.fetch(&path, options).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_create_branch(path: String, options: GitBranchOptions) -> Result<(), String> {
    let service = GitService::new();
    service.create_branch(&path, options).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_delete_branch(path: String, name: String, force: bool) -> Result<(), String> {
    let service = GitService::new();
    service.delete_branch(&path, &name, force).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_checkout(path: String, target: String) -> Result<(), String> {
    let service = GitService::new();
    service.checkout(&path, &target).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_stash(path: String, options: GitStashOptions) -> Result<(), String> {
    let service = GitService::new();
    service.stash(&path, options).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_stash_list(path: String) -> Result<Vec<GitStashEntry>, String> {
    let service = GitService::new();
    service.stash_list(&path).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_stash_pop(path: String, index: Option<usize>) -> Result<(), String> {
    let service = GitService::new();
    service.stash_pop(&path, index).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_submodules(path: String) -> Result<Vec<GitSubmodule>, String> {
    let service = GitService::new();
    service.submodules(&path).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_worktrees(path: String) -> Result<Vec<GitWorktree>, String> {
    let service = GitService::new();
    service.worktrees(&path).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_tags(path: String) -> Result<Vec<GitTag>, String> {
    let service = GitService::new();
    service.tags(&path).await.map_err(|e| e.to_string())
}

#[command]
pub async fn git_merge(path: String, options: GitMergeOptions) -> Result<(), String> {
    let service = GitService::new();
    service.merge(&path, options).await.map_err(|e| e.to_string())
}