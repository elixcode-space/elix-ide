import React, { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import './FileExplorer.css';

interface FileTreeNode {
  path: string;
  name: string;
  type: 'file' | 'directory';
  size?: number;
  modified?: number;
  children?: FileTreeNode[];
  expanded?: boolean;
}

interface FileExplorerProps {
  workspacePath: string;
  onFileOpen?: (path: string) => void;
}

export const FileExplorer: React.FC<FileExplorerProps> = ({ workspacePath, onFileOpen }) => {
  const [tree, setTree] = useState<FileTreeNode | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; path: string } | null>(null);
  const [renamingPath, setRenamingPath] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState('');
  const [showHidden, setShowHidden] = useState(false);
  const treeRef = useRef<HTMLDivElement>(null);

  const loadTree = useCallback(async () => {
    if (!workspacePath) return;
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<FileTreeNode>('get_file_tree', {
        options: {
          root: workspacePath,
          max_depth: 10,
          include_hidden: showHidden,
          exclude_patterns: ['node_modules', '.git', 'target', 'dist', 'build', '.next', '.cache', '*.log'],
        },
      });
      setTree(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load file tree');
    } finally {
      setLoading(false);
    }
  }, [workspacePath, showHidden]);

  useEffect(() => {
    loadTree();
  }, [loadTree]);

  useEffect(() => {
    const unlisten = listen('file-system-change', () => {
      loadTree();
    });
    return () => { unlisten.then(fn => fn()); };
  }, [loadTree]);

  const handleNodeClick = (node: FileTreeNode, e: React.MouseEvent) => {
    e.stopPropagation();
    if (node.type === 'file') {
      setSelectedPath(node.path);
      onFileOpen?.(node.path);
    } else {
      setTree(prev => toggleExpanded(prev, node.path));
    }
  };

  const toggleExpanded = (node: FileTreeNode | null, path: string): FileTreeNode | null => {
    if (!node) return null;
    if (node.path === path) {
      return { ...node, expanded: !node.expanded };
    }
    if (node.children) {
      return {
        ...node,
        children: node.children.map(child => toggleExpanded(child, path)).filter(Boolean) as FileTreeNode[],
      };
    }
    return node;
  };

  const handleContextMenu = (e: React.MouseEvent, path: string) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY, path });
  };

  const closeContextMenu = () => setContextMenu(null);

  const handleNewFile = async () => {
    if (!contextMenu) return;
    const parentDir = contextMenu.path;
    const name = prompt('Enter file name:');
    if (!name) return;
    try {
      await invoke('create_file', { path: `${parentDir}/${name}` });
      loadTree();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create file');
    }
    closeContextMenu();
  };

  const handleNewFolder = async () => {
    if (!contextMenu) return;
    const parentDir = contextMenu.path;
    const name = prompt('Enter folder name:');
    if (!name) return;
    try {
      await invoke('create_folder', { path: `${parentDir}/${name}` });
      loadTree();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create folder');
    }
    closeContextMenu();
  };

  const handleDelete = async () => {
    if (!contextMenu) return;
    if (!confirm(`Delete ${contextMenu.path}?`)) return;
    try {
      await invoke('delete_path', { path: contextMenu.path });
      loadTree();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete');
    }
    closeContextMenu();
  };

  const handleRename = () => {
    if (!contextMenu) return;
    setRenamingPath(contextMenu.path);
    setRenameValue(contextMenu.path.split('/').pop() || '');
    closeContextMenu();
  };

  const confirmRename = async () => {
    if (!renamingPath || !renameValue.trim()) return;
    const newPath = renamingPath.substring(0, renamingPath.lastIndexOf('/') + 1) + renameValue.trim();
    try {
      await invoke('rename_path', { old_path: renamingPath, new_path: newPath });
      loadTree();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to rename');
    }
    setRenamingPath(null);
  };

  const cancelRename = () => {
    setRenamingPath(null);
  };

  const handleReveal = () => {
    if (!contextMenu) return;
    invoke('show_in_folder', { path: contextMenu.path }).catch(console.error);
    closeContextMenu();
  };

  const handleCopyPath = () => {
    if (!contextMenu) return;
    navigator.clipboard.writeText(contextMenu.path);
    closeContextMenu();
  };

  useEffect(() => {
    const handleClick = () => closeContextMenu();
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        closeContextMenu();
        cancelRename();
      }
    };
    document.addEventListener('click', handleClick);
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('click', handleClick);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, []);

  const renderNode = (node: FileTreeNode, depth: number = 0): React.ReactElement => {
    const isSelected = selectedPath === node.path;
    const isRenaming = renamingPath === node.path;
    const hasChildren = node.type === 'directory' && node.children && node.children.length > 0;
    const isExpanded = node.expanded ?? false;

    return (
      <div
        key={node.path}
        className="file-node"
        style={{ marginLeft: depth * 16 }}
        onContextMenu={e => handleContextMenu(e, node.path)}
      >
        <div
          className={`file-row ${isSelected ? 'selected' : ''} ${isRenaming ? 'renaming' : ''}`}
          onClick={e => handleNodeClick(node, e)}
          onDoubleClick={e => { if (node.type === 'directory') handleNodeClick(node, e); }}
        >
          {node.type === 'directory' && hasChildren && (
            <span className={`expand-icon ${isExpanded ? 'expanded' : ''}`} onClick={e => e.stopPropagation()}>
              {isExpanded ? '▼' : '▶'}
            </span>
          )}
          {node.type === 'directory' && !hasChildren && <span className="expand-icon empty">▶</span>}
          {node.type === 'file' && <span className="expand-icon file-icon">📄</span>}

          {isRenaming ? (
            <input
              type="text"
              value={renameValue}
              onChange={e => setRenameValue(e.target.value)}
              onBlur={confirmRename}
              onKeyDown={e => {
                if (e.key === 'Enter') confirmRename();
                if (e.key === 'Escape') cancelRename();
              }}
              autoFocus
              className="rename-input"
            />
          ) : (
            <span className="file-name">{node.name}</span>
          )}
        </div>

        {node.type === 'directory' && isExpanded && node.children && (
          <div className="file-children">
            {node.children.map(child => renderNode(child, depth + 1))}
          </div>
        )}
      </div>
    );
  };

  if (loading) {
    return <div className="file-explorer loading">Loading...</div>;
  }

  return (
    <div className="file-explorer" ref={treeRef}>
      <div className="explorer-header">
        <div className="explorer-title">EXPLORER</div>
        <div className="explorer-actions">
          <button className="icon-btn" onClick={loadTree} title="Refresh">
            ⟳
          </button>
          <button className="icon-btn" onClick={() => setShowHidden(!showHidden)} title={showHidden ? 'Hide hidden files' : 'Show hidden files'}>
            {showHidden ? '👁' : '👁‍🗨'}
          </button>
          <button className="icon-btn" onClick={() => { if (workspacePath) onFileOpen?.(workspacePath); }} title="New File">
            ➕
          </button>
        </div>
      </div>

      {error && <div className="error-banner">{error}</div>}

      <div className="explorer-tree">
        {tree ? renderNode(tree) : (
          <div className="empty-state">
            <p>No folder opened</p>
            <button onClick={() => { if (workspacePath) onFileOpen?.(workspacePath); }}>
              Open Folder
            </button>
          </div>
        )}
      </div>

      {contextMenu && (
        <div
          className="context-menu"
          style={{ left: contextMenu.x, top: contextMenu.y }}
        >
          <div className="context-menu-item" onClick={handleNewFile}>New File</div>
          <div className="context-menu-item" onClick={handleNewFolder}>New Folder</div>
          <hr className="context-menu-separator" />
          <div className="context-menu-item" onClick={handleRename}>Rename</div>
          <div className="context-menu-item danger" onClick={handleDelete}>Delete</div>
          <hr className="context-menu-separator" />
          <div className="context-menu-item" onClick={handleReveal}>Reveal in Finder</div>
          <div className="context-menu-item" onClick={handleCopyPath}>Copy Path</div>
        </div>
      )}
    </div>
  );
};

export default FileExplorer;