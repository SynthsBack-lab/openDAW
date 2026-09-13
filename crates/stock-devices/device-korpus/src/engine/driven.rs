//! Driven modal bank: exactly-unity-peak resonators (constant-skirt bandpass with G = 1/decay —
//! peak gain G·decay = 1 identically at the design frequency, DC and Nyquist zeros for free) fed
//! by any mono excitation. Injection-side hit comb, per-mode alternating pan, per-mode slow gain
//! shimmer (decorrelated — lockstep partials read as an organ), √T60-neutral sustained injection,
//! and impulse-normalized injection for strikes. Eigensplit couples two banks at note-on.

use libm::{cosf, fabsf, powf, sinf, sqrtf};

use crate::engine::tables::{spec, Material, MAX_MODES};

const PI: f32 = core::f32::consts::PI;
const LN1000: f32 = 6.9077554;

#[derive(Clone, Copy)]
pub struct DrivenSpec {
    pub frequency: f32,
    pub t60: f32,
    pub gain_in: f32,
    pub gain_out: f32,
    pub pan: f32,
}

pub const SILENT_SPEC: DrivenSpec =
    DrivenSpec {frequency: 0.0, t60: 0.1, gain_in: 0.0, gain_out: 0.0, pan: 0.5};

/// Fills `out` from the material law; returns the mode count.
pub fn build_specs(material: Material, f0: f32, damping: f32, position: f32, width: f32,
                   sample_rate: f32, out: &mut [DrivenSpec; MAX_MODES]) -> usize {
    let t60_scale = 0.12 + 3.4 * damping * damping;
    let mut seed = 0x2545f491u32;
    let mut count = 0;
    let mut index = 0;
    while let Some(mode) = spec(material, index) {
        index += 1;
        let frequency = f0 * mode.ratio;
        if frequency < 20.0 || frequency > sample_rate * 0.45 {
            continue;
        }
        let t60 = (mode.t60 * t60_scale * powf(f0 * 1.6 / frequency, 0.5)).max(0.012);
        let position_gain = fabsf(sinf(index as f32 * PI * (0.06 + 0.88 * position)));
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let rand = (seed >> 8) as f32 / 16777216.0;
        let side = if index % 2 == 1 {1.0} else {-1.0};
        let pan = (0.5 + side * width * (0.12 + 0.38 * rand)).clamp(0.02, 0.98);
        out[count] = DrivenSpec {
            frequency,
            t60,
            gain_in: 0.25 + 0.75 * position_gain,
            gain_out: mode.amp,
            pan,
        };
        count += 1;
        if count == MAX_MODES {
            break;
        }
    }
    count
}

/// Eigensplit coupling (bridge-type bilinear, applied at note-on): pairs within `reach` Hz move
/// to f± = mean ± √(Δ² + k²); gains, pan and decay rotate by θ = ½·atan2(2k, f_hi − f_lo), pairs
/// ordered by frequency so θ → 0 as k → 0 (no slot swap for weak coupling). Decay eigenvalues are
/// the plain weighted blend — no cross term (it belongs to the off-diagonal, not the eigenvalues).
pub fn eigensplit(a: &mut [DrivenSpec], a_count: usize, b: &mut [DrivenSpec], b_count: usize,
                  k: f32, reach: f32) {
    if k <= 0.0 {
        return;
    }
    let mut b_used = [false; MAX_MODES];
    for index_a in 0..a_count {
        let mut best: Option<(usize, f32)> = None;
        for index_b in 0..b_count {
            if b_used[index_b] {
                continue;
            }
            let distance = fabsf(a[index_a].frequency - b[index_b].frequency);
            if distance <= reach && best.map_or(true, |(_, d)| distance < d) {
                best = Some((index_b, distance));
            }
        }
        let Some((index_b, _)) = best else {continue};
        b_used[index_b] = true;
        let a_is_high = a[index_a].frequency >= b[index_b].frequency;
        let (mut high, mut low) = if a_is_high {(a[index_a], b[index_b])} else {(b[index_b], a[index_a])};
        let mean = 0.5 * (high.frequency + low.frequency);
        let half_delta = 0.5 * (high.frequency - low.frequency);
        let split = sqrtf(half_delta * half_delta + k * k);
        let theta = 0.5 * libm::atan2f(2.0 * k, 2.0 * half_delta);
        let (sin_t, cos_t) = (sinf(theta), cosf(theta));
        let (gh_in, gl_in) = (high.gain_in, low.gain_in);
        let (gh_out, gl_out) = (high.gain_out, low.gain_out);
        let (decay_h, decay_l) = (1.0 / high.t60, 1.0 / low.t60);
        high.frequency = mean + split;
        low.frequency = mean - split;
        high.gain_in = cos_t * gh_in + sin_t * gl_in;
        low.gain_in = cos_t * gl_in - sin_t * gh_in;
        high.gain_out = cos_t * gh_out + sin_t * gl_out;
        low.gain_out = cos_t * gl_out - sin_t * gh_out;
        let (weight, cross_weight) = (cos_t * cos_t, sin_t * sin_t);
        let pan_h = high.pan;
        high.pan = weight * pan_h + cross_weight * low.pan;
        low.pan = weight * low.pan + cross_weight * pan_h;
        high.t60 = 1.0 / (weight * decay_h + cross_weight * decay_l).max(1.0e-3);
        low.t60 = 1.0 / (weight * decay_l + cross_weight * decay_h).max(1.0e-3);
        if a_is_high {
            a[index_a] = high;
            b[index_b] = low;
        } else {
            b[index_b] = high;
            a[index_a] = low;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Injection {
    Strike,
    Noise,
    Tonal,
}

#[derive(Clone, Copy)]
struct DrivenMode {
    a1: f32,
    a2: f32,
    inject: f32,
    tap_l: f32,
    tap_r: f32,
    inv_a0: f32,
    shim_state: f32,
    shim: f32,
    y1: f32,
    y2: f32,
}

const SILENT_MODE: DrivenMode = DrivenMode {a1: 0.0, a2: 0.0, inject: 0.0, tap_l: 0.0,
    tap_r: 0.0, inv_a0: 1.0, shim_state: 0.0, shim: 1.0, y1: 0.0, y2: 0.0};

pub struct DrivenBank {
    modes: [DrivenMode; MAX_MODES],
    count: usize,
    noise_state: u32,
    x1: f32,
    x2: f32,
}

impl DrivenBank {
    pub const fn silent() -> Self {
        Self {modes: [SILENT_MODE; MAX_MODES], count: 0, noise_state: 0x51f15eed, x1: 0.0, x2: 0.0}
    }

    /// Injection normalization per drive type: an impulse through a unity-peak mode rings at
    /// ~2ε, so strikes inject 0.5 (= ε·1/2ε) and the ring follows the table amp law. Broadband
    /// sustained drive (breath) injects ε·√T60 for T60-neutral loudness (unity peak passes noise
    /// power ∝ bandwidth ∝ 1/T60). TONAL sustained drive (serial ring-through) injects plain ε —
    /// unity peak IS tonal-neutral, and √T60 would hand long-ring objects +15dB.
    pub fn build(&mut self, specs: &[DrivenSpec], count: usize, injection_kind: Injection,
                 sample_rate: f32) {
        self.count = count;
        self.x1 = 0.0;
        self.x2 = 0.0;
        for index in 0..count {
            let mode_spec = &specs[index];
            let omega = 2.0 * PI * mode_spec.frequency / sample_rate;
            let eps = LN1000 / (mode_spec.t60 * sample_rate);
            let inv_a0 = 1.0 / (1.0 + eps);
            let injection = match injection_kind {
                Injection::Strike => 0.5,
                Injection::Noise => eps * sqrtf(mode_spec.t60),
                Injection::Tonal => eps,
            };
            self.modes[index] = DrivenMode {
                a1: -2.0 * cosf(omega),
                a2: 1.0 - eps,
                inject: injection * mode_spec.gain_in * inv_a0,
                tap_l: mode_spec.gain_out * sqrtf(1.0 - mode_spec.pan),
                tap_r: mode_spec.gain_out * sqrtf(mode_spec.pan),
                inv_a0,
                shim_state: 0.0,
                shim: 1.0,
                y1: 0.0,
                y2: 0.0,
            };
        }
    }

    /// Adds the stereo render into the output slices and writes the mono sum (for serial
    /// routing) into `mono_out`. Returns the chunk peak for voice-freeing.
    pub fn render(&mut self, excitation: &[f32], out_l: &mut [f32], out_r: &mut [f32],
                  mono_out: &mut [f32], gain: f32) -> f32 {
        let mut peak = 0.0f32;
        // Per-mode slow gain wobble (~0.35dB RMS, decorrelated, chunk-rate walk).
        for mode in self.modes[..self.count].iter_mut() {
            self.noise_state = self.noise_state.wrapping_mul(1664525).wrapping_add(1013904223);
            let noise = (self.noise_state >> 8) as f32 / 8388608.0 - 1.0;
            mode.shim_state += 0.02 * (noise - mode.shim_state);
            mode.shim = (1.0 + mode.shim_state * 0.4).clamp(0.6, 1.5);
        }
        for index in 0..excitation.len() {
            let x = excitation[index];
            let drive = x - self.x2;
            self.x2 = self.x1;
            self.x1 = x;
            let mut left = 0.0f32;
            let mut right = 0.0f32;
            let mut mono = 0.0f32;
            for mode in self.modes[..self.count].iter_mut() {
                let y = mode.inject * drive - (mode.a1 * mode.y1 + mode.a2 * mode.y2) * mode.inv_a0;
                mode.y2 = mode.y1;
                mode.y1 = y;
                let shimmed = y * mode.shim;
                left += shimmed * mode.tap_l;
                right += shimmed * mode.tap_r;
                mono += y * (mode.tap_l + mode.tap_r);
            }
            mono_out[index] = mono * gain * 0.5;
            out_l[index] += left * gain;
            out_r[index] += right * gain;
            peak = peak.max(fabsf(left * gain)).max(fabsf(right * gain));
        }
        peak
    }
}
