import React, { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import './TerminalPanel.css';

interface TerminalInstance {
  id: string;
  profileId: string;
  title: string;
  cwd: string;
  pid?: number;
  status: 'starting' | 'running' | 'exited';
  exitCode?: number;
  cols: number;
  rows: number;
}

interface TerminalProfile {
  id: string;
  name: string;
  shell: string;
  args?: string[];
  env?: Record<string, string>;
  cwd?: string;
  icon?: string;
  color?: string;
}

interface TerminalConfig {
  profiles: TerminalProfile[];
  defaultProfile: string;
  fontSize: number;
  fontFamily: string;
  cursorElixirIDE: boolean;
  cursorStyle: 'block' | 'underline' | 'bar';
  scrollback: number;
  bell: boolean;
}

interface TerminalLink {
  text: string;
  url: string;
  range: { start: number; end: number };
  line: number;
}

export const TerminalPanel: React.FC = () => {
  const [instances, setInstances] = useState<TerminalInstance[]>([]);
  const [profiles, setProfiles] = useState<TerminalProfile[]>([]);
  const [config, setConfig] = useState<TerminalConfig | null>(null);
  const [activeTerminalId, setActiveTerminalId] = useState<string | null>(null);
  const [splitDirection, setSplitDirection] = useState<'horizontal' | 'vertical'>('horizontal');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showProfileSelector, setShowProfileSelector] = useState(false);
  const terminalRefs = useRef<Map<string, HTMLDivElement>>(new Map());

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      const [instResult, profResult, configResult] = await Promise.all([
        invoke<{ terminals: TerminalInstance[] }>('terminal_list'),
        invoke<{ profiles: TerminalProfile[] }>('terminal_get_profiles'),
        invoke<{ config: TerminalConfig }>('terminal_get_config'),
      ]);
      setInstances(instResult.terminals || []);
      setProfiles(profResult.profiles || []);
      setConfig(configResult.config);
      if (instResult.terminals?.[0]?.id && !activeTerminalId) {
        setActiveTerminalId(instResult.terminals[0].id);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load terminal data');
    } finally {
      setLoading(false);
    }
  }, [activeTerminalId]);

  const createTerminal = async (profileId?: string) => {
    try {
      setError(null);
      const result = await invoke<{ id: string }>('terminal_create', {
        profileId: profileId || config?.defaultProfile,
      });
      await loadData();
      setActiveTerminalId(result.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create terminal');
    }
  };

  const killTerminal = async (id: string) => {
    if (!confirm('Kill this terminal?')) return;
    try {
      await invoke('terminal_kill', { id });
      await loadData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to kill terminal');
    }
  };

  const resizeTerminal = async (id: string, cols: number, rows: number) => {
    try {
      await invoke('terminal_resize', { id, cols, rows });
    } catch (err) {
      console.error('Failed to resize terminal:', err);
    }
  };

  const sendInput = async (id: string, data: string) => {
    try {
      await invoke('terminal_write', { id, data });
    } catch (err) {
      console.error('Failed to send input:', err);
    }
  };

  const handleSplit = async (direction: 'horizontal' | 'vertical') => {
    if (!activeTerminalId) return;
    try {
      const result = await invoke<{ id: string }>('terminal_split', {
        id: activeTerminalId,
        direction,
      });
      await loadData();
      setActiveTerminalId(result.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to split terminal');
    }
  };

  const handleProfileSelect = async (profile: TerminalProfile) => {
    await createTerminal(profile.id);
    setShowProfileSelector(false);
  };

  useEffect(() => {
    loadData();

    let unlistenFn: (() => void) | null = null;
    listen<string>('terminal-output', (event) => {
      const { payload } = event;
      console.log('Terminal output:', payload);
    }).then(fn => { unlistenFn = fn; });

    return () => {
      if (unlistenFn) unlistenFn();
    };
  }, [loadData]);

  const activeTerminal = instances.find(t => t.id === activeTerminalId);

  return (
    <div className="terminal-panel">
      <div className="terminal-header">
        <div className="terminal-tabs">
          {instances.map(term => (
            <TerminalTab
              key={term.id}
              terminal={term}
              active={term.id === activeTerminalId}
              onClick={() => setActiveTerminalId(term.id)}
              onClose={() => killTerminal(term.id)}
            />
          ))}
          <button className="tab-new" onClick={() => setShowProfileSelector(true)} title="New Terminal">
            +
          </button>
        </div>
        <div className="terminal-actions">
          <select
            value={splitDirection}
            onChange={e => setSplitDirection(e.target.value as 'horizontal' | 'vertical')}
            className="split-select"
          >
            <option value="horizontal">Split Horizontal</option>
            <option value="vertical">Split Vertical</option>
          </select>
          <button className="btn-split" onClick={() => handleSplit(splitDirection)} disabled={!activeTerminalId}>
            Split
          </button>
          <button className="btn-profile" onClick={() => setShowProfileSelector(true)}>
            Profiles
          </button>
        </div>
      </div>

      {error && <div className="error-banner">{error}</div>}

      <div className="terminal-content">
        {activeTerminal ? (
          <TerminalView
            terminal={activeTerminal}
            ref={el => { if (el) terminalRefs.current.set(activeTerminal.id, el); }}
            onResize={resizeTerminal}
            onInput={sendInput}
            config={config}
          />
        ) : (
          <div className="terminal-empty">
            <p>No terminals</p>
            <button className="btn-primary" onClick={() => setShowProfileSelector(true)}>
              Create Terminal
            </button>
          </div>
        )}
      </div>

      {showProfileSelector && (
        <ProfileSelectorModal
          profiles={profiles}
          onSelect={handleProfileSelect}
          onClose={() => setShowProfileSelector(false)}
        />
      )}
    </div>
  );
};

const TerminalTab: React.FC<{
  terminal: TerminalInstance;
  active: boolean;
  onClick: () => void;
  onClose: () => void;
}> = ({ terminal, active, onClick, onClose }) => (
  <div
    className={`terminal-tab ${active ? 'active' : ''} ${terminal.status === 'exited' ? 'exited' : ''}`}
    onClick={onClick}
  >
    <span className="tab-title">{terminal.title}</span>
    <span className="tab-status">{terminal.status === 'running' ? '●' : terminal.status === 'exited' ? '✕' : '◐'}</span>
    <button className="tab-close" onClick={e => { e.stopPropagation(); onClose(); }}>
      ✕
    </button>
  </div>
);

const TerminalView: React.FC<{
  terminal: TerminalInstance;
  ref: React.Ref<HTMLDivElement>;
  onResize: (id: string, cols: number, rows: number) => void;
  onInput: (id: string, data: string) => void;
  config?: TerminalConfig | null;
}> = ({ terminal, ref, onResize, onInput, config }) => {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [output, setOutput] = useState<string>('');
  const [cursorPos, setCursorPos] = useState(0);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Handle special keys
      if (e.key === 'Enter') {
        onInput(terminal.id, '\r');
        e.preventDefault();
      } else if (e.key === 'Tab') {
        onInput(terminal.id, '\t');
        e.preventDefault();
      } else if (e.key === 'ArrowUp') {
        onInput(terminal.id, '\x1b[A');
        e.preventDefault();
      } else if (e.key === 'ArrowDown') {
        onInput(terminal.id, '\x1b[B');
        e.preventDefault();
      } else if (e.key === 'ArrowRight') {
        onInput(terminal.id, '\x1b[C');
        e.preventDefault();
      } else if (e.key === 'ArrowLeft') {
        onInput(terminal.id, '\x1b[D');
        e.preventDefault();
      } else if (e.ctrlKey && e.key === 'c') {
        onInput(terminal.id, '\x03');
        e.preventDefault();
      } else if (e.ctrlKey && e.key === 'd') {
        onInput(terminal.id, '\x04');
        e.preventDefault();
      } else if (e.ctrlKey && e.key === 'l') {
        onInput(terminal.id, '\x0c');
        e.preventDefault();
      }
    };

    const handleInput = (e: InputEvent) => {
      const target = e.currentTarget;
      const value = target.value;
      if (value.length > cursorPos) {
        const newChar = value.slice(cursorPos);
        onInput(terminal.id, newChar);
      }
      setCursorPos(value.length);
    };

    const textarea = textareaRef.current;
    if (textarea) {
      textarea.addEventListener('keydown', handleKeyDown);
      textarea.addEventListener('input', handleInput);
    }

    return () => {
      if (textarea) {
        textarea.removeEventListener('keydown', handleKeyDown);
        textarea.removeEventListener('input', handleInput);
      }
    };
  }, [terminal.id, cursorPos, onInput]);

  useEffect(() => {
    const handleResize = () => {
      if (textareaRef.current) {
        const charsPerLine = Math.floor(textareaRef.current.clientWidth / 9);
        const lines = Math.floor(textareaRef.current.clientHeight / 20);
        onResize(terminal.id, charsPerLine, lines);
      }
    };

    const observer = new ResizeObserver(handleResize);
    if (textareaRef.current) {
      observer.observe(textareaRef.current);
    }
    return () => observer.disconnect();
  }, [terminal.id, onResize]);

  const fontSize = config?.fontSize || 13;
  const fontFamily = config?.fontFamily || 'Monospace';

  return (
    <div ref={ref} className="terminal-view">
      <div className="terminal-output" style={{ fontSize, fontFamily }}>
        {output}
      </div>
      <textarea
        ref={textareaRef}
        className="terminal-input"
        style={{ fontSize, fontFamily }}
        spellCheck={false}
        autoComplete="off"
        autoCorrect="off"
        autoCapitalize="off"
      />
    </div>
  );
};

const ProfileSelectorModal: React.FC<{
  profiles: TerminalProfile[];
  onSelect: (profile: TerminalProfile) => void;
  onClose: () => void;
}> = ({ profiles, onSelect, onClose }) => (
  <div className="modal-overlay" onClick={onClose}>
    <div className="modal-content" onClick={e => e.stopPropagation()}>
      <div className="modal-header">
        <h3>Select Profile</h3>
        <button className="modal-close" onClick={onClose}>✕</button>
      </div>
      <div className="modal-body">
        {profiles.map(profile => (
          <div key={profile.id} className="profile-item" onClick={() => onSelect(profile)}>
            <div className="profile-icon" style={{ backgroundColor: profile.color }}>
              {profile.icon || '💻'}
            </div>
            <div className="profile-info">
              <h4>{profile.name}</h4>
              <span className="profile-shell">{profile.shell}</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  </div>
);

export default TerminalPanel;