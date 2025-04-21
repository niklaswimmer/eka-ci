use anyhow;
use sqlx;
use sqlx::SqlitePool;

use super::model::build::DrvBuildMetadata;

/// Build attempt needs to be a new one.
/// Passwords contained in the git_repo URL are stored as clear text.
pub async fn new_drv_build_metadata(
    metadata: &DrvBuildMetadata,
    pool: &SqlitePool,
) -> anyhow::Result<i64> {
    let id = sqlx::query(
        r#"
INSERT INTO DrvBuildMetadata
(derivation, build_attempt, git_repo, git_commit, build_command)
VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(&metadata.build.derivation)
    .bind(metadata.build.build_attempt)
    .bind(&metadata.git_repo)
    .bind(&metadata.git_commit)
    .bind(&metadata.build_command)
    .execute(pool)
    .await?
    .last_insert_rowid();

    Ok(id)
}
