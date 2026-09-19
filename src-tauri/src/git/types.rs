use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRepository {
    pub path: String,
    pub is_bare: bool,
    pub head: Option<String>,
    pub branches: Vec<GitBranch>,
    pub remotes: Vec<GitRemote>,
    pub worktree_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBranch {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub upstream: Option<String>,
    pub commit: GitCommitRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRemote {
    pub name: String,
    pub url: String,
    pub fetch_refspec: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitRef {
    pub id: String,
    pub short_id: String,
    pub message: String,
    pub author: GitSignature,
    pub committer: GitSignature,
    pub time: String,
    pub parents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GitSignature {
    pub name: String,
    pub email: String,
    pub time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub repository_path: String,
    pub current_branch: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub files: Vec<GitStatusFile>,
    pub stash_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatusFile {
    pub path: String,
    pub index_status: GitFileStatus,
    pub worktree_status: GitFileStatus,
    pub old_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GitFileStatus {
    Unmodified,
    Added,
    Deleted,
    Modified,
    Renamed,
    Copied,
    Untracked,
    Ignored,
    TypeChange,
    Conflict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitDiff {
    pub repository_path: String,
    pub files: Vec<GitDiffFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitDiffFile {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub old_mode: Option<u32>,
    pub new_mode: Option<u32>,
    pub status: GitFileStatus,
    pub hunks: Vec<GitDiffHunk>,
    pub is_binary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitDiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub lines: Vec<GitDiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitDiffLine {
    pub origin: char,
    pub content: String,
    pub old_line_number: Option<u32>,
    pub new_line_number: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommit {
    pub id: String,
    pub short_id: String,
    pub message: String,
    pub body: Option<String>,
    pub author: GitSignature,
    pub committer: GitSignature,
    pub time: String,
    pub parents: Vec<String>,
    pub tree_id: String,
    pub stats: GitCommitStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitStats {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitLogOptions {
    pub max_count: Option<usize>,
    pub skip: Option<usize>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub author: Option<String>,
    pub path: Option<String>,
    pub all_branches: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBlameLine {
    pub line_number: u32,
    pub commit_id: String,
    pub commit_short_id: String,
    pub author: GitSignature,
    pub content: String,
    pub final_commit_id: String,
    pub final_start_line_number: u32,
    pub orig_start_line_number: u32,
    pub orig_commit_id: String,
    pub boundary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBlame {
    pub file_path: String,
    pub lines: Vec<GitBlameLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitPushOptions {
    pub remote: String,
    pub branch: String,
    pub force: bool,
    pub set_upstream: bool,
    pub tags: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitPullOptions {
    pub remote: String,
    pub branch: String,
    pub rebase: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFetchOptions {
    pub remote: String,
    pub prune: bool,
    pub tags: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBranchOptions {
    pub name: String,
    pub start_point: Option<String>,
    pub checkout: bool,
    pub track: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitOptions {
    pub message: String,
    pub author: Option<GitSignature>,
    pub amend: bool,
    pub allow_empty: bool,
    pub signoff: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStashOptions {
    pub message: Option<String>,
    pub include_untracked: bool,
    pub keep_index: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStashEntry {
    pub index: usize,
    pub message: String,
    pub commit_id: String,
    pub branch: String,
    pub time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitSubmodule {
    pub name: String,
    pub path: String,
    pub url: String,
    pub branch: Option<String>,
    pub commit_id: Option<String>,
    pub is_initialized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitWorktree {
    pub path: String,
    pub branch: Option<String>,
    pub commit_id: String,
    pub is_bare: bool,
    pub is_detached: bool,
    pub is_locked: bool,
    pub prunable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitTag {
    pub name: String,
    pub commit_id: String,
    pub message: Option<String>,
    pub tagger: Option<GitSignature>,
    pub is_annotated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitMergeOptions {
    pub branch: String,
    pub strategy: Option<String>,
    pub no_ff: bool,
    pub squash: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRebaseOptions {
    pub upstream: String,
    pub onto: Option<String>,
    pub interactive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCherryPickOptions {
    pub commit_id: String,
    pub no_commit: bool,
    pub edit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitDiffOptions {
    pub context_lines: Option<u32>,
    pub ignore_whitespace: bool,
    pub ignore_whitespace_change: bool,
    pub ignore_whitespace_change_at_eol: bool,
    pub cached: Option<bool>,
}