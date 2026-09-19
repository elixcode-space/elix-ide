import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export class CoreEngineService {
  private useUnikernel: boolean = false;
  private unikernelUrl: string = 'http://localhost:8080';
  private initialized: boolean = false;
  private unlisten: (() => void) | null = null;

  constructor() {
    this.useUnikernel = import.meta.env.VITE_USE_UNIKERNEL === 'true';
    this.unikernelUrl = import.meta.env.VITE_CORE_ENGINE_URL || 'http://localhost:8080';
  }

  async initialize(): Promise<void> {
    if (this.initialized) return;

    if (this.useUnikernel) {
      await fetch(`${this.unikernelUrl}/init`);
    } else {
      await invoke('initialize_core');
    }
    this.initialized = true;
  }

  async runQuery(query: string): Promise<string> {
    await this.initialize();

    if (this.useUnikernel) {
      const response = await fetch(`${this.unikernelUrl}/query`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ query })
      });
      const data = await response.json();
      return data.response;
    }

    return await invoke('run_agent_query', { query });
  }

  async streamResponse(query: string, callback: (chunk: string) => void): Promise<void> {
    if (this.useUnikernel) {
      const ws = new WebSocket(`${this.unikernelUrl.replace('http', 'ws')}/stream`);

      ws.onopen = () => {
        ws.send(JSON.stringify({ query }));
      };

      ws.onmessage = (event) => {
        callback(event.data);
      };
    } else {
      this.unlisten = await listen('core-response-chunk', (event) => {
        callback(event.payload as string);
      });
      invoke('start_agent_stream', { query });
    }
  }

  cleanup(): void {
    if (this.unlisten) {
      this.unlisten();
      this.unlisten = null;
    }
  }
}

export const coreEngine = new CoreEngineService();