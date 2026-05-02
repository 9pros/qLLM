#!/usr/bin/env python3
"""
Revolutionary BQIP Training Demo

This script demonstrates how to train the BQIP model with the new
revolutionary features:

1. Twin-State Consistency Regularization
2. Learnable Phase Envelopes (α, β)
3. Coherence-Signature Curriculum
4. Knowledge Graph Contrastive Pre-training

Usage:
    cargo run --example train_revolutionary

Or build and run:
    cargo build --release
    ./target/release/examples/train_revolutionary
"""

import subprocess
import sys

def run_command(cmd, description):
    """Run a shell command and display results."""
    print(f"\n{'='*60}")
    print(f"▶ {description}")
    print(f"{'='*60}")
    print(f"$ {cmd}\n")
    
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    
    if result.returncode == 0:
        print(result.stdout)
        print("✅ Success!")
    else:
        print("❌ Error:")
        print(result.stderr)
        return False
    
    return True

def main():
    print("""
╔════════════════════════════════════════════════════════════════╗
║   Revolutionary BQIP Training Demo                             ║
║   Quantum-Inspired Neural Architecture                         ║
╚════════════════════════════════════════════════════════════════╝
    """)
    
    print("""
This demo trains a BQIP model with revolutionary features:

🌟 Twin-State Consistency Regularization
   → Enforces quantum-like twin projection consistency
   
🌟 Learnable Phase Envelopes (α, β)
   → Model discovers optimal quantum basis
   
🌟 Coherence-Signature Curriculum
   → Progressive relaxation of phase constraints
   
🌟 Knowledge Graph Contrastive Pre-training
   → Pulls related concepts together
""")
    
    input("\nPress Enter to start training...")
    
    # Step 1: Check if project builds
    if not run_command(
        "cargo check --workspace --quiet",
        "Checking project build..."
    ):
        print("\n❌ Build failed. Please fix errors first.")
        sys.exit(1)
    
    # Step 2: Run tests
    if not run_command(
        "cargo test --workspace --quiet 2>&1 | tail -5",
        "Running tests..."
    ):
        print("\n⚠️  Some tests failed, but continuing...")
    
    # Step 3: Build release
    if not run_command(
        "cargo build --release --quiet 2>&1 | tail -3",
        "Building release binary..."
    ):
        print("\n❌ Release build failed.")
        sys.exit(1)
    
    # Step 4: Run training example
    print(f"\n{'='*60}")
    print(f"▶ Running Revolutionary Training Example")
    print(f"{'='*60}")
    print("""
This will:
  1. Create a concept tokenizer (vocab_size=512)
  2. Build training corpus from sample documents
  3. Configure revolutionary training:
     • twin_loss_weight=0.1
     • twin_perturbation_scale=0.05
     • phase_delta_increment=1
     • graph_contrastive_weight=0.15
  4. Train for 50 epochs
  5. Save model to bqip_model.bin
""")
    
    result = subprocess.run(
        "cargo run --release --example train_revolutionary",
        shell=True,
        capture_output=False,
        text=True
    )
    
    if result.returncode == 0:
        print("\n" + "="*60)
        print("🎉 Training Complete!")
        print("="*60)
        print("""
The model has been trained with revolutionary features:

✅ Twin-State Consistency: Enforced via decoherence loss
✅ Learnable Envelopes: α, β optimized for each concept
✅ Coherence Curriculum: Phase delta relaxed progressively
✅ Graph Contrastive: Related concepts pulled together

Model saved to: bqip_model.bin

You can now:
  • Use the model for inference
  • Fine-tune on your own data
  • Experiment with different hyperparameters
  • Explore the learned phase envelopes
""")
    else:
        print("\n❌ Training failed.")
        sys.exit(1)
    
    print("\n📚 For more details, see REVOLUTIONARY_TRAINING.md")
    print("💡 To customize training, modify examples/train_revolutionary.rs")

if __name__ == "__main__":
    main()
