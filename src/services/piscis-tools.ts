// Piscis Tools Service - Frontend integration for piscis-engine native tools

export interface ToolDefinition {
  name: string;
  description: string;
  input_schema: Record<string, any>;
}

export interface ExecuteToolRequest {
  tool_name: string;
  parameters: Record<string, any>;
  session_id?: string;
  workspace_root?: string;
}

export interface ExecuteToolResponse {
  success: boolean;
  result?: any;
  error?: string;
  tool_use_id?: string;
}

export interface ToolEvent {
  session_id: string;
  tool: string;
  tool_use_id: string;
  parameters?: Record<string, any>;
  result?: any;
  error?: string;
  is_error?: boolean;
}

class PiscisToolsService {
  private listeners: Map<string, Set<(event: ToolEvent) => void>> = new Map();
  private initialized = false;

  async initialize(): Promise<void> {
    if (this.initialized) return;
    
    try {
      await window.__TAURI__.invoke('initialize_piscis_tools');
      this.initialized = true;
      console.log('[PiscisTools] Initialized');
    } catch (error) {
      console.error('[PiscisTools] Initialization failed:', error);
      throw error;
    }
  }

  async listTools(): Promise<ToolDefinition[]> {
    return await window.__TAURI__.invoke('list_piscis_tools');
  }

  async executeTool(request: ExecuteToolRequest): Promise<ExecuteToolResponse> {
    return await window.__TAURI__.invoke('execute_piscis_tool', request);
  }

  onToolStart(callback: (event: ToolEvent) => void): () => void {
    return this.addListener('piscis-tool-start', callback);
  }

  onToolComplete(callback: (event: ToolEvent) => void): () => void {
    return this.addListener('piscis-tool-complete', callback);
  }

  onToolError(callback: (event: ToolEvent) => void): () => void {
    return this.addListener('piscis-tool-error', callback);
  }

  private addListener(eventName: string, callback: (event: ToolEvent) => void): () => void {
    if (!this.listeners.has(eventName)) {
      this.listeners.set(eventName, new Set());
      
      // Set up Tauri listener
      window.__TAURI__.event.listen(eventName, (event: any) => {
        const listeners = this.listeners.get(eventName);
        if (listeners) {
          listeners.forEach(cb => cb(event.payload));
        }
      });
    }
    
    this.listeners.get(eventName)!.add(callback);
    
    // Return unsubscribe function
    return () => {
      const listeners = this.listeners.get(eventName);
      if (listeners) {
        listeners.delete(callback);
      }
    };
  }

  // Helper methods for common tool operations
  async readFile(path: string, sessionId?: string): Promise<string> {
    const result = await this.executeTool({
      tool_name: 'file_read',
      parameters: { path },
      session_id: sessionId,
    });
    
    if (!result.success) throw new Error(result.error || 'Failed to read file');
    return result.result;
  }

  async writeFile(path: string, content: string, sessionId?: string): Promise<void> {
    const result = await this.executeTool({
      tool_name: 'file_write',
      parameters: { path, content },
      session_id: sessionId,
    });
    
    if (!result.success) throw new Error(result.error || 'Failed to write file');
  }

  async editFile(path: string, oldText: string, newText: string, sessionId?: string): Promise<void> {
    const result = await this.executeTool({
      tool_name: 'file_edit',
      parameters: { path, old_text: oldText, new_text: newText },
      session_id: sessionId,
    });
    
    if (!result.success) throw new Error(result.error || 'Failed to edit file');
  }

  async listFiles(path: string = '.', sessionId?: string): Promise<string[]> {
    const result = await this.executeTool({
      tool_name: 'file_list',
      parameters: { path },
      session_id: sessionId,
    });
    
    if (!result.success) throw new Error(result.error || 'Failed to list files');
    return result.result || [];
  }

  async searchFiles(pattern: string, path?: string, sessionId?: string): Promise<any[]> {
    const result = await this.executeTool({
      tool_name: 'file_search',
      parameters: { pattern, path },
      session_id: sessionId,
    });
    
    if (!result.success) throw new Error(result.error || 'Failed to search files');
    return result.result || [];
  }

  async runShell(command: string, cwd?: string, timeoutMs?: number, sessionId?: string): Promise<string> {
    const result = await this.executeTool({
      tool_name: 'shell',
      parameters: { command, cwd, timeout_ms: timeoutMs },
      session_id: sessionId,
    });
    
    if (!result.success) throw new Error(result.error || 'Shell command failed');
    return result.result;
  }

  async runCode(language: string, code: string, files?: Record<string, string>, sessionId?: string): Promise<any> {
    const result = await this.executeTool({
      tool_name: 'code_run',
      parameters: { language, code, files },
      session_id: sessionId,
    });
    
    if (!result.success) throw new Error(result.error || 'Code execution failed');
    return result.result;
  }

  async webSearch(query: string, maxResults: number = 10, sessionId?: string): Promise<any[]> {
    const result = await this.executeTool({
      tool_name: 'web_search',
      parameters: { query, max_results: maxResults },
      session_id: sessionId,
    });
    
    if (!result.success) throw new Error(result.error || 'Web search failed');
    return result.result || [];
  }

  async webFetch(url: string, maxLength?: number, sessionId?: string): Promise<string> {
    const result = await this.executeTool({
      tool_name: 'web_fetch',
      parameters: { url, max_length: maxLength },
      session_id: sessionId,
    });
    
    if (!result.success) throw new Error(result.error || 'Web fetch failed');
    return result.result;
  }

  async planTodo(items: string[], sessionId?: string): Promise<void> {
    const result = await this.executeTool({
      tool_name: 'plan_todo',
      parameters: { items },
      session_id: sessionId,
    });
    
    if (!result.success) throw new Error(result.error || 'Failed to create todo plan');
  }

  async storeMemory(key: string, value: string, sessionId?: string): Promise<void> {
    const result = await this.executeTool({
      tool_name: 'memory_store',
      parameters: { key, value },
      session_id: sessionId,
    });
    
    if (!result.success) throw new Error(result.error || 'Failed to store memory');
  }

  async recallMemory(key: string, sessionId?: string): Promise<string> {
    const result = await this.executeTool({
      tool_name: 'memory_recall',
      parameters: { key },
      session_id: sessionId,
    });
    
    if (!result.success) throw new Error(result.error || 'Failed to recall memory');
    return result.result;
  }
}

export const piscisTools = new PiscisToolsService();
export default piscisTools;