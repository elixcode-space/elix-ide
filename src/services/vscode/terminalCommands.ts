/**
 * Terminal Commands for VS Code Workbench
 *
 * Registers commands to open the custom Terminal panel
 */

import { registerAction2, Action2, MenuId } from '@codingame/monaco-vscode-api/vscode/vs/platform/actions/common/actions';
import { KeyCode, KeyMod } from '@codingame/monaco-vscode-api/vscode/vs/base/common/keyCodes';
import { KeybindingWeight } from '@codingame/monaco-vscode-api/vscode/vs/platform/keybinding/common/keybindingsRegistry';
import type { ServicesAccessor } from '@codingame/monaco-vscode-api/vscode/vs/platform/instantiation/common/instantiation';
import { IWorkspaceContextService } from '@codingame/monaco-vscode-api/vscode/vs/platform/workspace/common/workspace.service';

import { TerminalPanel } from '../../components/TerminalPanel';
import { createRoot } from 'react-dom/client';
import * as React from 'react';

let _registered = false;

/**
 * Open Terminal Panel Action - opens Terminal panel
 */
class TauriOpenTerminalPanelAction extends Action2 {
  constructor() {
    super({
      id: 'tauri.openTerminalPanel',
      title: { value: 'Terminal', original: 'Terminal' },
      category: { value: 'View', original: 'View' },
      f1: true,
      keybinding: {
        weight: KeybindingWeight.WorkbenchContrib,
        primary: KeyMod.CtrlCmd | KeyMod.Shift | KeyCode.Backquote,
      },
      menu: {
        id: MenuId.View,
        group: 'terminal',
        order: 1,
      },
    });
  }

  async run(accessor: ServicesAccessor): Promise<void> {
    const workspaceService = accessor.get(IWorkspaceContextService);
    const workspace = workspaceService.getWorkspace();
    const folderPath = workspace?.folders[0]?.uri.fsPath || '';

    // Create a floating panel for Terminal
    const existingPanel = document.getElementById('tauri-terminal-panel');
    if (existingPanel) {
      existingPanel.remove();
      return;
    }

    const panel = document.createElement('div');
    panel.id = 'tauri-terminal-panel';
    panel.style.cssText = `
      position: fixed;
      bottom: 0;
      left: 0;
      right: 0;
      height: 40%;
      min-height: 200px;
      background: var(--vscode-editor-background, #1e1e1e);
      border-top: 1px solid var(--vscode-panel-border, #3c3c3c);
      border-radius: 6px 6px 0 0;
      box-shadow: 0 -8px 32px rgba(0,0,0,0.4);
      z-index: 10000;
      font-family: var(--vscode-font-family, -apple-system, BlinkMacSystemFont, 'Segoe UI', monospace);
      display: flex;
      flex-direction: column;
    `;
    document.body.appendChild(panel);

    const root = createRoot(panel);
    root.render(
      React.createElement(TerminalPanel, {
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
 * Register the Terminal commands
 */
export function registerTerminalPanel(): void {
  if (_registered) return;
  _registered = true;

  registerAction2(TauriOpenTerminalPanelAction);

  console.log('[TerminalCommands] Terminal panel command registered');
}