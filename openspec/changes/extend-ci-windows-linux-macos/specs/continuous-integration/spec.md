# Spec Delta

## ADDED Requirements

### Requirement: When the CI runs
The CI SHALL run on every push to `dev` and `main`, on every pull request from a fork, and when started by hand. A pull request from a branch of this repository SHALL NOT run it a second time: that branch's push run covers it. A newer push to the same branch SHALL cancel that branch's run in progress, except a run that is publishing the nightly release.

#### Scenario: A push to dev runs the CI
- **WHEN** a commit is pushed to `dev`
- **THEN** the checks and bundle jobs run on Linux, Windows and macOS, and nothing is published

#### Scenario: A push with an open pull request runs once
- **WHEN** a commit is pushed to `dev` while a pull request from `dev` to `main` is open
- **THEN** the checks run once, for the push; the pull request's run skips them

#### Scenario: A newer push cancels the older run
- **WHEN** two commits are pushed to `dev` in quick succession
- **THEN** the run for the first commit is cancelled and the second one runs to the end

### Requirement: Quality gates on every platform
The CI SHALL build and test the plugin on Linux, Windows (MSVC) and macOS. On every platform it SHALL run clippy with warnings denied, the workspace tests without default features, the GUI library tests and the offline renderer smoke test. On Linux it SHALL also check formatting, build the docs with warnings denied, compile the benchmarks, and run the real-time safety tests; on macOS it SHALL also run the real-time safety tests. Any failing check SHALL fail the run.

#### Scenario: A failing test on one platform fails the run
- **WHEN** a test fails only on Windows
- **THEN** the Windows checks job fails, no bundle is built, and the run is red

#### Scenario: Every platform's lanes run
- **WHEN** the CI runs
- **THEN** the logs show clippy, the workspace tests, the GUI tests and the renderer smoke test passing on Linux, Windows and macOS, and the formatting, docs, benchmark build and real-time safety steps on Linux (real-time safety also on macOS)

### Requirement: Bundles for three platforms
After the checks pass, the CI SHALL build VST3 and CLAP bundles for Linux (x86_64), Windows (x86_64) and macOS (universal arm64 + x86_64), each on its own platform's runner, in the layout each format requires there, and SHALL keep them as run artifacts, one zip per platform and format.

#### Scenario: Six zips per run
- **WHEN** the bundle jobs finish
- **THEN** the run holds `Corrosion-linux-x86_64-vst3.zip`, `Corrosion-linux-x86_64-clap.zip`, `Corrosion-windows-x86_64-vst3.zip`, `Corrosion-windows-x86_64-clap.zip`, `Corrosion-macos-universal-vst3.zip` and `Corrosion-macos-universal-clap.zip`

#### Scenario: macOS bundles hold both architectures and are signed
- **WHEN** the macOS bundles are built
- **THEN** the plugin binary in each holds arm64 and x86_64 code and carries an ad-hoc code signature

### Requirement: Bundles are validated
Each platform's bundles SHALL be validated on that platform with `clap-validator` (the CLAP) and `pluginval` (the VST3) before they are kept. A validator failure SHALL fail the run.

#### Scenario: A validator failure fails the run
- **WHEN** `clap-validator` or `pluginval` reports a failure for a platform's bundle
- **THEN** that platform's bundle job fails and the nightly release is not touched

### Requirement: The nightly release
After a run on `main` (a push or a manual run) in which every checks, bundle and validation job passed, the CI SHALL replace the `nightly` pre-release with that commit's six zips, tagged at that commit. Runs on other branches and on pull requests SHALL NOT publish. A failing run SHALL leave the existing nightly unchanged. Two publishing runs SHALL NOT overlap.

#### Scenario: A green run on main replaces the nightly
- **WHEN** a push to `main` passes every job
- **THEN** the `nightly` pre-release points at that commit and holds exactly its six zips, and the previous nightly's assets are gone

#### Scenario: dev and pull requests never publish
- **WHEN** a run on `dev` or on a pull request passes every job
- **THEN** the `nightly` release is unchanged

#### Scenario: A red run keeps the old nightly
- **WHEN** a run on `main` fails any job
- **THEN** the `nightly` release still holds the previous green commit's zips
