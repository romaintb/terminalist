# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-09-11

### Added
- Task creation dialog now shows sub-projects for better project organization
- Color helper to match Todoist color names
- README badges for better project visibility

### Changed
- Upgraded todoist-api dependency to v0.3.0
- Simplified tasks list box title rendering
- Renamed new_render to simply renderer for cleaner code structure

### Fixed
- Task creation dialog properly pre-selects the current project as task's project
- Linting errors and missing newlines

### Removed
- Removed mentions of the old status bar
- Cleaned up unused/dead code
- Removed traces of the old statusbar implementation

## [Unreleased]

### Dependencies
- Updated various project dependencies to latest versions