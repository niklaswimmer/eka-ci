//! Data structures for a derivation build.
//!
//! A derivation build is the process of realizing a derivation in the Nix store to check if it
//! builds successfully. Each build attempt is uniquely identified by a [`DrvBuildId`].
//!
//! For each attempt to build a derivation a new [`DrvBuildMetadata`] entry is stored in the
//! database.
//!
//! During the build process several [`DrvBuildEvent`] entries are inserted into the database. The
//! latest of these entries is the current build status.
use std::{borrow::Cow, collections::HashMap, num::NonZeroU32, path::PathBuf};

use serde::{Deserialize, Serialize};
use sqlx::{encode::IsNull, sqlite::SqliteArgumentValue, Decode, Encode, FromRow, Sqlite, Type};

use crate::db::model::git::{GitCommit, GitRepo};

use super::ForInsert;

/// Unique identifier for a derivation build attempt.
///
/// Combines the derivation identifier with a counter that keeps track of the number of build
/// attempts for that derivation.
#[derive(Clone, Debug, FromRow)]
pub struct DrvBuildId {
    /// The derivation that is attempted to be build.
    pub derivation: DrvId,

    /// The build attempt counter.
    ///
    /// This value is increased for each new attempt at building the derivation.
    ///
    /// Note that once the build outcome of a derivation has been determined, there is no point in
    /// trying to build the same derivation again. If it failed once, it will always fail.
    ///
    /// This counter is intended for cases in which the derivation build was interrupted due to
    /// external factors (see [`DrvBuildState::Interrupted`]). In these situations it may make sense
    /// to reattempt the build (depending on the interruption kind).
    pub build_attempt: NonZeroU32,
}

/// Metadata about a derivation build.
///
/// This metadata is useful to reproduce a build on a different machine. Note that in general,
/// only builds that ended with a state of [`DrvBuildState::Completed`] can be reproduced.
#[derive(Clone, Debug, FromRow)]
pub struct DrvBuildMetadata {
    /// The derivation build this metadata is associated with.
    #[sqlx(flatten)]
    pub build: DrvBuildId,

    /// The Git repository this derivation build originates from.
    pub git_repo: GitRepo,

    /// The Git commit this derivation build originates from.
    ///
    /// Note that this may not be the only commit that can produce this derivation. Because a
    /// derivation only needs to fully build once, later commits may still include this
    /// derivation but do not trigger a new build.
    pub git_commit: GitCommit,

    /// The Nix command that was used to build this derivation.
    pub build_command: DrvBuildCommand,
}

impl DrvBuildMetadata {
    pub fn for_insert(
        derivation: DrvId,
        git_repo: GitRepo,
        git_commit: GitCommit,
        build_command: DrvBuildCommand,
    ) -> ForInsert<Self> {
        ForInsert(Self {
            build: DrvBuildId {
                derivation,
                build_attempt: NonZeroU32::MAX,
            },
            git_repo,
            git_commit,
            build_command,
        })
    }
}

/// Command used to build the derivation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")] // use internally tagged serialization
pub enum DrvBuildCommand {
    /// Build a single attribute.
    SingleAttr {
        /// Path to the Nix executable.
        ///
        /// Since this will be a Nix store path, it conveniently also includes the executable's
        /// version and unique identifier.
        exec: PathBuf,
        /// Nix arguments.
        args: Vec<String>,
        /// Environment variables for the subprocess.
        env: HashMap<String, String>,
        /// The `.nix` file that contains the attribute.
        file: PathBuf,
        /// The attribute to build.
        attr: String,
    },
}

#[cfg(test)]
impl DrvBuildCommand {
    /// Returns a dummy build command. Useful for database inserts in tests.
    pub fn dummy() -> Self {
        Self::SingleAttr {
            exec: "/bin/nix".into(),
            args: Vec::new(),
            env: HashMap::new(),
            file: "/path/to/file.nix".into(),
            attr: "hello".to_owned(),
        }
    }
}

impl<'q> Encode<'q, Sqlite> for DrvBuildCommand {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        let encoded = serde_json::to_string(self)?;
        buf.push(SqliteArgumentValue::Text(Cow::Owned(encoded)));

        Ok(IsNull::No)
    }
}

impl<'r> Decode<'r, Sqlite> for DrvBuildCommand {
    fn decode(
        value: <Sqlite as sqlx::Database>::ValueRef<'r>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let value = <&str as Decode<Sqlite>>::decode(value)?;
        let command = serde_json::from_str(value)?;

        Ok(command)
    }
}

impl Type<Sqlite> for DrvBuildCommand {
    fn type_info() -> <Sqlite as sqlx::Database>::TypeInfo {
        <str as Type<Sqlite>>::type_info()
    }
}

/// Emitted whenever a derivation build's state changes.
#[derive(Clone, Debug, FromRow)]
pub struct DrvBuildEvent {
    /// The derivation build this event is associated with.
    #[sqlx(flatten)]
    pub build: DrvBuildId,

    /// The build state this event propagates.
    pub state: DrvBuildState,

    /// The timestamp when this event happened.
    ///
    /// This timestamp only has second accuracy, which makes it unsuitable for sorting of build
    /// events. If for example the build queue is empty, it is not unlikely that a build is
    /// scheduled ([`DrvBuildState::Pending`]) and started ([`DrvBuildState::Building`]) within the
    /// same second.
    ///
    /// Instead, use the table's ROWID to sort the events during select.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl DrvBuildEvent {
    pub fn for_insert(build: DrvBuildId, state: DrvBuildState) -> ForInsert<Self> {
        ForInsert(Self {
            build,
            state,
            timestamp: chrono::DateTime::<chrono::Utc>::MAX_UTC,
        })
    }
}

/// Describes the possible states a derivation build can be in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DrvBuildState {
    /// Build is scheduled in a queue and has not yet started.
    Pending,
    /// Build is currently running.
    Building,
    /// Build has completed, either successfully or not.
    ///
    /// True means successful, false means an error was encountered while building the derivation.
    Completed(DrvBuildResult),
    /// Build was interrupted before it could complete.
    Interrupted(DrvBuildInterruptionKind),
    /// Build depends on a derivation that is [Interrupted][DrvBuildState::Interrupted].
    Blocked,
}

impl DrvBuildState {
    pub fn as_i64(&self) -> i64 {
        match self {
            DrvBuildState::Pending => 1,
            DrvBuildState::Building => 7, // a lucky number
            DrvBuildState::Completed(drv_build_result) => match drv_build_result {
                DrvBuildResult::Success => 42, // found the answer to the Ultimate Question of Life, the Universe, and Everything
                DrvBuildResult::Failure => -1,
            },
            DrvBuildState::Interrupted(drv_build_interruption_kind) => {
                match drv_build_interruption_kind {
                    DrvBuildInterruptionKind::OutOfMemory => -404, // 404 Memory Not Found
                    DrvBuildInterruptionKind::Timeout => -420,     // time for a timeout
                    DrvBuildInterruptionKind::Cancelled => -86,    // cancel the order
                    DrvBuildInterruptionKind::ProcessDeath => -666, // number of the beast
                    DrvBuildInterruptionKind::SchedulerDeath => -13, // an unlucky number
                }
            }
            DrvBuildState::Blocked => 11,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid state number: {0}")]
pub struct InvalidStateNumber(i64);

impl TryFrom<i64> for DrvBuildState {
    type Error = InvalidStateNumber;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Pending),
            7 => Ok(Self::Building),
            42 => Ok(Self::Completed(DrvBuildResult::Success)),
            -1 => Ok(Self::Completed(DrvBuildResult::Failure)),
            -404 => Ok(Self::Interrupted(DrvBuildInterruptionKind::OutOfMemory)),
            -420 => Ok(Self::Interrupted(DrvBuildInterruptionKind::Timeout)),
            -86 => Ok(Self::Interrupted(DrvBuildInterruptionKind::Cancelled)),
            -666 => Ok(Self::Interrupted(DrvBuildInterruptionKind::ProcessDeath)),
            -13 => Ok(Self::Interrupted(DrvBuildInterruptionKind::SchedulerDeath)),
            11 => Ok(Self::Blocked),
            _ => Err(InvalidStateNumber(value)),
        }
    }
}

impl<'q> Encode<'q, Sqlite> for DrvBuildState {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        buf.push(SqliteArgumentValue::Int64(self.as_i64()));

        Ok(IsNull::No)
    }

    fn size_hint(&self) -> usize {
        std::mem::size_of::<i64>()
    }
}

impl<'r> Decode<'r, Sqlite> for DrvBuildState {
    fn decode(
        value: <Sqlite as sqlx::Database>::ValueRef<'r>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let value = <i64 as Decode<Sqlite>>::decode(value)?;

        Ok(value.try_into()?)
    }
}

impl Type<Sqlite> for DrvBuildState {
    fn type_info() -> <Sqlite as sqlx::Database>::TypeInfo {
        <i64 as Type<Sqlite>>::type_info()
    }
}

/// The result of building a derivation.
///
/// In essence, this enum captures whether the status code returned by the build command was `0`
/// or not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DrvBuildResult {
    /// The derivation built successfully.
    Success,
    /// The derivation failed to build.
    Failure,
}

impl DrvBuildResult {
    /// Handy helper that allows processing the build result in a more functional style using
    /// [map][Result::map], [map_err][Result::map_err], [map_or_else][Result::map_or_else] and
    /// the like.
    pub fn as_result(&self) -> Result<(), ()> {
        match self {
            DrvBuildResult::Success => Ok(()),
            DrvBuildResult::Failure => Err(()),
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failure)
    }
}

/// Possible causes for why the derivation build was interrupted.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// derivation builds which do not have the status [`DrvBuildState::Completed`] whilst
    /// starting.
    SchedulerDeath,
}

/// A derivation identifier of the form `hash-name.drv`.
///
/// Many derivations that describe a package (binaries, libraries, ...) additionally include a
/// version identifier in the name component. For these derivations, the identifier often looks
/// like `hash-name-version.drv`. This is however only a convention. Many intermediate build
/// artifacts for example do not have a version.
///
/// Each derivation identifier corresponds to a file with the same name located in a nix store. The
/// filesystem path of the store depends on the evaluator that produced the derivation and is part
/// of the identifier's hash component[^nix-by-hand]. It is not possible to determine the store
/// path given only a derivation identifier.
///
/// # Examples
///
/// Derivation for the hello package, version 2.12.1:
/// `jd83l3jn2mkn530lgcg0y523jq5qji85-hello-2.12.1.drv`
///
/// Derivation for the source of an unknown other derivation:
/// `0aykaqxhbby7mx7lgb217m9b3gkl52fn-source.drv`
///
/// [^nix-by-hand]: <https://bernsteinbear.com/blog/nix-by-hand/>
#[derive(Clone, Debug, PartialEq, Eq, Type)]
#[sqlx(transparent)]
pub struct DrvId(String);

/// Constructors and methods useful for testing.
#[cfg(test)]
impl DrvId {
    /// Returns a known good derivation identifier. Useful for database inserts in tests.
    pub fn dummy() -> Self {
        DrvId("jd83l3jn2mkn530lgcg0y523jq5qji85-hello-2.12.1.drv".to_owned())
    }
}

/// The edge in a derivation dependency DAG.
///
/// Maps a derivation to all the derivations it directly depends on and vice-versa to all the
/// derivations that directly depend on it.
#[derive(Clone, Debug)]
pub struct DrvRefs {
    /// Also known as dependant or consumer.
    pub referrer: DrvId,
    /// Also known as dependency.
    pub reference: DrvId,
}
