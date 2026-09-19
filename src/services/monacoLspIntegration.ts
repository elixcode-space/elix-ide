import * as monaco from 'monaco-editor';
import tauriLSPService from './tauriLspService';

interface LSPClient {
  dispose(): void;
}

function createLSPClient(languageId: string): LSPClient {
  const disposables: monaco.IDisposable[] = [];

  // Completion provider
  disposables.push(
    monaco.languages.registerCompletionItemProvider(languageId, {
      provideCompletionItems: async (model, position) => {
        try {
          const uri = model.uri.toString();
          const path = uri.replace('file://', '');
          const result = await tauriLSPService.completion(path, position.lineNumber - 1, position.column - 1);
          if (!result) return { suggestions: [] };
          
          const items = result.items || result;
          return {
            suggestions: items.map((item: any) => ({
              label: item.label,
              kind: monaco.languages.CompletionItemKind[item.kind] || monaco.languages.CompletionItemKind.Text,
              detail: item.detail,
              documentation: item.documentation ? (typeof item.documentation === 'string' ? item.documentation : item.documentation.value) : undefined,
              insertText: item.insertText || item.label,
              insertTextRules: item.insertTextFormat === 2 ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
              range: item.textEdit ? {
                startLineNumber: item.textEdit.range.start.line + 1,
                startColumn: item.textEdit.range.start.character + 1,
                endLineNumber: item.textEdit.range.end.line + 1,
                endColumn: item.textEdit.range.end.character + 1,
              } : undefined,
              sortText: item.sortText,
              filterText: item.filterText,
            })),
            incomplete: result.isIncomplete,
          };
        } catch (err) {
          console.error('[LSP Client] Completion error:', err);
          return { suggestions: [] };
        }
      },
      triggerCharacters: ['.', ':', '<', '"', "'", '/', '@', '#'],
    })
  );

  // Hover provider
  disposables.push(
    monaco.languages.registerHoverProvider(languageId, {
      provideHover: async (model, position) => {
        try {
          const uri = model.uri.toString();
          const path = uri.replace('file://', '');
          const hover = await tauriLSPService.hover(path, position.lineNumber - 1, position.column - 1);
          if (!hover) return null;
          
          return {
            range: hover.range ? {
              startLineNumber: hover.range.start.line + 1,
              startColumn: hover.range.start.character + 1,
              endLineNumber: hover.range.end.line + 1,
              endColumn: hover.range.end.character + 1,
            } : undefined,
            contents: [
              typeof hover.contents === 'string' 
                ? hover.contents 
                : hover.contents.map((c: any) => typeof c === 'string' ? c : c.value).join('\n'),
            ],
          };
        } catch (err) {
          console.error('[LSP Client] Hover error:', err);
          return null;
        }
      },
    })
  );

  // Definition provider
  disposables.push(
    monaco.languages.registerDefinitionProvider(languageId, {
      provideDefinition: async (model, position) => {
        try {
          const uri = model.uri.toString();
          const path = uri.replace('file://', '');
          const result = await tauriLSPService.gotoDefinition(path, position.lineNumber - 1, position.column - 1);
          if (!result) return [];
          
          const locations = Array.isArray(result) ? result : [result];
          return locations.map((loc: any) => ({
            uri: monaco.Uri.parse(loc.uri || loc.targetUri),
            range: {
              startLineNumber: (loc.range || loc.targetRange).start.line + 1,
              startColumn: (loc.range || loc.targetRange).start.character + 1,
              endLineNumber: (loc.range || loc.targetRange).end.line + 1,
              endColumn: (loc.range || loc.targetRange).end.character + 1,
            },
          }));
        } catch (err) {
          console.error('[LSP Client] Definition error:', err);
          return [];
        }
      },
    })
  );

  // Reference provider
  disposables.push(
    monaco.languages.registerReferenceProvider(languageId, {
      provideReferences: async (model, position, context) => {
        try {
          const uri = model.uri.toString();
          const path = uri.replace('file://', '');
          const result = await tauriLSPService.references(path, position.lineNumber - 1, position.column - 1, context.includeDeclaration);
          if (!result) return [];
          
          return result.map((loc: any) => ({
            uri: monaco.Uri.parse(loc.uri),
            range: {
              startLineNumber: loc.range.start.line + 1,
              startColumn: loc.range.start.character + 1,
              endLineNumber: loc.range.end.line + 1,
              endColumn: loc.range.end.character + 1,
            },
          }));
        } catch (err) {
          console.error('[LSP Client] References error:', err);
          return [];
        }
      },
    })
  );

  // Document symbol provider
  disposables.push(
    monaco.languages.registerDocumentSymbolProvider(languageId, {
      provideDocumentSymbols: async (model) => {
        try {
          const uri = model.uri.toString();
          const path = uri.replace('file://', '');
          const result = await tauriLSPService.documentSymbols(path);
          if (!result) return [];
          
          return (result as any[]).map((sym: any) => ({
            name: sym.name,
            detail: sym.detail,
            kind: monaco.languages.SymbolKind[sym.kind] || monaco.languages.SymbolKind.Variable,
            range: {
              startLineNumber: sym.range.start.line + 1,
              startColumn: sym.range.start.character + 1,
              endLineNumber: sym.range.end.line + 1,
              endColumn: sym.range.end.character + 1,
            },
            selectionRange: {
              startLineNumber: sym.selectionRange.start.line + 1,
              startColumn: sym.selectionRange.start.character + 1,
              endLineNumber: sym.selectionRange.end.line + 1,
              endColumn: sym.selectionRange.end.character + 1,
            },
            children: sym.children?.map((c: any) => ({
              name: c.name,
              kind: monaco.languages.SymbolKind[c.kind] || monaco.languages.SymbolKind.Variable,
              range: {
                startLineNumber: c.range.start.line + 1,
                startColumn: c.range.start.character + 1,
                endLineNumber: c.range.end.line + 1,
                endColumn: c.range.end.character + 1,
              },
              selectionRange: {
                startLineNumber: c.selectionRange.start.line + 1,
                startColumn: c.selectionRange.start.character + 1,
                endLineNumber: c.selectionRange.end.line + 1,
                endColumn: c.selectionRange.end.character + 1,
              },
            })) || [],
          }));
        } catch (err) {
          console.error('[LSP Client] Document symbols error:', err);
          return [];
        }
      },
    })
  );

  // Code action provider
  disposables.push(
    monaco.languages.registerCodeActionProvider(languageId, {
      provideCodeActions: async (model, range, context) => {
        try {
          const uri = model.uri.toString();
          const path = uri.replace('file://', '');
          const result = await tauriLSPService.codeAction(path, range, context.markers.map(m => ({
            severity: m.severity,
            message: m.message,
            range: {
              startLineNumber: m.startLineNumber,
              startColumn: m.startColumn,
              endLineNumber: m.endLineNumber,
              endColumn: m.endColumn,
            },
            source: m.source,
            code: m.code,
          } as any)));
          if (!result) return { actions: [], dispose: () => {} };
          
          return {
            actions: result.map((action: any) => ({
              title: action.title,
              kind: action.kind,
              diagnostics: action.diagnostics,
              isPreferred: action.isPreferred,
              edit: action.edit ? {
                edits: Object.entries(action.edit.changes || {}).flatMap(([uri, edits]: [string, any]) => 
                  (edits as any[]).map((edit: any) => ({
                    resource: monaco.Uri.parse(uri),
                    edit: {
                      range: {
                        startLineNumber: edit.range.start.line + 1,
                        startColumn: edit.range.start.character + 1,
                        endLineNumber: edit.range.end.line + 1,
                        endColumn: edit.range.end.character + 1,
                      },
                      text: edit.newText,
                    },
                  }))
                ),
              } : undefined,
              command: action.command,
            })),
            dispose: () => {},
          };
        } catch (err) {
          console.error('[LSP Client] Code action error:', err);
          return { actions: [], dispose: () => {} };
        }
      },
    })
  );

  // Rename provider
  disposables.push(
    monaco.languages.registerRenameProvider(languageId, {
      provideRenameEdits: async (model, position, newName) => {
        try {
          const uri = model.uri.toString();
          const path = uri.replace('file://', '');
          const result = await tauriLSPService.rename(path, position.lineNumber - 1, position.column - 1, newName);
          if (!result) return { edits: [] };
          
          return {
            edits: Object.entries(result.changes || {}).flatMap(([uri, edits]: [string, any]) => 
              (edits as any[]).map((edit: any) => ({
                resource: monaco.Uri.parse(uri),
                edit: {
                  range: {
                    startLineNumber: edit.range.start.line + 1,
                    startColumn: edit.range.start.character + 1,
                    endLineNumber: edit.range.end.line + 1,
                    endColumn: edit.range.end.character + 1,
                  },
                  text: edit.newText,
                },
              }))
            ),
          };
        } catch (err) {
          console.error('[LSP Client] Rename error:', err);
          return { edits: [] };
        }
      },
    })
  );

  // Formatting provider
  disposables.push(
    monaco.languages.registerDocumentFormattingEditProvider(languageId, {
      provideDocumentFormattingEdits: async (model, options) => {
        try {
          const uri = model.uri.toString();
          const path = uri.replace('file://', '');
          const result = await tauriLSPService.formatting(path, options.tabSize, options.insertSpaces);
          if (!result) return [];
          
          return result.map((edit: any) => ({
            range: {
              startLineNumber: edit.range.start.line + 1,
              startColumn: edit.range.start.character + 1,
              endLineNumber: edit.range.end.line + 1,
              endColumn: edit.range.end.character + 1,
            },
            text: edit.newText,
          }));
        } catch (err) {
          console.error('[LSP Client] Formatting error:', err);
          return [];
        }
      },
    })
  );

  // Signature help provider
  disposables.push(
    monaco.languages.registerSignatureHelpProvider(languageId, {
      provideSignatureHelp: async (model, position) => {
        try {
          const uri = model.uri.toString();
          const path = uri.replace('file://', '');
          const result = await tauriLSPService.signatureHelp(path, position.lineNumber - 1, position.column - 1);
          if (!result) return { signatures: [], activeSignature: 0, activeParameter: 0, dispose: () => {} };
          
          return {
            signatures: (result.signatures || []).map((sig: any) => ({
              label: sig.label,
              documentation: sig.documentation ? (typeof sig.documentation === 'string' ? sig.documentation : sig.documentation.value) : undefined,
              parameters: (sig.parameters || []).map((param: any) => ({
                label: param.label,
                documentation: param.documentation ? (typeof param.documentation === 'string' ? param.documentation : param.documentation.value) : undefined,
              })),
            })),
            activeSignature: result.activeSignature || 0,
            activeParameter: result.activeParameter || 0,
            dispose: () => {},
          };
        } catch (err) {
          console.error('[LSP Client] Signature help error:', err);
          return { signatures: [], activeSignature: 0, activeParameter: 0, dispose: () => {} };
        }
      },
      signatureHelpTriggerCharacters: ['(', ','],
      signatureHelpRetriggerCharacters: [')'],
    })
  );

  return {
    dispose: () => {
      disposables.forEach(d => d.dispose());
    },
  };
}

const clients = new Map<string, LSPClient>();

export function registerLSPForLanguage(languageId: string): void {
  if (clients.has(languageId)) return;
  clients.set(languageId, createLSPClient(languageId));
  console.log('[LSP Client] Registered for language:', languageId);
}

export function unregisterLSPForLanguage(languageId: string): void {
  const client = clients.get(languageId);
  if (client) {
    client.dispose();
    clients.delete(languageId);
  }
}

export function registerLSPForAllLanguages(): void {
  ['rust', 'typescript', 'javascript', 'typescriptreact', 'javascriptreact', 'python', 'go'].forEach(registerLSPForLanguage);
}

export function disposeAllLSPClients(): void {
  clients.forEach(client => client.dispose());
  clients.clear();
}