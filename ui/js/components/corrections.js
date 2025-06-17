// BJJ Video Analyzer UI - Corrections Component

class CorrectionsComponent {
    constructor() {
        this.corrections = [];
        this.initializeElements();
        this.bindEvents();
        this.loadCorrections();
    }

    initializeElements() {
        this.container = document.getElementById('corrections-container');
        this.exportButton = document.getElementById('export-corrections');
        this.seriesForm = document.getElementById('series-correction-form');
        this.productForm = document.getElementById('product-correction-form');
    }

    bindEvents() {
        this.exportButton.addEventListener('click', () => {
            this.exportCorrections();
        });

        // Form submissions are handled in app.js
    }

    async loadCorrections() {
        try {
            this.corrections = await window.api.getCorrections();
            this.renderCorrections();
        } catch (error) {
            console.warn('Could not load existing corrections:', error.message);
            // Don't show error to user as corrections might not be implemented yet
        }
    }

    renderCorrections() {
        // The corrections forms are already in the HTML
        // This method could add a list of existing corrections if implemented
    }

    exportCorrections() {
        window.toast.info('Export functionality coming soon');
        // TODO: Implement corrections export
    }

    // Utility methods for form validation
    validateSeriesCorrection(data) {
        const errors = [];
        
        if (!data.series_name || data.series_name.trim().length === 0) {
            errors.push('Series name is required');
        }
        
        if (!data.instructor || data.instructor.trim().length === 0) {
            errors.push('Instructor name is required');
        }
        
        if (!data.videos || data.videos.length === 0) {
            errors.push('At least one video file must be specified');
        }
        
        if (data.product_url && !this.isValidUrl(data.product_url)) {
            errors.push('Product URL must be a valid URL');
        }
        
        return errors;
    }

    validateProductCorrection(data) {
        const errors = [];
        
        if (!data.video_filename || data.video_filename.trim().length === 0) {
            errors.push('Video filename is required');
        }
        
        if (!data.product_url || !this.isValidUrl(data.product_url)) {
            errors.push('Valid product URL is required');
        }
        
        if (data.confidence < 0 || data.confidence > 1) {
            errors.push('Confidence must be between 0 and 100');
        }
        
        return errors;
    }

    isValidUrl(string) {
        try {
            const url = new URL(string);
            return url.protocol === 'http:' || url.protocol === 'https:';
        } catch (_) {
            return false;
        }
    }

    // Helper methods for form auto-completion
    suggestInstructorNames() {
        // TODO: Return list of known instructors from API
        return [
            'Gordon Ryan',
            'Craig Jones', 
            'Mikey Musumeci',
            'Adam Wardzinski',
            'John Danaher',
            'Bernardo Faria'
        ];
    }

    suggestSeriesNames() {
        // TODO: Return list of known series from API
        return [
            'Systematically Attacking The Guard',
            'Closed Guard Reintroduced',
            'Just Stand Up',
            'The Knee Shield System'
        ];
    }

    // Auto-fill helpers
    setupAutoComplete() {
        // TODO: Implement auto-complete for instructor and series names
        // This would use the suggestions above to help users fill forms
    }
}

// Initialize corrections component
let correctionsComponent;
document.addEventListener('DOMContentLoaded', () => {
    correctionsComponent = new CorrectionsComponent();
});

window.CorrectionsComponent = CorrectionsComponent;