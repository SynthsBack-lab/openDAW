use device_korpus::engine::pluck::build_body;
use device_korpus::voice::{Exciter, KorpusShared, KorpusVoice, CHUNK_MAX};
use voicing::Voice;

const SAMPLE_RATE: f32 = 48_000.0;

// A named patch: the ear-approved preset families, distilled to device parameters.
struct Config {
    name: &'static str,
    exciter: Exciter,
    intensity: f32,
    position: f32,
    vibrato: f32,
    object_a: i32,
    damping_a: f32,
    object_b: i32,
    damping_b: f32,
    tune_b: i32,
    detune_b: f32,
    level_b: f32,
    routing: i32,
    couple: f32,
    sustained: bool,
}

const BASE: Config = Config {name: "", exciter: Exciter::Strike, intensity: 0.5, position: 0.35,
    vibrato: 0.0, object_a: 0, damping_a: 0.5, object_b: 6, damping_b: 0.5, tune_b: 0,
    detune_b: 0.0, level_b: 0.5, routing: 0, couple: 0.0, sustained: false};

fn configs() -> Vec<Config> {
    vec![
        Config {name: "strike_marimba", ..BASE},
        Config {name: "strike_plate_hard", intensity: 0.9, object_a: 4, damping_a: 0.25, ..BASE},
        Config {name: "gamelan_pair", object_a: 0, damping_a: 0.55, object_b: 1, damping_b: 0.75,
            detune_b: 7.0, couple: 0.25, ..BASE},
        Config {name: "bar_into_skin", damping_a: 0.3, object_b: 3, damping_b: 0.45, tune_b: -12,
            routing: 1, level_b: 0.8, ..BASE},
        Config {name: "bell_into_wire", intensity: 0.7, object_a: 2, damping_a: 0.8, object_b: 5,
            damping_b: 0.9, tune_b: 12, routing: 1, level_b: 0.75, ..BASE},
        Config {name: "breath_bar", exciter: Exciter::Breath, intensity: 0.45, object_a: 0,
            damping_a: 0.5, sustained: true, ..BASE},
        Config {name: "breath_vibe_pair", exciter: Exciter::Breath, intensity: 0.6, object_a: 1,
            damping_a: 0.8, object_b: 1, damping_b: 0.68, detune_b: 5.0, couple: 0.12,
            level_b: 0.4, sustained: true, ..BASE},
        Config {name: "bow_vibe", exciter: Exciter::Bow, intensity: 0.3, position: 0.25,
            object_a: 1, damping_a: 0.85, vibrato: 0.15, sustained: true, ..BASE},
        Config {name: "bow_wire", exciter: Exciter::Bow, intensity: 0.55, position: 0.12,
            object_a: 5, damping_a: 0.7, vibrato: 0.25, sustained: true, ..BASE},
        Config {name: "bow_membrane", exciter: Exciter::Bow, intensity: 0.45, position: 0.6,
            object_a: 3, damping_a: 0.6, vibrato: 0.2, sustained: true, ..BASE},
        Config {name: "pick_nylon", exciter: Exciter::Pick, intensity: 0.68, position: 0.18,
            object_a: 0, damping_a: 0.6, ..BASE},
        Config {name: "pick_wire", exciter: Exciter::Pick, intensity: 0.6, position: 0.12,
            object_a: 5, damping_a: 0.8, ..BASE},
    ]
}

fn shared_for(config: &Config) -> KorpusShared {
    let mut shared = KorpusShared {
        exciter: config.exciter,
        intensity: config.intensity,
        position: config.position,
        vibrato: config.vibrato,
        object_a: config.object_a,
        damping_a: config.damping_a,
        object_b: config.object_b,
        damping_b: config.damping_b,
        tune_b: config.tune_b,
        detune_b: config.detune_b,
        level_b: config.level_b,
        routing: config.routing,
        couple: config.couple,
        sample_rate: SAMPLE_RATE,
        ..KorpusShared::silent()
    };
    build_body(&mut shared.body, SAMPLE_RATE);
    shared
}

fn note_on(pitch: u32, velocity: f32) -> abi::EventRecord {
    abi::EventRecord {position: 0.0, offset: 0, kind: abi::EVENT_NOTE_ON, id: 1, pitch,
        velocity, cent: 0.0, duration: 0.0}
}

fn render(config: &Config, pitch: u32, velocity: f32, seconds: f32, held_seconds: f32)
    -> (Vec<f32>, Vec<f32>) {
    let shared = shared_for(config);
    let mut voice = KorpusVoice::default();
    let frequency = 440.0 * libm::powf(2.0, (pitch as f32 - 69.0) / 12.0);
    voice.start(&note_on(pitch, velocity), frequency, 1.0, 0.0, 1, &shared);
    let block = abi::Block {index: 0, flags: abi::BlockFlags(0), bpm: 120.0, p0: 0.0, p1: 0.0,
        s0: 0, s1: 0};
    let total = (seconds * SAMPLE_RATE) as usize;
    let held = (held_seconds * SAMPLE_RATE) as usize;
    let mut output_l = Vec::with_capacity(total);
    let mut output_r = Vec::with_capacity(total);
    let mut done = false;
    let mut position = 0;
    while position < total {
        let len = CHUNK_MAX.min(total - position);
        let mut left = [0.0f32; CHUNK_MAX];
        let mut right = [0.0f32; CHUNK_MAX];
        if position >= held && voice.gate() {
            voice.stop();
        }
        if !done {
            let mut slices: [&mut [f32]; 2] = [&mut left[..len], &mut right[..len]];
            let [l, r] = &mut slices;
            done = voice.process([l, r], &block, &shared);
        }
        output_l.extend_from_slice(&left[..len]);
        output_r.extend_from_slice(&right[..len]);
        position += len;
    }
    (output_l, output_r)
}

fn momentary_rms(left: &[f32], right: &[f32]) -> f32 {
    let window = (0.25 * SAMPLE_RATE) as usize;
    let energy: Vec<f64> = left.iter().zip(right)
        .map(|(l, r)| (l * l + r * r) as f64 * 0.5).collect();
    let mut sum: f64 = energy[..window].iter().sum();
    let mut max_mean = sum;
    for index in window..energy.len() {
        sum += energy[index] - energy[index - window];
        if sum > max_mean {max_mean = sum;}
    }
    (max_mean / window as f64).sqrt() as f32
}

#[test]
fn every_config_renders_finite_stereo_audible_output() {
    for config in configs() {
        let held = if config.sustained {2.6} else {1.2};
        let (left, right) = render(&config, 57, 0.9, 4.2, held);
        let name = config.name;
        let peak = left.iter().chain(right.iter()).fold(0.0f32, |acc, s| acc.max(s.abs()));
        assert!(left.iter().chain(right.iter()).all(|s| s.is_finite()), "{name}: non-finite");
        assert!(peak > 0.05, "{name}: too quiet (peak {peak})");
        assert!(peak < 2.5, "{name}: too hot (peak {peak})");
        let difference = left.iter().zip(&right)
            .fold(0.0f32, |acc, (l, r)| acc.max((l - r).abs()));
        assert!(difference > 1.0e-4, "{name}: output is mono");
        let tail = &left[left.len() - 2400..];
        let tail_peak = tail.iter().fold(0.0f32, |acc, s| acc.max(s.abs()));
        assert!(tail_peak < 0.6, "{name}: tail not decaying (peak {tail_peak})");
        let speaks_at = left.iter().position(|s| s.abs() > 0.02)
            .map(|index| index as f32 / SAMPLE_RATE)
            .unwrap_or(f32::INFINITY);
        let onset_limit = if config.sustained {0.6} else {0.05};
        assert!(speaks_at < onset_limit, "{name}: slow onset ({speaks_at:.3}s)");
        if let Ok(dir) = std::env::var("KORPUS_RENDER_DIR") {
            let spec = hound::WavSpec {channels: 2, sample_rate: SAMPLE_RATE as u32,
                bits_per_sample: 16, sample_format: hound::SampleFormat::Int};
            let mut writer = hound::WavWriter::create(
                format!("{dir}/korpus_v2_{name}.wav"), spec).unwrap();
            // Fixed gain so the dumps preserve relative loudness.
            for index in 0..left.len() {
                writer.write_sample((left[index] * 0.7 * 32767.0) as i16).unwrap();
                writer.write_sample((right[index] * 0.7 * 32767.0) as i16).unwrap();
            }
            writer.finalize().unwrap();
        }
        println!("{name}: peak {peak:.3}, speaks at {speaks_at:.4}s");
    }
}

#[test]
fn configs_are_level_matched() {
    let list = configs();
    let loudness: Vec<f32> = list.iter().map(|config| {
        let held = if config.sustained {1.5} else {1.2};
        let (left, right) = render(config, 57, 0.9, 2.0, held);
        momentary_rms(&left, &right)
    }).collect();
    let mut sorted = loudness.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = sorted[sorted.len() / 2];
    for (config, rms) in list.iter().zip(&loudness) {
        let db = 20.0 * libm::log10f(rms / median);
        println!("{}: {rms:.4} rms ({db:+.2}dB from median)", config.name);
    }
    for (config, rms) in list.iter().zip(&loudness) {
        let db = 20.0 * libm::log10f(rms / median);
        assert!(db.abs() <= 3.0, "{}: {db:+.2}dB from the config median", config.name);
    }
}

#[test]
fn sustained_configs_speak_across_the_range() {
    let list = configs();
    for config in list.iter().filter(|config| config.sustained) {
        let (low, high) = match config.exciter {
            Exciter::Bow if config.object_a == 3 => (45u32, 76u32), // membrane sings from ~45
            Exciter::Bow => (40, 84),
            _ => (48, 84),
        };
        for pitch in (low..=high).step_by(6) {
            for velocity in [0.5f32, 0.9] {
                let (left, right) = render(config, pitch, velocity, 1.6, 1.2);
                let peak = left.iter().chain(right.iter())
                    .fold(0.0f32, |acc, s| acc.max(s.abs()));
                assert!(peak.is_finite() && peak > 0.015 && peak < 3.0,
                    "{} pitch {pitch} vel {velocity}: peak {peak}", config.name);
            }
        }
    }
}

#[test]
fn release_damps_sustained_tails() {
    let config = configs().into_iter().find(|config| config.name == "bow_vibe").unwrap();
    let (held_l, _) = render(&config, 57, 0.9, 3.0, 2.8);
    let (released_l, _) = render(&config, 57, 0.9, 3.0, 0.8);
    let window = (2.2 * SAMPLE_RATE) as usize..(2.8 * SAMPLE_RATE) as usize;
    let held_energy: f32 = held_l[window.clone()].iter().map(|s| s * s).sum();
    let released_energy: f32 = released_l[window].iter().map(|s| s * s).sum();
    assert!(released_energy < held_energy * 0.5,
        "release must damp the bow (held {held_energy:.6}, released {released_energy:.6})");
}
