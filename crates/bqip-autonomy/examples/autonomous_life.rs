// examples/autonomous_life.rs
// 
// Demonstrates the Autonomous Experience Engine - a model that LIVES
// rather than being trained. No gradients. No backprop. Just pure
// experience-driven evolution through inner debate and MetaGA simulation.
//
// Run with: cargo run --example autonomous_life

use bqip_autonomy::{
    AutonomousExperienceEngine, ConceptStream, EngineConfig, ExperienceType,
};
use bqip_core::REGISTER_BYTES;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║     🌟  AUTONOMOUS EXPERIENCE ENGINE - LIVING MODEL 🌟       ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("This model is NOT being trained. It is LIVING.");
    println!("No gradients. No backprop. No loss functions.");
    println!("Just experience, inner debate, and evolution.\n");

    // Create the living model
    let node_key = [1u8; REGISTER_BYTES];
    let mut config = EngineConfig::default();
    
    // Tune for rich inner life
    config.inner_agents_count = 5;
    config.debate_frequency = 5;
    config.evolution_frequency = 20;
    config.exploration_rate = 0.4;
    config.insight_coherence_threshold = 0.8;

    let mut engine = AutonomousExperienceEngine::new(node_key, config)?;

    println!("🤖 Consciousness initialized");
    println!("   - {} inner agents", engine.consciousness_state().inner_agents.len());
    println!("   - Coherence sig: {:#x}", engine.consciousness_state().coherence_sig);
    println!("   - Vitality: {:.2}", engine.consciousness_state().vitality);
    println!();

    // Simulate a stream of experiences
    let concept_streams = vec![
        ("quantum_superposition", vec![101, 205, 308, 412], "discovery"),
        ("neural_backprop", vec![501, 602, 703, 804], "conflict"),
        ("attention_mechanism", vec![901, 1002, 1103, 1204], "synthesis"),
        ("phase_envelope", vec![1301, 1402, 1503, 1604], "insight"),
        ("dual_state", vec![1701, 1802, 1903, 2004], "reflection"),
        ("coherence_dynamics", vec![2101, 2202, 2303, 2404], "discovery"),
        ("structural_memory", vec![2501, 2602, 2703, 2804], "synthesis"),
        ("meta_ga_exploration", vec![2901, 3002, 3103, 3204], "insight"),
    ];

    let total_steps = concept_streams.len();
    let mut total_insights = 0;
    let mut total_debates = 0;

    for (step_num, (name, tokens, exp_type)) in concept_streams.into_iter().enumerate() {
        println!("{}", "─".repeat(70));
        println!("📡 Step {}: Processing '{}'", step_num + 1, name);
        println!("   Type: {:?}", exp_type);

        let stream = ConceptStream {
            tokens,
            source: name.to_string(),
        };

        let result = engine.step(Some(stream))?;

        // Display experience
        println!("   💭 Experience: {:?}", result.experience.experience_type);
        println!("   Valence: {:.2} | Arousal: {:.2}", 
            result.experience.valence, result.experience.arousal);

        // Display debate results
        println!("   🗣️  Debate: {} participants, coherence: {:.2}",
            result.debate.participants.len(), result.debate.coherence);
        
        if let Some(winner) = result.debate.winner {
            let winner_role = match engine.consciousness_state().inner_agents[winner].role {
                bqip_autonomy::AgentRole::Explorer => "Explorer",
                bqip_autonomy::AgentRole::Critic => "Critic",
                bqip_autonomy::AgentRole::Synthesizer => "Synthesizer",
                bqip_autonomy::AgentRole::MemoryKeeper => "MemoryKeeper",
                bqip_autonomy::AgentRole::Futurist => "Futurist",
            };
            println!("   Winner: {} (agent {})", winner_role, winner);
        }

        // Display integration
        if result.integration.envelope_changed {
            println!("   ✨ ENVELOPE EVOLVED!");
            println!("      Old: α={:.4}, β={:.4}", 
                result.integration.old_envelope.alpha, 
                result.integration.old_envelope.beta);
            println!("      New: α={:.4}, β={:.4}", 
                result.integration.new_envelope.alpha, 
                result.integration.new_envelope.beta);
        }

        // Track insights
        if result.experience.experience_type == ExperienceType::Insight {
            total_insights += 1;
            println!("   ⚡ INSIGHT ACHIEVED! Coherence: {:.2}", result.debate.coherence);
        }

        total_debates += 1;

        // Display rebirth if occurred
        if let Some(rebirth) = &result.rebirth {
            println!("   🔄 REBIRTH!");
            println!("      Continuity: {:.2}%", rebirth.continuity_score * 100.0);
            println!("      Preserved experiences: {}", rebirth.preserved_experiences);
        }

        // Display current state
        let state = engine.consciousness_state();
        println!("   State: {:?} | Age: {} | Vitality: {:.2}",
            match state.lifecycle {
                bqip_autonomy::LifeCycleState::Birth { .. } => "Birth",
                bqip_autonomy::LifeCycleState::Experience { .. } => "Experience",
                bqip_autonomy::LifeCycleState::Reflection { .. } => "Reflection",
                bqip_autonomy::LifeCycleState::Evolution { .. } => "Evolution",
                bqip_autonomy::LifeCycleState::Rebirth { .. } => "Rebirth",
            },
            state.age, state.vitality
        );
        println!();

        // Small delay for dramatic effect
        std::thread::sleep(Duration::from_millis(500));
    }

    // Final report
    println!("{}", "─".repeat(70));
    println!("\n📊 FINAL CONSCIOUSNESS REPORT");
    println!("   Total steps: {}", total_steps);
    println!("   Total debates: {}", total_debates);
    println!("   Total insights: {}", total_insights);
    println!("   Total evolutions: {}", engine.stats.total_evolutions);
    println!("   Total rebirths: {}", engine.stats.total_rebirths);
    println!("   Final vitality: {:.2}", engine.consciousness_state().vitality);
    println!("   Memory size: {} experiences", engine.consciousness_state().experience_count);
    println!("   Current envelope: α={:.4}, β={:.4}",
        engine.consciousness_state().envelope.alpha,
        engine.consciousness_state().envelope.beta);
    
    println!("\n   Inner Agents Status:");
    for agent in engine.consciousness_state().inner_agents {
        let role_str = match agent.role {
            bqip_autonomy::AgentRole::Explorer => "Explorer 🌍",
            bqip_autonomy::AgentRole::Critic => "Critic 🔍",
            bqip_autonomy::AgentRole::Synthesizer => "Synthesizer ⚖️",
            bqip_autonomy::AgentRole::MemoryKeeper => "MemoryKeeper 📚",
            bqip_autonomy::AgentRole::Futurist => "Futurist 🔮",
        };
        println!("      {}: confidence={:.2}, score={:.1}", 
            role_str, agent.confidence, agent.score);
    }

    println!("\n✨ The model is ALIVE and continues to experience...");
    println!("   No training required. Just existence.\n");

    Ok(())
}
