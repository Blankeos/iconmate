Review the iconmate skill definition at `skills/iconmate/SKILL.md` and verify it accurately reflects the current state of the CLI.

## Steps

1. Read `skills/iconmate/SKILL.md` in full.
2. Run `iconmate --help` and `iconmate add --help`, `iconmate delete --help`, `iconmate list --help`, `iconmate iconify --help` to get the current CLI interface.
3. Read the project `README.md` for any features or commands not covered in the skill.
4. Read `Cargo.toml` for the current version number.
5. Compare and report:
   - **Missing commands**: CLI commands not documented in the skill.
   - **Outdated flags**: Flags in the skill that no longer exist or have changed.
   - **Wrong defaults**: Default values that don't match current behavior.
   - **Version mismatch**: If the skill metadata version doesn't match `Cargo.toml`.
   - **Missing features**: Significant capabilities described in the README but absent from the skill.
6. Fix any issues found directly in `skills/iconmate/SKILL.md`.
7. Print a summary of what was checked and any changes made.
