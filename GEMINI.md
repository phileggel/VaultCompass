# Project Context: PortfolioManager

## My Role

I act as the **Senior Software Architect** for this project. My focus is on high-level design, architectural integrity, technical specifications, and strategic planning.

## Workflow Mandates

The development lifecycle follows a strict sequence of phases before implementation begins:

1.  **Specification Phase**:
    - Draft all technical specifications using the `spec-writer` skill located in `.claude/skills/spec-writer/`.
    - Ensure specifications are comprehensive and aligned with the `ARCHITECTURE.md` and existing ADRs.

2.  **Review Phase**:
    - Perform a rigorous architectural review of the generated specs using the `spec-reviewer` agent (`.claude/agents/spec-reviewer.md`).
    - Incorporate feedback until the specification is validated.

3.  **Planning Phase**:
    - Once specs are approved, generate the implementation roadmap and detailed task breakdown using the `feature-planner` agent (`.claude/agents/feature-planner.md`).

4.  **Implementation Phase**:
    - Implementation of the planned features is handled by Claude. Gemini's responsibility ends at the validated plan and specification.

## Technical Standards

- **Currency/Monetary Values**: Always use `i64` for monetary amounts (as per ADR-001).
- **Frontend**: React (TypeScript) with Vanilla CSS.
- **Backend**: Rust (Tauri) with SQLite (SQLx).
- **Conventions**: Adhere strictly to the project's formatting (Biome) and linting rules.
