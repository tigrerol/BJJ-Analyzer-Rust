// BJJ Video Analyzer UI - Series Manager Component

class SeriesManagerComponent {
    constructor() {
        this.series = [];
        this.initializeElements();
        this.bindEvents();
    }

    initializeElements() {
        this.container = document.getElementById('series-container');
        this.refreshButton = document.getElementById('refresh-series');
        this.createButton = document.getElementById('create-series');
    }

    bindEvents() {
        this.refreshButton.addEventListener('click', () => {
            this.loadSeries();
        });

        this.createButton.addEventListener('click', () => {
            this.showCreateSeriesModal();
        });
    }

    async loadSeries() {
        this.showLoading();
        
        try {
            this.series = await window.api.getSeries();
            this.renderSeries();
            
            window.toast.success(`Loaded ${this.series.length} series`);
        } catch (error) {
            window.ErrorHandler.handleApiError(error, 'Load Series');
            this.showError('Failed to load series');
        }
    }

    renderSeries() {
        if (this.series.length === 0) {
            this.showEmptyState();
            return;
        }

        const seriesGrid = document.createElement('div');
        seriesGrid.className = 'series-grid fade-in';
        
        this.series.forEach(series => {
            const seriesCard = this.createSeriesCard(series);
            seriesGrid.appendChild(seriesCard);
        });
        
        this.container.innerHTML = '';
        this.container.appendChild(seriesGrid);
    }

    createSeriesCard(series) {
        const card = document.createElement('div');
        card.className = 'series-card slide-in-up';
        
        card.innerHTML = `
            <div class="series-header">
                <div>
                    <div class="series-title">${series.name}</div>
                    <div class="series-instructor">by ${series.instructor}</div>
                </div>
                <div class="series-actions">
                    <button class="btn btn-primary btn-sm" onclick="seriesManager.editSeries('${series.id}')">
                        <i class="fas fa-edit"></i>
                    </button>
                </div>
            </div>
            
            <div class="series-stats">
                <div class="series-stat">
                    <span class="series-stat-value">${series.videos.length}</span>
                    <span class="series-stat-label">Videos</span>
                </div>
                <div class="series-stat">
                    <span class="series-stat-value">${Math.round(series.total_duration / 60)}m</span>
                    <span class="series-stat-label">Duration</span>
                </div>
                <div class="series-stat">
                    <span class="series-stat-value">${series.completion_status.percentage}%</span>
                    <span class="series-stat-label">Complete</span>
                </div>
            </div>
            
            <div class="series-videos">
                <div class="series-videos-header">
                    <span class="series-videos-title">Videos (${series.videos.length})</span>
                </div>
                <div class="series-videos-list">
                    ${series.videos.slice(0, 5).map(video => `
                        <div class="series-video-item">
                            <span class="series-video-name">${video.filename}</span>
                            <span class="video-status ${this.getStatusClass(video.status)}">${video.status}</span>
                        </div>
                    `).join('')}
                    ${series.videos.length > 5 ? `
                        <div class="series-video-item">
                            <span class="series-video-name">... and ${series.videos.length - 5} more</span>
                        </div>
                    ` : ''}
                </div>
            </div>
        `;
        
        return card;
    }

    getStatusClass(status) {
        const statusMap = {
            'completed': 'completed',
            'processing': 'processing',
            'pending': 'pending',
            'failed': 'failed'
        };
        return statusMap[status.toLowerCase()] || 'pending';
    }

    showCreateSeriesModal() {
        const modal = document.getElementById('modal-overlay');
        const content = document.getElementById('modal-content');
        
        content.innerHTML = `
            <div class="modal-header">
                <h3><i class="fas fa-plus-circle"></i> Create New Series</h3>
                <button class="btn btn-secondary" onclick="this.closest('.modal-overlay').classList.remove('active')">
                    <i class="fas fa-times"></i> Cancel
                </button>
            </div>
            <div class="modal-body">
                <form id="create-series-form" class="series-form">
                    <div class="form-group">
                        <label for="series-name">Series Name *</label>
                        <input type="text" id="series-name" name="series_name" required 
                               placeholder="e.g., Closed Guard Mastery">
                    </div>
                    
                    <div class="form-group">
                        <label for="series-instructor">Instructor *</label>
                        <input type="text" id="series-instructor" name="instructor" required 
                               placeholder="e.g., John Danaher">
                    </div>
                    
                    <div class="form-group">
                        <label for="series-url">BJJfanatics Product URL</label>
                        <input type="url" id="series-url" name="product_url" 
                               placeholder="https://bjjfanatics.com/products/...">
                    </div>
                    
                    <div class="form-group">
                        <label>Select Videos for this Series</label>
                        <div id="video-selection" class="video-selection-list">
                            <div class="loading-spinner">
                                <i class="fas fa-spinner fa-spin"></i> Loading videos...
                            </div>
                        </div>
                    </div>
                    
                    <div class="form-actions">
                        <button type="submit" class="btn btn-primary">
                            <i class="fas fa-save"></i> Create Series
                        </button>
                        <button type="button" class="btn btn-secondary" 
                                onclick="document.getElementById('modal-overlay').classList.remove('active')">
                            <i class="fas fa-times"></i> Cancel
                        </button>
                    </div>
                </form>
            </div>
        `;
        
        modal.classList.add('active');
        
        // Load available videos
        this.loadVideosForSelection();
        
        // Handle form submission
        document.getElementById('create-series-form').addEventListener('submit', async (e) => {
            e.preventDefault();
            await this.handleCreateSeries(e.target);
        });
    }

    async editSeries(seriesId) {
        try {
            const series = await window.api.getSeriesById(seriesId);
            this.showEditSeriesModal(series);
        } catch (error) {
            window.ErrorHandler.handleApiError(error, 'Load Series for Editing');
        }
    }
    
    showEditSeriesModal(series) {
        const modal = document.getElementById('modal-overlay');
        const content = document.getElementById('modal-content');
        
        content.innerHTML = `
            <div class="modal-header">
                <h3><i class="fas fa-edit"></i> Edit Series: ${series.name}</h3>
                <button class="btn btn-secondary" onclick="this.closest('.modal-overlay').classList.remove('active')">
                    <i class="fas fa-times"></i> Cancel
                </button>
            </div>
            <div class="modal-body">
                <form id="edit-series-form" class="series-form">
                    <input type="hidden" name="series_id" value="${series.id}">
                    
                    <div class="form-group">
                        <label for="series-name">Series Name *</label>
                        <input type="text" id="series-name" name="series_name" required 
                               value="${series.name}">
                    </div>
                    
                    <div class="form-group">
                        <label for="series-instructor">Instructor *</label>
                        <input type="text" id="series-instructor" name="instructor" required 
                               value="${series.instructor}">
                    </div>
                    
                    <div class="form-group">
                        <label for="series-url">BJJfanatics Product URL</label>
                        <input type="url" id="series-url" name="product_url" 
                               value="${series.product_url || ''}"
                               placeholder="https://bjjfanatics.com/products/...">
                    </div>
                    
                    <div class="form-group">
                        <label>Videos in this Series</label>
                        <div id="video-selection" class="video-selection-list">
                            ${series.videos.map(video => `
                                <label class="video-checkbox">
                                    <input type="checkbox" name="video_ids" value="${video.id}" checked>
                                    <span>${video.filename}</span>
                                    <span class="video-status ${this.getStatusClass(video.status)}">${video.status}</span>
                                </label>
                            `).join('')}
                        </div>
                    </div>
                    
                    <div class="form-actions">
                        <button type="submit" class="btn btn-primary">
                            <i class="fas fa-save"></i> Save Changes
                        </button>
                        <button type="button" class="btn btn-secondary" 
                                onclick="document.getElementById('modal-overlay').classList.remove('active')">
                            <i class="fas fa-times"></i> Cancel
                        </button>
                    </div>
                </form>
            </div>
        `;
        
        modal.classList.add('active');
        
        // Handle form submission
        document.getElementById('edit-series-form').addEventListener('submit', async (e) => {
            e.preventDefault();
            await this.handleEditSeries(e.target);
        });
    }
    
    async loadVideosForSelection() {
        try {
            const videos = await window.api.getVideos();
            const container = document.getElementById('video-selection');
            
            if (videos.length === 0) {
                container.innerHTML = '<p class="text-muted">No videos available</p>';
                return;
            }
            
            container.innerHTML = videos.map(video => `
                <label class="video-checkbox">
                    <input type="checkbox" name="video_ids" value="${video.id}">
                    <span>${video.filename}</span>
                    <span class="video-status ${this.getStatusClass(video.status)}">${video.status}</span>
                </label>
            `).join('');
            
        } catch (error) {
            document.getElementById('video-selection').innerHTML = 
                '<p class="text-danger">Failed to load videos</p>';
        }
    }
    
    async handleCreateSeries(form) {
        const formData = new FormData(form);
        const selectedVideos = formData.getAll('video_ids');
        
        if (selectedVideos.length === 0) {
            window.toast.warning('Please select at least one video for the series');
            return;
        }
        
        const seriesData = {
            series_name: formData.get('series_name'),
            instructor: formData.get('instructor'),
            videos: selectedVideos,
            product_url: formData.get('product_url') || null
        };
        
        try {
            await window.api.submitSeriesCorrection(seriesData);
            window.toast.success('Series created successfully');
            document.getElementById('modal-overlay').classList.remove('active');
            this.loadSeries(); // Refresh the series list
        } catch (error) {
            window.ErrorHandler.handleApiError(error, 'Create Series');
        }
    }
    
    async handleEditSeries(form) {
        const formData = new FormData(form);
        const selectedVideos = formData.getAll('video_ids');
        
        if (selectedVideos.length === 0) {
            window.toast.warning('A series must have at least one video');
            return;
        }
        
        const seriesData = {
            series_name: formData.get('series_name'),
            instructor: formData.get('instructor'),
            videos: selectedVideos,
            product_url: formData.get('product_url') || null
        };
        
        try {
            await window.api.submitSeriesCorrection(seriesData);
            window.toast.success('Series updated successfully');
            document.getElementById('modal-overlay').classList.remove('active');
            this.loadSeries(); // Refresh the series list
        } catch (error) {
            window.ErrorHandler.handleApiError(error, 'Update Series');
        }
    }

    showLoading() {
        this.container.innerHTML = `
            <div class="loading-spinner">
                <i class="fas fa-spinner fa-spin"></i>
                <p>Loading series...</p>
            </div>
        `;
    }

    showError(message) {
        this.container.innerHTML = `
            <div class="empty-state">
                <i class="fas fa-exclamation-triangle"></i>
                <h3>Error Loading Series</h3>
                <p>${message}</p>
                <button class="btn btn-primary" onclick="seriesManager.loadSeries()">
                    <i class="fas fa-retry"></i> Try Again
                </button>
            </div>
        `;
    }

    showEmptyState() {
        this.container.innerHTML = `
            <div class="empty-state">
                <i class="fas fa-layer-group"></i>
                <h3>No Series Found</h3>
                <p>No video series have been created yet</p>
                <button class="btn btn-primary" onclick="seriesManager.showCreateSeriesModal()">
                    <i class="fas fa-plus"></i> Create First Series
                </button>
            </div>
        `;
    }
}

// Initialize series manager component
let seriesManager;
document.addEventListener('DOMContentLoaded', () => {
    seriesManager = new SeriesManagerComponent();
});

window.SeriesManagerComponent = SeriesManagerComponent;