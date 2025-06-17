// BJJ Video Analyzer UI - WebSocket Connection Manager

class WebSocketManager {
    constructor() {
        this.ws = null;
        this.url = Config.websocket.url;
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = Config.websocket.reconnectAttempts;
        this.reconnectDelay = Config.websocket.reconnectDelay;
        this.heartbeatInterval = Config.websocket.heartbeatInterval;
        this.heartbeatTimer = null;
        this.isConnecting = false;
        this.listeners = new Map();
        this.connectionStatus = 'disconnected';
        
        this.updateConnectionStatus('disconnected');
    }

    // Connect to WebSocket
    connect() {
        if (this.isConnecting || (this.ws && this.ws.readyState === WebSocket.OPEN)) {
            return;
        }

        this.isConnecting = true;
        this.updateConnectionStatus('connecting');
        this.log('Attempting WebSocket connection...');

        try {
            this.ws = new WebSocket(this.url);
            this.setupEventHandlers();
        } catch (error) {
            this.log('WebSocket connection failed:', error);
            this.handleConnectionError();
        }
    }

    // Disconnect from WebSocket
    disconnect() {
        this.log('Disconnecting WebSocket...');
        
        if (this.heartbeatTimer) {
            clearInterval(this.heartbeatTimer);
            this.heartbeatTimer = null;
        }
        
        if (this.ws) {
            this.ws.close(1000, 'Client disconnect');
            this.ws = null;
        }
        
        this.isConnecting = false;
        this.reconnectAttempts = 0;
        this.updateConnectionStatus('disconnected');
    }

    // Setup WebSocket event handlers
    setupEventHandlers() {
        this.ws.onopen = () => {
            this.log('WebSocket connected successfully');
            this.isConnecting = false;
            this.reconnectAttempts = 0;
            this.updateConnectionStatus('connected');
            this.startHeartbeat();
            this.emit('connected');
        };

        this.ws.onmessage = (event) => {
            try {
                const data = JSON.parse(event.data);
                this.log('WebSocket message received:', data);
                this.handleMessage(data);
            } catch (error) {
                this.log('Error parsing WebSocket message:', error);
            }
        };

        this.ws.onclose = (event) => {
            this.log('WebSocket connection closed:', event);
            this.isConnecting = false;
            this.updateConnectionStatus('disconnected');
            this.stopHeartbeat();
            this.emit('disconnected', event);
            
            // Attempt reconnection if not a normal closure
            if (event.code !== 1000) {
                this.scheduleReconnect();
            }
        };

        this.ws.onerror = (error) => {
            this.log('WebSocket error:', error);
            this.emit('error', error);
            this.handleConnectionError();
        };
    }

    // Handle incoming WebSocket messages
    handleMessage(data) {
        switch (data.type) {
            case 'ProcessingUpdate':
                this.emit('processingUpdate', data);
                this.addLogEntry(`Processing update: ${data.video_id} - ${data.message}`);
                break;
                
            case 'SystemStatus':
                this.emit('systemStatus', data.status);
                break;
                
            case 'Error':
                this.emit('error', data.error);
                this.addLogEntry(`Error: ${data.error.message}`, 'error');
                break;
                
            default:
                this.log('Unknown message type:', data.type);
                this.emit('message', data);
        }
    }

    // Schedule reconnection attempt
    scheduleReconnect() {
        if (this.reconnectAttempts >= this.maxReconnectAttempts) {
            this.log('Max reconnection attempts reached');
            this.updateConnectionStatus('failed');
            return;
        }

        this.reconnectAttempts++;
        const delay = this.reconnectDelay * Math.pow(1.5, this.reconnectAttempts - 1);
        
        this.log(`Scheduling reconnection attempt ${this.reconnectAttempts} in ${delay}ms`);
        this.updateConnectionStatus('reconnecting');
        
        setTimeout(() => {
            if (this.connectionStatus !== 'connected') {
                this.connect();
            }
        }, delay);
    }

    // Handle connection errors
    handleConnectionError() {
        this.isConnecting = false;
        this.updateConnectionStatus('disconnected');
        
        if (window.toast) {
            window.toast.warning('WebSocket connection lost. Attempting to reconnect...');
        }
    }

    // Start heartbeat to keep connection alive
    startHeartbeat() {
        this.heartbeatTimer = setInterval(() => {
            if (this.ws && this.ws.readyState === WebSocket.OPEN) {
                this.send({ type: 'ping' });
            }
        }, this.heartbeatInterval);
    }

    // Stop heartbeat
    stopHeartbeat() {
        if (this.heartbeatTimer) {
            clearInterval(this.heartbeatTimer);
            this.heartbeatTimer = null;
        }
    }

    // Send message through WebSocket
    send(data) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            try {
                this.ws.send(JSON.stringify(data));
                this.log('WebSocket message sent:', data);
            } catch (error) {
                this.log('Error sending WebSocket message:', error);
            }
        } else {
            this.log('Cannot send message: WebSocket not connected');
        }
    }

    // Event listener management
    on(event, callback) {
        if (!this.listeners.has(event)) {
            this.listeners.set(event, []);
        }
        this.listeners.get(event).push(callback);
    }

    off(event, callback) {
        if (this.listeners.has(event)) {
            const callbacks = this.listeners.get(event);
            const index = callbacks.indexOf(callback);
            if (index > -1) {
                callbacks.splice(index, 1);
            }
        }
    }

    emit(event, data = null) {
        if (this.listeners.has(event)) {
            this.listeners.get(event).forEach(callback => {
                try {
                    callback(data);
                } catch (error) {
                    this.log(`Error in event listener for ${event}:`, error);
                }
            });
        }
    }

    // Update connection status in UI
    updateConnectionStatus(status) {
        this.connectionStatus = status;
        
        const indicator = document.getElementById('connection-indicator');
        const text = document.getElementById('connection-text');
        
        if (indicator && text) {
            indicator.className = `status-indicator ${status}`;
            
            const statusTexts = {
                connected: 'Connected',
                connecting: 'Connecting...',
                reconnecting: 'Reconnecting...',
                disconnected: 'Disconnected',
                failed: 'Connection Failed'
            };
            
            text.textContent = statusTexts[status] || status;
        }
    }

    // Add log entry to real-time log
    addLogEntry(message, level = 'info') {
        const logContainer = document.getElementById('realtime-log');
        if (!logContainer) return;

        const entry = document.createElement('div');
        entry.className = 'log-entry';
        
        const timestamp = new Date().toLocaleTimeString();
        entry.innerHTML = `
            <span class="log-time">[${timestamp}]</span>
            <span class="log-level ${level}">${level.toUpperCase()}</span>
            <span class="log-message">${message}</span>
        `;

        logContainer.appendChild(entry);
        
        // Keep only last 100 entries
        const entries = logContainer.querySelectorAll('.log-entry');
        if (entries.length > 100) {
            entries[0].remove();
        }
        
        // Auto-scroll to bottom
        logContainer.scrollTop = logContainer.scrollHeight;
    }

    // Get current connection status
    getStatus() {
        return {
            status: this.connectionStatus,
            reconnectAttempts: this.reconnectAttempts,
            maxReconnectAttempts: this.maxReconnectAttempts,
            isConnected: this.ws && this.ws.readyState === WebSocket.OPEN
        };
    }

    // Utility logging
    log(message, data = null) {
        if (Config.development.enableConsoleLogging) {
            if (data) {
                console.log(`[WebSocket] ${message}`, data);
            } else {
                console.log(`[WebSocket] ${message}`);
            }
        }
    }
}

// Initialize global WebSocket manager
window.websocket = new WebSocketManager();

// Auto-connect when page loads
document.addEventListener('DOMContentLoaded', () => {
    // Wait a bit for other components to initialize
    setTimeout(() => {
        window.websocket.connect();
    }, 1000);
});

// Cleanup on page unload
window.addEventListener('beforeunload', () => {
    if (window.websocket) {
        window.websocket.disconnect();
    }
});