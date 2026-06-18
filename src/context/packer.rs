use crate::decay::ScoredMemory;
use crate::storage::models::MemoryUnit;

/// Pack a memory into a string within a char budget.
/// Fidelity degradation: full → summary → essence.
pub fn build_entry(mem: &MemoryUnit, budget: usize) -> String {
    let mut entry = String::new();

    // Essence always included if budget allows
    if mem.essence.len() + 4 > budget {
        return String::new();
    }
    entry.push_str("## ");
    entry.push_str(&mem.essence);
    entry.push('\n');

    // Source ref
    let src_line = format!("[source: {}]\n", mem.source_ref);
    if entry.len() + src_line.len() <= budget {
        entry.push_str(&src_line);
    }

    // Facts
    if !mem.facts.is_empty() {
        let facts_str = mem.facts.join("\n- ");
        let facts_block = format!("Facts:\n- {}\n", facts_str);
        if entry.len() + facts_block.len() <= budget {
            entry.push_str(&facts_block);
        }
    }

    // Summary
    if let Some(ref summary) = mem.summary {
        if entry.len() + summary.len() + 3 <= budget {
            entry.push_str(summary);
            entry.push('\n');
        }
    }

    entry
}

/// Build context string from ranked memories within char budget.
/// Returns (context_string, used_chars, n_units).
pub fn build_context(memories: &[ScoredMemory], char_budget: usize) -> (String, usize, usize) {
    let mut context = String::new();
    let mut used = 0usize;
    let mut n_units = 0usize;

    for scored in memories {
        if used >= char_budget {
            break;
        }
        let remaining = char_budget.saturating_sub(used);

        let entry = build_entry(&scored.memory, remaining);
        if entry.is_empty() {
            continue;
        }

        let separator = if n_units > 0 { "\n---\n" } else { "" };
        let needed = separator.len() + entry.len();

        if used + needed > char_budget && n_units > 0 {
            break;
        }

        context.push_str(separator);
        context.push_str(&entry);
        used += needed;
        n_units += 1;
    }

    (context, used, n_units)
}
