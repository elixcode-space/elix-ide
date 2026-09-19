/**
 * File Explorer Commands for VS Code Workbench
 *
 * Registers commands to open the custom File Explorer panel
 */

import { registerAction2, Action2, MenuId } from '@codingame/monaco-vscode-api/vscode/vs/platform/actions/common/actions';
import { KeyCode, KeyMod } from '@codingame/monaco-vscode-api/vscode/vs/base/common/keyCodes';
import { KeybindingWeight } from '@codingame/monaco-vscode-api/vscode/vs/platform/keybinding/common/keybindingsRegistry';
import type { ServicesAccessor } from '@codingame/monaco-vscode-api/vscode/vs/platform/instantiation/common/instantiation';
import { IWorkspaceContextService } from '@codingame/monaco-vscode-api/vscode/vs/platform/workspace/common/workspace.service';

import { FileExplorer } from '../../components/FileExplorer';
import { createRoot } from 'react-dom/client';
import * as React from 'react';

let _registered = false;

/**
 * Open File Explorer Panel Action
 */
class TauriOpenFileExplorerAction extends Action2 {
  constructor() {
    super({
      id: 'tauri.openFileExplorer',
      title: { value: 'File Explorer', original: 'File Explorer' },
      category: { value: 'View', original: 'View' },
      f1: true,
      keybinding: {
        weight: KeybindingWeight.WorkbenchContrib,
        primary: KeyMod.CtrlCmd | KeyMod.Shift | KeyCode.KeyE,
      },
      menu: {
        id: MenuId.View,
        group: 'explorer',
        order: 1,
      },
    });
  }

  async run(accessor: ServicesAccessor): Promise<void> {
    const workspaceService = accessor.get(IWorkspaceContextService);
    const workspace = workspaceService.getWorkspace();
    const folderPath = workspace?.folders[0]?.uri.fsPath || '';

    // Create a floating panel for File Explorer
    const existingPanel = document.getElementById('tauri-file-explorer-panel');
    if (existingPanel) {
      existingPanel.remove();
      return;
    }

    const panel = document.createElement('div');
    panel.id = 'tauri-file-explorer-panel';
    panel.style.cssText = `
      position: fixed;
      top: 60px;
      left: 20px;
      width: 300px;
      height: 80%;
      max-height: 800px;
      background: var(--vscode-sideBar-background, #1e1e1e);
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
      React.createElement(FileExplorer, {
        workspacePath: folderPath,
        onFileOpen: (path) => {
          // Open file in editor
          console.log('Open file:', path);
        },
        onClose: () => {
          root.unmount();
          panel.remove();
        }
      })
    );
  }
}

/**
 * Register the File Explorer commands
 */
export function registerFileExplorer(): void {
  if (_registered) return;
  _registered = true;

  registerAction2(TauriOpenFileExplorerAction);

  console.log('[FileExplorerCommands] File Explorer command registered');
}