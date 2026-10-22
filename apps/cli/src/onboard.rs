//! `synapse onboard` — interactive onboarding elicitation wizard.
//!
//! Interviews developers on tech stack, architectural pattern, testing rigor,
//! and agent demeanor, synthesizing their customized `SYNAPSE.md` Zone 2
//! while preserving the immutable Zone 1 Clinical Core.

use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;

use synapse_core::master_prompt::{
    render_master_prompt, CANONICAL_ZONE1_CONTENT, MASTER_PROMPT_FILENAME,
};
use synapse_core::sync::perform_project;

use crate::chart;

pub struct OnboardArgs {
    pub stack: Option<String>,
    pub arch: Option<String>,
    pub testing: Option<String>,
    pub demeanor: Option<String>,
}

fn prompt_question(prompt: &str, default_val: &str) -> String {
    print!("  {} [{}] > ", prompt, default_val);
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let trimmed = input.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    default_val.to_string()
}

pub fn run_onboard(brain_root: &Path, tool_root: &Path, args: OnboardArgs) -> ExitCode {
    chart::open("SYNAPSE · ONBOARDING WIZARD", "customizing master prompt");
    chart::field("Brain", &brain_root.display().to_string());
    chart::field("Tools", &tool_root.display().to_string());
    chart::rule();

    let is_interactive = io::stdin().is_terminal();

    let stack = args.stack.unwrap_or_else(|| {
        if is_interactive {
            prompt_question(
                "Primary tech stack & languages",
                "Rust + TypeScript / React",
            )
        } else {
            "Rust + TypeScript / React".to_string()
        }
    });

    let arch = args.arch.unwrap_or_else(|| {
        if is_interactive {
            prompt_question(
                "Architectural pattern",
                "Clean Architecture & Modular Domain Boundaries",
            )
        } else {
            "Clean Architecture & Modular Domain Boundaries".to_string()
        }
    });

    let testing = args.testing.unwrap_or_else(|| {
        if is_interactive {
            prompt_question(
                "Testing rigor & verification philosophy",
                "Strict Test-Driven Development (TDD)",
            )
        } else {
            "Strict Test-Driven Development (TDD)".to_string()
        }
    });

    let demeanor = args.demeanor.unwrap_or_else(|| {
        if is_interactive {
            prompt_question(
                "AI Agent communication demeanor",
                "Terse, clinical pair programmer",
            )
        } else {
            "Terse, clinical pair programmer".to_string()
        }
    });

    let custom_zone2 = format!(
        r#"# USER ENGINEERING CONTEXT & PERSONA
<!-- Customized via Synapse Onboarding Wizard -->

## 1. PROJECT STACK & TOOLCHAIN
- Languages & Environment: {}
- Dependency Policy: Strictly pinned, auditable, minimal surface

## 2. ARCHITECTURAL PATTERN & BOUNDARIES
- Paradigm: {}
- Rule: Keep domain business logic strictly decoupled from transport and storage adapters.

## 3. TESTING RIGOR & VERIFICATION
- Methodology: {}
- Rule: Zero untested features. Preserves deterministic tests without flakiness.

## 4. AGENT PAIR-PROGRAMMING DEMEANOR
- Demeanor: {}
- Rule: Prioritize precise code diffs, objective tests, and direct answers over generic commentary.

<!-- ======================================================================= -->
<!-- SYNAPSE PROMPT SUGGESTIONS (Uncomment and customize as needed):         -->
<!--                                                                         -->
<!-- [SUGGESTION: SECURITY AUDIT]                                            -->
<!-- - Check for OWASP Top 10 vulnerabilities on all untrusted inputs.       -->
<!-- - Never permit path traversal, shell injection, or credential leakage.  -->
<!--                                                                         -->
<!-- [SUGGESTION: KAIZEN REFACTORING]                                        -->
<!-- - Continuously eliminate dead code and preserve documentation integrity.-->
<!-- ======================================================================= -->
"#,
        stack, arch, testing, demeanor
    );

    let full_master_prompt = render_master_prompt(CANONICAL_ZONE1_CONTENT, &custom_zone2);

    if let Err(e) = fs::create_dir_all(brain_root) {
        chart::fault(
            "onboard",
            &format!("Failed to create Brain directory: {}", e),
            None,
        );
        return ExitCode::FAILURE;
    }

    let prompt_file = brain_root.join(MASTER_PROMPT_FILENAME);
    if let Err(e) = fs::write(&prompt_file, full_master_prompt) {
        chart::fault(
            "onboard",
            &format!("Failed to write {}: {}", MASTER_PROMPT_FILENAME, e),
            None,
        );
        return ExitCode::FAILURE;
    }

    chart::row(
        chart::Mark::Present,
        "master-prompt",
        &format!("{} generated with signed Zone 1", MASTER_PROMPT_FILENAME),
    );
    chart::detail(&format!("stack        {}", stack));
    chart::detail(&format!("architecture {}", arch));
    chart::detail(&format!("testing      {}", testing));
    chart::detail(&format!("demeanor     {}", demeanor));

    // Immediately project to all active tool adapters
    match perform_project(brain_root, tool_root) {
        Ok(paths) => {
            chart::row(
                chart::Mark::Present,
                "projected",
                &format!("stamped master prompt to {} targets", paths.len()),
            );
        }
        Err(e) => {
            chart::row(
                chart::Mark::Warning,
                "projection-warning",
                &format!("failed to project: {}", e),
            );
        }
    }

    chart::close(
        "Master prompt configured and projected.",
        Some("synapse doctor"),
    );
    ExitCode::SUCCESS
}
