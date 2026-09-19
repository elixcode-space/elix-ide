import React, { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './SearchPanel.css';

interface SearchResult {
  file: string;
  line: number;
  column: number;
  match: string;
  contextBefore: string[];
  contextAfter: string[];
}

interface SearchResponse {
  results: SearchResult[];
  totalMatches: number;
  filesWithMatches: number;
  durationMs: number;
}

interface ReplaceResult {
  replaced: number;
  files: string[];
}

export const SearchPanel: React.FC = () => {
  const [query, setQuery] = useState('');
  const [replaceQuery, setReplaceQuery] = useState('');
  const [results, setResults] = useState<SearchResult[]>([]);
  const [totalMatches, setTotalMatches] = useState(0);
  const [filesWithMatches, setFilesWithMatches] = useState(0);
  const [duration, setDuration] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchOptions, setSearchOptions] = useState({
    caseSensitive: false,
    wholeWord: false,
    regex: false,
    includePattern: '',
    excludePattern: '',
    maxResults: 1000,
  });
  const [showReplace, setShowReplace] = useState(false);
  const [expandedFiles, setExpandedFiles] = useState<Set<string>>(new Set());
  const searchDebounceRef = useRef<NodeJS.Timeout | null>(null);

  const performSearch = useCallback(async () => {
    if (!query.trim()) {
      setResults([]);
      setTotalMatches(0);
      setFilesWithMatches(0);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const response = await invoke<SearchResponse>('search_code', {
        query,
        options: searchOptions,
      });
      setResults(response.results || []);
      setTotalMatches(response.totalMatches || 0);
      setFilesWithMatches(response.filesWithMatches || 0);
      setDuration(response.durationMs || 0);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Search failed');
      setResults([]);
    } finally {
      setLoading(false);
    }
  }, [query, searchOptions]);

  const handleSearch = useCallback(() => {
    if (searchDebounceRef.current) {
      clearTimeout(searchDebounceRef.current);
    }
    searchDebounceRef.current = setTimeout(() => {
      performSearch();
    }, 150);
  }, [performSearch]);

  const handleReplace = async (mode: 'all' | 'file' | 'selection') => {
    if (!query.trim() || !replaceQuery) return;
    if (!confirm(`Replace "${query}" with "${replaceQuery}" ${mode === 'all' ? 'in all files' : mode === 'file' ? 'in current file' : 'in selection'}?`)) return;

    try {
      setLoading(true);
      const result = await invoke<ReplaceResult>('search_replace', {
        query,
        replacement: replaceQuery,
        options: { ...searchOptions, mode },
      });
      alert(`Replaced ${result.replaced} occurrences in ${result.files.length} files`);
      await performSearch();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Replace failed');
    } finally {
      setLoading(false);
    }
  };

  const handleOptionChange = (key: string, value: boolean | string) => {
    setSearchOptions(prev => ({ ...prev, [key]: value }));
    handleSearch();
  };

  const toggleFileExpand = (file: string) => {
    setExpandedFiles(prev => {
      const next = new Set(prev);
      if (next.has(file)) next.delete(file);
      else next.add(file);
      return next;
    });
  };

  const getResultsByFile = () => {
    const grouped: Record<string, SearchResult[]> = {};
    for (const result of results) {
      if (!grouped[result.file]) grouped[result.file] = [];
      grouped[result.file].push(result);
    }
    return grouped;
  };

  useEffect(() => {
    handleSearch();
    return () => {
      if (searchDebounceRef.current) clearTimeout(searchDebounceRef.current);
    };
  }, [handleSearch]);

  const groupedResults = getResultsByFile();
  const files = Object.keys(groupedResults);

  return (
    <div className="search-panel">
      <div className="search-header">
        <div className="search-input-row">
          <label htmlFor="search-query" className="search-label">Search</label>
          <input
            id="search-query"
            type="text"
            value={query}
            onChange={e => { setQuery(e.target.value); handleSearch(); }}
            placeholder="Search in files..."
            className="search-input"
            autoFocus
          />
          <div className="search-options">
            <label className="option-checkbox">
              <input
                type="checkbox"
                checked={searchOptions.caseSensitive}
                onChange={e => handleOptionChange('caseSensitive', e.target.checked)}
              />
              <span title="Match Case (Alt+C)">Aa</span>
            </label>
            <label className="option-checkbox">
              <input
                type="checkbox"
                checked={searchOptions.wholeWord}
                onChange={e => handleOptionChange('wholeWord', e.target.checked)}
              />
              <span title="Match Whole Word (Alt+W)">\b</span>
            </label>
            <label className="option-checkbox">
              <input
                type="checkbox"
                checked={searchOptions.regex}
                onChange={e => handleOptionChange('regex', e.target.checked)}
              />
              <span title="Use Regular Expression (Alt+R)">.*</span>
            </label>
          </div>
        </div>

        {showReplace && (
          <div className="replace-input-row">
            <label htmlFor="replace-query" className="search-label">Replace</label>
            <input
              id="replace-query"
              type="text"
              value={replaceQuery}
              onChange={e => setReplaceQuery(e.target.value)}
              placeholder="Replace with..."
              className="search-input"
            />
            <div className="replace-actions">
              <button className="btn-secondary" onClick={() => handleReplace('selection')} disabled={loading || !query}>
                Replace in Selection
              </button>
              <button className="btn-secondary" onClick={() => handleReplace('file')} disabled={loading || !query}>
                Replace in File
              </button>
              <button className="btn-primary" onClick={() => handleReplace('all')} disabled={loading || !query}>
                Replace All
              </button>
            </div>
          </div>
        )}

        <div className="search-filters">
          <input
            type="text"
            placeholder="files to include (e.g. *.ts,*.tsx)"
            value={searchOptions.includePattern}
            onChange={e => handleOptionChange('includePattern', e.target.value)}
            className="filter-input"
          />
          <input
            type="text"
            placeholder="files to exclude (e.g. node_modules,dist)"
            value={searchOptions.excludePattern}
            onChange={e => handleOptionChange('excludePattern', e.target.value)}
            className="filter-input"
          />
        </div>
      </div>

      <div className="search-toolbar">
        <div className="search-stats">
          {totalMatches > 0 && (
            <>
              <span>{totalMatches} matches</span>
              <span>{filesWithMatches} files</span>
              <span>{duration}ms</span>
            </>
          )}
        </div>
        <div className="search-actions">
          <button className="btn-icon" onClick={() => setShowReplace(!showReplace)} title="Toggle Replace">
            {showReplace ? '⌄' : '⌃'}
          </button>
          <button className="btn-icon" onClick={() => { setQuery(''); setResults([]); }} title="Clear Search">
            ✕
          </button>
        </div>
      </div>

      {error && <div className="error-banner">{error}</div>}

      <div className="search-results">
        {loading && <div className="loading">Searching...</div>}

        {!loading && results.length === 0 && query.trim() && (
          <div className="no-results">No results found for "{query}"</div>
        )}

        {!loading && results.length === 0 && !query.trim() && (
          <div className="empty-state">
            <p>Enter a search query to find code</p>
            <p className="hint">Supports regex, case-sensitive, whole word matching</p>
          </div>
        )}

        {files.map(file => {
          const fileResults = groupedResults[file];
          const expanded = expandedFiles.has(file);
          return (
            <div key={file} className="file-result">
              <div className="file-header" onClick={() => toggleFileExpand(file)}>
                <span className="expand-icon">{expanded ? '▼' : '▶'}</span>
                <span className="file-path">{file}</span>
                <span className="file-match-count">{fileResults.length} matches</span>
              </div>
              {expanded && (
                <div className="file-matches">
                  {fileResults.map((match, idx) => (
                    <div key={idx} className="match-line">
                      <span className="line-number">{match.line}:{match.column}</span>
                      <div className="match-context">
                        {match.contextBefore.map((ctx, i) => (
                          <span key={`before-${i}`} className="context-line">{ctx}</span>
                        ))}
                        <span className="match-highlight">{match.match}</span>
                        {match.contextAfter.map((ctx, i) => (
                          <span key={`after-${i}`} className="context-line">{ctx}</span>
                        ))}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};

export default SearchPanel;