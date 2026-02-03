//! Skills system for LocalGPT
//!
//! Skills are SKILL.md files in the workspace/skills/ directory that provide
//! specialized instructions for specific tasks.

use anyhow::Result;
use std::fs;
use std::path::Path;

/// A skill loaded from the workspace
#[derive(Debug, Clone)]
pub struct Skill {
    /// Skill name (directory name)
    pub name: String,
    /// Path to SKILL.md
    pub path: String,
    /// Brief description (first non-empty line after frontmatter)
    pub description: String,
}

/// Load all skills from the workspace/skills directory
pub fn load_skills(workspace: &Path) -> Result<Vec<Skill>> {
    let skills_dir = workspace.join("skills");
    if !skills_dir.exists() {
        return Ok(Vec::new());
    }

    let mut skills = Vec::new();

    for entry in fs::read_dir(&skills_dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let skill_file = path.join("SKILL.md");
        if !skill_file.exists() {
            continue;
        }

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let content = fs::read_to_string(&skill_file)?;
        let description = extract_description(&content);

        skills.push(Skill {
            name,
            path: skill_file.to_string_lossy().to_string(),
            description,
        });
    }

    // Sort by name for consistent ordering
    skills.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(skills)
}

/// Extract a brief description from the SKILL.md content
/// Takes the first non-empty line after any frontmatter
fn extract_description(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();

    // Skip frontmatter if present
    let start_idx = if lines.first().map(|l| l.trim()) == Some("---") {
        // Find closing ---
        lines
            .iter()
            .skip(1)
            .position(|l| l.trim() == "---")
            .map(|i| i + 2)
            .unwrap_or(0)
    } else {
        0
    };

    // Find first non-empty, non-heading line
    for line in lines.iter().skip(start_idx) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Skip markdown headings
        if trimmed.starts_with('#') {
            continue;
        }
        // Return first content line, truncated
        return trimmed.chars().take(100).collect();
    }

    String::new()
}

/// Build skills prompt section for the system prompt
pub fn build_skills_prompt(skills: &[Skill]) -> String {
    if skills.is_empty() {
        return String::new();
    }

    let mut lines = Vec::new();
    lines.push("## Skills".to_string());
    lines.push(
        "Before replying: scan available skills below. If one clearly applies, \
         read its SKILL.md with read_file, then follow it."
            .to_string(),
    );
    lines.push(String::new());
    lines.push("<available_skills>".to_string());

    for skill in skills {
        lines.push(format!(
            "- name: {}\n  description: {}\n  location: {}",
            skill.name, skill.description, skill.path
        ));
    }

    lines.push("</available_skills>".to_string());
    lines.push(String::new());
    lines.push("Rules:".to_string());
    lines.push("- If exactly one skill clearly applies: read its SKILL.md, then follow it.".to_string());
    lines.push("- If multiple could apply: choose the most specific one.".to_string());
    lines.push("- If none clearly apply: do not read any SKILL.md.".to_string());
    lines.push(String::new());

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_description() {
        let content = r#"---
name: test
---
# Test Skill

This is a test skill that does something useful.
"#;
        let desc = extract_description(content);
        assert_eq!(desc, "This is a test skill that does something useful.");
    }

    #[test]
    fn test_extract_description_no_frontmatter() {
        let content = r#"# My Skill

A skill for doing things.
"#;
        let desc = extract_description(content);
        assert_eq!(desc, "A skill for doing things.");
    }
}
