/**
 * Search Commands for VS Code Workbench
 *
 * Registers commands to open the custom Search panel
 */

import { registerAction2, Action2, MenuId } from '@codingame/monaco-vscode-api/vscode/vs/platform/actions/common/actions';
import { KeyCode, KeyMod } from '@codingame/monaco-vscode-api/vscode/vs/base/common/keyCodes';
import { KeybindingWeight } from '@codingame/monaco-vscode-api/vscode/vs/platform/keybinding/common/keybindingsRegistry';
import type { ServicesAccessor } from '@codingame/monaco-vscode-api/vscode/vs/platform/instantiation/common/instantiation';
import { IWorkspaceContextService } from '@codingame/monaco-vscode-api/vscode/vs/platform/workspace/common/workspace.service';

import { SearchPanel } from '../../components/SearchPanel';
import { createRoot } from 'react-dom/client';
import * as React from 'react';

let _registered = false;

/**
 * Open Search Panel Action - opens Search panel
 */
class TauriOpenSearchPanelAction extends Action2 {
  constructor() {
    super({
      id: 'tauri.openSearchPanel',
      title: { value: 'Search', original: 'Search' },
      category: { value: 'View', original: 'View' },
      f1: true,
      keybinding: {
        weight: KeybindingWeight.WorkbenchContrib,
        primary: KeyMod.CtrlCmd | KeyMod.Shift | KeyCode.KeyF,
      },
      menu: {
        id: MenuId.View,
        group: 'search',
        order: 1,
      },
    });
  }

  async run(accessor: ServicesAccessor): Promise<void> {
    const workspaceService = accessor.get(IWorkspaceContextService);
    const workspace = workspaceService.getWorkspace();
    const folderPath = workspace?.folders[0]?.uri.fsPath || '';

    // Create a floating panel for Search
    const existingPanel = document.getElementById('tauri-search-panel');
    if (existingPanel) {
      existingPanel.remove();
      return;
    }

    const panel = document.createElement('div');
    panel.id = 'tauri-search-panel';
    panel.style.cssText = `
      position: fixed;
      top: 60px;
      right: 20px;
      width: 550px;
      height: 80%;
      max-height: 800px;
      background: var(--vscode-editor-background, #1e1e1e);
      border: 1px solid var(--vscode-panel-border, #3c3c3c);
      border-radius: 6px;
      box-shadow: 0 8px 32px rgba(0,0,0,0.4);
      z-index: 10000;
      font-family: var(--vscode-font-family, -apple-system, BlinkMacSystemFont, 'Segoe UI', monospace);
      display: flex;
      flex-direction: column;
    `;
    document.body.appendChild(panel);

    const root = createRoot(panel);
    root.render(
      React.createElement(SearchPanel, {
        workspacePath: folderPath,
        onClose: () => {
          root.unmount();
          panel.remove();
        }
      })
    );
  }
}

/**
 * Register the Search commands
 */
export function registerSearchPanel(): void {
  if (_registered) return;
  _registered = true;

  registerAction2(TauriOpenSearchPanelAction);

  console.log('[SearchCommands] Search panel command registered');
}