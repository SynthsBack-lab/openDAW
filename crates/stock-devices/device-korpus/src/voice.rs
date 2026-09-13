//! One Korpus 2 voice behind the shared `voicing::Voice` contract: an exciter (strike, breath,
//! bow or pick) against driven modal banks (single, parallel with eigensplit coupling, or serial
//! A→B), the bowed modal bank, or the dual-polarization plucked string. Stereo throughout.

use abi::EventRecord;
use libm::powf;
use voicing::Voice;

use crate::engine::bow::BowState;
use crate::engine::driven::{build_specs, eigensplit, DrivenBank, Injection, SILENT_SPEC};
use crate::engine::exciter::{Breath, Strike};
use crate::engine::pluck::{PluckState, BODY_LEN};
use crate::engine::tables::{Material, MAX_MODES};

pub const CHUNK_MAX: usize = 128;
const TAIL_SILENCE_BLOCKS: u32 = 16;
const TAIL_THRESHOLD: f32 = 1.0e-4;
const OBJECT_B_OFF: i32 = 6;
const BREATH_DRIVE: f32 = 100.0; // unity-peak modes pass only their sliver of the noise band
const BREATH_BLEED: f32 = 0.004; // chiff + air survive dark objects via a little direct bleed

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exciter {
    Strike,
    Breath,
    Bow,
    Pick,
}

impl Exciter {
    pub fn from_index(index: i32) -> Self {
        match index {
            1 => Self::Breath,
            2 => Self::Bow,
            3 => Self::Pick,
            _ => Self::Strike,
        }
    }
}

pub struct KorpusShared {
    pub exciter: Exciter,
    pub intensity: f32,
    pub position: f32,
    pub vibrato: f32,
    pub object_a: i32,
    pub damping_a: f32,
    pub tune_a: i32,
    pub width_a: f32,
    pub object_b: i32,
    pub damping_b: f32,
    pub tune_b: i32,
    pub detune_b: f32,
    pub width_b: f32,
    pub level_b: f32,
    pub routing: i32,
    pub couple: f32,
    pub sample_rate: f32,
    pub body: [f32; BODY_LEN],
}

impl KorpusShared {
    pub const fn silent() -> Self {
        Self {exciter: Exciter::Strike, intensity: 0.5, position: 0.35, vibrato: 0.0,
            object_a: 0, damping_a: 0.5, tune_a: 0, width_a: 0.6, object_b: OBJECT_B_OFF,
            damping_b: 0.5, tune_b: 0, detune_b: 0.0, width_b: 0.8, level_b: 0.5, routing: 0,
            couple: 0.0, sample_rate: 48_000.0, body: [0.0; BODY_LEN]}
    }
}

struct DrivenVoice {
    strike: Strike,
    breath: Breath,
    breathing: bool,
    bank_a: DrivenBank,
    bank_b: DrivenBank,
    has_b: bool,
    serial: bool,
    gain_a: f32,
    gain_b: f32,
}

impl DrivenVoice {
    const fn silent() -> Self {
        Self {strike: Strike::silent(), breath: Breath::silent(), breathing: false,
            bank_a: DrivenBank::silent(), bank_b: DrivenBank::silent(), has_b: false,
            serial: false, gain_a: 0.35, gain_b: 0.35}
    }
}

enum EngineState {
    Idle,
    Driven(DrivenVoice),
    Bow(BowState),
    Pluck(PluckState),
}

pub struct KorpusVoice {
    state: EngineState,
    frequency: f32,
    gate: bool,
    tail_blocks: u32,
    active: bool,
}

impl Default for KorpusVoice {
    fn default() -> Self {
        Self {state: EngineState::Idle, frequency: 220.0, gate: false, tail_blocks: 0, active: false}
    }
}

fn semitones(steps: i32) -> f32 {
    powf(2.0, steps as f32 / 12.0)
}

impl Voice for KorpusVoice {
    type Shared = KorpusShared;

    fn start(&mut self, event: &EventRecord, frequency: f32, _gain: f32, _spread: f32,
             _unison: usize, shared: &Self::Shared) {
        self.frequency = frequency;
        let velocity = (0.15 + 0.85 * event.velocity).clamp(0.0, 1.0);
        let f0_a = frequency * semitones(shared.tune_a);
        match shared.exciter {
            Exciter::Pick => {
                if !matches!(self.state, EngineState::Pluck(_)) {
                    self.state = EngineState::Pluck(PluckState::silent());
                }
                let EngineState::Pluck(pluck) = &mut self.state else {unreachable!()};
                let stiffness = if Material::from_index(shared.object_a) == Material::PianoWire
                    {0.7} else {0.08};
                pluck.pluck(f0_a, velocity, stiffness, shared.intensity, shared.damping_a,
                    shared.position, shared.sample_rate);
            }
            Exciter::Bow => {
                if !matches!(self.state, EngineState::Bow(_)) {
                    self.state = EngineState::Bow(BowState::silent());
                }
                let EngineState::Bow(bow) = &mut self.state else {unreachable!()};
                bow.start(Material::from_index(shared.object_a), f0_a, velocity,
                    shared.intensity, shared.position, shared.damping_a, shared.width_a,
                    shared.vibrato, shared.sample_rate);
            }
            Exciter::Strike | Exciter::Breath => {
                if !matches!(self.state, EngineState::Driven(_)) {
                    self.state = EngineState::Driven(DrivenVoice::silent());
                }
                let EngineState::Driven(voice) = &mut self.state else {unreachable!()};
                let breathing = shared.exciter == Exciter::Breath;
                let serial = shared.routing == 1;
                let has_b = shared.object_b < OBJECT_B_OFF;
                // Spec arrays are small (64 × 20B) — safe stack temps even on the wasm side.
                let mut specs_a = [SILENT_SPEC; MAX_MODES];
                let count_a = build_specs(Material::from_index(shared.object_a), f0_a,
                    shared.damping_a, shared.position, shared.width_a, shared.sample_rate,
                    &mut specs_a);
                if has_b {
                    let f0_b = frequency * semitones(shared.tune_b)
                        * powf(2.0, shared.detune_b / 1200.0);
                    let mut specs_b = [SILENT_SPEC; MAX_MODES];
                    let count_b = build_specs(Material::from_index(shared.object_b), f0_b,
                        shared.damping_b, shared.position, shared.width_b, shared.sample_rate,
                        &mut specs_b);
                    // The B send is part of the physical drive vector: fold it in BEFORE the
                    // coupling rotation (post-rotation scaling buries the lower doublet member).
                    if !serial {
                        for spec in specs_b[..count_b].iter_mut() {
                            spec.gain_in *= 0.4 + 0.6 * shared.level_b;
                        }
                        if shared.couple > 0.0 {
                            let k = shared.couple * shared.couple * 8.0;
                            eigensplit(&mut specs_a, count_a, &mut specs_b, count_b, k, 25.0);
                        }
                    }
                    let kind_b = if serial {Injection::Tonal}
                        else if breathing {Injection::Noise} else {Injection::Strike};
                    voice.bank_b.build(&specs_b, count_b, kind_b, shared.sample_rate);
                    voice.gain_b = if serial {18.0 * shared.level_b}
                        else if breathing {0.5} else {0.38};
                }
                let kind_a = if breathing {Injection::Noise} else {Injection::Strike};
                voice.bank_a.build(&specs_a, count_a, kind_a, shared.sample_rate);
                voice.has_b = has_b;
                voice.serial = serial;
                voice.breathing = breathing;
                // Path-aware make-up: a lone struck object carries the whole level; pairs sum.
                voice.gain_a = if breathing {0.55}
                    else if !has_b {0.62}
                    else if serial {0.5} else {0.38};
                if breathing {
                    voice.breath.blow(velocity, shared.intensity, shared.damping_a,
                        shared.sample_rate);
                } else {
                    voice.strike.strike(velocity, shared.intensity, shared.sample_rate);
                }
            }
        }
        self.gate = true;
        self.tail_blocks = 0;
        self.active = true;
    }

    fn stop(&mut self) {
        self.gate = false;
        match &mut self.state {
            EngineState::Pluck(pluck) => pluck.release(),
            EngineState::Bow(bow) => bow.release(),
            EngineState::Driven(voice) => voice.breath.release(),
            EngineState::Idle => {}
        }
    }

    fn force_stop(&mut self) {
        self.gate = false;
        self.active = false;
    }

    fn start_glide(&mut self, target_frequency: f32, _glide_duration: f64) {
        self.frequency = target_frequency;
    }

    fn gate(&self) -> bool {
        self.gate
    }

    fn current_frequency(&self) -> f32 {
        self.frequency
    }

    fn process(&mut self, output: [&mut [f32]; 2], _block: &abi::Block, shared: &Self::Shared) -> bool {
        if !self.active {
            return true;
        }
        let [out_left, out_right] = output;
        let len = out_left.len().min(CHUNK_MAX);
        let peak = match &mut self.state {
            EngineState::Idle => return true,
            EngineState::Pluck(pluck) =>
                pluck.render(&shared.body, &mut out_left[..len], &mut out_right[..len]),
            EngineState::Bow(bow) => bow.render(&mut out_left[..len], &mut out_right[..len]),
            EngineState::Driven(voice) => {
                let mut excitation = [0.0f32; CHUNK_MAX];
                if voice.breathing {
                    for sample in excitation[..len].iter_mut() {
                        *sample = voice.breath.tick() * BREATH_DRIVE;
                    }
                } else {
                    for sample in excitation[..len].iter_mut() {
                        *sample = voice.strike.tick();
                    }
                }
                let mut mono = [0.0f32; CHUNK_MAX];
                let mut peak = voice.bank_a.render(&excitation[..len], &mut out_left[..len],
                    &mut out_right[..len], &mut mono[..len], voice.gain_a);
                if voice.has_b {
                    let mut mono_b = [0.0f32; CHUNK_MAX];
                    let drive: &[f32] = if voice.serial {&mono[..len]} else {&excitation[..len]};
                    peak = peak.max(voice.bank_b.render(drive, &mut out_left[..len],
                        &mut out_right[..len], &mut mono_b[..len], voice.gain_b));
                }
                if voice.breathing {
                    for index in 0..len {
                        let bleed = excitation[index] * BREATH_BLEED;
                        out_left[index] += bleed;
                        out_right[index] += bleed;
                        peak = peak.max(libm::fabsf(bleed));
                    }
                    peak = peak.max(voice.breath.level() * 1.0e-3);
                }
                peak
            }
        };
        // A held note is never done — slow bow/breath attacks are quiet for their first
        // ~50ms and must not be freed mid-swell. Tail detection only runs after release.
        if peak < TAIL_THRESHOLD && !self.gate {
            self.tail_blocks += 1;
            if self.tail_blocks >= TAIL_SILENCE_BLOCKS {
                self.active = false;
                return true;
            }
        } else {
            self.tail_blocks = 0;
        }
        false
    }
}
