import { invoke } from '@tauri-apps/api/core';

export interface TelemetryEvent {
  name: string;
  properties?: Record<string, unknown>;
  timestamp: number;
  sessionId: string;
  version: string;
  platform: string;
}

export interface TelemetryMetrics {
  filesOpened: number;
  filesSaved: number;
  charactersTyped: number;
  commandsExecuted: number;
  terminalsCreated: number;
  terminalCommandsRun: number;
  gitOperations: number;
  gitCommits: number;
  gitPushes: number;
  gitPulls: number;
  extensionsInstalled: number;
  extensionsActivated: number;
  aiQueries: number;
  aiCompletionsAccepted: number;
  aiCompletionsRejected: number;
  startupTime: number;
  memoryUsage: number;
  cpuUsage: number;
  errors: number;
  crashes: number;
}

class TelemetryService {
  private sessionId: string;
  private version: string;
  private platform: string;
  private events: TelemetryEvent[] = [];
  private metrics: TelemetryMetrics = {
    filesOpened: 0,
    filesSaved: 0,
    charactersTyped: 0,
    commandsExecuted: 0,
    terminalsCreated: 0,
    terminalCommandsRun: 0,
    gitOperations: 0,
    gitCommits: 0,
    gitPushes: 0,
    gitPulls: 0,
    extensionsInstalled: 0,
    extensionsActivated: 0,
    aiQueries: 0,
    aiCompletionsAccepted: 0,
    aiCompletionsRejected: 0,
    startupTime: 0,
    memoryUsage: 0,
    cpuUsage: 0,
    errors: 0,
    crashes: 0,
  };
  private flushInterval: number | null = null;
  private enabled: boolean = false;

  constructor() {
    this.sessionId = this.generateSessionId();
    this.version = '0.1.0';
    this.platform = navigator.platform;
  }

  private generateSessionId(): string {
    return `session_${Date.now()}_${Math.random().toString(36).substring(2, 15)}`;
  }

  async initialize(enabled: boolean = false): Promise<void> {
    this.enabled = enabled;
    if (enabled) {
      this.startFlushInterval();
      this.trackEvent('session_start', { version: this.version });
    }
  }

  private startFlushInterval(): void {
    this.flushInterval = window.setInterval(() => {
      this.flush();
    }, 60000);
  }

  stop(): void {
    if (this.flushInterval) {
      clearInterval(this.flushInterval);
      this.flushInterval = null;
    }
    if (this.enabled) {
      this.trackEvent('session_end', { duration: Date.now() });
      this.flush();
    }
  }

  trackEvent(name: string, properties?: Record<string, unknown>): void {
    if (!this.enabled) return;
    this.events.push({
      name,
      properties,
      timestamp: Date.now(),
      sessionId: this.sessionId,
      version: this.version,
      platform: this.platform,
    });
  }

  incrementMetric<K extends keyof TelemetryMetrics>(key: K, value: number = 1): void {
    if (!this.enabled) return;
    (this.metrics[key] as number) += value;
  }

  async flush(): Promise<void> {
    if (!this.enabled || this.events.length === 0) return;

    const eventsToSend = [...this.events];
    const metricsToSend = { ...this.metrics };
    this.events = [];

    try {
      await invoke('telemetry_flush', {
        events: eventsToSend,
        metrics: metricsToSend,
        sessionId: this.sessionId,
      });
    } catch (error) {
      console.error('Failed to flush telemetry:', error);
      this.events.unshift(...eventsToSend);
    }
  }

  getSessionId(): string {
    return this.sessionId;
  }

  isEnabled(): boolean {
    return this.enabled;
  }

  setEnabled(enabled: boolean): void {
    this.enabled = enabled;
    if (enabled && !this.flushInterval) {
      this.startFlushInterval();
    } else if (!enabled && this.flushInterval) {
      this.stop();
    }
  }
}

export const telemetryService = new TelemetryService();
export default telemetryService;