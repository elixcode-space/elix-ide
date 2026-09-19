use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub editor: EditorSettings,
    #[serde(default)]
    pub workbench: WorkbenchSettings,
    #[serde(default)]
    pub terminal: TerminalSettings,
    #[serde(default)]
    pub lsp: LSPSettings,
    #[serde(default)]
    pub extensions: ExtensionsSettings,
    #[serde(default)]
    pub files: FilesSettings,
    #[serde(default)]
    pub search: SearchSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            editor: EditorSettings::default(),
            workbench: WorkbenchSettings::default(),
            terminal: TerminalSettings::default(),
            lsp: LSPSettings::default(),
            extensions: ExtensionsSettings::default(),
            files: FilesSettings::default(),
            search: SearchSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSettings {
    #[serde(default = "default_font_size")]
    pub font_size: u16,
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default = "default_line_height")]
    pub line_height: f32,
    #[serde(default)]
    pub font_ligatures: bool,
    #[serde(default = "default_tab_size")]
    pub tab_size: u16,
    #[serde(default)]
    pub insert_spaces: bool,
    #[serde(default)]
    pub detect_indentation: bool,
    #[serde(default)]
    pub word_wrap: WordWrap,
    #[serde(default)]
    pub minimap_enabled: bool,
    #[serde(default)]
    pub line_numbers: LineNumbers,
    #[serde(default)]
    pub cursor_blinking: CursorBlinking,
    #[serde(default)]
    pub cursor_style: CursorStyle,
    #[serde(default)]
    pub smooth_scrolling: bool,
    #[serde(default)]
    pub bracket_pair_colorization: bool,
    #[serde(default)]
    pub guides_bracket_pairs: bool,
    #[serde(default)]
    pub render_whitespace: RenderWhitespace,
    #[serde(default)]
    pub render_control_characters: bool,
    #[serde(default)]
    pub folding_strategy: FoldingStrategy,
    #[serde(default)]
    pub unfold_on_click_after_end_of_line: bool,
}

fn default_font_size() -> u16 { 14 }
fn default_font_family() -> String { "Monospace".to_string() }
fn default_line_height() -> f32 { 1.5 }
fn default_tab_size() -> u16 { 4 }

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            font_size: default_font_size(),
            font_family: default_font_family(),
            line_height: default_line_height(),
            font_ligatures: true,
            tab_size: default_tab_size(),
            insert_spaces: true,
            detect_indentation: true,
            word_wrap: WordWrap::Off,
            minimap_enabled: true,
            line_numbers: LineNumbers::On,
            cursor_blinking: CursorBlinking::Blink,
            cursor_style: CursorStyle::Line,
            smooth_scrolling: true,
            bracket_pair_colorization: true,
            guides_bracket_pairs: true,
            render_whitespace: RenderWhitespace::Selection,
            render_control_characters: false,
            folding_strategy: FoldingStrategy::Auto,
            unfold_on_click_after_end_of_line: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WordWrap {
    #[default]
    Off,
    On,
    WordWrapColumn,
    Bounded,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LineNumbers {
    #[default]
    On,
    Off,
    Relative,
    RelativeOn,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CursorBlinking {
    #[default]
    Blink,
    Smooth,
    Phase,
    Expand,
    Solid,
    Off,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CursorStyle {
    #[default]
    Line,
    Block,
    Underline,
    LineThin,
    BlockOutline,
    UnderlineThin,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RenderWhitespace {
    #[default]
    Selection,
    None,
    All,
    Boundary,
    Trailing,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FoldingStrategy {
    #[default]
    Auto,
    Indentation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbenchSettings {
    #[serde(default)]
    pub color_theme: String,
    #[serde(default)]
    pub icon_theme: String,
    #[serde(default)]
    pub startup_editor: StartupEditor,
    #[serde(default)]
    pub restore_windows: RestoreWindows,
    #[serde(default)]
    pub open_mode: OpenMode,
    #[serde(default)]
    pub editor_enable_preview: bool,
    #[serde(default)]
    pub editor_enable_preview_from_quick_open: bool,
    #[serde(default)]
    pub editor_close_empty_groups: bool,
    #[serde(default)]
    pub panel_default_location: PanelLocation,
    #[serde(default)]
    pub sidebar_location: SidebarLocation,
    #[serde(default)]
    pub activity_bar_visible: bool,
    #[serde(default)]
    pub status_bar_visible: bool,
    #[serde(default)]
    pub command_center: bool,
}

impl Default for WorkbenchSettings {
    fn default() -> Self {
        Self {
            color_theme: "Default Dark+".to_string(),
            icon_theme: "vs-seti".to_string(),
            startup_editor: StartupEditor::WelcomePage,
            restore_windows: RestoreWindows::Preserve,
            open_mode: OpenMode::SingleInstance,
            editor_enable_preview: true,
            editor_enable_preview_from_quick_open: true,
            editor_close_empty_groups: true,
            panel_default_location: PanelLocation::Bottom,
            sidebar_location: SidebarLocation::Left,
            activity_bar_visible: true,
            status_bar_visible: true,
            command_center: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum StartupEditor {
    #[default]
    WelcomePage,
    ReadOnly,
    NewUntitledFile,
    WelcomePageInEmptyWindow,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RestoreWindows {
    #[default]
    Preserve,
    None,
    All,
    Folders,
    One,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OpenMode {
    #[default]
    SingleInstance,
    SingleInstancePerWorkspace,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PanelLocation {
    #[default]
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SidebarLocation {
    #[default]
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSettings {
    #[serde(default)]
    pub integrated_shell: Option<String>,
    #[serde(default)]
    pub integrated_shell_args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default = "default_shell_integration")]
    pub shell_integration_enabled: bool,
    #[serde(default = "default_cursor_blink")]
    pub cursor_blink: bool,
    #[serde(default = "default_scrollback")]
    pub scrollback: u32,
    #[serde(default)]
    pub font_size: u16,
    #[serde(default)]
    pub font_family: String,
    #[serde(default)]
    pub font_weight: String,
    #[serde(default)]
    pub font_weight_bold: String,
    #[serde(default)]
    pub line_height: f32,
    #[serde(default)]
    pub letter_spacing: f32,
    #[serde(default)]
    pub allow_chords: bool,
    #[serde(default)]
    pub fast_scroll_sensitivity: u16,
    #[serde(default)]
    pub mac_option_click_forces_selection: bool,
    #[serde(default)]
    pub mac_option_is_meta: bool,
    #[serde(default)]
    pub right_click_behavior: RightClickBehavior,
    #[serde(default)]
    pub copy_on_selection: bool,
    #[serde(default)]
    pub word_separators: String,
}

fn default_shell_integration() -> bool { true }
fn default_cursor_blink() -> bool { true }
fn default_scrollback() -> u32 { 10000 }

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            integrated_shell: None,
            integrated_shell_args: vec![],
            cwd: None,
            env: HashMap::new(),
            shell_integration_enabled: default_shell_integration(),
            cursor_blink: default_cursor_blink(),
            scrollback: default_scrollback(),
            font_size: 14,
            font_family: "Monospace".to_string(),
            font_weight: "normal".to_string(),
            font_weight_bold: "bold".to_string(),
            line_height: 1.5,
            letter_spacing: 0.0,
            allow_chords: true,
            fast_scroll_sensitivity: 5,
            mac_option_click_forces_selection: false,
            mac_option_is_meta: true,
            right_click_behavior: RightClickBehavior::Default,
            copy_on_selection: false,
            word_separators: " ()[]{}'\"`,;<>│".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RightClickBehavior {
    #[default]
    Default,
    CopyPaste,
    SelectWord,
    Nothing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSPSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub servers: HashMap<String, LSPServerConfig>,
    #[serde(default)]
    pub log_level: LSPLogLevel,
    #[serde(default)]
    pub trace_server: TraceServer,
    #[serde(default)]
    pub workspace_symbols_enabled: bool,
    #[serde(default)]
    pub document_highlight_enabled: bool,
    #[serde(default)]
    pub code_lens_enabled: bool,
    #[serde(default)]
    pub folding_range_enabled: bool,
    #[serde(default)]
    pub selection_range_enabled: bool,
    #[serde(default)]
    pub linked_editing_range_enabled: bool,
    #[serde(default)]
    pub call_hierarchy_enabled: bool,
    #[serde(default)]
    pub semantic_tokens_enabled: bool,
    #[serde(default)]
    pub inlay_hints_enabled: bool,
    #[serde(default)]
    pub inline_completion_enabled: bool,
}

impl Default for LSPSettings {
    fn default() -> Self {
        let mut servers = HashMap::new();
        servers.insert("rust-analyzer".to_string(), LSPServerConfig::default_rust());
        servers.insert("typescript".to_string(), LSPServerConfig::default_typescript());
        servers.insert("pyright".to_string(), LSPServerConfig::default_python());
        servers.insert("gopls".to_string(), LSPServerConfig::default_go());

        Self {
            enabled: true,
            auto_start: true,
            servers,
            log_level: LSPLogLevel::Info,
            trace_server: TraceServer::Off,
            workspace_symbols_enabled: true,
            document_highlight_enabled: true,
            code_lens_enabled: true,
            folding_range_enabled: true,
            selection_range_enabled: true,
            linked_editing_range_enabled: true,
            call_hierarchy_enabled: true,
            semantic_tokens_enabled: true,
            inlay_hints_enabled: true,
            inline_completion_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSPServerConfig {
    pub command: Vec<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub file_extensions: Vec<String>,
    #[serde(default)]
    pub root_patterns: Vec<String>,
    #[serde(default)]
    pub initialization_options: Option<serde_json::Value>,
    #[serde(default)]
    pub settings: Option<serde_json::Value>,
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub download_checksum: Option<String>,
}

impl LSPServerConfig {
    fn default_rust() -> Self {
        Self {
            command: vec!["rust-analyzer".to_string()],
            args: vec![],
            env: HashMap::new(),
            languages: vec!["rust".to_string()],
            file_extensions: vec!["rs".to_string()],
            root_patterns: vec!["Cargo.toml".to_string(), "rust-project.json".to_string()],
            initialization_options: None,
            settings: None,
            download_url: Some("https://github.com/rust-lang/rust-analyzer/releases/latest/download/rust-analyzer-x86_64-unknown-linux-gnu.gz".to_string()),
            download_checksum: None,
        }
    }

    fn default_typescript() -> Self {
        Self {
            command: vec!["typescript-language-server".to_string()],
            args: vec!["--stdio".to_string()],
            env: HashMap::new(),
            languages: vec!["typescript".to_string(), "javascript".to_string(), "typescriptreact".to_string(), "javascriptreact".to_string()],
            file_extensions: vec!["ts".to_string(), "js".to_string(), "tsx".to_string(), "jsx".to_string()],
            root_patterns: vec!["package.json".to_string(), "tsconfig.json".to_string(), "jsconfig.json".to_string()],
            initialization_options: None,
            settings: None,
            download_url: None,
            download_checksum: None,
        }
    }

    fn default_python() -> Self {
        Self {
            command: vec!["pyright-langserver".to_string()],
            args: vec!["--stdio".to_string()],
            env: HashMap::new(),
            languages: vec!["python".to_string()],
            file_extensions: vec!["py".to_string()],
            root_patterns: vec!["pyproject.toml".to_string(), "setup.py".to_string(), "setup.cfg".to_string(), "requirements.txt".to_string(), "Pipfile".to_string(), "pyrightconfig.json".to_string()],
            initialization_options: None,
            settings: None,
            download_url: None,
            download_checksum: None,
        }
    }

    fn default_go() -> Self {
        Self {
            command: vec!["gopls".to_string()],
            args: vec![],
            env: HashMap::new(),
            languages: vec!["go".to_string()],
            file_extensions: vec!["go".to_string()],
            root_patterns: vec!["go.mod".to_string(), "go.work".to_string()],
            initialization_options: None,
            settings: None,
            download_url: None,
            download_checksum: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LSPLogLevel {
    #[default]
    Info,
    Debug,
    Trace,
    Warn,
    Error,
    Off,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TraceServer {
    #[default]
    Off,
    Messages,
    Verbose,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionsSettings {
    #[serde(default)]
    pub auto_update: bool,
    #[serde(default)]
    pub auto_check_updates: bool,
    #[serde(default)]
    pub install_verification: bool,
    #[serde(default)]
    pub marketplace_url: String,
    #[serde(default)]
    pub gallery: GallerySettings,
    #[serde(default)]
    pub ignore_recommendations: bool,
}

impl Default for ExtensionsSettings {
    fn default() -> Self {
        Self {
            auto_update: true,
            auto_check_updates: true,
            install_verification: true,
            marketplace_url: "https://open-vsx.org/api".to_string(),
            gallery: GallerySettings::default(),
            ignore_recommendations: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GallerySettings {
    #[serde(default)]
    pub service_url: String,
    #[serde(default)]
    pub item_url: String,
    #[serde(default)]
    pub cache_path: Option<String>,
    #[serde(default)]
    pub publisher_display_name: String,
}

impl Default for GallerySettings {
    fn default() -> Self {
        Self {
            service_url: "https://open-vsx.org/api".to_string(),
            item_url: "https://open-vsx.org/api".to_string(),
            cache_path: None,
            publisher_display_name: "Open VSX".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesSettings {
    #[serde(default)]
    pub auto_save: AutoSave,
    #[serde(default = "default_auto_save_delay")]
    pub auto_save_delay: u32,
    #[serde(default)]
    pub hot_exit: HotExit,
    #[serde(default)]
    pub confirm_sync: bool,
    #[serde(default)]
    pub enable_trash: bool,
    #[serde(default)]
    pub trash_path: Option<String>,
    #[serde(default)]
    pub encoding: String,
    #[serde(default)]
    pub guess_encoding: bool,
    #[serde(default)]
    pub eol: Eol,
    #[serde(default)]
    pub insert_final_newline: bool,
    #[serde(default)]
    pub trim_final_newlines: bool,
    #[serde(default)]
    pub trim_trailing_whitespace: bool,
    #[serde(default)]
    pub max_memory_for_reason: u32,
    #[serde(default)]
    pub associations: HashMap<String, String>,
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
    #[serde(default)]
    pub watcher_exclude: HashMap<String, bool>,
}

fn default_auto_save_delay() -> u32 { 1000 }

impl Default for FilesSettings {
    fn default() -> Self {
        let mut exclude = HashMap::new();
        exclude.insert("**/.git/**".to_string(), true);
        exclude.insert("**/node_modules/**".to_string(), true);
        exclude.insert("**/target/**".to_string(), true);
        exclude.insert("**/.svn/**".to_string(), true);
        exclude.insert("**/.hg/**".to_string(), true);
        exclude.insert("**/CVS/**".to_string(), true);
        exclude.insert("**/.DS_Store".to_string(), true);
        exclude.insert("**/Thumbs.db".to_string(), true);

        Self {
            auto_save: AutoSave::Off,
            auto_save_delay: default_auto_save_delay(),
            hot_exit: HotExit::OnExit,
            confirm_sync: true,
            enable_trash: true,
            trash_path: None,
            encoding: "utf-8".to_string(),
            guess_encoding: true,
            eol: Eol::LF,
            insert_final_newline: true,
            trim_final_newlines: true,
            trim_trailing_whitespace: true,
            max_memory_for_reason: 4096,
            associations: HashMap::new(),
            exclude_patterns: vec!["**/node_modules/**".to_string(), "**/target/**".to_string(), "**/.git/**".to_string()],
            watcher_exclude: exclude,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AutoSave {
    #[default]
    Off,
    AfterDelay,
    OnFocusChange,
    OnWindowChange,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HotExit {
    #[default]
    OnExit,
    Off,
    OnExitAndWindowClose,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Eol {
    #[default]
    LF,
    CRLF,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSettings {
    #[serde(default)]
    pub follow_symlinks: bool,
    #[serde(default)]
    pub use_ignore_files: bool,
    #[serde(default)]
    pub use_global_ignore: bool,
    #[serde(default)]
    pub max_results: u32,
    #[serde(default)]
    pub show_line_numbers: bool,
    #[serde(default)]
    pub preview_enabled: bool,
    #[serde(default)]
    pub match_whole_word: bool,
    #[serde(default)]
    pub match_case: bool,
    #[serde(default)]
    pub use_regex: bool,
    #[serde(default)]
    pub exclude_pattern: Vec<String>,
    #[serde(default)]
    pub include_pattern: Vec<String>,
    #[serde(default)]
    pub search_on_type: bool,
    #[serde(default)]
    pub search_on_type_debounce: u32,
    #[serde(default)]
    pub smart_case: bool,
}

impl Default for SearchSettings {
    fn default() -> Self {
        Self {
            follow_symlinks: false,
            use_ignore_files: true,
            use_global_ignore: true,
            max_results: 10000,
            show_line_numbers: true,
            preview_enabled: true,
            match_whole_word: false,
            match_case: false,
            use_regex: false,
            exclude_pattern: vec!["**/node_modules/**".to_string(), "**/target/**".to_string(), "**/.git/**".to_string(), "**/dist/**".to_string(), "**/build/**".to_string()],
            include_pattern: vec![],
            search_on_type: true,
            search_on_type_debounce: 300,
            smart_case: true,
        }
    }
}

impl Settings {
    pub fn load_from_file(path: &PathBuf) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let settings: Settings = serde_json::from_str(&content)?;
        Ok(settings)
    }

    pub fn save_to_file(&self, path: &PathBuf) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn merge(&mut self, other: Settings) {
        // Deep merge logic would go here
        // For now, simple overwrite
        *self = other;
    }
}