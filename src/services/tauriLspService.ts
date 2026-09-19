import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Position, Range, CompletionItem, CompletionList, Hover, Location, Diagnostic, CodeAction, WorkspaceEdit, TextEdit, SignatureHelp, SemanticTokens } from 'monaco-editor';

interface LSPServerConfig {
  language: string;
  name: string;
  command: string[];
  file_extensions: string[];
  root_patterns: string[];
  available: boolean;
  installed_version?: string;
  installed_path?: string;
}

class TauriLSPService {
  private initialized = false;
  private workspaceRoot: string | null = null;
  private documentVersions = new Map<string, number>();
  private diagnosticsListeners = new Map<string, (diagnostics: Diagnostic[]) => void>();
  private semanticTokensCache = new Map<string, SemanticTokens>();

  async initialize(workspaceRoot: string): Promise<void> {
    if (this.initialized && this.workspaceRoot === workspaceRoot) return;
    
    this.workspaceRoot = workspaceRoot;
    try {
      await invoke('lsp_initialize', { workspace_root: workspaceRoot });
      this.initialized = true;
      console.log('[TauriLSP] Initialized for workspace:', workspaceRoot);
    } catch (err) {
      console.error('[TauriLSP] Failed to initialize:', err);
      throw err;
    }
  }

  async shutdown(): Promise<void> {
    if (!this.initialized) return;
    try {
      await invoke('lsp_shutdown');
      this.initialized = false;
      this.workspaceRoot = null;
      this.documentVersions.clear();
      console.log('[TauriLSP] Shutdown complete');
    } catch (err) {
      console.error('[TauriLSP] Failed to shutdown:', err);
    }
  }

  async getConfigs(): Promise<LSPServerConfig[]> {
    return await invoke('get_lsp_configs');
  }

  async installServer(language: string): Promise<string> {
    return await invoke('lsp_install_server', { language });
  }

  async uninstallServer(name: string): Promise<string> {
    return await invoke('lsp_uninstall_server', { name });
  }

  private getUri(path: string): string {
    return `file://${path}`;
  }

  private getVersion(uri: string): number {
    const version = this.documentVersions.get(uri) || 1;
    this.documentVersions.set(uri, version + 1);
    return version;
  }

  async didOpen(path: string, languageId: string, text: string): Promise<void> {
    const uri = this.getUri(path);
    const version = this.getVersion(uri);
    await invoke('lsp_did_open', { uri, language_id: languageId, version, text });
  }

  async didChange(path: string, text: string, changes?: { range: Range; text: string; rangeLength?: number }[]): Promise<void> {
    const uri = this.getUri(path);
    const version = this.getVersion(uri);
    
    const lspChanges = changes || [{ 
      range: { 
        start: { line: 0, character: 0 }, 
        end: { line: 10000, character: 0 } 
      }, 
      text 
    }];
    
    await invoke('lsp_did_change', { uri, version, changes: lspChanges });
  }

  async didClose(path: string): Promise<void> {
    const uri = this.getUri(path);
    await invoke('lsp_did_close', { uri });
    this.documentVersions.delete(uri);
  }

  async didSave(path: string, text?: string): Promise<void> {
    const uri = this.getUri(path);
    await invoke('lsp_did_save', { uri, text });
  }

  async completion(path: string, line: number, character: number): Promise<CompletionList | null> {
    const uri = this.getUri(path);
    return await invoke('lsp_completion', { uri, line: line as u32, character: character as u32 });
  }

  async hover(path: string, line: number, character: number): Promise<Hover | null> {
    const uri = this.getUri(path);
    return await invoke('lsp_hover', { uri, line: line as u32, character: character as u32 });
  }

  async gotoDefinition(path: string, line: number, character: number): Promise<Location | Location[] | null> {
    const uri = this.getUri(path);
    return await invoke('lsp_goto_definition', { uri, line: line as u32, character: character as u32 });
  }

  async references(path: string, line: number, character: number, includeDeclaration = true): Promise<Location[] | null> {
    const uri = this.getUri(path);
    return await invoke('lsp_references', { uri, line: line as u32, character: character as u32, include_declaration: includeDeclaration });
  }

  async documentSymbols(path: string): Promise<any> {
    const uri = this.getUri(path);
    return await invoke('lsp_document_symbols', { uri });
  }

  async codeAction(path: string, range: Range, diagnostics: Diagnostic[]): Promise<CodeAction[] | null> {
    const uri = this.getUri(path);
    return await invoke('lsp_code_action', {
      uri,
      start_line: range.startLineNumber as u32,
      start_char: range.startColumn as u32,
      end_line: range.endLineNumber as u32,
      end_char: range.endColumn as u32,
      diagnostics,
    });
  }

  async rename(path: string, line: number, character: number, newName: string): Promise<WorkspaceEdit | null> {
    const uri = this.getUri(path);
    return await invoke('lsp_rename', { uri, line: line as u32, character: character as u32, new_name: newName });
  }

  async formatting(path: string, tabSize: number, insertSpaces: boolean): Promise<TextEdit[] | null> {
    const uri = this.getUri(path);
    return await invoke('lsp_formatting', { uri, tab_size: tabSize as u32, insert_spaces: insertSpaces });
  }

  async signatureHelp(path: string, line: number, character: number): Promise<SignatureHelp | null> {
    const uri = this.getUri(path);
    return await invoke('lsp_signature_help', { uri, line: line as u32, character: character as u32 });
  }

  async semanticTokensFull(path: string): Promise<SemanticTokens | null> {
    const uri = this.getUri(path);
    const tokens = await invoke('lsp_semantic_tokens_full', { uri });
    if (tokens) this.semanticTokensCache.set(uri, tokens);
    return tokens;
  }

  onDiagnostics(path: string, callback: (diagnostics: Diagnostic[]) => void): () => void {
    const uri = this.getUri(path);
    this.diagnosticsListeners.set(uri, callback);
    
    return () => {
      this.diagnosticsListeners.delete(uri);
    };
  }

  async triggerDiagnosticsRefresh(): Promise<void> {
    for (const [uri, callback] of this.diagnosticsListeners) {
      // Trigger a didSave to refresh diagnostics
      const path = uri.replace('file://', '');
      await this.didSave(path);
    }
  }
}

export const tauriLSPService = new TauriLSPService();
export default tauriLSPService;