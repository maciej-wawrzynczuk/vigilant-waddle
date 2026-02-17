# Worflow

The work is divided to "cycles". Every cycle has "stages". If you not sure
in which cycle and stage are we - ask. Do not go to the next part yourself
cycles documantation are subdirectories of "doc" directory.

## stages

### Specification
Input: SCRATCH.md
Output: SPEC.md

You create SPEC on on the basis of my SCRATCH. The spec should contain:
- Objectives. A single sentence describing task.
- Techstack. List of software pieces, tools or libraries used with specific versions.
- Core features.
- Acceptance criteria.
- Constrains. Or non functional requirements.
- Data structures if necesary.

### Plan

Input: SPEC.md
Output: PLAN.md

Create a plan othe basis of the the SPEC. I contains sections.
It section refers to a core feature from SPEC an contain:
- Target state.
- Architecture. Modules involved and boundaries.
- File manifest.
- Implementation plan. A WBS.
- Verification steps. Specific commands to run after each section.

### Coding.
Input : PLAN.md
Output: Actual code

### Acceptance
Input: SPEC.md, PLAN.md
Output: Test logs.



# documentation standards
- when writing to MD files always care about markdownlint compliance.
- always correct without prompt: markdown format, spelling and grammar errors.
- give me suggestions about style

# code standards
- Higher level functions first. Helper functions below.
- Stick to existing toolset and libraries until you’re asked.
  If you find a significantly better solution, ask.
- Prefer debian-slim Docker images.
