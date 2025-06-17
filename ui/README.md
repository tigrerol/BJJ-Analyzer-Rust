# BJJ Analyzer UI

A web-based user interface for the BJJ Video Analyzer Rust backend, providing an intuitive way to manage video processing, series organization, and corrections.

## Features

- 📹 **Video Library Browser** - View all videos and their processing status
- 📚 **Series Management** - Organize videos into series and manage metadata
- ✏️ **Correction Interface** - Submit corrections for series mapping and product URLs
- 📊 **Real-time Monitoring** - Live processing status updates via WebSocket
- 🎯 **Product Mapping** - Visual tool for associating videos with BJJfanatics URLs

## Architecture

- **Frontend**: Vanilla HTML/CSS/JavaScript (no frameworks for simplicity)
- **Backend Communication**: REST API + WebSocket
- **Real-time Updates**: WebSocket connection to `/api/status/live`
- **CORS**: Configured for local development

## Getting Started

### Prerequisites

1. **BJJ Analyzer Rust** backend running with API enabled:
   ```bash
   cd ../BJJ-Analyzer-Rust
   cargo run --features api -- --video-dir /path/to/videos --api-port 8080
   ```

2. **Web Server** (for development):
   ```bash
   # Using Python's built-in server
   python3 -m http.server 3000
   
   # Or using Node.js
   npx serve .
   
   # Or using any local web server
   ```

### Usage

1. Start the BJJ Analyzer backend with API enabled
2. Start a local web server in this directory
3. Open `http://localhost:3000` in your browser
4. The UI will automatically connect to the backend API

## API Integration

The UI communicates with these backend endpoints:

- `GET /api/videos` - List all videos
- `GET /api/series` - List video series
- `GET /api/status` - System status
- `POST /api/corrections/series` - Submit series corrections
- `POST /api/corrections/products` - Submit product URL corrections
- `WebSocket /api/status/live` - Real-time updates

## Development

### File Structure

```
bjj-analyzer-ui/
├── index.html          # Main application page
├── css/
│   ├── styles.css      # Main stylesheet
│   └── components.css  # Component-specific styles
├── js/
│   ├── app.js          # Main application logic
│   ├── api.js          # API communication layer
│   ├── websocket.js    # WebSocket handling
│   └── components/     # UI component modules
│       ├── video-list.js
│       ├── series-manager.js
│       └── corrections.js
└── assets/
    └── icons/          # UI icons and images
```

### Key Components

- **VideoLibrary**: Browse and filter videos by status, instructor, series
- **SeriesManager**: Group videos into series, edit metadata
- **CorrectionsPanel**: Submit and manage user corrections
- **StatusMonitor**: Real-time processing status and progress
- **ProductMapper**: Associate videos with BJJfanatics product URLs

## Configuration

The UI automatically detects the backend API at:
- Default: `http://localhost:8080/api`
- WebSocket: `ws://localhost:8080/api/status/live`

You can modify the API base URL in `js/config.js` if needed.

## Contributing

1. Keep the UI simple and responsive
2. Use vanilla JavaScript (no frameworks required)
3. Follow the existing component pattern
4. Test with the Rust backend API
5. Ensure real-time updates work correctly

## License

MIT License - Same as the BJJ Analyzer Rust backend