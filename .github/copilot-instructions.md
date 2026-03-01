# Copilot Instructions for image_viewer

## Project Overview
A comprehensive Rust-based image viewer with dual-pane interface:
- **Left pane**: Folder tree navigation (including network drives)
- **Right pane**: Thumbnail grid of images
- **Viewer**: Modal image display with keyboard navigation (Space/↑↓/Esc)

## Core Architecture

### UI Structure (Two-Pane Layout)
```
┌─────────────┬─────────────────────────┐
│ Folder Tree │     Thumbnail Grid      │
│   (Left)    │        (Right)          │
│             │                         │
│   ├─ Dir1   │  ┌────┐ ┌────┐ ┌────┐   │
│   ├─ Dir2   │  │img1│ │img2│ │img3│   │
│   └─ Dir3   │  └────┘ └────┘ └────┘   │
└─────────────┴─────────────────────────┘
```

### Key Components to Implement
- `TreeView`: Hierarchical folder navigation with network path support (`\\server\share`, `Z:\`)
- `ThumbnailGrid`: Virtualized grid for large image collections (5000+ files)
- `ImageViewer`: Modal overlay with zoom/pan controls
- `ThumbnailCache`: Memory + disk cache with key format: `path + mtime + size`
- `AsyncLoader`: Background thread pool for directory enumeration and thumbnail generation

## Critical Keyboard Specifications

### Right Pane (Grid) Focus
- `↑/↓`: Navigate selection (row-wise)
- `Space`: Open image in viewer modal
- `Enter`: Alternative to Space
- Focus control is essential - Space should only work when right pane has focus

### Image Viewer Mode
- `↑/↓` or `←/→`: Previous/next image
- `Space` or `Esc`: Close viewer
- `+/-`: Zoom in/out (optional)
- `0`: Fit to window (optional)

## Network Drive Requirements

### Path Support
- UNC paths: `\\server\share\path`
- Mapped drives: `Z:\path` 
- Manual path entry UI for UNC (folder dialogs may not handle UNC well)

### Resilience Patterns
- All directory enumeration must be async with timeout (10s default)
- Graceful handling of access denied (show in tree as "Access Denied")
- Retry mechanisms for network disconnections
- Cancel ongoing operations when user changes selection

## Performance Requirements

### Non-Blocking Operations
- Directory traversal: Background thread with progress indication
- Thumbnail generation: Worker thread pool with queue
- UI updates: Use async channels to update UI from background threads

### Memory Management
- Thumbnail memory cache with configurable limit (256MB default)
- Virtualized grid - only render visible thumbnails
- LRU eviction for thumbnail cache

### File Format Support
- Primary: `.jpg`, `.jpeg`, `.png`, `.bmp`, `.gif`, `.webp`
- Optional: `.tiff`, `.heic` (OS-dependent)

## Error Handling Patterns

### File-Level Errors
- Corrupted images: Show "Broken" placeholder in grid, log error, continue
- Access denied: Skip file, show warning icon
- Network timeouts: Retry with exponential backoff

### UI Error States
- Network disconnection: Show "Reconnect" button in right pane
- Empty folders: Show "No images found" message
- Loading states: Progress spinners for long operations

## Development Workflow

### Recommended Crate Dependencies
```toml
# GUI Framework
egui = "0.XX"          # Immediate mode GUI
eframe = "0.XX"        # Application framework

# Image Processing  
image = "0.XX"         # Core image loading/manipulation
fast_image_resize = "0.XX"  # Efficient thumbnail generation

# Async Runtime
tokio = { version = "1.0", features = ["full"] }

# File System
notify = "6.0"         # File system watching (optional)
walkdir = "2.0"        # Directory traversal

# Caching
lru = "0.XX"           # LRU cache implementation
```

### Build Commands
```bash
# Development
cargo run --features dev
cargo test
cargo check

# Performance testing with large directories
cargo run --release -- --test-dir /path/to/large/image/dir
```

### Code Organization
```
src/
├── main.rs              # Application entry point
├── ui/
│   ├── mod.rs          # UI module
│   ├── tree_view.rs    # Folder tree component
│   ├── thumbnail_grid.rs # Image grid component
│   └── image_viewer.rs # Modal image viewer
├── core/
│   ├── mod.rs          # Core business logic
│   ├── file_scanner.rs # Directory enumeration
│   ├── thumbnail_cache.rs # Caching system
│   └── image_loader.rs # Image loading/processing
└── utils/
    ├── network_path.rs # UNC/network path handling
    └── keyboard.rs     # Keyboard event handling
```

## Testing Acceptance Criteria
- Local folder → tree display → thumbnail grid appears
- UNC path (`\\server\share`) works identically to local paths
- Space key opens viewer, Esc closes it consistently
- Arrow keys navigate images in viewer mode without lag
- 5000+ images load without UI freeze (background processing)
- Network interruption recovery (reconnect functionality)

## Settings to Persist
- Last root folder path
- Left pane width ratio
- Thumbnail size (grid scale)
- Sort order (name/date/size)
- Recursive subdirectory toggle
- Supported file extensions filter