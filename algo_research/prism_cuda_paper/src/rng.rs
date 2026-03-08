#[derive(Clone, Debug)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        let seed = if seed == 0 { 0x9e3779b97f4a7c15 } else { seed };
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    pub fn next_f32(&mut self) -> f32 {
        let x = (self.next_u64() >> 40) as u32;
        (x as f32) / ((1u32 << 24) as f32)
    }

    pub fn gen_usize(&mut self, upper_exclusive: usize) -> usize {
        if upper_exclusive <= 1 {
            return 0;
        }
        (self.next_u64() as usize) % upper_exclusive
    }

    pub fn gen_range_f32(&mut self, low: f32, high: f32) -> f32 {
        low + self.next_f32() * (high - low)
    }

    pub fn gen_bool(&mut self, p: f32) -> bool {
        self.next_f32() < p
    }

    pub fn standard_normal(&mut self) -> f32 {
        // Box-Muller
        let u1 = self.next_f32().clamp(1e-7, 1.0 - 1e-7);
        let u2 = self.next_f32().clamp(1e-7, 1.0 - 1e-7);
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * core::f32::consts::PI * u2;
        r * theta.cos()
    }

    pub fn shuffle<T>(&mut self, data: &mut [T]) {
        for i in (1..data.len()).rev() {
            let j = self.gen_usize(i + 1);
            data.swap(i, j);
        }
    }
}

pub fn stable_seed(parts: &[&str]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for part in parts {
        for &b in part.as_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h ^= b'|' as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn lhs_sample(n: usize, d: usize, rng: &mut SplitMix64) -> Vec<Vec<f32>> {
    let mut out = vec![vec![0.0; d]; n];
    for j in 0..d {
        let mut perm: Vec<usize> = (0..n).collect();
        rng.shuffle(&mut perm);
        for i in 0..n {
            out[i][j] = (perm[i] as f32 + rng.next_f32()) / (n as f32);
        }
    }
    out
}
