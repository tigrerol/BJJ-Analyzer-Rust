// BJJ Video Analyzer UI - Video List Component

class VideoListComponent {
    constructor() {
        this.videos = [];
        this.filteredVideos = [];
        this.currentFilter = '';
        this.currentStatus = '';
        this.searchTimeout = null;
        
        this.initializeElements();
        this.bindEvents();
        this.loadVideos();
    }

    initializeElements() {
        this.container = document.getElementById('videos-container');
        this.searchInput = document.getElementById('video-search');
        this.statusFilter = document.getElementById('status-filter');
        this.refreshButton = document.getElementById('refresh-videos');
    }

    bindEvents() {
        // Search input with debouncing
        this.searchInput.addEventListener('input', (e) => {
            clearTimeout(this.searchTimeout);
            this.searchTimeout = setTimeout(() => {
                this.currentFilter = e.target.value.toLowerCase();
                this.filterVideos();
            }, Config.ui.debounceDelay);
        });

        // Status filter
        this.statusFilter.addEventListener('change', (e) => {
            this.currentStatus = e.target.value;
            this.filterVideos();
        });

        // Refresh button
        this.refreshButton.addEventListener('click', () => {
            this.loadVideos();
        });

        // Listen for WebSocket updates
        if (window.websocket) {
            window.websocket.on('processingUpdate', (data) => {
                this.handleProcessingUpdate(data);
            });
        }
    }

    async loadVideos() {
        this.showLoading();
        
        try {
            this.videos = await window.api.getVideos();
            this.filteredVideos = [...this.videos];
            this.filterVideos();
            this.renderVideos();
            
            window.toast.success(`Loaded ${this.videos.length} videos`);
        } catch (error) {
            window.ErrorHandler.handleApiError(error, 'Load Videos');
            this.showError('Failed to load videos');
        }
    }

    filterVideos() {
        this.filteredVideos = this.videos.filter(video => {
            const matchesSearch = !this.currentFilter || 
                video.filename.toLowerCase().includes(this.currentFilter) ||
                (video.metadata.instructor && video.metadata.instructor.toLowerCase().includes(this.currentFilter));
            
            const matchesStatus = !this.currentStatus || 
                video.status.toLowerCase() === this.currentStatus.toLowerCase();
            
            return matchesSearch && matchesStatus;
        });
        
        this.renderVideos();
    }

    renderVideos() {
        if (this.filteredVideos.length === 0) {
            this.showEmptyState();
            return;
        }

        const videoGrid = document.createElement('div');
        videoGrid.className = 'video-grid fade-in';
        
        this.filteredVideos.forEach(video => {
            const videoCard = this.createVideoCard(video);
            videoGrid.appendChild(videoCard);
        });
        
        this.container.innerHTML = '';
        this.container.appendChild(videoGrid);
    }

    createVideoCard(video) {
        const card = document.createElement('div');
        card.className = 'video-card slide-in-up';
        card.dataset.videoId = video.id;
        
        const statusClass = this.getStatusClass(video.status);
        const progress = this.calculateProgress(video);
        const duration = this.formatDuration(video.metadata.duration);
        const fileSize = this.formatFileSize(video.metadata.file_size);
        
        card.innerHTML = `
            <div class="video-header">
                <div class="video-title" title="${video.filename}">
                    ${this.truncateFilename(video.filename)}
                </div>
                <div class="video-status ${statusClass}">
                    ${video.status}
                </div>
            </div>
            
            <div class="video-meta">
                <div class="video-meta-item">
                    <span class="video-meta-label">
                        <i class="fas fa-clock"></i> Duration
                    </span>
                    <span class="video-meta-value">${duration}</span>
                </div>
                <div class="video-meta-item">
                    <span class="video-meta-label">
                        <i class="fas fa-hdd"></i> Size
                    </span>
                    <span class="video-meta-value">${fileSize}</span>
                </div>
                <div class="video-meta-item">
                    <span class="video-meta-label">
                        <i class="fas fa-cogs"></i> Stage
                    </span>
                    <span class="video-meta-value">${this.formatStage(video.current_stage)}</span>
                </div>
                <div class="video-meta-item">
                    <span class="video-meta-label">
                        <i class="fas fa-calendar"></i> Updated
                    </span>
                    <span class="video-meta-value">${this.formatDate(video.last_updated)}</span>
                </div>
            </div>
            
            ${progress < 100 ? `
                <div class="video-progress">
                    <div class="progress-bar">
                        <div class="progress-fill" style="width: ${progress}%"></div>
                    </div>
                    <div class="progress-text">${progress}% complete</div>
                </div>
            ` : ''}
            
            <div class="video-actions">
                <button class="btn btn-primary btn-sm" onclick="videoList.viewDetails('${video.id}')">
                    <i class="fas fa-eye"></i> Details
                </button>
                ${video.chapters && video.chapters.length > 0 ? `
                    <button class="btn btn-secondary btn-sm" onclick="videoList.viewChapters('${video.id}')">
                        <i class="fas fa-list"></i> Chapters (${video.chapters.length})
                    </button>
                ` : ''}
                ${video.status === 'completed' ? `
                    <button class="btn btn-success btn-sm" onclick="videoList.downloadResults('${video.id}')">
                        <i class="fas fa-download"></i> Download
                    </button>
                ` : ''}
                ${video.status === 'failed' ? `
                    <button class="btn btn-warning btn-sm" onclick="videoList.retryProcessing('${video.id}')">
                        <i class="fas fa-redo"></i> Retry
                    </button>
                ` : ''}
            </div>
        `;
        
        return card;
    }

    getStatusClass(status) {
        const statusMap = {
            'completed': 'completed',
            'processing': 'processing',
            'in_progress': 'processing',
            'pending': 'pending',
            'failed': 'failed',
            'cancelled': 'failed'
        };
        return statusMap[status.toLowerCase()] || 'pending';
    }

    calculateProgress(video) {
        if (video.status === 'completed') return 100;
        if (video.status === 'pending') return 0;
        
        // Calculate based on completed stages
        const totalStages = 7; // From the backend ProcessingStage enum
        const completedStages = video.completed_stages.length;
        return Math.round((completedStages / totalStages) * 100);
    }

    formatDuration(seconds) {
        if (!seconds) return 'Unknown';
        
        const hours = Math.floor(seconds / 3600);
        const minutes = Math.floor((seconds % 3600) / 60);
        const secs = Math.floor(seconds % 60);
        
        if (hours > 0) {
            return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
        } else {
            return `${minutes}:${secs.toString().padStart(2, '0')}`;
        }
    }

    formatFileSize(bytes) {
        if (!bytes) return 'Unknown';
        
        const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
        const i = Math.floor(Math.log(bytes) / Math.log(1024));
        return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${sizes[i]}`;
    }

    formatStage(stage) {
        return stage.replace(/([A-Z])/g, ' $1').trim();
    }

    formatDate(dateString) {
        const date = new Date(dateString);
        return date.toLocaleDateString() + ' ' + date.toLocaleTimeString([], {hour: '2-digit', minute:'2-digit'});
    }

    truncateFilename(filename, maxLength = 50) {
        if (filename.length <= maxLength) return filename;
        return filename.substring(0, maxLength - 3) + '...';
    }

    handleProcessingUpdate(data) {
        const videoCard = document.querySelector(`[data-video-id="${data.video_id}"]`);
        if (videoCard) {
            // Update progress if applicable
            const progressFill = videoCard.querySelector('.progress-fill');
            const progressText = videoCard.querySelector('.progress-text');
            
            if (progressFill && data.progress) {
                progressFill.style.width = `${data.progress}%`;
                if (progressText) {
                    progressText.textContent = `${data.progress}% complete`;
                }
            }
            
            // Update stage
            const stageElement = videoCard.querySelector('.video-meta-value:last-child');
            if (stageElement && data.stage) {
                stageElement.textContent = this.formatStage(data.stage);
            }
        }
        
        // Add to log
        if (window.websocket) {
            window.websocket.addLogEntry(`${data.video_id}: ${data.message}`);
        }
    }

    // Action handlers
    async viewDetails(videoId) {
        try {
            const video = await window.api.getVideo(videoId);
            this.showVideoDetailsModal(video);
        } catch (error) {
            window.ErrorHandler.handleApiError(error, 'View Video Details');
        }
    }

    viewChapters(videoId) {
        const video = this.videos.find(v => v.id === videoId);
        if (video && video.chapters) {
            this.showChaptersModal(video);
        }
    }

    async downloadResults(videoId) {
        window.toast.info('Download functionality not yet implemented');
        // TODO: Implement download functionality
    }

    async retryProcessing(videoId) {
        try {
            await window.api.processVideos({ video_ids: [videoId] });
            window.toast.success('Processing restarted');
            this.loadVideos(); // Refresh the list
        } catch (error) {
            window.ErrorHandler.handleApiError(error, 'Retry Processing');
        }
    }

    showVideoDetailsModal(video) {
        const modal = document.getElementById('modal-overlay');
        const content = document.getElementById('modal-content');
        
        content.innerHTML = `
            <div class="modal-header">
                <h3><i class="fas fa-film"></i> Video Details</h3>
                <button class="btn btn-secondary" onclick="this.closest('.modal-overlay').classList.remove('active')">
                    <i class="fas fa-times"></i> Close
                </button>
            </div>
            <div class="modal-body">
                <h4>${video.filename}</h4>
                <div class="video-details-grid">
                    <div class="detail-item">
                        <strong>Status:</strong> 
                        <span class="video-status ${this.getStatusClass(video.status)}">${video.status}</span>
                    </div>
                    <div class="detail-item">
                        <strong>Duration:</strong> ${this.formatDuration(video.metadata.duration)}
                    </div>
                    <div class="detail-item">
                        <strong>File Size:</strong> ${this.formatFileSize(video.metadata.file_size)}
                    </div>
                    <div class="detail-item">
                        <strong>Resolution:</strong> ${video.metadata.width}x${video.metadata.height}
                    </div>
                    <div class="detail-item">
                        <strong>Last Updated:</strong> ${this.formatDate(video.last_updated)}
                    </div>
                </div>
            </div>
        `;
        
        modal.classList.add('active');
    }

    showChaptersModal(video) {
        const modal = document.getElementById('modal-overlay');
        const content = document.getElementById('modal-content');
        
        const chaptersList = video.chapters.map((chapter, index) => `
            <div class="chapter-item">
                <div class="chapter-number">${index + 1}</div>
                <div class="chapter-details">
                    <div class="chapter-title">${chapter.title}</div>
                    <div class="chapter-timestamp">${this.formatDuration(chapter.timestamp)}</div>
                </div>
            </div>
        `).join('');
        
        content.innerHTML = `
            <div class="modal-header">
                <h3><i class="fas fa-list"></i> Chapters - ${video.filename}</h3>
                <button class="btn btn-secondary" onclick="this.closest('.modal-overlay').classList.remove('active')">
                    <i class="fas fa-times"></i> Close
                </button>
            </div>
            <div class="modal-body">
                <div class="chapters-list">
                    ${chaptersList}
                </div>
            </div>
        `;
        
        modal.classList.add('active');
    }

    showLoading() {
        this.container.innerHTML = `
            <div class="loading-spinner">
                <i class="fas fa-spinner fa-spin"></i>
                <p>Loading videos...</p>
            </div>
        `;
    }

    showError(message) {
        this.container.innerHTML = `
            <div class="empty-state">
                <i class="fas fa-exclamation-triangle"></i>
                <h3>Error Loading Videos</h3>
                <p>${message}</p>
                <button class="btn btn-primary" onclick="videoList.loadVideos()">
                    <i class="fas fa-retry"></i> Try Again
                </button>
            </div>
        `;
    }

    showEmptyState() {
        const message = this.currentFilter || this.currentStatus ? 
            'No videos match your current filters' : 
            'No videos found';
            
        this.container.innerHTML = `
            <div class="empty-state">
                <i class="fas fa-film"></i>
                <h3>No Videos</h3>
                <p>${message}</p>
                ${this.currentFilter || this.currentStatus ? `
                    <button class="btn btn-secondary" onclick="videoList.clearFilters()">
                        <i class="fas fa-filter"></i> Clear Filters
                    </button>
                ` : ''}
            </div>
        `;
    }

    clearFilters() {
        this.searchInput.value = '';
        this.statusFilter.value = '';
        this.currentFilter = '';
        this.currentStatus = '';
        this.filterVideos();
    }
}

// Initialize video list component
let videoList;
document.addEventListener('DOMContentLoaded', () => {
    videoList = new VideoListComponent();
});

// Export for global access
window.VideoListComponent = VideoListComponent;