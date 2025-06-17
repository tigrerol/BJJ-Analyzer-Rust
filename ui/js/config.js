// BJJ Video Analyzer UI - Configuration

const Config = {
    // API Configuration
    api: {
        baseUrl: 'http://localhost:8080/api',
        timeout: 10000, // 10 seconds
        retryAttempts: 3,
        retryDelay: 1000, // 1 second
    },
    
    // WebSocket Configuration
    websocket: {
        url: 'ws://localhost:8080/api/status/live',
        reconnectAttempts: 5,
        reconnectDelay: 3000, // 3 seconds
        heartbeatInterval: 30000, // 30 seconds
    },
    
    // UI Configuration
    ui: {
        refreshInterval: 5000, // 5 seconds
        toastDuration: 4000, // 4 seconds
        animationDuration: 300, // 300ms
        debounceDelay: 300, // 300ms for search
    },
    
    // Pagination
    pagination: {
        defaultPageSize: 20,
        maxPageSize: 100,
    },
    
    // Video Processing
    processing: {
        supportedFormats: ['.mp4', '.avi', '.mkv', '.mov', '.wmv', '.flv'],
        maxFileSize: 10 * 1024 * 1024 * 1024, // 10GB
    },
    
    // Development
    development: {
        enableConsoleLogging: true,
        enableDebugMode: false,
        mockApiCalls: false,
    }
};

// Auto-detect environment and adjust config
if (window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1') {
    Config.development.enableConsoleLogging = true;
    
    // Check if running on different port
    const currentPort = window.location.port;
    if (currentPort && currentPort !== '8080') {
        console.log(`UI running on port ${currentPort}, API expected on port 8080`);
    }
} else {
    // Production environment
    Config.development.enableConsoleLogging = false;
    Config.development.enableDebugMode = false;
    
    // Update API URLs for production
    const protocol = window.location.protocol === 'https:' ? 'https:' : 'http:';
    const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.hostname;
    
    Config.api.baseUrl = `${protocol}//${host}:8080/api`;
    Config.websocket.url = `${wsProtocol}//${host}:8080/api/status/live`;
}

// Utility function to log configuration
Config.debug = function() {
    if (Config.development.enableConsoleLogging) {
        console.log('BJJ Analyzer UI Configuration:', Config);
    }
};

// Utility function to check if API is reachable
Config.checkApiHealth = async function() {
    try {
        const response = await fetch(`${Config.api.baseUrl}/health`, {
            method: 'GET',
            timeout: Config.api.timeout,
        });
        
        if (response.ok) {
            const data = await response.json();
            console.log('API Health Check:', data);
            return true;
        }
        return false;
    } catch (error) {
        console.error('API Health Check Failed:', error);
        return false;
    }
};

// Export for use in other modules
window.Config = Config;