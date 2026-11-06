# Evlog Lens

<img src="assets/evlog-lens.svg" alt="Evlog Lens logo" width="80" height="80">

A native workspace for structured logs. Follow the signal. Find the story.

A native, local-first port of [Dyrean/evlog-explorer](https://github.com/Dyrean/evlog-explorer), built with Rust and [Rune](https://github.com/anomalyco/rune). The UI uses Rune elements and its GPU renderer throughout.

Rune is pinned to commit `781e2b6d87cd5f32d6f4365956fad4baaf24ee50`; `Cargo.lock` pins the dependency graph.

## Build from source

The Cargo commands below work in **Windows PowerShell**, **Linux shells**, and **macOS shells**. Platform-specific commands are labeled. The build and headless checks have been verified on Linux with Rust **1.97.0**; Windows and macOS builds have not yet been verified. Use a current stable Rust toolchain; an older minimum Rust version has not been established. Cargo downloads the pinned Rune Git revision and the crates in `Cargo.lock` on the first build, so that build requires internet access. You do not need Node.js, a JavaScript package manager, or a separate Rune checkout.

### 1. Install build prerequisites

#### Windows · PowerShell

1. Install [Git for Windows](https://git-scm.com/downloads/win) if you will clone the repository.
2. Install Visual Studio or its C++ Build Tools with the **Desktop development with C++** workload, including the MSVC compiler and a Windows SDK. See the [Rust MSVC prerequisites](https://rust-lang.github.io/rustup/installation/windows-msvc.html).
3. Install Rust using the Windows installer from [rustup.rs](https://rustup.rs/), accepting the default MSVC toolchain. If WinGet is available, you can instead run:

```powershell
winget install --exact --id Rustlang.Rustup
```

4. Open a new PowerShell window after installation. You can use ordinary PowerShell once the tools are installed; WSL and the Linux packages below are not required for a native Windows build.
5. Install Python 3 if you want to run the headless verification script, and keep your GPU's Windows graphics drivers up to date.

#### macOS · Terminal

Install Git and the Xcode Command Line Tools:

```sh
xcode-select --install
```

Install Rust using the Unix instructions in step 2. Python 3 is optional unless you run the headless verification script. Rendering uses the backend available through Rune on macOS; Linux Vulkan packages are not required.

#### Linux · Terminal

On Linux, install a native C/C++ toolchain, Git, `pkg-config`, and the XKB/Wayland development libraries required by [the pinned Rune version](https://github.com/anomalyco/rune/blob/781e2b6d87cd5f32d6f4365956fad4baaf24ee50/crates/rune/README.md). Python 3 is used only by the headless verification script.

**Debian / Ubuntu:**

```sh
sudo apt-get update
sudo apt-get install -y build-essential git curl pkg-config \
  libxkbcommon-dev libwayland-dev libxkbcommon-x11-0 libvulkan1 python3
```

**Fedora:**

```sh
sudo dnf install gcc gcc-c++ make git curl pkgconf-pkg-config \
  libxkbcommon-devel libxkbcommon-x11 wayland-devel vulkan-loader python3
```

**Arch Linux:**

```sh
sudo pacman -S --needed base-devel git curl pkgconf \
  libxkbcommon libxkbcommon-x11 wayland vulkan-icd-loader python
```

Windowed runs also need an X11 or Wayland desktop session and a working Vulkan driver appropriate for your GPU. The Vulkan loader packages above do not include a hardware driver. For a machine without a physical GPU, see the software-rendering instructions below.

Native **Import files** and **Export NDJSON** dialogs use the desktop's XDG FileChooser portal. Install `xdg-desktop-portal` and a backend that provides a file chooser for your desktop, such as `xdg-desktop-portal-gtk` or `xdg-desktop-portal-kde`, if your desktop does not already provide them. File drops and `--load` are also available.

### 2. Install or update Rust

Install Rust using [rustup](https://rust-lang.org/tools/install/). On Linux or macOS:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

On Windows, use the installer or WinGet command in step 1 instead of the Unix installer command above. After installation, open a new terminal so Cargo is on `PATH`. The following commands are the same in PowerShell and Unix shells:

```text
rustup update stable
rustup default stable
rustup component add rustfmt clippy
rustc --version
cargo --version
```

### 3. Build the development executable

Clone or download **this repository**. From the directory containing your checkout, enter `evlog-lens` (the directory with `Cargo.toml`) and build as shown below. The original [evlog-explorer](https://github.com/Dyrean/evlog-explorer) is the source project for the port; it is not a build dependency and does not need to be checked out alongside this repository.

```text
cd evlog-lens
cargo build --locked
```

The executable is `target/debug/evlog-lens` on Linux/macOS, or `target/debug/evlog-lens.exe` on Windows. `--locked` keeps dependency resolution consistent with the checked-in lockfile. The first build compiles the native renderer and its dependencies; subsequent builds reuse Cargo's cache.

### 4. Build and run an optimized executable

Use Cargo to build and launch the release version on any supported platform:

```text
cargo build --release --locked
cargo run --release --locked -- --sample
```

To launch the executable directly after building, use your platform's path syntax.

**Windows PowerShell:**

```powershell
.\target\release\evlog-lens.exe --sample
```

**Linux / macOS:**

```sh
./target/release/evlog-lens --sample
```

These commands produce a native executable, not an installer or a macOS application bundle. Build on the operating system where you intend to run it; these instructions do not configure cross-compilation. A Linux executable still needs compatible system libraries and GPU drivers on its destination machine.

For an optional local Cargo installation:

```text
cargo install --path . --locked
evlog-lens --sample
```

Cargo installs into its binary directory: normally `$HOME/.cargo/bin` on Linux/macOS or `$env:USERPROFILE\.cargo\bin` on Windows. A custom `CARGO_HOME` changes that location. Keep the binary directory on `PATH`.

## Run

Launch an empty workspace, explore the sample incident, or open your own files:

```text
cargo run --locked
cargo run --locked -- --sample
cargo run --locked -- --load examples/sample-evlog.ndjson
cargo run --locked -- --load "api.ndjson" --load "worker.jsonl"
```

The `--` separates Cargo options from application arguments. Relative file paths are resolved from the current directory. Forward slashes in the relative paths shown here work in both PowerShell and Unix shells.

When running the built executable directly, omit Cargo's separator:

**Windows PowerShell:**

```powershell
.\target\debug\evlog-lens.exe --load "C:\Logs\events.ndjson"
```

**Linux / macOS:**

```sh
./target/debug/evlog-lens --load "/path/to/events.ndjson"
```

Use **Import files**, or drop multiple files anywhere in the window. `--load` can repeat. Quote file paths containing spaces. The included sample has 50 events, 12 requests, and four sessions.

### Headless rendering

Headless snapshots use the real renderer without opening a window. They do not need a running desktop display session, but still need a compatible graphics backend and the executable's system libraries.

Create the output directory using your shell:

**Windows PowerShell:**

```powershell
New-Item -ItemType Directory -Force artifacts | Out-Null
```

**Linux / macOS:**

```sh
mkdir -p artifacts
```

Then run the same single-line command on any platform:

```text
cargo run --locked -- --scene incident --snap artifacts/incident.png --tree artifacts/incident.json --scale 1
```

Shared commands use one line so they do not depend on Bash's `\` continuation or PowerShell's backtick continuation.

#### Linux software Vulkan

On Linux, a Vulkan-capable GPU driver or software Vulkan driver is required. To use Mesa's software Vulkan driver on Linux, install the appropriate package:

```sh
# Debian / Ubuntu
sudo apt-get install mesa-vulkan-drivers

# Fedora
sudo dnf install mesa-vulkan-drivers

# Arch Linux
sudo pacman -S --needed vulkan-swrast
```

Then rerun the snapshot command. If Vulkan chooses the wrong driver, inspect `/usr/share/vulkan/icd.d/` and point `VK_DRIVER_FILES` at the installed Lavapipe (`lvp`) JSON manifest before launching. The filename varies by distribution and architecture; use the actual installed path.

### Build and startup troubleshooting

- **`cargo` or `rustc` is missing:** reopen your terminal after installing rustup and confirm Cargo's binary directory is on `PATH`.
- **The compiler reports an unsupported edition or dependency Rust-version requirement:** update the stable toolchain with `rustup update stable`, then retry.
- **Windows reports `link.exe` is missing or cannot find a Windows SDK library:** install or repair the Visual Studio C++ workload and Windows SDK, confirm your Rust toolchain uses MSVC, and reopen PowerShell.
- **PowerShell cannot find a built executable:** use the leading `.\` and the `.exe` extension shown above, or launch through `cargo run`.
- **Linux linking fails for `xkbcommon` or `wayland-client`:** install the development packages listed above, not just their runtime libraries.
- **Linux reports `libxkbcommon-x11.so` cannot be loaded:** install `libxkbcommon-x11-0` on Debian/Ubuntu, or `libxkbcommon-x11` on Fedora/Arch, then relaunch.
- **No graphics adapter is available:** update your platform's graphics drivers. On Linux, check the Vulkan driver installation or install the software driver for headless checks.
- **A native window cannot open:** launch from an active desktop session, or use `--snap` / `--tree` for a headless run.
- **The Linux file chooser reports a portal error:** check that your desktop's FileChooser portal backend is installed and running in the same user session. You can also open files with `--load` or drag and drop.

## Explore

- Import JSON objects, arrays, `{ "events": [...] }`, JSONL, or NDJSON. Valid entries survive malformed lines; issues retain file and line information. Background imports commit as one batch. Cancel signals the worker during chunked reading and JSON parsing, discards the pending batch, and keeps loaded data.
- Search all JSON fields with unquoted fuzzy text, `"literal phrases"`, exact predicates such as `status:503`, `request:req-011`, `session:sess-local-003`, `user.id:user-3`, or numeric comparisons such as `duration>=1000`. Predicates combine with AND. Duration comparisons use milliseconds. Fuzzy text uses subsequence matching.
- Use quick views for errors, warnings, slow requests, and 5xx responses. Click a source to filter it. **Filters & views** adds level, method, status, ISO 8601 time bounds, and named views. The time-range button cycles all time, 15 minutes, one hour, and 24 hours, ending at the newest imported event.
- Select an event to inspect normalized fields, its preserved JSON, and evlog's `why` / `fix`. Expand nested JSON sections or copy the full event. Click request, trace, session, or user IDs to follow exact matches across loaded files.
- **Timeline** shows request/trace events in chronological order, independent of table filters. **Session** groups the session's events by request ID; events without a request ID remain separate. Select any timeline event to inspect it. Escape closes the inspector.
- Switch UTC/local timestamp display, change chronological sort order, and export matching original objects as NDJSON. Tables paginate at 100 events to bound the rendered tree.

Nothing is uploaded. Logs stay in memory unless you export or click **Save session**. Views are saved when you name and save them. Session and view files use `XDG_DATA_HOME/evlog-lens`, or `HOME/.local/share/evlog-lens` when `XDG_DATA_HOME` is unset. This resolver currently uses those environment variables on every platform. On native Windows, set a per-user data location before launching if `HOME` is unset:

```powershell
$env:XDG_DATA_HOME = $env:LOCALAPPDATA
cargo run --locked
```

That stores data under `%LOCALAPPDATA%\evlog-lens` for that PowerShell session. Without either environment variable, the current implementation falls back to `.local/share/evlog-lens` relative to the working directory. **Restore saved session** appears on the start screen; **Delete saved session** deletes the saved copy. **Clear logs** clears memory and cancels pending imports. Saved filter views exclude source selection and correlation. Existing installations continue using the previous `evlog-explorer-rust` data folder when it exists and the new folder does not, so the rename keeps saved sessions and views accessible.

## Verify headlessly

The files in `scripts/` are optional development checks; building and running the application does not require them or Python.

- `scripts/check_headless.py` builds the app, renders its screens, and checks the resulting element trees and window layout.
- `scripts/investigate.json` supplies the search and timeline interactions used by that check through Rune's headless input runner.

The Rust checks are the same in PowerShell and Unix shells:

```text
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

Run the renderer checks with your Python 3 launcher.

**Windows PowerShell:**

```powershell
py -3 scripts/check_headless.py
```

**Linux / macOS:**

```sh
python3 scripts/check_headless.py
```

If Python 3 is available as `python` instead, use `python scripts/check_headless.py`. The script resolves the native executable through Cargo metadata, including Windows's `.exe` suffix.

Tests exercise normalization and fallback IDs, malformed imports, interruption of large JSON/NDJSON parsing, worker shutdown, same-length dataset replacement, source rebasing, paging, correlation, persistence and corruption recovery, atomic concurrent saves, export, sidebar scrolling with 30 sources, and the production Rune polling/key/dialog hooks. Import diagnostics remain fully accessible beyond the first 100 issues. Undated events appear after dated events in either table sort direction and in chronological timelines. The headless check renders eight PNGs with matching element trees, asserts the scripted search/session workflow, checks the minimum window layout, and rejects Rune warnings. Outputs are in `artifacts/`.

```text
cargo run --locked -- --scene incident --snap artifacts/incident.png --tree artifacts/incident.json --scale 1
cargo run --locked -- --scene incident --script scripts/investigate.json --at 4.5 --snap artifacts/investigation.png --tree artifacts/investigation.json --scale 1
```

Scenes: `sample`, `incident`, `errors`, `json`, `session`. Without a scene the app starts empty.

## Scope

Desktop layout, minimum 1180×720. Files and parsed events must fit in memory. Normalized timestamps, severity, duration, correlation IDs, and searchable JSON text are prepared once per event. Filter results and summaries are cached until the dataset or query changes; new queries still scan the imported events. This is not a disk-indexed log database. Native OS dialog presentation has not been exercised interactively; its callbacks are tested headlessly. The port uses native files for persistence, ISO timestamps with explicit timezones for custom bounds, and pagination in place of the web app's IndexedDB, local datetime inputs, and virtual table.

Source sample data and application behavior come from [Dyrean/evlog-explorer](https://github.com/Dyrean/evlog-explorer); see `LICENSE`. Rune is MIT licensed.

## Identity

The lavender lens and mint event lines are native Rune vector geometry in the app. The matching reusable logo is [`assets/evlog-lens.svg`](assets/evlog-lens.svg), with a PNG export alongside it. The interface uses Inter for controls and IBM Plex Mono for log values, with a midnight-blue palette and separate error/warning colors.

To regenerate logo assets from the shared native geometry:

```text
cargo run --locked -- --export-logo assets/evlog-lens.svg
cargo run --locked -- --snap assets/evlog-lens.png --crop welcome-logo --scale 4
```

## Code layout

- `log/` owns immutable events, normalization, accepted file formats, and search syntax.
- `query.rs` defines plain filters, saved queries, and correlation/inspector vocabulary.
- `workspace.rs` owns collections, mutations, cached results, summaries, and timelines. Callers receive read-only data; every mutation invalidates caches and navigation generation.
- `importer.rs` owns cancellable file reading and job completion/failure. Startup and background imports use the same batch format and source-rebasing rules.
- `storage.rs` owns saved data and legacy discovery. `State::default()` performs no filesystem reads; production startup injects the store.
- `app.rs` adapts Rune edit buffers and user actions to those boundaries. `shell.rs` supplies startup options, scenes, and the shared production runtime specification; `main.rs` launches it.
- `ui/` contains feature views and shared presentation. It consumes domain timelines and results rather than rebuilding them.
