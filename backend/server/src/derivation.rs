use std::num::NonZero;

/// Unique identifier for a derivation build attempt.
pub struct DrvBuild {
    /// The derivation that is attempted to be build.
    pub derivation: DrvId,
    /// Denotes the number of times this derivation has been attempted to be build.
    ///
    /// Note that once the build outcome of a derivation has been determined, there is no point in
    /// trying to build the same derivation again. If it failed once, it will always fail.
    ///
    /// This counter is intended for cases in which the derivation build was interrupted due to
    /// external factors (see [DrvBuildState::Interrupted]). In these situations it may make sense
    /// to reattempt the build (depending on the interruption kind).
    pub build_attempt: NonZero<u32>,
}

/// Metadata about a derivation build.
///
/// This metadata is useful to reproduce a build on a different machine. Note that in general,
/// only builds that ended with a state of [DrvBuildState::Completed] can be reproduced.
pub struct DrvBuildMetadata {
    /// The derivation build this metadata is associated with.
    pub build: DrvBuild,

    /// The Git commit this derivation build is based upon.
    ///
    /// Note that this may not be the only commit that can produce this derivation. Because a
    /// derivation only needs to fully build once, later commits may still include this
    /// derivation but do not trigger a new build.
    pub commit: gix_hash::ObjectId,

    /// The Nix command that was used to build this derivation.
    pub build_command: DrvBuildCommand,
}

/// Command used to build the derivation.
pub enum DrvBuildCommand {
    /// Nix 1.0 style command. Fields are TBD.
    Nix1
}

/// Build events are emitted whenever a derivation build's state changes.
pub struct DrvBuildEvent {
    /// The derivation build this event is associated with.
    pub build: DrvBuild,

    /// Monotonically increasing numeric event identifier. Use this to determine the order of
    /// events. Events with a higher id happened later.
    ///
    /// Since this id is global it can be used to retrieve the latest build state for a derivation
    /// without having to check [DrvBuild::build_attempt].
    pub event_id: u32,

    /// The build state this event propagates.
    pub state: DrvBuildState,

    /// The timestamp when this event happened.
    ///
    /// This timestamp only has second accuracy, which makes it unsuitable for sorting of build
    /// events. If for example the build queue is empty, it is not unlikely that a build is
    /// scheduled ([DrvBuildState::Pending]) and started ([DrvBuildState::Building]) within the
    /// same second.
    ///
    /// Using a higher accuracy timestamp would make it incompatible with SQlite's datetime
    /// functions which is also not desirable.
    ///
    /// Instead, use [event_id] to determine the order of events.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Describes the possible states a derivation build can be in.
pub enum DrvBuildState {
    /// Build is scheduled in a queue and has not yet started.
    Pending,
    /// Build is currently running.
    Building,
    /// Build has completed, either successfully or not.
    Completed {
        result: DrvBuildResult,

        // TBD
        build_log: (),
    },
    /// Build was interrupted before it could complete.
    Interrupted {
        reason: DrvBuildInterruptionKind,
    },
}

/// Possible causes for why the derivation build was interrupted.
pub enum DrvBuildInterruptionKind {
    /// Build process ran out of memory and was killed by the system.
    OutOfMemory,
    /// Build process timed out and was killed by the build scheduler.
    Timeout,
    /// Scheduler process performed a graceful shutdown and cancelled the derivation build in the
    /// process.
    Cancelled,
    /// Build process died for unknown reasons, most likely a fault in the build command.
    ProcessDeath,
    /// Scheduler process died. The scheduler can infer that this happend by checking for
    /// derivation builds which do not have the status [DrvBuildStatus::Completed] whilst
    /// starting.
    SchedulerDeath,
}

/// The result of building a derivation.
///
/// In essence, this enum captures whether the status code returned by the build command was `0`
/// or not.
pub enum DrvBuildResult {
    /// The derivation build successfully.
    Ok,
    /// The derivation failed to build.
    Error,
}

/// A derivation id of the form `hash-name.drv`.
///
/// For example: `jd83l3jn2mkn530lgcg0y523jq5qji85-hello-2.12.1.drv`
///
/// Note that most derivation include a version number in their name, this is however only a
/// convention and not a requirement for a valid derivation id.
///
/// Additionally, derivation ids are *not* independent from the store path, as their hash is in
/// part based on it. For the purpose of the derivation build, the store path does however not
/// matter and we can reduce memory consumption by not including that common prefix here.
pub struct DrvId(String);
