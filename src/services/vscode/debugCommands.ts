/**
 * Debug Commands for VS Code Workbench
 */

import { registerAction2, Action2, MenuId } from '@codingame/monaco-vscode-api/vscode/vs/platform/actions/common/actions';
import { KeyCode, KeyMod } from '@codingame/monaco-vscode-api/vscode/vs/base/common/keyCodes';
import { KeybindingWeight } from '@codingame/monaco-vscode-api/vscode/vs/platform/keybinding/common/keybindingsRegistry';
import type { ServicesAccessor } from '@codingame/monaco-vscode-api/vscode/vs/platform/instantiation/common/instantiation';
import { IWorkspaceContextService } from '@codingame/monaco-vscode-api/vscode/vs/platform/workspace/common/workspace.service';

import { DebugPanel } from '../../components/DebugPanel';
import { createRoot } from 'react-dom/client';
import * as React from 'react';

let _registered = false;

class TauriOpenDebugPanelAction extends Action2 {
  constructor() {
    super({
      id: 'tauri.openDebugPanel',
      title: { value: 'Debug', original: 'Debug' },
      category: { value: 'View', original: 'View' },
      f1: true,
      keybinding: {
        weight: KeybindingWeight.WorkbenchContrib,
        primary: KeyMod.CtrlCmd | KeyMod.Shift | KeyCode.KeyD,
      },
      menu: {
        id: MenuId.View,
        group: 'debug',
        order: 1,
      },
    });
  }

  async run(accessor: ServicesAccessor): Promise<void> {
    const workspaceService = accessor.get(IWorkspaceContextService);
    const workspace = workspaceService.getWorkspace();
    const folderPath = workspace?.folders[0]?.uri.fsPath || '';
    
    if (!folderPath) {
      console.warn('[DebugCommands] No workspace folder open');
      return;
    }

    const existingPanel = document.getElementById('tauri-debug-panel');
    if (existingPanel) {
      existingPanel.remove();
      return;
    }

    const panel = document.createElement('div');
    panel.id = 'tauri-debug-panel';
    panel.style.cssText = `
      position: fixed;
      top: 60px;
      right: 20px;
      width: 800px;
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
      React.createElement(DebugPanel, {})
    );
  }
}

export function registerDebugPanel(): void {
  if (_registered) return;
  _registered = true;

  registerAction2(TauriOpenDebugPanelAction);

  console.log('[DebugCommands] Debug panel command registered');
}