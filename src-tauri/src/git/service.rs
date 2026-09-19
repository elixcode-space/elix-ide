use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use git2::{
    BranchType, Cred, CredentialType, Direction, FetchOptions, IndexAddOption, MergeOptions,
    ProxyOptions, PushOptions, RemoteCallbacks, Repository, RepositoryInitOptions, ResetType,
    Signature, StatusOptions, SubmoduleUpdateOptions,
};

use crate::git::types::*;

#[derive(thiserror::Error, Debug)]
pub enum GitError {
    #[error("Repository not found: {0}")]
    RepoNotFound(String),
    #[error("Git2 error: {0}")]
    Git2(#[from] git2::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("Authentication failed: {0}")]
    AuthFailed(String),
    #[error("Merge conflict: {0}")]
    MergeConflict(String),
}

pub type Result<T> = std::result::Result<T, GitError>;

pub struct GitService {
}

impl GitService {
    pub fn new() -> Self {
        Self {}
    }

    fn open_repo(&self, path: &str) -> Result<Repository> {
        Repository::open(path).map_err(GitError::from)
    }

    fn get_repo_path(&self, path: &str) -> Result<PathBuf> {
        let path = Path::new(path);
        if !path.exists() {
            return Err(GitError::InvalidPath(format!("Path does not exist: {}", path.display())));
        }
        Ok(path.to_path_buf())
    }

    pub async fn init(&self, path: &str, bare: bool) -> Result<GitRepository> {
        let path_buf = self.get_repo_path(path)?;
        let mut opts = RepositoryInitOptions::new();
        opts.bare(bare);
        let repo = Repository::init_opts(&path_buf, &opts)?;
        self.get_repository_info(path).await
    }

    pub async fn open(&self, path: &str) -> Result<GitRepository> {
        let _repo = self.open_repo(path)?;
        self.get_repository_info(path).await
    }

    pub async fn get_repository_info(&self, path: &str) -> Result<GitRepository> {
        let repo = self.open_repo(path)?;
        let workdir = repo.workdir().map(|p| p.to_string_lossy().to_string());
        let is_bare = repo.is_bare();
        let head = repo.head().ok().and_then(|h| h.shorthand().map(|s| s.to_string()));

        let branches = self.get_branches(&repo)?;
        let remotes = self.get_remotes(&repo)?;

        Ok(GitRepository {
            path: path.to_string(),
            is_bare,
            head,
            branches,
            remotes,
            worktree_path: workdir,
        })
    }

    fn get_branches(&self, repo: &Repository) -> Result<Vec<GitBranch>> {
        let mut branches = Vec::new();
        let branch_iter = repo.branches(Some(BranchType::Local))?;

        for branch in branch_iter {
            let (branch, _) = branch?;
            let name = branch.name()?.unwrap_or_default().to_string();
            let is_current = branch.is_head();
            let is_remote = false;

            let upstream = branch.upstream().ok().and_then(|u| u.name().ok().flatten().map(|s| s.to_string()));
            let commit = branch.get().peel_to_commit().ok().and_then(|c| self.get_commit_ref(&c).ok());

            branches.push(GitBranch {
                name,
                is_current,
                is_remote,
                upstream,
                commit: commit.unwrap_or_default(),
            });
        }

        let remote_branches = repo.branches(Some(BranchType::Remote))?;
        for branch in remote_branches {
            let (branch, _) = branch?;
            let name = branch.name()?.unwrap_or_default().to_string();
            let is_current = false;
            let is_remote = true;

            let upstream = branch.upstream().ok().and_then(|u| u.name().ok().flatten().map(|s| s.to_string()));
            let commit = branch.get().peel_to_commit().ok().and_then(|c| self.get_commit_ref(&c).ok());

            branches.push(GitBranch {
                name,
                is_current,
                is_remote,
                upstream,
                commit: commit.unwrap_or_default(),
            });
        }

        Ok(branches)
    }

    fn get_remotes(&self, repo: &Repository) -> Result<Vec<GitRemote>> {
        let mut remotes = Vec::new();
        for remote_name in repo.remotes()?.iter().flatten() {
            let remote = repo.find_remote(remote_name)?;
            let fetch_refspec = remote.fetch_refspecs()
                .ok()
                .and_then(|r| r.get(0).map(|s| s.to_string()))
                .unwrap_or_default();
            remotes.push(GitRemote {
                name: remote_name.to_string(),
                url: remote.url().unwrap_or_default().to_string(),
                fetch_refspec,
            });
        }
        Ok(remotes)
    }

    fn get_commit_ref(&self, commit: &git2::Commit) -> Result<GitCommitRef> {
        let id = commit.id().to_string();
        let short_id = format!("{:.7}", commit.id());
        let message = commit.message().unwrap_or_default().to_string();
        let author = self.signature_to_git_signature(&commit.author());
        let committer = self.signature_to_git_signature(&commit.committer());
        let time = commit.time().seconds().to_string();
        let parents = commit.parent_ids().map(|id| id.to_string()).collect();

        Ok(GitCommitRef {
            id,
            short_id,
            message,
            author,
            committer,
            time,
            parents,
        })
    }

    fn signature_to_git_signature(&self, sig: &Signature) -> GitSignature {
        GitSignature {
            name: sig.name().unwrap_or_default().to_string(),
            email: sig.email().unwrap_or_default().to_string(),
            time: sig.when().seconds().to_string(),
        }
    }

    pub async fn status(&self, path: &str) -> Result<GitStatus> {
        let repo = self.open_repo(path)?;
        let mut opts = StatusOptions::new();
        opts.include_untracked(true).include_ignored(false).recurse_untracked_dirs(true);

        let statuses = repo.statuses(Some(&mut opts))?;
        let head = repo.head().ok();
        let current_branch = head.as_ref().and_then(|h| h.shorthand()).map(|s| s.to_string());

        // Simplified - ahead/behind calculation requires upstream branch info
        let (ahead, behind) = (0, 0);

        let mut files = Vec::new();
        for entry in statuses.iter() {
            let path = entry.path().unwrap_or_default().to_string();
            let index_status = self.status_to_git_status(entry.status(), true);
            let worktree_status = self.status_to_git_status(entry.status(), false);

            files.push(GitStatusFile {
                path,
                index_status,
                worktree_status,
                old_path: None,
            });
        }

        let stash_count = 0;

        Ok(GitStatus {
            repository_path: path.to_string(),
            current_branch,
            ahead,
            behind,
            files,
            stash_count,
        })
    }

    fn status_to_git_status(&self, status: git2::Status, is_index: bool) -> GitFileStatus {
        let flags = if is_index {
            git2::Status::INDEX_NEW
                | git2::Status::INDEX_MODIFIED
                | git2::Status::INDEX_DELETED
                | git2::Status::INDEX_RENAMED
                | git2::Status::INDEX_TYPECHANGE
        } else {
            git2::Status::WT_NEW
                | git2::Status::WT_MODIFIED
                | git2::Status::WT_DELETED
                | git2::Status::WT_RENAMED
                | git2::Status::WT_TYPECHANGE
        };

        let s = status & flags;
        if s.contains(git2::Status::INDEX_NEW) || s.contains(git2::Status::WT_NEW) {
            GitFileStatus::Added
        } else if s.contains(git2::Status::INDEX_DELETED) || s.contains(git2::Status::WT_DELETED) {
            GitFileStatus::Deleted
        } else if s.contains(git2::Status::INDEX_MODIFIED) || s.contains(git2::Status::WT_MODIFIED) {
            GitFileStatus::Modified
        } else if s.contains(git2::Status::INDEX_RENAMED) || s.contains(git2::Status::WT_RENAMED) {
            GitFileStatus::Renamed
        } else if s.contains(git2::Status::INDEX_TYPECHANGE) || s.contains(git2::Status::WT_TYPECHANGE) {
            GitFileStatus::TypeChange
        } else if status.contains(git2::Status::CONFLICTED) {
            GitFileStatus::Conflict
        } else if status.contains(git2::Status::IGNORED) {
            GitFileStatus::Ignored
        } else {
            GitFileStatus::Unmodified
        }
    }

    pub async fn diff(&self, path: &str, options: Option<GitDiffOptions>) -> Result<GitDiff> {
        let repo = self.open_repo(path)?;
        let mut diff_opts = git2::DiffOptions::new();
        if let Some(opts) = options.as_ref() {
            if let Some(context_lines) = opts.context_lines {
                diff_opts.context_lines(context_lines);
            }
            if opts.ignore_whitespace {
                diff_opts.ignore_whitespace(true);
            }
            if opts.ignore_whitespace_change {
                diff_opts.ignore_whitespace_change(true);
            }
        }

        let diff = if let Some(cached) = options.as_ref().and_then(|o| o.cached) {
            if cached {
                repo.diff_index_to_workdir(None, Some(&mut diff_opts))?
            } else {
                let head = repo.head()?.peel_to_tree()?;
                repo.diff_tree_to_workdir_with_index(Some(&head), Some(&mut diff_opts))?
            }
        } else {
            let head = repo.head()?.peel_to_tree()?;
            repo.diff_tree_to_workdir_with_index(Some(&head), Some(&mut diff_opts))?
        };

        let mut files = Vec::new();
        diff.foreach(
            &mut |delta, _| {
                let old_path = delta.old_file().path().map(|p| p.to_string_lossy().to_string());
                let new_path = delta.new_file().path().map(|p| p.to_string_lossy().to_string());
                let status = self.delta_status_to_git_status(delta.status());

                files.push(GitDiffFile {
                    old_path,
                    new_path,
                    old_mode: Some(0),
                    new_mode: Some(0),
                    status,
                    hunks: Vec::new(),
                    is_binary: delta.old_file().is_binary() || delta.new_file().is_binary(),
                });
                true
            },
            None,
            None,
            None,
        )?;

        for (i, file) in files.iter_mut().enumerate() {
            let mut hunks = Vec::new();
            diff.foreach(
                &mut |_, _| true,
                None,
                Some(&mut |_delta, hunk| {
                    if i == 0 {
                        hunks.push(GitDiffHunk {
                            old_start: hunk.old_start(),
                            old_lines: hunk.old_lines(),
                            new_start: hunk.new_start(),
                            new_lines: hunk.new_lines(),
                            header: String::from_utf8_lossy(hunk.header()).to_string(),
                            lines: Vec::new(),
                        });
                    }
                    true
                }),
                None,
            )?;
            file.hunks = hunks;
        }

        Ok(GitDiff {
            repository_path: path.to_string(),
            files,
        })
    }

    fn delta_status_to_git_status(&self, status: git2::Delta) -> GitFileStatus {
        match status {
            git2::Delta::Added => GitFileStatus::Added,
            git2::Delta::Deleted => GitFileStatus::Deleted,
            git2::Delta::Modified => GitFileStatus::Modified,
            git2::Delta::Renamed => GitFileStatus::Renamed,
            git2::Delta::Copied => GitFileStatus::Copied,
            git2::Delta::Typechange => GitFileStatus::TypeChange,
            _ => GitFileStatus::Unmodified,
        }
    }

    pub async fn log(&self, path: &str, options: GitLogOptions) -> Result<Vec<GitCommit>> {
        let repo = self.open_repo(path)?;
        let mut revwalk = repo.revwalk()?;

        if options.all_branches {
            revwalk.push_glob("refs/heads/*")?;
            revwalk.push_glob("refs/remotes/*")?;
        } else if let Some(branch) = &options.path {
            revwalk.push_ref(&format!("refs/heads/{}", branch))?;
        } else {
            revwalk.push_head()?;
        }

        if let Some(since) = options.since {
            let time = git2::Time::new(since.parse::<i64>().unwrap_or(0), 0);
            revwalk.set_sorting(git2::Sort::TIME)?;
            revwalk.simplify_first_parent()?;
        }

        if let Some(max_count) = options.max_count {
            // revwalk doesn't have a direct limit, we'll limit in the loop
        }

        let mut commits = Vec::new();
        let mut count = 0;
        let skip = options.skip.unwrap_or(0);

        for oid in revwalk {
            let oid = oid?;
            if count < skip {
                count += 1;
                continue;
            }
            if let Some(max) = options.max_count {
                if commits.len() >= max {
                    break;
                }
            }

            let commit = repo.find_commit(oid)?;
            let git_commit = self.commit_to_git_commit(&repo, &commit)?;
            
            if let Some(author) = &options.author {
                if !git_commit.author.name.contains(author) && !git_commit.author.email.contains(author) {
                    continue;
                }
            }

            if let Some(path_filter) = &options.path {
                let tree = commit.tree()?;
                let mut found = false;
                tree.walk(git2::TreeWalkMode::PreOrder, |_, entry| {
                    if entry.name().unwrap_or_default().contains(path_filter) {
                        found = true;
                    }
                    git2::TreeWalkResult::Ok
                })?;
                if !found {
                    continue;
                }
            }

            commits.push(git_commit);
            count += 1;
        }

        Ok(commits)
    }

    fn commit_to_git_commit(&self, repo: &Repository, commit: &git2::Commit) -> Result<GitCommit> {
        let id = commit.id().to_string();
        let short_id = format!("{:.7}", commit.id());
        let message = commit.summary().unwrap_or_default().to_string();
        let body = commit.body().map(|s| s.to_string());
        let author = self.signature_to_git_signature(&commit.author());
        let committer = self.signature_to_git_signature(&commit.committer());
        let time = commit.time().seconds().to_string();
        let parents = commit.parent_ids().map(|id| id.to_string()).collect();
        let tree_id = commit.tree_id().to_string();

        let mut stats = GitCommitStats {
            files_changed: 0,
            insertions: 0,
            deletions: 0,
        };

        if commit.parent_count() > 0 {
            let parent = commit.parent(0)?;
            let diff = repo.diff_tree_to_tree(Some(&parent.tree()?), Some(&commit.tree()?), None)?;
            if let Ok(diff_stats) = diff.stats() {
                stats.files_changed = diff_stats.files_changed() as usize;
                stats.insertions = diff_stats.insertions() as usize;
                stats.deletions = diff_stats.deletions() as usize;
            }
        }

        Ok(GitCommit {
            id,
            short_id,
            message,
            body,
            author,
            committer,
            time,
            parents,
            tree_id,
            stats,
        })
    }

    pub async fn blame(&self, path: &str, file_path: &str) -> Result<GitBlame> {
        let _repo = self.open_repo(path)?;
        // Simplified - just return empty blame for now
        Ok(GitBlame {
            file_path: file_path.to_string(),
            lines: Vec::new(),
        })
    }

    pub async fn add(&self, path: &str, files: Vec<String>) -> Result<()> {
        let repo = self.open_repo(path)?;
        let mut index = repo.index()?;
        for file in files {
            index.add_path(Path::new(&file))?;
        }
        index.write()?;
        Ok(())
    }

    pub async fn commit(&self, path: &str, options: GitCommitOptions) -> Result<String> {
        let repo = self.open_repo(path)?;
        let signature = if let Some(author) = options.author {
            Signature::new(&author.name, &author.email, &git2::Time::new(chrono::Utc::now().timestamp(), 0))?
        } else {
            repo.signature()?
        };

        let mut index = repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        let parent_commits: Vec<git2::Commit> = if options.amend {
            let head = repo.head()?.peel_to_commit()?;
            if head.parent_count() > 0 {
                vec![head.parent(0)?]
            } else {
                vec![]
            }
        } else {
            let head = repo.head().ok();
            if let Some(head) = head {
                vec![head.peel_to_commit()?]
            } else {
                vec![]
            }
        };

        let parent_refs: Vec<&git2::Commit> = parent_commits.iter().collect();

        let commit_id = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &options.message,
            &tree,
            &parent_refs,
        )?;

        Ok(commit_id.to_string())
    }

    pub async fn push(&self, path: &str, options: GitPushOptions) -> Result<()> {
        let repo = self.open_repo(path)?;
        let mut remote = repo.find_remote(&options.remote)?;
        let mut callbacks = RemoteCallbacks::new();

        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
        });

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);

        let refspec = if options.set_upstream {
            format!("refs/heads/{}:refs/heads/{}", options.branch, options.branch)
        } else {
            format!("refs/heads/{}", options.branch)
        };

        remote.push(&[&refspec], Some(&mut push_options))?;
        Ok(())
    }

    pub async fn pull(&self, path: &str, options: GitPullOptions) -> Result<()> {
        let repo = self.open_repo(path)?;
        let mut remote = repo.find_remote(&options.remote)?;
        let mut callbacks = RemoteCallbacks::new();

        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        remote.fetch(&[&format!("refs/heads/{}", options.branch)], Some(&mut fetch_options), None)?;

        let fetch_head = repo.find_reference("FETCH_HEAD")?.peel_to_commit()?;
        let head = repo.head()?.peel_to_commit()?;

        let mut merge_options = MergeOptions::new();
        if options.rebase {
            // For rebase, we'd need more complex logic
            repo.reset(fetch_head.as_object(), ResetType::Hard, None)?;
        } else {
            let _ = repo.merge_commits(&head, &fetch_head, Some(&mut merge_options));
        }

        Ok(())
    }

    pub async fn fetch(&self, path: &str, options: GitFetchOptions) -> Result<()> {
        let repo = self.open_repo(path)?;
        let mut remote = repo.find_remote(&options.remote)?;
        let mut callbacks = RemoteCallbacks::new();

        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);
        fetch_options.prune(git2::FetchPrune::On);

        remote.fetch(&["refs/heads/*:refs/remotes/*"], Some(&mut fetch_options), None)?;
        Ok(())
    }

    pub async fn create_branch(&self, path: &str, options: GitBranchOptions) -> Result<()> {
        let repo = self.open_repo(path)?;
        let start_point = options.start_point.unwrap_or_else(|| "HEAD".to_string());
        let obj = repo.revparse_single(&start_point)?;
        repo.branch(&options.name, &obj.peel_to_commit()?, false)?;
        
        if options.checkout {
            let branch_ref = format!("refs/heads/{}", options.name);
            repo.set_head(&branch_ref)?;
            repo.checkout_head(None)?;
        }
        Ok(())
    }

    pub async fn delete_branch(&self, path: &str, name: &str, force: bool) -> Result<()> {
        let repo = self.open_repo(path)?;
        let mut branch = repo.find_branch(name, BranchType::Local)?;
        if force {
            branch.delete()?;
        } else {
            branch.delete()?;
        }
        Ok(())
    }

    pub async fn checkout(&self, path: &str, target: &str) -> Result<()> {
        let repo = self.open_repo(path)?;
        let obj = repo.revparse_single(target)?;
        repo.checkout_tree(&obj, None)?;
        repo.set_head(target)?;
        Ok(())
    }

    pub async fn stash(&self, path: &str, options: GitStashOptions) -> Result<()> {
        let mut repo = self.open_repo(path)?;
        let signature = repo.signature()?;
        repo.stash_save(&signature, options.message.as_deref().unwrap_or(""), None)?;
        Ok(())
    }

    pub async fn stash_list(&self, path: &str) -> Result<Vec<GitStashEntry>> {
        let mut repo = self.open_repo(path)?;
        let mut entries = Vec::new();
        repo.stash_foreach(|index, message, commit_id| {
            entries.push(GitStashEntry {
                index,
                message: message.to_string(),
                commit_id: commit_id.to_string(),
                branch: "".to_string(), // Would need more logic to get branch
                time: "".to_string(),
            });
            true
        })?;
        Ok(entries)
    }

    pub async fn stash_pop(&self, path: &str, index: Option<usize>) -> Result<()> {
        let mut repo = self.open_repo(path)?;
        let idx = index.unwrap_or(0);
        repo.stash_pop(idx, None)?;
        Ok(())
    }

    pub async fn submodules(&self, path: &str) -> Result<Vec<GitSubmodule>> {
        let repo = self.open_repo(path)?;
        let mut submodules = Vec::new();
        for submodule in repo.submodules()?.iter() {
            submodules.push(GitSubmodule {
                name: submodule.name().unwrap_or_default().to_string(),
                path: submodule.path().to_string_lossy().to_string(),
                url: submodule.url().unwrap_or_default().to_string(),
                branch: submodule.branch().map(|s| s.to_string()),
                commit_id: None,
                is_initialized: false,
            });
        }
        Ok(submodules)
    }

    pub async fn worktrees(&self, path: &str) -> Result<Vec<GitWorktree>> {
        let repo = self.open_repo(path)?;
        let mut worktrees = Vec::new();
        let worktree_list = repo.worktrees()?;
        for i in 0..worktree_list.len() {
            if let Some(wt_path) = worktree_list.get(i) {
                worktrees.push(GitWorktree {
                    path: wt_path.to_string(),
                    branch: None,
                    commit_id: "".to_string(),
                    is_bare: false,
                    is_detached: false,
                    is_locked: false,
                    prunable: false,
                });
            }
        }
        Ok(worktrees)
    }

    pub async fn tags(&self, path: &str) -> Result<Vec<GitTag>> {
        let repo = self.open_repo(path)?;
        let mut tags = Vec::new();
        repo.tag_foreach(|oid, name| {
            let tag_name = String::from_utf8_lossy(name).to_string();
            let obj = repo.find_object(oid, None).ok();
            let commit_id = obj.as_ref().map(|o| o.id().to_string()).unwrap_or_default();
            let (message, tagger, is_annotated) = if let Some(tag) = obj.as_ref().and_then(|o| o.as_tag()) {
                (tag.message().map(|s| s.to_string()), tag.tagger().map(|t| self.signature_to_git_signature(&t)), true)
            } else {
                (None, None, false)
            };
            tags.push(GitTag {
                name: tag_name,
                commit_id,
                message,
                tagger,
                is_annotated,
            });
            true
        })?;
        Ok(tags)
    }

    pub async fn merge(&self, path: &str, options: GitMergeOptions) -> Result<()> {
        let repo = self.open_repo(path)?;
        let branch_ref = format!("refs/heads/{}", options.branch);
        let branch_commit = repo.find_reference(&branch_ref)?.peel_to_commit()?;
        let head_commit = repo.head()?.peel_to_commit()?;

        let mut merge_options = MergeOptions::new();
        if let Some(strategy) = options.strategy {
            // merge_options.merge_strategy(strategy); // Not directly supported
        }

        // Use merge_commits instead for simpler API
        let _ = repo.merge_commits(&head_commit, &branch_commit, Some(&mut merge_options));

        if options.squash {
            let mut index = repo.index()?;
            let tree_id = index.write_tree_to(&repo)?;
            let tree = repo.find_tree(tree_id)?;
            let signature = repo.signature()?;
            let msg = options.message.unwrap_or_else(|| format!("Merge branch '{}'", options.branch));
            repo.commit(Some("HEAD"), &signature, &signature, &msg, &tree, &[&head_commit])?;
        }
        Ok(())
    }
}