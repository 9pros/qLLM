use bqip_core::{phase_project, DualState, PhaseEnvelope, Register, REGISTER_BYTES};
use bqip_evolver::{
    evaluate_fitness, population_diversity, CandidateGenome, EvolverError, FitnessTarget,
    MetaGaEvolver, MetaGaPolicy,
};

fn envelope() -> PhaseEnvelope {
    PhaseEnvelope::balanced(0xfeed_cafe)
}

fn target() -> FitnessTarget {
    FitnessTarget::balanced(
        Register::deterministic(b"desired-live"),
        Register::deterministic(b"novelty-basis"),
    )
}

#[test]
fn seeded_population_is_deterministic_and_phase_projected() {
    let policy = MetaGaPolicy::new(8).unwrap();
    let mut left =
        MetaGaEvolver::new([1u8; REGISTER_BYTES], envelope(), target(), policy.clone()).unwrap();
    let mut right =
        MetaGaEvolver::new([1u8; REGISTER_BYTES], envelope(), target(), policy).unwrap();

    let left_population = left.seed_population(b"seed");
    let right_population = right.seed_population(b"seed");

    assert_eq!(left_population, right_population);
    assert_eq!(left_population.len(), 8);
    for candidate in left_population {
        candidate.validate().unwrap();
        assert_eq!(
            candidate.state.twin,
            phase_project(candidate.state.live, candidate.envelope)
        );
    }
}

#[test]
fn fitness_rewards_correctness_and_reversibility() {
    let envelope = envelope();
    let live = Register::deterministic(b"target");
    let candidate =
        CandidateGenome::from_live(1, 0, envelope, &[2u8; REGISTER_BYTES], b"candidate", live);
    let target = FitnessTarget::balanced(live, Register::deterministic(b"other"));

    let score = evaluate_fitness(&candidate, &target).unwrap();

    assert_eq!(score.correctness, 1.0);
    assert_eq!(score.reversibility, 1.0);
    assert!(score.total > 0.5);
}

#[test]
fn invalid_twin_projection_is_rejected() {
    let envelope = envelope();
    let live = Register::deterministic(b"live");
    let invalid =
        CandidateGenome::from_live(1, 0, envelope, &[3u8; REGISTER_BYTES], b"invalid", live);
    let invalid = CandidateGenome {
        state: DualState::new(invalid.state.live, Register::zero()),
        ..invalid
    };

    assert!(matches!(
        invalid.validate(),
        Err(EvolverError::InvalidTwinProjection)
    ));
}

#[test]
fn evolution_round_preserves_population_and_changes_generation() {
    let policy = MetaGaPolicy::new(10).unwrap();
    let mut evolver =
        MetaGaEvolver::new([4u8; REGISTER_BYTES], envelope(), target(), policy).unwrap();
    let population = evolver.seed_population(b"round");

    let round = evolver.evolve_round(&population, b"round-1").unwrap();

    assert_eq!(round.generation, 1);
    assert_eq!(round.ranked.len(), 10);
    assert!(round.diversity > 0.0);
    assert!(round
        .ranked
        .windows(2)
        .all(|pair| pair[0].score.total >= pair[1].score.total));
    for candidate in round.ranked {
        candidate.genome.validate().unwrap();
    }
}

#[test]
fn policy_adapts_under_stagnation_and_low_diversity() {
    let mut policy = MetaGaPolicy::new(8).unwrap();
    let initial_mutation = policy.mutation_rate;
    let initial_phase = policy.phase_shift_rate;

    policy.adapt(Some(0.7), 0.7001, 0.05);

    assert!(policy.mutation_rate > initial_mutation);
    assert!(policy.phase_shift_rate > initial_phase);
}

#[test]
fn population_diversity_tracks_identical_and_distinct_candidates() {
    let envelope = envelope();
    let live = Register::deterministic(b"same");
    let same_a =
        CandidateGenome::from_live(1, 0, envelope, &[5u8; REGISTER_BYTES], b"same-a", live);
    let same_b =
        CandidateGenome::from_live(2, 0, envelope, &[5u8; REGISTER_BYTES], b"same-b", live);
    let distinct = CandidateGenome::from_live(
        3,
        0,
        envelope,
        &[5u8; REGISTER_BYTES],
        b"distinct",
        Register::deterministic(b"distinct"),
    );

    assert_eq!(population_diversity(&[same_a.clone(), same_b.clone()]), 0.0);
    assert!(population_diversity(&[same_a, same_b, distinct]) > 0.0);
}
