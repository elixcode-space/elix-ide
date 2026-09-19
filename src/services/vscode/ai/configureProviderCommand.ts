/**
 * Configure AI Provider Command
 *
 * Registers the 'elixide.configureAIProvider' VSCode command.
 * Guides the user through picking a provider, entering an API key,
 * selecting a model, and (for custom providers) entering a base URL.
 */

import {
  setAIProviderConfig,
  getAIProviderConfig,
  clearAIProviderConfig,
  PROVIDER_DEFAULTS,
  type ProviderType,
} from './aiProviderService';

let _registered = false;

export async function registerConfigureProviderCommand(): Promise<void> {
  if (_registered) return;
  _registered = true;

  try {
    const { commands, window } = await import('vscode');

    commands.registerCommand('elixide.configureAIProvider', async () => {
      // Step 1: Pick provider type
      const providerItems = [
        {
          label: 'Anthropic',
          description: 'Claude models (claude-opus-4-6, claude-sonnet-4-6, etc.)',
          value: 'anthropic' as ProviderType,
        },
        {
          label: 'OpenAI',
          description: 'GPT models (gpt-4o, gpt-4o-mini, etc.)',
          value: 'openai' as ProviderType,
        },
        {
          label: 'Custom (OpenAI-compatible)',
          description: 'Any provider with an OpenAI-compatible API',
          value: 'custom' as ProviderType,
        },
        {
          label: 'Clear / Disconnect',
          description: 'Remove the current AI provider configuration',
          value: '__clear__' as ProviderType | '__clear__',
        },
      ];

      const current = getAIProviderConfig();
      const picked = await window.showQuickPick(providerItems, {
        placeHolder: current
          ? `Current: ${PROVIDER_DEFAULTS[current.type]?.label ?? current.type} — ${current.model}`
          : 'Select an AI provider',
        title: 'ElixirIDE: Configure AI Provider',
      });

      if (!picked) return;

      if (picked.value === '__clear__') {
        clearAIProviderConfig();
        window.showInformationMessage('ElixirIDE: AI provider configuration cleared.');
        return;
      }

      const providerType = picked.value as ProviderType;
      const defaults = PROVIDER_DEFAULTS[providerType];

      // Step 2: Enter API key
      const apiKey = await window.showInputBox({
        title: `ElixirIDE: ${defaults.label} API Key`,
        prompt: `Paste your ${defaults.label} API key`,
        password: true,
        value: current?.type === providerType ? current.apiKey : '',
        validateInput: (v) => (v.trim() ? null : 'API key cannot be empty'),
      });

      if (apiKey === undefined) return; // cancelled

      // Step 3: Model
      const modelValue = await window.showInputBox({
        title: 'ElixirIDE: Model',
        prompt: 'Enter the model name to use',
        value: current?.type === providerType ? current.model : defaults.defaultModel,
        validateInput: (v) => (v.trim() ? null : 'Model cannot be empty'),
      });

      if (modelValue === undefined) return;

      // Step 4: Base URL (custom providers or optional override)
      let baseUrl: string | undefined;
      if (providerType === 'custom') {
        const urlValue = await window.showInputBox({
          title: 'ElixirIDE: Base URL',
          prompt: 'Enter the OpenAI-compatible base URL (e.g. http://localhost:11434)',
          value: current?.type === 'custom' ? (current.baseUrl ?? '') : '',
          validateInput: (v) => (v.trim() ? null : 'Base URL cannot be empty for custom providers'),
        });
        if (urlValue === undefined) return;
        baseUrl = urlValue.trim();
      } else {
        // Optional override for Anthropic / OpenAI
        const urlValue = await window.showInputBox({
          title: 'ElixirIDE: Base URL (optional)',
          prompt: `Leave blank to use the default ${defaults.baseUrl}`,
          value: current?.type === providerType ? (current.baseUrl ?? '') : '',
        });
        if (urlValue === undefined) return;
        baseUrl = urlValue.trim() || undefined;
      }

      setAIProviderConfig({
        type: providerType,
        apiKey: apiKey.trim(),
        model: modelValue.trim(),
        baseUrl,
      });

      window.showInformationMessage(
        `ElixirIDE: AI provider configured — ${defaults.label} / ${modelValue.trim()}`,
      );
    });

    console.log('[ElixirIDE] Registered elixide.configureAIProvider command');
  } catch (e) {
    console.error('[ElixirIDE] Failed to register configureAIProvider command:', e);
  }
}
