import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './SettingsEditor.css';

interface SettingValue {
  [key: string]: unknown;
}

interface SettingDefinition {
  key: string;
  type: 'boolean' | 'number' | 'string' | 'enum';
  default: unknown;
  label: string;
  description: string;
  category: string;
  order?: number;
  enum?: unknown[];
  enumLabels?: string[];
  min?: number;
  max?: number;
  step?: number;
}

const SETTING_CATEGORIES = [
  { id: 'editor', label: 'Editor', order: 1 },
  { id: 'workbench', label: 'Workbench', order: 2 },
  { id: 'terminal', label: 'Terminal', order: 3 },
  { id: 'lsp', label: 'Language Server', order: 4 },
  { id: 'extensions', label: 'Extensions', order: 5 },
  { id: 'files', label: 'Files', order: 6 },
  { id: 'search', label: 'Search', order: 7 },
];

const SETTINGS_SCHEMA: SettingDefinition[] = [
  // Editor
  { key: 'editor.font_size', type: 'number', default: 14, label: 'Font Size', description: 'Controls the font size in pixels.', category: 'editor', order: 1, min: 8, max: 64, step: 1 },
  { key: 'editor.font_family', type: 'string', default: 'Monospace', label: 'Font Family', description: 'Font family for the editor.', category: 'editor', order: 2 },
  { key: 'editor.line_height', type: 'number', default: 1.5, label: 'Line Height', description: 'Line height as a multiplier.', category: 'editor', order: 3, min: 1, max: 3, step: 0.1 },
  { key: 'editor.font_ligatures', type: 'boolean', default: true, label: 'Font Ligatures', description: 'Enable font ligatures.', category: 'editor', order: 4 },
  { key: 'editor.tab_size', type: 'number', default: 4, label: 'Tab Size', description: 'Number of spaces per tab.', category: 'editor', order: 5, min: 1, max: 16, step: 1 },
  { key: 'editor.insert_spaces', type: 'boolean', default: true, label: 'Insert Spaces', description: 'Use spaces instead of tabs.', category: 'editor', order: 6 },
  { key: 'editor.detect_indentation', type: 'boolean', default: true, label: 'Detect Indentation', description: 'Auto-detect indentation from file.', category: 'editor', order: 7 },
  { key: 'editor.word_wrap', type: 'enum', default: 'off', label: 'Word Wrap', description: 'Word wrap mode.', category: 'editor', order: 8, enum: ['off', 'on', 'wordWrapColumn', 'bounded'], enumLabels: ['Off', 'On', 'Wrap at Column', 'Bounded'] },
  { key: 'editor.minimap_enabled', type: 'boolean', default: true, label: 'Minimap', description: 'Show minimap.', category: 'editor', order: 9 },
  { key: 'editor.line_numbers', type: 'enum', default: 'on', label: 'Line Numbers', description: 'Line number display mode.', category: 'editor', order: 10, enum: ['on', 'off', 'relative', 'relativeOn'], enumLabels: ['On', 'Off', 'Relative', 'Relative On'] },
  { key: 'editor.cursor_blinking', type: 'enum', default: 'blink', label: 'Cursor Blinking', description: 'Cursor blink style.', category: 'editor', order: 11, enum: ['blink', 'smooth', 'phase', 'expand', 'solid', 'off'], enumLabels: ['Blink', 'Smooth', 'Phase', 'Expand', 'Solid', 'Off'] },
  { key: 'editor.cursor_style', type: 'enum', default: 'line', label: 'Cursor Style', description: 'Cursor shape.', category: 'editor', order: 12, enum: ['line', 'block', 'underline', 'lineThin', 'blockOutline', 'underlineThin'], enumLabels: ['Line', 'Block', 'Underline', 'Line Thin', 'Block Outline', 'Underline Thin'] },
  { key: 'editor.smooth_scrolling', type: 'boolean', default: true, label: 'Smooth Scrolling', description: 'Enable smooth scrolling.', category: 'editor', order: 13 },
  { key: 'editor.bracket_pair_colorization', type: 'boolean', default: true, label: 'Bracket Pair Colorization', description: 'Colorize matching brackets.', category: 'editor', order: 14 },
  { key: 'editor.guides_bracket_pairs', type: 'boolean', default: true, label: 'Bracket Pair Guides', description: 'Show guides for bracket pairs.', category: 'editor', order: 15 },
  { key: 'editor.render_whitespace', type: 'enum', default: 'selection', label: 'Render Whitespace', description: 'Whitespace rendering.', category: 'editor', order: 16, enum: ['selection', 'none', 'all', 'boundary', 'trailing'], enumLabels: ['Selection', 'None', 'All', 'Boundary', 'Trailing'] },
  { key: 'editor.render_control_characters', type: 'boolean', default: false, label: 'Render Control Characters', description: 'Show control characters.', category: 'editor', order: 17 },
  { key: 'editor.folding_strategy', type: 'enum', default: 'auto', label: 'Folding Strategy', description: 'Code folding strategy.', category: 'editor', order: 18, enum: ['auto', 'indentation'], enumLabels: ['Auto', 'Indentation'] },
  { key: 'editor.unfold_on_click_after_end_of_line', type: 'boolean', default: true, label: 'Unfold on Click', description: 'Unfold when clicking after end of line.', category: 'editor', order: 19 },

  // Workbench
  { key: 'workbench.color_theme', type: 'string', default: 'Default Dark+', label: 'Color Theme', description: 'Color theme for the workbench.', category: 'workbench', order: 1 },
  { key: 'workbench.icon_theme', type: 'string', default: 'vs-seti', label: 'Icon Theme', description: 'File icon theme.', category: 'workbench', order: 2 },
  { key: 'workbench.startup_editor', type: 'enum', default: 'welcomePage', label: 'Startup Editor', description: 'Editor shown on startup.', category: 'workbench', order: 3, enum: ['welcomePage', 'readOnly', 'newUntitledFile', 'welcomePageInEmptyWindow'], enumLabels: ['Welcome Page', 'Read Only', 'New Untitled File', 'Welcome Page (Empty Window)'] },
  { key: 'workbench.restore_windows', type: 'enum', default: 'preserve', label: 'Restore Windows', description: 'Window restoration behavior.', category: 'workbench', order: 4, enum: ['preserve', 'none', 'all', 'folders', 'one'], enumLabels: ['Preserve', 'None', 'All', 'Folders', 'One'] },
  { key: 'workbench.open_mode', type: 'enum', default: 'singleInstance', label: 'Open Mode', description: 'How windows are opened.', category: 'workbench', order: 5, enum: ['singleInstance', 'singleInstancePerWorkspace'], enumLabels: ['Single Instance', 'Single Instance Per Workspace'] },
  { key: 'workbench.editor_enable_preview', type: 'boolean', default: true, label: 'Enable Preview', description: 'Preview editors from quick open.', category: 'workbench', order: 6 },
  { key: 'workbench.editor_enable_preview_from_quick_open', type: 'boolean', default: true, label: 'Preview from Quick Open', description: 'Preview from quick open.', category: 'workbench', order: 7 },
  { key: 'workbench.editor_close_empty_groups', type: 'boolean', default: true, label: 'Close Empty Groups', description: 'Close empty editor groups.', category: 'workbench', order: 8 },
  { key: 'workbench.panel_default_location', type: 'enum', default: 'bottom', label: 'Panel Location', description: 'Default panel position.', category: 'workbench', order: 9, enum: ['bottom', 'left', 'right'], enumLabels: ['Bottom', 'Left', 'Right'] },
  { key: 'workbench.sidebar_location', type: 'enum', default: 'left', label: 'Sidebar Location', description: 'Sidebar position.', category: 'workbench', order: 10, enum: ['left', 'right'], enumLabels: ['Left', 'Right'] },
  { key: 'workbench.activity_bar_visible', type: 'boolean', default: true, label: 'Activity Bar Visible', description: 'Show activity bar.', category: 'workbench', order: 11 },
  { key: 'workbench.status_bar_visible', type: 'boolean', default: true, label: 'Status Bar Visible', description: 'Show status bar.', category: 'workbench', order: 12 },
  { key: 'workbench.command_center', type: 'boolean', default: true, label: 'Command Center', description: 'Show command center in title bar.', category: 'workbench', order: 13 },

  // Terminal
  { key: 'terminal.integrated_shell', type: 'string', default: '', label: 'Default Shell', description: 'Path to default shell (empty for system default).', category: 'terminal', order: 1 },
  { key: 'terminal.integrated_shell_args', type: 'string', default: '', label: 'Shell Args', description: 'Shell arguments (comma-separated).', category: 'terminal', order: 2 },
  { key: 'terminal.cwd', type: 'string', default: '', label: 'Default CWD', description: 'Default working directory.', category: 'terminal', order: 3 },
  { key: 'terminal.shell_integration_enabled', type: 'boolean', default: true, label: 'Shell Integration', description: 'Enable shell integration (prompt tracking, etc.).', category: 'terminal', order: 4 },
  { key: 'terminal.cursor_blink', type: 'boolean', default: true, label: 'Cursor ElixirIDE', description: 'Enable cursor blinking.', category: 'terminal', order: 5 },
  { key: 'terminal.scrollback', type: 'number', default: 10000, label: 'Scrollback', description: 'Lines of scrollback history.', category: 'terminal', order: 6, min: 100, max: 100000, step: 100 },
  { key: 'terminal.font_size', type: 'number', default: 14, label: 'Font Size', description: 'Terminal font size.', category: 'terminal', order: 7, min: 8, max: 64, step: 1 },
  { key: 'terminal.font_family', type: 'string', default: 'Monospace', label: 'Font Family', description: 'Terminal font family.', category: 'terminal', order: 8 },
  { key: 'terminal.font_weight', type: 'string', default: 'normal', label: 'Font Weight', description: 'Font weight.', category: 'terminal', order: 9 },
  { key: 'terminal.font_weight_bold', type: 'string', default: 'bold', label: 'Bold Font Weight', description: 'Bold font weight.', category: 'terminal', order: 10 },
  { key: 'terminal.line_height', type: 'number', default: 1.5, label: 'Line Height', description: 'Line height multiplier.', category: 'terminal', order: 11, min: 1, max: 3, step: 0.1 },
  { key: 'terminal.letter_spacing', type: 'number', default: 0, label: 'Letter Spacing', description: 'Letter spacing in pixels.', category: 'terminal', order: 12, min: -2, max: 10, step: 0.5 },
  { key: 'terminal.allow_chords', type: 'boolean', default: true, label: 'Allow Chords', description: 'Allow key chords.', category: 'terminal', order: 13 },
  { key: 'terminal.fast_scroll_sensitivity', type: 'number', default: 5, label: 'Fast Scroll Sensitivity', description: 'Fast scroll multiplier.', category: 'terminal', order: 14, min: 1, max: 20, step: 1 },
  { key: 'terminal.mac_option_click_forces_selection', type: 'boolean', default: false, label: 'Option-Click Selection', description: 'Force selection with Option+Click on macOS.', category: 'terminal', order: 15 },
  { key: 'terminal.mac_option_is_meta', type: 'boolean', default: true, label: 'Option as Meta', description: 'Treat Option as Meta key on macOS.', category: 'terminal', order: 16 },
  { key: 'terminal.right_click_behavior', type: 'enum', default: 'default', label: 'Right Click Behavior', description: 'Right click action.', category: 'terminal', order: 17, enum: ['default', 'copyPaste', 'selectWord', 'nothing'], enumLabels: ['Default', 'Copy/Paste', 'Select Word', 'Nothing'] },
  { key: 'terminal.copy_on_selection', type: 'boolean', default: false, label: 'Copy on Selection', description: 'Copy to clipboard on text selection.', category: 'terminal', order: 18 },
  { key: 'terminal.word_separators', type: 'string', default: ' ()[]{}\'"`,;<>│', label: 'Word Separators', description: 'Characters that separate words.', category: 'terminal', order: 19 },

  // LSP
  { key: 'lsp.enabled', type: 'boolean', default: true, label: 'Enable LSP', description: 'Enable Language Server Protocol.', category: 'lsp', order: 1 },
  { key: 'lsp.auto_start', type: 'boolean', default: true, label: 'Auto Start', description: 'Automatically start language servers.', category: 'lsp', order: 2 },
  { key: 'lsp.log_level', type: 'enum', default: 'info', label: 'Log Level', description: 'LSP log verbosity.', category: 'lsp', order: 3, enum: ['info', 'debug', 'trace', 'warn', 'error', 'off'], enumLabels: ['Info', 'Debug', 'Trace', 'Warn', 'Error', 'Off'] },
  { key: 'lsp.trace_server', type: 'enum', default: 'off', label: 'Trace Server', description: 'Trace server communication.', category: 'lsp', order: 4, enum: ['off', 'messages', 'verbose'], enumLabels: ['Off', 'Messages', 'Verbose'] },
  { key: 'lsp.workspace_symbols_enabled', type: 'boolean', default: true, label: 'Workspace Symbols', description: 'Enable workspace symbols.', category: 'lsp', order: 5 },
  { key: 'lsp.document_highlight_enabled', type: 'boolean', default: true, label: 'Document Highlight', description: 'Highlight symbol occurrences.', category: 'lsp', order: 6 },
  { key: 'lsp.code_lens_enabled', type: 'boolean', default: true, label: 'Code Lens', description: 'Show code lens.', category: 'lsp', order: 7 },
  { key: 'lsp.folding_range_enabled', type: 'boolean', default: true, label: 'Folding Range', description: 'Enable folding ranges.', category: 'lsp', order: 8 },
  { key: 'lsp.selection_range_enabled', type: 'boolean', default: true, label: 'Selection Range', description: 'Enable selection ranges.', category: 'lsp', order: 9 },
  { key: 'lsp.linked_editing_range_enabled', type: 'boolean', default: true, label: 'Linked Editing', description: 'Enable linked editing.', category: 'lsp', order: 10 },
  { key: 'lsp.call_hierarchy_enabled', type: 'boolean', default: true, label: 'Call Hierarchy', description: 'Enable call hierarchy.', category: 'lsp', order: 11 },
  { key: 'lsp.semantic_tokens_enabled', type: 'boolean', default: true, label: 'Semantic Tokens', description: 'Enable semantic highlighting.', category: 'lsp', order: 12 },
  { key: 'lsp.inlay_hints_enabled', type: 'boolean', default: true, label: 'Inlay Hints', description: 'Show inlay hints.', category: 'lsp', order: 13 },
  { key: 'lsp.inline_completion_enabled', type: 'boolean', default: false, label: 'Inline Completion', description: 'Enable inline completions.', category: 'lsp', order: 14 },

  // Extensions
  { key: 'extensions.auto_update', type: 'boolean', default: true, label: 'Auto Update', description: 'Automatically update extensions.', category: 'extensions', order: 1 },
  { key: 'extensions.auto_check_updates', type: 'boolean', default: true, label: 'Auto Check Updates', description: 'Check for extension updates.', category: 'extensions', order: 2 },
  { key: 'extensions.install_verification', type: 'boolean', default: true, label: 'Install Verification', description: 'Verify extension signatures.', category: 'extensions', order: 3 },
  { key: 'extensions.marketplace_url', type: 'string', default: 'https://open-vsx.org/api', label: 'Marketplace URL', description: 'Extension marketplace API URL.', category: 'extensions', order: 4 },
  { key: 'extensions.ignore_recommendations', type: 'boolean', default: false, label: 'Ignore Recommendations', description: 'Hide extension recommendations.', category: 'extensions', order: 5 },

  // Files
  { key: 'files.auto_save', type: 'enum', default: 'off', label: 'Auto Save', description: 'Auto save behavior.', category: 'files', order: 1, enum: ['off', 'afterDelay', 'onFocusChange', 'onWindowChange'], enumLabels: ['Off', 'After Delay', 'On Focus Change', 'On Window Change'] },
  { key: 'files.auto_save_delay', type: 'number', default: 1000, label: 'Auto Save Delay', description: 'Delay in ms before auto save.', category: 'files', order: 2, min: 100, max: 60000, step: 100 },
  { key: 'files.hot_exit', type: 'enum', default: 'onExit', label: 'Hot Exit', description: 'Save dirty files on exit.', category: 'files', order: 3, enum: ['onExit', 'off', 'onExitAndWindowClose'], enumLabels: ['On Exit', 'Off', 'On Exit and Window Close'] },
  { key: 'files.confirm_sync', type: 'boolean', default: true, label: 'Confirm Sync', description: 'Confirm file synchronization.', category: 'files', order: 4 },
  { key: 'files.enable_trash', type: 'boolean', default: true, label: 'Enable Trash', description: 'Move deleted files to trash.', category: 'files', order: 5 },
  { key: 'files.encoding', type: 'string', default: 'utf-8', label: 'Default Encoding', description: 'Default file encoding.', category: 'files', order: 6 },
  { key: 'files.guess_encoding', type: 'boolean', default: true, label: 'Guess Encoding', description: 'Auto-detect file encoding.', category: 'files', order: 7 },
  { key: 'files.eol', type: 'enum', default: 'lf', label: 'End of Line', description: 'Default end-of-line character.', category: 'files', order: 8, enum: ['lf', 'crlf'], enumLabels: ['LF (Unix)', 'CRLF (Windows)'] },
  { key: 'files.insert_final_newline', type: 'boolean', default: true, label: 'Insert Final Newline', description: 'Add newline at end of file.', category: 'files', order: 9 },
  { key: 'files.trim_final_newlines', type: 'boolean', default: true, label: 'Trim Final Newlines', description: 'Trim extra newlines at end.', category: 'files', order: 10 },
  { key: 'files.trim_trailing_whitespace', type: 'boolean', default: true, label: 'Trim Trailing Whitespace', description: 'Remove trailing whitespace on save.', category: 'files', order: 11 },

  // Search
  { key: 'search.follow_symlinks', type: 'boolean', default: false, label: 'Follow Symlinks', description: 'Follow symbolic links during search.', category: 'search', order: 1 },
  { key: 'search.use_ignore_files', type: 'boolean', default: true, label: 'Use Ignore Files', description: 'Respect .gitignore/.ignore files.', category: 'search', order: 2 },
  { key: 'search.use_global_ignore', type: 'boolean', default: true, label: 'Use Global Ignore', description: 'Use global ignore patterns.', category: 'search', order: 3 },
  { key: 'search.max_results', type: 'number', default: 10000, label: 'Max Results', description: 'Maximum search results.', category: 'search', order: 4, min: 100, max: 100000, step: 100 },
  { key: 'search.show_line_numbers', type: 'boolean', default: true, label: 'Show Line Numbers', description: 'Show line numbers in results.', category: 'search', order: 5 },
  { key: 'search.preview_enabled', type: 'boolean', default: true, label: 'Preview', description: 'Show preview of matches.', category: 'search', order: 6 },
  { key: 'search.match_whole_word', type: 'boolean', default: false, label: 'Match Whole Word', description: 'Match whole words only.', category: 'search', order: 7 },
  { key: 'search.match_case', type: 'boolean', default: false, label: 'Match Case', description: 'Case-sensitive search.', category: 'search', order: 8 },
  { key: 'search.use_regex', type: 'boolean', default: false, label: 'Use Regex', description: 'Enable regex search.', category: 'search', order: 9 },
  { key: 'search.smart_case', type: 'boolean', default: true, label: 'Smart Case', description: 'Smart case sensitivity.', category: 'search', order: 10 },
  { key: 'search.search_on_type', type: 'boolean', default: true, label: 'Search on Type', description: 'Search as you type.', category: 'search', order: 11 },
  { key: 'search.search_on_type_debounce', type: 'number', default: 300, label: 'Search Debounce', description: 'Debounce in ms for search on type.', category: 'search', order: 12, min: 50, max: 5000, step: 50 },
];

export const SettingsEditor: React.FC = () => {
  const [settings, setSettings] = useState<SettingValue>({});
  const [activeCategory, setActiveCategory] = useState('editor');
  const [searchFilter, setSearchFilter] = useState('');
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadSettings = useCallback(async () => {
    try {
      const result = await invoke<SettingValue>('get_settings');
      setSettings(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load settings');
    }
  }, []);

  const saveSettings = useCallback(async (key: string, value: unknown) => {
    setSaving(true);
    setError(null);
    try {
      const updates = { [key]: value };
      await invoke('update_settings', { partial: updates });
      setSettings(prev => ({ ...prev, [key]: value }));
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save setting');
      loadSettings(); // Revert on error
    } finally {
      setSaving(false);
    }
  }, [loadSettings]);

  const handleReset = async () => {
    setSaving(true);
    setError(null);
    try {
      await invoke('reset_settings');
      await loadSettings();
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to reset settings');
    } finally {
      setSaving(false);
    }
  };

  const handleOpenSettingsFile = async () => {
    try {
      await invoke('open_settings_file');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to open settings file');
    }
  };

  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  const categories = SETTING_CATEGORIES
    .map(cat => ({
      ...cat,
      settings: SETTINGS_SCHEMA
        .filter(s => s.category === cat.id)
        .sort((a, b) => (a.order ?? 0) - (b.order ?? 0))
        .filter(s => searchFilter === '' || 
          s.label.toLowerCase().includes(searchFilter.toLowerCase()) ||
          s.description.toLowerCase().includes(searchFilter.toLowerCase()) ||
          s.key.toLowerCase().includes(searchFilter.toLowerCase())
        ),
    }))
    .filter(cat => cat.settings.length > 0);

  const getNestedValue = (obj: SettingValue, path: string): unknown => {
    return path.split('.').reduce((o: unknown, k: string) => (o as SettingValue)?.[k], obj);
  };

  const _getCategorySettings = (categoryId: string) => {
    return SETTINGS_SCHEMA
      .filter(s => s.category === categoryId)
      .sort((a, b) => (a.order ?? 0) - (b.order ?? 0));
  };

  return (
    <div className="settings-editor">
      <div className="settings-header">
        <h1>Settings</h1>
        <div className="settings-actions">
          {saved && <span className="saved-indicator">Saved</span>}
          {error && <span className="error-indicator">{error}</span>}
          <button className="btn-secondary" onClick={handleOpenSettingsFile}>
            Open Settings File
          </button>
          <button className="btn-secondary" onClick={handleReset}>
            Reset to Defaults
          </button>
        </div>
      </div>

      <div className="settings-toolbar">
        <input
          type="text"
          placeholder="Search settings..."
          value={searchFilter}
          onChange={e => setSearchFilter(e.target.value)}
          className="search-input"
        />
      </div>

      <div className="settings-content">
        <nav className="settings-categories" role="navigation" aria-label="Settings categories">
          <ul>
            {categories.map(cat => (
              <li key={cat.id}>
                <button
                  className={`category-btn ${activeCategory === cat.id ? 'active' : ''}`}
                  onClick={() => setActiveCategory(cat.id)}
                >
                  {cat.label}
                </button>
              </li>
            ))}
          </ul>
        </nav>

        <div className="settings-detail">
          {categories.find(c => c.id === activeCategory)?.settings.map(setting => (
            <SettingRow
              key={setting.key}
              setting={setting}
              value={getNestedValue(settings, setting.key) ?? setting.default}
              onChange={saveSettings}
              disabled={saving}
            />
          ))}
        </div>
      </div>
    </div>
  );
};

interface SettingRowProps {
  setting: SettingDefinition;
  value: unknown;
  onChange: (key: string, value: unknown) => void;
  disabled: boolean;
}

const SettingRow: React.FC<SettingRowProps> = ({ setting, value, onChange, disabled }) => {
  const handleChange = (newValue: unknown) => {
    onChange(setting.key, newValue);
  };

  switch (setting.type) {
    case 'boolean':
      return (
        <div className="setting-row">
          <div className="setting-info">
            <label>{setting.label}</label>
            <span className="setting-description">{setting.description}</span>
          </div>
          <input
            type="checkbox"
            checked={value as boolean}
            onChange={e => handleChange(e.target.checked)}
            disabled={disabled}
          />
        </div>
      );

    case 'number':
      return (
        <div className="setting-row">
          <div className="setting-info">
            <label>{setting.label}</label>
            <span className="setting-description">{setting.description}</span>
          </div>
          <input
            type="number"
            value={value as number}
            onChange={e => handleChange(parseFloat(e.target.value))}
            min={setting.min}
            max={setting.max}
            step={setting.step ?? 1}
            disabled={disabled}
            style={{ width: '100px' }}
          />
        </div>
      );

    case 'enum':
      return (
        <div className="setting-row">
          <div className="setting-info">
            <label>{setting.label}</label>
            <span className="setting-description">{setting.description}</span>
          </div>
          <span className="setting-value" key={i}>{setting.enumLabels?.[i] ?? opt}</span>
        </div>
      );

    case 'string':
    default:
      return (
        <div className="setting-row">
          <div className="setting-info">
            <label>{setting.label}</label>
            <span className="setting-description">{setting.description}</span>
          </div>
          <input
            type="text"
            value={value as string}
            onChange={e => handleChange(e.target.value)}
            disabled={disabled}
            style={{ width: '300px' }}
          />
        </div>
      );
  }
};

export default SettingsEditor;