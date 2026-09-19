/**
 * ElixirIDE Dark Theme for VS Code Workbench
 *
 * Custom dark theme with ElixirIDE brand colors.
 * Primary: ElixirIDE Red (#C74634)
 * Secondary: ElixirIDE Yellow (#F5A623) for highlights
 */

// ElixirIDE brand colors
const ELIXIDE_RED = '#C74634';
const ELIXIDE_RED_LIGHT = '#E85A4F';
const ELIXIDE_RED_DARK = '#A33D2E';
const ELIXIDE_YELLOW = '#F5A623';
const ELIXIDE_YELLOW_LIGHT = '#FFB84D';

// Dark theme base colors
const DARK_FG = '#D4D4D4';
const DARK_BORDER = '#3C3C3C';

/**
 * Generate CSS overrides for ElixirIDE Dark theme
 */
export function getElixirIDEThemeCSS(): string {
  return `
/* ElixirIDE Dark Theme */

/* CSS Variables */
:root {
  --elixide-red: ${ELIXIDE_RED};
  --elixide-red-light: ${ELIXIDE_RED_LIGHT};
  --elixide-red-dark: ${ELIXIDE_RED_DARK};
  --elixide-yellow: ${ELIXIDE_YELLOW};
  --elixide-yellow-light: ${ELIXIDE_YELLOW_LIGHT};

  /* Focus & Borders */
  --vscode-focusBorder: ${ELIXIDE_RED};
  --vscode-contrastBorder: transparent;
  --vscode-contrastActiveBorder: ${ELIXIDE_RED};

  /* Buttons */
  --vscode-button-background: ${ELIXIDE_RED};
  --vscode-button-foreground: #ffffff;
  --vscode-button-hoverBackground: ${ELIXIDE_RED_LIGHT};
  --vscode-button-secondaryBackground: ${DARK_BORDER};
  --vscode-button-secondaryForeground: ${DARK_FG};

  /* Progress */
  --vscode-progressBar-background: ${ELIXIDE_RED};

  /* Badges */
  --vscode-badge-background: ${ELIXIDE_RED};
  --vscode-badge-foreground: #ffffff;

  /* Activity Bar */
  --vscode-activityBar-foreground: ${DARK_FG};
  --vscode-activityBar-activeBorder: ${ELIXIDE_RED};
  --vscode-activityBar-activeBackground: #2D2D2D;
  --vscode-activityBarBadge-background: ${ELIXIDE_RED};
  --vscode-activityBarBadge-foreground: #ffffff;

  /* Tabs */
  --vscode-tab-activeBorderTop: ${ELIXIDE_RED};
  --vscode-tab-unfocusedActiveBorderTop: ${ELIXIDE_RED_DARK};

  /* Panel */
  --vscode-panelTitle-activeBorder: ${ELIXIDE_RED};

  /* Links */
  --vscode-textLink-foreground: ${ELIXIDE_RED};
  --vscode-textLink-activeForeground: ${ELIXIDE_RED_LIGHT};

  /* Editor */
  --vscode-editorCursor-foreground: ${ELIXIDE_YELLOW};
  --vscode-editor-findMatchBackground: ${ELIXIDE_YELLOW}66;
  --vscode-editor-findMatchHighlightBackground: ${ELIXIDE_YELLOW}33;

  /* List */
  --vscode-list-highlightForeground: ${ELIXIDE_YELLOW};
  --vscode-list-focusHighlightForeground: ${ELIXIDE_YELLOW_LIGHT};

  /* Input */
  --vscode-inputOption-activeBorder: ${ELIXIDE_RED};
  --vscode-inputOption-activeBackground: ${ELIXIDE_RED}66;
  --vscode-inputValidation-errorBorder: ${ELIXIDE_RED};

  /* Scrollbar */
  --vscode-scrollbarSlider-activeBackground: ${ELIXIDE_RED}99;

  /* Sash (panel dividers) */
  --vscode-sash-hoverBorder: ${ELIXIDE_YELLOW};
  --vscode-sash-activeBackground: ${ELIXIDE_YELLOW};
  --vscode-sash-activeBorder: ${ELIXIDE_YELLOW};

  /* Extensions */
  --vscode-extensionButton-prominentBackground: ${ELIXIDE_RED};
  --vscode-extensionButton-prominentForeground: #ffffff;
  --vscode-extensionButton-prominentHoverBackground: ${ELIXIDE_RED_LIGHT};

  /* Checkbox */
  --vscode-checkbox-background: ${DARK_BORDER};
  --vscode-checkbox-selectBackground: ${ELIXIDE_RED};
  --vscode-checkbox-selectBorder: ${ELIXIDE_RED};

  /* Settings */
  --vscode-settings-modifiedItemIndicator: ${ELIXIDE_RED};

  /* Status Bar */
  --vscode-statusBar-background: #181818;
  --vscode-statusBar-foreground: ${DARK_FG};
  --vscode-statusBar-border: ${DARK_BORDER};
  --vscode-statusBar-noFolderBackground: #181818;
  --vscode-statusBar-noFolderForeground: ${DARK_FG};
  --vscode-statusBar-debuggingBackground: ${ELIXIDE_RED};
  --vscode-statusBar-debuggingForeground: #ffffff;
  --vscode-statusBarItem-activeBackground: ${ELIXIDE_RED}66;
  --vscode-statusBarItem-hoverBackground: #3C3C3C;
  --vscode-statusBarItem-prominentBackground: ${ELIXIDE_RED};
  --vscode-statusBarItem-prominentForeground: #ffffff;
  --vscode-statusBarItem-prominentHoverBackground: ${ELIXIDE_RED_LIGHT};
  --vscode-statusBarItem-remoteBackground: ${ELIXIDE_RED};
  --vscode-statusBarItem-remoteForeground: #ffffff;
  --vscode-statusBarItem-errorBackground: ${ELIXIDE_RED};
  --vscode-statusBarItem-errorForeground: #ffffff;
  --vscode-statusBarItem-warningBackground: ${ELIXIDE_YELLOW};
  --vscode-statusBarItem-warningForeground: #000000;

  /* Selection & Focus (blue -> yellow) */
  --vscode-editor-selectionBackground: ${ELIXIDE_YELLOW}44;
  --vscode-editor-selectionHighlightBackground: ${ELIXIDE_YELLOW}33;
  --vscode-editor-inactiveSelectionBackground: ${ELIXIDE_YELLOW}22;
  --vscode-editor-wordHighlightBackground: ${ELIXIDE_YELLOW}33;
  --vscode-editor-wordHighlightStrongBackground: ${ELIXIDE_YELLOW}44;
  --vscode-editor-wordHighlightTextBackground: ${ELIXIDE_YELLOW}33;

  /* List Selection (blue -> yellow) */
  --vscode-list-activeSelectionBackground: ${ELIXIDE_YELLOW}33;
  --vscode-list-activeSelectionForeground: #ffffff;
  --vscode-list-inactiveSelectionBackground: ${ELIXIDE_YELLOW}22;
  --vscode-list-focusBackground: ${ELIXIDE_YELLOW}33;
  --vscode-list-focusOutline: ${ELIXIDE_YELLOW};
  --vscode-list-focusAndSelectionOutline: ${ELIXIDE_YELLOW};

  /* Quick Input */
  --vscode-quickInput-list-focusBackground: ${ELIXIDE_YELLOW}33;
  --vscode-quickInputList-focusBackground: ${ELIXIDE_YELLOW}33;

  /* Editor Highlight */
  --vscode-editor-hoverHighlightBackground: ${ELIXIDE_YELLOW}22;
  --vscode-editor-lineHighlightBackground: ${ELIXIDE_YELLOW}11;
  --vscode-editor-rangeHighlightBackground: ${ELIXIDE_YELLOW}22;

  /* Peek View */
  --vscode-peekView-border: ${ELIXIDE_YELLOW};
  --vscode-peekViewEditor-matchHighlightBackground: ${ELIXIDE_YELLOW}44;
  --vscode-peekViewResult-matchHighlightBackground: ${ELIXIDE_YELLOW}44;
  --vscode-peekViewResult-selectionBackground: ${ELIXIDE_YELLOW}33;

  /* Editor Widget */
  --vscode-editorWidget-border: ${ELIXIDE_YELLOW}66;

  /* Suggest Widget */
  --vscode-editorSuggestWidget-selectedBackground: ${ELIXIDE_YELLOW}33;
  --vscode-editorSuggestWidget-focusHighlightForeground: ${ELIXIDE_YELLOW};
  --vscode-editorSuggestWidget-highlightForeground: ${ELIXIDE_YELLOW};

  /* Menu */
  --vscode-menu-selectionBackground: ${ELIXIDE_YELLOW}33;

  /* Dropdown */
  --vscode-dropdown-listBackground: #252526;

  /* Tree indent guides */
  --vscode-tree-indentGuidesStroke: ${ELIXIDE_YELLOW}44;

  /* Minimap */
  --vscode-minimap-selectionHighlight: ${ELIXIDE_YELLOW}66;
  --vscode-minimap-findMatchHighlight: ${ELIXIDE_YELLOW};

  /* Editor marker navigation */
  --vscode-editorMarkerNavigation-background: #252526;

  /* Bracket match */
  --vscode-editorBracketMatch-background: ${ELIXIDE_YELLOW}33;
  --vscode-editorBracketMatch-border: ${ELIXIDE_YELLOW};
}

/* Buttons */
.monaco-button,
.monaco-text-button,
.monaco-workbench .monaco-button,
.monaco-workbench .monaco-text-button {
  background-color: ${ELIXIDE_RED} !important;
  color: #ffffff !important;
  border: none !important;
}

.monaco-button:hover,
.monaco-text-button:hover,
.monaco-workbench .monaco-button:hover,
.monaco-workbench .monaco-text-button:hover {
  background-color: ${ELIXIDE_RED_LIGHT} !important;
}

.monaco-button.secondary,
.monaco-workbench .monaco-button.secondary {
  background-color: ${DARK_BORDER} !important;
  color: ${DARK_FG} !important;
}

/* Activity Bar */
.monaco-workbench .activitybar > .content :not(.monaco-menu) > .monaco-action-bar .action-item.checked .active-item-indicator:before {
  border-left-color: ${ELIXIDE_RED} !important;
}

.monaco-workbench .activitybar .badge {
  background-color: ${ELIXIDE_RED} !important;
  color: #ffffff !important;
}

/* Tabs */
.monaco-workbench .part.editor > .content .editor-group-container > .title .tabs-container > .tab.active {
  border-top: 2px solid ${ELIXIDE_RED} !important;
}

/* Panel */
.monaco-workbench .part.panel > .title > .panel-switcher-container > .monaco-action-bar .action-item.checked .action-label {
  border-bottom-color: ${ELIXIDE_RED} !important;
}

/* Progress Bar */
.monaco-workbench .monaco-progress-container .progress-bit {
  background-color: ${ELIXIDE_RED} !important;
}

/* Links */
a,
.monaco-link,
.monaco-workbench a,
.monaco-workbench .monaco-link {
  color: ${ELIXIDE_RED} !important;
}

a:hover,
.monaco-link:hover,
.monaco-workbench a:hover,
.monaco-workbench .monaco-link:hover {
  color: ${ELIXIDE_RED_LIGHT} !important;
}

/* Focus Border */
.monaco-workbench *:focus {
  outline-color: ${ELIXIDE_RED} !important;
}

/* Input Focus */
.monaco-workbench .monaco-inputbox.synthetic-focus,
.monaco-workbench .monaco-inputbox:focus-within {
  border-color: ${ELIXIDE_RED} !important;
}

/* Cursor */
.monaco-editor .cursor {
  background-color: ${ELIXIDE_YELLOW} !important;
  border-color: ${ELIXIDE_YELLOW} !important;
}

/* Find Match */
.monaco-editor .findMatch {
  background-color: ${ELIXIDE_YELLOW}66 !important;
}

.monaco-editor .currentFindMatch {
  background-color: ${ELIXIDE_YELLOW}99 !important;
}

/* List Highlight */
.monaco-list .monaco-highlighted-label .highlight,
.monaco-workbench .monaco-list .monaco-highlighted-label .highlight {
  color: ${ELIXIDE_YELLOW} !important;
}

/* Pane header titles */
.monaco-pane-view .pane > .pane-header h3.title,
.monaco-workbench .monaco-pane-view .pane > .pane-header h3.title {
  color: ${ELIXIDE_RED} !important;
}

/* Sash (panel dividers) - only on hover/active */
.monaco-sash.hover,
.monaco-sash.active,
.monaco-workbench .monaco-sash.hover,
.monaco-workbench .monaco-sash.active {
  background-color: ${ELIXIDE_YELLOW} !important;
}

.monaco-sash.hover:before,
.monaco-sash.hover:after,
.monaco-sash.active:before,
.monaco-sash.active:after,
.monaco-workbench .monaco-sash.hover:before,
.monaco-workbench .monaco-sash.hover:after,
.monaco-workbench .monaco-sash.active:before,
.monaco-workbench .monaco-sash.active:after {
  background-color: ${ELIXIDE_YELLOW} !important;
}

/* Scrollbar Active */
.monaco-scrollable-element > .scrollbar > .slider:active {
  background-color: ${ELIXIDE_RED}99 !important;
}

/* Status Bar Remote */
.monaco-workbench .part.statusbar > .items-container > .statusbar-item.remote-kind {
  background-color: ${ELIXIDE_RED} !important;
  color: #ffffff !important;
}

/* Extension Button */
.extension-editor .monaco-button,
.extensions-list .monaco-button {
  background-color: ${ELIXIDE_RED} !important;
  color: #ffffff !important;
}

/* Checkbox checked */
.monaco-custom-checkbox.checked:before,
.monaco-workbench .monaco-custom-checkbox.checked:before {
  background-color: ${ELIXIDE_RED} !important;
  border-color: ${ELIXIDE_RED} !important;
}

/* Settings modified indicator */
.monaco-workbench .settings-editor .setting-item-modified-indicator {
  background-color: ${ELIXIDE_RED} !important;
}

/* Notification buttons */
.monaco-workbench .notifications-list-container .notification-list-item .notification-list-item-buttons-container .monaco-button {
  background-color: ${ELIXIDE_RED} !important;
  color: #ffffff !important;
}

/* Status Bar */
.monaco-workbench .part.statusbar {
  background-color: #181818 !important;
  color: ${DARK_FG} !important;
  border-top: 1px solid ${DARK_BORDER} !important;
}

.monaco-workbench .part.statusbar.no-folder-workspace {
  background-color: #181818 !important;
}

/* List Focus Outline */
.monaco-workbench .monaco-list:focus:before,
.monaco-workbench .monaco-list:not(.element-focused):focus:before,
.monaco-workbench .monaco-list:not(.element-focused):not(:active):focus:before {
  outline-color: ${ELIXIDE_RED} !important;
}

/* Any remaining blue selection backgrounds */
.monaco-workbench .monaco-list .monaco-list-row.selected,
.monaco-workbench .monaco-list .monaco-list-row.focused {
  background-color: ${ELIXIDE_YELLOW}33 !important;
}

.monaco-workbench .monaco-list .monaco-list-row.selected.focused {
  background-color: ${ELIXIDE_YELLOW}44 !important;
}
`;
}

export { ELIXIDE_RED, ELIXIDE_RED_LIGHT, ELIXIDE_RED_DARK, ELIXIDE_YELLOW, ELIXIDE_YELLOW_LIGHT };
