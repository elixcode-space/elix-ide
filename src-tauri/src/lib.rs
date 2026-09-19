mod channels;
pub mod commands;
pub mod config;
pub mod core_engine;
pub mod dap;
pub mod git;
pub mod lsp;
mod services;
pub mod svm_service;
pub mod terminal;
#[cfg(feature = "unikernel")]
pub mod unikernel;


// #[cfg(debug_assertions)]
// mod test_server;
// #[cfg(debug_assertions)]
// use test_server::commands::test_server_callback;


use commands::{
    apply_document_edits, create_document, get_document_context_for_ai, get_supported_extensions,
    is_document_supported, read_document, render_document_html, DocumentState,
    // AI Chat commands
    cancel_ai_chat, is_ai_sidecar_ready, send_ai_chat, start_ai_sidecar, stop_ai_sidecar,
    AISidecarState,
    // Extension commands
    install_extension, install_extension_from_data, list_installed_extensions,
                     read_extension_manifest, scan_extensions, uninstall_extension,
    // Extension Management commands
    extension_search, extension_get_info, extension_install, extension_uninstall,
    extension_enable, extension_disable, extension_list_installed, extension_get_installed,
    extension_get_activation_events, extension_activate, extension_deactivate,
    // Extension Host commands
    activate_extension, deactivate_extension, execute_extension_command, get_activated_extensions,
    is_extension_host_ready, set_extension_host_workspace, start_extension_host,
    stop_extension_host, ExtensionHostState,
    // VSCode Server commands
    get_vscode_server_info, is_vscode_server_running, restart_vscode_server, start_vscode_server,
    stop_vscode_server, VscodeServerState,
    // Channel Router commands
    channel_call, init_channel_router, is_extension_host_connection_ready,
    list_extension_host_connections, spawn_extension_host_connection,
    terminate_extension_host_connection, ChannelRouterState,
    // Singleton extension host commands (new)
    start_default_extension_host, stop_default_extension_host, is_default_extension_host_ready,
    set_default_extension_host_workspace, activate_default_extension, deactivate_default_extension,
    execute_default_extension_command, get_default_activated_extensions,
    // Singleton document management commands
    open_default_document, update_default_document, close_default_document, set_default_configuration,
    // Singleton language provider commands
    request_default_completion, request_default_hover, request_default_definition,
    request_default_references, request_default_document_symbols, request_default_code_actions,
    request_default_formatting, request_default_signature_help,
    // Core Engine commands
    initialize_core, run_agent_query, start_agent_stream, load_svm_model, run_svm_prediction,
    // Piscis Engine Tools commands
    list_piscis_tools, execute_piscis_tool, initialize_piscis_tools,
    // Settings commands
    get_settings, update_settings, reset_settings, get_settings_path, open_settings_file,
    // Search commands
    initialize_search, search_code, search_symbols, search_replace, get_search_stats, reindex_workspace,
    // LSP commands
    lsp_initialize, lsp_shutdown, lsp_completion, lsp_hover, lsp_goto_definition,
    lsp_references, lsp_document_symbols, lsp_code_action, lsp_rename, lsp_formatting,
    lsp_signature_help, lsp_did_open, lsp_did_change, lsp_did_close, lsp_did_save,
    get_lsp_configs,
    lsp_install_server, lsp_uninstall_server, lsp_check_updates, lsp_update_server,
    lsp_get_installed_servers, lsp_semantic_tokens_full, lsp_semantic_tokens_range,
    lsp_semantic_tokens_full_delta,
    // DAP commands
    dap_initialize, dap_shutdown, dap_launch, dap_attach, dap_set_breakpoints,
    dap_continue, dap_pause, dap_step_over, dap_step_into, dap_step_out,
    dap_get_threads, dap_get_stack_trace, dap_get_scopes, dap_get_variables,
    dap_evaluate, dap_disconnect,
    dap_install_server, dap_uninstall_server, dap_get_installed_servers, dap_get_configs,
    // Git commands
    git_init, git_open, git_get_repository_info, git_status, git_diff, git_log,
    git_blame, git_add, git_commit, git_push, git_pull, git_fetch,
    git_create_branch, git_delete_branch, git_checkout, git_stash, git_stash_list,
    git_stash_pop, git_submodules, git_worktrees, git_tags, git_merge,
    // Terminal 2.0 commands
    terminal_create, terminal_get, terminal_list, terminal_write, terminal_resize,
    terminal_kill, terminal_set_title, terminal_get_profiles, terminal_add_profile,
    terminal_update_profile, terminal_delete_profile, terminal_get_config, terminal_update_config,
    terminal_detect_links, terminal_get_process_info,
    // Telemetry commands
    telemetry_flush,
    // Updater commands
    check_for_updates, download_and_install_update, install_update_and_restart, get_current_version, set_update_channel,
    UpdaterState,
    TerminalState,
};

// use channels::v2_streaming::StreamManager;
use services::lazy_sidecar::{SidecarManager, default_sidecar_configs};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        // .plugin(tauri_plugin_isolation::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_sql::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        // .plugin(tauri_plugin_global_shortcut::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        // .plugin(tauri_plugin_sentry::init())
.manage(DocumentState::new())
        .manage(TerminalState::new())
        .manage(AISidecarState::new())
        .manage(ExtensionHostState::new())
         .manage(VscodeServerState::new())
        .manage(UpdaterState::new())
 // Channel router state is initialized with the sidecar path (relative to src-tauri/ in dev)
          .manage(ChannelRouterState::new("binaries/extension-host-sidecar.js".to_string()))
          // .manage(StreamManager::new())
          // Lazy sidecar manager
          // .manage({
          //     let manager = SidecarManager::new();
          //     for (name, config) in default_sidecar_configs() {
          //         manager.register(name, config);
          //     }
          //     manager
          // })
          .invoke_handler({

            #[cfg(debug_assertions)]
            {
                tauri::generate_handler![
                    greet,
                    // Document commands
                    read_document,
                    create_document,
                    apply_document_edits,
                    render_document_html,
                    get_document_context_for_ai,
                    get_supported_extensions,
                    is_document_supported,
                    // AI Chat commands
                    start_ai_sidecar,
                    stop_ai_sidecar,
                    is_ai_sidecar_ready,
                    send_ai_chat,
                    cancel_ai_chat,
                    // Extension commands
                    install_extension,
                    install_extension_from_data,
                    uninstall_extension,
                    list_installed_extensions,
                    read_extension_manifest,
                    scan_extensions,
                    // Extension Management commands
                    extension_search,
                    extension_get_info,
                    extension_install,
                    extension_uninstall,
                    extension_enable,
                    extension_disable,
                    extension_list_installed,
                    extension_get_installed,
                    extension_get_activation_events,
                    extension_activate,
                    extension_deactivate,
                    // Extension Host commands
                    start_extension_host,
                    stop_extension_host,
                    is_extension_host_ready,
                    set_extension_host_workspace,
                    activate_extension,
                    deactivate_extension,
                    execute_extension_command,
                    get_activated_extensions,
                    // VSCode Server commands
                    start_vscode_server,
                    stop_vscode_server,
                    get_vscode_server_info,
                    restart_vscode_server,
                    is_vscode_server_running,
                    // Channel Router commands
                    init_channel_router,
                    channel_call,
                    spawn_extension_host_connection,
                    terminate_extension_host_connection,
                    is_extension_host_connection_ready,
                    list_extension_host_connections,
                    // Singleton extension host commands
                    start_default_extension_host,
                    stop_default_extension_host,
                    is_default_extension_host_ready,
                    set_default_extension_host_workspace,
                    activate_default_extension,
                    deactivate_default_extension,
                    execute_default_extension_command,
                    get_default_activated_extensions,
                    // Singleton document management commands
                    open_default_document,
                    update_default_document,
                    close_default_document,
                    set_default_configuration,
                    // Singleton language provider commands
                    request_default_completion,
                    request_default_hover,
                    request_default_definition,
                    request_default_references,
                    request_default_document_symbols,
                    request_default_code_actions,
                    request_default_formatting,
                    request_default_signature_help,
                    // Core Engine commands
                    initialize_core,
                    run_agent_query,
                    start_agent_stream,
                    load_svm_model,
                    run_svm_prediction,
                    // Piscis Engine Tools commands
                    list_piscis_tools,
                    execute_piscis_tool,
                    initialize_piscis_tools,
                    // Settings commands
                    get_settings,
                    update_settings,
                    reset_settings,
                    get_settings_path,
                    open_settings_file,
                    // Search commands
                    initialize_search,
                    search_code,
                    search_symbols,
                    search_replace,
                    get_search_stats,
                    reindex_workspace,
                    // LSP commands
                    lsp_initialize,
                    lsp_shutdown,
                    lsp_completion,
                    lsp_hover,
                    lsp_goto_definition,
                    lsp_references,
                    lsp_document_symbols,
                    lsp_code_action,
                    lsp_rename,
                    lsp_formatting,
                    lsp_signature_help,
                    lsp_did_open,
                    lsp_did_change,
                    lsp_did_close,
                    lsp_did_save,
                    get_lsp_configs,
                    lsp_install_server,
                    lsp_uninstall_server,
                    lsp_check_updates,
                    lsp_update_server,
                    lsp_get_installed_servers,
                    lsp_semantic_tokens_full,
                    lsp_semantic_tokens_range,
                    lsp_semantic_tokens_full_delta,
                    // DAP commands
                    dap_initialize,
                    dap_shutdown,
                    dap_launch,
                    dap_attach,
                    dap_set_breakpoints,
                    dap_continue,
                    dap_pause,
                    dap_step_over,
                    dap_step_into,
                    dap_step_out,
                    dap_get_threads,
                    dap_get_stack_trace,
                    dap_get_scopes,
                    dap_get_variables,
                    dap_evaluate,
                    dap_disconnect,
                    dap_install_server,
                    dap_uninstall_server,
                    dap_get_installed_servers,
                    dap_get_configs,
                    // Git commands
                    git_init,
                    git_open,
                    git_get_repository_info,
                    git_status,
                    git_diff,
                    git_log,
                    git_blame,
                    git_add,
                    git_commit,
                    git_push,
                    git_pull,
                    git_fetch,
                    git_create_branch,
                    git_delete_branch,
                    git_checkout,
                    git_stash,
                    git_stash_list,
                    git_stash_pop,
                    git_submodules,
                    git_worktrees,
                    git_tags,
                    git_merge,
                    // Terminal 2.0 commands
                    terminal_create,
                    terminal_get,
                    terminal_list,
                    terminal_write,
                    terminal_resize,
                    terminal_kill,
                    terminal_set_title,
                    terminal_get_profiles,
                    terminal_add_profile,
                    terminal_update_profile,
                    terminal_delete_profile,
                    terminal_get_config,
                    terminal_update_config,
                    terminal_detect_links,
                    terminal_get_process_info,
                    // Telemetry commands
                    telemetry_flush,
                    // Updater commands
                    check_for_updates,
                    download_and_install_update,
                    install_update_and_restart,
                    get_current_version,
                    set_update_channel,
                    // v2 Streaming commands
                    // start_chat_stream,
                    // start_terminal_stream,
                    // start_extension_host_stream,
                    // Vector store commands
                    // init_vector_store,
                    // index_workspace,
                    // search_codebase,
                    // reindex_file,
                    // get_vector_store_stats,
                    // Test server callback (for E2E tests)
                    // test_server_callback,
                ]
            }
            #[cfg(not(debug_assertions))]

            {
                tauri::generate_handler![
                    greet,
                    // Document commands
                    read_document,
                    create_document,
                    apply_document_edits,
                    render_document_html,
                    get_document_context_for_ai,
                    get_supported_extensions,
                    is_document_supported,
                    // AI Chat commands
                    start_ai_sidecar,
                    stop_ai_sidecar,
                    is_ai_sidecar_ready,
                    send_ai_chat,
                    cancel_ai_chat,
                    // Extension commands
                    install_extension,
                    install_extension_from_data,
                    uninstall_extension,
                    list_installed_extensions,
                    read_extension_manifest,
                    scan_extensions,
                    // Extension Management commands
                    extension_search,
                    extension_get_info,
                    extension_install,
                    extension_uninstall,
                    extension_enable,
                    extension_disable,
                    extension_list_installed,
                    extension_get_installed,
                    extension_get_activation_events,
                    extension_activate,
                    extension_deactivate,
                    // Extension Host commands
                    start_extension_host,
                    stop_extension_host,
                    is_extension_host_ready,
                    set_extension_host_workspace,
                    activate_extension,
                    deactivate_extension,
                    execute_extension_command,
                    get_activated_extensions,
                    // VSCode Server commands
                    start_vscode_server,
                    stop_vscode_server,
                    get_vscode_server_info,
                    restart_vscode_server,
                    is_vscode_server_running,
                    // Channel Router commands
                    init_channel_router,
                    channel_call,
                    spawn_extension_host_connection,
                    terminate_extension_host_connection,
                    is_extension_host_connection_ready,
                    list_extension_host_connections,
                    // Singleton extension host commands
                    start_default_extension_host,
                    stop_default_extension_host,
                    is_default_extension_host_ready,
                    set_default_extension_host_workspace,
                    activate_default_extension,
                    deactivate_default_extension,
                    execute_default_extension_command,
                    get_default_activated_extensions,
                    // Singleton document management commands
                    open_default_document,
                    update_default_document,
                    close_default_document,
                    set_default_configuration,
                    // Singleton language provider commands
                    request_default_completion,
                    request_default_hover,
                    request_default_definition,
                    request_default_references,
                    request_default_document_symbols,
                    request_default_code_actions,
                    request_default_formatting,
                    request_default_signature_help,
                    // Core Engine commands
                    initialize_core,
                    run_agent_query,
                    start_agent_stream,
                    load_svm_model,
                    run_svm_prediction,
                    // Piscis Engine Tools commands
                    list_piscis_tools,
                    execute_piscis_tool,
                    initialize_piscis_tools,
                    // Settings commands
                    get_settings,
                    update_settings,
                    reset_settings,
                    get_settings_path,
                    open_settings_file,
                    // Search commands
                    initialize_search,
                    search_code,
                    search_symbols,
                    search_replace,
                    get_search_stats,
                    reindex_workspace,
                    // LSP commands
                    lsp_initialize,
                    lsp_shutdown,
                    lsp_completion,
                    lsp_hover,
                    lsp_goto_definition,
                    lsp_references,
                    lsp_document_symbols,
                    lsp_code_action,
                    lsp_rename,
                    lsp_formatting,
                    lsp_signature_help,
                    lsp_did_open,
                    lsp_did_change,
                    lsp_did_close,
                    lsp_did_save,
                    get_lsp_configs,
                    lsp_install_server,
                    lsp_uninstall_server,
                    lsp_check_updates,
                    lsp_update_server,
                    lsp_get_installed_servers,
                    lsp_semantic_tokens_full,
                    lsp_semantic_tokens_range,
                    lsp_semantic_tokens_full_delta,
                    // DAP commands
                    dap_initialize,
                    dap_shutdown,
                    dap_launch,
                    dap_attach,
                    dap_set_breakpoints,
                    dap_continue,
                    dap_pause,
                    dap_step_over,
                    dap_step_into,
                    dap_step_out,
                    dap_get_threads,
                    dap_get_stack_trace,
                    dap_get_scopes,
                    dap_get_variables,
                    dap_evaluate,
                    dap_disconnect,
                    dap_install_server,
                    dap_uninstall_server,
                    dap_get_installed_servers,
                    dap_get_configs,
                    // Git commands
                    git_init,
                    git_open,
                    git_get_repository_info,
                    git_status,
                    git_diff,
                    git_log,
                    git_blame,
                    git_add,
                    git_commit,
                    git_push,
                    git_pull,
                    git_fetch,
                    git_create_branch,
                    git_delete_branch,
                    git_checkout,
                    git_stash,
                    git_stash_list,
                    git_stash_pop,
                    git_submodules,
                    git_worktrees,
                    git_tags,
                    git_merge,
                    // Terminal 2.0 commands
                    terminal_create,
                    terminal_get,
                    terminal_list,
                    terminal_write,
                    terminal_resize,
                    terminal_kill,
                    terminal_set_title,
                    terminal_get_profiles,
                    terminal_add_profile,
                    terminal_update_profile,
                    terminal_delete_profile,
                    terminal_get_config,
                    terminal_update_config,
                    terminal_detect_links,
                    terminal_get_process_info,
                    // Telemetry commands
                    telemetry_flush,
                    // Updater commands
                    check_for_updates,
                    download_and_install_update,
                    install_update_and_restart,
                    get_current_version,
                    set_update_channel,
                ]
            }
        })
        .setup(|app| {
            // Start the debug test server in debug builds only
            // #[cfg(debug_assertions)]
            // {
            //     test_server::start_test_server(app.handle().clone());
            // }
            
            // Start the lazy sidecar idle checker
            // let sidecar_manager: tauri::State<'_, SidecarManager> = app.state();
            // sidecar_manager.start_idle_checker();
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}