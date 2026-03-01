# Image Viewer

A comprehensive Rust-based image viewer with dual-pane interface supporting local and network drives.

## Features

### Core Interface
- **Left Pane**: Hierarchical folder tree navigation with expansion/collapse
- **Right Pane**: Thumbnail grid display with virtualization for large image collections
- **Image Viewer**: Modal overlay for full-size image viewing with keyboard navigation

### Keyboard Controls
- **Space**: Open/close image viewer (when right pane has focus)
- **↑/↓**: Navigate images in viewer, or move selection in grid
- **←/→**: Navigate images in viewer, or move selection in grid
- **Esc**: Close image viewer
- **Enter**: Alternative to Space for opening viewer

### Network Drive Support
- UNC paths: `\\server\share\path`
- Mapped drives: `Z:\path`
- Manual path entry for network locations
- Graceful handling of access denied and network timeouts

### Performance Features
- Non-blocking directory enumeration
- Background thumbnail generation with caching
- Virtualized grid for large image collections (5000+ files)
- Memory-efficient thumbnail cache with LRU eviction

### Supported Formats
- Primary: `.jpg`, `.jpeg`, `.png`, `.bmp`, `.gif`, `.webp`
- Additional: `.tiff` (platform dependent)

## Building and Running

### Prerequisites
- Rust 1.70+ with 2021 edition support
- Platform-specific GUI dependencies (automatically handled by eframe)

### Commands
```bash
# Check compilation
cargo check

# Run in development mode
cargo run

# Build optimized release
cargo build --release

# Run with specific test directory
cargo run --release -- --test-dir /path/to/images
```

### Development Dependencies
```toml
[dependencies]
egui = "0.29"          # Immediate mode GUI framework
eframe = "0.29"        # Application framework
image = "0.25"         # Image processing
walkdir = "2.5"        # Directory traversal
anyhow = "1.0"         # Error handling
rfd = "0.15"           # File dialogs
```

## Architecture

### Module Structure
```
src/
├── main.rs              # Application entry point
├── ui/
│   ├── app.rs           # Main application state and coordination
│   ├── tree_view.rs     # Folder tree component
│   ├── thumbnail_grid.rs # Image grid with thumbnails
│   └── image_viewer.rs  # Modal image viewer
├── core/
│   ├── file_scanner.rs  # Directory enumeration and file detection
│   ├── thumbnail_cache.rs # Memory cache for thumbnails
│   └── image_loader.rs  # Image loading and thumbnail generation
└── utils/
    ├── network_path.rs  # UNC/network path utilities
    └── keyboard.rs      # Keyboard input handling
```

### Key Components

#### TreeView
- Hierarchical directory display
- Lazy loading of subdirectories
- Network path support with timeout handling
- Image count display for each folder

#### ThumbnailGrid
- Virtualized rendering for performance
- Asynchronous thumbnail loading
- Keyboard navigation support
- Configurable thumbnail sizes

#### ImageViewer
- Modal overlay presentation
- Zoom and pan capabilities
- Keyboard navigation between images
- Image metadata display

#### ThumbnailCache
- Memory-efficient caching with configurable limits
- Cache keys based on file path, modified time, and size
- Background cache population

## Usage

1. **Launch** the application
2. **Set Root Path** using the browse button or manual entry (supports UNC paths)
3. **Navigate** folders using the left tree view
4. **Browse Images** in the right thumbnail grid
5. **View Images** by pressing Space or double-clicking
6. **Navigate** in viewer using arrow keys
7. **Close Viewer** with Esc or Space

## Network Drive Usage

### UNC Paths
```
\\server\share\photos
\\nas\media\images
```

### Mapped Drives
```
Z:\photos
Y:\backup\images
```

### Manual Path Entry
Enter network paths directly in the root path field if the folder dialog doesn't handle them properly.

## Error Handling

- **Corrupted Images**: Shows "Broken" placeholder, logs error, continues processing
- **Access Denied**: Displays warning, skips inaccessible files/folders
- **Network Issues**: Timeout handling with retry mechanisms
- **Large Collections**: Progressive loading prevents UI freezing

## Configuration

The application remembers:
- Last root folder path
- Left pane width ratio  
- Thumbnail size preference
- Sort order (name/date/size)
- Window position and size

## Performance Considerations

- **Large Directories**: Background enumeration prevents UI blocking
- **Memory Usage**: Configurable thumbnail cache limits (default: 256MB)
- **Network Drives**: Timeout-based resilience for slow connections
- **Thumbnail Generation**: Worker thread pool for parallel processing

## Development Notes

This implementation follows the specifications outlined in `.github/copilot-instructions.md` and provides a solid foundation for a production-ready image viewer with enterprise network support.

## License

MIT License - see LICENSE file for details.