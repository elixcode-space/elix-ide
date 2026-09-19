import React, { useState, useEffect, useCallback } from 'react';
import { tauriGitService, GitFileStatus } from '../services/tauriGitService';
import type { GitStatus, GitStatusFile, GitDiff, GitCommit, GitBranch, GitRepository, GitStashEntry, GitRemote, GitTag } from '../services/tauriGitService';
import './GitPanel.css';

interface GitPanelProps {
  repositoryPath: string;
  onClose?: () => void;
}

export const GitPanel: React.FC<GitPanelProps> = ({ repositoryPath, onClose }) => {
  const [repository, setRepository] = useState<GitRepository | null>(null);
  const [status, setStatus] = useState<GitStatus | null>(null);
  const [diff, setDiff] = useState<GitDiff | null>(null);
  const [commits, setCommits] = useState<GitCommit[]>([]);
  const [branches, setBranches] = useState<GitBranch[]>([]);
  const [stashes, setStashes] = useState<GitStashEntry[]>([]);
  const [remotes, setRemotes] = useState<GitRemote[]>([]);
  const [tags, setTags] = useState<GitTag[]>([]);
  const [selectedFile, setSelectedFile] = useState<GitStatusFile | null>(null);
  const [selectedCommit, setSelectedCommit] = useState<GitCommit | null>(null);
  const [activeTab, setActiveTab] = useState<'status' | 'history' | 'branches' | 'stashes' | 'remotes' | 'tags'>('status');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [commitMessage, setCommitMessage] = useState('');
  const [stagingFiles, setStagingFiles] = useState<Set<string>>(new Set());

  const loadRepository = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const repo = await tauriGitService.getRepositoryInfo(repositoryPath);
      setRepository(repo);
      await loadAll();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load repository');
    } finally {
      setLoading(false);
    }
  }, [repositoryPath]);

  const loadAll = async () => {
    try {
      await Promise.all([
        loadStatus(),
        loadBranches(),
        loadStashes(),
        loadRemotes(),
        loadTags(),
        loadCommits(),
      ]);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load data');
    }
  };

  const loadStatus = async () => {
    const s = await tauriGitService.status(repositoryPath);
    setStatus(s);
  };

  const loadBranches = async () => {
    if (repository) {
      setBranches(repository.branches);
    }
  };

  const loadStashes = async () => {
    const s = await tauriGitService.stashList(repositoryPath);
    setStashes(s);
  };

  const loadRemotes = async () => {
    if (repository) {
      setRemotes(repository.remotes);
    }
  };

  const loadTags = async () => {
    const t = await tauriGitService.tags(repositoryPath);
    setTags(t);
  };

  const loadCommits = async () => {
    const c = await tauriGitService.log(repositoryPath, { maxCount: 50, allBranches: true });
    setCommits(c);
  };

  const loadDiff = async (_filePath: string, staged: boolean) => {
    const d = await tauriGitService.diff(repositoryPath, { cached: staged } as any);
    setDiff(d);
  };

  const handleStageFile = async (file: GitStatusFile) => {
    try {
      await tauriGitService.add(repositoryPath, [file.path]);
      setStagingFiles(prev => new Set(prev).add(file.path));
      await loadStatus();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to stage file');
    }
  };

  const handleUnstageFile = async (file: GitStatusFile) => {
    try {
      // Reset the file
      await tauriGitService.checkout(repositoryPath, file.path);
      setStagingFiles(prev => {
        const next = new Set(prev);
        next.delete(file.path);
        return next;
      });
      await loadStatus();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to unstage file');
    }
  };

  const handleDiscardFile = async (file: GitStatusFile) => {
    if (!confirm(`Discard changes to ${file.path}?`)) return;
    try {
      await tauriGitService.checkout(repositoryPath, file.path);
      await loadStatus();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to discard changes');
    }
  };

  const handleCommit = async () => {
    if (!commitMessage.trim()) {
      setError('Commit message is required');
      return;
    }
    try {
      await tauriGitService.commit(repositoryPath, { message: commitMessage } as any);
      setCommitMessage('');
      await loadAll();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to commit');
    }
  };

  const handlePush = async () => {
    if (!repository?.head) return;
    const remote = remotes[0]?.name || 'origin';
    const branch = repository.head;
    try {
      await tauriGitService.push(repositoryPath, { remote, branch, force: false, setUpstream: true, tags: false });
      await loadAll();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to push');
    }
  };

  const handlePull = async () => {
    if (!repository?.head) return;
    const remote = remotes[0]?.name || 'origin';
    const branch = repository.head;
    try {
      await tauriGitService.pull(repositoryPath, { remote, branch, rebase: false });
      await loadAll();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to pull');
    }
  };

  const handleCheckoutBranch = async (branch: GitBranch) => {
    try {
      await tauriGitService.checkout(repositoryPath, branch.name);
      await loadAll();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to checkout branch');
    }
  };

  const handleCreateBranch = async () => {
    const name = prompt('New branch name:');
    if (!name) return;
    try {
      await tauriGitService.createBranch(repositoryPath, { name, checkout: true });
      await loadAll();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create branch');
    }
  };

  const handleDeleteBranch = async (branch: GitBranch) => {
    if (!confirm(`Delete branch ${branch.name}?`)) return;
    try {
      await tauriGitService.deleteBranch(repositoryPath, branch.name, false);
      await loadAll();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete branch');
    }
  };

  const handleStash = async () => {
    const message = prompt('Stash message (optional):');
    try {
      await tauriGitService.stash(repositoryPath, { message: message || undefined, includeUntracked: true, keepIndex: false } as any);
      await loadAll();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to stash');
    }
  };

  const handleStashPop = async (index: number) => {
    try {
      await tauriGitService.stashPop(repositoryPath, index);
      await loadAll();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to pop stash');
    }
  };

  const handleFileSelect = (file: GitStatusFile) => {
    setSelectedFile(file);
    if (stagingFiles.has(file.path)) {
      loadDiff(file.path, true);
    } else {
      loadDiff(file.path, false);
    }
  };

  const handleCommitSelect = (commit: GitCommit) => {
    setSelectedCommit(commit);
  };

  const _getFileStatusIcon = (file: GitStatusFile): string | null => null;

  const _getFileStatusLabel = (file: GitStatusFile): string | null => null;

  useEffect(() => {
    loadRepository();
  }, [loadRepository]);

  if (loading && !repository) {
    return <div className="git-panel loading">Loading repository...</div>;
  }

  if (error && !repository) {
    return (
      <div className="git-panel error">
        <p>Error: {error}</p>
        <button onClick={loadRepository}>Retry</button>
      </div>
    );
  }

  return (
    <div className="git-panel">
      <div className="git-header">
        <div className="repo-info">
          <h2>Source Control</h2>
          {repository && (
            <span className="repo-path">{repository.path}</span>
          )}
        </div>
        <div className="header-actions">
          <span className="branch-badge">
            {repository?.head || 'no branch'}
          </span>
          {repository?.head && (
            <>
              <span className="sync-status">
                {status && status.ahead > 0 && <span className="ahead">↑{status.ahead}</span>}
                {status && status.behind > 0 && <span className="behind">↓{status.behind}</span>}
              </span>
              <button className="btn-icon" onClick={handlePull} title="Pull" disabled={!remotes.length}>
                ⬇
              </button>
              <button className="btn-icon" onClick={handlePush} title="Push" disabled={!remotes.length}>
                ⬆
              </button>
            </>
          )}
          <button className="btn-icon" onClick={loadAll} title="Refresh">
            ⟳
          </button>
          {onClose && <button className="btn-icon" onClick={onClose} title="Close">✕</button>}
        </div>
      </div>

      <div className="git-tabs">
        <button className={`tab ${activeTab === 'status' ? 'active' : ''}`} onClick={() => setActiveTab('status')}>
          Changes {status && (status.files.length > 0 && ` (${status.files.length})`)}
        </button>
        <button className={`tab ${activeTab === 'history' ? 'active' : ''}`} onClick={() => setActiveTab('history')}>
          History
        </button>
        <button className={`tab ${activeTab === 'branches' ? 'active' : ''}`} onClick={() => setActiveTab('branches')}>
          Branches {branches.length > 0 && ` (${branches.length})`}
        </button>
        <button className={`tab ${activeTab === 'stashes' ? 'active' : ''}`} onClick={() => setActiveTab('stashes')}>
          Stashes {stashes.length > 0 && ` (${stashes.length})`}
        </button>
        <button className={`tab ${activeTab === 'remotes' ? 'active' : ''}`} onClick={() => setActiveTab('remotes')}>
          Remotes {remotes.length > 0 && ` (${remotes.length})`}
        </button>
        <button className={`tab ${activeTab === 'tags' ? 'active' : ''}`} onClick={() => setActiveTab('tags')}>
          Tags {tags.length > 0 && ` (${tags.length})`}
        </button>
      </div>

      <div className="git-content">
        {activeTab === 'status' && (
          <GitStatusView
            status={status}
            selectedFile={selectedFile}
            onFileSelect={handleFileSelect}
            onStageFile={handleStageFile}
            onUnstageFile={handleUnstageFile}
            onDiscardFile={handleDiscardFile}
            commitMessage={commitMessage}
            setCommitMessage={setCommitMessage}
            onCommit={handleCommit}
            stagingFiles={stagingFiles}
          />
        )}

        {activeTab === 'history' && (
          <GitHistoryView
            commits={commits}
            selectedCommit={selectedCommit}
            onCommitSelect={handleCommitSelect}
          />
        )}

        {activeTab === 'branches' && (
          <GitBranchesView
            branches={branches}
            currentBranch={repository?.head}
            onCheckout={handleCheckoutBranch}
            onCreate={handleCreateBranch}
            onDelete={handleDeleteBranch}
          />
        )}

        {activeTab === 'stashes' && (
          <GitStashesView
            stashes={stashes}
            onPop={handleStashPop}
            onCreate={handleStash}
          />
        )}

        {activeTab === 'remotes' && (
          <GitRemotesView
            remotes={remotes}
          />
        )}

        {activeTab === 'tags' && (
          <GitTagsView
            tags={tags}
          />
        )}
      </div>

      {(selectedFile || selectedCommit) && (
        <GitDiffView
          diff={diff}
          selectedFile={selectedFile}
          selectedCommit={selectedCommit}
          onClose={() => {
            setSelectedFile(null);
            setSelectedCommit(null);
            setDiff(null);
          }}
        />
      )}
    </div>
  );
};

// Status Tab
const GitStatusView: React.FC<{
  status: GitStatus | null;
  selectedFile: GitStatusFile | null;
  onFileSelect: (file: GitStatusFile) => void;
  onStageFile: (file: GitStatusFile) => void;
  onUnstageFile: (file: GitStatusFile) => void;
  commitMessage: string;
  setCommitMessage: (msg: string) => void;
  onCommit: () => void;
}> = ({ status, selectedFile, onFileSelect, onStageFile, onUnstageFile, commitMessage, setCommitMessage, onCommit }) => {
  if (!status) return <div className="empty-state">No repository loaded</div>;

  const stagedFiles = status.files.filter(f => 
    f.indexStatus !== GitFileStatus.Unmodified && f.indexStatus !== GitFileStatus.Untracked
  );
  const unstagedFiles = status.files.filter(f => 
    f.worktreeStatus !== GitFileStatus.Unmodified && 
    !(f.indexStatus !== GitFileStatus.Unmodified && f.indexStatus !== GitFileStatus.Untracked)
  );
  const untrackedFiles = status.files.filter(f => f.indexStatus === GitFileStatus.Untracked);

  return (
    <div className="git-status-view">
      <div className="commit-box">
        <textarea
          placeholder="Commit message..."
          value={commitMessage}
          onChange={e => setCommitMessage(e.target.value)}
          rows={3}
        />
        <button className="btn-primary" onClick={onCommit} disabled={!commitMessage.trim() || !stagedFiles.length}>
          Commit ({stagedFiles.length})
        </button>
      </div>

      {stagedFiles.length > 0 && (
        <FileGroup
          title="Staged Changes"
          files={stagedFiles}
          selectedFile={selectedFile}
          onFileSelect={onFileSelect}
          isStaged={true}
          onAction={onUnstageFile}
          actionLabel="Unstage"
        />
      )}

      {unstagedFiles.length > 0 && (
        <FileGroup
          title="Changes"
          files={unstagedFiles}
          selectedFile={selectedFile}
          onFileSelect={onFileSelect}
          isStaged={false}
          onAction={onUnstageFile}
          onDiscard={onDiscardFile}
          actionLabel="Unstage"
        />
      )}

      {untrackedFiles.length > 0 && (
        <FileGroup
          title="Untracked"
          files={untrackedFiles}
          selectedFile={selectedFile}
          onFileSelect={onFileSelect}
          isStaged={false}
          onAction={onStageFile}
          actionLabel="Stage"
        />
      )}

      {status.files.length === 0 && (
        <div className="empty-state">No changes. Working tree clean.</div>
      )}
    </div>
  );
};

export const FileGroup: React.FC<{
  title: string;
  files: GitStatusFile[];
  selectedFile: GitStatusFile | null;
  onFileSelect: (file: GitStatusFile) => void;
  isStaged: boolean;
  onAction: (file: GitStatusFile) => void;
  actionLabel: string;
  onDiscard?: (file: GitStatusFile) => void;
}> = ({ title, files, selectedFile, onFileSelect, isStaged, onAction, actionLabel, onDiscard }) => (
  <div className="file-group">
    <h4 className="file-group-title">{title} ({files.length})</h4>
    {files.map(file => (
      <div
        key={file.path}
        className={`file-item ${selectedFile?.path === file.path ? 'selected' : ''}`}
        onClick={() => onFileSelect(file)}
      >
        <span className="file-status-icon">{file.indexStatus === GitFileStatus.Conflict || file.worktreeStatus === GitFileStatus.Conflict ? '⚠️' : '🟢'}</span>
        <span className="file-path">{file.path}</span>
        <button className="btn-action" onClick={e => { e.stopPropagation(); onAction(file); }}>
          {actionLabel}
        </button>
        {!isStaged && file.worktreeStatus !== GitFileStatus.Untracked && (
          <button className="btn-action danger" onClick={e => { e.stopPropagation(); }}>
            Discard
          </button>
        )}
      </div>
    ))}
  </div>
);

// History Tab
const GitHistoryView: React.FC<{
  commits: GitCommit[];
  selectedCommit: GitCommit | null;
  onCommitSelect: (commit: GitCommit) => void;
}> = ({ commits, selectedCommit, onCommitSelect }) => (
  <div className="git-history-view">
    {commits.length === 0 ? (
      <div className="empty-state">No commits yet</div>
    ) : (
      <div className="commit-list">
        {commits.map(commit => (
          <div
            key={commit.id}
            className={`commit-item ${selectedCommit?.id === commit.id ? 'selected' : ''}`}
            onClick={() => onCommitSelect(commit)}
          >
            <div className="commit-header">
              <span className="commit-hash">{commit.shortId}</span>
              <span className="commit-time">{new Date(commit.time).toLocaleString()}</span>
            </div>
            <div className="commit-message">{commit.message.split('\n')[0]}</div>
            <div className="commit-author">{commit.author.name + ' <' + commit.author.email + '>'}</div>
          </div>
        ))}
      </div>
    )}
  </div>
);

// Branches Tab
const GitBranchesView: React.FC<{
  branches: GitBranch[];
  currentBranch?: string;
  onCheckout: (branch: GitBranch) => void;
  onCreate: () => void;
  onDelete: (branch: GitBranch) => void;
}> = ({ branches, currentBranch, onCheckout, onCreate, onDelete }) => (
  <div className="git-branches-view">
    <div className="branches-header">
      <h4>Branches</h4>
      <button className="btn-secondary" onClick={onCreate}>+ New Branch</button>
    </div>
    {branches.length === 0 ? (
      <div className="empty-state">No branches</div>
    ) : (
      <div className="branch-list">
        {branches.map(branch => (
          <div key={branch.name} className={`branch-item ${branch.name === currentBranch ? 'current' : ''}`}>
            <span className="branch-name">{branch.name} {branch.isCurrent && '●'}</span>
            <span className="branch-commit">{branch.commit.shortId} - {branch.commit.message.split('\n')[0]}</span>
            <div className="branch-actions">
              {!branch.isCurrent && (
                <button className="btn-action" onClick={e => { e.stopPropagation(); onCheckout(branch); }}>Checkout</button>
              )}
              {!branch.isCurrent && (
                <button className="btn-action danger" onClick={e => { e.stopPropagation(); onDelete(branch); }}>Delete</button>
              )}
            </div>
          </div>
        ))}
      </div>
    )}
  </div>
);

// Stashes Tab
const GitStashesView: React.FC<{
  stashes: GitStashEntry[];
  onPop: (index: number) => void;
  onCreate: () => void;
}> = ({ stashes, onPop, onCreate }) => (
  <div className="git-stashes-view">
    <div className="stashes-header">
      <h4>Stashes</h4>
      <button className="btn-secondary" onClick={onCreate}>+ New Stash</button>
    </div>
    {stashes.length === 0 ? (
      <div className="empty-state">No stashes</div>
    ) : (
      <div className="stash-list">
        {stashes.map((stash, index) => (
          <div key={index} className="stash-item">
            <div className="stash-info">
              <span className="stash-index">stash@{index}</span>
              <span className="stash-message">{stash.message || 'No message'}</span>
              <span className="stash-branch">{stash.branch}</span>
              <span className="stash-time">{new Date(stash.time).toLocaleString()}</span>
            </div>
            <button className="btn-action" onClick={() => onPop(index)}>Pop</button>
          </div>
        ))}
      </div>
    )}
  </div>
);

// Remotes Tab
const GitRemotesView: React.FC<{ remotes: GitRemote[] }> = ({ remotes }) => (
  <div className="git-remotes-view">
    {remotes.length === 0 ? (
      <div className="empty-state">No remotes configured</div>
    ) : (
      <div className="remote-list">
        {remotes.map(remote => (
          <div key={remote.name} className="remote-item">
            <h4>{remote.name}</h4>
            <p>Fetch: {remote.url}</p>
            <p>Push: {remote.url}</p>
          </div>
        ))}
      </div>
    )}
  </div>
);

// Tags Tab
const GitTagsView: React.FC<{ tags: GitTag[] }> = ({ tags }) => (
  <div className="git-tags-view">
    {tags.length === 0 ? (
      <div className="empty-state">No tags</div>
    ) : (
      <div className="tag-list">
        {tags.map(tag => (
          <div key={tag.name} className="tag-item">
            <span className="tag-name">{tag.name}</span>
            <span className="tag-commit">{tag.commitId.substring(0, 8)}</span>
            {tag.message && <span className="tag-message">{tag.message}</span>}
          </div>
        ))}
      </div>
    )}
  </div>
);

// Diff View
const GitDiffView: React.FC<{
  diff: GitDiff | null;
  selectedFile: GitStatusFile | null;
  selectedCommit: GitCommit | null;
  onClose: () => void;
}> = ({ diff, selectedFile, selectedCommit, onClose }) => {
  if (!diff) return null;

  const fileDiff = selectedFile ? diff.files.find(f => f.newPath === selectedFile.path || f.oldPath === selectedFile.path) : null;

  return (
    <div className="git-diff-view">
      <div className="diff-header">
        <h4>
          {selectedFile ? `Changes in ${selectedFile.path}` : selectedCommit ? `Commit ${selectedCommit.shortId}` : 'Diff'}
        </h4>
        <button className="btn-icon" onClick={onClose}>✕</button>
      </div>
      <div className="diff-content">
        {fileDiff ? (
          <pre className="diff-text">
            {fileDiff.hunks.map(hunk => (
              <React.Fragment key={hunk.header}>
                <div className="diff-hunk-header">{hunk.header}</div>
                {hunk.lines.map((line, i) => (
                  <div key={i} className={`diff-line ${line.origin === '+' ? 'added' : line.origin === '-' ? 'removed' : 'context'}`}>
                    <span className="line-origin">{line.origin}</span>
                    <span className="line-content">{line.content}</span>
                  </div>
                ))}
              </React.Fragment>
            ))}
          </pre>
        ) : (
          <div className="empty-state">No diff available</div>
        )}
      </div>
    </div>
  );
};

export default GitPanel;