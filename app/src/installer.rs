//! Pure-Rust Real-World First-Run Setup & Desktop Installation Engine (`v3.0.16`).
//!
//! When a user downloads `rustpad.exe` (Windows) or `rustpad` (Linux) and launches it for the
//! first time, RustPad opens a native 60–120Hz Setup & Installation Wizard — just like a
//! production desktop installer — with zero external crates and zero flashing console windows:
//!   - Copies the executable + icon assets into the user's application directory
//!     (`%LOCALAPPDATA%\Programs\RustPad` on Windows, `~/.local/share/rustpad` on Linux)
//!   - Creates real Desktop & Start Menu / App Launcher shortcuts (`.lnk` on Windows via COM
//!     `IShellLinkW` + pure-Rust `[MS-SHLLINK]` fallback; `.desktop` on Linux)
//!   - Registers per-user `.docx` and `.pdf` "Open with RustPad" file associations
//!   - Optionally adds `rustpad` CLI to User `PATH` (`HKCU\Environment\Path` / `~/.local/bin`)
//!   - Registers a real Uninstaller (`HKCU\...\Uninstall\RustPad` in Windows Installed Apps
//!     and `--uninstall` CLI self-cleanup)
//!   - Supports 1-click "Run Portable (Skip Installation)" for USB / air-gapped workflows.

use rustpad_render::{active_theme_mode, save_theme_preference, ThemeMode};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};

const EMBEDDED_ICO: &[u8] = include_bytes!("../../assets/icons/rustpad.ico");
const EMBEDDED_PNG_256: &[u8] = include_bytes!("../../assets/icons/icon-256.png");

/// Wizard step inside the full-window First-Run Setup & Desktop Installer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupStep {
    /// Step 1: Welcome, binary verification & MIT License agreement.
    Welcome,
    /// Step 2: Target install directory & real OS integration checkboxes.
    Options,
    /// Step 3: Studio theme live preview & default author profile.
    Personalize,
    /// Step 4a: Asynchronous real-time installation progress & log stream.
    Installing,
    /// Step 4b: Installation complete summary & launch button.
    Completed,
}

impl SetupStep {
    pub const fn index(self) -> usize {
        match self {
            Self::Welcome => 0,
            Self::Options => 1,
            Self::Personalize => 2,
            Self::Installing | Self::Completed => 3,
        }
    }
}

/// Active text input inside the Setup Wizard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupFocusField {
    None,
    InstallDir,
    AuthorName,
}

/// Which checkbox option is toggled in Step 2 / Step 1 / Step 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupToggleOption {
    AcceptLicense,
    DesktopShortcut,
    StartMenuEntry,
    FileAssociations,
    AddToUserPath,
    RegisterUninstaller,
    LaunchHomeOnFinish,
}

/// User-selected installation configuration.
#[derive(Debug, Clone)]
pub struct SetupOptions {
    pub accepted_license: bool,
    pub install_dir: String,
    pub desktop_shortcut: bool,
    pub start_menu_entry: bool,
    pub file_associations: bool,
    pub add_to_user_path: bool,
    pub register_uninstaller: bool,
    pub selected_theme: ThemeMode,
    pub author_name: String,
    pub launch_home_on_finish: bool,
}

impl Default for SetupOptions {
    fn default() -> Self {
        Self {
            accepted_license: true,
            install_dir: default_install_dir().to_string_lossy().into_owned(),
            desktop_shortcut: true,
            start_menu_entry: true,
            file_associations: true,
            add_to_user_path: true,
            register_uninstaller: true,
            selected_theme: active_theme_mode(),
            author_name: detect_os_username(),
            launch_home_on_finish: true,
        }
    }
}

/// Artifact summary produced when real installation finishes.
#[derive(Debug, Clone)]
pub struct InstallSummary {
    pub installed_exe: PathBuf,
    pub install_dir: PathBuf,
    pub desktop_shortcut_path: Option<PathBuf>,
    pub start_menu_path: Option<PathBuf>,
    pub cli_path_configured: bool,
    pub file_associations_registered: bool,
    pub uninstaller_registered: bool,
}

/// Persisted setup state read on startup to know whether first-run setup is needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistedSetupMode {
    NotConfigured,
    Portable {
        version: String,
        author_name: String,
    },
    Installed {
        version: String,
        install_dir: PathBuf,
        installed_exe: PathBuf,
        author_name: String,
    },
}

/// Full interactive state of the First-Run Setup & Installation Wizard.
pub struct SetupWizardState {
    pub step: SetupStep,
    pub options: SetupOptions,
    pub focus_field: SetupFocusField,
    pub progress_permille: Arc<AtomicU32>,
    pub current_task: Arc<Mutex<String>>,
    pub logs: Arc<Mutex<Vec<String>>>,
    pub install_rx: Option<Receiver<Result<InstallSummary, String>>>,
    pub result: Option<Result<InstallSummary, String>>,
    pub source_exe_path: PathBuf,
    pub source_exe_bytes: u64,
    pub existing_mode: PersistedSetupMode,
    pub custom_config_dir: Option<PathBuf>,
}

impl Default for SetupWizardState {
    fn default() -> Self {
        Self::new()
    }
}

impl SetupWizardState {
    pub fn new() -> Self {
        let source_exe_path = std::env::current_exe().unwrap_or_else(|_| {
            if cfg!(target_os = "windows") {
                PathBuf::from("rustpad.exe")
            } else {
                PathBuf::from("rustpad")
            }
        });
        let source_exe_bytes = fs::metadata(&source_exe_path)
            .map(|m| m.len())
            .unwrap_or(5_196_800);
        let existing_mode = load_persisted_setup_mode();
        let mut options = SetupOptions::default();
        match &existing_mode {
            PersistedSetupMode::Installed {
                install_dir,
                author_name,
                ..
            } => {
                options.install_dir = install_dir.to_string_lossy().into_owned();
                if !author_name.trim().is_empty() {
                    options.author_name = author_name.clone();
                }
            }
            PersistedSetupMode::Portable { author_name, .. } => {
                if !author_name.trim().is_empty() {
                    options.author_name = author_name.clone();
                }
            }
            PersistedSetupMode::NotConfigured => {}
        }
        Self {
            step: SetupStep::Welcome,
            options,
            focus_field: SetupFocusField::None,
            progress_permille: Arc::new(AtomicU32::new(0)),
            current_task: Arc::new(Mutex::new(String::from("Ready to install"))),
            logs: Arc::new(Mutex::new(Vec::new())),
            install_rx: None,
            result: None,
            source_exe_path,
            source_exe_bytes,
            existing_mode,
            custom_config_dir: None,
        }
    }

    pub fn toggle_option(&mut self, opt: SetupToggleOption) {
        match opt {
            SetupToggleOption::AcceptLicense => {
                self.options.accepted_license = !self.options.accepted_license;
            }
            SetupToggleOption::DesktopShortcut => {
                self.options.desktop_shortcut = !self.options.desktop_shortcut;
            }
            SetupToggleOption::StartMenuEntry => {
                self.options.start_menu_entry = !self.options.start_menu_entry;
            }
            SetupToggleOption::FileAssociations => {
                self.options.file_associations = !self.options.file_associations;
            }
            SetupToggleOption::AddToUserPath => {
                self.options.add_to_user_path = !self.options.add_to_user_path;
            }
            SetupToggleOption::RegisterUninstaller => {
                self.options.register_uninstaller = !self.options.register_uninstaller;
            }
            SetupToggleOption::LaunchHomeOnFinish => {
                self.options.launch_home_on_finish = !self.options.launch_home_on_finish;
            }
        }
    }

    pub fn start_installation(&mut self) {
        self.focus_field = SetupFocusField::None;
        self.step = SetupStep::Installing;
        self.progress_permille.store(40, Ordering::Relaxed);
        if let Ok(mut t) = self.current_task.lock() {
            *t = String::from("Preparing installation directory...");
        }
        if let Ok(mut lg) = self.logs.lock() {
            lg.clear();
            lg.push(format!(
                "Starting RustPad Studio v{} installation...",
                env!("CARGO_PKG_VERSION")
            ));
        }
        let opts = self.options.clone();
        let src_exe = self.source_exe_path.clone();
        let custom_cfg = self.custom_config_dir.clone();
        let prog = Arc::clone(&self.progress_permille);
        let task = Arc::clone(&self.current_task);
        let logs = Arc::clone(&self.logs);
        let (tx, rx) = mpsc::channel();
        self.install_rx = Some(rx);
        self.result = None;

        std::thread::spawn(move || {
            let outcome = perform_installation_with_config_dir(
                &src_exe,
                &opts,
                custom_cfg.as_deref(),
                &prog,
                &task,
                &logs,
            );
            let _ = tx.send(outcome);
        });
    }

    /// Polls background installation thread; returns `true` whenever UI should repaint.
    pub fn poll(&mut self) -> bool {
        let Some(rx) = &self.install_rx else {
            return false;
        };
        match rx.try_recv() {
            Ok(outcome) => {
                self.install_rx = None;
                self.progress_permille.store(1000, Ordering::Relaxed);
                self.result = Some(outcome);
                self.step = SetupStep::Completed;
                true
            }
            Err(mpsc::TryRecvError::Empty) => true,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.install_rx = None;
                self.result = Some(Err("Installer worker thread disconnected.".into()));
                self.step = SetupStep::Completed;
                true
            }
        }
    }
}

/// Returns the user's configuration directory for RustPad (`%APPDATA%\RustPad` or `~/.config/rustpad`).
pub fn rustpad_config_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("RUSTPAD_SETUP_CONFIG_DIR") {
        if !custom.trim().is_empty() {
            return PathBuf::from(custom);
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            if !appdata.trim().is_empty() {
                return PathBuf::from(appdata).join("RustPad");
            }
        }
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.trim().is_empty() {
            return PathBuf::from(xdg).join("rustpad");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.trim().is_empty() {
            return PathBuf::from(home).join(".config").join("rustpad");
        }
    }
    PathBuf::from(".rustpad_config")
}

pub fn setup_state_file_path() -> PathBuf {
    rustpad_config_dir().join("setup_state.conf")
}

/// Default per-user installation directory requiring zero Administrator / UAC elevation.
pub fn default_install_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(local) = win32_known_folder(0x001C)
            .or_else(|| std::env::var("LOCALAPPDATA").ok().map(PathBuf::from))
        {
            return local.join("Programs").join("RustPad");
        }
        if let Ok(profile) = std::env::var("USERPROFILE") {
            return PathBuf::from(profile)
                .join("AppData")
                .join("Local")
                .join("Programs")
                .join("RustPad");
        }
        return PathBuf::from("C:\\RustPad");
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
            if !xdg_data.trim().is_empty() {
                return PathBuf::from(xdg_data).join("rustpad");
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            if !home.trim().is_empty() {
                return PathBuf::from(home)
                    .join(".local")
                    .join("share")
                    .join("rustpad");
            }
        }
        PathBuf::from("/tmp/rustpad")
    }
}

pub fn installed_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "rustpad.exe"
    } else {
        "rustpad"
    }
}

pub fn detect_os_username() -> String {
    for var in ["USERNAME", "USER", "LOGNAME"] {
        if let Ok(val) = std::env::var(var) {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    String::from("RustPad User")
}

pub fn os_platform_label() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "Windows 10/11 (x86_64 Native Win32)"
    }
    #[cfg(all(target_os = "windows", not(target_arch = "x86_64")))]
    {
        "Windows (Native Win32)"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "Linux Desktop (x86_64 Native X11)"
    }
    #[cfg(all(target_os = "linux", not(target_arch = "x86_64")))]
    {
        "Linux Desktop"
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        "Desktop Native"
    }
}

/// Parses `setup_state.conf` from disk.
pub fn load_persisted_setup_mode() -> PersistedSetupMode {
    load_persisted_setup_mode_from(&setup_state_file_path())
}

pub fn load_persisted_setup_mode_from(path: &Path) -> PersistedSetupMode {
    let Ok(raw) = fs::read_to_string(path) else {
        return PersistedSetupMode::NotConfigured;
    };
    let mut mode = "";
    let mut version = String::new();
    let mut install_dir = PathBuf::new();
    let mut installed_exe = PathBuf::new();
    let mut author_name = String::new();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim();
            let v = v.trim();
            match k {
                "mode" => mode = v,
                "version" => version = v.to_string(),
                "install_dir" => install_dir = PathBuf::from(v),
                "installed_exe" => installed_exe = PathBuf::from(v),
                "author_name" => author_name = v.to_string(),
                _ => {}
            }
        }
    }

    match mode {
        "portable" => PersistedSetupMode::Portable {
            version,
            author_name,
        },
        "installed" => PersistedSetupMode::Installed {
            version,
            install_dir,
            installed_exe,
            author_name,
        },
        _ => PersistedSetupMode::NotConfigured,
    }
}

/// Determines whether launching `rustpad` with no file arguments should open the
/// First-Run Setup & Desktop Installation Wizard.
pub fn should_launch_first_run_setup() -> bool {
    if std::env::var("RUSTPAD_SKIP_SETUP").ok().as_deref() == Some("1") {
        return false;
    }
    match load_persisted_setup_mode() {
        PersistedSetupMode::NotConfigured => true,
        PersistedSetupMode::Portable { .. } => false,
        PersistedSetupMode::Installed { installed_exe, .. } => {
            // If the user manually deleted the installed folder and downloaded a fresh binary,
            // offer the setup wizard again so shortcuts and binary are restored cleanly.
            !installed_exe.as_os_str().is_empty() && !installed_exe.exists()
        }
    }
}

/// Saves Portable Mode choice so the user goes straight to Studio on subsequent launches.
pub fn save_portable_mode_choice(theme: ThemeMode, author_name: &str) -> Result<(), String> {
    save_theme_preference(theme);
    let dir = rustpad_config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {e}"))?;
    let clean_author = sanitize_single_line(author_name, "RustPad User");
    let body = format!(
        "# RustPad Studio First-Run Setup State\n\
         mode=portable\n\
         version={ver}\n\
         author_name={author}\n",
        ver = env!("CARGO_PKG_VERSION"),
        author = clean_author,
    );
    fs::write(setup_state_file_path(), body)
        .map_err(|e| format!("Failed to write setup_state.conf: {e}"))
}

fn sanitize_single_line(s: &str, fallback: &str) -> String {
    let cleaned: String = s
        .chars()
        .filter(|&c| c != '\n' && c != '\r')
        .collect::<String>()
        .trim()
        .to_string();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

fn push_log(
    logs: &Arc<Mutex<Vec<String>>>,
    task: &Arc<Mutex<String>>,
    prog: &Arc<AtomicU32>,
    permille: u32,
    msg: impl Into<String>,
) {
    let s = msg.into();
    prog.store(permille.min(1000), Ordering::Relaxed);
    if let Ok(mut t) = task.lock() {
        *t = s.clone();
    }
    if let Ok(mut lg) = logs.lock() {
        lg.push(s);
    }
}

fn same_canonical_file(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => false,
    }
}

/// Executes the real OS installation on a worker thread (or via `rustpad --install`).
pub fn perform_installation_blocking(
    source_exe: &Path,
    options: &SetupOptions,
    prog: &Arc<AtomicU32>,
    task: &Arc<Mutex<String>>,
    logs: &Arc<Mutex<Vec<String>>>,
) -> Result<InstallSummary, String> {
    perform_installation_with_config_dir(source_exe, options, None, prog, task, logs)
}

pub fn perform_installation_with_config_dir(
    source_exe: &Path,
    options: &SetupOptions,
    custom_config_dir: Option<&Path>,
    prog: &Arc<AtomicU32>,
    task: &Arc<Mutex<String>>,
    logs: &Arc<Mutex<Vec<String>>>,
) -> Result<InstallSummary, String> {
    let install_dir_str = options.install_dir.trim();
    if install_dir_str.is_empty() {
        return Err("Installation folder path cannot be empty.".into());
    }
    let install_dir = PathBuf::from(install_dir_str);

    // Step 1: Create target directory & copy binary + icon assets
    push_log(
        logs,
        task,
        prog,
        150,
        format!("Creating folder '{}'...", install_dir.display()),
    );
    fs::create_dir_all(&install_dir).map_err(|e| {
        format!(
            "Could not create installation folder '{}': {e}",
            install_dir.display()
        )
    })?;

    let target_exe = install_dir.join(installed_binary_name());
    push_log(
        logs,
        task,
        prog,
        320,
        format!("Installing executable to '{}'...", target_exe.display()),
    );

    if !same_canonical_file(source_exe, &target_exe) {
        if source_exe.exists() {
            let staging = install_dir.join(format!("{}.new", installed_binary_name()));
            fs::copy(source_exe, &staging).map_err(|e| {
                format!(
                    "Failed to copy '{}' to '{}': {e}",
                    source_exe.display(),
                    staging.display()
                )
            })?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&staging, fs::Permissions::from_mode(0o755));
            }
            if let Err(_rename_err) = fs::rename(&staging, &target_exe) {
                // On Windows, if an older instance is open, fall back to direct copy or surface clear message
                fs::copy(&staging, &target_exe).map_err(|e| {
                    let _ = fs::remove_file(&staging);
                    format!(
                        "Could not overwrite '{}' (close any other running RustPad window first): {e}",
                        target_exe.display()
                    )
                })?;
                let _ = fs::remove_file(&staging);
            }
        } else {
            return Err(format!(
                "Source executable '{}' was not found.",
                source_exe.display()
            ));
        }
    } else {
        push_log(
            logs,
            task,
            prog,
            340,
            "Executable is already running from target folder — refreshing integration...",
        );
    }

    // Write standalone icon assets alongside binary
    let ico_target = install_dir.join("rustpad.ico");
    let _ = fs::write(&ico_target, EMBEDDED_ICO);
    let png_target = install_dir.join("rustpad.png");
    let _ = fs::write(&png_target, EMBEDDED_PNG_256);

    // Step 2: Desktop & Start Menu / Launcher shortcuts
    let mut desktop_shortcut_path = None;
    let mut start_menu_path = None;

    if options.desktop_shortcut || options.start_menu_entry {
        push_log(
            logs,
            task,
            prog,
            550,
            "Creating Desktop & Start Menu shortcuts...",
        );
        let (desk, menu) = create_os_shortcuts(
            &install_dir,
            &target_exe,
            &png_target,
            options.desktop_shortcut,
            options.start_menu_entry,
        )?;
        desktop_shortcut_path = desk;
        start_menu_path = menu;
        if let Some(ref dp) = desktop_shortcut_path {
            push_log(
                logs,
                task,
                prog,
                620,
                format!("Created Desktop shortcut: {}", dp.display()),
            );
        }
        if let Some(ref mp) = start_menu_path {
            push_log(
                logs,
                task,
                prog,
                670,
                format!("Registered launcher entry: {}", mp.display()),
            );
        }
    }

    // Step 3: File associations (.docx & .pdf "Open with RustPad")
    let mut file_associations_registered = false;
    if options.file_associations {
        push_log(
            logs,
            task,
            prog,
            780,
            "Registering .docx and .pdf file associations...",
        );
        register_os_file_associations(&install_dir, &target_exe)?;
        file_associations_registered = true;
    }

    // Step 4: User PATH + Uninstaller registration
    let mut cli_path_configured = false;
    if options.add_to_user_path {
        push_log(
            logs,
            task,
            prog,
            870,
            "Configuring 'rustpad' command in user PATH...",
        );
        if configure_user_cli_path(&install_dir, &target_exe).is_ok() {
            cli_path_configured = true;
        }
    }

    let mut uninstaller_registered = false;
    if options.register_uninstaller {
        push_log(logs, task, prog, 930, "Registering uninstaller metadata...");
        let exe_bytes = fs::metadata(&target_exe)
            .map(|m| m.len())
            .unwrap_or(5_196_800);
        register_os_uninstaller(&install_dir, &target_exe, exe_bytes)?;
        uninstaller_registered = true;
    }

    // Step 5: Persist theme + setup_state.conf
    push_log(
        logs,
        task,
        prog,
        980,
        "Saving studio preferences & setup profile...",
    );
    save_theme_preference(options.selected_theme);
    let cfg_dir = custom_config_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(rustpad_config_dir);
    fs::create_dir_all(&cfg_dir).map_err(|e| format!("Failed to create config dir: {e}"))?;
    let clean_author = sanitize_single_line(&options.author_name, "RustPad User");
    let state_body = format!(
        "# RustPad Studio First-Run Setup State\n\
         mode=installed\n\
         version={ver}\n\
         install_dir={dir}\n\
         installed_exe={exe}\n\
         author_name={author}\n\
         desktop_shortcut={desk}\n\
         start_menu_entry={menu}\n\
         file_associations={assoc}\n\
         add_to_user_path={path_opt}\n\
         register_uninstaller={uninst}\n",
        ver = env!("CARGO_PKG_VERSION"),
        dir = install_dir.display(),
        exe = target_exe.display(),
        author = clean_author,
        desk = options.desktop_shortcut,
        menu = options.start_menu_entry,
        assoc = options.file_associations,
        path_opt = options.add_to_user_path,
        uninst = options.register_uninstaller,
    );
    fs::write(cfg_dir.join("setup_state.conf"), state_body)
        .map_err(|e| format!("Failed to save setup_state.conf: {e}"))?;

    push_log(
        logs,
        task,
        prog,
        1000,
        format!(
            "RustPad Studio v{} installed cleanly!",
            env!("CARGO_PKG_VERSION")
        ),
    );

    Ok(InstallSummary {
        installed_exe: target_exe,
        install_dir,
        desktop_shortcut_path,
        start_menu_path,
        cli_path_configured,
        file_associations_registered,
        uninstaller_registered,
    })
}

/// Native folder browser dialog (`SHBrowseForFolderW` on Windows, `zenity` on Linux).
pub fn browse_install_folder_dialog(current_dir: &str) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let _ = current_dir;
        return win32_browse_folder("Choose RustPad Studio Installation Folder");
    }
    #[cfg(target_os = "linux")]
    {
        let mut cmd = std::process::Command::new("zenity");
        cmd.args([
            "--file-selection",
            "--directory",
            "--title=Choose RustPad Studio Installation Folder",
        ]);
        if !current_dir.trim().is_empty() {
            cmd.arg(format!("--filename={}/", current_dir.trim()));
        }
        if let Ok(out) = cmd.output() {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !s.is_empty() {
                    return Some(PathBuf::from(s));
                }
            }
        }
        None
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = current_dir;
        None
    }
}

/// Real `--uninstall` implementation invoked from Windows Installed Apps or Start Menu / CLI.
pub fn run_uninstaller(quiet: bool) -> Result<(), String> {
    let state = load_persisted_setup_mode();
    let (install_dir, installed_exe) = match state {
        PersistedSetupMode::Installed {
            install_dir,
            installed_exe,
            ..
        } => (install_dir, installed_exe),
        _ => {
            let def = default_install_dir();
            let exe = def.join(installed_binary_name());
            (def, exe)
        }
    };

    remove_os_integration(&install_dir, &installed_exe);

    let state_file = setup_state_file_path();
    let _ = fs::remove_file(&state_file);

    #[cfg(target_os = "windows")]
    {
        let _ = fs::remove_file(install_dir.join("rustpad.ico"));
        let _ = fs::remove_file(install_dir.join("rustpad.png"));
        let cur_exe = std::env::current_exe().unwrap_or_default();
        if same_canonical_file(&cur_exe, &installed_exe) && installed_exe.exists() {
            // Schedule silent background self-removal after this process exits (`CREATE_NO_WINDOW`)
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let cmd_line = format!(
                "ping 127.0.0.1 -n 2 > nul & del /f /q \"{}\" & rmdir \"{}\"",
                installed_exe.display(),
                install_dir.display()
            );
            let _ = std::process::Command::new("cmd.exe")
                .args(["/C", &cmd_line])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
        } else {
            let _ = fs::remove_file(&installed_exe);
            let _ = fs::remove_dir(&install_dir);
        }
        if !quiet {
            win32_message_box(
                "RustPad Studio Uninstaller",
                "RustPad Studio has been cleanly uninstalled from your computer.",
            );
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = fs::remove_file(install_dir.join("rustpad.ico"));
        let _ = fs::remove_file(install_dir.join("rustpad.png"));
        let _ = fs::remove_file(install_dir.join("uninstall.sh"));
        let _ = fs::remove_file(&installed_exe);
        let _ = fs::remove_dir(&install_dir);
        if !quiet {
            println!("RustPad Studio has been cleanly uninstalled.");
        }
    }

    Ok(())
}

// ============================================================================
// Pure-Rust `[MS-SHLLINK]` Binary `.lnk` Encoder (Cross-Platform Testable!)
// ============================================================================

/// Builds a valid Windows Shell Link (`.lnk`) binary conforming to `[MS-SHLLINK]` §2.1 & §2.5.4
/// (`HasName | HasWorkingDir | HasIconLocation | IsUnicode | ForceNoLinkInfo | HasExpString`
/// + `EnvironmentVariableDataBlock` `0xA000_0001`). Used both as an automatic fallback if COM
///   `IShellLinkW` is unavailable and for deterministic unit testing on any OS.
pub fn build_ms_shllink_bytes(
    target_exe: &str,
    arguments: &str,
    working_dir: &str,
    description: &str,
    icon_path: &str,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(1200);

    let has_args = !arguments.trim().is_empty();
    // LinkFlags:
    // HasName (0x04) | HasWorkingDir (0x10) | HasArguments (0x20 optional)
    // | HasIconLocation (0x40) | IsUnicode (0x80) | ForceNoLinkInfo (0x100) | HasExpString (0x200)
    let mut flags: u32 = 0x0004 | 0x0010 | 0x0040 | 0x0080 | 0x0100 | 0x0200;
    if has_args {
        flags |= 0x0020;
    }

    // 1. ShellLinkHeader (76 bytes = 0x4C)
    out.extend_from_slice(&0x0000_004Cu32.to_le_bytes()); // HeaderSize
    out.extend_from_slice(&[
        // LinkCLSID = 00021401-0000-0000-C000-000000000046
        0x01, 0x14, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x46,
    ]);
    out.extend_from_slice(&flags.to_le_bytes()); // LinkFlags
    out.extend_from_slice(&0x0000_0020u32.to_le_bytes()); // FileAttributes (FILE_ATTRIBUTE_ARCHIVE)
    out.extend_from_slice(&0u64.to_le_bytes()); // CreationTime
    out.extend_from_slice(&0u64.to_le_bytes()); // AccessTime
    out.extend_from_slice(&0u64.to_le_bytes()); // WriteTime
    out.extend_from_slice(&0u32.to_le_bytes()); // FileSize
    out.extend_from_slice(&0i32.to_le_bytes()); // IconIndex (0)
    out.extend_from_slice(&1u32.to_le_bytes()); // ShowCommand (SW_SHOWNORMAL = 1)
    out.extend_from_slice(&0u16.to_le_bytes()); // HotKey
    out.extend_from_slice(&0u16.to_le_bytes()); // Reserved1
    out.extend_from_slice(&0u32.to_le_bytes()); // Reserved2
    out.extend_from_slice(&0u32.to_le_bytes()); // Reserved3
    debug_assert_eq!(out.len(), 76);

    // 2. StringData (CountCharacters u16 + UTF-16LE chars, no null terminator):
    // Order mandated by [MS-SHLLINK] §2.4: NAME_STRING, WORKING_DIR, COMMAND_LINE_ARGUMENTS, ICON_LOCATION
    let push_string_data = |buf: &mut Vec<u8>, s: &str| {
        let u16s: Vec<u16> = s.encode_utf16().collect();
        let len = u16s.len().min(u16::MAX as usize);
        buf.extend_from_slice(&(len as u16).to_le_bytes());
        for ch in &u16s[..len] {
            buf.extend_from_slice(&ch.to_le_bytes());
        }
    };
    push_string_data(&mut out, description);
    push_string_data(&mut out, working_dir);
    if has_args {
        push_string_data(&mut out, arguments);
    }
    push_string_data(&mut out, icon_path);

    // 3. ExtraData: EnvironmentVariableDataBlock (788 bytes = 0x0000_0314, sig = 0xA000_0001)
    out.extend_from_slice(&0x0000_0314u32.to_le_bytes()); // BlockSize = 788
    out.extend_from_slice(&0xA000_0001u32.to_le_bytes()); // BlockSignature

    // TargetAnsi (260 bytes null-padded)
    let mut ansi = [0u8; 260];
    let ascii_bytes: Vec<u8> = target_exe
        .bytes()
        .map(|b| if b < 0x80 { b } else { b'?' })
        .take(259)
        .collect();
    ansi[..ascii_bytes.len()].copy_from_slice(&ascii_bytes);
    out.extend_from_slice(&ansi);

    // TargetUnicode (520 bytes = 260 u16s null-padded)
    let mut wide = [0u16; 260];
    for (i, ch) in target_exe.encode_utf16().take(259).enumerate() {
        wide[i] = ch;
    }
    for ch in wide {
        out.extend_from_slice(&ch.to_le_bytes());
    }

    // TerminalBlock (4 zero bytes)
    out.extend_from_slice(&0u32.to_le_bytes());
    out
}

// ============================================================================
// Platform Integration: Windows (Native Win32 + COM, Zero Console Windows)
// ============================================================================

#[cfg(target_os = "windows")]
#[repr(C)]
struct GUID {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[cfg(target_os = "windows")]
fn to_wide_z(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
fn win32_known_folder(csidl: i32) -> Option<PathBuf> {
    #[link(name = "shell32")]
    extern "system" {
        fn SHGetFolderPathW(
            hwnd: isize,
            csidl: i32,
            hToken: isize,
            dwFlags: u32,
            pszPath: *mut u16,
        ) -> i32;
    }
    let mut buf = [0u16; 260];
    let hr = unsafe { SHGetFolderPathW(0, csidl, 0, 0, buf.as_mut_ptr()) };
    if hr == 0 {
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        if len > 0 {
            return Some(PathBuf::from(String::from_utf16_lossy(&buf[..len])));
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn win32_browse_folder(title: &str) -> Option<PathBuf> {
    #[repr(C)]
    struct BROWSEINFOW {
        hwnd_owner: isize,
        pidl_root: *const std::ffi::c_void,
        psz_display_name: *mut u16,
        lpsz_title: *const u16,
        ul_flags: u32,
        lpfn: Option<unsafe extern "system" fn(isize, u32, isize, isize) -> i32>,
        l_param: isize,
        i_image: i32,
    }
    #[link(name = "shell32")]
    extern "system" {
        fn SHBrowseForFolderW(lpbi: *const BROWSEINFOW) -> *mut std::ffi::c_void;
        fn SHGetPathFromIDListW(pidl: *const std::ffi::c_void, pszPath: *mut u16) -> i32;
    }
    #[link(name = "ole32")]
    extern "system" {
        fn CoInitializeEx(pvReserved: *mut std::ffi::c_void, dwCoInit: u32) -> i32;
        fn CoTaskMemFree(pv: *mut std::ffi::c_void);
    }

    unsafe {
        let _ = CoInitializeEx(std::ptr::null_mut(), 0x2); // COINIT_APARTMENTTHREADED
        let mut disp = [0u16; 260];
        let title_w = to_wide_z(title);
        let bi = BROWSEINFOW {
            hwnd_owner: 0,
            pidl_root: std::ptr::null(),
            psz_display_name: disp.as_mut_ptr(),
            lpsz_title: title_w.as_ptr(),
            ul_flags: 0x0001 | 0x0040, // BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE
            lpfn: None,
            l_param: 0,
            i_image: 0,
        };
        let pidl = SHBrowseForFolderW(&bi);
        if pidl.is_null() {
            return None;
        }
        let mut out_buf = [0u16; 260];
        let ok = SHGetPathFromIDListW(pidl, out_buf.as_mut_ptr());
        CoTaskMemFree(pidl);
        if ok != 0 {
            let len = out_buf
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(out_buf.len());
            if len > 0 {
                return Some(PathBuf::from(String::from_utf16_lossy(&out_buf[..len])));
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn win32_message_box(caption: &str, text: &str) {
    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(hwnd: isize, lpText: *const u16, lpCaption: *const u16, uType: u32) -> i32;
    }
    let cap_w = to_wide_z(caption);
    let txt_w = to_wide_z(text);
    unsafe {
        let _ = MessageBoxW(0, txt_w.as_ptr(), cap_w.as_ptr(), 0x0000_0040); // MB_ICONINFORMATION
    }
}

#[cfg(target_os = "windows")]
fn write_windows_shortcut(
    lnk_path: &Path,
    target_exe: &Path,
    arguments: &str,
    working_dir: &Path,
    description: &str,
    icon_path: &Path,
) -> Result<(), String> {
    if let Some(parent) = lnk_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create shortcut dir: {e}"))?;
    }

    // Attempt native Windows COM `IShellLinkW` + `IPersistFile` first (100% native PIDL + Icon)
    if unsafe {
        try_com_create_shortcut(
            lnk_path,
            target_exe,
            arguments,
            working_dir,
            description,
            icon_path,
        )
    } {
        return Ok(());
    }

    // Fallback: Pure-Rust `[MS-SHLLINK]` binary writer
    let bytes = build_ms_shllink_bytes(
        &target_exe.to_string_lossy(),
        arguments,
        &working_dir.to_string_lossy(),
        description,
        &icon_path.to_string_lossy(),
    );
    fs::write(lnk_path, bytes)
        .map_err(|e| format!("Failed to write shortcut '{}': {e}", lnk_path.display()))
}

#[cfg(target_os = "windows")]
unsafe fn try_com_create_shortcut(
    lnk_path: &Path,
    target_exe: &Path,
    arguments: &str,
    working_dir: &Path,
    description: &str,
    icon_path: &Path,
) -> bool {
    #[repr(C)]
    struct IShellLinkWVtbl {
        query_interface: unsafe extern "system" fn(
            *mut std::ffi::c_void,
            *const GUID,
            *mut *mut std::ffi::c_void,
        ) -> i32,
        add_ref: unsafe extern "system" fn(*mut std::ffi::c_void) -> u32,
        release: unsafe extern "system" fn(*mut std::ffi::c_void) -> u32,
        get_path: usize,
        get_id_list: usize,
        set_id_list: usize,
        get_description: usize,
        set_description: unsafe extern "system" fn(*mut std::ffi::c_void, *const u16) -> i32,
        get_working_directory: usize,
        set_working_directory: unsafe extern "system" fn(*mut std::ffi::c_void, *const u16) -> i32,
        get_arguments: usize,
        set_arguments: unsafe extern "system" fn(*mut std::ffi::c_void, *const u16) -> i32,
        get_hotkey: usize,
        set_hotkey: usize,
        get_show_cmd: usize,
        set_show_cmd: usize,
        get_icon_location: usize,
        set_icon_location: unsafe extern "system" fn(*mut std::ffi::c_void, *const u16, i32) -> i32,
        set_relative_path: usize,
        resolve: usize,
        set_path: unsafe extern "system" fn(*mut std::ffi::c_void, *const u16) -> i32,
    }

    #[repr(C)]
    struct IShellLinkWObj {
        lp_vtbl: *const IShellLinkWVtbl,
    }

    #[repr(C)]
    struct IPersistFileVtbl {
        query_interface: usize,
        add_ref: usize,
        release: unsafe extern "system" fn(*mut std::ffi::c_void) -> u32,
        get_class_id: usize,
        is_dirty: usize,
        load: usize,
        save: unsafe extern "system" fn(*mut std::ffi::c_void, *const u16, i32) -> i32,
        save_completed: usize,
        get_cur_file: usize,
    }

    #[repr(C)]
    struct IPersistFileObj {
        lp_vtbl: *const IPersistFileVtbl,
    }

    #[link(name = "ole32")]
    extern "system" {
        fn CoInitializeEx(pvReserved: *mut std::ffi::c_void, dwCoInit: u32) -> i32;
        fn CoCreateInstance(
            rclsid: *const GUID,
            pUnkOuter: *mut std::ffi::c_void,
            dwClsContext: u32,
            riid: *const GUID,
            ppv: *mut *mut std::ffi::c_void,
        ) -> i32;
        fn CoUninitialize();
    }

    const CLSID_SHELL_LINK: GUID = GUID {
        data1: 0x0002_1401,
        data2: 0x0000,
        data3: 0x0000,
        data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
    };
    const IID_ISHELL_LINK_W: GUID = GUID {
        data1: 0x0002_14F9,
        data2: 0x0000,
        data3: 0x0000,
        data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
    };
    const IID_IPERSIST_FILE: GUID = GUID {
        data1: 0x0000_010B,
        data2: 0x0000,
        data3: 0x0000,
        data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
    };

    let co_hr = CoInitializeEx(std::ptr::null_mut(), 0x2); // COINIT_APARTMENTTHREADED
    let should_uninit = co_hr == 0 || co_hr == 1; // S_OK or S_FALSE

    let mut sl_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
    let hr = CoCreateInstance(
        &CLSID_SHELL_LINK,
        std::ptr::null_mut(),
        1, // CLSCTX_INPROC_SERVER
        &IID_ISHELL_LINK_W,
        &mut sl_ptr,
    );
    if hr < 0 || sl_ptr.is_null() {
        if should_uninit {
            CoUninitialize();
        }
        return false;
    }

    let sl = &*(sl_ptr as *const IShellLinkWObj);
    let target_w = to_wide_z(&target_exe.to_string_lossy());
    let work_w = to_wide_z(&working_dir.to_string_lossy());
    let desc_w = to_wide_z(description);
    let icon_w = to_wide_z(&icon_path.to_string_lossy());

    let _ = ((*sl.lp_vtbl).set_path)(sl_ptr, target_w.as_ptr());
    let _ = ((*sl.lp_vtbl).set_working_directory)(sl_ptr, work_w.as_ptr());
    let _ = ((*sl.lp_vtbl).set_description)(sl_ptr, desc_w.as_ptr());
    let _ = ((*sl.lp_vtbl).set_icon_location)(sl_ptr, icon_w.as_ptr(), 0);
    if !arguments.trim().is_empty() {
        let args_w = to_wide_z(arguments);
        let _ = ((*sl.lp_vtbl).set_arguments)(sl_ptr, args_w.as_ptr());
    }

    let mut pf_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
    let qi_hr = ((*sl.lp_vtbl).query_interface)(sl_ptr, &IID_IPERSIST_FILE, &mut pf_ptr);
    let mut saved_ok = false;
    if qi_hr >= 0 && !pf_ptr.is_null() {
        let pf = &*(pf_ptr as *const IPersistFileObj);
        let lnk_w = to_wide_z(&lnk_path.to_string_lossy());
        let save_hr = ((*pf.lp_vtbl).save)(pf_ptr, lnk_w.as_ptr(), 1);
        saved_ok = save_hr >= 0 && lnk_path.exists();
        let _ = ((*pf.lp_vtbl).release)(pf_ptr);
    }
    let _ = ((*sl.lp_vtbl).release)(sl_ptr);
    if should_uninit {
        CoUninitialize();
    }
    saved_ok
}

#[cfg(target_os = "windows")]
fn create_os_shortcuts(
    install_dir: &Path,
    target_exe: &Path,
    _png_target: &Path,
    desktop: bool,
    start_menu: bool,
) -> Result<(Option<PathBuf>, Option<PathBuf>), String> {
    let mut desk_out = None;
    let mut menu_out = None;

    if desktop {
        let desk_dir = win32_known_folder(0x0010).or_else(|| {
            std::env::var("USERPROFILE")
                .ok()
                .map(|p| PathBuf::from(p).join("Desktop"))
        });
        if let Some(dir) = desk_dir {
            let lnk = dir.join("RustPad Studio.lnk");
            write_windows_shortcut(
                &lnk,
                target_exe,
                "",
                install_dir,
                "RustPad Studio — Offline Native Word (.docx) & PDF Suite",
                target_exe,
            )?;
            desk_out = Some(lnk);
        }
    }

    if start_menu {
        let prog_dir = win32_known_folder(0x0002).or_else(|| {
            std::env::var("APPDATA").ok().map(|p| {
                PathBuf::from(p)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Start Menu")
                    .join("Programs")
            })
        });
        if let Some(dir) = prog_dir {
            let folder = dir.join("RustPad");
            fs::create_dir_all(&folder)
                .map_err(|e| format!("Failed to create Start Menu folder: {e}"))?;
            let main_lnk = folder.join("RustPad Studio.lnk");
            write_windows_shortcut(
                &main_lnk,
                target_exe,
                "",
                install_dir,
                "RustPad Studio — Offline Native Word (.docx) & PDF Suite",
                target_exe,
            )?;
            let uninst_lnk = folder.join("Uninstall RustPad.lnk");
            let _ = write_windows_shortcut(
                &uninst_lnk,
                target_exe,
                "--uninstall",
                install_dir,
                "Uninstall RustPad Studio",
                target_exe,
            );
            menu_out = Some(main_lnk);
        }
    }

    Ok((desk_out, menu_out))
}

#[cfg(target_os = "windows")]
mod winreg_hkcu {
    use super::to_wide_z;

    const HKEY_CURRENT_USER: isize = 0x8000_0001u32 as i32 as isize;
    const KEY_READ: u32 = 0x20019;
    const KEY_WRITE: u32 = 0x20006;
    const REG_SZ: u32 = 1;
    const REG_EXPAND_SZ: u32 = 2;
    const REG_DWORD: u32 = 4;

    #[link(name = "advapi32")]
    extern "system" {
        fn RegCreateKeyExW(
            hKey: isize,
            lpSubKey: *const u16,
            Reserved: u32,
            lpClass: *mut u16,
            dwOptions: u32,
            samDesired: u32,
            lpSecurityAttributes: *const std::ffi::c_void,
            phkResult: *mut isize,
            lpdwDisposition: *mut u32,
        ) -> i32;
        fn RegOpenKeyExW(
            hKey: isize,
            lpSubKey: *const u16,
            ulOptions: u32,
            samDesired: u32,
            phkResult: *mut isize,
        ) -> i32;
        fn RegSetValueExW(
            hKey: isize,
            lpValueName: *const u16,
            Reserved: u32,
            dwType: u32,
            lpData: *const u8,
            cbData: u32,
        ) -> i32;
        fn RegQueryValueExW(
            hKey: isize,
            lpValueName: *const u16,
            lpReserved: *const u32,
            lpType: *mut u32,
            lpData: *mut u8,
            lpcbData: *mut u32,
        ) -> i32;
        fn RegDeleteValueW(hKey: isize, lpValueName: *const u16) -> i32;
        fn RegDeleteTreeW(hKey: isize, lpSubKey: *const u16) -> i32;
        fn RegCloseKey(hKey: isize) -> i32;
    }

    pub fn set_sz(subkey: &str, name: Option<&str>, val: &str) -> Result<(), String> {
        set_str_typed(subkey, name, val, REG_SZ)
    }

    pub fn set_expand_sz(subkey: &str, name: Option<&str>, val: &str) -> Result<(), String> {
        set_str_typed(subkey, name, val, REG_EXPAND_SZ)
    }

    fn set_str_typed(
        subkey: &str,
        name: Option<&str>,
        val: &str,
        reg_type: u32,
    ) -> Result<(), String> {
        unsafe {
            let sk = to_wide_z(subkey);
            let mut hkey: isize = 0;
            let st = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                sk.as_ptr(),
                0,
                std::ptr::null_mut(),
                0,
                KEY_WRITE,
                std::ptr::null(),
                &mut hkey,
                std::ptr::null_mut(),
            );
            if st != 0 {
                return Err(format!("RegCreateKeyExW({subkey}) failed with status {st}"));
            }
            let val_w = to_wide_z(val);
            let name_w = name.map(to_wide_z);
            let name_ptr = name_w
                .as_ref()
                .map(|v| v.as_ptr())
                .unwrap_or(std::ptr::null());
            let st2 = RegSetValueExW(
                hkey,
                name_ptr,
                0,
                reg_type,
                val_w.as_ptr() as *const u8,
                (val_w.len() * 2) as u32,
            );
            RegCloseKey(hkey);
            if st2 != 0 {
                return Err(format!("RegSetValueExW({subkey}) failed with status {st2}"));
            }
            Ok(())
        }
    }

    pub fn set_dword(subkey: &str, name: &str, val: u32) -> Result<(), String> {
        unsafe {
            let sk = to_wide_z(subkey);
            let mut hkey: isize = 0;
            if RegCreateKeyExW(
                HKEY_CURRENT_USER,
                sk.as_ptr(),
                0,
                std::ptr::null_mut(),
                0,
                KEY_WRITE,
                std::ptr::null(),
                &mut hkey,
                std::ptr::null_mut(),
            ) != 0
            {
                return Err(format!("RegCreateKeyExW({subkey}) failed"));
            }
            let name_w = to_wide_z(name);
            let st = RegSetValueExW(
                hkey,
                name_w.as_ptr(),
                0,
                REG_DWORD,
                (&val as *const u32) as *const u8,
                4,
            );
            RegCloseKey(hkey);
            if st != 0 {
                return Err(format!("RegSetValueExW({name}) failed"));
            }
            Ok(())
        }
    }

    pub fn get_string(subkey: &str, name: &str) -> Option<String> {
        unsafe {
            let sk = to_wide_z(subkey);
            let mut hkey: isize = 0;
            if RegOpenKeyExW(HKEY_CURRENT_USER, sk.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
                return None;
            }
            let name_w = to_wide_z(name);
            let mut sz: u32 = 0;
            let mut ty: u32 = 0;
            let st = RegQueryValueExW(
                hkey,
                name_w.as_ptr(),
                std::ptr::null(),
                &mut ty,
                std::ptr::null_mut(),
                &mut sz,
            );
            if st != 0 || sz < 2 {
                RegCloseKey(hkey);
                return None;
            }
            let mut buf = vec![0u16; (sz as usize + 1) / 2];
            let st2 = RegQueryValueExW(
                hkey,
                name_w.as_ptr(),
                std::ptr::null(),
                &mut ty,
                buf.as_mut_ptr() as *mut u8,
                &mut sz,
            );
            RegCloseKey(hkey);
            if st2 != 0 {
                return None;
            }
            let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            Some(String::from_utf16_lossy(&buf[..end]))
        }
    }

    pub fn delete_tree(subkey: &str) {
        unsafe {
            let sk = to_wide_z(subkey);
            let _ = RegDeleteTreeW(HKEY_CURRENT_USER, sk.as_ptr());
        }
    }

    pub fn delete_value(subkey: &str, name: &str) {
        unsafe {
            let sk = to_wide_z(subkey);
            let mut hkey: isize = 0;
            if RegOpenKeyExW(HKEY_CURRENT_USER, sk.as_ptr(), 0, KEY_WRITE, &mut hkey) == 0 {
                let nw = to_wide_z(name);
                let _ = RegDeleteValueW(hkey, nw.as_ptr());
                RegCloseKey(hkey);
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn notify_shell_associations_changed() {
    #[link(name = "shell32")]
    extern "system" {
        fn SHChangeNotify(
            wEventId: i32,
            uFlags: u32,
            dwItem1: *const std::ffi::c_void,
            dwItem2: *const std::ffi::c_void,
        );
    }
    unsafe {
        // SHCNE_ASSOCCHANGED = 0x0800_0000, SHCNF_IDLIST = 0x0000
        SHChangeNotify(0x0800_0000, 0x0000, std::ptr::null(), std::ptr::null());
    }
}

#[cfg(target_os = "windows")]
fn register_os_file_associations(install_dir: &Path, target_exe: &Path) -> Result<(), String> {
    let exe_str = target_exe.to_string_lossy();
    let open_cmd = format!("\"{exe_str}\" \"%1\"");
    let icon_val = format!("\"{exe_str}\",0");

    for (prog_id, ext, friendly) in [
        (
            "RustPad.Document.docx",
            ".docx",
            "RustPad Word Document (.docx)",
        ),
        (
            "RustPad.Document.pdf",
            ".pdf",
            "RustPad PDF Document (.pdf)",
        ),
    ] {
        let base = format!("Software\\Classes\\{prog_id}");
        winreg_hkcu::set_sz(&base, None, friendly)?;
        winreg_hkcu::set_sz(&format!("{base}\\DefaultIcon"), None, &icon_val)?;
        winreg_hkcu::set_sz(&format!("{base}\\shell\\open\\command"), None, &open_cmd)?;
        winreg_hkcu::set_sz(
            &format!("Software\\Classes\\{ext}\\OpenWithProgids"),
            Some(prog_id),
            "",
        )?;
    }

    let app_paths = "Software\\Microsoft\\Windows\\CurrentVersion\\App Paths\\rustpad.exe";
    winreg_hkcu::set_sz(app_paths, None, &exe_str)?;
    winreg_hkcu::set_sz(app_paths, Some("Path"), &install_dir.to_string_lossy())?;

    notify_shell_associations_changed();
    Ok(())
}

#[cfg(target_os = "windows")]
fn configure_user_cli_path(install_dir: &Path, _target_exe: &Path) -> Result<(), String> {
    let dir_str = install_dir.to_string_lossy();
    let existing = winreg_hkcu::get_string("Environment", "Path").unwrap_or_default();
    let already_present = existing
        .split(';')
        .any(|seg| seg.trim().eq_ignore_ascii_case(dir_str.trim()));

    if !already_present {
        let updated = if existing.trim().is_empty() {
            dir_str.into_owned()
        } else if existing.ends_with(';') {
            format!("{existing}{dir_str}")
        } else {
            format!("{existing};{dir_str}")
        };
        winreg_hkcu::set_expand_sz("Environment", Some("Path"), &updated)?;

        #[link(name = "user32")]
        extern "system" {
            fn SendMessageTimeoutW(
                hWnd: isize,
                Msg: u32,
                wParam: usize,
                lParam: *const u16,
                fuFlags: u32,
                uTimeout: u32,
                lpdwResult: *mut usize,
            ) -> isize;
        }
        let env_w = to_wide_z("Environment");
        let mut res: usize = 0;
        unsafe {
            // HWND_BROADCAST = 0xFFFF, WM_SETTINGCHANGE = 0x001A, SMTO_ABORTIFHUNG = 0x0002
            let _ = SendMessageTimeoutW(0xFFFF, 0x001A, 0, env_w.as_ptr(), 0x0002, 200, &mut res);
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn register_os_uninstaller(
    install_dir: &Path,
    target_exe: &Path,
    exe_bytes: u64,
) -> Result<(), String> {
    let key = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\RustPad";
    let exe_str = target_exe.to_string_lossy();
    winreg_hkcu::set_sz(key, Some("DisplayName"), "RustPad Studio")?;
    winreg_hkcu::set_sz(key, Some("DisplayVersion"), env!("CARGO_PKG_VERSION"))?;
    winreg_hkcu::set_sz(key, Some("Publisher"), "Nour Yahyaoui - RustPad Studio")?;
    winreg_hkcu::set_sz(key, Some("DisplayIcon"), &format!("{exe_str},0"))?;
    winreg_hkcu::set_sz(key, Some("InstallLocation"), &install_dir.to_string_lossy())?;
    winreg_hkcu::set_sz(
        key,
        Some("UninstallString"),
        &format!("\"{exe_str}\" --uninstall"),
    )?;
    winreg_hkcu::set_sz(
        key,
        Some("QuietUninstallString"),
        &format!("\"{exe_str}\" --uninstall --quiet"),
    )?;
    winreg_hkcu::set_sz(
        key,
        Some("URLInfoAbout"),
        "https://rustpad-studio.vercel.app/",
    )?;
    winreg_hkcu::set_dword(key, "NoModify", 1)?;
    winreg_hkcu::set_dword(key, "NoRepair", 1)?;
    winreg_hkcu::set_dword(key, "EstimatedSize", (exe_bytes / 1024).max(1) as u32)?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn remove_os_integration(install_dir: &Path, _installed_exe: &Path) {
    if let Some(desk) = win32_known_folder(0x0010).or_else(|| {
        std::env::var("USERPROFILE")
            .ok()
            .map(|p| PathBuf::from(p).join("Desktop"))
    }) {
        let _ = fs::remove_file(desk.join("RustPad Studio.lnk"));
    }
    if let Some(prog) = win32_known_folder(0x0002).or_else(|| {
        std::env::var("APPDATA").ok().map(|p| {
            PathBuf::from(p)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
        })
    }) {
        let _ = fs::remove_dir_all(prog.join("RustPad"));
    }

    winreg_hkcu::delete_tree("Software\\Classes\\RustPad.Document.docx");
    winreg_hkcu::delete_tree("Software\\Classes\\RustPad.Document.pdf");
    winreg_hkcu::delete_value(
        "Software\\Classes\\.docx\\OpenWithProgids",
        "RustPad.Document.docx",
    );
    winreg_hkcu::delete_value(
        "Software\\Classes\\.pdf\\OpenWithProgids",
        "RustPad.Document.pdf",
    );
    winreg_hkcu::delete_tree(
        "Software\\Microsoft\\Windows\\CurrentVersion\\App Paths\\rustpad.exe",
    );
    winreg_hkcu::delete_tree("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\RustPad");

    if let Some(existing) = winreg_hkcu::get_string("Environment", "Path") {
        let target = install_dir.to_string_lossy();
        let filtered: Vec<&str> = existing
            .split(';')
            .filter(|seg| !seg.trim().is_empty() && !seg.trim().eq_ignore_ascii_case(target.trim()))
            .collect();
        let _ = winreg_hkcu::set_expand_sz("Environment", Some("Path"), &filtered.join(";"));
    }
    notify_shell_associations_changed();
}

// ============================================================================
// Platform Integration: Linux / Freedesktop.org Desktop Entries
// ============================================================================

#[cfg(not(target_os = "windows"))]
fn build_freedesktop_entry(target_exe: &Path, icon_png: &Path) -> String {
    format!(
        "[Desktop Entry]\n\
         Version=1.0\n\
         Type=Application\n\
         Name=RustPad Studio\n\
         GenericName=Word & PDF Studio\n\
         Comment=Offline-First Pure-Rust Native Word (.docx) & PDF Suite\n\
         Exec=\"{exe}\" %f\n\
         Icon={icon}\n\
         Terminal=false\n\
         Categories=Office;WordProcessor;Viewer;\n\
         MimeType=application/vnd.openxmlformats-officedocument.wordprocessingml.document;application/pdf;\n\
         StartupWMClass=rustpad\n",
        exe = target_exe.display(),
        icon = icon_png.display(),
    )
}

#[cfg(not(target_os = "windows"))]
fn linux_home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

#[cfg(not(target_os = "windows"))]
fn create_os_shortcuts(
    _install_dir: &Path,
    target_exe: &Path,
    png_target: &Path,
    desktop: bool,
    start_menu: bool,
) -> Result<(Option<PathBuf>, Option<PathBuf>), String> {
    let home = linux_home_dir();
    let entry_body = build_freedesktop_entry(target_exe, png_target);
    let mut desk_out = None;
    let mut menu_out = None;

    if start_menu {
        let apps_dir = home.join(".local").join("share").join("applications");
        fs::create_dir_all(&apps_dir)
            .map_err(|e| format!("Failed to create applications dir: {e}"))?;
        let icon_dir = home
            .join(".local")
            .join("share")
            .join("icons")
            .join("hicolor")
            .join("256x256")
            .join("apps");
        if fs::create_dir_all(&icon_dir).is_ok() {
            let _ = fs::write(icon_dir.join("rustpad.png"), EMBEDDED_PNG_256);
        }
        let desktop_file = apps_dir.join("rustpad.desktop");
        fs::write(&desktop_file, &entry_body)
            .map_err(|e| format!("Failed to write rustpad.desktop: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&desktop_file, fs::Permissions::from_mode(0o755));
        }
        menu_out = Some(desktop_file);
    }

    if desktop {
        let desk_dir = home.join("Desktop");
        if desk_dir.is_dir() {
            let desk_file = desk_dir.join("rustpad.desktop");
            fs::write(&desk_file, &entry_body)
                .map_err(|e| format!("Failed to write Desktop shortcut: {e}"))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&desk_file, fs::Permissions::from_mode(0o755));
            }
            desk_out = Some(desk_file);
        }
    }

    Ok((desk_out, menu_out))
}

#[cfg(not(target_os = "windows"))]
fn register_os_file_associations(_install_dir: &Path, _target_exe: &Path) -> Result<(), String> {
    let mimeapps = linux_home_dir().join(".config").join("mimeapps.list");
    if let Some(parent) = mimeapps.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut content = fs::read_to_string(&mimeapps).unwrap_or_default();
    if !content.contains("[Added Associations]") {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str("[Added Associations]\n");
    }
    for mime in [
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "application/pdf",
    ] {
        let needle = format!("{mime}=rustpad.desktop");
        if !content.contains(&needle) {
            content.push_str(&format!("{needle};\n"));
        }
    }
    fs::write(&mimeapps, content).map_err(|e| format!("Failed to update mimeapps.list: {e}"))
}

#[cfg(not(target_os = "windows"))]
fn configure_user_cli_path(_install_dir: &Path, target_exe: &Path) -> Result<(), String> {
    let bin_dir = linux_home_dir().join(".local").join("bin");
    fs::create_dir_all(&bin_dir).map_err(|e| format!("Failed to create ~/.local/bin: {e}"))?;
    let link_path = bin_dir.join("rustpad");
    if link_path.exists() || link_path.symlink_metadata().is_ok() {
        let _ = fs::remove_file(&link_path);
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target_exe, &link_path)
            .map_err(|e| format!("Failed to symlink ~/.local/bin/rustpad: {e}"))?;
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn register_os_uninstaller(
    install_dir: &Path,
    target_exe: &Path,
    _exe_bytes: u64,
) -> Result<(), String> {
    let script_path = install_dir.join("uninstall.sh");
    let script = format!(
        "#!/usr/bin/env sh\nexec \"{}\" --uninstall \"$@\"\n",
        target_exe.display()
    );
    fs::write(&script_path, script).map_err(|e| format!("Failed to write uninstall.sh: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn remove_os_integration(_install_dir: &Path, _installed_exe: &Path) {
    let home = linux_home_dir();
    let _ = fs::remove_file(home.join("Desktop").join("rustpad.desktop"));
    let _ = fs::remove_file(
        home.join(".local")
            .join("share")
            .join("applications")
            .join("rustpad.desktop"),
    );
    let _ = fs::remove_file(
        home.join(".local")
            .join("share")
            .join("icons")
            .join("hicolor")
            .join("256x256")
            .join("apps")
            .join("rustpad.png"),
    );
    let _ = fs::remove_file(home.join(".local").join("bin").join("rustpad"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ms_shllink_binary_format_has_valid_header_clsid_and_exp_block() {
        let bytes = build_ms_shllink_bytes(
            r"C:\Users\Nour\AppData\Local\Programs\RustPad\rustpad.exe",
            "--setup",
            r"C:\Users\Nour\AppData\Local\Programs\RustPad",
            "RustPad Studio",
            r"C:\Users\Nour\AppData\Local\Programs\RustPad\rustpad.exe",
        );
        // HeaderSize == 0x4C (76 bytes)
        assert_eq!(u32::from_le_bytes(bytes[0..4].try_into().unwrap()), 0x4C);
        // LinkCLSID == 00021401-0000-0000-C000-000000000046
        assert_eq!(
            &bytes[4..20],
            &[
                0x01, 0x14, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x46
            ]
        );
        // Ends with 4-byte zero TerminalBlock, preceded by 788-byte EnvironmentVariableDataBlock
        let n = bytes.len();
        assert!(n > 76 + 788 + 4);
        assert_eq!(&bytes[n - 4..n], &[0, 0, 0, 0]);
        let exp_start = n - 4 - 788;
        assert_eq!(
            u32::from_le_bytes(bytes[exp_start..exp_start + 4].try_into().unwrap()),
            0x0000_0314
        );
        assert_eq!(
            u32::from_le_bytes(bytes[exp_start + 4..exp_start + 8].try_into().unwrap()),
            0xA000_0001
        );
    }

    #[test]
    fn real_installation_and_setup_state_roundtrip_in_isolated_temp_dir() {
        let base =
            std::env::temp_dir().join(format!("rustpad_installer_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let fake_src_dir = base.join("downloads");
        let fake_install_dir = base.join("installed_app");
        let fake_cfg_dir = base.join("config");
        fs::create_dir_all(&fake_src_dir).unwrap();
        fs::create_dir_all(&fake_cfg_dir).unwrap();

        let fake_src_exe = fake_src_dir.join("rustpad-downloaded-bin");
        fs::write(&fake_src_exe, b"FAKE_RUSTPAD_BINARY_PAYLOAD_V3_0_16").unwrap();

        assert_eq!(
            load_persisted_setup_mode_from(&fake_cfg_dir.join("setup_state.conf")),
            PersistedSetupMode::NotConfigured
        );

        let opts = SetupOptions {
            accepted_license: true,
            install_dir: fake_install_dir.to_string_lossy().into_owned(),
            desktop_shortcut: false,
            start_menu_entry: false,
            file_associations: false,
            add_to_user_path: false,
            register_uninstaller: true,
            selected_theme: ThemeMode::Midnight,
            author_name: "Nour Yahyaoui".into(),
            launch_home_on_finish: true,
        };
        let prog = Arc::new(AtomicU32::new(0));
        let task = Arc::new(Mutex::new(String::new()));
        let logs = Arc::new(Mutex::new(Vec::new()));

        let summary = perform_installation_with_config_dir(
            &fake_src_exe,
            &opts,
            Some(&fake_cfg_dir),
            &prog,
            &task,
            &logs,
        )
        .unwrap();
        assert!(summary.installed_exe.exists());
        assert_eq!(
            fs::read(&summary.installed_exe).unwrap(),
            b"FAKE_RUSTPAD_BINARY_PAYLOAD_V3_0_16"
        );
        assert!(fake_install_dir.join("rustpad.ico").exists());
        assert!(fake_install_dir.join("rustpad.png").exists());
        assert_eq!(prog.load(Ordering::Relaxed), 1000);

        let parsed = load_persisted_setup_mode_from(&fake_cfg_dir.join("setup_state.conf"));
        match parsed {
            PersistedSetupMode::Installed {
                installed_exe,
                author_name,
                ..
            } => {
                assert_eq!(installed_exe, summary.installed_exe);
                assert_eq!(author_name, "Nour Yahyaoui");
            }
            other => panic!("Expected Installed mode, got {other:?}"),
        }

        let _ = fs::remove_dir_all(&base);
    }
}
