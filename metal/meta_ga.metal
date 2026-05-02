#include <metal_stdlib>
using namespace metal;

struct PhaseEnvelopeMetal {
    ulong coherence_sig;
    float alpha;
    float beta;
    uint resuperposition_n;
};

struct DualStateGenomeMetal {
    uint4 live_lo;
    uint4 live_hi;
    uint4 twin_lo;
    uint4 twin_hi;
    ulong id;
    uint generation;
    PhaseEnvelopeMetal envelope;
};

struct FitnessTargetMetal {
    uint4 desired_lo;
    uint4 desired_hi;
    uint4 novelty_lo;
    uint4 novelty_hi;
    float correctness_weight;
    float novelty_weight;
    float simplicity_weight;
    float reversibility_weight;
};

struct MetaGaPolicyMetal {
    float mutation_rate;
    float crossover_rate;
    float phase_shift_rate;
    uint population_count;
    uint generation;
};

struct FitnessScoreMetal {
    float total;
    float correctness;
    float novelty;
    float simplicity;
    float reversibility;
};

static inline uint mix32(uint x) {
    x ^= x >> 16;
    x *= 0x7feb352du;
    x ^= x >> 15;
    x *= 0x846ca68bu;
    x ^= x >> 16;
    return x;
}

static inline uint lane_entropy(uint seed, uint gid, uint lane, uint generation) {
    return mix32(seed ^ (gid * 0x9e3779b9u) ^ (lane * 0x85ebca6bu) ^ (generation * 0xc2b2ae35u));
}

static inline uint popcount4(uint4 value) {
    return popcount(value.x) + popcount(value.y) + popcount(value.z) + popcount(value.w);
}

static inline float normalized_hamming(uint4 a_lo, uint4 a_hi, uint4 b_lo, uint4 b_hi) {
    uint distance = popcount4(a_lo ^ b_lo) + popcount4(a_hi ^ b_hi);
    return float(distance) / 256.0f;
}

static inline float bit_density(uint4 lo, uint4 hi) {
    return float(popcount4(lo) + popcount4(hi)) / 256.0f;
}

static inline uint mutation_word(uint base, float mutation_rate) {
    uint threshold = uint(clamp(mutation_rate, 0.0f, 1.0f) * 255.0f);
    uint out = 0u;
    for (uint byte_index = 0u; byte_index < 4u; byte_index++) {
        uint byte_value = (base >> (byte_index * 8u)) & 0xffu;
        for (uint bit = 0u; bit < 8u; bit++) {
            uint lane = ((byte_value << bit) | (byte_value >> (8u - bit))) & 0xffu;
            if (lane <= threshold) {
                out |= 1u << (byte_index * 8u + bit);
            }
        }
    }
    return out;
}

static inline uint4 mutate4(uint4 value, uint seed, uint gid, uint base_lane, uint generation, float mutation_rate) {
    return value ^ uint4(
        mutation_word(lane_entropy(seed, gid, base_lane + 0u, generation), mutation_rate),
        mutation_word(lane_entropy(seed, gid, base_lane + 1u, generation), mutation_rate),
        mutation_word(lane_entropy(seed, gid, base_lane + 2u, generation), mutation_rate),
        mutation_word(lane_entropy(seed, gid, base_lane + 3u, generation), mutation_rate)
    );
}

static inline uint4 select4(uint4 left, uint4 right, uint4 mask) {
    return (left & mask) ^ (right & ~mask);
}

kernel void meta_ga_evolve(
    device const DualStateGenomeMetal* parents [[buffer(0)]],
    device DualStateGenomeMetal* children [[buffer(1)]],
    device FitnessScoreMetal* scores [[buffer(2)]],
    constant FitnessTargetMetal& target [[buffer(3)]],
    constant MetaGaPolicyMetal& policy [[buffer(4)]],
    constant uint& entropy_seed [[buffer(5)]],
    uint gid [[thread_position_in_grid]]
) {
    if (gid >= policy.population_count) {
        return;
    }

    uint parent_a_index = gid % policy.population_count;
    uint parent_b_index = (gid + 1u) % policy.population_count;
    DualStateGenomeMetal a = parents[parent_a_index];
    DualStateGenomeMetal b = parents[parent_b_index];

    uint4 mask_lo = uint4(
        lane_entropy(entropy_seed, gid, 0u, policy.generation),
        lane_entropy(entropy_seed, gid, 1u, policy.generation),
        lane_entropy(entropy_seed, gid, 2u, policy.generation),
        lane_entropy(entropy_seed, gid, 3u, policy.generation)
    );
    uint4 mask_hi = uint4(
        lane_entropy(entropy_seed, gid, 4u, policy.generation),
        lane_entropy(entropy_seed, gid, 5u, policy.generation),
        lane_entropy(entropy_seed, gid, 6u, policy.generation),
        lane_entropy(entropy_seed, gid, 7u, policy.generation)
    );

    DualStateGenomeMetal child = a;
    child.id = (ulong(policy.generation) << 32) | ulong(gid);
    child.generation = policy.generation + 1u;
    child.live_lo = select4(a.live_lo, b.live_lo, mask_lo);
    child.live_hi = select4(a.live_hi, b.live_hi, mask_hi);
    child.live_lo = mutate4(child.live_lo, entropy_seed, gid, 8u, policy.generation, policy.mutation_rate);
    child.live_hi = mutate4(child.live_hi, entropy_seed, gid, 12u, policy.generation, policy.mutation_rate);

    child.twin_lo = child.live_lo ^ uint4(
        lane_entropy(uint(child.envelope.coherence_sig), gid, 16u, child.envelope.resuperposition_n),
        lane_entropy(uint(child.envelope.coherence_sig >> 32), gid, 17u, child.envelope.resuperposition_n),
        lane_entropy(as_type<uint>(child.envelope.alpha), gid, 18u, child.envelope.resuperposition_n),
        lane_entropy(as_type<uint>(child.envelope.beta), gid, 19u, child.envelope.resuperposition_n)
    );
    child.twin_hi = child.live_hi ^ uint4(
        lane_entropy(uint(child.envelope.coherence_sig), gid, 20u, child.envelope.resuperposition_n),
        lane_entropy(uint(child.envelope.coherence_sig >> 32), gid, 21u, child.envelope.resuperposition_n),
        lane_entropy(as_type<uint>(child.envelope.alpha), gid, 22u, child.envelope.resuperposition_n),
        lane_entropy(as_type<uint>(child.envelope.beta), gid, 23u, child.envelope.resuperposition_n)
    );

    float correctness = 1.0f - normalized_hamming(child.live_lo, child.live_hi, target.desired_lo, target.desired_hi);
    float novelty = normalized_hamming(child.live_lo, child.live_hi, target.novelty_lo, target.novelty_hi);
    float simplicity = 1.0f - bit_density(child.live_lo, child.live_hi);
    float reversibility = 1.0f;
    float weight_sum = target.correctness_weight + target.novelty_weight + target.simplicity_weight + target.reversibility_weight;
    float total = (
        correctness * target.correctness_weight +
        novelty * target.novelty_weight +
        simplicity * target.simplicity_weight +
        reversibility * target.reversibility_weight
    ) / max(weight_sum, 0.000001f);

    children[gid] = child;
    scores[gid] = FitnessScoreMetal {
        total,
        correctness,
        novelty,
        simplicity,
        reversibility
    };
}
