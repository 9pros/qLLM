use std::fmt;

use serde::{Deserialize, Serialize};

pub const REGISTER_BYTES: usize = 32;
pub const REGISTER_BITS: usize = REGISTER_BYTES * 8;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Register([u8; REGISTER_BYTES]);

impl Register {
    pub const fn zero() -> Self {
        Self([0; REGISTER_BYTES])
    }

    pub const fn all_ones() -> Self {
        Self([0xff; REGISTER_BYTES])
    }

    pub const fn from_bytes(bytes: [u8; REGISTER_BYTES]) -> Self {
        Self(bytes)
    }

    pub fn deterministic(seed: &[u8]) -> Self {
        Self(*blake3::hash(seed).as_bytes())
    }

    pub const fn as_bytes(&self) -> &[u8; REGISTER_BYTES] {
        &self.0
    }

    pub fn xor(&self, other: &Self) -> Self {
        let mut out = [0u8; REGISTER_BYTES];
        xor_bytes_32(&self.0, &other.0, &mut out);
        Self(out)
    }

    pub fn and(&self, other: &Self) -> Self {
        let mut out = [0u8; REGISTER_BYTES];
        for ((dst, left), right) in out.iter_mut().zip(self.0).zip(other.0) {
            *dst = left & right;
        }
        Self(out)
    }

    pub fn rotate_left_bits(&self, shift: u16) -> Self {
        let shift = usize::from(shift) % REGISTER_BITS;
        if shift == 0 {
            return *self;
        }

        let mut out = [0u8; REGISTER_BYTES];
        for dst_bit in 0..REGISTER_BITS {
            let src_bit = (dst_bit + REGISTER_BITS - shift) % REGISTER_BITS;
            if bit_at(&self.0, src_bit) {
                set_bit(&mut out, dst_bit);
            }
        }
        Self(out)
    }
}

impl fmt::Debug for Register {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Register(0x")?;
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        write!(formatter, ")")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DualState {
    pub live: Register,
    pub twin: Register,
}

impl DualState {
    pub const fn zero() -> Self {
        Self {
            live: Register::zero(),
            twin: Register::zero(),
        }
    }

    pub const fn new(live: Register, twin: Register) -> Self {
        Self { live, twin }
    }

    pub fn from_live(live: Register, envelope: PhaseEnvelope) -> Self {
        Self {
            live,
            twin: phase_project(live, envelope),
        }
    }

    pub fn deterministic(seed: &[u8], envelope: PhaseEnvelope) -> Self {
        Self::from_live(Register::deterministic(seed), envelope)
    }

    pub fn xor(&self, other: &Self) -> Self {
        Self {
            live: self.live.xor(&other.live),
            twin: self.twin.xor(&other.twin),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhaseEnvelope {
    pub coherence_sig: u64,
    pub alpha: f32,
    pub beta: f32,
    pub resuperposition_n: u32,
}

impl PhaseEnvelope {
    pub fn new(
        coherence_sig: u64,
        alpha: f32,
        beta: f32,
        resuperposition_n: u32,
    ) -> Result<Self, CoreError> {
        let envelope = Self {
            coherence_sig,
            alpha,
            beta,
            resuperposition_n,
        };
        envelope.validate()?;
        Ok(envelope)
    }

    pub const fn unchecked(
        coherence_sig: u64,
        alpha: f32,
        beta: f32,
        resuperposition_n: u32,
    ) -> Self {
        Self {
            coherence_sig,
            alpha,
            beta,
            resuperposition_n,
        }
    }

    pub fn balanced(coherence_sig: u64) -> Self {
        let value = std::f32::consts::FRAC_1_SQRT_2;
        Self::unchecked(coherence_sig, value, value, 0)
    }

    pub fn live_trusted(coherence_sig: u64) -> Self {
        Self::unchecked(coherence_sig, 1.0, 0.0, 0)
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        if !self.alpha.is_finite() || !self.beta.is_finite() {
            return Err(CoreError::InvalidAmplitude);
        }
        let norm = self.alpha.mul_add(self.alpha, self.beta * self.beta);
        if (norm - 1.0).abs() > 1.0e-4 {
            return Err(CoreError::AmplitudeNotNormalized { norm });
        }
        Ok(())
    }

    pub fn compatible_with(&self, other: &Self, delta: u32) -> bool {
        self.coherence_sig == other.coherence_sig
            && self.resuperposition_n.abs_diff(other.resuperposition_n) <= delta
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegisterLane {
    Ipv4,
    Ipv6,
    PublicApi,
    PublicProxy,
    GenericEndpoint,
}

impl RegisterLane {
    pub const fn tag(self) -> &'static [u8] {
        match self {
            Self::Ipv4 => b"ipv4",
            Self::Ipv6 => b"ipv6",
            Self::PublicApi => b"public-api",
            Self::PublicProxy => b"public-proxy",
            Self::GenericEndpoint => b"generic-endpoint",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterfaceKind {
    Physical,
    Aggregate,
    Bridge,
    Tunnel,
    Vm,
    Application,
}

impl InterfaceKind {
    pub const fn tag(self) -> &'static [u8] {
        match self {
            Self::Physical => b"physical",
            Self::Aggregate => b"aggregate",
            Self::Bridge => b"bridge",
            Self::Tunnel => b"tunnel",
            Self::Vm => b"vm",
            Self::Application => b"application",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RegisterId([u8; REGISTER_BYTES]);

impl RegisterId {
    pub const fn as_bytes(&self) -> &[u8; REGISTER_BYTES] {
        &self.0
    }
}

pub fn derive_register_id(
    lane: RegisterLane,
    slot_index: u64,
    canonical_address: &[u8],
    interface_kind: InterfaceKind,
    node_public_key: &[u8; REGISTER_BYTES],
) -> RegisterId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(lane.tag());
    hasher.update(&slot_index.to_le_bytes());
    hasher.update(canonical_address);
    hasher.update(interface_kind.tag());
    hasher.update(node_public_key);
    RegisterId(*hasher.finalize().as_bytes())
}

pub fn phase_project(live: Register, envelope: PhaseEnvelope) -> Register {
    let rotation = phase_rotation(envelope);
    let mask = phase_mask(envelope);
    live.rotate_left_bits(rotation).xor(&mask)
}

pub fn phase_rotation(envelope: PhaseEnvelope) -> u16 {
    let sig_mix = envelope
        .coherence_sig
        .rotate_left(envelope.resuperposition_n % 64);
    ((sig_mix ^ u64::from(envelope.resuperposition_n)) % REGISTER_BITS as u64) as u16
}

pub fn phase_mask(envelope: PhaseEnvelope) -> Register {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&envelope.coherence_sig.to_le_bytes());
    hasher.update(&envelope.alpha.to_bits().to_le_bytes());
    hasher.update(&envelope.beta.to_bits().to_le_bytes());
    hasher.update(&envelope.resuperposition_n.to_le_bytes());
    Register::from_bytes(*hasher.finalize().as_bytes())
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum CoreError {
    #[error("phase amplitudes must be finite")]
    InvalidAmplitude,
    #[error("phase amplitudes must satisfy alpha^2 + beta^2 = 1, got {norm}")]
    AmplitudeNotNormalized { norm: f32 },
}

#[inline]
fn xor_bytes_32(
    left: &[u8; REGISTER_BYTES],
    right: &[u8; REGISTER_BYTES],
    out: &mut [u8; REGISTER_BYTES],
) {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        xor_bytes_32_neon(left, right, out);
    }

    #[cfg(not(target_arch = "aarch64"))]
    {
        for ((dst, left), right) in out.iter_mut().zip(left).zip(right) {
            *dst = *left ^ *right;
        }
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn xor_bytes_32_neon(
    left: &[u8; REGISTER_BYTES],
    right: &[u8; REGISTER_BYTES],
    out: &mut [u8; REGISTER_BYTES],
) {
    use std::arch::aarch64::{uint8x16_t, veorq_u8, vld1q_u8, vst1q_u8};

    let left_low: uint8x16_t = vld1q_u8(left.as_ptr());
    let right_low: uint8x16_t = vld1q_u8(right.as_ptr());
    vst1q_u8(out.as_mut_ptr(), veorq_u8(left_low, right_low));

    let left_high: uint8x16_t = vld1q_u8(left.as_ptr().add(16));
    let right_high: uint8x16_t = vld1q_u8(right.as_ptr().add(16));
    vst1q_u8(out.as_mut_ptr().add(16), veorq_u8(left_high, right_high));
}

fn bit_at(bytes: &[u8; REGISTER_BYTES], bit_index: usize) -> bool {
    let byte_index = bit_index / 8;
    let bit_in_byte = 7 - (bit_index % 8);
    bytes[byte_index] & (1 << bit_in_byte) != 0
}

fn set_bit(bytes: &mut [u8; REGISTER_BYTES], bit_index: usize) {
    let byte_index = bit_index / 8;
    let bit_in_byte = 7 - (bit_index % 8);
    bytes[byte_index] |= 1 << bit_in_byte;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dual_state_group_axioms_hold() {
        let env = PhaseEnvelope::balanced(0xfeed_beef);
        let a = DualState::deterministic(b"a", env);
        let b = DualState::deterministic(b"b", env);
        let c = DualState::deterministic(b"c", env);
        let zero = DualState::zero();

        assert_eq!(a.xor(&a), zero);
        assert_eq!(a.xor(&b), b.xor(&a));
        assert_eq!(a.xor(&b).xor(&c), a.xor(&b.xor(&c)));
        assert_eq!(a.xor(&zero), a);
    }

    #[test]
    fn k_flow_decodability_holds() {
        let env = PhaseEnvelope::balanced(0x1234_5678);
        for k in 2..=32 {
            let flows = (0..k)
                .map(|index| DualState::deterministic(&(index as u64).to_le_bytes(), env))
                .collect::<Vec<_>>();

            let combined = flows
                .iter()
                .fold(DualState::zero(), |acc, flow| acc.xor(flow));

            for target in 0..k {
                let decoded = flows
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != target)
                    .fold(combined, |acc, (_, twin)| acc.xor(twin));
                assert_eq!(decoded, flows[target]);
            }
        }
    }

    #[test]
    fn phase_project_is_deterministic_and_envelope_sensitive() {
        let live = Register::deterministic(b"live");
        let env_a = PhaseEnvelope::balanced(42);
        let env_b = PhaseEnvelope::unchecked(42, 1.0, 0.0, 1);

        assert_eq!(phase_project(live, env_a), phase_project(live, env_a));
        assert_ne!(phase_project(live, env_a), phase_project(live, env_b));
    }

    #[test]
    fn register_id_uses_all_lane_components() {
        let key = [7u8; REGISTER_BYTES];
        let id_a = derive_register_id(
            RegisterLane::GenericEndpoint,
            4,
            b"token:17",
            InterfaceKind::Application,
            &key,
        );
        let id_b = derive_register_id(
            RegisterLane::PublicApi,
            4,
            b"token:17",
            InterfaceKind::Application,
            &key,
        );

        assert_ne!(id_a, id_b);
    }

    #[test]
    fn functional_completeness_primitives_hold() {
        let x = Register::deterministic(b"x");
        let y = Register::deterministic(b"y");
        let all_ones = Register::all_ones();

        let not_x = x.xor(&all_ones);
        assert_eq!(not_x.and(&x), Register::zero());
        assert_eq!(not_x.xor(&all_ones), x);

        let nand_xy = x.and(&y).xor(&all_ones);
        let nand_yx = y.and(&x).xor(&all_ones);
        assert_eq!(nand_xy, nand_yx);
    }
}
