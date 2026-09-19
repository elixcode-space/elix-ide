/**
 * Extension Commands
 *
 * VS Code commands for managing extensions including:
 * - Installing VSIX files from disk
 * - Managing extension lifecycle
 */

import { registerAction2, Action2, MenuId } from '@codingame/monaco-vscode-api/vscode/vs/platform/actions/common/actions';
import { KeybindingWeight } from '@codingame/monaco-vscode-api/vscode/vs/platform/keybinding/common/keybindingsRegistry';
import type { ServicesAccessor } from '@codingame/monaco-vscode-api/vscode/vs/platform/instantiation/common/instantiation';
import { INotificationService } from '@codingame/monaco-vscode-api/vscode/vs/platform/notification/common/notification.service';
import Severity from '@codingame/monaco-vscode-api/vscode/vs/base/common/severity';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { getExtensionManager } from '../extensions/extensionManager';
import { extensionHostService } from './extensionHostService';
import { createRoot } from 'react-dom/client';
import { ExtensionMarketplace } from '../../components/ExtensionMarketplace';

/**
 * Install a VSIX file from disk
 */
async function installVsixFromDisk(notificationService: INotificationService): Promise<void> {
  try {
    // Open file picker for VSIX files
    const selected = await openDialog({
      multiple: false,
      title: 'Install Extension from VSIX',
      filters: [{
        name: 'VS Code Extension',
        extensions: ['vsix']
      }]
    });

    if (!selected || typeof selected !== 'string') {
      return; // User cancelled
    }

    console.log('[ExtensionCommands] Installing VSIX:', selected);

    // Show progress notification
    notificationService.info(`Installing extension from ${selected.split('/').pop()}...`);

    // Install via extension manager
    const manager = getExtensionManager();
    await manager.initialize();
    const info = await manager.installFromPath(selected);

    console.log('[ExtensionCommands] Extension installed:', info.id);

    // If it's a Node.js extension, activate it in the sidecar
    if (info.manifest.main && !info.manifest.browser) {
      console.log('[ExtensionCommands] Activating Node.js extension in sidecar:', info.id);
      try {
        const ready = await extensionHostService.checkReady();
        if (ready) {
          await extensionHostService.activateExtension(info.extensionPath, info.id);
          notificationService.notify({
            severity: Severity.Info,
            message: `Extension "${info.displayName}" installed and activated successfully.`
          });
        } else {
          notificationService.notify({
            severity: Severity.Warning,
            message: `Extension "${info.displayName}" installed. Restart to activate Node.js extensions.`
          });
        }
      } catch (err) {
        console.error('[ExtensionCommands] Failed to activate in sidecar:', err);
        notificationService.notify({
          severity: Severity.Warning,
          message: `Extension "${info.displayName}" installed but activation failed. Try restarting.`
        });
      }
    } else {
      // Browser extension - may need reload
      notificationService.info(`Extension "${info.displayName}" installed. Reload window to activate.`);
    }
  } catch (err) {
    console.error('[ExtensionCommands] Failed to install VSIX:', err);
    notificationService.error(`Failed to install extension: ${err instanceof Error ? err.message : String(err)}`);
  }
}

/**
 * Show Extensions Marketplace Panel
 */
async function showExtensionsMarketplace(): Promise<void> {
  const existingPanel = document.getElementById('tauri-extensions-marketplace-panel');
  if (existingPanel) {
    existingPanel.focus();
    return;
  }

  const panel = document.createElement('div');
  panel.id = 'tauri-extensions-marketplace-panel';
  panel.style.cssText = `
    position: fixed;
    top: 20%;
    left: 20%;
    width: 60%;
    height: 60%;
    min-width: 800px;
    min-height: 500px;
    background: var(--vscode-editor-background, #1e1e1e);
    border: 1px solid var(--vscode-panel-border, #3c3c3c);
    border-radius: 8px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.4);
    z-index: 10000;
    display: flex;
    flex-direction: column;
    font-family: var(--vscode-font-family, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif);
    color: var(--vscode-editor-foreground, #cccccc);
  `;

  // Make panel draggable
  let isDragging = false;
  let dragStartX = 0;
  let dragStartY = 0;
  let panelStartX = 0;
  let panelStartY = 0;

  const handleMouseDown = (e: MouseEvent) => {
    if (e.target === panel || (e.target as HTMLElement).closest?.('#tauri-extensions-marketplace-panel') === panel) {
      isDragging = true;
      dragStartX = e.clientX;
      dragStartY = e.clientY;
      panelStartX = panel.offsetLeft;
      panelStartY = panel.offsetTop;
      e.preventDefault();
    }
  };

  const handleMouseMove = (e: MouseEvent) => {
    if (isDragging) {
      panel.style.left = `${panelStartX + e.clientX - dragStartX}px`;
      panel.style.top = `${panelStartY + e.clientY - dragStartY}px`;
    }
  };

  const handleMouseUp = () => {
    isDragging = false;
  };

  panel.addEventListener('mousedown', handleMouseDown);
  document.addEventListener('mousemove', handleMouseMove);
  document.addEventListener('mouseup', handleMouseUp);

  const closePanel = () => {
    document.removeEventListener('mousemove', handleMouseMove);
    document.removeEventListener('mouseup', handleMouseUp);
    panel.remove();
  };

  const header = document.createElement('div');
  header.style.cssText = `
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid var(--vscode-panel-border, #3c3c3c);
    cursor: move;
    user-select: none;
    background: var(--vscode-titleBar-activeBackground, #007acc);
  `;
  header.innerHTML = `
    <span style="font-weight: 600; font-size: 14px;">Extensions Marketplace</span>
    <button style="
      background: none;
      border: none;
      color: var(--vscode-titleBar-activeForeground, #fff);
      font-size: 18px;
      cursor: pointer;
      padding: 4px 8px;
      line-height: 1;
    ">✕</button>
  `;
  header.querySelector('button')?.addEventListener('click', closePanel);
  panel.appendChild(header);

  const content = document.createElement('div');
  content.style.cssText = `
    flex: 1;
    overflow: hidden;
  `;
  panel.appendChild(content);

  document.body.appendChild(panel);

  const root = createRoot(content);
  root.render(<ExtensionMarketplace />);
}

/**
 * Show Extensions Marketplace Action
 */
class ShowExtensionsMarketplaceAction extends Action2 {
  constructor() {
    super({
      id: 'extensions.showMarketplace',
      title: { value: 'Extensions: Show Marketplace', original: 'Extensions: Show Marketplace' },
      category: { value: 'Extensions', original: 'Extensions' },
      f1: true,
      keybinding: {
        weight: KeybindingWeight.WorkbenchContrib,
        primary: 0,
      },
      menu: {
        id: MenuId.MenubarViewMenu,
        group: '4_panels',
        order: 2,
      },
    });
  }

  async run(): Promise<void> {
    await showExtensionsMarketplace();
  }
}

/**
 * Install Extension from VSIX Action
 */
class InstallVsixAction extends Action2 {
  constructor() {
    super({
      id: 'extensions.installFromVSIX',
      title: { value: 'Install from VSIX...', original: 'Install from VSIX...' },
      category: { value: 'Extensions', original: 'Extensions' },
      f1: true,
      keybinding: {
        weight: KeybindingWeight.WorkbenchContrib,
        primary: 0, // No default keybinding
      },
      menu: {
        id: MenuId.MenubarPreferencesMenu,
        group: '2_extensions',
        order: 1,
      },
    });
  }

  async run(accessor: ServicesAccessor): Promise<void> {
    const notificationService = accessor.get(INotificationService);
    await installVsixFromDisk(notificationService);
  }
}

/**
 * Register extension-related commands
 */
let _registered = false;

export function registerExtensionCommands(): void {
  if (_registered) return;
  _registered = true;

  registerAction2(ShowExtensionsMarketplaceAction);
  registerAction2(InstallVsixAction);

  console.log('[ExtensionCommands] Extension commands registered');
}
