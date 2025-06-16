/// Python-Rust Bridge using PyO3
/// Provides Python interface to Rust video processing functions

#[cfg(feature = "python-bindings")]
mod python_bindings {
    use pyo3::prelude::*;
    use pyo3::types::{PyDict, PyList};
    use std::path::PathBuf;
    use tokio::runtime::Runtime;
    
    use crate::{BatchProcessor, Config, ConfigBuilder};
    use crate::config::{TranscriptionProvider, ExportFormat};

    /// Python wrapper for BatchProcessor
    #[pyclass]
    pub struct PyBatchProcessor {
        processor: BatchProcessor,
        runtime: Runtime,
    }

    #[pymethods]
    impl PyBatchProcessor {
        #[new]
        fn new(workers: Option<usize>) -> PyResult<Self> {
            let config = ConfigBuilder::new()
                .with_workers(workers.unwrap_or(4))
                .build();

            let runtime = Runtime::new()
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to create runtime: {}", e)))?;

            let processor = runtime.block_on(async {
                BatchProcessor::new(config, workers.unwrap_or(4)).await
            }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to create processor: {}", e)))?;

            Ok(Self { processor, runtime })
        }

        /// Process videos in a directory
        fn process_directory(&self, input_dir: String, output_dir: String) -> PyResult<PyDict> {
            let input_path = PathBuf::from(input_dir);
            let output_path = PathBuf::from(output_dir);

            let result = self.runtime.block_on(async {
                self.processor.process_directory(input_path, output_path).await
            }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Processing failed: {}", e)))?;

            Python::with_gil(|py| {
                let dict = PyDict::new(py);
                dict.set_item("total", result.total)?;
                dict.set_item("successful", result.successful)?;
                dict.set_item("failed", result.failed)?;
                dict.set_item("total_time", result.total_time.as_secs_f64())?;
                
                let results_list = PyList::empty(py);
                for video_result in result.results {
                    let video_dict = PyDict::new(py);
                    video_dict.set_item("filename", video_result.video_info.filename)?;
                    video_dict.set_item("duration", video_result.video_info.duration.as_secs_f64())?;
                    video_dict.set_item("width", video_result.video_info.width)?;
                    video_dict.set_item("height", video_result.video_info.height)?;
                    video_dict.set_item("fps", video_result.video_info.fps)?;
                    video_dict.set_item("processing_time", video_result.processing_time.as_secs_f64())?;
                    video_dict.set_item("status", format!("{:?}", video_result.status))?;
                    
                    if let Some(error) = video_result.error_message {
                        video_dict.set_item("error", error)?;
                    }
                    
                    results_list.append(video_dict)?;
                }
                dict.set_item("results", results_list)?;
                
                Ok(dict.into())
            })
        }

        /// Get processor statistics
        fn get_stats(&self) -> PyResult<PyDict> {
            let stats = self.processor.get_stats();
            
            Python::with_gil(|py| {
                let dict = PyDict::new(py);
                dict.set_item("max_workers", stats.max_workers)?;
                dict.set_item("available_permits", stats.available_permits)?;
                Ok(dict.into())
            })
        }
    }

    /// Python wrapper for Config
    #[pyclass]
    pub struct PyConfig {
        config: Config,
    }

    #[pymethods]
    impl PyConfig {
        #[new]
        fn new() -> Self {
            Self {
                config: Config::default(),
            }
        }

        /// Create config from dictionary
        #[classmethod]
        fn from_dict(_cls: &PyType, dict: &PyDict) -> PyResult<Self> {
            let mut builder = ConfigBuilder::new();

            if let Some(workers) = dict.get_item("workers")? {
                builder = builder.with_workers(workers.extract::<usize>()?);
            }

            if let Some(sample_rate) = dict.get_item("sample_rate")? {
                builder = builder.with_sample_rate(sample_rate.extract::<u32>()?);
            }

            if let Some(output_dir) = dict.get_item("output_dir")? {
                let path = PathBuf::from(output_dir.extract::<String>()?);
                builder = builder.with_output_dir(path);
            }

            if let Some(enhancement) = dict.get_item("enable_enhancement")? {
                builder = builder.enable_enhancement(enhancement.extract::<bool>()?);
            }

            if let Some(caching) = dict.get_item("enable_caching")? {
                builder = builder.enable_caching(caching.extract::<bool>()?);
            }

            Ok(Self {
                config: builder.build(),
            })
        }

        /// Set transcription provider
        fn set_transcription_provider(&mut self, provider: String, api_key: Option<String>) -> PyResult<()> {
            self.config.transcription.provider = match provider.as_str() {
                "openai" => TranscriptionProvider::OpenAI,
                "assemblyai" => TranscriptionProvider::AssemblyAI,
                "google" => TranscriptionProvider::GoogleCloud,
                "azure" => TranscriptionProvider::Azure,
                "local" => TranscriptionProvider::Local,
                "external" => TranscriptionProvider::External,
                _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid transcription provider")),
            };
            
            self.config.transcription.api_key = api_key;
            Ok(())
        }

        /// Get configuration summary
        fn summary(&self) -> String {
            self.config.summary()
        }

        /// Save configuration to file
        fn save(&self, path: String) -> PyResult<()> {
            self.config.save(&path)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to save config: {}", e)))
        }

        /// Validate configuration
        fn validate(&self) -> PyResult<()> {
            self.config.validate()
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Config validation failed: {}", e)))
        }
    }

    /// High-level processing functions
    #[pyfunction]
    fn process_videos_rust(
        video_dir: String,
        output_dir: Option<String>,
        workers: Option<usize>,
        sample_rate: Option<u32>,
        transcription_provider: Option<String>,
        api_key: Option<String>,
    ) -> PyResult<PyDict> {
        let output_path = output_dir.unwrap_or_else(|| format!("{}/output", video_dir));
        
        let mut config_builder = ConfigBuilder::new()
            .with_workers(workers.unwrap_or(4))
            .with_output_dir(PathBuf::from(&output_path));

        if let Some(rate) = sample_rate {
            config_builder = config_builder.with_sample_rate(rate);
        }

        if let Some(provider) = transcription_provider {
            let provider_enum = match provider.as_str() {
                "openai" => TranscriptionProvider::OpenAI,
                "local" => TranscriptionProvider::Local,
                _ => TranscriptionProvider::Local,
            };
            config_builder = config_builder.with_transcription_provider(provider_enum);
        }

        if let Some(key) = api_key {
            config_builder = config_builder.with_api_key(key);
        }

        let config = config_builder.build();

        let runtime = Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to create runtime: {}", e)))?;

        let result = runtime.block_on(async {
            let processor = BatchProcessor::new(config, workers.unwrap_or(4)).await?;
            processor.process_directory(PathBuf::from(video_dir), PathBuf::from(output_path)).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Processing failed: {}", e)))?;

        Python::with_gil(|py| {
            let dict = PyDict::new(py);
            dict.set_item("total", result.total)?;
            dict.set_item("successful", result.successful)?;
            dict.set_item("failed", result.failed)?;
            dict.set_item("processing_time", result.total_time.as_secs_f64())?;
            Ok(dict.into())
        })
    }

    /// Get video information without processing
    #[pyfunction]
    fn get_video_info_rust(video_path: String) -> PyResult<PyDict> {
        use crate::video::VideoProcessor;

        let runtime = Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to create runtime: {}", e)))?;

        let video_info = runtime.block_on(async {
            let processor = VideoProcessor::new();
            processor.get_video_info(&PathBuf::from(video_path)).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to get video info: {}", e)))?;

        Python::with_gil(|py| {
            let dict = PyDict::new(py);
            dict.set_item("filename", video_info.filename)?;
            dict.set_item("duration", video_info.duration.as_secs_f64())?;
            dict.set_item("width", video_info.width)?;
            dict.set_item("height", video_info.height)?;
            dict.set_item("fps", video_info.fps)?;
            dict.set_item("format", video_info.format)?;
            dict.set_item("file_size", video_info.file_size)?;
            Ok(dict.into())
        })
    }

    /// Extract audio from video
    #[pyfunction]
    fn extract_audio_rust(video_path: String, output_dir: Option<String>) -> PyResult<String> {
        use crate::audio::AudioExtractor;

        let output_path = output_dir.unwrap_or_else(|| "./output".to_string());
        
        let runtime = Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to create runtime: {}", e)))?;

        let audio_info = runtime.block_on(async {
            let extractor = AudioExtractor::new();
            extractor.extract_for_transcription(&PathBuf::from(video_path), &PathBuf::from(output_path)).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Audio extraction failed: {}", e)))?;

        Ok(audio_info.path.to_string_lossy().to_string())
    }

    /// Check system compatibility and requirements
    #[pyfunction]
    fn check_system_requirements() -> PyResult<PyDict> {
        Python::with_gil(|py| {
            let dict = PyDict::new(py);
            
            // Check FFmpeg
            let runtime = Runtime::new().unwrap();
            let ffmpeg_available = runtime.block_on(async {
                tokio::process::Command::new("ffmpeg")
                    .arg("-version")
                    .output()
                    .await
                    .is_ok()
            });
            dict.set_item("ffmpeg_available", ffmpeg_available)?;

            // Check FFprobe
            let ffprobe_available = runtime.block_on(async {
                tokio::process::Command::new("ffprobe")
                    .arg("-version")
                    .output()
                    .await
                    .is_ok()
            });
            dict.set_item("ffprobe_available", ffprobe_available)?;

            // System info
            dict.set_item("cpu_count", num_cpus::get())?;
            dict.set_item("rust_version", env!("CARGO_PKG_VERSION"))?;
            
            Ok(dict.into())
        })
    }

    /// Performance benchmark function
    #[pyfunction]
    fn benchmark_performance(video_path: String, iterations: Option<usize>) -> PyResult<PyDict> {
        use crate::video::VideoProcessor;
        use std::time::Instant;

        let iterations = iterations.unwrap_or(5);
        let runtime = Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to create runtime: {}", e)))?;

        let mut times = Vec::new();
        
        for _ in 0..iterations {
            let start = Instant::now();
            
            let _result = runtime.block_on(async {
                let processor = VideoProcessor::new();
                processor.get_video_info(&PathBuf::from(&video_path)).await
            });
            
            times.push(start.elapsed().as_secs_f64());
        }

        let avg_time = times.iter().sum::<f64>() / times.len() as f64;
        let min_time = times.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_time = times.iter().fold(0.0, |a, &b| a.max(b));

        Python::with_gil(|py| {
            let dict = PyDict::new(py);
            dict.set_item("iterations", iterations)?;
            dict.set_item("avg_time", avg_time)?;
            dict.set_item("min_time", min_time)?;
            dict.set_item("max_time", max_time)?;
            dict.set_item("times", PyList::new(py, &times))?;
            Ok(dict.into())
        })
    }

    /// Module initialization
    #[pymodule]
    fn bjj_analyzer_rust(_py: Python, m: &PyModule) -> PyResult<()> {
        m.add_class::<PyBatchProcessor>()?;
        m.add_class::<PyConfig>()?;
        m.add_function(wrap_pyfunction!(process_videos_rust, m)?)?;
        m.add_function(wrap_pyfunction!(get_video_info_rust, m)?)?;
        m.add_function(wrap_pyfunction!(extract_audio_rust, m)?)?;
        m.add_function(wrap_pyfunction!(check_system_requirements, m)?)?;
        m.add_function(wrap_pyfunction!(benchmark_performance, m)?)?;
        
        // Add version info
        m.add("__version__", env!("CARGO_PKG_VERSION"))?;
        
        Ok(())
    }
}

#[cfg(feature = "python-bindings")]
pub use python_bindings::*;