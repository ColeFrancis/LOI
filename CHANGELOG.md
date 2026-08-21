# Change Log
All notable changes to this project will be documented in this file.
 
The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [0.4.0]

### Added

- All front-end stages of the compiler are completed and tested

### Changed

- Changed project name from nhdl to Language of Interaction (LOI)

## [0.3.0] - 2026-06-4

### Changed

- The network and simulator are now compile-time generic for different entity types and different operations
- Doc comments added to the beginning of every module

### Added

- Added new core entity type: Real with ADD and MUL ops

## [0.2.0] - 2026-06-1

### Changed

- Events happening in the same time step are handled in a random order
- Run takes as a first argument the max number of time steps it will be allowed to take.
- Run returns the number of steps it took as an option, none if max number of steps was exceeded
- Run takes as a second argument whether only necessary events are handled or not.
- Changed names of fundamental keywords
    - circuit -> network
    - gate -> relation
    - net -> entity

### Added

- Added watchers

## [0.1.0] - 2026-05-30

### Added

- Created project directory structure
- Created core logic simulator
