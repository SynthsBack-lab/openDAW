//! Plucked string: dual-polarization waveguide (detuned pair mixed L/R), stiffness dispersion via
//! an allpass cascade (nylon -> piano on one knob), pick-position combing, pick-color filtering,
//! and a commuted synthetic body — the body impulse response is read through the pick filter as
//! the excitation (JOS commuted synthesis), so every note rings through a body at no runtime cost.
//! Zero-allocation: fixed delay lines, re-initialised in place at note-on; the body table is built
//! once per device.

use libm::{cosf, fabsf, powf};

const PI: f32 = core::f32::consts::PI;
const MAX_DELAY: usize = 4096;
pub const BODY_LEN: usize = 2400;

const BODY_MODES: [(f32, f32, f32); 8] = [(96.0, 0.22, 1.0), (189.0, 0.16, 0.8), (247.0, 0.14, 0.7),
    (405.0, 0.10, 0.55), (532.0, 0.08, 0.4), (748.0, 0.06, 0.33), (1120.0, 0.05, 0.25),
    (1630.0, 0.04, 0.18)];

/// Builds the synthetic body impulse response in place (called once from device `init`).
pub fn build_body(table: &mut [f32; BODY_LEN], sample_rate: f32) {
    table.fill(0.0);
    for (frequency, t60, amp) in BODY_MODES {
        let omega = 2.0 * PI * frequency / sample_rate;
        let r = powf(10.0, -3.0 / (t60 * sample_rate));
        let (mut y1, mut y2) = (0.0f32, 0.0f32);
        for (index, slot) in table.iter_mut().enumerate() {
            let input = if index == 0 {1.0} else {0.0};
            let y = 2.0 * r * cosf(omega) * y1 - r * r * y2 + input;
            y2 = y1;
            y1 = y;
            *slot += y * amp;
        }
    }
    let peak = table.iter().fold(0.0f32, |acc, sample| acc.max(fabsf(*sample))).max(1.0e-9);
    for slot in table.iter_mut() {
        *slot /= peak;
    }
}

struct Polarization {
    line: [f32; MAX_DELAY],
    write: usize,
    length: f32,
    damp_state: f32,
    damp_coefficient: f32,
    loop_gain: f32,
    ap_states: [f32; 4],
    ap_coefficient: f32,
}

impl Polarization {
    const fn silent() -> Self {
        Self {line: [0.0; MAX_DELAY], write: 0, length: 100.0, damp_state: 0.0,
            damp_coefficient: 0.5, loop_gain: 0.0, ap_states: [0.0; 4], ap_coefficient: 0.0}
    }

    fn init(&mut self, frequency: f32, sample_rate: f32, stiffness: f32, brightness: f32, t60: f32) {
        self.line.fill(0.0);
        self.write = 0;
        self.damp_state = 0.0;
        self.ap_states = [0.0; 4];
        self.ap_coefficient = -0.55 * stiffness;
        let ap_delay = 4.0 * (1.0 - self.ap_coefficient) / (1.0 + self.ap_coefficient) * 0.5;
        self.length = (sample_rate / frequency - 0.5 - ap_delay).clamp(2.0, (MAX_DELAY - 4) as f32);
        self.loop_gain = powf(10.0, -3.0 / (frequency * t60));
        self.damp_coefficient = 0.12 + 0.62 * (1.0 - brightness);
    }

    #[inline]
    fn read(&self, delay: f32) -> f32 {
        let read_pos = self.write as f32 - delay + MAX_DELAY as f32;
        let index = read_pos as usize;
        let frac = read_pos - index as f32;
        let a = self.line[index % MAX_DELAY];
        let b = self.line[(index + 1) % MAX_DELAY];
        a + (b - a) * frac
    }

    #[inline]
    fn tick(&mut self, input: f32, release_damp: f32) -> f32 {
        let raw = self.read(self.length);
        self.damp_state += self.damp_coefficient * (raw - self.damp_state);
        let mut signal = self.damp_state * self.loop_gain * release_damp;
        for state in &mut self.ap_states {
            let output = self.ap_coefficient * signal + *state;
            *state = signal - self.ap_coefficient * output;
            signal = output;
        }
        self.line[self.write % MAX_DELAY] = signal + input;
        self.write = (self.write + 1) % MAX_DELAY;
        signal
    }
}

pub struct PluckState {
    vertical: Polarization,
    horizontal: Polarization,
    excitation_pos: usize,
    excitation_gain: f32,
    pick_lp: f32,
    pick_state: f32,
    position_delay: f32,
    released: bool,
    dc_l: (f32, f32),
    dc_r: (f32, f32),
}

impl PluckState {
    pub const fn silent() -> Self {
        Self {vertical: Polarization::silent(), horizontal: Polarization::silent(),
            excitation_pos: BODY_LEN, excitation_gain: 0.0, pick_lp: 0.5, pick_state: 0.0,
            position_delay: 20.0, released: false, dc_l: (0.0, 0.0), dc_r: (0.0, 0.0)}
    }

    #[allow(clippy::too_many_arguments)]
    pub fn pluck(&mut self, frequency: f32, velocity: f32, stiffness: f32, brightness: f32,
                 damping: f32, position: f32, sample_rate: f32) {
        let t60 = 0.4 + 7.0 * damping * damping;
        let detune = 1.0 + 0.0009 * 0.75;
        self.vertical.init(frequency, sample_rate, stiffness, brightness, t60);
        self.horizontal.init(frequency * detune, sample_rate, stiffness, brightness, t60 * 0.72);
        self.excitation_pos = 0;
        // The body resonances boost low fundamentals; trim below 220Hz to keep the keyboard even.
        self.excitation_gain = velocity * 0.85 * powf((frequency * (1.0 / 220.0)).min(1.0), 0.9);
        self.pick_lp = 0.08 + 0.85 * brightness * (0.5 + 0.5 * velocity);
        self.pick_state = 0.0;
        self.position_delay = (0.04 + 0.42 * position) * self.vertical.length;
        self.released = false;
        self.dc_l = (0.0, 0.0);
        self.dc_r = (0.0, 0.0);
    }

    pub fn release(&mut self) {
        self.released = true;
    }

    #[inline]
    fn dc_block(state: &mut (f32, f32), input: f32) -> f32 {
        let output = input - state.0 + 0.995 * state.1;
        state.0 = input;
        state.1 = output;
        output
    }

    #[inline]
    fn excitation_at(&self, body: &[f32; BODY_LEN], index: f32) -> f32 {
        if index < 0.0 {return 0.0}
        let position = index as usize;
        if position >= BODY_LEN {return 0.0}
        let fade = if position + 240 > BODY_LEN {(BODY_LEN - position) as f32 / 240.0} else {1.0};
        body[position] * fade
    }

    /// Renders additively; returns the chunk peak so the caller can free silent voices.
    pub fn render(&mut self, body: &[f32; BODY_LEN], out_l: &mut [f32], out_r: &mut [f32]) -> f32 {
        let release_damp = if self.released {0.72} else {1.0};
        let mut peak = 0.0f32;
        for index in 0..out_l.len() {
            let mut input = 0.0f32;
            if self.excitation_pos < BODY_LEN {
                let raw = self.excitation_at(body, self.excitation_pos as f32);
                self.pick_state += self.pick_lp * (raw - self.pick_state);
                let delayed = self.excitation_at(body, self.excitation_pos as f32 - self.position_delay);
                input = (self.pick_state - 0.85 * delayed * self.pick_lp) * self.excitation_gain;
                self.excitation_pos += 1;
            }
            let v = self.vertical.tick(input, release_damp);
            let h = self.horizontal.tick(input * 0.6, release_damp);
            let left = Self::dc_block(&mut self.dc_l, v * 0.72 + h * 0.28) * 1.85;
            let right = Self::dc_block(&mut self.dc_r, v * 0.28 + h * 0.72) * 1.85;
            out_l[index] += left;
            out_r[index] += right;
            peak = peak.max(fabsf(left)).max(fabsf(right));
        }
        peak
    }
}
