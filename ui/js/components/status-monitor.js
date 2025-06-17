// BJJ Video Analyzer UI - Status Monitor Component

class StatusMonitorComponent {
    constructor() {
        this.systemStatus = null;
        this.refreshInterval = null;
        this.initializeElements();
        this.bindEvents();
        this.setupWebSocketListeners();
        this.startAutoRefresh();
    }

    initializeElements() {
        this.container = document.getElementById('status-container');
        this.refreshButton = document.getElementById('refresh-status');
        
        // Status metric elements
        this.totalVideosEl = document.getElementById('total-videos');
        this.processingVideosEl = document.getElementById('processing-videos');
        this.completedVideosEl = document.getElementById('completed-videos');
        this.failedVideosEl = document.getElementById('failed-videos');
        this.activeWorkersEl = document.getElementById('active-workers');
        this.queueSizeEl = document.getElementById('queue-size');
        this.uptimeEl = document.getElementById('uptime');
        this.realtimeLogEl = document.getElementById('realtime-log');
    }

    bindEvents() {
        this.refreshButton.addEventListener('click', () => {
            this.refresh();
        });
    }

    setupWebSocketListeners() {
        if (window.websocket) {
            window.websocket.on('systemStatus', (status) => {
                this.updateSystemStatus(status);
            });

            window.websocket.on('processingUpdate', (data) => {
                this.addLogEntry(`${data.video_id}: ${data.message}`, 'info');
            });

            window.websocket.on('error', (error) => {
                this.addLogEntry(`Error: ${error.message}`, 'error');
            });
        }
    }

    async refresh() {
        try {
            this.systemStatus = await window.api.getStatus();
            this.updateSystemStatus(this.systemStatus);
            this.addLogEntry('System status refreshed', 'info');
        } catch (error) {
            window.ErrorHandler.handleApiError(error, 'Refresh Status');
            this.addLogEntry(`Failed to refresh status: ${error.message}`, 'error');
        }
    }

    updateSystemStatus(status) {
        this.systemStatus = status;
        
        // Update metrics
        this.updateMetric(this.totalVideosEl, status.total_videos);
        this.updateMetric(this.processingVideosEl, status.processing_videos);
        this.updateMetric(this.completedVideosEl, status.completed_videos);
        this.updateMetric(this.failedVideosEl, status.failed_videos);
        this.updateMetric(this.activeWorkersEl, status.active_workers);
        this.updateMetric(this.queueSizeEl, status.queue_size);
        this.updateMetric(this.uptimeEl, this.formatUptime(status.uptime));
        
        // Update progress indicators if any
        this.updateProgressIndicators(status);
    }

    updateMetric(element, value) {
        if (element) {
            element.textContent = value !== undefined ? value : '-';
            
            // Add visual feedback for changes
            element.classList.add('updated');
            setTimeout(() => {
                element.classList.remove('updated');
            }, 500);
        }
    }

    updateProgressIndicators(status) {
        // Calculate overall completion percentage
        if (status.total_videos > 0) {
            const completionRate = (status.completed_videos / status.total_videos) * 100;
            
            // Update any progress bars or indicators
            const progressElements = document.querySelectorAll('.overall-progress');
            progressElements.forEach(el => {
                el.style.width = `${completionRate}%`;
            });
        }
    }

    formatUptime(seconds) {
        if (!seconds) return '0s';
        
        const days = Math.floor(seconds / 86400);
        const hours = Math.floor((seconds % 86400) / 3600);
        const minutes = Math.floor((seconds % 3600) / 60);
        const secs = seconds % 60;
        
        if (days > 0) {
            return `${days}d ${hours}h ${minutes}m`;
        } else if (hours > 0) {
            return `${hours}h ${minutes}m`;
        } else if (minutes > 0) {
            return `${minutes}m ${secs}s`;
        } else {
            return `${secs}s`;
        }
    }

    addLogEntry(message, level = 'info') {
        if (!this.realtimeLogEl) return;

        const entry = document.createElement('div');
        entry.className = 'log-entry';
        
        const timestamp = new Date().toLocaleTimeString();
        entry.innerHTML = `
            <span class="log-time">[${timestamp}]</span>
            <span class="log-level ${level}">${level.toUpperCase()}</span>
            <span class="log-message">${message}</span>
        `;

        this.realtimeLogEl.appendChild(entry);
        
        // Keep only last 100 entries
        const entries = this.realtimeLogEl.querySelectorAll('.log-entry');
        if (entries.length > 100) {
            entries[0].remove();
        }
        
        // Auto-scroll to bottom
        this.realtimeLogEl.scrollTop = this.realtimeLogEl.scrollHeight;
    }

    startAutoRefresh() {
        // Refresh status every 30 seconds
        this.refreshInterval = setInterval(() => {
            this.refresh();
        }, 30000);
        
        // Initial load
        this.refresh();
    }

    stopAutoRefresh() {
        if (this.refreshInterval) {
            clearInterval(this.refreshInterval);
            this.refreshInterval = null;
        }
    }

    // Get current status for external access
    getStatus() {
        return this.systemStatus;
    }

    // Clear log entries
    clearLog() {
        if (this.realtimeLogEl) {
            this.realtimeLogEl.innerHTML = '';
            this.addLogEntry('Log cleared', 'info');
        }
    }

    // Export log entries
    exportLog() {
        const entries = this.realtimeLogEl.querySelectorAll('.log-entry');
        const logText = Array.from(entries).map(entry => entry.textContent).join('\n');
        
        const blob = new Blob([logText], { type: 'text/plain' });
        const url = URL.createObjectURL(blob);
        
        const a = document.createElement('a');
        a.href = url;
        a.download = `bjj-analyzer-log-${new Date().toISOString()}.txt`;
        a.click();
        
        URL.revokeObjectURL(url);
        
        window.toast.success('Log exported successfully');
    }

    // Cleanup method
    destroy() {
        this.stopAutoRefresh();
    }
}

// Initialize status monitor component
let statusMonitor;
document.addEventListener('DOMContentLoaded', () => {
    statusMonitor = new StatusMonitorComponent();
});

// Cleanup on page unload
window.addEventListener('beforeunload', () => {
    if (statusMonitor) {
        statusMonitor.destroy();
    }
});

window.StatusMonitorComponent = StatusMonitorComponent;

// Add CSS for metric update animation
const statusStyles = document.createElement('style');
statusStyles.textContent = `
    .metric-value.updated {
        background-color: rgba(37, 99, 235, 0.1);
        transition: background-color 0.5s ease;
    }
    
    .overall-progress {
        height: 4px;
        background-color: var(--primary-color);
        transition: width 0.3s ease;
    }
    
    .log-controls {
        display: flex;
        gap: 0.5rem;
        margin-bottom: 1rem;
    }
    
    .log-controls .btn {
        font-size: 0.75rem;
        padding: 0.375rem 0.75rem;
    }
`;
document.head.appendChild(statusStyles);