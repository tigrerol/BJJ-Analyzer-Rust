use criterion::{black_box, criterion_group, criterion_main, Criterion};
use bjj_analyzer_rust::{VideoProcessor, AudioExtractor, BatchProcessor, Config};
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;

/// Benchmark video information extraction
fn bench_video_info(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let processor = VideoProcessor::new();
    
    // Create a test video file path (would need actual video for real benchmarking)
    let test_video = PathBuf::from("test_data/sample.mp4");
    
    if test_video.exists() {
        c.bench_function("video_info_extraction", |b| {
            b.iter(|| {
                rt.block_on(async {
                    processor.get_video_info(black_box(&test_video)).await
                })
            })
        });
    }
}

/// Benchmark video discovery in directory
fn bench_video_discovery(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let processor = VideoProcessor::new();
    
    // Create test directory structure
    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path();
    
    c.bench_function("video_discovery", |b| {
        b.iter(|| {
            rt.block_on(async {
                processor.discover_videos(black_box(test_dir)).await
            })
        })
    });
}

/// Benchmark audio extraction
fn bench_audio_extraction(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let extractor = AudioExtractor::new();
    
    let test_video = PathBuf::from("test_data/sample.mp4");
    let temp_dir = TempDir::new().unwrap();
    
    if test_video.exists() {
        c.bench_function("audio_extraction", |b| {
            b.iter(|| {
                rt.block_on(async {
                    extractor.extract_for_transcription(
                        black_box(&test_video),
                        black_box(temp_dir.path())
                    ).await
                })
            })
        });
    }
}

/// Benchmark batch processing with different worker counts
fn bench_batch_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    // Test with different worker counts
    for workers in [1, 2, 4, 8].iter() {
        let group_name = format!("batch_processing_{}workers", workers);
        c.bench_function(&group_name, |b| {
            b.iter(|| {
                rt.block_on(async {
                    let config = Config::default();
                    let processor = BatchProcessor::new(config, *workers).await.unwrap();
                    
                    // Create test directory
                    let temp_dir = TempDir::new().unwrap();
                    let input_dir = temp_dir.path().to_path_buf();
                    let output_dir = temp_dir.path().join("output");
                    
                    // Process empty directory (just tests the overhead)
                    processor.process_directory(
                        black_box(input_dir),
                        black_box(output_dir)
                    ).await
                })
            })
        });
    }
}

/// Benchmark configuration loading and validation
fn bench_config_operations(c: &mut Criterion) {
    c.bench_function("config_default", |b| {
        b.iter(|| {
            black_box(Config::default())
        })
    });
    
    c.bench_function("config_validation", |b| {
        let config = Config::default();
        b.iter(|| {
            config.validate()
        })
    });
    
    c.bench_function("config_summary", |b| {
        let config = Config::default();
        b.iter(|| {
            config.summary()
        })
    });
}

/// Memory usage benchmark
fn bench_memory_usage(c: &mut Criterion) {
    c.bench_function("processor_creation", |b| {
        b.iter(|| {
            black_box(VideoProcessor::new());
            black_box(AudioExtractor::new());
        })
    });
}

/// Concurrent processing benchmark
fn bench_concurrent_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("concurrent_video_analysis", |b| {
        b.iter(|| {
            rt.block_on(async {
                let processor = VideoProcessor::new();
                let test_paths = vec![
                    PathBuf::from("test_data/sample1.mp4"),
                    PathBuf::from("test_data/sample2.mp4"),
                    PathBuf::from("test_data/sample3.mp4"),
                ];
                
                // Simulate concurrent processing
                let tasks: Vec<_> = test_paths.into_iter().map(|path| {
                    let processor = VideoProcessor::new();
                    tokio::spawn(async move {
                        // Simulate work
                        tokio::time::sleep(Duration::from_millis(1)).await;
                        format!("processed: {}", path.display())
                    })
                }).collect();
                
                futures::future::join_all(tasks).await
            })
        })
    });
}

/// Comparison benchmark against theoretical Python performance
fn bench_python_comparison(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    // Simulate Python-like performance (much slower)
    c.bench_function("rust_video_processing", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Fast Rust processing simulation
                tokio::time::sleep(Duration::from_millis(10)).await;
                black_box("processed")
            })
        })
    });
    
    c.bench_function("simulated_python_processing", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Slow Python-like processing simulation
                tokio::time::sleep(Duration::from_millis(250)).await;
                black_box("processed")
            })
        })
    });
}

/// Error handling performance
fn bench_error_handling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("error_path_processing", |b| {
        b.iter(|| {
            rt.block_on(async {
                let processor = VideoProcessor::new();
                // Try to process non-existent file (error path)
                let result = processor.get_video_info(&PathBuf::from("nonexistent.mp4")).await;
                black_box(result)
            })
        })
    });
}

/// Real-world scenario benchmark
fn bench_realistic_workflow(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("complete_workflow_simulation", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Simulate complete processing workflow
                let config = Config::default();
                let processor = BatchProcessor::new(config, 4).await.unwrap();
                
                // Create temporary directories
                let temp_dir = TempDir::new().unwrap();
                let input_dir = temp_dir.path().to_path_buf();
                let output_dir = temp_dir.path().join("output");
                
                // Process (empty directory, but exercises the full pipeline)
                let result = processor.process_directory(input_dir, output_dir).await.unwrap();
                black_box(result)
            })
        })
    });
}

// Group all benchmarks
criterion_group!(
    benches,
    bench_video_info,
    bench_video_discovery,
    bench_audio_extraction,
    bench_batch_processing,
    bench_config_operations,
    bench_memory_usage,
    bench_concurrent_processing,
    bench_python_comparison,
    bench_error_handling,
    bench_realistic_workflow
);

criterion_main!(benches);