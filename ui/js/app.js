// BJJ Video Analyzer UI - Main Application

class BJJAnalyzerApp {
    constructor() {
        this.currentTab = 'videos';
        this.isInitialized = false;
        
        this.init();
    }

    async init() {
        console.log('🥋 BJJ Video Analyzer UI starting...');
        
        // Show loading state
        this.showGlobalLoading();
        
        try {
            // Initialize configuration
            Config.debug();
            
            // Test API connection
            const apiHealthy = await this.testApiConnection();
            if (!apiHealthy) {
                this.showConnectionError();
                return;
            }
            
            // Initialize UI components
            this.initializeNavigation();
            this.initializeModals();
            this.initializeKeyboardShortcuts();
            
            // Set up WebSocket listeners
            this.setupWebSocketListeners();
            
            // Set up correction forms
            this.setupCorrectionForms();
            
            // Initialize tab-specific components
            this.initializeComponents();
            
            this.isInitialized = true;
            this.hideGlobalLoading();
            
            console.log('✅ BJJ Video Analyzer UI initialized successfully');
            window.toast.success('Application loaded successfully');
            
        } catch (error) {
            console.error('❌ Failed to initialize application:', error);
            this.showInitializationError(error);
        }
    }

    async testApiConnection() {
        try {
            const health = await window.api.getHealth();
            console.log('🔗 API connection successful:', health);
            return true;
        } catch (error) {
            console.error('🔗 API connection failed:', error);
            return false;
        }
    }

    initializeNavigation() {
        const navTabs = document.querySelectorAll('.nav-tab');
        navTabs.forEach(tab => {
            tab.addEventListener('click', () => {
                const tabName = tab.dataset.tab;
                this.switchTab(tabName);
            });
        });
    }

    switchTab(tabName) {
        if (this.currentTab === tabName) return;
        
        // Update navigation
        document.querySelectorAll('.nav-tab').forEach(tab => {
            tab.classList.toggle('active', tab.dataset.tab === tabName);
        });
        
        // Update content
        document.querySelectorAll('.tab-content').forEach(content => {
            content.classList.toggle('active', content.id === `${tabName}-tab`);
        });
        
        this.currentTab = tabName;
        
        // Initialize tab-specific functionality if needed
        this.onTabChange(tabName);
    }

    onTabChange(tabName) {
        switch (tabName) {
            case 'videos':
                if (window.videoList) {
                    window.videoList.loadVideos();
                }
                break;
            case 'series':
                if (window.seriesManager) {
                    window.seriesManager.loadSeries();
                }
                break;
            case 'status':
                if (window.statusMonitor) {
                    window.statusMonitor.refresh();
                }
                break;
            case 'corrections':
                // Corrections tab is static forms, no loading needed
                break;
        }
    }

    initializeModals() {
        const modalOverlay = document.getElementById('modal-overlay');
        
        // Close modal when clicking overlay
        modalOverlay.addEventListener('click', (e) => {
            if (e.target === modalOverlay) {
                modalOverlay.classList.remove('active');
            }
        });
        
        // Close modal with escape key
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape' && modalOverlay.classList.contains('active')) {
                modalOverlay.classList.remove('active');
            }
        });
    }

    initializeKeyboardShortcuts() {
        document.addEventListener('keydown', (e) => {
            // Only handle shortcuts when not typing in inputs
            if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') {
                return;
            }
            
            // Ctrl/Cmd + number keys for tab switching
            if ((e.ctrlKey || e.metaKey) && e.key >= '1' && e.key <= '4') {
                e.preventDefault();
                const tabNames = ['videos', 'series', 'corrections', 'status'];
                const tabIndex = parseInt(e.key) - 1;
                if (tabNames[tabIndex]) {
                    this.switchTab(tabNames[tabIndex]);
                }
            }
            
            // R for refresh current tab
            if (e.key === 'r' || e.key === 'R') {
                e.preventDefault();
                this.refreshCurrentTab();
            }
        });
    }

    refreshCurrentTab() {
        const refreshButtons = {
            'videos': document.getElementById('refresh-videos'),
            'series': document.getElementById('refresh-series'),
            'status': document.getElementById('refresh-status')
        };
        
        const button = refreshButtons[this.currentTab];
        if (button) {
            button.click();
        }
    }

    setupWebSocketListeners() {
        if (!window.websocket) return;
        
        window.websocket.on('connected', () => {
            window.toast.success('Real-time connection established');
        });
        
        window.websocket.on('disconnected', () => {
            window.toast.warning('Real-time connection lost');
        });
        
        window.websocket.on('error', (error) => {
            console.error('WebSocket error:', error);
            window.toast.error('Real-time connection error');
        });
        
        window.websocket.on('systemStatus', (status) => {
            this.updateSystemStatusInHeader(status);
        });
    }
    
    setupCorrectionForms() {
        // Series correction form
        const seriesForm = document.getElementById('series-correction-form');
        if (seriesForm) {
            seriesForm.addEventListener('submit', async (e) => {
                e.preventDefault();
                await this.handleSeriesCorrection(e.target);
            });
        }
        
        // Product correction form
        const productForm = document.getElementById('product-correction-form');
        if (productForm) {
            productForm.addEventListener('submit', async (e) => {
                e.preventDefault();
                await this.handleProductCorrection(e.target);
            });
        }
    }
    
    async handleSeriesCorrection(form) {
        const formData = new FormData(form);
        const videoFiles = document.getElementById('video-files').value || '';
        const videos = videoFiles.split('\n')
            .map(line => line.trim())
            .filter(line => line.length > 0);
        
        const correctionData = {
            series_name: document.getElementById('series-name').value,
            instructor: document.getElementById('instructor-name').value,
            videos: videos,
            product_url: document.getElementById('product-url').value || null
        };
        
        // Validate
        if (!correctionData.series_name || !correctionData.instructor) {
            window.toast.error('Series name and instructor are required');
            return;
        }
        
        if (videos.length === 0) {
            window.toast.error('At least one video file must be specified');
            return;
        }
        
        try {
            await window.api.submitSeriesCorrection(correctionData);
            window.toast.success('Series correction submitted successfully');
            form.reset();
        } catch (error) {
            window.ErrorHandler.handleApiError(error, 'Submit Series Correction');
        }
    }
    
    async handleProductCorrection(form) {
        const formData = new FormData(form);
        const confidence = parseInt(document.getElementById('confidence').value) / 100; // Convert to 0-1 range
        
        const correctionData = {
            video_filename: document.getElementById('video-filename').value,
            product_url: document.getElementById('correct-product-url').value,
            confidence: confidence
        };
        
        // Validate
        if (!correctionData.video_filename || !correctionData.product_url) {
            window.toast.error('Video filename and product URL are required');
            return;
        }
        
        try {
            await window.api.submitProductCorrection(correctionData);
            window.toast.success('Product correction submitted successfully');
            form.reset();
            // Reset confidence to default
            document.getElementById('confidence').value = 95;
        } catch (error) {
            window.ErrorHandler.handleApiError(error, 'Submit Product Correction');
        }
    }

    updateSystemStatusInHeader(status) {
        // Update connection indicator with system info
        const indicator = document.getElementById('connection-indicator');
        if (indicator && status.total_videos !== undefined) {
            const title = `${status.total_videos} videos (${status.processing_videos} processing, ${status.completed_videos} completed)`;
            indicator.title = title;
        }
    }

    initializeComponents() {
        // Video list component is already initialized in its own file
        // Other components will be initialized when their tabs are accessed
        
        // Initialize corrections forms
        this.initializeCorrectionsForm();
    }

    initializeCorrectionsForm() {
        // Series correction form
        const seriesForm = document.getElementById('series-correction-form');
        if (seriesForm) {
            seriesForm.addEventListener('submit', async (e) => {
                e.preventDefault();
                await this.submitSeriesCorrection(new FormData(seriesForm));
            });
        }
        
        // Product correction form
        const productForm = document.getElementById('product-correction-form');
        if (productForm) {
            productForm.addEventListener('submit', async (e) => {
                e.preventDefault();
                await this.submitProductCorrection(new FormData(productForm));
            });
        }
    }

    async submitSeriesCorrection(formData) {
        try {
            const correction = {
                series_name: formData.get('series-name') || document.getElementById('series-name').value,
                instructor: formData.get('instructor-name') || document.getElementById('instructor-name').value,
                videos: (formData.get('video-files') || document.getElementById('video-files').value)
                    .split('\n')
                    .map(line => line.trim())
                    .filter(line => line.length > 0),
                product_url: formData.get('product-url') || document.getElementById('product-url').value || null
            };
            
            if (!correction.series_name || !correction.instructor) {
                window.toast.error('Series name and instructor are required');
                return;
            }
            
            await window.api.submitSeriesCorrection(correction);
            window.toast.success('Series correction submitted successfully');
            
            // Reset form
            document.getElementById('series-correction-form').reset();
            
        } catch (error) {
            window.ErrorHandler.handleApiError(error, 'Submit Series Correction');
        }
    }

    async submitProductCorrection(formData) {
        try {
            const correction = {
                video_filename: formData.get('video-filename') || document.getElementById('video-filename').value,
                product_url: formData.get('correct-product-url') || document.getElementById('correct-product-url').value,
                confidence: parseFloat(formData.get('confidence') || document.getElementById('confidence').value) / 100
            };
            
            if (!correction.video_filename || !correction.product_url) {
                window.toast.error('Video filename and product URL are required');
                return;
            }
            
            await window.api.submitProductCorrection(correction);
            window.toast.success('Product correction submitted successfully');
            
            // Reset form
            document.getElementById('product-correction-form').reset();
            
        } catch (error) {
            window.ErrorHandler.handleApiError(error, 'Submit Product Correction');
        }
    }

    showGlobalLoading() {
        const loadingOverlay = document.createElement('div');
        loadingOverlay.id = 'global-loading';
        loadingOverlay.innerHTML = `
            <div class="loading-spinner">
                <i class="fas fa-spinner fa-spin"></i>
                <p>Initializing BJJ Video Analyzer...</p>
            </div>
        `;
        loadingOverlay.style.cssText = `
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background: rgba(255, 255, 255, 0.95);
            z-index: 9999;
            display: flex;
            align-items: center;
            justify-content: center;
        `;
        document.body.appendChild(loadingOverlay);
    }

    hideGlobalLoading() {
        const loadingOverlay = document.getElementById('global-loading');
        if (loadingOverlay) {
            loadingOverlay.remove();
        }
    }

    showConnectionError() {
        this.hideGlobalLoading();
        
        const errorOverlay = document.createElement('div');
        errorOverlay.innerHTML = `
            <div class="error-container">
                <div class="error-icon">
                    <i class="fas fa-exclamation-triangle"></i>
                </div>
                <h2>Cannot Connect to Backend</h2>
                <p>Make sure the BJJ Analyzer Rust backend is running with API enabled:</p>
                <code>cargo run --features api -- --video-dir /path/to/videos --api-port 8080</code>
                <div class="error-actions">
                    <button class="btn btn-primary" onclick="location.reload()">
                        <i class="fas fa-sync-alt"></i> Retry Connection
                    </button>
                    <button class="btn btn-secondary" onclick="window.Config.checkApiHealth().then(healthy => healthy && location.reload())">
                        <i class="fas fa-check"></i> Test Connection
                    </button>
                </div>
            </div>
        `;
        errorOverlay.style.cssText = `
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background: var(--background-color);
            z-index: 9999;
            display: flex;
            align-items: center;
            justify-content: center;
            text-align: center;
            padding: 2rem;
        `;
        document.body.appendChild(errorOverlay);
    }

    showInitializationError(error) {
        this.hideGlobalLoading();
        
        const errorOverlay = document.createElement('div');
        errorOverlay.innerHTML = `
            <div class="error-container">
                <div class="error-icon">
                    <i class="fas fa-bug"></i>
                </div>
                <h2>Initialization Error</h2>
                <p>Failed to initialize the application:</p>
                <code>${error.message}</code>
                <div class="error-actions">
                    <button class="btn btn-primary" onclick="location.reload()">
                        <i class="fas fa-sync-alt"></i> Reload Application
                    </button>
                </div>
            </div>
        `;
        errorOverlay.style.cssText = `
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background: var(--background-color);
            z-index: 9999;
            display: flex;
            align-items: center;
            justify-content: center;
            text-align: center;
            padding: 2rem;
        `;
        document.body.appendChild(errorOverlay);
    }

    // Public methods for debugging
    getStatus() {
        return {
            initialized: this.isInitialized,
            currentTab: this.currentTab,
            apiConnected: window.api ? true : false,
            websocketConnected: window.websocket ? window.websocket.getStatus() : null
        };
    }

    switchToTab(tabName) {
        this.switchTab(tabName);
    }

    refresh() {
        this.refreshCurrentTab();
    }
}

// Initialize application when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    window.app = new BJJAnalyzerApp();
});

// Make app globally available for debugging
window.BJJAnalyzerApp = BJJAnalyzerApp;

// Add some additional CSS for error states
const additionalStyles = document.createElement('style');
additionalStyles.textContent = `
    .error-container {
        max-width: 600px;
        margin: 0 auto;
    }
    
    .error-icon {
        font-size: 4rem;
        color: var(--danger-color);
        margin-bottom: 1rem;
    }
    
    .error-container h2 {
        color: var(--text-primary);
        margin-bottom: 1rem;
    }
    
    .error-container p {
        color: var(--text-secondary);
        margin-bottom: 1rem;
        line-height: 1.6;
    }
    
    .error-container code {
        display: block;
        background: var(--background-color);
        padding: 1rem;
        border-radius: var(--border-radius);
        margin: 1rem 0;
        font-family: monospace;
        color: var(--text-primary);
        border: 1px solid var(--border-color);
    }
    
    .error-actions {
        display: flex;
        gap: 1rem;
        justify-content: center;
        margin-top: 2rem;
    }
    
    .modal-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 1.5rem;
        padding-bottom: 1rem;
        border-bottom: 1px solid var(--border-color);
    }
    
    .modal-header h3 {
        margin: 0;
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }
    
    .chapters-list {
        max-height: 400px;
        overflow-y: auto;
    }
    
    .chapter-item {
        display: flex;
        align-items: center;
        gap: 1rem;
        padding: 0.75rem;
        border-bottom: 1px solid var(--border-color);
    }
    
    .chapter-item:last-child {
        border-bottom: none;
    }
    
    .chapter-number {
        background: var(--primary-color);
        color: white;
        border-radius: 50%;
        width: 2rem;
        height: 2rem;
        display: flex;
        align-items: center;
        justify-content: center;
        font-weight: 600;
        font-size: 0.875rem;
        flex-shrink: 0;
    }
    
    .chapter-details {
        flex: 1;
    }
    
    .chapter-title {
        font-weight: 500;
        color: var(--text-primary);
        margin-bottom: 0.25rem;
    }
    
    .chapter-timestamp {
        font-size: 0.875rem;
        color: var(--text-secondary);
    }
    
    .video-details-grid {
        display: grid;
        gap: 1rem;
        margin-top: 1rem;
    }
    
    .detail-item {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 0.75rem;
        background: var(--background-color);
        border-radius: var(--border-radius);
    }
`;
document.head.appendChild(additionalStyles);