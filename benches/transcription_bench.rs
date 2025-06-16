use criterion::{black_box, criterion_group, criterion_main, Criterion};
use bjj_analyzer_rust::{BJJDictionary, SRTGenerator, Config};
use std::time::Duration;

fn bench_bjj_dictionary(c: &mut Criterion) {
    c.bench_function("bjj_dictionary_creation", |b| {
        b.iter(|| {
            black_box(BJJDictionary::new())
        })
    });
    
    let dict = BJJDictionary::new();
    c.bench_function("bjj_prompt_generation", |b| {
        b.iter(|| {
            black_box(dict.generate_prompt())
        })
    });
    
    c.bench_function("bjj_term_lookup", |b| {
        b.iter(|| {
            black_box(dict.contains_term("guard"));
            black_box(dict.contains_term("armbar"));
            black_box(dict.contains_term("triangle"));
        })
    });
    
    c.bench_function("bjj_correction_lookup", |b| {
        b.iter(|| {
            black_box(dict.get_correction("coast guard"));
            black_box(dict.get_correction("arm bar"));
            black_box(dict.get_correction("jujitsu"));
        })
    });
}

fn bench_srt_generation(c: &mut Criterion) {
    c.bench_function("srt_small_file", |b| {
        b.iter(|| {
            let mut generator = black_box(SRTGenerator::new());
            
            // Generate 10 entries
            for i in 0..10 {
                let entry = bjj_analyzer_rust::transcription::srt::SRTEntry::new(
                    i + 1,
                    Duration::from_secs(i * 5),
                    Duration::from_secs((i + 1) * 5),
                    format!("BJJ technique {} with guard and submission", i + 1),
                );
                generator.add_entry(entry);
            }
            
            generator.sort_entries();
            black_box(generator.generate())
        })
    });
    
    c.bench_function("srt_large_file", |b| {
        b.iter(|| {
            let mut generator = black_box(SRTGenerator::new());
            
            // Generate 1000 entries (typical for long BJJ instructional)
            for i in 0..1000 {
                let entry = bjj_analyzer_rust::transcription::srt::SRTEntry::new(
                    i + 1,
                    Duration::from_millis(i * 3000), // 3 second segments
                    Duration::from_millis((i + 1) * 3000),
                    format!("BJJ instruction segment {}: working on guard, mount, side control, submission techniques", i + 1),
                );
                generator.add_entry(entry);
            }
            
            generator.sort_entries();
            black_box(generator.generate())
        })
    });
}

fn bench_config_operations(c: &mut Criterion) {
    c.bench_function("config_creation", |b| {
        b.iter(|| {
            black_box(Config::default())
        })
    });
    
    c.bench_function("config_validation", |b| {
        let config = Config::default();
        b.iter(|| {
            black_box(config.validate())
        })
    });
}

fn bench_bjj_terms_parsing(c: &mut Criterion) {
    let bjj_terms_content = r#"
[Positions]
Guard
Closed Guard
Half Guard
Mount
Side Control

[Submissions]  
Armbar
Triangle
Kimura
Americana

[Corrections]
coast guard -> closed guard
arm bar -> armbar
jujitsu -> jiu-jitsu
"#;

    c.bench_function("bjj_terms_file_parsing", |b| {
        b.iter(|| {
            let mut dict = BJJDictionary::new();
            // Simulate file parsing (we can't easily test the actual async file parsing in criterion)
            for line in bjj_terms_content.lines() {
                let line = line.trim();
                if !line.is_empty() && !line.starts_with('#') && !line.starts_with('[') {
                    if line.contains(" -> ") {
                        let parts: Vec<&str> = line.split(" -> ").collect();
                        if parts.len() == 2 {
                            dict.add_correction(parts[0].to_string(), parts[1].to_string());
                        }
                    }
                }
            }
            black_box(dict)
        })
    });
}

criterion_group!(
    benches,
    bench_bjj_dictionary,
    bench_srt_generation,
    bench_config_operations,
    bench_bjj_terms_parsing
);
criterion_main!(benches);