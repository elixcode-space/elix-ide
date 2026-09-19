/**
 * Tauri Git Service
 *
 * Provides a TypeScript wrapper around the Tauri Git commands
 * for use in the frontend.
 */

import { invoke } from '@tauri-apps/api/core';

// Type definitions matching Rust Git types
export interface GitRepository {
  path: string;
  isBare: boolean;
  head?: string;
  branches: GitBranch[];
  remotes: GitRemote[];
  worktreePath?: string;
}

export interface GitBranch {
  name: string;
  isCurrent: boolean;
  isRemote: boolean;
  upstream?: string;
  commit: GitCommitRef;
}

export interface GitRemote {
  name: string;
  url: string;
  fetchRefspec: string;
}

export interface GitCommitRef {
  id: string;
  shortId: string;
  message: string;
  author: GitSignature;
  committer: GitSignature;
  time: string;
  parents: string[];
}

export interface GitSignature {
  name: string;
  email: string;
  time: string;
}

export interface GitStatus {
  repositoryPath: string;
  currentBranch?: string;
  ahead: number;
  behind: number;
  files: GitStatusFile[];
  stashCount: number;
}

export interface GitStatusFile {
  path: string;
  indexStatus: GitFileStatus;
  worktreeStatus: GitFileStatus;
  oldPath?: string;
}

export enum GitFileStatus {
  Unmodified = 'Unmodified',
  Added = 'Added',
  Deleted = 'Deleted',
  Modified = 'Modified',
  Renamed = 'Renamed',
  Copied = 'Copied',
  Untracked = 'Untracked',
  Ignored = 'Ignored',
  TypeChange = 'TypeChange',
  Conflict = 'Conflict',
}

export interface GitDiff {
  repositoryPath: string;
  files: GitDiffFile[];
}

export interface GitDiffFile {
  oldPath?: string;
  newPath?: string;
  oldMode?: number;
  newMode?: number;
  status: GitFileStatus;
  hunks: GitDiffHunk[];
  isBinary: boolean;
}

export interface GitDiffHunk {
  oldStart: number;
  oldLines: number;
  newStart: number;
  newLines: number;
  header: string;
  lines: GitDiffLine[];
}

export interface GitDiffLine {
  origin: string;
  content: string;
  oldLineNumber?: number;
  newLineNumber?: number;
}

export interface GitCommit {
  id: string;
  shortId: string;
  message: string;
  body?: string;
  author: GitSignature;
  committer: GitSignature;
  time: string;
  parents: string[];
  treeId: string;
  stats: GitCommitStats;
}

export interface GitCommitStats {
  filesChanged: number;
  insertions: number;
  deletions: number;
}

export interface GitLogOptions {
  maxCount?: number;
  skip?: number;
  since?: string;
  until?: string;
  author?: string;
  path?: string;
  allBranches: boolean;
}

export interface GitBlame {
  filePath: string;
  lines: GitBlameLine[];
}

export interface GitBlameLine {
  lineNumber: number;
  commitId: string;
  commitShortId: string;
  author: GitSignature;
  content: string;
  finalCommitId: string;
  finalStartLineNumber: number;
  origStartLineNumber: number;
  origCommitId: string;
  boundary: boolean;
}

export interface GitPushOptions {
  remote: string;
  branch: string;
  force: boolean;
  setUpstream: boolean;
  tags: boolean;
}

export interface GitPullOptions {
  remote: string;
  branch: string;
  rebase: boolean;
}

export interface GitFetchOptions {
  remote: string;
  prune: boolean;
  tags: boolean;
}

export interface GitBranchOptions {
  name: string;
  startPoint?: string;
  checkout: boolean;
  track?: string;
}

export interface GitCommitOptions {
  message: string;
  author?: GitSignature;
  amend: boolean;
  allowEmpty: boolean;
  signoff: boolean;
}

export interface GitStashOptions {
  message?: string;
  includeUntracked: boolean;
  keepIndex: boolean;
}

export interface GitStashEntry {
  index: number;
  message: string;
  commitId: string;
  branch: string;
  time: string;
}

export interface GitSubmodule {
  name: string;
  path: string;
  url: string;
  branch?: string;
  commitId?: string;
  isInitialized: boolean;
}

export interface GitWorktree {
  path: string;
  branch?: string;
  commitId: string;
  isBare: boolean;
  isDetached: boolean;
  isLocked: boolean;
  prunable: boolean;
}

export interface GitTag {
  name: string;
  commitId: string;
  message?: string;
  tagger?: GitSignature;
  isAnnotated: boolean;
}

export interface GitMergeOptions {
  branch: string;
  strategy?: string;
  noFf: boolean;
  squash: boolean;
  message?: string;
}

export interface GitDiffOptions {
  contextLines?: number;
  ignoreWhitespace: boolean;
  ignoreWhitespaceChange: boolean;
  ignoreWhitespaceChangeAtEol: boolean;
  cached?: boolean;
}

/**
 * Git service class providing access to Tauri Git commands
 */
export class TauriGitService {
  /**
   * Initialize a new Git repository
   */
  static async init(path: string, bare: boolean = false): Promise<GitRepository> {
    return invoke('git_init', { path, bare });
  }

  /**
   * Open an existing Git repository
   */
  static async open(path: string): Promise<GitRepository> {
    return invoke('git_open', { path });
  }

  /**
   * Get repository information
   */
  static async getRepositoryInfo(path: string): Promise<GitRepository> {
    return invoke('git_get_repository_info', { path });
  }

  /**
   * Get repository status (working tree changes)
   */
  static async status(path: string): Promise<GitStatus> {
    return invoke('git_status', { path });
  }

  /**
   * Get diff for the repository
   */
  static async diff(path: string, options?: GitDiffOptions): Promise<GitDiff> {
    return invoke('git_diff', { path, options });
  }

  /**
   * Get commit history
   */
  static async log(path: string, options: GitLogOptions): Promise<GitCommit[]> {
    return invoke('git_log', { path, options });
  }

  /**
   * Get blame for a file
   */
  static async blame(path: string, filePath: string): Promise<GitBlame> {
    return invoke('git_blame', { path, filePath });
  }

  /**
   * Stage files
   */
  static async add(path: string, files: string[]): Promise<void> {
    return invoke('git_add', { path, files });
  }

  /**
   * Create a commit
   */
  static async commit(path: string, options: GitCommitOptions): Promise<string> {
    return invoke('git_commit', { path, options });
  }

  /**
   * Push to remote
   */
  static async push(path: string, options: GitPushOptions): Promise<void> {
    return invoke('git_push', { path, options });
  }

  /**
   * Pull from remote
   */
  static async pull(path: string, options: GitPullOptions): Promise<void> {
    return invoke('git_pull', { path, options });
  }

  /**
   * Fetch from remote
   */
  static async fetch(path: string, options: GitFetchOptions): Promise<void> {
    return invoke('git_fetch', { path, options });
  }

  /**
   * Create a new branch
   */
  static async createBranch(path: string, options: GitBranchOptions): Promise<void> {
    return invoke('git_create_branch', { path, options });
  }

  /**
   * Delete a branch
   */
  static async deleteBranch(path: string, name: string, force: boolean = false): Promise<void> {
    return invoke('git_delete_branch', { path, name, force });
  }

  /**
   * Checkout a branch or commit
   */
  static async checkout(path: string, target: string): Promise<void> {
    return invoke('git_checkout', { path, target });
  }

  /**
   * Stash changes
   */
  static async stash(path: string, options: GitStashOptions): Promise<void> {
    return invoke('git_stash', { path, options });
  }

  /**
   * List stashes
   */
  static async stashList(path: string): Promise<GitStashEntry[]> {
    return invoke('git_stash_list', { path });
  }

  /**
   * Pop stash
   */
  static async stashPop(path: string, index?: number): Promise<void> {
    return invoke('git_stash_pop', { path, index });
  }

  /**
   * Get submodules
   */
  static async submodules(path: string): Promise<GitSubmodule[]> {
    return invoke('git_submodules', { path });
  }

  /**
   * Get worktrees
   */
  static async worktrees(path: string): Promise<GitWorktree[]> {
    return invoke('git_worktrees', { path });
  }

  /**
   * Get tags
   */
  static async tags(path: string): Promise<GitTag[]> {
    return invoke('git_tags', { path });
  }

  /**
   * Merge a branch
   */
  static async merge(path: string, options: GitMergeOptions): Promise<void> {
    return invoke('git_merge', { path, options });
  }
}

export type { GitRepository, GitBranch, GitRemote, GitCommitRef, GitSignature, GitStatus, GitStatusFile, GitFileStatus, GitDiff, GitDiffFile, GitDiffHunk, GitDiffLine, GitCommit, GitCommitStats, GitLogOptions, GitBlame, GitBlameLine, GitPushOptions, GitPullOptions, GitFetchOptions, GitBranchOptions, GitCommitOptions, GitStashOptions, GitStashEntry, GitSubmodule, GitWorktree, GitTag, GitMergeOptions, GitDiffOptions };

// Export a singleton instance
export const tauriGitService = TauriGitService;

// Also expose on window for debugging
if (typeof window !== 'undefined') {
  (window as any).__TAURI_GIT_SERVICE__ = tauriGitService;
}