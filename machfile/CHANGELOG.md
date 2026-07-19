# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.1](https://github.com/machfile/machfile/compare/machfile-v0.3.0...machfile-v0.3.1) - 2026-07-19

### Other

- bump dependencies
- bump dependencies

## [0.3.0](https://github.com/machfile/machfile/compare/machfile-v0.2.0...machfile-v0.3.0) - 2026-07-18

### Added

- implement additional argument forwarding for tasks

### Other

- Integrate dry-run into builder pattern
- Implement `--no-env-file` & `--env-file` arguments
- Implement new builder in CLI
- Implement env variable substitution in task
- Prepare new public execution API
- Remove unused `Config` default impl
- Finish config parsing in builder implementation
- Start implementing builder pattern for `MachConfig`
- Start reimplementing as struct based API
- Allow empty values in env file to override existing values
- Strip text after quotes and handle inline comments
- Implement variable expansion
- Rewrite into struct
- Start working on quotes and add more tests
- Use custom error for env file parsing
- Start working on env_file parsing
- Reapply "Merge branch 'dry-run-flag'"
- Revert "Merge branch 'dry-run-flag'"
- Adjust formatting
- Add verbose flag & CliConfig
- Add dry run flag
- Add task specific --show-config
- Validate task names
- Fix lints and follow error guidelines
- Implement task name validation logic
