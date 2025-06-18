// BJJ Video Analyzer UI - API Communication Layer

class ApiClient {
    constructor() {
        this.baseUrl = Config.api.baseUrl;
        this.timeout = Config.api.timeout;
        this.retryAttempts = Config.api.retryAttempts;
        this.retryDelay = Config.api.retryDelay;
    }

    // Generic HTTP request method with retry logic
    async request(endpoint, options = {}) {
        const url = `${this.baseUrl}${endpoint}`;
        const config = {
            timeout: this.timeout,
            headers: {
                'Content-Type': 'application/json',
                ...options.headers
            },
            ...options
        };

        for (let attempt = 0; attempt < this.retryAttempts; attempt++) {
            try {
                const controller = new AbortController();
                const timeoutId = setTimeout(() => controller.abort(), this.timeout);

                const response = await fetch(url, {
                    ...config,
                    signal: controller.signal
                });

                clearTimeout(timeoutId);

                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
                }

                const data = await response.json();
                this.log('API Request Success:', { endpoint, data });
                return data;

            } catch (error) {
                this.log('API Request Failed:', { endpoint, attempt: attempt + 1, error: error.message });
                
                if (attempt === this.retryAttempts - 1) {
                    throw new Error(`API request failed after ${this.retryAttempts} attempts: ${error.message}`);
                }
                
                // Wait before retrying
                await this.delay(this.retryDelay * (attempt + 1));
            }
        }
    }

    // GET request
    async get(endpoint, params = {}) {
        const queryString = new URLSearchParams(params).toString();
        const url = queryString ? `${endpoint}?${queryString}` : endpoint;
        
        return this.request(url, {
            method: 'GET'
        });
    }

    // POST request
    async post(endpoint, data = {}) {
        return this.request(endpoint, {
            method: 'POST',
            body: JSON.stringify(data)
        });
    }

    // PUT request
    async put(endpoint, data = {}) {
        return this.request(endpoint, {
            method: 'PUT',
            body: JSON.stringify(data)
        });
    }

    // DELETE request
    async delete(endpoint) {
        return this.request(endpoint, {
            method: 'DELETE'
        });
    }

    // Video-related API calls
    async getVideos(filters = {}) {
        const response = await this.get('/videos', filters);
        // Extract videos array from the API response structure
        if (response && response.success && response.data && response.data.videos) {
            return response.data.videos;
        }
        // Fallback: if response format is unexpected, return empty array
        return [];
    }

    async getVideo(id) {
        const response = await this.get(`/videos/${id}`);
        // Extract video data from the API response structure
        if (response && response.success && response.data) {
            return response.data;
        }
        return null;
    }

    async processVideos(videoData) {
        return this.post('/videos/process', videoData);
    }

    // Series-related API calls
    async getSeries() {
        const response = await this.get('/series');
        if (response && response.success && response.data) {
            return response.data;
        }
        return [];
    }

    async getSeriesById(id) {
        const response = await this.get(`/series/${id}`);
        if (response && response.success && response.data) {
            return response.data;
        }
        return null;
    }

    // Corrections API calls
    async getCorrections() {
        const response = await this.get('/corrections');
        if (response && response.success && response.data) {
            return response.data;
        }
        return [];
    }

    async submitSeriesCorrection(correction) {
        const response = await this.post('/corrections/series', correction);
        if (response && response.success) {
            return response.data;
        }
        throw new Error(response?.error || 'Failed to submit series correction');
    }

    async submitProductCorrection(correction) {
        const response = await this.post('/corrections/products', correction);
        if (response && response.success) {
            return response.data;
        }
        throw new Error(response?.error || 'Failed to submit product correction');
    }

    // Status API calls
    async getStatus() {
        const response = await this.get('/status');
        if (response && response.success && response.data) {
            return response.data;
        }
        return {};
    }

    async getHealth() {
        const response = await this.get('/health');
        if (response && response.success && response.data) {
            return response.data;
        }
        return null;
    }

    // Utility methods
    delay(ms) {
        return new Promise(resolve => setTimeout(resolve, ms));
    }

    log(message, data = null) {
        if (Config.development.enableConsoleLogging) {
            if (data) {
                console.log(message, data);
            } else {
                console.log(message);
            }
        }
    }

    // Connection testing
    async testConnection() {
        try {
            const health = await this.getHealth();
            this.log('API Connection Test:', health);
            return true;
        } catch (error) {
            this.log('API Connection Test Failed:', error.message);
            return false;
        }
    }
}

// Toast notification system
class ToastManager {
    constructor() {
        this.container = document.getElementById('toast-container');
        this.toasts = [];
    }

    show(message, type = 'info', duration = Config.ui.toastDuration) {
        const toast = document.createElement('div');
        toast.className = `toast ${type}`;
        
        const icon = this.getIcon(type);
        toast.innerHTML = `
            <i class="${icon}"></i>
            <span>${message}</span>
            <button class="toast-close" onclick="this.parentElement.remove()">
                <i class="fas fa-times"></i>
            </button>
        `;

        this.container.appendChild(toast);
        this.toasts.push(toast);

        // Auto-remove after duration
        setTimeout(() => {
            if (toast.parentElement) {
                toast.remove();
                this.toasts = this.toasts.filter(t => t !== toast);
            }
        }, duration);

        return toast;
    }

    getIcon(type) {
        const icons = {
            success: 'fas fa-check-circle',
            error: 'fas fa-exclamation-circle',
            warning: 'fas fa-exclamation-triangle',
            info: 'fas fa-info-circle'
        };
        return icons[type] || icons.info;
    }

    success(message) {
        return this.show(message, 'success');
    }

    error(message) {
        return this.show(message, 'error');
    }

    warning(message) {
        return this.show(message, 'warning');
    }

    info(message) {
        return this.show(message, 'info');
    }

    clear() {
        this.toasts.forEach(toast => toast.remove());
        this.toasts = [];
    }
}

// Error handling utilities
class ErrorHandler {
    static handle(error, context = 'Unknown') {
        const message = error.message || 'An unexpected error occurred';
        
        console.error(`Error in ${context}:`, error);
        
        // Show user-friendly error message
        if (window.toast) {
            window.toast.error(`${context}: ${message}`);
        }
        
        // Log to analytics (if implemented)
        this.logError(error, context);
    }

    static logError(error, context) {
        // Implement error logging to external service if needed
        if (Config.development.enableConsoleLogging) {
            console.group(`Error Log: ${context}`);
            console.error('Error:', error);
            console.error('Stack:', error.stack);
            console.error('Context:', context);
            console.error('Timestamp:', new Date().toISOString());
            console.groupEnd();
        }
    }

    static async handleApiError(error, operation) {
        let userMessage = 'Network error occurred';
        
        if (error.message.includes('Failed to fetch')) {
            userMessage = 'Cannot connect to server. Please check if the backend is running.';
        } else if (error.message.includes('HTTP 404')) {
            userMessage = 'Requested resource not found';
        } else if (error.message.includes('HTTP 500')) {
            userMessage = 'Server error occurred';
        } else if (error.message.includes('timeout')) {
            userMessage = 'Request timed out';
        }
        
        this.handle(error, `API ${operation}`);
        
        if (window.toast) {
            window.toast.error(userMessage);
        }
    }
}

// Initialize global instances
window.api = new ApiClient();
window.toast = new ToastManager();
window.ErrorHandler = ErrorHandler;

// Add CSS for toast close button
const style = document.createElement('style');
style.textContent = `
    .toast-close {
        background: none;
        border: none;
        color: inherit;
        cursor: pointer;
        padding: 0;
        margin-left: auto;
        opacity: 0.7;
        transition: opacity 0.2s;
    }
    
    .toast-close:hover {
        opacity: 1;
    }
    
    .toast {
        position: relative;
        display: flex;
        align-items: center;
        gap: 0.75rem;
    }
`;
document.head.appendChild(style);