import React, { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './ExtensionMarketplace.css';

interface ExtensionInfo {
  name: string;
  displayName: string;
  publisher: string;
  version: string;
  description: string;
  iconUrl?: string;
  installCount: number;
  rating: number;
  categories: string[];
  tags: string[];
  repository?: string;
  latestVersion?: string;
}

interface InstalledExtension {
  id: string;
  name: string;
  version: string;
  publisher: string;
  enabled: boolean;
  description: string;
  path: string;
}

export const ExtensionMarketplace: React.FC = () => {
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<ExtensionInfo[]>([]);
  const [installedExtensions, setInstalledExtensions] = useState<InstalledExtension[]>([]);
  const [activeTab, setActiveTab] = useState<'marketplace' | 'installed'>('marketplace');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedExtension, setSelectedExtension] = useState<ExtensionInfo | null>(null);
  const [page, setPage] = useState(1);
  const [hasMore, setHasMore] = useState(true);

  const loadInstalled = useCallback(async () => {
    try {
      const result = await invoke<{ extensions: InstalledExtension[] }>('extension_list_installed');
      setInstalledExtensions(result.extensions || []);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load installed extensions');
    }
  }, []);

  const searchExtensions = async (query: string, pageNum: number = 1) => {
    if (!query.trim()) {
      setSearchResults([]);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<{ extensions: ExtensionInfo[]; hasMore: boolean }>('extension_search', {
        query,
        page: pageNum,
        pageSize: 20,
      });
      if (pageNum === 1) {
        setSearchResults(result.extensions || []);
      } else {
        setSearchResults(prev => [...prev, ...(result.extensions || [])]);
      }
      setHasMore(result.hasMore || false);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Search failed');
    } finally {
      setLoading(false);
    }
  };

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    setPage(1);
    searchExtensions(searchQuery, 1);
  };

  const handleInstall = async (ext: ExtensionInfo) => {
    try {
      setError(null);
      await invoke('extension_install', { 
        name: ext.name, 
        publisher: ext.publisher, 
        version: ext.version 
      });
      await loadInstalled();
      alert(`Installed ${ext.displayName}@${ext.version}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Install failed');
    }
  };

  const handleUninstall = async (ext: InstalledExtension) => {
    if (!confirm(`Uninstall ${ext.name}?`)) return;
    try {
      await invoke('extension_uninstall', { id: ext.id });
      await loadInstalled();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Uninstall failed');
    }
  };

  const handleToggleEnable = async (ext: InstalledExtension) => {
    try {
      if (ext.enabled) {
        await invoke('extension_disable', { id: ext.id });
      } else {
        await invoke('extension_enable', { id: ext.id });
      }
      await loadInstalled();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to toggle extension');
    }
  };

  const handleShowDetails = (ext: ExtensionInfo) => {
    setSelectedExtension(ext);
  };

  const closeDetails = () => {
    setSelectedExtension(null);
  };

  useEffect(() => {
    loadInstalled();
  }, [loadInstalled]);

  return (
    <div className="extension-marketplace">
      <div className="marketplace-header">
        <h2>Extensions</h2>
        <form onSubmit={handleSearch} className="search-form">
          <input
            type="text"
            placeholder="Search extensions in marketplace..."
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
            className="search-input"
          />
          <button type="submit" className="btn-primary" disabled={loading}>
            {loading ? 'Searching...' : 'Search'}
          </button>
        </form>
      </div>

      {error && <div className="error-banner">{error}</div>}

      <div className="marketplace-tabs">
        <button 
          className={`tab ${activeTab === 'marketplace' ? 'active' : ''}`}
          onClick={() => setActiveTab('marketplace')}
        >
          Marketplace
        </button>
        <button 
          className={`tab ${activeTab === 'installed' ? 'active' : ''}`}
          onClick={() => setActiveTab('installed')}
        >
          Installed {installedExtensions.length > 0 && `(${installedExtensions.length})`}
        </button>
      </div>

      <div className="marketplace-content">
        {activeTab === 'marketplace' && (
          <MarketplaceView
            results={searchResults}
            loading={loading}
            hasMore={hasMore}
            page={page}
            onLoadMore={() => searchExtensions(searchQuery, page + 1)}
            onInstall={handleInstall}
            onShowDetails={handleShowDetails}
          />
        )}

        {activeTab === 'installed' && (
          <InstalledView
            extensions={installedExtensions}
            onUninstall={handleUninstall}
            onToggleEnable={handleToggleEnable}
          />
        )}
      </div>

      {selectedExtension && (
        <ExtensionDetailModal
          extension={selectedExtension}
          onClose={closeDetails}
          onInstall={handleInstall}
        />
      )}
    </div>
  );
};

const MarketplaceView: React.FC<{
  results: ExtensionInfo[];
  loading: boolean;
  hasMore: boolean;
  page: number;
  onLoadMore: () => void;
  onInstall: (ext: ExtensionInfo) => void;
  onShowDetails: (ext: ExtensionInfo) => void;
}> = ({ results, loading, hasMore, page: _page, onLoadMore, onInstall, onShowDetails }) => {
  if (results.length === 0 && !loading) {
    return (
      <div className="empty-state">
        <p>Search for extensions in the Open VSX marketplace</p>
        <p className="hint">Try: "prettier", "eslint", "gitlens", "rust-analyzer"</p>
      </div>
    );
  }

  return (
    <div className="marketplace-list">
      {results.map(ext => (
        <ExtensionCard
          key={`${ext.publisher}.${ext.name}`}
          extension={ext}
          onInstall={onInstall}
          onShowDetails={onShowDetails}
        />
      ))}
      {hasMore && (
        <button className="btn-load-more" onClick={onLoadMore} disabled={loading}>
          {loading ? 'Loading...' : 'Load More'}
        </button>
      )}
    </div>
  );
};

const ExtensionCard: React.FC<{
  extension: ExtensionInfo;
  onInstall: (ext: ExtensionInfo) => void;
  onShowDetails: (ext: ExtensionInfo) => void;
}> = ({ extension, onInstall, onShowDetails }) => (
  <div className="extension-card">
    <div className="ext-icon">
      {extension.iconUrl ? (
        <img src={extension.iconUrl} alt="" onError={e => { e.currentTarget.style.display = 'none'; }} />
      ) : (
        '📦'
      )}
    </div>
    <div className="ext-info">
      <div className="ext-header">
        <h4>{extension.displayName || extension.name}</h4>
        <span className="ext-publisher">{extension.publisher}</span>
      </div>
      <p className="ext-description">{extension.description}</p>
      <div className="ext-meta">
        <span className="ext-version">v{extension.version}</span>
        <span className="ext-installs">{extension.installCount.toLocaleString()} installs</span>
        {extension.rating > 0 && (
          <span className="ext-rating">★ {extension.rating.toFixed(1)}</span>
        )}
      </div>
      <div className="ext-tags">
        {extension.categories.slice(0, 3).map((cat, i) => (
          <span key={i} className="tag">{cat}</span>
        ))}
      </div>
    </div>
    <div className="ext-actions">
      <button className="btn-secondary" onClick={() => onShowDetails(extension)}>
        Details
      </button>
      <button className="btn-primary" onClick={() => onInstall(extension)}>
        Install
      </button>
    </div>
  </div>
);

const InstalledView: React.FC<{
  extensions: InstalledExtension[];
  onUninstall: (ext: InstalledExtension) => void;
  onToggleEnable: (ext: InstalledExtension) => void;
}> = ({ extensions, onUninstall, onToggleEnable }) => {
  if (extensions.length === 0) {
    return (
      <div className="empty-state">
        <p>No extensions installed</p>
        <p className="hint">Go to Marketplace tab to install extensions</p>
      </div>
    );
  }

  return (
    <div className="installed-list">
      {extensions.map(ext => (
        <div key={ext.id} className="installed-card">
          <div className="ext-info">
            <div className="ext-header">
              <h4>{ext.name}</h4>
              <span className={`ext-status ${ext.enabled ? 'enabled' : 'disabled'}`}>
                {ext.enabled ? 'Enabled' : 'Disabled'}
              </span>
            </div>
            <p className="ext-description">{ext.description}</p>
            <div className="ext-meta">
              <span className="ext-version">v{ext.version}</span>
              <span className="ext-publisher">{ext.publisher}</span>
            </div>
          </div>
          <div className="ext-actions">
            <button 
              className={ext.enabled ? 'btn-secondary' : 'btn-primary'}
              onClick={() => onToggleEnable(ext)}
            >
              {ext.enabled ? 'Disable' : 'Enable'}
            </button>
            <button className="btn-secondary danger" onClick={() => onUninstall(ext)}>
              Uninstall
            </button>
          </div>
        </div>
      ))}
    </div>
  );
};

const ExtensionDetailModal: React.FC<{
  extension: ExtensionInfo;
  onClose: () => void;
  onInstall: (ext: ExtensionInfo) => void;
}> = ({ extension, onClose, onInstall }) => (
  <div className="modal-overlay" onClick={onClose}>
    <div className="modal-content" onClick={e => e.stopPropagation()}>
      <div className="modal-header">
        <h3>{extension.displayName || extension.name}</h3>
        <button className="modal-close" onClick={onClose}>✕</button>
      </div>
      <div className="modal-body">
        <div className="ext-detail-header">
          {extension.iconUrl && <img src={extension.iconUrl} alt="" />}
          <div>
            <h4>{extension.displayName || extension.name}</h4>
            <span className="ext-publisher">{extension.publisher}</span>
            <div className="ext-meta">
              <span>v{extension.version}</span>
              <span>{extension.installCount.toLocaleString()} installs</span>
              {extension.rating > 0 && <span>★ {extension.rating.toFixed(1)}</span>}
            </div>
          </div>
        </div>
        <p className="ext-description">{extension.description}</p>
        {extension.repository && (
          <a href={extension.repository} target="_blank" rel="noopener noreferrer" className="ext-link">
            View Repository
          </a>
        )}
        <div className="ext-tags">
          {extension.categories.map((cat, i) => (
            <span key={i} className="tag">{cat}</span>
          ))}
        </div>
      </div>
      <div className="modal-footer">
        <button className="btn-primary" onClick={() => { onInstall(extension); onClose(); }}>
          Install
        </button>
      </div>
    </div>
  </div>
);

export default ExtensionMarketplace;